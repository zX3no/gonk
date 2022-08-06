use std::error::Error;
use std::mem::{transmute, zeroed};
use std::ptr::null_mut;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;

use wasapi::SampleType;
use widestring::U16CString;
use winapi::shared::devpkey::DEVPKEY_Device_FriendlyName;
use winapi::um::combaseapi::{CoCreateInstance, PropVariantClear, CLSCTX_ALL};
use winapi::um::mmdeviceapi::{
    eConsole, eRender, CLSID_MMDeviceEnumerator, IMMDevice, IMMDeviceEnumerator,
};
use winapi::Interface;

use crate::Device;

const PREALLOC_FRAMES: usize = 48_000;
const BUFFER_SIZE: u32 = 512;
const MAX_BUFFER_SIZE: u32 = 1024;

#[derive(Debug, Clone)]
pub struct StreamInfo {
    /// The id of the audio device.
    pub id: Device,
    /// If this is `false` then it means the app failed to connect to
    /// the system device and is using "fake/virtual" empty buffers
    /// instead which will only input and output silence.
    pub connected_to_system: bool,
    /// The sample rate of the stream.
    pub sample_rate: u32,
    /// The audio buffer size.
    pub buffer_size: AudioBufferStreamInfo,
    /// The number of audio output channels that will be passed into the
    /// process method.
    pub num_out_channels: u32,
}

/// The audio buffer size of a stream.
#[derive(Debug, Clone, Copy)]
pub enum AudioBufferStreamInfo {
    FixedSized(u32),
    UnfixedWithMaxSize(u32),
}

pub struct StreamHandle {
    pub stream_info: StreamInfo,
    pub stream_dropped: Arc<AtomicBool>,
}

impl Drop for StreamHandle {
    fn drop(&mut self) {
        self.stream_dropped.store(true, Ordering::Relaxed);
    }
}

pub fn get_default_device() -> (IMMDevice, String, String) {
    super::check_init();

    unsafe {
        let mut enumerator: *mut IMMDeviceEnumerator = null_mut();

        let result = CoCreateInstance(
            &CLSID_MMDeviceEnumerator,
            null_mut(),
            CLSCTX_ALL,
            &IMMDeviceEnumerator::uuidof(),
            &mut enumerator as *mut *mut IMMDeviceEnumerator as *mut _,
        );

        if result != 0 {
            panic!("{result:#x}")
        }

        let mut device: *mut IMMDevice = null_mut();
        let result = (*enumerator).GetDefaultAudioEndpoint(
            eRender,
            eConsole,
            &mut device as *mut *mut IMMDevice,
        );

        if result != 0 {
            panic!("{result:#x}")
        }

        let mut test_device: IMMDevice = zeroed();
        let result = (*enumerator).GetDefaultAudioEndpoint(
            eRender,
            eConsole,
            &mut test_device as *mut IMMDevice as *mut *mut IMMDevice,
        );

        if result != 0 {
            panic!("{result:#x}")
        }

        let mut store = null_mut();
        let result = (*device).OpenPropertyStore(0x00000000, &mut store);

        if result != 0 {
            panic!("{result:#x}")
        }
        let mut value = zeroed();
        let result = (*store).GetValue(
            &DEVPKEY_Device_FriendlyName as *const _ as *const _,
            &mut value,
        );

        if result != 0 {
            panic!("{result:#x}")
        }

        let ptr_utf16 = *(&value.data as *const _ as *const *const u16);
        let name = U16CString::from_ptr_str(ptr_utf16).to_string().unwrap();
        // Clean up the property.
        PropVariantClear(&mut value);

        let mut id = null_mut();
        let result = (*device).GetId(&mut id);
        if result != 0 {
            panic!("{result:#x}")
        }

        let id = U16CString::from_ptr_str(id).to_string().unwrap();

        (test_device, name, id)
    }
}

pub fn create_stream() -> Result<StreamHandle, Box<dyn Error>> {
    super::check_init();

    // let (device, name, id) = get_default_device();
    // let id = Device { id, name };
    // dbg!(id);

    let (id, device) = match wasapi::get_default_device(&wasapi::Direction::Render) {
        Ok(device) => {
            let name = match device.get_friendlyname() {
                Ok(n) => n,
                Err(e) => {
                    eprintln!("Failed to get name of default WASAPI device: {}", e);
                    return Err("Unknown device name")?;
                }
            };

            let id = match device.get_id() {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("Failed to get ID of WASAPI device {}: {}", &name, e);
                    return Err("Unknown device id")?;
                }
            };
            (Device { name, id }, device)
        }
        Err(e) => panic!("{}", e),
    };

    let mut audio_client = device.get_iaudioclient()?;
    let default_format = audio_client.get_mixformat()?;
    let default_sample_type = default_format.get_subformat()?;
    let (default_period, _) = audio_client.get_periods()?;
    let default_bps = default_format.get_bitspersample();
    let default_vbps = default_format.get_validbitspersample();
    let default_sample_rate = default_format.get_samplespersec();
    let default_num_channels = default_format.get_nchannels();

    if let SampleType::Int = default_sample_type {
        return Err("SampleType::Int is not supported.")?;
    }

    // Check that the device has at-least two output channels.
    if default_num_channels < 2 {
        return Err("Stereo output not found")?;
    }

    // Check if this device supports running in exclusive mode.
    let (share_mode, sample_rate, bps, vbps, period) = (
        wasapi::ShareMode::Shared,
        default_sample_rate,
        default_bps,
        default_vbps,
        default_period,
    );

    let desired_format = wasapi::WaveFormat::new(
        bps as usize,
        vbps as usize,
        &SampleType::Float,
        sample_rate as usize,
        default_num_channels as usize,
    );

    let block_align = desired_format.get_blockalign();

    audio_client.initialize_client(
        &desired_format,
        period,
        &wasapi::Direction::Render,
        &share_mode,
        false,
    )?;

    let h_event = audio_client.set_get_eventhandle()?;

    let render_client = audio_client.get_audiorenderclient()?;

    audio_client.start_stream()?;

    let stream_dropped = Arc::new(AtomicBool::new(false));
    let stream_dropped_clone = Arc::clone(&stream_dropped);

    let stream_info = StreamInfo {
        id,
        connected_to_system: true,
        sample_rate,
        buffer_size: AudioBufferStreamInfo::UnfixedWithMaxSize(MAX_BUFFER_SIZE),
        num_out_channels: default_num_channels as u32,
    };

    let audio_thread = AudioThread {
        stream_info: stream_info.clone(),
        stream_dropped: stream_dropped_clone,
        audio_client,
        h_event,
        render_client,
        block_align: block_align as usize,
        vbps,
        channels: default_num_channels as usize,
        max_frames: MAX_BUFFER_SIZE as usize,
    };

    thread::spawn(move || {
        audio_thread.run();
    });

    Ok(StreamHandle {
        stream_info,
        stream_dropped,
    })
}

pub struct AudioThread {
    pub stream_info: StreamInfo,
    pub stream_dropped: Arc<AtomicBool>,
    pub audio_client: wasapi::AudioClient,
    pub h_event: wasapi::Handle,
    pub render_client: wasapi::AudioRenderClient,
    pub block_align: usize,
    pub vbps: u16,
    pub channels: usize,
    pub max_frames: usize,
}

impl AudioThread {
    pub fn run(self) {
        let AudioThread {
            stream_info,
            stream_dropped,
            audio_client,
            h_event,
            render_client,
            block_align,
            vbps,
            channels,
            max_frames,
        } = self;

        // The buffer that is sent to WASAPI. Pre-allocate a reasonably large size.
        let mut device_buffer = vec![0u8; PREALLOC_FRAMES * block_align];
        let mut device_buffer_capacity_frames = PREALLOC_FRAMES;

        // The owned buffers whose slices get sent to the process method in chunks.
        let mut proc_owned_buffers: Vec<Vec<f32>> = (0..channels)
            .map(|_| vec![0.0; max_frames as usize])
            .collect();

        let channel_align = block_align / channels;

        eprintln!("WASAPI stream bits per sample: {}", vbps);

        let mut phase: f32 = 0.0;
        let pitch: f32 = 440.0;
        let gain: f32 = 0.1;
        let step = std::f32::consts::PI * 2.0 * pitch / stream_info.sample_rate as f32;

        while !stream_dropped.load(Ordering::Relaxed) {
            let buffer_frame_count = match audio_client.get_available_space_in_frames() {
                Ok(f) => f as usize,
                Err(e) => {
                    eprintln!(
                        "Fatal WASAPI stream error getting buffer frame count: {}",
                        e
                    );
                    break;
                }
            };

            // Make sure that the device's buffer is large enough. In theory if we pre-allocated
            // enough frames this shouldn't ever actually trigger any allocation.
            if buffer_frame_count > device_buffer_capacity_frames {
                device_buffer_capacity_frames = buffer_frame_count;
                eprintln!("WASAPI wants a buffer of size {}. This may trigger an allocation on the audio thread.", buffer_frame_count);
                device_buffer.resize(buffer_frame_count as usize * block_align, 0);
            }

            let mut frames_written = 0;
            while frames_written < buffer_frame_count {
                let frames = (buffer_frame_count - frames_written).min(max_frames);

                // Clear and resize the buffer first. Since we never allow more than
                // `max_frames`, this will never allocate.
                for b in proc_owned_buffers.iter_mut() {
                    b.clear();
                    b.resize(frames, 0.0);
                }

                //Process audio here:
                {
                    let audio_outputs = proc_owned_buffers.as_mut_slice();

                    let frames = frames
                        .min(audio_outputs[0].len())
                        .min(audio_outputs[1].len());

                    for i in 0..frames {
                        // generate rudamentary sine wave
                        let smp = phase.sin() * gain;
                        phase += step;
                        if phase >= std::f32::consts::PI * 2.0 {
                            phase -= std::f32::consts::PI * 2.0
                        }

                        audio_outputs[0][i] = smp * 0.1;
                        audio_outputs[1][i] = smp * 0.1;
                    }
                }

                let device_buffer_part = &mut device_buffer
                    [frames_written * block_align..(frames_written + frames) * block_align];

                // Fill each slice into the device's output buffer
                if vbps == 32 {
                    for (frame_i, out_frame) in
                        device_buffer_part.chunks_exact_mut(block_align).enumerate()
                    {
                        for (ch_i, out_smp_bytes) in
                            out_frame.chunks_exact_mut(channel_align).enumerate()
                        {
                            let smp_bytes = proc_owned_buffers[ch_i][frame_i].to_le_bytes();

                            out_smp_bytes[0..smp_bytes.len()].copy_from_slice(&smp_bytes);
                        }
                    }
                } else {
                    todo!("64 bit buffers?");
                }

                frames_written += frames;
            }

            // Write the now filled output buffer to the device.
            if let Err(e) = render_client.write_to_device(
                buffer_frame_count as usize,
                block_align,
                &device_buffer[0..buffer_frame_count * block_align],
                None,
            ) {
                eprintln!("Fatal WASAPI stream error while writing to device: {}", e);
                break;
            }

            if let Err(e) = h_event.wait_for_event(1000) {
                eprintln!("Fatal WASAPI stream error while waiting for event: {}", e);
                break;
            }
        }

        if let Err(e) = audio_client.stop_stream() {
            eprintln!("Error stopping WASAPI stream: {}", e);
        }

        eprintln!("WASAPI audio thread ended");
    }
}

unsafe impl Send for AudioThread {}
