#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unused)]
use std::{
    thread::JoinHandle,
    time::{Duration, Instant},
};

use mu_core::{Database, SongId, db::Artwork};
use neoui::*;

pub mod sidebar;
pub use sidebar::*;
pub mod controls;
pub use controls::*;
pub mod library;
pub use library::*;
pub mod settings;
pub use settings::*;
pub mod queue;
pub use queue::*;

pub const GRAY_50: u32 = hex("#f9f6f4");
pub const GRAY_100: u32 = hex("#ede9e5");
pub const GRAY_200: u32 = hex("#cbc7c4");
pub const GRAY_300: u32 = hex("#aaa6a2");
pub const GRAY_400: u32 = hex("#827f7c");
pub const GRAY_500: u32 = hex("#5c5957");
pub const GRAY_600: u32 = hex("#403f3e");
pub const GRAY_700: u32 = hex("#2a2929");
pub const GRAY_800: u32 = hex("#1b1b1c");
pub const GRAY_850: u32 = hex("#201f20");
pub const GRAY_900: u32 = hex("#101011");
pub const GRAY_950: u32 = hex("#0b0b0c");

pub const ACCENT_50: u32 = hex("#f6f3ff");
pub const ACCENT_100: u32 = hex("#e6e0fd");
pub const ACCENT_200: u32 = hex("#cdc2f4");
pub const ACCENT_300: u32 = hex("#b9a8ee");
pub const ACCENT_400: u32 = hex("#ad98e2");
pub const ACCENT_500: u32 = hex("#9b84d9");
pub const ACCENT_600: u32 = hex("#8871c6");
pub const ACCENT_700: u32 = hex("#69559b");
pub const ACCENT_800: u32 = hex("#463968");
pub const ACCENT_900: u32 = hex("#261f39");
pub const ACCENT_950: u32 = hex("#100d1b");

pub const GRAY_100_A4: u32 = hex("#ede9e509");
pub const GRAY_100_A8: u32 = hex("#ede9e512");
pub const GRAY_100_A10: u32 = hex("#ede9e51a");
pub const GRAY_100_A15: u32 = hex("#ede9e51f");
pub const GRAY_100_A60: u32 = hex("#ede9e599");
pub const GRAY_100_A65: u32 = hex("#ede9e5a6");
pub const ACCENT_A22: u32 = hex("#9b84d938");

pub const BODY: u32 = GRAY_950;
pub const SIDEBAR: u32 = GRAY_900;
pub const ROW_SELECTED: u32 = GRAY_850;
pub const BORDER_DIM: u32 = GRAY_800;
pub const TEXT_FAINT: u32 = GRAY_600;
pub const TEXT_MUTED: u32 = GRAY_500;
pub const TEXT_TERTIARY: u32 = GRAY_100_A60;
pub const PLAY_HOVER: u32 = GRAY_300;
pub const TEXT_SECONDARY: u32 = GRAY_200;
pub const TEXT: u32 = GRAY_100;
pub const KNOB: u32 = GRAY_100;

pub const BORDER: u32 = GRAY_100_A10;
pub const BORDER_SUBTLE: u32 = GRAY_100_A8;
pub const TRACK_EMPTY: u32 = GRAY_100_A15;
pub const TRACK_FILL: u32 = GRAY_100_A65;
pub const ROW_HOVER: u32 = GRAY_100_A4;

pub const ACCENT: u32 = ACCENT_500;
pub const ACCENT_HOVER: u32 = ACCENT_400;
pub const ACCENT_PRESSED: u32 = ACCENT_600;
pub const ACCENT_SOFT: u32 = ACCENT_A22;

const ALPHABET: &[&str] = &[
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z",
];

fn icon(ui: &mut FrameContext, kind: &str, style: RectStyle) -> State {
    ui.widget(24, 24, &style.layout, &style.paint, |ui, r, _, depth| {
        let fill = style.paint.fg.unwrap_or(white());
        let palette = [fill, with_alpha(fill, 110), SIDEBAR];
        let u = |v: i32| v * r.width / 24;

        #[rustfmt::skip]
        let bars: &[[i32; 5]] = match kind {
            "Panel"    => &[[2,4,20,16,0], [4,6,16,12,2], [4,6,5,12,0]],
            "Library"  => &[[3,4,4,17,0], [9,4,4,17,0], [15,6,4,15,0], [19,7,3,14,1]],
            "Queue"    => &[[3,6,12,2,0], [3,11,12,2,0], [3,16,8,2,1]],
            "Playlist" => &[[3,6,15,2,0], [3,11,15,2,0], [3,16,9,2,1], [15,16,7,2,0], [17,14,2,7,0]],
            "Settings" => &[[5,3,2,18,1], [12,3,2,18,1], [19,3,2,18,1], [3,7,6,3,0], [10,13,6,3,0], [17,6,6,3,0]],
            "Shuffle"  => &[[4,7,13,2,0], [7,15,13,2,0]],
            "Pause"    => &[[7,4,4,16,0], [13,4,4,16,0]],
            "Volume"   => &[[3,9,4,6,0], [14,9,2,6,1]],
            _ => &[],
        };

        for &[x, y, w, h, c] in bars {
            let bar = Rect::new(r.x + u(x), r.y + u(y), u(w).max(1), u(h).max(1));
            ui.paint_rect(bar, rect().bg(palette[c as usize]).radius(1).depth(depth));
        }

        let (x, y) = (r.x, r.y);
        let tri = |ui: &mut FrameContext, a: (i32, i32), b: (i32, i32), c: (i32, i32)| {
            let p = |(px, py): (i32, i32)| (x + u(px), y + u(py));
            ui.paint_triangle(p(a), p(b), p(c), rect().bg(fill).depth(depth));
        };
        match kind {
            "Queue" => tri(ui, (15, 11), (15, 20), (22, 15)),
            "Shuffle" => {
                tri(ui, (16, 4), (16, 12), (21, 8));
                tri(ui, (8, 12), (8, 20), (3, 16));
            }
            "Play" => tri(ui, (6, 3), (6, 21), (21, 12)),
            "Rewind" => {
                tri(ui, (11, 3), (11, 21), (1, 12));
                tri(ui, (22, 3), (22, 21), (12, 12));
            }
            "Forward" => {
                tri(ui, (13, 3), (13, 21), (23, 12));
                tri(ui, (2, 3), (2, 21), (12, 12));
            }
            "Volume" => tri(ui, (11, 3), (11, 21), (6, 12)),
            "Repeat" => {
                let ring = Rect::new(x + u(3), y + u(3), u(18), u(18));
                let stroke = rect()
                    .border(fill)
                    .border_thickness(u(2).max(1) as usize)
                    .radius(u(9).max(1) as usize)
                    .depth(depth);
                //Draw the ring twice, clipped, so everything but the top right is covered.
                ui.clipped(Rect::new(x, y, u(12), u(24)), |ui| ui.paint_rect(ring, stroke));
                ui.clipped(Rect::new(x + u(12), y + u(9), u(12), u(15)), |ui| {
                    ui.paint_rect(ring, stroke)
                });
                tri(ui, (12, 1), (12, 7), (18, 4));
            }
            _ => {}
        }
    })
}

fn time<'a>(ui: &mut FrameContext<'_, 'a>, t: f32) -> &'a str {
    let total_seconds = t.max(0.0) as u32;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    ui.fmt(format_args!("{:02}:{:02}", minutes, seconds))
}

fn target_artwork(db: &Database, artist: &str) -> Vec<(usize, String)> {
    let mut targets = Vec::new();
    let artist = db.artist_by_name(artist).unwrap();
    for alb in &db.albums[artist.albums.clone()] {
        let first_song = &db.songs[alb.songs.start];
        if first_song.artwork.is_none() {
            targets.push((alb.songs.start, first_song.path.clone()));
        }
    }
    targets
}

fn spawn_load_artwork(targets: Vec<(usize, String)>) -> JoinHandle<Vec<(usize, Artwork)>> {
    std::thread::spawn(move || {
        use rayon::prelude::*;
        let now = Instant::now();
        let loaded: Vec<(usize, Artwork)> = targets
            .into_par_iter()
            .filter_map(|(idx, path)| {
                if let Ok(s) = onmi::metadata(&path, false, true)
                    && let Some(artwork) = s.artwork
                    && let Ok((pixels, width, height)) = image::decode(&artwork.data)
                {
                    let size = 512;
                    let pixels = image::resize(
                        Image {
                            pixels: &pixels,
                            width,
                            height,
                        },
                        size,
                        size,
                    );
                    Some((idx, Artwork::Decoded(pixels.into_boxed_slice(), size, size)))
                } else {
                    None
                }
            })
            .collect();

        println!(
            "Loaded {} artworks in {}ms",
            loaded.len(),
            now.elapsed().as_millis()
        );
        loaded
    })
}

//TODO: Add tailwind style font size and padding builders.
//Allow the user to customize them.
//Currently keeping track of all the sizings is very difficult.
fn main() {
    defer_results!();

    // let config = mu_core::config_paths();
    // let s = mu_core::db::create("/Users/bay/Music/gdrive", config.database);
    // s.join().unwrap();

    let now = Instant::now();
    let player = std::thread::spawn(move || {
        let now = Instant::now();
        let outputs = onmi::OutputDevices::new();
        let player = onmi::Player::new(outputs.default_device());
        println!("Loaded Player in {}ms", now.elapsed().as_millis());
        player
    });

    let mut font = Some(std::thread::spawn(|| {
        let now = Instant::now();
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fonts")
            .join("NotoSansCJK-Subset.otf");
        let font = std::fs::read(path).unwrap();
        let f = fontdue::Font::from_bytes(font, fontdue::FontSettings::default()).unwrap();
        println!("Loaded Font in {}ms", now.elapsed().as_millis());
        f
    }));

    let db = std::thread::spawn(|| {
        let now = Instant::now();
        let config = mu_core::config_paths();
        let db = mu_core::vdb::Database::new(&config.database);
        let mut artists = db.get_artists();
        artists.sort_by_key(|a| a.to_ascii_lowercase());
        println!("Loaded DB in {}ms", now.elapsed().as_millis());
        (db, artists)
    });

    let mut ui = ui("mu", 1200, 780);
    ui.default_font_size = 16;
    // ui.debug_damage = true;

    let mut player = player.join().unwrap();
    let (mut db, artists) = db.join().unwrap();

    let mut artwork_task: Option<JoinHandle<Vec<(usize, Artwork)>>> = None;
    let initial_targets = target_artwork(&db, "Duster");
    if !initial_targets.is_empty() {
        artwork_task = Some(spawn_load_artwork(initial_targets));
    }

    println!("Loaded {}ms", now.elapsed().as_millis());

    let mut sidebar = Sidebar {
        bounds: Rect::default(),
        artists: &artists,
        selected_artist: "Duster",
        artist_scroll: Scroll::new(),
        active: true,
        update_library: false,
        selected_mode: "Queue",
        current_letter: None,
        jump_to_letter: None,
    };

    let initial_songs = db
        .artist_by_name("Duster")
        .map(|a| a.songs.clone())
        .unwrap();

    let mut library = Library {
        scroll: Scroll::new(),
        bounds: Rect::default(),
        total_tracks: initial_songs.len(),
        artist: "Duster",
        playing_song: None,
        selected_song: None,
        update_playing: false,
    };

    let mut controls = Controls {
        current_song: None,
        bounds: Rect::default(),
        playing: false,
        shuffle: false,
        repeat: false,
        muted: false,
        elapsed: 0.0,
        duration: 0.0,
        volume: player.volume(),
    };

    let mut queue = Queue {
        songs: initial_songs.collect(),
        playing_song: None,
        bounds: Rect::default(),
        scroll: Scroll::new(),
        drag: None,
        playing_artist: Some("Duster"),
    };

    while ui.window.open() {
        if ui.window.pressed(Key::Escape) {
            ui.window.close();
        }

        for key in ui.window.pressed_keys() {
            match *key {
                Key::Char('1') => sidebar.selected_mode = "Library",
                Key::Char('2') => sidebar.selected_mode = "Queue",
                Key::Char('3') => sidebar.selected_mode = "Playlist",
                Key::Char('4') => sidebar.selected_mode = "Settings",
                Key::Tab => sidebar.active = !sidebar.active,
                //TODO: Should only trigger when library is focused.
                //Sidebar jumps on alphabet key press.
                Key::Char('W') => {
                    player.volume_up();
                    controls.volume = player.volume();
                }
                Key::Char('S') => {
                    player.volume_down();
                    controls.volume = player.volume()
                }
                Key::Char('E') => player.seek_forward(10.0),
                Key::Char('Q') => player.seek_backward(10.0),
                Key::Char('A') if let Some(current) = queue.playing_song => {
                    prev(current, &mut queue, &db, &mut player);
                    if let Some(idx) = queue.playing_song {
                        let song_id = queue.songs[idx];
                        controls.current_song = Some(song_id);
                        library.playing_song = Some(song_id);
                    }
                }
                Key::Char('D') if let Some(current) = queue.playing_song => {
                    next(current, &mut queue, &db, &mut player);
                    if let Some(idx) = queue.playing_song {
                        let song_id = queue.songs[idx];
                        controls.current_song = Some(song_id);
                        library.playing_song = Some(song_id);
                    }
                }
                Key::Space => {
                    player.toggle_playback();
                    controls.playing = !controls.playing
                }
                _ => {}
            }
        }

        if let Some(handle) = &artwork_task
            && handle.is_finished()
        {
            let loaded = artwork_task.take().unwrap().join().unwrap();
            for (song_idx, artwork) in loaded {
                if let Some(song) = db.songs.get_mut(song_idx) {
                    song.artwork = Some(artwork);
                }
            }
        }

        if let Some(f) = &font {
            if f.is_finished() {
                let font = font.take().unwrap().join().unwrap();
                ui.add_font_fallback(font);
            }
        }

        if library.update_playing {
            library.update_playing = false;
            let Some(song_id) = library.playing_song else {
                break;
            };
            let Some(song) = db.song(song_id) else {
                break;
            };

            player.play_song(&song.path, Some(song.gain), true);
            controls.current_song = Some(song_id);
            controls.playing = true;

            if queue.playing_artist != Some(library.artist) {
                queue.playing_artist = Some(library.artist);
                let artist_entry = db.artist_by_name(library.artist).unwrap();
                queue.songs = artist_entry.songs.clone().collect();
            }

            queue.playing_song = queue.songs.iter().position(|&id| id == song_id);
        }

        if sidebar.update_library {
            sidebar.update_library = false;
            sidebar.selected_mode = "Library";
            library.playing_song = None;
            library.selected_song = None;

            let artist = sidebar.selected_artist;
            let artist_entry = db.artist_by_name(artist).unwrap();
            library.artist = artist;
            library.total_tracks = artist_entry.songs.len();
            library.scroll = Scroll::new();

            if let Some(current_id) = controls.current_song {
                if artist_entry.songs.contains(&current_id) {
                    library.playing_song = Some(current_id);
                }
            }

            let targets = target_artwork(&db, artist);
            if !targets.is_empty() {
                artwork_task = Some(spawn_load_artwork(targets));
            }
        }

        if player.is_finished()
            && let Some(current) = queue.playing_song
        {
            next(current, &mut queue, &db, &mut player);
            if let Some(idx) = queue.playing_song {
                let song_id = queue.songs[idx];
                controls.current_song = Some(song_id);
                library.playing_song = Some(song_id);
            }
        }

        controls.duration = player.duration().as_secs_f32();
        controls.elapsed = player.elapsed().as_secs_f32();

        ui.frame(|ui| {
            let target = if sidebar.active { 280.0 } else { 56.0 };
            let width = ui.animate_f32(target, 0.15, Ease::OutCubic) as i32;
            let (sb, body) = ui.split_h(width);
            sidebar.bounds = sb;

            if width > 168 {
                draw_sidebar(&mut sidebar, ui);
            } else {
                draw_rail(&mut sidebar, ui);
            }

            let (body, con) = ui.split_rect_v(body, -84);
            library.bounds = body;
            queue.bounds = body;
            controls.bounds = con;

            match sidebar.selected_mode {
                "Library" => draw_library(&db, &mut library, ui),
                "Queue" => draw_queue(ui, &mut queue, &db),
                "Playlist" => {}
                "Settings" => {}
                _ => unreachable!(),
            }

            draw_controls(&mut controls, &mut player, ui, &db);
        });
    }
}
