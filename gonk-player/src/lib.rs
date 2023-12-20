//! TODO: Describe the audio backend
#![feature(const_float_bits_conv, lazy_cell)]
use crossbeam_queue::SegQueue;
use decoder::Symphonia;
use gonk_core::{Index, Song};
use makepad_windows::core::PCSTR;
use makepad_windows::Win32::{
    Devices::FunctionDiscovery::PKEY_Device_FriendlyName,
    Foundation::WAIT_OBJECT_0,
    Foundation::{BOOL, HANDLE},
    Media::Audio::*,
    Media::{
        Audio::{IAudioClient, IMMDeviceEnumerator, WAVEFORMATEXTENSIBLE},
        KernelStreaming::WAVE_FORMAT_EXTENSIBLE,
    },
    System::{
        Com::{CoCreateInstance, STGM_READ},
        Threading::CreateEventA,
        Variant::VT_LPWSTR,
    },
    System::{
        Com::{CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED},
        Threading::WaitForSingleObject,
    },
};
use mini::*;
use ringbuf::StaticRb;
use std::mem::MaybeUninit;
use std::{
    path::{Path, PathBuf},
    sync::Once,
    thread,
    time::Duration,
};
use symphonia::core::audio::SampleBuffer;

mod decoder;

const VOLUME_REDUCTION: f32 = 150.0;
const RB_SIZE: usize = 4096;
const COMMON_SAMPLE_RATES: [u32; 13] = [
    5512, 8000, 11025, 16000, 22050, 32000, 44100, 48000, 64000, 88200, 96000, 176400, 192000,
];

static mut EVENTS: SegQueue<Event> = SegQueue::new();
static mut ELAPSED: Duration = Duration::from_secs(0);
static mut DURATION: Duration = Duration::from_secs(0);
static mut VOLUME: f32 = 15.0 / VOLUME_REDUCTION;
static mut GAIN: f32 = 0.5;
static mut OUTPUT_DEVICE: Option<Device> = None;
static mut PAUSED: bool = false;

//Safety: Only written on decoder thread.
static mut NEXT: bool = false;
static mut SAMPLE_RATE: Option<u32> = None;

static ONCE: Once = Once::new();
static mut ENUMERATOR: MaybeUninit<IMMDeviceEnumerator> = MaybeUninit::uninit();

pub unsafe fn init_com() {
    ONCE.call_once(|| {
        CoInitializeEx(None, COINIT_MULTITHREADED).unwrap();
        ENUMERATOR =
            MaybeUninit::new(CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).unwrap());
    });
}

#[derive(Debug, PartialEq)]
enum Event {
    Stop,
    Song(PathBuf),
    Seek(f32),
    SeekBackward,
    SeekForward,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Device {
    pub inner: IMMDevice,
    pub name: String,
}

unsafe impl Send for Device {}
unsafe impl Sync for Device {}

///Get a list of output devices.
pub fn devices() -> Vec<Device> {
    unsafe {
        init_com();
        let collection = ENUMERATOR
            .assume_init_mut()
            .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
            .unwrap();

        (0..collection.GetCount().unwrap())
            .map(|i| {
                let device = collection.Item(i).unwrap();
                let name = device_name(&device);
                Device {
                    inner: device,
                    name,
                }
            })
            .collect()
    }
}

///Get the default output device.
pub fn default_device() -> Device {
    unsafe {
        init_com();
        let device = ENUMERATOR
            .assume_init_mut()
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .unwrap();
        Device {
            name: device_name(&device),
            inner: device,
        }
    }
}

pub unsafe fn device_name(device: &IMMDevice) -> String {
    let store = device.OpenPropertyStore(STGM_READ).unwrap();
    let prop = store.GetValue(&PKEY_Device_FriendlyName).unwrap();
    assert!(prop.Anonymous.Anonymous.vt == VT_LPWSTR);
    let data = prop.Anonymous.Anonymous.Anonymous.pwszVal;
    data.to_string().unwrap()
}

pub unsafe fn create_wasapi(
    device: &Device,
    sample_rate: Option<u32>,
) -> (
    IAudioClient,
    IAudioRenderClient,
    WAVEFORMATEXTENSIBLE,
    HANDLE,
) {
    let audio_client: IAudioClient = device.inner.Activate(CLSCTX_ALL, None).unwrap();
    let fmt_ptr = audio_client.GetMixFormat().unwrap();
    let fmt = *fmt_ptr;
    let mut format = if fmt.cbSize == 22 && fmt.wFormatTag as u32 == WAVE_FORMAT_EXTENSIBLE {
        (fmt_ptr as *const _ as *const WAVEFORMATEXTENSIBLE).read()
    } else {
        todo!("Unsupported format?");
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

    (audio_client, render_client, format, event)
}

pub fn spawn_audio_threads(device: Device) {
    unsafe {
        let rb = StaticRb::<f32, RB_SIZE>::default();
        let (mut prod, mut cons) = rb.split();

        thread::spawn(move || {
            info!("Spawned decoder thread!");

            let mut sym: Option<Symphonia> = None;
            let mut packet: Option<SampleBuffer<f32>> = None;
            let mut i = 0;

            loop {
                std::thread::sleep(std::time::Duration::from_millis(1));

                match EVENTS.pop() {
                    Some(Event::Song(new_path)) => {
                        info!("{} paused: {}", new_path.display(), PAUSED);
                        let s = Symphonia::new(&new_path).unwrap();
                        //We don't set the playback state here because it might be delayed.
                        SAMPLE_RATE = Some(s.sample_rate());
                        DURATION = s.duration();
                        NEXT = false;
                        sym = Some(s);
                    }
                    Some(Event::Stop) => {
                        info!("Stopping playback.");
                        sym = None;
                        //Don't stop playback here because it might be delayed.
                    }
                    Some(Event::Seek(pos)) => {
                        if let Some(sym) = &mut sym {
                            info!(
                                "Seeking {} / {} paused: {}",
                                pos as u32,
                                DURATION.as_secs_f32() as u32,
                                PAUSED
                            );
                            sym.seek(pos);
                        }
                    }
                    Some(Event::SeekForward) => {
                        if let Some(sym) = &mut sym {
                            info!(
                                "Seeking {} / {}",
                                sym.elapsed().as_secs_f32() + 10.0,
                                sym.duration().as_secs_f32()
                            );
                            sym.seek((sym.elapsed().as_secs_f32() + 10.0).clamp(0.0, f32::MAX))
                        }
                    }
                    Some(Event::SeekBackward) => {
                        if let Some(sym) = &mut sym {
                            info!(
                                "Seeking {} / {}",
                                sym.elapsed().as_secs_f32() - 10.0,
                                sym.duration().as_secs_f32()
                            );
                            sym.seek((sym.elapsed().as_secs_f32() - 10.0).clamp(0.0, f32::MAX))
                        }
                    }
                    None => {}
                }

                if PAUSED {
                    continue;
                }

                let Some(sym) = &mut sym else {
                    continue;
                };

                if let Some(p) = &mut packet {
                    i += prod.push_slice(&p.samples()[i..]);
                    if i == p.len() {
                        i = 0;
                        packet = None;
                    }

                    if let Some(device) = &OUTPUT_DEVICE {
                        info!("Changing output device {}", device.name);
                        OUTPUT_DEVICE = None;
                    }
                } else {
                    packet = sym.next_packet();
                    ELAPSED = sym.elapsed();

                    if packet.is_none() && !PAUSED {
                        info!("Playback ended.");
                        NEXT = true;
                    }
                }
            }
        });

        thread::spawn(move || {
            info!("Spawned WASAPI thread!");
            init_com();

            let (mut audio, mut render, mut format, mut event) = create_wasapi(&device, None);
            let block_align = format.Format.nBlockAlign as u32;
            let mut sample_rate = format.Format.nSamplesPerSec;

            loop {
                //I wish there was a better fix for this.
                if cons.is_empty() {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                    continue;
                }

                if let Some(sr) = SAMPLE_RATE {
                    if sr != sample_rate {
                        info!("Changing sample rate to {}", sr);
                        let device = OUTPUT_DEVICE.as_ref().unwrap_or(&device);
                        audio.Stop().unwrap();
                        (audio, render, format, event) = create_wasapi(device, Some(sr));
                        sample_rate = sr;
                    }
                }

                //Sample-rate probably changed if this fails.
                let padding = audio.GetCurrentPadding().unwrap();
                let buffer_size = audio.GetBufferSize().unwrap();

                let n_frames = buffer_size - 1 - padding;
                assert!(n_frames < buffer_size - padding);

                let size = (n_frames * block_align) as usize;

                if size == 0 {
                    continue;
                }

                let b = render.GetBuffer(n_frames).unwrap();
                let output = std::slice::from_raw_parts_mut(b, size);
                let channels = format.Format.nChannels as usize;
                let volume = VOLUME * GAIN;

                for bytes in output.chunks_mut(std::mem::size_of::<f32>() * channels) {
                    let Some(sample) = cons.pop() else {
                        break;
                    };

                    bytes[0..4].copy_from_slice(&(sample * volume).to_le_bytes());

                    if channels > 1 {
                        let Some(sample) = cons.pop() else {
                            break;
                        };

                        bytes[4..8].copy_from_slice(&(sample * volume).to_le_bytes());
                    }
                }

                render.ReleaseBuffer(n_frames, 0).unwrap();

                if WaitForSingleObject(event, u32::MAX) != WAIT_OBJECT_0 {
                    unreachable!()
                }
            }
        });
    }
}

pub fn toggle_playback() {
    unsafe { PAUSED = !PAUSED };
}

pub fn play() {
    unsafe { PAUSED = false };
}

pub fn pause() {
    unsafe { PAUSED = true };
}

pub fn toggle_mute() {
    todo!();
}

pub fn get_volume() -> u8 {
    unsafe { (VOLUME * VOLUME_REDUCTION) as u8 }
}

pub fn set_volume(volume: u8) {
    unsafe {
        VOLUME = volume as f32 / VOLUME_REDUCTION;
    }
}

pub fn volume_up() {
    unsafe {
        VOLUME = (VOLUME * VOLUME_REDUCTION + 5.0).clamp(0.0, 100.0) / VOLUME_REDUCTION;
    }
}

pub fn volume_down() {
    unsafe {
        VOLUME = (VOLUME * VOLUME_REDUCTION - 5.0).clamp(0.0, 100.0) / VOLUME_REDUCTION;
    }
}

pub fn seek(pos: f32) {
    unsafe {
        EVENTS.push(Event::Seek(pos));
        ELAPSED = Duration::from_secs_f32(pos);
    }
}

pub fn seek_foward() {
    unsafe { EVENTS.push(Event::SeekForward) };
}

pub fn seek_backward() {
    unsafe { EVENTS.push(Event::SeekBackward) };
}

pub fn play_path<P: AsRef<Path>>(path: P) {
    unsafe {
        ELAPSED = Duration::from_secs(0);
        EVENTS.push(Event::Song(path.as_ref().to_path_buf()));
    }
}

pub fn play_song(song: &Song) {
    unsafe {
        GAIN = if song.gain != 0.0 { 0.5 } else { song.gain };
        PAUSED = false;
        ELAPSED = Duration::from_secs(0);
        EVENTS.push(Event::Song(PathBuf::from(&song.path)));
    }
}

pub fn stop() {
    unsafe { EVENTS.push(Event::Stop) };
}

pub fn set_output_device(device: &str) {
    let d = devices();
    unsafe {
        match d.into_iter().find(|d| d.name == device) {
            Some(device) => OUTPUT_DEVICE = Some(device),
            None => panic!(
                "Could not find {} in {:?}",
                device,
                devices()
                    .into_iter()
                    .map(|d| d.name)
                    .collect::<Vec<String>>()
            ),
        }
    }
}

pub fn play_index(songs: &mut Index<Song>, i: usize) {
    songs.select(Some(i));
    if let Some(song) = songs.selected() {
        play_song(song);
    }
}

pub fn delete(songs: &mut Index<Song>, index: usize) {
    if songs.is_empty() {
        return;
    }

    songs.remove(index);

    if let Some(playing) = songs.index() {
        let len = songs.len();
        if len == 0 {
            *songs = Index::default();
            stop();
        } else if index == playing && index == 0 {
            songs.select(Some(0));
            if let Some(song) = songs.selected() {
                play_song(song);
            }
        } else if index == playing && index == len {
            songs.select(Some(len - 1));
            if let Some(song) = songs.selected() {
                play_song(song);
            }
        } else if index < playing {
            songs.select(Some(playing - 1));
        }
    };
}

pub fn clear_except_playing(songs: &mut Index<Song>) {
    if let Some(index) = songs.index() {
        let playing = songs.remove(index);
        *songs = Index::new(vec![playing], Some(0));
    }
}

pub fn is_paused() -> bool {
    unsafe { PAUSED }
}

pub fn play_next() -> bool {
    unsafe { NEXT }
}

pub fn elapsed() -> Duration {
    unsafe { ELAPSED }
}

pub fn duration() -> Duration {
    unsafe { DURATION }
}
