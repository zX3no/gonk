use makepad_windows::{
    core::{Result, PCSTR},
    Win32::{
        Devices::FunctionDiscovery::PKEY_Device_FriendlyName,
        Foundation::{BOOL, HANDLE},
        Media::{Audio::*, KernelStreaming::WAVE_FORMAT_EXTENSIBLE},
        System::{
            Com::{
                CoCreateInstance, CoInitializeEx, StructuredStorage::PROPVARIANT, CLSCTX_ALL,
                COINIT_MULTITHREADED, STGM_READ,
            },
            Threading::CreateEventA,
            Variant::VT_LPWSTR,
        },
    },
};
use std::{
    ops::{Deref, DerefMut},
    sync::Once,
};

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Device {
    pub device: IMMDevice,
    pub name: String,
}

unsafe impl Send for Device {}
unsafe impl Sync for Device {}

impl Deref for Device {
    type Target = IMMDevice;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl DerefMut for Device {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.device
    }
}

static ONCE: Once = Once::new();

#[rustfmt::skip]
const COMMON_SAMPLE_RATES: [u32; 13] = [5512, 8000, 11025, 16000, 22050, 32000, 44100, 48000, 64000, 88200, 96000, 176400, 192000];

static mut ENUMERATOR: Option<IMMDeviceEnumerator> = None;

unsafe fn init() {
    ONCE.call_once(|| {
        CoInitializeEx(None, COINIT_MULTITHREADED).unwrap();
        ENUMERATOR = Some(imm_device_enumerator());

        // let enumerator = imm_device_enumerator();
        // update_output_devices(enumerator);

        //HACK: Not a hack apparently.
        // let ptr: usize = enumerator as usize;
        // thread::spawn(move || {
        //     let enumerator: *mut IMMDeviceEnumerator = ptr as *mut IMMDeviceEnumerator;
        //     loop {
        //         thread::sleep(Duration::from_millis(300));
        //         update_output_devices(enumerator);
        //     }
        // });
    });
}

pub struct Wasapi {
    pub audio_client: IAudioClient,
    pub render_client: IAudioRenderClient,
    pub format: WAVEFORMATEXTENSIBLE,
    pub event: HANDLE,
}

impl Wasapi {
    pub fn new(device: &Device, sample_rate: Option<u32>) -> Result<Self> {
        unsafe {
            init();

            let audio_client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
            let fmt_ptr = audio_client.GetMixFormat()?;
            let fmt = *fmt_ptr;
            let mut format = if fmt.cbSize == 22 && fmt.wFormatTag as u32 == WAVE_FORMAT_EXTENSIBLE
            {
                (fmt_ptr as *const _ as *const WAVEFORMATEXTENSIBLE).read()
            } else {
                // let validbits = wavefmt.wBitsPerSample as usize;
                // let blockalign = wavefmt.nBlockAlign as usize;
                // let samplerate = wavefmt.nSamplesPerSec as usize;
                // let formattag = wavefmt.wFormatTag;
                // let channels = wavefmt.nChannels as usize;
                // let sample_type = match formattag as u32 {
                //     WAVE_FORMAT_IEEE_FLOAT => SampleType::Float,
                //     _ => {
                //         return Err(WasapiError::new("Unsupported format").into());
                //     }
                // };
                // let storebits = 8 * blockalign / channels;
                todo!()
            };

            if format.Format.nChannels < 2 {
                todo!("Support mono devices.");
            }

            //Update format to desired sample rate.
            if let Some(sample_rate) = sample_rate {
                assert!(COMMON_SAMPLE_RATES.contains(&sample_rate));
                format.Format.nSamplesPerSec = sample_rate;
                format.Format.nAvgBytesPerSec = sample_rate * format.Format.nBlockAlign as u32;
            }

            let mut default_period = 0;
            audio_client
                .GetDevicePeriod(Some(&mut default_period), None)
                .unwrap();

            audio_client
                .Initialize(
                    AUDCLNT_SHAREMODE_SHARED,
                    AUDCLNT_STREAMFLAGS_EVENTCALLBACK
                        | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM
                        | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
                    // | AUDCLNT_STREAMFLAGS_RATEADJUST,
                    default_period,
                    default_period,
                    &format as *const _ as *const WAVEFORMATEX,
                    None,
                )
                .unwrap();

            //This must be set for some reason.
            let event = CreateEventA(None, BOOL(0), BOOL(0), PCSTR::null()).unwrap();
            audio_client.SetEventHandle(event).unwrap();

            let render_client: IAudioRenderClient = audio_client.GetService().unwrap();

            audio_client.Start().unwrap();

            Ok(Self {
                audio_client,
                render_client,
                format,
                event,
            })
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.format.Format.nSamplesPerSec
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32, device: &Device) -> Result<()> {
        assert!(COMMON_SAMPLE_RATES.contains(&sample_rate));
        if sample_rate == self.sample_rate() {
            return Ok(());
        }

        //Not sure if this is necessary.
        unsafe { self.audio_client.Stop().unwrap() };

        Ok(*self = Wasapi::new(device, Some(sample_rate))?)
    }
}

///Get a list of output devices.
pub fn devices() -> Vec<Device> {
    unsafe {
        init();
        let collection = ENUMERATOR
            .as_mut()
            .unwrap()
            .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
            .unwrap();

        (0..collection.GetCount().unwrap())
            .into_iter()
            .map(|i| {
                let device = collection.Item(i).unwrap();
                let name = device_name(&device);
                Device { device, name }
            })
            .collect()
    }
}

///Get the default output device.
pub fn default_device() -> Device {
    unsafe {
        init();
        let device = ENUMERATOR
            .as_mut()
            .unwrap()
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .unwrap();
        Device {
            name: device_name(&device),
            device,
        }
    }
}
pub unsafe fn device_name(device: &IMMDevice) -> String {
    let store = device.OpenPropertyStore(STGM_READ).unwrap();
    let name_prop = store.GetValue(&PKEY_Device_FriendlyName).unwrap();
    prop_to_string(name_prop)
}

#[inline]
#[track_caller]
pub unsafe fn prop_to_string(prop: PROPVARIANT) -> String {
    assert!(prop.Anonymous.Anonymous.vt == VT_LPWSTR);
    let data = prop.Anonymous.Anonymous.Anonymous.pwszVal;
    data.to_string().unwrap()
}

#[inline]
pub unsafe fn imm_device_enumerator() -> IMMDeviceEnumerator {
    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).unwrap()
}
