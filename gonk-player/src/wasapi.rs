use crate::Queue;
use core::slice;
use std::ffi::OsString;
use std::mem::{transmute, zeroed};
use std::os::windows::prelude::OsStringExt;
use std::ptr::{null, null_mut};
use std::sync::Once;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use winapi::shared::devpkey::DEVPKEY_Device_FriendlyName;
use winapi::shared::guiddef::GUID;
use winapi::shared::mmreg::{
    WAVEFORMATEX, WAVEFORMATEXTENSIBLE, WAVE_FORMAT_EXTENSIBLE, WAVE_FORMAT_IEEE_FLOAT,
};
use winapi::shared::ntdef::HANDLE;
use winapi::um::audioclient::{IAudioClient, IAudioRenderClient, IID_IAudioClient};
use winapi::um::audiosessiontypes::{
    AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_EVENTCALLBACK, AUDCLNT_STREAMFLAGS_RATEADJUST,
};
use winapi::um::combaseapi::{CoCreateInstance, CoInitializeEx, PropVariantClear, CLSCTX_ALL};
use winapi::um::mmdeviceapi::{
    eConsole, eRender, CLSID_MMDeviceEnumerator, IMMDevice, IMMDeviceEnumerator,
    DEVICE_STATE_ACTIVE,
};
use winapi::um::synchapi::{CreateEventA, WaitForSingleObject};
use winapi::um::unknwnbase::{IUnknown, IUnknownVtbl};
use winapi::um::winnt::HRESULT;
use winapi::{Interface, RIDL};

const MAX_BUFFER_SIZE: u32 = 1024;
const WAIT_OBJECT_0: u32 = 0x00000000;
const STGM_READ: u32 = 0;
const COINIT_MULTITHREADED: u32 = 0;
const AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM: u32 = 0x80000000;
const AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY: u32 = 0x08000000;

const KSDATAFORMAT_SUBTYPE_IEEE_FLOAT: GUID = GUID {
    Data1: 0x00000003,
    Data2: 0x0000,
    Data3: 0x0010,
    Data4: [0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71],
};

const COMMON_SAMPLE_RATES: [u32; 13] = [
    5512, 8000, 11025, 16000, 22050, 32000, 44100, 48000, 64000, 88200, 96000, 176400, 192000,
];

static INIT: Once = Once::new();
static mut DEVICES: Vec<Device> = Vec::new();
static mut DEFAULT_DEVICE: Option<Device> = None;

RIDL! {#[uuid(4142186656, 18137, 20408, 190, 33, 87, 163, 239, 43, 98, 108)]
interface IAudioClockAdjustment(IAudioClockAdjustmentVtbl): IUnknown(IUnknownVtbl) {
   fn SetSampleRate(
        flSampleRate: f32,
    ) -> HRESULT,
}}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Device {
    pub inner: *mut IMMDevice,
    pub name: String,
}

unsafe impl Send for Device {}
unsafe impl Sync for Device {}

#[inline]
pub fn check(result: i32) -> Result<(), String> {
    if result != 0 {
        //https://docs.microsoft.com/en-us/windows/win32/seccrypto/common-hresult-values
        match result {
            1 => Err("AUDCLNT_E_NOT_INITIALIZED".to_string()),
            //0x80070057
            -2147024809 => Err("Invalid argument.".to_string()),
            // 0x80004003
            -2147467261 => Err("Pointer is not valid".to_string()),
            -2004287483 => Err("AUDCLNT_E_NOT_STOPPED".to_string()),
            _ => Err(format!("{result}")),
        }
    } else {
        Ok(())
    }
}

pub fn init() {
    INIT.call_once(|| unsafe {
        CoInitializeEx(null_mut(), COINIT_MULTITHREADED);
    });
}

pub fn update_devices() {
    init();
    unsafe {
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
        let result =
            (*enumerator).EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE, &mut collection);
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
            //This is slow. Around 250us.
            let result = (*store).GetValue(
                &DEVPKEY_Device_FriendlyName as *const _ as *const _,
                &mut prop,
            );
            check(result).unwrap();

            let ptr_utf16 = *(&prop.data as *const _ as *const *const u16);
            let name = utf16_string(ptr_utf16);
            PropVariantClear(&mut prop);

            let device = Device {
                inner: device,
                name,
            };

            devices.push(device);
        }

        //Default device
        let mut device: *mut IMMDevice = null_mut();
        let result = (*enumerator).GetDefaultAudioEndpoint(
            eRender,
            eConsole,
            &mut device as *mut *mut IMMDevice,
        );
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
        let name = utf16_string(ptr_utf16);
        PropVariantClear(&mut prop);

        let default = Device {
            inner: device,
            name,
        };

        DEFAULT_DEVICE = Some(default);
        DEVICES = devices;
    }
}

pub fn devices() -> &'static [Device] {
    unsafe { &DEVICES }
}

pub fn default_device() -> Option<&'static Device> {
    unsafe { DEFAULT_DEVICE.as_ref() }
}

pub fn utf16_string(ptr_utf16: *const u16) -> String {
    // Find the length of the friendly name.
    let mut len = 0;

    unsafe {
        while *ptr_utf16.offset(len) != 0 {
            len += 1;
        }
    }

    // Create the utf16 slice and convert it into a string.
    let name_slice = unsafe { slice::from_raw_parts(ptr_utf16, len as usize) };
    let name_os_string: OsString = OsStringExt::from_wide(name_slice);
    name_os_string.to_string_lossy().to_string()
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

pub unsafe fn get_mix_format(audio_client: *mut IAudioClient) -> WAVEFORMATEXTENSIBLE {
    let mut format = null_mut();
    (*audio_client).GetMixFormat(&mut format);
    let format = &*format;

    if format.wFormatTag == WAVE_FORMAT_EXTENSIBLE && format.cbSize == 22 {
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
    }
}

pub unsafe fn is_format_supported(
    format: WAVEFORMATEXTENSIBLE,
    audio_client: *mut IAudioClient,
) -> *mut WAVEFORMATEX {
    let mut new_format = null_mut();
    let result = (*audio_client).IsFormatSupported(
        AUDCLNT_SHAREMODE_SHARED,
        &format as *const _ as *const WAVEFORMATEX,
        &mut new_format,
    );
    if result != 1 {
        check(result).unwrap();
    }
    new_format
}

pub struct StreamHandle {
    pub queue: Queue<f32>,
    pub audio_client: *mut IAudioClient,
    pub audio_clock_adjust: *mut IAudioClockAdjustment,
    pub device: Device,
    pub sample_rate: u32,
    pub buffer_size: u32,
    pub num_out_channels: u32,
    pub stream_dropped: Arc<AtomicBool>,
}

impl StreamHandle {
    pub unsafe fn new(device: &Device, sample_rate: u32) -> Self {
        init();
        if !COMMON_SAMPLE_RATES.contains(&sample_rate) {
            panic!("Invalid sample rate.");
        }

        let audio_client: *mut IAudioClient = {
            let mut audio_client = null_mut();
            let result = (*device.inner).Activate(
                &IID_IAudioClient,
                CLSCTX_ALL,
                null_mut(),
                &mut audio_client,
            );
            check(result).unwrap();
            assert!(!audio_client.is_null());
            audio_client as *mut _
        };

        let format = get_mix_format(audio_client);

        let mut deafult_period = zeroed();
        (*audio_client).GetDevicePeriod(&mut deafult_period, null_mut());

        if format.Format.nChannels < 2 {
            let channels = format.Format.nChannels;
            panic!("Device only has {} channels", channels);
        }

        let desired_format = new_wavefmtex(
            format.Format.wBitsPerSample as usize,
            format.Samples as usize,
            sample_rate as usize,
            format.Format.nChannels as usize,
        );

        if desired_format.Samples != 32 {
            panic!("64-bit buffers are not supported!");
        }

        let block_align = desired_format.Format.nBlockAlign as u32;

        let result = (*audio_client).Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_EVENTCALLBACK
                | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM
                | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY
                | AUDCLNT_STREAMFLAGS_RATEADJUST,
            deafult_period,
            deafult_period,
            &desired_format as *const _ as *const WAVEFORMATEX,
            null(),
        );
        check(result).unwrap();

        let mut audio_clock_ptr = null_mut();
        let result =
            (*audio_client).GetService(&IAudioClockAdjustment::uuidof(), &mut audio_clock_ptr);
        check(result).unwrap();
        let audio_clock_adjust: *mut IAudioClockAdjustment = transmute(audio_clock_ptr);

        let h_event = CreateEventA(null_mut(), 0, 0, null());
        (*audio_client).SetEventHandle(h_event);

        let mut renderclient_ptr = null_mut();
        let result =
            (*audio_client).GetService(&IAudioRenderClient::uuidof(), &mut renderclient_ptr);
        check(result).unwrap();
        let render_client: *mut IAudioRenderClient = transmute(renderclient_ptr);

        (*audio_client).Start();

        let stream_dropped = Arc::new(AtomicBool::new(false));
        //4096 samples that can be buffered. I've had a few lag spikes when opening chrome
        //so I know 1024 isn't enough. Going bigger may cause a delay since we don't actaull stop the stream.
        //This still isn't big enough.
        let queue = Queue::new(MAX_BUFFER_SIZE as usize);

        let audio_thread = AudioThread {
            queue: queue.clone(),
            stream_dropped: Arc::clone(&stream_dropped),
            audio_client,
            h_event,
            render_client,
            block_align: block_align as usize,
            channels: desired_format.Format.nChannels as usize,
            max_frames: MAX_BUFFER_SIZE as usize,
        };

        //TODO: Need a way to start/stop the audio thread.
        thread::spawn(move || {
            run(audio_thread);
        });

        StreamHandle {
            queue,
            audio_client,
            audio_clock_adjust,
            device: device.clone(),
            sample_rate: desired_format.Format.nSamplesPerSec,
            buffer_size: MAX_BUFFER_SIZE,
            num_out_channels: desired_format.Format.nChannels as u32,
            stream_dropped,
        }
    }
    pub fn set_sample_rate(&self, rate: u32) -> Result<(), String> {
        if COMMON_SAMPLE_RATES.contains(&rate) && rate != 192_000 {
            let result = unsafe { (*self.audio_clock_adjust).SetSampleRate(rate as f32) };
            check(result)
        } else {
            Err(String::from("Unsupported sample rate."))
        }
    }
}

impl Drop for StreamHandle {
    fn drop(&mut self) {
        self.stream_dropped.store(true, Ordering::Relaxed);
    }
}

pub struct AudioThread {
    pub queue: Queue<f32>,
    pub stream_dropped: Arc<AtomicBool>,
    pub audio_client: *mut IAudioClient,
    pub h_event: HANDLE,
    pub render_client: *mut IAudioRenderClient,
    pub block_align: usize,
    pub channels: usize,
    pub max_frames: usize,
}

unsafe impl Send for AudioThread {}

//TODO: Don't crash when a device disconnects.
//ie. Recover from check(result).unwrap()
pub unsafe fn run(thread: AudioThread) {
    let AudioThread {
        queue,
        stream_dropped,
        audio_client,
        h_event,
        render_client,
        block_align,
        channels,
        max_frames,
    } = thread;

    let mut device_buffer = Vec::new();

    while !stream_dropped.load(Ordering::Relaxed) {
        let channel_align = block_align / channels;

        let mut padding_count = zeroed();
        let result = (*audio_client).GetCurrentPadding(&mut padding_count);
        check(result).unwrap();

        let mut buffer_frame_count = zeroed();
        let result = (*audio_client).GetBufferSize(&mut buffer_frame_count);
        check(result).unwrap();

        let buffer_frame_count = (buffer_frame_count - padding_count) as usize;

        if buffer_frame_count > device_buffer.len() {
            device_buffer.resize(buffer_frame_count * block_align, 0);
        }

        let mut frames_written = 0;
        while frames_written < buffer_frame_count {
            let frames = (buffer_frame_count - frames_written).min(max_frames);

            for out_frame in &mut device_buffer
                [frames_written * block_align..(frames_written + frames) * block_align]
                .chunks_exact_mut(block_align)
            {
                for out_smp_bytes in out_frame.chunks_exact_mut(channel_align) {
                    let smp_bytes = queue.pop().unwrap_or(0.0).to_le_bytes();

                    out_smp_bytes[0..smp_bytes.len()].copy_from_slice(&smp_bytes);
                }
            }

            frames_written += frames;
        }

        // Write the output buffer to the device.
        let data = &device_buffer[0..buffer_frame_count * block_align];

        let nbr_bytes = buffer_frame_count * block_align;
        debug_assert_eq!(nbr_bytes, data.len());

        let mut buffer_ptr = null_mut();
        let result = (*render_client).GetBuffer(buffer_frame_count as u32, &mut buffer_ptr);
        check(result).unwrap();

        let buffer_slice = slice::from_raw_parts_mut(buffer_ptr, nbr_bytes);
        buffer_slice.copy_from_slice(data);
        (*render_client).ReleaseBuffer(buffer_frame_count as u32, 0);
        check(result).unwrap();

        if WaitForSingleObject(h_event, 1000) != WAIT_OBJECT_0 {
            panic!("Fatal WASAPI stream error while waiting for event");
        }
    }

    let result = (*audio_client).Stop();
    check(result).unwrap();
}
