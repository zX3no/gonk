use core::slice;
use crossbeam_channel::Receiver;
use std::ffi::OsString;
use std::mem::{transmute, zeroed};
use std::os::windows::prelude::OsStringExt;
use std::ptr::{null, null_mut};
use std::sync::Once;
use std::time::Duration;
use winapi::shared::devpkey::DEVPKEY_Device_FriendlyName;
use winapi::shared::guiddef::GUID;
use winapi::shared::mmreg::{WAVEFORMATEX, WAVEFORMATEXTENSIBLE, WAVE_FORMAT_IEEE_FLOAT};
use winapi::um::audioclient::{IAudioClient, IAudioRenderClient, IID_IAudioClient};
use winapi::um::audiosessiontypes::{
    AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_EVENTCALLBACK, AUDCLNT_STREAMFLAGS_RATEADJUST,
};
use winapi::um::combaseapi::{CoCreateInstance, CoInitializeEx, PropVariantClear, CLSCTX_ALL};
use winapi::um::mmdeviceapi::{
    eConsole, eRender, CLSID_MMDeviceEnumerator, IMMDevice, IMMDeviceEnumerator,
    DEVICE_STATE_ACTIVE,
};
use winapi::um::synchapi::{CreateEventA, ResetEvent, WaitForSingleObject};
use winapi::um::unknwnbase::{IUnknown, IUnknownVtbl};
use winapi::um::winnt::HRESULT;
use winapi::{Interface, RIDL};

use crate::{Symphonia, VOLUME_REDUCTION};

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
//TODO: It is very slow to collect devices
//I'm not sure if this is necessary though.
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
            -2004287480 => Err("AUDCLNT_E_UNSUPPORTED_FORMAT".to_string()),
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

pub unsafe fn audio_client(_device: &Device) {
    todo!();
}

pub unsafe fn new(device: &Device, r: Receiver<Event>) {
    init();

    let audio_client: *mut IAudioClient = {
        let mut audio_client = null_mut();
        let result =
            (*device.inner).Activate(&IID_IAudioClient, CLSCTX_ALL, null_mut(), &mut audio_client);
        check(result).unwrap();
        assert!(!audio_client.is_null());
        audio_client as *mut _
    };

    let mut format = null_mut();
    (*audio_client).GetMixFormat(&mut format);
    let format = &mut *format;

    if format.wFormatTag != WAVE_FORMAT_IEEE_FLOAT {
        let format = &*(format as *const _ as *const WAVEFORMATEXTENSIBLE);
        if format.SubFormat.Data1 != KSDATAFORMAT_SUBTYPE_IEEE_FLOAT.Data1
            || format.SubFormat.Data2 != KSDATAFORMAT_SUBTYPE_IEEE_FLOAT.Data2
            || format.SubFormat.Data3 != KSDATAFORMAT_SUBTYPE_IEEE_FLOAT.Data3
            || format.SubFormat.Data4 != KSDATAFORMAT_SUBTYPE_IEEE_FLOAT.Data4
        {
            panic!("Unsupported sample format!");
        }
    }

    let mut mask = 0;
    for n in 0..format.nChannels {
        mask += 1 << n;
    }
    let format = WAVEFORMATEXTENSIBLE {
        Format: *format,
        Samples: format.wBitsPerSample as u16,
        SubFormat: KSDATAFORMAT_SUBTYPE_IEEE_FLOAT,
        dwChannelMask: mask,
    };

    if format.Format.nChannels < 2 {
        panic!("Device has less than 2 channels.");
    }

    let mut default_period = zeroed();
    let mut _min_period = zeroed();
    (*audio_client).GetDevicePeriod(&mut default_period, &mut _min_period);

    let result = (*audio_client).Initialize(
        AUDCLNT_SHAREMODE_SHARED,
        AUDCLNT_STREAMFLAGS_EVENTCALLBACK
            | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM
            | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY
            | AUDCLNT_STREAMFLAGS_RATEADJUST,
        default_period,
        default_period,
        &format as *const _ as *const WAVEFORMATEX,
        null(),
    );
    check(result).unwrap();

    let mut audio_clock_ptr = null_mut();
    let result = (*audio_client).GetService(&IAudioClockAdjustment::uuidof(), &mut audio_clock_ptr);
    check(result).unwrap();
    let audio_clock_adjust: *mut IAudioClockAdjustment = transmute(audio_clock_ptr);

    let h_event = CreateEventA(null_mut(), 0, 0, null());
    (*audio_client).SetEventHandle(h_event);

    let mut renderclient_ptr = null_mut();
    let result = (*audio_client).GetService(&IAudioRenderClient::uuidof(), &mut renderclient_ptr);
    check(result).unwrap();
    let render_client: *mut IAudioRenderClient = transmute(renderclient_ptr);

    (*audio_client).Start();

    let block_align = format.Format.nBlockAlign as usize;
    let sample_rate = format.Format.nSamplesPerSec;
    let channels = format.Format.nChannels as usize;
    let max_frames = MAX_BUFFER_SIZE as usize;

    let mut device_buffer = Vec::new();

    let mut decoder: Option<Symphonia> = None;

    loop {
        if let Ok(event) = r.try_recv() {
            match event {
                Event::PlaySong((path, s)) => {
                    STATE = s;
                    //TODO: Handle different sample rates.
                    match Symphonia::new(&path) {
                        Ok(sym) => {
                            DURATION = sym.duration();
                            ELAPSED = Duration::default();
                            let new = sym.sample_rate();
                            if sample_rate != new {
                                if COMMON_SAMPLE_RATES.contains(&new) && new != 192_000 {
                                    println!(
                                        "Adjusting sample rate from {} to {}",
                                        sample_rate, new
                                    );
                                    let result = (*audio_clock_adjust).SetSampleRate(new as f32);
                                    check(result).unwrap();
                                } else {
                                    todo!();
                                }
                            }
                            decoder = Some(sym);
                        }
                        Err(err) => panic!("{}", err),
                    }
                }
                Event::Seek(pos) => {
                    if let Some(decoder) = &mut decoder {
                        decoder.seek(pos);
                    }
                }
                Event::Play => STATE = State::Playing,
                Event::Pause => STATE = State::Paused,
                Event::Stop => STATE = State::Stopped,
            }
        }

        match STATE {
            State::Stopped => decoder = None,
            State::Paused => (),
            State::Finished => (),
            State::Playing => {
                if let Some(decoder) = &mut decoder {
                    ELAPSED = decoder.elapsed();

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
                        //This can range from 441 to 1024. I haven't see it go over MAX_FRAMES.
                        let frames = (buffer_frame_count - frames_written).min(max_frames);

                        for out_frame in &mut device_buffer
                            [frames_written * block_align..(frames_written + frames) * block_align]
                            .chunks_exact_mut(block_align)
                        {
                            for out_smp_bytes in out_frame.chunks_exact_mut(channel_align) {
                                let smp = decoder.next().unwrap_or(0.0) * VOLUME;
                                let smp_bytes = smp.to_le_bytes();
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
                    let result =
                        (*render_client).GetBuffer(buffer_frame_count as u32, &mut buffer_ptr);
                    check(result).unwrap();

                    let buffer_slice = slice::from_raw_parts_mut(buffer_ptr, nbr_bytes);
                    buffer_slice.copy_from_slice(data);
                    (*render_client).ReleaseBuffer(buffer_frame_count as u32, 0);
                    check(result).unwrap();

                    if WaitForSingleObject(h_event, 1000) != WAIT_OBJECT_0 {
                        panic!("Error occured while waiting for object.");
                    }

                    ResetEvent(h_event);
                }
            }
        }
    }

    // let result = (*audio_client).Stop();
    // check(result).unwrap();
}

#[derive(Debug)]
pub enum Event {
    ///Sometimes I want to queue a song but not play it.
    PlaySong((String, State)),
    Play,
    Pause,
    Stop,
    Seek(f32),
}

#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Stopped,
    Paused,
    Playing,
    Finished,
}

pub static mut STATE: State = State::Stopped;
pub static mut ELAPSED: Duration = Duration::from_secs(0);
pub static mut DURATION: Duration = Duration::from_secs(0);
pub static mut GAIN: f32 = 0.0;
pub static mut VOLUME: f32 = 10.0 / VOLUME_REDUCTION;

// pub fn set_sample_rate(&self, rate: u32) -> Result<(), String> {
// }
