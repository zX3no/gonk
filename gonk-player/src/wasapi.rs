use core::{ffi::c_void, slice};
use std::os::windows::prelude::OsStringExt;
use std::ptr::{null, null_mut};
use std::thread;
use std::time::Duration;
use std::{
    error::Error,
    mem::{size_of, transmute, zeroed},
};
use std::{ffi::OsString, sync::Once};
use winapi::{
    shared::devpkey::DEVPKEY_Device_FriendlyName,
    shared::guiddef::GUID,
    shared::mmreg::{WAVEFORMATEX, WAVEFORMATEXTENSIBLE, WAVE_FORMAT_IEEE_FLOAT},
    um::{
        audioclient::{
            IAudioClient, IAudioRenderClient, IID_IAudioClient, AUDCLNT_E_DEVICE_INVALIDATED,
        },
        audiosessiontypes::{
            AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
            AUDCLNT_STREAMFLAGS_RATEADJUST,
        },
        combaseapi::{CoCreateInstance, CoInitializeEx, PropVariantClear, CLSCTX_ALL},
        mmdeviceapi::{
            eConsole, eRender, CLSID_MMDeviceEnumerator, IMMDevice, IMMDeviceEnumerator,
            DEVICE_STATE_ACTIVE,
        },
        synchapi::{CreateEventA, WaitForSingleObject},
        winbase::{
            FormatMessageW, FORMAT_MESSAGE_FROM_SYSTEM, FORMAT_MESSAGE_IGNORE_INSERTS, INFINITE,
            WAIT_OBJECT_0,
        },
    },
    Interface,
};

use crate::backend::{Backend, Device};
use crate::decoder::Symphonia;

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

const COMMON_SAMPLE_RATES: [usize; 13] = [
    5512, 8000, 11025, 16000, 22050, 32000, 44100, 48000, 64000, 88200, 96000, 176400, 192000,
];

//TODO: It is very slow to collect devices
//I'm not sure if this is necessary though.
pub static mut DEVICES: Vec<Device> = Vec::new();
pub static mut DEFAULT_DEVICE: Option<Device> = None;

static ONCE: Once = Once::new();

pub unsafe fn init() {
    ONCE.call_once(|| {
        CoInitializeEx(null_mut(), COINIT_MULTITHREADED);

        let enumerator = imm_device_enumerator();
        update_output_devices(enumerator);

        //HACK: Not a hack apparently.
        let ptr: usize = enumerator as usize;
        thread::spawn(move || {
            let enumerator: *mut IMMDeviceEnumerator = ptr as *mut IMMDeviceEnumerator;
            loop {
                thread::sleep(Duration::from_millis(300));
                update_output_devices(enumerator);
            }
        });
    });
}

pub unsafe fn imm_device_enumerator() -> *mut IMMDeviceEnumerator {
    let mut enumerator: *mut IMMDeviceEnumerator = null_mut();
    check(CoCreateInstance(
        &CLSID_MMDeviceEnumerator,
        null_mut(),
        CLSCTX_ALL,
        &IMMDeviceEnumerator::uuidof(),
        &mut enumerator as *mut *mut IMMDeviceEnumerator as *mut _,
    ));
    enumerator
}

pub unsafe fn default_device(enumerator: *mut IMMDeviceEnumerator) -> Device {
    let mut device: *mut IMMDevice = null_mut();
    check((*enumerator).GetDefaultAudioEndpoint(
        eRender,
        eConsole,
        &mut device as *mut *mut IMMDevice,
    ));
    let mut store = null_mut();
    check((*device).OpenPropertyStore(STGM_READ, &mut store));

    let mut prop = zeroed();
    check((*store).GetValue(
        &DEVPKEY_Device_FriendlyName as *const _ as *const _,
        &mut prop,
    ));

    let ptr_utf16 = *(&prop.data as *const _ as *const *const u16);
    let name = utf16_string(ptr_utf16);
    PropVariantClear(&mut prop);

    Device {
        inner: device,
        name,
    }
}

pub unsafe fn update_output_devices(enumerator: *mut IMMDeviceEnumerator) {
    let mut collection = null_mut();

    check((*enumerator).EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE, &mut collection));

    let mut count: u32 = zeroed();
    check((*collection).GetCount(&mut count as *mut u32 as *const u32));

    if count == 0 {
        return;
    }

    let mut devices = Vec::new();

    for i in 0..count {
        //Get IMMDevice.
        let mut device = null_mut();
        check((*collection).Item(i, &mut device));

        //Get name.
        let mut store = null_mut();
        check((*device).OpenPropertyStore(STGM_READ, &mut store));

        let mut prop = zeroed();
        //This is slow. Around 250us.
        check((*store).GetValue(
            &DEVPKEY_Device_FriendlyName as *const _ as *const _,
            &mut prop,
        ));

        let ptr_utf16 = *(&prop.data as *const _ as *const *const u16);
        let name = utf16_string(ptr_utf16);
        PropVariantClear(&mut prop);

        let device = Device {
            inner: device,
            name,
        };

        devices.push(device);
    }

    DEFAULT_DEVICE = Some(default_device(enumerator));
    DEVICES = devices;
}

///https://docs.microsoft.com/en-us/windows/win32/seccrypto/common-hresult-values
#[inline]
#[track_caller]
pub unsafe fn check(result: i32) {
    if result != 0 {
        let mut buf = [0u16; 2048];
        let message_result = FormatMessageW(
            FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
            null_mut(),
            result as u32,
            0,
            buf.as_mut_ptr(),
            buf.len() as u32,
            null_mut(),
        );

        if message_result == 0 {
            let b = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
            if b > 2 {
                if let Some(slice) = &buf.get(..b - 2) {
                    let msg = String::from_utf16(slice).unwrap();
                    panic!("{msg}");
                }
            }
        }

        panic!("Failed to get error message from Windows. Error Code: {result:#x} {result}");
    }
}

pub unsafe fn utf16_string(ptr_utf16: *const u16) -> String {
    let mut len = 0;
    while *ptr_utf16.offset(len) != 0 {
        len += 1;
    }
    let slice = unsafe { slice::from_raw_parts(ptr_utf16, len as usize) };
    let os_str: OsString = OsStringExt::from_wide(slice);
    os_str.to_string_lossy().to_string()
}

pub struct Wasapi {
    pub audio_client: *mut IAudioClient,
    pub render_client: *mut IAudioRenderClient,
    pub format: WAVEFORMATEXTENSIBLE,

    pub buffer: Vec<f32>,

    pub h_event: *mut c_void,
}

impl Drop for Wasapi {
    fn drop(&mut self) {
        unsafe {
            (*self.render_client).Release();
            (*self.audio_client).Release();
        };
    }
}

impl Wasapi {
    pub fn new(device: &Device, sample_rate: Option<usize>) -> Self {
        unsafe {
            init();

            let audio_client: *mut IAudioClient = {
                let mut audio_client = null_mut();
                let result = (*device.inner).Activate(
                    &IID_IAudioClient,
                    CLSCTX_ALL,
                    null_mut(),
                    &mut audio_client,
                );
                if result == AUDCLNT_E_DEVICE_INVALIDATED {
                    todo!("	The user has removed either the audio endpoint device or the adapter device that the endpoint device connects to.");
                }
                check(result);

                assert!(!audio_client.is_null());
                audio_client as *mut _
            };

            let mut format = null_mut();
            (*audio_client).GetMixFormat(&mut format);
            let format = &mut *format;

            //Update format to desired sample rate.
            if let Some(sample_rate) = sample_rate {
                assert!(COMMON_SAMPLE_RATES.contains(&sample_rate));
                format.nSamplesPerSec = sample_rate as u32;
                format.nAvgBytesPerSec = sample_rate as u32 * format.nBlockAlign as u32;
            }

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
                Samples: format.wBitsPerSample,
                SubFormat: KSDATAFORMAT_SUBTYPE_IEEE_FLOAT,
                dwChannelMask: mask,
            };

            if format.Format.nChannels < 2 {
                // panic!("Ouput device has less than 2 channels.");
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
            check(result);

            //This must be set for some reason.
            let h_event = CreateEventA(null_mut(), 0, 0, null());
            (*audio_client).SetEventHandle(h_event);

            let mut renderclient_ptr = null_mut();
            check((*audio_client).GetService(&IAudioRenderClient::uuidof(), &mut renderclient_ptr));

            let render_client: *mut IAudioRenderClient = transmute(renderclient_ptr);

            (*audio_client).Start();

            Self {
                audio_client,
                render_client,
                format,
                buffer: Vec::new(),
                h_event,
            }
        }
    }
}

impl Backend for Wasapi {
    fn sample_rate(&self) -> usize {
        self.format.Format.nSamplesPerSec as usize
    }

    //(Outdated): IAudioClockAdjustment::SetSampleRate is not used anymore.
    //It seems like 192_000 & 96_000 Hz are a different grouping than the rest.
    //44100 cannot convert to 192_000 and vise versa.

    ///Name is misleading since the device is updated aswell.
    fn set_sample_rate(&mut self, sample_rate: usize, device: &Device) {
        debug_assert!(COMMON_SAMPLE_RATES.contains(&sample_rate));
        if sample_rate == self.sample_rate() {
            return;
        }

        //Not sure if this is necessary.
        unsafe { check((*self.audio_client).Stop()) };

        *self = Wasapi::new(device, Some(sample_rate));
    }

    fn fill_buffer(
        &mut self,
        volume: f32,
        symphonia: &mut Symphonia,
    ) -> Result<(), Box<dyn Error>> {
        unsafe {
            let mut padding_count = zeroed();
            if (*self.audio_client).GetCurrentPadding(&mut padding_count) != 0 {
                return Err("Sample-rate changed.")?;
            }

            let mut buffer_frame_count = zeroed();
            check((*self.audio_client).GetBufferSize(&mut buffer_frame_count));

            let buffer_frame_count = (buffer_frame_count - padding_count) as usize;
            let block_align = self.format.Format.nBlockAlign as usize;
            let buffer_size = buffer_frame_count * block_align;
            let channels = self.format.Format.nChannels as usize;

            let mut buffer_ptr = null_mut();
            check((*self.render_client).GetBuffer(buffer_frame_count as u32, &mut buffer_ptr));

            let slice = slice::from_raw_parts_mut(buffer_ptr, buffer_size);

            //Channel [0] & [1] are left and right. Other channels should be zeroed.
            //Float is 4 bytes so 0..4 is left and 4..8 is right.
            for bytes in slice.chunks_mut(size_of::<f32>() * channels) {
                let sample_bytes = &(symphonia.pop().unwrap_or(0.0) * volume).to_le_bytes();
                bytes[0..4].copy_from_slice(sample_bytes);

                let sample_bytes = &(symphonia.pop().unwrap_or(0.0) * volume).to_le_bytes();
                if channels > 1 {
                    bytes[4..8].copy_from_slice(sample_bytes);
                }
            }

            (*self.render_client).ReleaseBuffer(buffer_frame_count as u32, 0);

            if WaitForSingleObject(self.h_event, INFINITE) != WAIT_OBJECT_0 {
                unreachable!()
            }
            Ok(())
        }
    }
}
