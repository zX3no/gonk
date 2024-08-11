//! TODO: Describe the audio backend
//!
use crossbeam_queue::SegQueue;
use decoder::Symphonia;
use gonk_core::{Index, Song};
use mini::*;
use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::StaticRb;
use std::mem::MaybeUninit;
use std::{
    path::{Path, PathBuf},
    sync::Once,
    thread,
    time::Duration,
};
use symphonia::core::audio::SampleBuffer;
use wasapi::*;

mod decoder;

//TODO: These should be configurable.
const VOLUME_REDUCTION: f32 = 75.0;

//Foobar uses a buffer size of 1000ms by default.
const RB_SIZE: usize = 4096;

const COMMON_SAMPLE_RATES: [u32; 13] = [
    5512, 8000, 11025, 16000, 22050, 32000, 44100, 48000, 64000, 88200, 96000, 176400, 192000,
];

static mut EVENTS: SegQueue<Event> = SegQueue::new();
static mut ELAPSED: Duration = Duration::from_secs(0);
static mut DURATION: Duration = Duration::from_secs(0);
static mut VOLUME: f32 = 15.0 / VOLUME_REDUCTION;
static mut GAIN: Option<f32> = None;
static mut OUTPUT_DEVICE: Option<Device> = None;
static mut PAUSED: bool = false;

//Safety: Only written on decoder thread.
static mut NEXT: bool = false;
static mut SAMPLE_RATE: Option<u32> = None;

static ONCE: Once = Once::new();
static mut ENUMERATOR: MaybeUninit<IMMDeviceEnumerator> = MaybeUninit::uninit();

pub unsafe fn init_com() {
    ONCE.call_once(|| {
        CoInitializeEx(ConcurrencyModel::MultiThreaded).unwrap();
        ENUMERATOR = MaybeUninit::new(IMMDeviceEnumerator::new().unwrap());
    });
}

#[derive(Debug, PartialEq)]
enum Event {
    Stop,
    //Path, Gain
    Song(PathBuf, f32),
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

//https://www.youtube.com/watch?v=zrWYJ6FdOFQ

///Get a list of output devices.
pub fn devices() -> Vec<Device> {
    profile!();
    unsafe {
        init_com();
        let collection = ENUMERATOR
            .assume_init_mut()
            .EnumAudioEndpoints(DataFlow::Render, DeviceState::Active)
            .unwrap();

        (0..collection.GetCount().unwrap())
            .map(|i| {
                let device = collection.Item(i).unwrap();
                Device {
                    name: device.name(),
                    inner: device,
                }
            })
            .collect()
    }
}

///Get the default output device.
pub fn default_device() -> Device {
    profile!();
    unsafe {
        init_com();
        let device = ENUMERATOR
            .assume_init_mut()
            .GetDefaultAudioEndpoint(DataFlow::Render, Role::Console)
            .unwrap();
        Device {
            name: device.name(),
            inner: device,
        }
    }
}

// unsafe fn wcslen(ptr: *mut u16) -> usize {
//     if ptr.is_null() {
//         return 0;
//     }

//     let mut len = 0;
//     let mut current = ptr;

//     while *current != 0 {
//         len += 1;
//         current = current.add(1);
//     }

//     len
// }

pub unsafe fn create_wasapi(
    device: &Device,
    sample_rate: Option<u32>,
) -> (
    IAudioClient,
    IAudioRenderClient,
    WAVEFORMATEXTENSIBLE,
    *mut c_void,
) {
    let client: IAudioClient = device.inner.Activate(ExecutionContext::All).unwrap();
    let mut format =
        (client.GetMixFormat().unwrap() as *const _ as *const WAVEFORMATEXTENSIBLE).read();

    //TODO: How did the old format even work?
    // let fmt = audio_client.GetMixFormat().unwrap();
    // let mut format = if (*fmt).cbSize == 22 && (*fmt).wFormatTag as u32 == WAVE_FORMAT_EXTENSIBLE {
    //     (fmt as *const _ as *const WAVEFORMATEXTENSIBLE).read()
    // } else {
    //     todo!("Unsupported format?");
    // };

    if format.Format.nChannels < 2 {
        todo!("Support mono devices.");
    }

    //Update format to desired sample rate.
    if let Some(sample_rate) = sample_rate {
        assert!(COMMON_SAMPLE_RATES.contains(&sample_rate));
        format.Format.nSamplesPerSec = sample_rate;
        format.Format.nAvgBytesPerSec = sample_rate * format.Format.nBlockAlign as u32;
    }

    let (default, _min) = client.GetDevicePeriod().unwrap();

    client
        .Initialize(
            ShareMode::Shared,
            AUDCLNT_STREAMFLAGS_EVENTCALLBACK
                | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM
                | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
            default,
            default,
            &format as *const _ as *const WAVEFORMATEX,
            None,
        )
        .unwrap();

    //This must be set for some reason.
    let event = CreateEventA(core::ptr::null_mut(), 0, 0, core::ptr::null_mut());
    assert!(!event.is_null());
    client.SetEventHandle(event as isize).unwrap();

    let render_client: IAudioRenderClient = client.GetService().unwrap();
    client.Start().unwrap();

    (client, render_client, format, event)
}

//0.016384MB, no stack overflow here.
// static mut QUEUE: [f32; RB_SIZE] = [0.0; RB_SIZE];

//Should probably just write my own queue.

pub fn spawn_audio_threads(device: Device) {
    unsafe {
        let rb = StaticRb::<f32, RB_SIZE>::default();
        let (mut prod, mut cons) = rb.split();

        thread::spawn(move || {
            info!("Spawned decoder thread!");

            let mut sym: Option<Symphonia> = None;
            let mut leftover_packet: Option<SampleBuffer<f32>> = None;
            let mut i = 0;
            let mut finished = true;

            loop {
                std::thread::sleep(std::time::Duration::from_millis(8));

                match EVENTS.pop() {
                    Some(Event::Song(new_path, gain)) => {
                        info!("{} paused: {}", new_path.display(), PAUSED);
                        // info!("Gain: {} prod capacity: {}", gain, prod.capacity());
                        let s = Symphonia::new(&new_path).unwrap();

                        //We don't set the playback state here because it might be delayed.
                        SAMPLE_RATE = Some(s.sample_rate());
                        DURATION = s.duration();

                        //Set the decoder for the new song.
                        sym = Some(s);

                        //Remove the leftovers.
                        leftover_packet = None;
                        //Start the playback
                        finished = false;

                        //Set the gain
                        GAIN = Some(gain);
                    }
                    Some(Event::Stop) => {
                        info!("Stopping playback.");
                        //Stop the decoder and remove the extra packet.
                        sym = None;
                        leftover_packet = None;
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

                if let Some(p) = &mut leftover_packet {
                    //Note: this has caused a crash before.
                    //This may not work as intended.
                    //Really need to write some unit tests for song playback.
                    //Stability has taken a huge hit since I stopped using it as my primary music player.

                    //Push as many samples as will fit.
                    if let Some(samples) = p.samples().get(i..) {
                        i += prod.push_slice(&samples);
                    } else {
                        i = 0;
                    }

                    //Did we push all the samples?
                    if i == p.len() {
                        i = 0;
                        leftover_packet = None;
                    }
                } else {
                    leftover_packet = sym.next_packet();
                    ELAPSED = sym.elapsed();

                    //It's important that finished is used as a guard.
                    //If next is used it can be changed by a different thread.
                    //This may be an excessive amount of conditions :/
                    if leftover_packet.is_none() && !PAUSED && !finished && !NEXT {
                        finished = true;
                        NEXT = true;
                        info!("Playback ended.");
                    }
                }
            }
        });

        thread::spawn(move || {
            info!("Spawned WASAPI thread!");
            init_com();

            let (mut audio, mut render, mut format, mut event) = create_wasapi(&device, None);
            let mut block_align = format.Format.nBlockAlign as u32;
            let mut sample_rate = format.Format.nSamplesPerSec;
            let mut gain = 0.5;

            loop {
                //Block until the output device is ready for new samples.
                if WaitForSingleObject(event, u32::MAX) != WAIT_OBJECT_0 {
                    unreachable!()
                }

                if let Some(device) = OUTPUT_DEVICE.take() {
                    info!("Changing output device to: {}", device.name);
                    //Set the new audio device.
                    audio.Stop().unwrap();
                    (audio, render, format, event) = create_wasapi(&device, Some(sample_rate));
                    //Different devices have different block alignments.
                    block_align = format.Format.nBlockAlign as u32;
                }

                if let Some(sr) = SAMPLE_RATE {
                    if sr != sample_rate {
                        info!("Changing sample rate to {}", sr);
                        let device = OUTPUT_DEVICE.as_ref().unwrap_or(&device);
                        sample_rate = sr;

                        //Set the new sample rate.
                        audio.Stop().unwrap();
                        (audio, render, format, event) = create_wasapi(device, Some(sample_rate));
                        //Doesn't need to be set since it's the same device.
                        //I just did this to avoid any issues.
                        block_align = format.Format.nBlockAlign as u32;
                    }
                }

                if let Some(g) = GAIN.take() {
                    if gain != g {
                        gain = g;
                    }
                    //Make sure there are no old samples before dramatically increasing the volume.
                    //Without this there were some serious jumps in volume when skipping songs.
                    cons.clear();
                    assert!(cons.is_empty())
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
                let volume = VOLUME * gain;

                let mut iter = cons.pop_iter();

                for bytes in output.chunks_mut(std::mem::size_of::<f32>() * channels) {
                    let sample = iter.next().unwrap_or_default();
                    bytes[0..4].copy_from_slice(&(sample * volume).to_le_bytes());

                    if channels > 1 {
                        let sample = iter.next().unwrap_or_default();
                        bytes[4..8].copy_from_slice(&(sample * volume).to_le_bytes());
                    }
                }

                render.ReleaseBuffer(n_frames, 0).unwrap();
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

//This is mainly for testing.
pub fn play_path<P: AsRef<Path>>(path: P) {
    unsafe {
        PAUSED = false;
        ELAPSED = Duration::from_secs(0);
        EVENTS.push(Event::Song(path.as_ref().to_path_buf(), 0.5));
    }
}

pub fn play_song(song: &Song) {
    unsafe {
        PAUSED = false;
        ELAPSED = Duration::from_secs(0);
        EVENTS.push(Event::Song(
            PathBuf::from(&song.path),
            if song.gain == 0.0 { 0.5 } else { song.gain },
        ));
    }
}

pub fn set_output_device(device: &str) {
    let d = devices();
    unsafe {
        match d.iter().find(|d| d.name == device) {
            Some(device) => OUTPUT_DEVICE = Some(device.clone()),
            None => panic!(
                "Could not find {} in {:?}",
                device,
                d.into_iter().map(|d| d.name).collect::<Vec<String>>()
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
            unsafe { EVENTS.push(Event::Stop) };
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

pub fn clear(songs: &mut Index<Song>) {
    unsafe { EVENTS.push(Event::Stop) };
    songs.clear();
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

//This function should only return `true` after every song has finshed.
pub fn play_next() -> bool {
    unsafe {
        if NEXT {
            NEXT = false;
            true
        } else {
            false
        }
    }
}

pub fn elapsed() -> Duration {
    unsafe { ELAPSED }
}

pub fn duration() -> Duration {
    unsafe { DURATION }
}
