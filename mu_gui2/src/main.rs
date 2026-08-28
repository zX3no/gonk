#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unused)]
use std::{
    thread::JoinHandle,
    time::{Duration, Instant},
};

use mu_core::{Album, db::Artwork};
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

fn spawn_load_artwork(artist: String, mut albums: Vec<Album>) -> JoinHandle<(String, Vec<Album>)> {
    std::thread::spawn(move || {
        let now = Instant::now();
        // let threads = std::thread::available_parallelism().map_or(16, |n| n.get());
        let threads = 16;
        let chunk = albums.len().div_ceil(threads).max(1);

        std::thread::scope(|scope| {
            for albums in albums.chunks_mut(chunk) {
                scope.spawn(move || {
                    for a in albums {
                        //Use the first song for the whole album.
                        //Technically each track can have a different album cover.
                        if let Some(first) = a.songs.first_mut()
                            && first.artwork.is_none()
                            && let Ok(s) = onmi::metadata(&first.path, false, true)
                            && let Some(artwork) = s.artwork
                        {
                            //TODO: The point of the thumbnails is to allow users to cache a downscaled version
                            //Currently even though the image is being rendered at 120x120px.
                            //We need a high resolution version stored ???
                            if let Ok((pixels, width, height)) = image::decode(&artwork.data) {
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
                                first.artwork =
                                    Some(Artwork::Decoded(pixels.into_boxed_slice(), size, size));
                            }
                        }
                    }
                });
            }
        });

        println!("Loaded {artist} in {}ms", now.elapsed().as_millis());

        (artist, albums)
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
        let mut artists: Vec<String> = db.btree.keys().cloned().collect();
        artists.sort_by_key(|a| a.to_ascii_lowercase());
        println!("Loaded DB in {}ms", now.elapsed().as_millis());
        (db, artists)
    });

    let mut ui = ui("mu", 1200, 780);
    ui.default_font_size = 16;
    // ui.debug_damage = true;

    let mut player = player.join().unwrap();
    let (mut db, artists) = db.join().unwrap();

    let mut artwork_task: Option<JoinHandle<(String, Vec<Album>)>> = None;

    if let Some(albums) = db.btree.get("Duster").cloned() {
        artwork_task = Some(spawn_load_artwork("Duster".to_string(), albums));
    }

    println!("Loaded {}ms", now.elapsed().as_millis());

    let mut sidebar = Sidebar {
        bounds: Rect::default(),
        selected_artist: "Duster",
        artists: &artists,
        artist_scroll: Scroll::new(),
        active: true,
        // update_library: true,
        // selected_mode: "Library",
        update_library: false,
        selected_mode: "Queue",
        current_letter: None,
        jump_to_letter: None,
    };

    let mut library = Library {
        scroll: Scroll::new(),
        bounds: Rect::default(),
        total_tracks: db
            .albums_by_artist("Duster")
            .iter()
            .map(|a| a.songs.len())
            .sum(),
        artist: "Duster",
        playing_song: None,
        selected_song: None,
        update_playing: false,
    };

    let mut controls = Controls {
        song: None,
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
        // songs: db.albums_by_artist("Duster")[0].songs.clone(),
        songs: db
            .albums_by_artist("Duster")
            .iter()
            .map(|a| a.songs.clone())
            .flatten()
            .collect(),
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
                    prev(current, &mut queue, &mut player);
                }
                Key::Char('D') if let Some(current) = queue.playing_song => {
                    next(current, &mut queue, &mut player);
                }
                Key::Space => {
                    player.toggle_playback();
                    controls.playing = !controls.playing
                }
                _ => {}
            }
        }

        if let Some(handle) = &artwork_task {
            if handle.is_finished() {
                let (artist, albums) = artwork_task.take().unwrap().join().unwrap();
                db.btree.insert(artist, albums);
            }
        }

        if let Some(f) = &font {
            if f.is_finished() {
                let font = font.take().unwrap().join().unwrap();
                ui.add_font_fallback(font);
            }
        }

        //Update the queue, library and controls.

        //Currently the queue and library can store the same artist list.
        //But it's fractured, one loops albums from the db the other has a cloned list of songs.
        //Not sure how I can unify this to simplify the program structure...?
        //I will probably just rewrite the database to be linear.
        //We need to invalidate playback when rebuilding the database anyway.
        //Unless we want to append changes, which...I should probably implement that too.
        if library.update_playing {
            library.update_playing = false;

            let (ai, si) = library.selected_song.unwrap();
            let albums = db.albums_by_artist(library.artist);

            // How can we unify / simplify this a bit more?
            // queue.playing_song = todo!();

            //User is playing a different artist now.
            if queue.playing_artist != Some(library.artist) {
                queue.playing_artist = Some(library.artist);
                queue.songs = albums
                    .iter()
                    .flat_map(|a| a.songs.iter().cloned())
                    .collect();
            }

            let album = &albums[ai];
            let mut song = album.songs[si].clone();
            player.play_song(&song.path, Some(song.gain), true);

            //Update the active queue song.
            let idx = queue.songs.iter().position(|s| s == &song).unwrap();
            queue.playing_song = Some(idx);

            // Assume first track artwork is preloaded.
            // TODO: Right now we iterate the db everytime instead.
            // song.artwork = album.songs[0].artwork.clone();

            controls.song = Some((song, ai, si));
            controls.playing = true;
        }

        //It's not as immediate, but easier than passing in db and library into sidebar.
        if sidebar.update_library {
            sidebar.update_library = false;
            sidebar.selected_mode = "Library";
            library.playing_song = None;
            library.selected_song = None;

            let artist = sidebar.selected_artist.to_string();

            //Restore the playing song in the library when going back to an artist.
            if let Some((song, ai, si)) = &controls.song
                && song.artist == artist
            {
                library.playing_song = Some((*ai, *si));
            }

            if let Some(albums) = db.btree.get(&artist).cloned() {
                artwork_task = Some(spawn_load_artwork(artist, albums));
            }

            library.scroll = Scroll::new();
            library.artist = sidebar.selected_artist;
            library.total_tracks = db
                .albums_by_artist(sidebar.selected_artist)
                .iter()
                .map(|a| a.songs.len())
                .sum();
        }

        if player.is_finished()
            && let Some(current) = queue.playing_song
        {
            next(current, &mut queue, &mut player);
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
                "Library" => draw_library(db.albums_by_artist(library.artist), &mut library, ui),
                "Queue" => draw_queue(ui, &mut queue, &db),
                "Playlist" => {}
                "Settings" => {}
                _ => unreachable!(),
            }

            draw_controls(&mut controls, &mut player, ui, &db);
        });
    }
}
