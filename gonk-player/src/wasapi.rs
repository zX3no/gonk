use crate::{Event, State, Symphonia, VOLUME_REDUCTION};
use core::slice;
use crossbeam_channel::Receiver;
use std::ffi::OsString;
use std::mem::{transmute, zeroed};
use std::os::windows::prelude::OsStringExt;
use std::ptr::{null, null_mut};
use std::sync::Once;
use std::thread;
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
use winapi::um::synchapi::CreateEventA;
use winapi::um::unknwnbase::{IUnknown, IUnknownVtbl};
use winapi::um::winbase::{
    FormatMessageW, FORMAT_MESSAGE_FROM_SYSTEM, FORMAT_MESSAGE_IGNORE_INSERTS,
};
use winapi::um::winnt::HRESULT;
use winapi::{Interface, RIDL};

const MAX_BUFFER_SIZE: usize = 1024;
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

pub static mut STATE: State = State::Stopped;
pub static mut ELAPSED: Duration = Duration::from_secs(0);
pub static mut DURATION: Duration = Duration::from_secs(0);
pub static mut VOLUME: f32 = 10.0 / VOLUME_REDUCTION;

unsafe impl Send for Device {}
unsafe impl Sync for Device {}

pub fn init() {
    INIT.call_once(|| unsafe {
        CoInitializeEx(null_mut(), COINIT_MULTITHREADED);

        let mut enumerator: *mut IMMDeviceEnumerator = null_mut();
        let result = CoCreateInstance(
            &CLSID_MMDeviceEnumerator,
            null_mut(),
            CLSCTX_ALL,
            &IMMDeviceEnumerator::uuidof(),
            &mut enumerator as *mut *mut IMMDeviceEnumerator as *mut _,
        );
        check(result).unwrap();

        update_output_devices(enumerator);

        //HACK: Not a hack apparently.
        let ptr: usize = enumerator as usize;
        thread::spawn(move || {
            let enumerator: *mut IMMDeviceEnumerator = ptr as *mut IMMDeviceEnumerator;
            loop {
                update_output_devices(enumerator);
                thread::sleep(Duration::from_millis(200));
            }
        });
    });
}

pub unsafe fn update_output_devices(enumerator: *mut IMMDeviceEnumerator) {
    let mut collection = null_mut();
    let result = (*enumerator).EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE, &mut collection);
    check(result).unwrap();

    let mut count: u32 = zeroed();
    let result = (*collection).GetCount(&mut count as *mut u32 as *const u32);
    check(result).unwrap();

    if count == 0 {
        panic!("No output devices.");
    }

    let mut devices = Vec::new();

    for i in 0..count {
        //Get IMMDevice.
        let mut device = null_mut();
        let result = (*collection).Item(i, &mut device);
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

    //Default default device.
    let mut device: *mut IMMDevice = null_mut();
    let result = (*enumerator).GetDefaultAudioEndpoint(
        eRender,
        eConsole,
        &mut device as *mut *mut IMMDevice,
    );
    //TODO: This can crash when there are no devices.
    check(result).unwrap();

    //Get default device name.
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

    DEFAULT_DEVICE = Some(Device {
        inner: device,
        name,
    });
    DEVICES = devices;
}

///https://docs.microsoft.com/en-us/windows/win32/seccrypto/common-hresult-values
pub unsafe fn check(result: i32) -> Result<(), String> {
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
            Err(msg)
        } else {
            Err("Failed to get error message")?
        }
    } else {
        Ok(())
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

    pub buffer: Vec<u8>,
}

impl Wasapi {
    pub unsafe fn new(device: &Device, sample_rate: Option<u32>) -> Self {
        init();

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

        let mut format = null_mut();
        (*audio_client).GetMixFormat(&mut format);
        let format = &mut *format;

        //Update format to desired sample rate.
        if let Some(sample_rate) = sample_rate {
            assert!(COMMON_SAMPLE_RATES.contains(&sample_rate));
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
        let result =
            (*audio_client).GetService(&IAudioClockAdjustment::uuidof(), &mut audio_clock_ptr);
        check(result).unwrap();
        let audio_clock_adjust: *mut IAudioClockAdjustment = transmute(audio_clock_ptr);

        //This must be set for some reason.
        let h_event = CreateEventA(null_mut(), 0, 0, null());
        (*audio_client).SetEventHandle(h_event);

        let mut renderclient_ptr = null_mut();
        let result =
            (*audio_client).GetService(&IAudioRenderClient::uuidof(), &mut renderclient_ptr);
        check(result).unwrap();
        let render_client: *mut IAudioRenderClient = transmute(renderclient_ptr);

        (*audio_client).Start();

        Self {
            audio_client,
            audio_clock_adjust,
            render_client,
            format,
            buffer: Vec::new(),
        }
    }
    pub unsafe fn buffer_frame_count(&mut self) -> usize {
        let mut padding_count = zeroed();
        let result = (*self.audio_client).GetCurrentPadding(&mut padding_count);
        check(result).unwrap();

        let mut buffer_frame_count = zeroed();
        let result = (*self.audio_client).GetBufferSize(&mut buffer_frame_count);
        check(result).unwrap();

        (buffer_frame_count - padding_count) as usize
    }
    //TODO: This should probably be moved out of the struct.
    pub unsafe fn fill_buffer(&mut self, sym: &mut Symphonia, gain: f32) {
        let block_align = self.format.Format.nBlockAlign as usize;
        let channels = self.format.Format.nChannels as usize;
        let channel_align = block_align / channels;
        let gain = if gain == 0.0 { 0.5 } else { gain };
        let buffer_frame_count = self.buffer_frame_count();

        if buffer_frame_count > self.buffer.len() {
            self.buffer.resize(buffer_frame_count * block_align, 0);
        }

        let mut frames_written = 0;
        while frames_written < buffer_frame_count {
            let frames = (buffer_frame_count - frames_written).min(MAX_BUFFER_SIZE);

            debug_assert!((frames_written + frames) * block_align <= self.buffer.len());
            for out_frame in &mut self.buffer
                [frames_written * block_align..(frames_written + frames) * block_align]
                .chunks_exact_mut(block_align)
            {
                for out_smp_bytes in out_frame.chunks_exact_mut(channel_align) {
                    let smp = sym.next().unwrap_or(0.0) * VOLUME * gain;
                    let smp_bytes = smp.to_le_bytes();
                    debug_assert!(smp_bytes.len() <= out_smp_bytes.len());
                    out_smp_bytes[0..smp_bytes.len()].copy_from_slice(&smp_bytes);
                }
            }

            frames_written += frames;
        }

        // Write the output buffer to the device.
        debug_assert!(self.buffer.len() >= buffer_frame_count * block_align);
        let data = &self.buffer[0..buffer_frame_count * block_align];

        let nbr_bytes = buffer_frame_count * block_align;
        debug_assert_eq!(nbr_bytes, data.len());

        let mut buffer_ptr = null_mut();
        let result = (*self.render_client).GetBuffer(buffer_frame_count as u32, &mut buffer_ptr);
        check(result).unwrap();

        let buffer_slice = slice::from_raw_parts_mut(buffer_ptr, nbr_bytes);
        buffer_slice.copy_from_slice(data);
        (*self.render_client).ReleaseBuffer(buffer_frame_count as u32, 0);
        check(result).unwrap();
    }
    //It seems like 192_000 & 96_000 Hz are a different grouping than the rest.
    //44100 cannot convert to 192_000 and vise versa.
    #[allow(clippy::result_unit_err)]
    pub unsafe fn set_sample_rate(&mut self, new: u32) -> Result<(), ()> {
        debug_assert!(COMMON_SAMPLE_RATES.contains(&new));
        let result = (*self.audio_clock_adjust).SetSampleRate(new as f32);

        match check(result) {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }
}

pub unsafe fn create_decoder(
    path: &str,
    device: &Device,
    decoder: &mut Option<Symphonia>,
    wasapi: &mut Wasapi,
    sample_rate: &mut u32,
) {
    match Symphonia::new(path) {
        Ok(sym) => {
            DURATION = sym.duration();

            let new = sym.sample_rate();
            if *sample_rate != new {
                if wasapi.set_sample_rate(new).is_err() {
                    *wasapi = Wasapi::new(device, Some(new));
                };
                *sample_rate = new;
            }

            *decoder = Some(sym);
        }
        Err(err) => gonk_core::log!("{}", err),
    }
}

//TODO: Devices with 4 channels don't play correctly?
pub unsafe fn new(device: &Device, r: Receiver<Event>) {
    let mut wasapi = Wasapi::new(device, None);
    let mut sample_rate = wasapi.format.Format.nSamplesPerSec;
    let mut decoder: Option<Symphonia> = None;
    let mut gain = 0.50;

    loop {
        if let Ok(event) = r.try_recv() {
            match event {
                Event::PlaySong((path, g)) => {
                    STATE = State::Playing;
                    ELAPSED = Duration::default();
                    if g != 0.0 {
                        gain = g;
                    }
                    create_decoder(&path, device, &mut decoder, &mut wasapi, &mut sample_rate);
                }
                Event::RestoreSong((path, g, elapsed)) => {
                    STATE = State::Paused;
                    ELAPSED = Duration::from_secs_f32(elapsed);
                    if g != 0.0 {
                        gain = g;
                    }
                    create_decoder(&path, device, &mut decoder, &mut wasapi, &mut sample_rate);
                    if let Some(decoder) = &mut decoder {
                        decoder.seek(elapsed);
                    }
                }
                Event::Seek(pos) => {
                    if let Some(decoder) = &mut decoder {
                        decoder.seek(pos);
                    }
                }
                Event::Play => STATE = State::Playing,
                Event::Pause => STATE = State::Paused,
                Event::Stop => {
                    STATE = State::Stopped;
                    decoder = None
                }
                Event::OutputDevice(device) => {
                    let device = if let Some(device) = devices().iter().find(|d| d.name == device) {
                        device
                    } else {
                        unreachable!("Requested a device that does not exist.")
                    };
                    wasapi = Wasapi::new(device, Some(sample_rate));
                }
            }
        }

        //HACK: Don't overwork the thread.
        //Updating the elapsed time is not that important.
        //Filling the buffer here is probably not good.
        thread::sleep(Duration::from_millis(2));

        //Update the elapsed time and fill the output buffer.
        if let State::Playing = STATE {
            if let Some(decoder) = &mut decoder {
                ELAPSED = decoder.elapsed();
                wasapi.fill_buffer(decoder, gain);
            }
        }
    }
}
