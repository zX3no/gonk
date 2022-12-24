use core::{ffi::c_void, slice};
use gonk_core::profile;
use std::ffi::OsString;
use std::mem::{transmute, zeroed};
use std::os::windows::prelude::OsStringExt;
use std::ptr::{null, null_mut};
use std::thread;
use std::time::Duration;
use winapi::um::audioclient::{IAudioClient, IAudioRenderClient, IID_IAudioClient};
use winapi::um::audiosessiontypes::{
    AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_EVENTCALLBACK, AUDCLNT_STREAMFLAGS_RATEADJUST,
};
use winapi::um::combaseapi::{CoCreateInstance, CoInitializeEx, PropVariantClear, CLSCTX_ALL};
use winapi::um::mmdeviceapi::{
    eConsole, eRender, CLSID_MMDeviceEnumerator, IMMDevice, IMMDeviceEnumerator,
    DEVICE_STATE_ACTIVE,
};
use winapi::um::synchapi::CreateEventA;
use winapi::um::unknwnbase::{IUnknown, IUnknownVtbl};
use winapi::um::winbase::{
    FormatMessageW, FORMAT_MESSAGE_FROM_SYSTEM, FORMAT_MESSAGE_IGNORE_INSERTS,
};
use winapi::um::winnt::HRESULT;
use winapi::{shared::devpkey::DEVPKEY_Device_FriendlyName, um::synchapi::WaitForSingleObject};
use winapi::{shared::guiddef::GUID, um::winbase::INFINITE};
use winapi::{
    shared::mmreg::{WAVEFORMATEX, WAVEFORMATEXTENSIBLE, WAVE_FORMAT_IEEE_FLOAT},
    um::winbase::WAIT_OBJECT_0,
};
use winapi::{Interface, RIDL};

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

//TODO: Inline this macro.
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

pub unsafe fn init() {
    CoInitializeEx(null_mut(), COINIT_MULTITHREADED);

    let mut enumerator: *mut IMMDeviceEnumerator = null_mut();
    check(CoCreateInstance(
        &CLSID_MMDeviceEnumerator,
        null_mut(),
        CLSCTX_ALL,
        &IMMDeviceEnumerator::uuidof(),
        &mut enumerator as *mut *mut IMMDeviceEnumerator as *mut _,
    ));

    update_output_devices(enumerator);

    //HACK: Not a hack apparently.
    let ptr: usize = enumerator as usize;
    thread::spawn(move || {
        let enumerator: *mut IMMDeviceEnumerator = ptr as *mut IMMDeviceEnumerator;
        loop {
            update_output_devices(enumerator);
            thread::sleep(Duration::from_millis(250));
        }
    });
}

pub unsafe fn update_output_devices(enumerator: *mut IMMDeviceEnumerator) {
    let mut collection = null_mut();

    check((*enumerator).EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE, &mut collection));

    let mut count: u32 = zeroed();
    check((*collection).GetCount(&mut count as *mut u32 as *const u32));

    if count == 0 {
        panic!("No output devices.");
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

    //Default default device.
    let mut device: *mut IMMDevice = null_mut();
    check((*enumerator).GetDefaultAudioEndpoint(
        eRender,
        eConsole,
        &mut device as *mut *mut IMMDevice,
    ));
    //TODO: This can crash when there are no devices.

    //Get default device name.
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

    DEFAULT_DEVICE = Some(Device {
        inner: device,
        name,
    });
    DEVICES = devices;
}

///https://docs.microsoft.com/en-us/windows/win32/seccrypto/common-hresult-values
#[inline]
#[track_caller]
pub unsafe fn check(result: i32) {
    if result != 0 {
        let mut buf = [0u16; 2048];
        let result = FormatMessageW(
            FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
            null_mut(),
            result as u32,
            0,
            buf.as_mut_ptr(),
            buf.len() as u32,
            null_mut(),
        );
        debug_assert!(result != 0);
        let b = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        if let Some(slice) = &buf.get(..b - 2) {
            let msg = String::from_utf16(slice).unwrap();
            panic!("{msg}");
        } else {
            panic!("Failed to get error message");
        }
    }
}

pub fn devices() -> &'static [Device] {
    unsafe { &DEVICES }
}

pub fn default_device() -> Option<&'static Device> {
    unsafe { DEFAULT_DEVICE.as_ref() }
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
    pub audio_clock_adjust: *mut IAudioClockAdjustment,
    pub render_client: *mut IAudioRenderClient,
    pub format: WAVEFORMATEXTENSIBLE,

    pub buffer: Vec<f32>,

    pub h_event: *mut c_void,
}

impl Wasapi {
    pub unsafe fn new(device: &Device, sample_rate: Option<usize>) -> Self {
        profile!();
        init();

        let audio_client: *mut IAudioClient = {
            let mut audio_client = null_mut();
            check((*device.inner).Activate(
                &IID_IAudioClient,
                CLSCTX_ALL,
                null_mut(),
                &mut audio_client,
            ));

            assert!(!audio_client.is_null());
            audio_client as *mut _
        };

        let mut format = null_mut();
        (*audio_client).GetMixFormat(&mut format);
        let format = &mut *format;

        //Update format to desired sample rate.
        if let Some(sample_rate) = sample_rate {
            assert!(COMMON_SAMPLE_RATES.contains(&sample_rate));
            let sample_rate = sample_rate as u32 / (format.nChannels as u32 / 2);
            format.nSamplesPerSec = sample_rate;
            format.nAvgBytesPerSec = sample_rate * format.nBlockAlign as u32;
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
            panic!("Ouput device has less than 2 channels.");
        }

        let mut default_period = zeroed();
        let mut _min_period = zeroed();
        (*audio_client).GetDevicePeriod(&mut default_period, &mut _min_period);

        check((*audio_client).Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_EVENTCALLBACK
                | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM
                | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY
                | AUDCLNT_STREAMFLAGS_RATEADJUST,
            default_period,
            default_period,
            &format as *const _ as *const WAVEFORMATEX,
            null(),
        ));

        let mut audio_clock_ptr = null_mut();
        check((*audio_client).GetService(&IAudioClockAdjustment::uuidof(), &mut audio_clock_ptr));


        let audio_clock_adjust: *mut IAudioClockAdjustment = transmute(audio_clock_ptr);

        //This must be set for some reason.
        let h_event = CreateEventA(null_mut(), 0, 0, null());
        (*audio_client).SetEventHandle(h_event);

        let mut renderclient_ptr = null_mut();
        check((*audio_client).GetService(&IAudioRenderClient::uuidof(), &mut renderclient_ptr));

        let render_client: *mut IAudioRenderClient = transmute(renderclient_ptr);

        (*audio_client).Start();

        Self {
            audio_client,
            audio_clock_adjust,
            render_client,
            format,
            buffer: Vec::new(),
            h_event,
        }
    }
    pub unsafe fn buffer_frame_count(&mut self) -> usize {
        let mut padding_count = zeroed();
        check((*self.audio_client).GetCurrentPadding(&mut padding_count));

        let mut buffer_frame_count = zeroed();
        check((*self.audio_client).GetBufferSize(&mut buffer_frame_count));

        (buffer_frame_count - padding_count) as usize
    }
    //TODO: This should probably be moved out of the struct.
    pub fn fill_buffer(&mut self, volume: f32, symphonia: &mut Symphonia) {
        profile!();
        unsafe {
            let block_align = self.format.Format.nBlockAlign as usize;
            let buffer_frame_count = self.buffer_frame_count();
            let buffer_size = buffer_frame_count * block_align;

            let channels = self.format.Format.nChannels as usize;
            if channels > 2 {
                //FIXME: Support >2 channels.
                gonk_core::log!("Unsupported output device.");
                return;
            }

            let mut buffer_ptr = null_mut();
            check((*self.render_client).GetBuffer(buffer_frame_count as u32, &mut buffer_ptr));

            let slice = slice::from_raw_parts_mut(buffer_ptr, buffer_size);

            for sample in slice.chunks_mut(4) {
                let sample_bytes = (symphonia.pop().unwrap_or(0.0) * volume).to_le_bytes();
                sample.copy_from_slice(&sample_bytes);
            }

            (*self.render_client).ReleaseBuffer(buffer_frame_count as u32, 0);

            let result = WaitForSingleObject(self.h_event, INFINITE);
            if result != WAIT_OBJECT_0 {
                panic!();
            }
        }
    }
    //It seems like 192_000 & 96_000 Hz are a different grouping than the rest.
    //44100 cannot convert to 192_000 and vise versa.
    #[allow(clippy::result_unit_err)]
    pub fn set_sample_rate(&mut self, new: usize) -> Result<(), ()> {
        debug_assert!(COMMON_SAMPLE_RATES.contains(&new));
        let result = unsafe { (*self.audio_clock_adjust).SetSampleRate(new as f32) };
        if result == 0 {
            Ok(())
        } else {
            Err(())
        }
    }
    pub fn sample_rate(&self) -> usize {
        self.format.Format.nSamplesPerSec as usize
    }
}
