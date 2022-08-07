use core::slice;
use std::mem::{transmute, zeroed};
use std::ptr::{null, null_mut};
use std::sync::Once;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use widestring::U16CString;
use winapi::shared::devpkey::DEVPKEY_Device_FriendlyName;
use winapi::shared::mmreg::{
    WAVEFORMATEX, WAVEFORMATEXTENSIBLE, WAVE_FORMAT_EXTENSIBLE, WAVE_FORMAT_IEEE_FLOAT,
};
use winapi::shared::ntdef::HANDLE;
use winapi::um::audioclient::{IAudioClient, IAudioRenderClient, IID_IAudioClient};
use winapi::um::audiosessiontypes::{AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_EVENTCALLBACK};
use winapi::um::combaseapi::{CoCreateInstance, CoInitializeEx, PropVariantClear, CLSCTX_ALL};
use winapi::um::mmdeviceapi::{
    eConsole, eRender, CLSID_MMDeviceEnumerator, IMMDevice, IMMDeviceEnumerator,
    DEVICE_STATE_ACTIVE,
};
use winapi::um::synchapi::{CreateEventA, WaitForSingleObject};
use winapi::Interface;

const PREALLOC_FRAMES: usize = 48_000;
const BUFFER_SIZE: u32 = 512;
const MAX_BUFFER_SIZE: u32 = 1024;
const WAIT_OBJECT_0: u32 = 0x00000000;
const STGM_READ: u32 = 0;
const COINIT_MULTITHREADED: u32 = 0;

static INIT: Once = Once::new();

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

#[derive(Debug, Clone)]
pub struct StreamInfo {
    /// The id of the audio device.
    pub id: Device,
    /// The sample rate of the stream.
    pub sample_rate: u32,
    /// The audio buffer size.
    pub buffer_size: u32,
    /// The number of audio output channels that will be passed into the
    /// process method.
    pub num_out_channels: u32,
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
    INIT.call_once(|| unsafe {
        CoInitializeEx(null_mut(), COINIT_MULTITHREADED);
    });
}

pub unsafe fn devices() -> Vec<(*mut IMMDevice, Device)> {
    check_init();

    let mut enumerator: *mut IMMDeviceEnumerator = null_mut();
    let result = CoCreateInstance(
        &CLSID_MMDeviceEnumerator,
        null_mut(),
        CLSCTX_ALL,
        &IMMDeviceEnumerator::uuidof(),
        &mut enumerator as *mut *mut IMMDeviceEnumerator as *mut _,
    );
    check(result).unwrap();

    let mut collection = null_mut();
    let result = (*enumerator).EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE, &mut collection);
    check(result).unwrap();
    let collection = &*collection;

    let mut count: u32 = zeroed();
    let result = collection.GetCount(&mut count as *mut u32 as *const u32);
    check(result).unwrap();

    let mut devices = Vec::new();

    for i in 0..count {
        //Get IMMDevice.
        let mut device = null_mut();
        let result = collection.Item(i, &mut device);
        check(result).unwrap();

        //Get name.
        let mut store = null_mut();
        let result = (*device).OpenPropertyStore(STGM_READ, &mut store);
        check(result).unwrap();
        let mut prop = zeroed();
        let result = (*store).GetValue(
            &DEVPKEY_Device_FriendlyName as *const _ as *const _,
            &mut prop,
        );
        check(result).unwrap();
        let ptr_utf16 = *(&prop.data as *const _ as *const *const u16);
        let wide_name = U16CString::from_ptr_str(ptr_utf16);
        let name = wide_name.to_string_lossy().to_string();
        PropVariantClear(&mut prop);

        //Get id.
        let mut id = zeroed();
        let result = (*device).GetId(&mut id);
        check(result).unwrap();
        let wide = U16CString::from_ptr_str(id);
        let id = wide.to_string_lossy().to_string();

        //Get device state.
        let mut state = zeroed();
        let result = (*device).GetState(&mut state);
        check(result).unwrap();
        if state != DEVICE_STATE_ACTIVE {
            panic!("Device is disabled?");
        }

        let id = Device { name, id };
        devices.push((device, id));
    }
    devices
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

pub unsafe fn default_device() -> *mut IMMDevice {
    check_init();
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
    device
}

pub unsafe fn create_stream() -> StreamHandle {
    check_init();
    // let device = default_device();
    let mut devices = devices();
    let (device, id) = devices.remove(0);

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
        panic!("Device only has {} channels", channels);
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
        sample_rate,
        buffer_size: MAX_BUFFER_SIZE,
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
