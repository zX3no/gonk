use bitflags::bitflags;
use core::slice;
use std::mem::{transmute, zeroed};
use std::ptr::{null, null_mut};
use std::sync::Once;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use wasapi::{DeviceCollection, Direction};
use widestring::U16CString;
use winapi::shared::devpkey::DEVPKEY_Device_FriendlyName;
use winapi::shared::mmreg::{
    WAVEFORMATEX, WAVEFORMATEXTENSIBLE, WAVE_FORMAT_EXTENSIBLE, WAVE_FORMAT_IEEE_FLOAT,
};
use winapi::shared::ntdef::HANDLE;
use winapi::um::audioclient::{IAudioClient, IAudioRenderClient, IID_IAudioClient};
use winapi::um::audiosessiontypes::{AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_EVENTCALLBACK};
use winapi::um::combaseapi::{CoCreateInstance, PropVariantClear, CLSCTX_ALL};
use winapi::um::mmdeviceapi::{
    eConsole, eRender, CLSID_MMDeviceEnumerator, IMMDevice, IMMDeviceEnumerator,
};
use winapi::um::synchapi::{CreateEventA, WaitForSingleObject};
use winapi::Interface;

const PREALLOC_FRAMES: usize = 48_000;
const BUFFER_SIZE: u32 = 512;
const MAX_BUFFER_SIZE: u32 = 1024;
const WAIT_OBJECT_0: u32 = 0x00000000;
const STGM_READ: u32 = 0;

static INIT: Once = Once::new();

// Defined at https://docs.microsoft.com/en-us/windows/win32/coreaudio/device-state-xxx-constants
bitflags! {
    struct DeviceState: u32 {
        const ACTIVE = 0b00000001;
        const DISABLED = 0b00000010;
        const NOTPRESENT = 0b00000100;
        const UNPLUGGED = 0b00001000;
        const ALL = 0b00001111;
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

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Device {
    pub name: String,
    pub id: String,
}

#[derive(Debug)]
pub struct OutputDeviceConfig {
    pub sample_rates: Vec<u32>,
    pub block_sizes: Option<BlockSizeRange>,
    pub num_out_channels: u16,
}

/// The range of possible block sizes for an audio device.
#[derive(Debug)]
pub struct BlockSizeRange {
    /// The minimum buffer/block size that can be used (inclusive)
    pub min: u32,

    /// The maximum buffer/block size that can be used (inclusive)
    pub max: u32,

    /// The default buffer/block size for this device
    pub default: u32,
}

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

#[inline]
pub fn check(result: i32) -> Result<(), String> {
    if result != 0 {
        Err(format!("{result:#x}"))
    } else {
        Ok(())
    }
}

pub fn check_init() {
    // Initialize this only once.
    INIT.call_once(|| wasapi::initialize_mta().unwrap());
}

pub fn enumerate_audio_backend() -> Vec<Device> {
    eprintln!("Enumerating WASAPI server...");

    check_init();

    let coll = match DeviceCollection::new(&Direction::Render) {
        Ok(coll) => coll,
        Err(e) => {
            panic!("Failed to get WASAPI device collection: {}", e);
        }
    };

    let num_devices = match coll.get_nbr_devices() {
        Ok(num_devices) => num_devices,
        Err(e) => {
            panic!("Failed to get number of WASAPI devices: {}", e);
        }
    };

    let mut devices: Vec<Device> = Vec::new();

    for i in 0..num_devices {
        match coll.get_device_at_index(i) {
            Ok(device) => {
                match device.get_id() {
                    Ok(device_id) => {
                        let device_name = match device.get_friendlyname() {
                            Ok(name) => name,
                            Err(e) => {
                                eprintln!(
                                    "Failed to get name of WASAPI device with ID {}: {}",
                                    &device_id, e
                                );
                                String::from("unkown device")
                            }
                        };

                        match device.get_state() {
                            Ok(state) => {
                                match DeviceState::from_bits(state) {
                                    Some(state) => {
                                        // What a weird API of using bit flags for each of the different
                                        // states the device can be in.
                                        if state.contains(DeviceState::DISABLED) {
                                            eprintln!("The WASAPI device {} has been disabled by the user", &device_name);
                                        } else if state.contains(DeviceState::NOTPRESENT) {
                                            eprintln!(
                                                "The WASAPI device {} is not present",
                                                &device_name
                                            );
                                        } else {
                                            devices.push(Device {
                                                name: device_name,
                                                id: device_id,
                                            })
                                        }
                                    }
                                    None => {
                                        eprintln!(
                                            "Got invalid state {} for WASAPI device {}",
                                            state, &device_name
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!(
                                    "Failed to get state of WASAPI device {}: {}",
                                    &device_name, e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to get ID of WASAPI device at index {}: {}", i, e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to get info of WASAPI device at index {}: {}", i, e);
            }
        }
    }

    if devices.is_empty() {
        panic!("Empty");
    } else {
        devices
    }
}

pub fn enumerate_audio_device(device: &Device) -> Result<OutputDeviceConfig, String> {
    eprintln!("Enumerating WASAPI device {} ...", &device.name);

    check_init();

    let (id, wdevice, _) = match find_device(device) {
        Some((id, device, jack_unpopulated)) => (id, device, jack_unpopulated),
        None => return Err(String::from("Failed to find device.")),
    };

    let audio_client = match wdevice.get_iaudioclient() {
        Ok(audio_client) => audio_client,
        Err(e) => {
            return Err(format!(
                "Failed to get audio client from WASAPI device {}: {}",
                &id.name, e
            ));
        }
    };

    // Get the default format for this device.
    let default_format = match audio_client.get_mixformat() {
        Ok(format) => format,
        Err(e) => {
            return Err(format!(
                "Failed to get default wave format of WASAPI device {}: {}",
                &id.name, e
            ));
        }
    };

    // We only care about channels and sample rate, and not the sample type.
    // We will always convert to/from `f32` buffers  anyway.
    // let default_sample_type = match default_format.get_subformat() {
    //     Ok(s) => s,
    //     Err(e) => {
    //         eprintln!(
    //             "Failed to get default wave format of WASAPI device {}: {}",
    //             &id.name,
    //             e
    //         );
    //         return Err(());
    //     }
    // };
    // let default_bps = default_format.get_bitspersample();
    // let default_vbps = default_format.get_validbitspersample();

    let default_sample_rate = default_format.get_samplespersec();
    let default_num_channels = default_format.get_nchannels();
    let default_buffer_size = match audio_client.get_bufferframecount() {
        Ok(b) => Some(BlockSizeRange {
            min: b,
            max: b,
            default: b,
        }),
        Err(e) => {
            eprintln!(
                "Could not get default buffer size of WASAPI device {}: {}",
                &id.name, e
            );
            None
        }
    };

    // We must use the default config when running in shared mode.
    Ok(OutputDeviceConfig {
        sample_rates: vec![default_sample_rate],
        block_sizes: default_buffer_size,
        num_out_channels: default_num_channels,
    })
}

pub fn find_device(device: &Device) -> Option<(Device, wasapi::Device, bool)> {
    eprintln!("Finding WASAPI device {} ...", &device.name);

    let coll = match DeviceCollection::new(&Direction::Render) {
        Ok(coll) => coll,
        Err(e) => {
            eprintln!("Failed to get WASAPI device collection: {}", e);
            return None;
        }
    };

    let num_devices = match coll.get_nbr_devices() {
        Ok(num_devices) => num_devices,
        Err(e) => {
            eprintln!("Failed to get number of WASAPI devices: {}", e);
            return None;
        }
    };

    for i in 0..num_devices {
        match coll.get_device_at_index(i) {
            Ok(d) => {
                match d.get_id() {
                    Ok(device_id) => {
                        let device_name = match d.get_friendlyname() {
                            Ok(name) => name,
                            Err(e) => {
                                eprintln!(
                                    "Failed to get name of WASAPI device with ID {}: {}",
                                    &device_id, e
                                );
                                String::from("unkown device")
                            }
                        };

                        match d.get_state() {
                            Ok(state) => {
                                match DeviceState::from_bits(state) {
                                    Some(state) => {
                                        // What a weird API of using bit flags for each of the different
                                        // states the device can be in.
                                        if state.contains(DeviceState::DISABLED) {
                                            eprintln!("The WASAPI device {} has been disabled by the user", &device_name);
                                        } else if state.contains(DeviceState::NOTPRESENT) {
                                            eprintln!(
                                                "The WASAPI device {} is not present",
                                                &device_name
                                            );
                                        } else {
                                            let id = Device {
                                                name: device_name,
                                                id: device_id,
                                            };

                                            if &id == device {
                                                let jack_unpopulated =
                                                    state.contains(DeviceState::UNPLUGGED);

                                                return Some((id, d, jack_unpopulated));
                                            }
                                        }
                                    }
                                    None => {
                                        eprintln!(
                                            "Got invalid state {} for WASAPI device {}",
                                            state, &device_name
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!(
                                    "Failed to get state of WASAPI device {}: {}",
                                    &device_name, e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to get ID of WASAPI device at index {}: {}", i, e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to get info of WASAPI device at index {}: {}", i, e);
            }
        }
    }

    None
}

pub fn new_wavefmtex(
    storebits: usize,
    validbits: usize,
    samplerate: usize,
    channels: usize,
) -> WAVEFORMATEXTENSIBLE {
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
    let sample = validbits as u16;
    let subformat = KSDATAFORMAT_SUBTYPE_IEEE_FLOAT;
    let mut mask = 0;
    for n in 0..channels {
        mask += 1 << n;
    }
    WAVEFORMATEXTENSIBLE {
        Format: wave_format,
        Samples: sample,
        SubFormat: subformat,
        dwChannelMask: mask,
    }
}

pub unsafe fn create_stream() -> StreamHandle {
    super::check_init();
    let mut enumerator: *mut IMMDeviceEnumerator = null_mut();

    let result = CoCreateInstance(
        &CLSID_MMDeviceEnumerator,
        null_mut(),
        CLSCTX_ALL,
        &IMMDeviceEnumerator::uuidof(),
        &mut enumerator as *mut *mut IMMDeviceEnumerator as *mut _,
    );
    check(result).unwrap();

    let mut device: *mut IMMDevice = null_mut();
    let result = (*enumerator).GetDefaultAudioEndpoint(
        eRender,
        eConsole,
        &mut device as *mut *mut IMMDevice,
    );
    check(result).unwrap();

    let mut store = null_mut();
    let result = (*device).OpenPropertyStore(STGM_READ, &mut store);
    check(result).unwrap();

    let mut value = zeroed();
    let result = (*store).GetValue(
        &DEVPKEY_Device_FriendlyName as *const _ as *const _,
        &mut value,
    );
    check(result).unwrap();

    let ptr_utf16 = *(&value.data as *const _ as *const *const u16);
    let name = U16CString::from_ptr_str(ptr_utf16).to_string().unwrap();
    // Clean up the property.
    PropVariantClear(&mut value);

    let mut id = null_mut();
    let result = (*device).GetId(&mut id);
    check(result).unwrap();

    let id = U16CString::from_ptr_str(id).to_string().unwrap();

    let id = Device { name, id };

    let audio_client: *mut IAudioClient = {
        let mut audio_client = null_mut();
        let result =
            (*device).Activate(&IID_IAudioClient, CLSCTX_ALL, null_mut(), &mut audio_client);
        check(result).unwrap();
        assert!(!audio_client.is_null());
        audio_client as *mut _
    };

    let mut format = null_mut();
    (*audio_client).GetMixFormat(&mut format);
    let format = &*format;

    let format = if format.wFormatTag == WAVE_FORMAT_EXTENSIBLE && format.cbSize == 22 {
        //TODO: Check that the sample type is float
        (format as *const _ as *const WAVEFORMATEXTENSIBLE).read()
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
        new_wavefmtex(storebits, validbits, samplerate, channels)
    };

    let mut deafult_period = zeroed();
    (*audio_client).GetDevicePeriod(&mut deafult_period, null_mut());

    let bps = format.Format.wBitsPerSample;
    let vbps = format.Samples;
    let sample_rate = format.Format.nSamplesPerSec;
    let channels = format.Format.nChannels;

    if channels < 2 {
        panic!();
    }

    let desired_format = new_wavefmtex(
        bps as usize,
        vbps as usize,
        sample_rate as usize,
        channels as usize,
    );
    let block_align = desired_format.Format.nBlockAlign as u32;

    let result = (*audio_client).Initialize(
        AUDCLNT_SHAREMODE_SHARED,
        AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
        deafult_period,
        deafult_period,
        &desired_format as *const _ as *const WAVEFORMATEX,
        null(),
    );
    check(result).unwrap();

    let h_event = CreateEventA(null_mut(), 0, 0, null());
    (*audio_client).SetEventHandle(h_event);

    let mut renderclient_ptr = null_mut();
    let result = (*audio_client).GetService(&IAudioRenderClient::uuidof(), &mut renderclient_ptr);
    check(result).unwrap();

    let render_client: *mut IAudioRenderClient = transmute(renderclient_ptr);

    (*audio_client).Start();

    let stream_dropped = Arc::new(AtomicBool::new(false));
    let stream_dropped_clone = Arc::clone(&stream_dropped);

    let stream_info = StreamInfo {
        id,
        connected_to_system: true,
        sample_rate,
        buffer_size: AudioBufferStreamInfo::UnfixedWithMaxSize(MAX_BUFFER_SIZE),
        num_out_channels: channels as u32,
    };

    let audio_thread = AudioThread {
        stream_info: stream_info.clone(),
        stream_dropped: stream_dropped_clone,
        audio_client,
        h_event,
        render_client,
        block_align: block_align as usize,
        vbps,
        channels: channels as usize,
        max_frames: MAX_BUFFER_SIZE as usize,
    };

    eprintln!("Creating audio thread");
    thread::spawn(move || {
        audio_thread.run();
    });

    StreamHandle {
        stream_info,
        stream_dropped,
    }
}

pub struct AudioThread {
    pub stream_info: StreamInfo,
    pub stream_dropped: Arc<AtomicBool>,
    pub audio_client: *mut IAudioClient,
    pub h_event: HANDLE,
    pub render_client: *mut IAudioRenderClient,
    pub block_align: usize,
    pub vbps: u16,
    pub channels: usize,
    pub max_frames: usize,
}

impl AudioThread {
    pub unsafe fn run(self) {
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
            let mut padding_count = zeroed();
            let result = (*audio_client).GetCurrentPadding(&mut padding_count);
            check(result).unwrap();

            let mut buffer_frame_count = zeroed();
            let result = (*audio_client).GetBufferSize(&mut buffer_frame_count);
            check(result).unwrap();

            let buffer_frame_count = (buffer_frame_count - padding_count) as usize;

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
            let nbr_frames = buffer_frame_count;
            let byte_per_frame = block_align;
            let data = &device_buffer[0..buffer_frame_count * block_align];

            let nbr_bytes = nbr_frames * byte_per_frame;
            if nbr_bytes != data.len() {
                panic!(
                    "Wrong length of data, got {}, expected {}",
                    data.len(),
                    nbr_bytes
                );
            }
            let mut bufferptr = null_mut();
            let result = (*render_client).GetBuffer(nbr_frames as u32, &mut bufferptr);
            check(result).unwrap();

            let bufferslice = slice::from_raw_parts_mut(bufferptr, nbr_bytes);
            bufferslice.copy_from_slice(data);
            let flags = 0;
            (*render_client).ReleaseBuffer(nbr_frames as u32, flags);
            check(result).unwrap();
            // eprintln!("wrote {} frames", nbr_frames);

            let retval = WaitForSingleObject(h_event, 1000);
            if retval != WAIT_OBJECT_0 {
                eprintln!("Fatal WASAPI stream error while waiting for event");
                break;
            }
        }

        let result = (*audio_client).Stop();
        if result != 0 {
            eprintln!("Error stopping WASAPI stream");
            check(result).unwrap();
        }

        eprintln!("WASAPI audio thread ended");
    }
}

unsafe impl Send for AudioThread {}
