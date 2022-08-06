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
use winapi::shared::mmreg::{
    WAVEFORMATEX, WAVEFORMATEXTENSIBLE, WAVE_FORMAT_EXTENSIBLE, WAVE_FORMAT_IEEE_FLOAT,
};
use winapi::um::audioclient::{IAudioClient, IID_IAudioClient};
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

const STGM_READ: u32 = 0;

#[inline]
pub fn check_result(result: i32) {
    if result != 0 {
        panic!("{result:#x}")
    }
}

#[macro_export]
macro_rules! DEFINE_GUID {
    (
        $name:ident, $l:expr, $w1:expr, $w2:expr,
        $b1:expr, $b2:expr, $b3:expr, $b4:expr, $b5:expr, $b6:expr, $b7:expr, $b8:expr
    ) => {
        pub const $name: winapi::shared::guiddef::GUID = winapi::shared::guiddef::GUID {
            Data1: $l,
            Data2: $w1,
            Data3: $w2,
            Data4: [$b1, $b2, $b3, $b4, $b5, $b6, $b7, $b8],
        };
    };
}

DEFINE_GUID! {KSDATAFORMAT_SUBTYPE_IEEE_FLOAT,
0x00000003, 0x0000, 0x0010, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71}

pub fn get_default_device() -> (&'static IMMDevice, String, String) {
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
        check_result(result);

        let mut device: *mut IMMDevice = null_mut();
        let result = (*enumerator).GetDefaultAudioEndpoint(
            eRender,
            eConsole,
            &mut device as *mut *mut IMMDevice,
        );
        check_result(result);

        let mut store = null_mut();
        let result = (*device).OpenPropertyStore(STGM_READ, &mut store);
        check_result(result);

        let mut value = zeroed();
        let result = (*store).GetValue(
            &DEVPKEY_Device_FriendlyName as *const _ as *const _,
            &mut value,
        );
        check_result(result);

        let ptr_utf16 = *(&value.data as *const _ as *const *const u16);
        let name = U16CString::from_ptr_str(ptr_utf16).to_string().unwrap();
        // Clean up the property.
        PropVariantClear(&mut value);

        let mut id = null_mut();
        let result = (*device).GetId(&mut id);
        check_result(result);

        let id = U16CString::from_ptr_str(id).to_string().unwrap();
        dbg!(&name);

        (&*device, name, id)
    }
}

pub fn get_mixformat(client: *mut IAudioClient) {
    unsafe {
        let mut format = null_mut();
        (*client).GetMixFormat(&mut format);
        let format = &*format;

        // let wavefmt = &*format;
        // let channels = wavefmt.nChannels;
        // let sample_rate = wavefmt.nSamplesPerSec;
        // let bps = wavefmt.wBitsPerSample;
        // let block_align = wavefmt.nBlockAlign;
        // let storebits = 8 * block_align as usize / channels as usize;

        let formatex = if format.wFormatTag == WAVE_FORMAT_EXTENSIBLE && format.cbSize == 22 {
            format as *const _ as *const WAVEFORMATEXTENSIBLE
        } else {
            let validbits = format.wBitsPerSample as usize;
            let blockalign = format.nBlockAlign as usize;
            let samplerate = format.nSamplesPerSec as usize;
            let formattag = format.wFormatTag;
            let channels = format.nChannels as usize;

            if formattag != WAVE_FORMAT_IEEE_FLOAT {
                panic!("Unsupported format!");
            }

            let storebits = 8 * blockalign / channels;

            let blockalign = channels * storebits / 8;
            let byterate = samplerate * blockalign;

            let wave_format = WAVEFORMATEX {
                cbSize: 22,
                nAvgBytesPerSec: byterate as u32,
                nBlockAlign: blockalign as u16,
                nChannels: channels as u16,
                nSamplesPerSec: samplerate as u32,
                wBitsPerSample: storebits as u16,
                wFormatTag: WAVE_FORMAT_EXTENSIBLE as u16,
            };

            // let sample = WAVEFORMATEXTENSIBLE_0 {
            //     wValidBitsPerSample: validbits as u16,
            // };
            let sample = validbits as u16;

            let subformat = KSDATAFORMAT_SUBTYPE_IEEE_FLOAT;
            let mut mask = 0;
            for n in 0..channels {
                mask += 1 << n;
            }
            let fmt = WAVEFORMATEXTENSIBLE {
                Format: wave_format,
                Samples: sample,
                SubFormat: subformat,
                dwChannelMask: mask,
            };
            &fmt as *const WAVEFORMATEXTENSIBLE
        };
        dbg!((*formatex).Format.nSamplesPerSec);
    }
}

pub fn get_periods(client: *mut IAudioClient) -> i64 {
    unsafe {
        let mut def_time = 0;
        (*client).GetDevicePeriod(&mut def_time, null_mut());
        def_time
    }
}

pub fn create_stream() -> Result<StreamHandle, Box<dyn Error>> {
    super::check_init();

    let (device, name, id) = get_default_device();

    let audio_client: *mut IAudioClient = unsafe {
        let mut audio_client = null_mut();
        let result = device.Activate(&IID_IAudioClient, CLSCTX_ALL, null_mut(), &mut audio_client);
        check_result(result);
        assert!(!audio_client.is_null());
        audio_client as *mut _
    };

    get_mixformat(audio_client);

    let id = Device { id, name };
    let device = wasapi::get_default_device(&wasapi::Direction::Render).unwrap();

    //Required sample_type, default_period, bps, vbps, sample_rate, channels
    //Then get the desired_format
    //Then the block align
    //Initialize the audio client

    let mut audio_client = device.get_iaudioclient()?;
    let default_format = audio_client.get_mixformat()?;
    let default_sample_type = default_format.get_subformat()?;
    let (default_period, _) = audio_client.get_periods()?;
    let bps = default_format.get_bitspersample();
    let vbps = default_format.get_validbitspersample();
    let sample_rate = default_format.get_samplespersec();
    let num_channels = default_format.get_nchannels();

    if let SampleType::Int = default_sample_type {
        return Err("SampleType::Int is not supported.")?;
    }

    // Check that the device has at-least two output channels.
    if num_channels < 2 {
        return Err("Stereo output not found")?;
    }

    let desired_format = wasapi::WaveFormat::new(
        bps as usize,
        vbps as usize,
        &SampleType::Float,
        sample_rate as usize,
        num_channels as usize,
    );

    let block_align = desired_format.get_blockalign();

    audio_client.initialize_client(
        &desired_format,
        default_period,
        &wasapi::Direction::Render,
        &wasapi::ShareMode::Shared,
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
        num_out_channels: num_channels as u32,
    };

    let audio_thread = AudioThread {
        stream_info: stream_info.clone(),
        stream_dropped: stream_dropped_clone,
        audio_client,
        h_event,
        render_client,
        block_align: block_align as usize,
        vbps,
        channels: num_channels as usize,
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
