use mu_core::{vdb::*, *};
use neoui::*;
use onmi::{OutputDevices, Player};
use std::{
    fs,
    time::{Duration, Instant},
};

mod artist;
mod command_palette;
mod context_menu;
mod player_bar;
mod playlist;
mod queue;
mod search;
mod selection;
mod settings;
mod sidebar;
mod theme;
mod toast;

use command_palette::CommandPalette;
use context_menu::{ContextMenu, MenuCommand};
use search::Search;
use selection::PathSelection;
use theme::colors;
use toast::Toast;

const PLAYER_H: i32 = player_bar::PLAYER_H;
const SIDEBAR_W: i32 = sidebar::SIDEBAR_W;

#[derive(PartialEq, Eq, Clone)]
pub enum Mode {
    Queue,
    Search,
    Playlist,
    PlaylistDetail { name: String },
    Artist { name: String },
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RepeatMode {
    Off,
    All,
    One,
}

fn path(mut path: String) -> Option<std::path::PathBuf> {
    if path.contains('~') {
        path = path.replace('~', &user_profile_directory().unwrap());
    }
    fs::canonicalize(path).ok()
}

fn play(player: &mut Player, song: &Song, start: bool) {
    if let Err(e) = player.play_song(
        &song.path,
        if song.gain == 0.0 {
            Some(0.5)
        } else {
            Some(song.gain)
        },
        start,
    ) {
        log!("{e}");
    }
}

/// Start a playback session. Does **not** modify the explicit queue.
fn start_playback(
    player: &mut Player,
    playback: &mut Index<Song>,
    list: Vec<Song>,
    index: usize,
) {
    if list.is_empty() {
        return;
    }
    let idx = index.min(list.len() - 1);
    *playback = Index::new(list, Some(idx));
    if let Some(song) = playback.selected().cloned() {
        play(player, &song, true);
    }
}

/// Append to the explicit queue only — never starts or changes playback.
fn append_queue(queue: &mut Index<Song>, list: Vec<Song>) -> usize {
    if list.is_empty() {
        return 0;
    }
    let n = list.len();
    queue.extend(list);
    n
}

fn append_toast(toast: &mut Option<Toast>, n: usize) {
    if n == 0 {
        return;
    }
    let message = if n == 1 {
        "Added to queue".to_string()
    } else {
        format!("Added {n} tracks")
    };
    *toast = Some(Toast::new(message, "Queue only — playback unchanged"));
}

fn move_queue_song(queue: &mut Index<Song>, from: usize, to: usize) {
    if from >= queue.len() || to >= queue.len() || from == to {
        return;
    }
    let song = queue.remove(from);
    queue.insert(to, song);
}

fn remove_queue_indices(queue: &mut Index<Song>, mut idxs: Vec<usize>) {
    idxs.sort_unstable();
    idxs.dedup();
    for i in idxs.into_iter().rev() {
        if i < queue.len() {
            queue.remove(i);
        }
    }
}

fn unique_playlist_name(lists: &Index<mu_core::Playlist>, base: &str) -> String {
    if !lists.iter().any(|p| p.name() == base) {
        return base.to_string();
    }
    for n in 2..1000 {
        let candidate = format!("{base} {n}");
        if !lists.iter().any(|p| p.name() == candidate) {
            return candidate;
        }
    }
    format!("{base} {}", lists.len() + 1)
}

fn save_queue_as_playlist(
    queue: &Index<Song>,
    playlists: &mut Index<mu_core::Playlist>,
    config_path: &std::path::Path,
) -> Option<String> {
    if queue.is_empty() {
        return None;
    }
    let name = unique_playlist_name(playlists, "Queue");
    let list: Vec<Song> = queue.iter().cloned().collect();
    let pl = mu_core::Playlist::new(&name, list, config_path);
    if pl.save().is_err() {
        return None;
    }
    playlists.push(pl);
    Some(name)
}

fn artist_discography(db: &Database, artist: &str) -> Vec<Song> {
    db.albums_by_artist(artist)
        .into_iter()
        .flat_map(|a| a.songs.clone())
        .collect()
}

fn refresh_artists(db: &Database) -> Vec<String> {
    db.artists().into_iter().cloned().collect()
}

fn apply_menu_command(
    cmd: MenuCommand,
    player: &mut Player,
    playback: &mut Index<Song>,
    queue: &mut Index<Song>,
    playlists: &mut Index<mu_core::Playlist>,
    selection: &mut PathSelection,
    toast: &mut Option<Toast>,
    mode: &mut Mode,
    config_path: &std::path::Path,
) {
    match cmd {
        MenuCommand::Play { songs, index } => {
            start_playback(player, playback, songs, index);
        }
        MenuCommand::AddToQueue(list) => {
            let n = append_queue(queue, list);
            append_toast(toast, n);
        }
        MenuCommand::RemoveFromQueue(idxs) => {
            remove_queue_indices(queue, idxs);
            selection.clear();
        }
        MenuCommand::MoveUp(i) => {
            if i > 0 {
                move_queue_song(queue, i, i - 1);
            }
        }
        MenuCommand::MoveDown(i) => {
            if i + 1 < queue.len() {
                move_queue_song(queue, i, i + 1);
            }
        }
        MenuCommand::ClearQueue => {
            queue.clear();
            selection.clear();
        }
        MenuCommand::ClearExceptPlaying => {
            let keep_path = playback.selected().map(|s| s.path.clone());
            if let Some(path) = keep_path {
                let keep = queue.iter().find(|s| s.path == path).cloned();
                queue.clear();
                if let Some(song) = keep {
                    queue.push(song);
                }
            } else {
                queue.clear();
            }
            selection.clear();
        }
        MenuCommand::SaveQueueAsPlaylist => match save_queue_as_playlist(queue, playlists, config_path)
        {
            Some(name) => {
                *toast = Some(Toast::new("Playlist saved", format!("Saved as “{name}”")));
            }
            None => {
                *toast = Some(Toast::new(
                    "Could not save",
                    "Queue is empty or write failed",
                ));
            }
        },
        MenuCommand::DeletePlaylist(name) => {
            if let Some(i) = playlists.iter().position(|p| p.name() == name) {
                playlists[i].delete();
                playlists.remove(i);
                if let Some(idx) = playlists.index() {
                    if idx >= playlists.len() {
                        playlists.select(playlists.len().checked_sub(1));
                    }
                }
                if matches!(mode, Mode::PlaylistDetail { name: n } if n == &name) {
                    *mode = Mode::Playlist;
                    selection.clear();
                }
                *toast = Some(Toast::new(
                    "Playlist deleted",
                    format!("Removed “{name}”"),
                ));
            }
        }
    }
}

fn apply_palette_action(
    action: command_palette::Action,
    palette: &mut CommandPalette,
    player: &mut Player,
    playback: &mut Index<Song>,
    queue: &mut Index<Song>,
    mode: &mut Mode,
    selected_artist: &mut Option<String>,
    selection: &mut PathSelection,
    main_scroll: &mut usize,
    toast: &mut Option<Toast>,
    scan_handle: &mut Option<std::thread::JoinHandle<db::ScanResult>>,
    scan_timer: &mut Instant,
    dots: &mut usize,
    persist: &mu_core::settings::Settings,
    config: &Config,
    artists: &[String],
    _db: &Database,
) {
    match action {
        command_palette::Action::RescanDatabase => {
            palette.close();
            start_scan(scan_handle, scan_timer, dots, toast, persist, config);
        }
        command_palette::Action::PlayAndQueue {
            play,
            play_index,
            queue_add,
        } => {
            palette.close();
            let n = append_queue(queue, queue_add);
            append_toast(toast, n);
            if !play.is_empty() {
                start_playback(player, playback, play, play_index);
            }
        }
        command_palette::Action::OpenArtist(name) => {
            palette.close();
            if artists.iter().any(|a| a == &name) {
                *selected_artist = Some(name.clone());
                *mode = Mode::Artist { name };
                selection.clear();
                *main_scroll = 0;
            }
        }
        command_palette::Action::Close => palette.close(),
    }
}

fn start_scan(
    scan_handle: &mut Option<std::thread::JoinHandle<db::ScanResult>>,
    scan_timer: &mut Instant,
    dots: &mut usize,
    toast: &mut Option<Toast>,
    persist: &mu_core::settings::Settings,
    config: &Config,
) {
    if scan_handle.is_some() {
        *toast = Some(Toast::new("Scan already running", "Please wait…"));
        return;
    }
    if persist.music_folder.is_empty() {
        log!("No music folder set. Use: mu_gui add <path>");
        *toast = Some(Toast::new(
            "No music folder",
            "Use: mu_gui add <path>",
        ));
        return;
    }
    *scan_timer = Instant::now();
    *dots = 1;
    *scan_handle = Some(db::create(
        &persist.music_folder,
        config.database.clone(),
    ));
    log!("Scanning {}…", persist.music_folder);
}

fn main() {
    mini::defer_results!();
    mini::profile!();

    let config = config_paths();
    let mut persist = mu_core::settings::Settings::new(&config.settings).unwrap();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut scan_handle = None;
    let mut scan_timer = Instant::now();

    if !args.is_empty() {
        match args[0].as_str() {
            "add" => {
                if args.len() == 1 {
                    println!("Usage: mu_gui add <path>");
                    return;
                }
                match path(args[1].clone()) {
                    Some(p) if p.exists() => {
                        persist.music_folder = p.to_string_lossy().to_string();
                        let _ = persist.save();
                        scan_handle =
                            Some(db::create(&persist.music_folder, config.database.clone()));
                        scan_timer = Instant::now();
                    }
                    _ => {
                        println!("Invalid path.");
                        return;
                    }
                }
            }
            "reset" => {
                return match mu_core::db::reset(&config) {
                    Ok(_) => println!("Database reset!"),
                    Err(e) => println!("Failed to reset database! {e}"),
                };
            }
            "help" | "--help" => {
                println!("Usage");
                println!("   mu_gui [<command> <args>]");
                println!();
                println!("Options");
                println!("   add    <path> Add music to the library");
                println!("   reset         Reset the database");
                return;
            }
            _ => {
                println!("Invalid command.");
                return;
            }
        }
    }

    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        std::process::exit(1);
    }));

    let elapsed = persist.elapsed;
    let volume = persist.volume;
    // Explicit queue (Add only). Playback is a separate session list.
    let mut queue = Index::from(persist.queue.clone());
    // Resume: if a saved queue index is valid, start playback from the queue.
    let resume_idx = (persist.index as usize).min(persist.queue.len().saturating_sub(1));
    let mut playback = if persist.queue.is_empty() {
        Index::default()
    } else {
        Index::new(persist.queue.clone(), Some(resume_idx))
    };

    let outputs = OutputDevices::new();
    let mut devices = outputs.devices();
    let device = outputs
        .find(&persist.output_device)
        .unwrap_or(outputs.default_device());
    let mut current_device = device.name.clone();
    let mut player = Player::new(device);

    let db_path = config.database.clone();
    let mut db = std::thread::spawn(move || Database::new(&db_path))
        .join()
        .unwrap();
    let mut artists = refresh_artists(&db);
    let mut playlists = Index::from(mu_core::playlist::playlists(&config.mu));

    let mut ui = ui("mu", 1200, 780);
    ui.default_font_size = 13;
    ui.clear_color = colors::BG;

    let icon_font = ui.add_font(theme::load_icon_font());

    player.set_volume(volume);
    if let Some(song) = playback.selected() {
        play(&mut player, song, false);
        if elapsed > 0.0 {
            player.seek_to(Duration::from_secs_f32(elapsed));
        }
    }

    let mut mode = Mode::Queue;
    let mut prev_mode = Mode::Queue;
    let mut search = Search::new();
    let mut palette = CommandPalette::new();
    let mut context_menu = ContextMenu::new();
    let mut toast: Option<Toast> = None;
    let mut selected_artist: Option<String> = None;
    let mut selection = PathSelection::new();
    let mut artist_scroll: usize = 0;
    let mut main_scroll: usize = 0;
    let mut seek_drag: Option<f32> = None;
    let mut shuffle = false;
    let mut repeat = RepeatMode::Off;
    let mut mute = false;
    let mut old_volume: u8 = 15;
    let mut last_tick = Instant::now();
    let mut dots: usize = 1;

    while ui.window.open() {
        if let Some(handle) = &scan_handle {
            if handle.is_finished() {
                let handle = scan_handle.take().unwrap();
                let result = handle.join().unwrap();
                db = Database::new(&config.database);
                artists = refresh_artists(&db);
                playlists = Index::from(mu_core::playlist::playlists(&config.mu));
                search.dirty = true;

                if let Some(name) = &selected_artist {
                    if !artists.iter().any(|a| a == name) {
                        selected_artist = None;
                        mode = Mode::Queue;
                    }
                }
                if let Mode::Artist { name } = &mode {
                    if !artists.iter().any(|a| a == name) {
                        mode = Mode::Queue;
                    }
                }

                // Prefer worker-thread duration (true scan cost). Wall-clock from
                // `scan_timer` also includes main-loop lag before we notice completion.
                match result {
                    db::ScanResult::Completed { elapsed, tracks } => {
                        let secs = elapsed.as_secs_f32();
                        log!(
                            "Finished scanning in {:.2}s ({} tracks, wall {:.2}s).",
                            secs,
                            tracks,
                            scan_timer.elapsed().as_secs_f32()
                        );
                        toast = Some(Toast::new(
                            "Scan complete",
                            format!("{secs:.2}s · {tracks} tracks"),
                        ));
                    }
                    db::ScanResult::CompletedWithErrors {
                        elapsed,
                        tracks,
                        errors,
                    } => {
                        let secs = elapsed.as_secs_f32();
                        log!(
                            "Scan finished with {} errors in {:.2}s ({} tracks).",
                            errors.len(),
                            secs,
                            tracks
                        );
                        toast = Some(Toast::new(
                            "Scan finished with errors",
                            format!("{secs:.2}s · {} errors", errors.len()),
                        ));
                    }
                    db::ScanResult::FileInUse => {
                        log!("Could not update database, file in use.");
                        toast = Some(Toast::new(
                            "Scan failed",
                            "Database file is in use",
                        ));
                    }
                }
            }
        }

        if toast.as_ref().is_some_and(|t| t.expired()) {
            toast = None;
        }

        if last_tick.elapsed() >= Duration::from_millis(150) {
            if scan_handle.is_some() {
                dots = if dots < 3 { dots + 1 } else { 1 };
                log!(
                    "Scanning {}{}",
                    persist.music_folder.replace("\\\\?\\", ""),
                    ".".repeat(dots)
                );
            }

            persist.volume = player.volume();
            // Persist the explicit queue. Index/elapsed track the playback session
            // only when it still matches a queue path (best-effort resume).
            persist.queue = queue.iter().cloned().collect();
            if let Some(path) = playback.selected().map(|s| s.path.as_str()) {
                if let Some(i) = queue.iter().position(|s| s.path == path) {
                    persist.index = i as u16;
                } else {
                    persist.index = 0;
                }
            } else {
                persist.index = 0;
            }
            persist.elapsed = player.elapsed().as_secs_f32();
            let _ = persist.save();

            devices = outputs.devices();
            last_tick = Instant::now();
        }

        if player.is_finished() && !playback.is_empty() {
            match repeat {
                RepeatMode::One => {
                    if let Some(song) = playback.selected().cloned() {
                        play(&mut player, &song, true);
                    }
                }
                RepeatMode::Off | RepeatMode::All => {
                    playback.down();
                    if let Some(song) = playback.selected().cloned() {
                        play(&mut player, &song, true);
                    }
                }
            }
        }

        {
            let window = &ui.window;
            let shift = window.modifiers().shift;
            let ctrl = window.modifiers().ctrl;
            let search_focus = search.focused && matches!(mode, Mode::Search);

            // Global shortcuts — available even while search/page focus is active.
            if ctrl
                && (window.pressed(Key::Char('P')) || window.pressed(Key::Char('p')))
            {
                search.focused = false;
                palette.open_commands();
            } else if ctrl
                && (window.pressed(Key::Char('F')) || window.pressed(Key::Char('f')))
            {
                search.focused = false;
                palette.open_search();
            } else if palette.open {
                command_palette::on_text_input(&mut palette, window.text_input());
                command_palette::on_backspace(&mut palette, window, shift);

                if window.pressed(Key::Escape) {
                    palette.close();
                }
                if window.pressed(Key::ArrowUp) || window.pressed(Key::Up) {
                    command_palette::move_selection(&mut palette, &db, -1);
                }
                if window.pressed(Key::ArrowDown) || window.pressed(Key::Down) {
                    command_palette::move_selection(&mut palette, &db, 1);
                }
                if window.pressed(Key::Enter) {
                    if let Some(action) =
                        command_palette::try_activate(&palette, &db, &artists, shift)
                    {
                        apply_palette_action(
                            action,
                            &mut palette,
                            &mut player,
                            &mut playback,
                            &mut queue,
                            &mut mode,
                            &mut selected_artist,
                            &mut selection,
                            &mut main_scroll,
                            &mut toast,
                            &mut scan_handle,
                            &mut scan_timer,
                            &mut dots,
                            &persist,
                            &config,
                            &artists,
                            &db,
                        );
                    }
                }
            } else if search_focus {
                for c in window.text_input() {
                    if !c.is_control() {
                        search.query.push(*c);
                        search.dirty = true;
                    }
                }
                search::on_backspace(&mut search, window, shift);
                if window.pressed(Key::Escape) {
                    search.focused = false;
                    search.backspace_held_since = None;
                    search.backspace_last_tick = None;
                    if search.query.is_empty() {
                        mode = prev_mode.clone();
                    }
                }
            } else {
                search.backspace_held_since = None;
                search.backspace_last_tick = None;

                if window.pressed(Key::Char('1')) {
                    mode = Mode::Queue;
                    selected_artist = None;
                    selection.clear();
                    main_scroll = 0;
                }
                if window.pressed(Key::Char('2')) {
                    prev_mode = mode.clone();
                    mode = Mode::Search;
                    selected_artist = None;
                    selection.clear();
                    search.focused = true;
                    main_scroll = 0;
                }
                if window.pressed(Key::Char('3')) {
                    mode = Mode::Playlist;
                    selected_artist = None;
                    selection.clear();
                    main_scroll = 0;
                }
                if window.pressed(Key::Char('4')) {
                    mode = Mode::Settings;
                    selected_artist = None;
                    selection.clear();
                    main_scroll = 0;
                }
                if window.pressed(Key::Char('/')) {
                    prev_mode = mode.clone();
                    mode = Mode::Search;
                    selected_artist = None;
                    selection.clear();
                    search.focused = true;
                    main_scroll = 0;
                }

                // Enter → append selection to queue (library) or play selection (queue).
                if window.pressed(Key::Enter) && !selection.is_empty() {
                    match &mode {
                        Mode::Artist { name } => {
                            let all = artist_discography(&db, name);
                            let n = append_queue(&mut queue, selection.collect_songs(&all));
                            append_toast(&mut toast, n);
                        }
                        Mode::PlaylistDetail { name } => {
                            let all: Vec<Song> = playlists
                                .iter()
                                .find(|p| p.name() == name)
                                .map(|p| p.songs.iter().cloned().collect())
                                .unwrap_or_default();
                            let n = append_queue(&mut queue, selection.collect_songs(&all));
                            append_toast(&mut toast, n);
                        }
                        Mode::Queue => {
                            if let Some(path) = selection.first() {
                                if let Some(i) = queue.iter().position(|s| s.path == path) {
                                    let list: Vec<Song> = queue.iter().cloned().collect();
                                    start_playback(&mut player, &mut playback, list, i);
                                }
                            }
                        }
                        _ => {}
                    }
                }

                if window.pressed(Key::Escape) {
                    if context_menu.is_open() {
                        context_menu.close();
                    } else if !selection.is_empty() {
                        selection.clear();
                    }
                }

                if window.pressed(Key::Space) {
                    player.toggle_playback();
                }
                if window.pressed(Key::Char('A')) || window.pressed(Key::Char('a')) {
                    if !playback.is_empty() {
                        playback.up();
                        if let Some(song) = playback.selected().cloned() {
                            play(&mut player, &song, true);
                        }
                    }
                }
                if window.pressed(Key::Char('D')) || window.pressed(Key::Char('d')) {
                    if !playback.is_empty() {
                        playback.down();
                        if let Some(song) = playback.selected().cloned() {
                            play(&mut player, &song, true);
                        }
                    }
                }
                if window.pressed(Key::Char('Q')) || window.pressed(Key::Char('q')) {
                    player.seek_backward(10.0);
                }
                if window.pressed(Key::Char('E')) || window.pressed(Key::Char('e')) {
                    player.seek_forward(10.0);
                }
                if window.pressed(Key::Char('W')) || window.pressed(Key::Char('w')) {
                    player.volume_up();
                    mute = false;
                }
                if window.pressed(Key::Char('S')) || window.pressed(Key::Char('s')) {
                    player.volume_down();
                }
                if window.pressed(Key::Char('Z')) || window.pressed(Key::Char('z')) {
                    if mute {
                        player.set_volume(old_volume);
                        mute = false;
                    } else {
                        old_volume = player.volume();
                        player.set_volume(0);
                        mute = true;
                    }
                }
                if window.pressed(Key::Char('C')) || window.pressed(Key::Char('c')) {
                    // Clear the explicit queue only — never stops artist/playlist playback.
                    if shift {
                        let keep_path = playback.selected().map(|s| s.path.clone());
                        if let Some(path) = keep_path {
                            let keep = queue.iter().find(|s| s.path == path).cloned();
                            queue.clear();
                            if let Some(song) = keep {
                                queue.push(song);
                            }
                        } else {
                            queue.clear();
                        }
                    } else {
                        queue.clear();
                    }
                    selection.clear();
                }
                if window.pressed(Key::Char('X')) || window.pressed(Key::Char('x')) {
                    if matches!(mode, Mode::Queue) {
                        if !selection.is_empty() {
                            let ordered: Vec<String> =
                                queue.iter().map(|s| s.path.clone()).collect();
                            let idxs: Vec<usize> = selection
                                .paths()
                                .iter()
                                .filter_map(|p| ordered.iter().position(|o| o == p))
                                .collect();
                            remove_queue_indices(&mut queue, idxs);
                            selection.clear();
                        }
                    } else if let Some(idx) = playback.index() {
                        // Remove from the current playback session only.
                        playback.remove_and_move(idx);
                        if let Some(song) = playback.selected().cloned() {
                            play(&mut player, &song, true);
                        } else {
                            player.stop();
                        }
                    }
                }
                if window.pressed(Key::Char('U')) || window.pressed(Key::Char('u')) {
                    start_scan(
                        &mut scan_handle,
                        &mut scan_timer,
                        &mut dots,
                        &mut toast,
                        &persist,
                        &config,
                    );
                }
            }
        }

        ui.frame(|ui| {
            ui.clear_color = colors::BG;
            let palette_open = palette.open;
            let block_nav = palette_open || context_menu.is_open();

            let (body, bar_rect) = ui.split_v(Size::FillMinus(PLAYER_H));
            let (sidebar_rect, main_rect) = ui.split_rect_h(body, SIDEBAR_W);

            if let Some(action) = sidebar::draw(
                ui,
                sidebar_rect,
                &mode,
                &artists,
                selected_artist.as_deref(),
                &mut artist_scroll,
                queue.len(),
                icon_font,
            ) {
                if !block_nav {
                    match action {
                        sidebar::Action::Mode(m) => {
                            mode = m;
                            selected_artist = None;
                            selection.clear();
                            context_menu.close();
                            main_scroll = 0;
                            if matches!(mode, Mode::Search) {
                                search.focused = true;
                            }
                        }
                        sidebar::Action::Artist(name) => {
                            if artists.iter().any(|a| a == &name) {
                                selected_artist = Some(name.clone());
                                mode = Mode::Artist { name };
                                selection.clear();
                                context_menu.close();
                                main_scroll = 0;
                            }
                        }
                    }
                }
            }

            ui.paint_rect(main_rect, style().bg(colors::BG));
            let playing_path = playback.selected().map(|s| s.path.clone());

            match &mode.clone() {
                Mode::Search => {
                    if let Some(action) =
                        search::draw(ui, main_rect, &mut search, &db, &artists, &mut main_scroll)
                    {
                        if !block_nav {
                            match action {
                                search::Action::OpenArtist(name) => {
                                    search.focused = false;
                                    selected_artist = Some(name.clone());
                                    mode = Mode::Artist { name };
                                    selection.clear();
                                    context_menu.close();
                                    main_scroll = 0;
                                }
                                search::Action::Play(song) => {
                                    search.focused = false;
                                    let artist = song.artist.clone();
                                    if artists.iter().any(|a| a == &artist) {
                                        let disco = artist_discography(&db, &artist);
                                        if let Some(index) =
                                            disco.iter().position(|s| s.path == song.path)
                                        {
                                            start_playback(
                                                &mut player,
                                                &mut playback,
                                                disco,
                                                index,
                                            );
                                        } else {
                                            start_playback(
                                                &mut player,
                                                &mut playback,
                                                vec![song],
                                                0,
                                            );
                                        }
                                    } else {
                                        start_playback(
                                            &mut player,
                                            &mut playback,
                                            vec![song],
                                            0,
                                        );
                                    }
                                }
                                search::Action::Append(song) => {
                                    search.focused = false;
                                    let n = append_queue(&mut queue, vec![song]);
                                    append_toast(&mut toast, n);
                                }
                                search::Action::AppendAlbum { artist, album } => {
                                    search.focused = false;
                                    let list = search::album_songs(&db, &artists, &artist, &album);
                                    let n = append_queue(&mut queue, list);
                                    append_toast(&mut toast, n);
                                }
                                search::Action::PlayAlbum { artist, album } => {
                                    search.focused = false;
                                    // Play artist discography starting at this album.
                                    let disco = artist_discography(&db, &artist);
                                    let album_songs =
                                        search::album_songs(&db, &artists, &artist, &album);
                                    let index = album_songs
                                        .first()
                                        .and_then(|first| {
                                            disco.iter().position(|s| s.path == first.path)
                                        })
                                        .unwrap_or(0);
                                    if disco.is_empty() {
                                        start_playback(
                                            &mut player,
                                            &mut playback,
                                            album_songs,
                                            0,
                                        );
                                    } else {
                                        start_playback(
                                            &mut player,
                                            &mut playback,
                                            disco,
                                            index,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                Mode::Playlist => {
                    if let Some(action) = playlist::draw_list(
                        ui,
                        main_rect,
                        &playlists,
                        &mut context_menu,
                        &mut main_scroll,
                    ) {
                        if !block_nav {
                            match action {
                                playlist::Action::OpenDetail(name) => {
                                    mode = Mode::PlaylistDetail { name };
                                    selection.clear();
                                    context_menu.close();
                                    main_scroll = 0;
                                }
                                playlist::Action::Play { songs: list, index } => {
                                    start_playback(&mut player, &mut playback, list, index);
                                }
                                playlist::Action::Back => {}
                            }
                        }
                    }
                }
                Mode::PlaylistDetail { name } => {
                    if let Some(action) = playlist::draw_detail(
                        ui,
                        main_rect,
                        name,
                        &playlists,
                        playing_path.as_deref(),
                        &mut selection,
                        &mut context_menu,
                        &mut main_scroll,
                    ) {
                        if !block_nav {
                            match action {
                                playlist::Action::Back => {
                                    mode = Mode::Playlist;
                                    selection.clear();
                                    context_menu.close();
                                    main_scroll = 0;
                                }
                                playlist::Action::Play { songs: list, index } => {
                                    start_playback(&mut player, &mut playback, list, index);
                                }
                                playlist::Action::OpenDetail(_) => {}
                            }
                        }
                    }
                }
                Mode::Artist { name } => {
                    if let Some(action) = artist::draw(
                        ui,
                        main_rect,
                        &db,
                        &artists,
                        name,
                        playing_path.as_deref(),
                        &mut selection,
                        &mut context_menu,
                        &mut main_scroll,
                    ) {
                        if !block_nav {
                            match action {
                                artist::Action::MissingArtist => {
                                    selected_artist = None;
                                    selection.clear();
                                    context_menu.close();
                                    mode = Mode::Queue;
                                    log!("Artist not found: {name}");
                                }
                                artist::Action::PlayDiscography { songs: list, index } => {
                                    start_playback(&mut player, &mut playback, list, index);
                                }
                            }
                        }
                    }
                }
                Mode::Queue => {
                    if let Some(action) = queue::draw(
                        ui,
                        main_rect,
                        &queue,
                        playing_path.as_deref(),
                        &mut selection,
                        &mut context_menu,
                        &mut main_scroll,
                    ) {
                        if !block_nav {
                            let queue::Action::PlayIndex(i) = action;
                            let list: Vec<Song> = queue.iter().cloned().collect();
                            start_playback(&mut player, &mut playback, list, i);
                        }
                    }
                }
                Mode::Settings => {
                    if let Some(settings::Action::SelectDevice(i)) = settings::draw(
                        ui,
                        main_rect,
                        &devices,
                        &current_device,
                        &persist.music_folder,
                        &mut main_scroll,
                    ) {
                        if !block_nav {
                            if let Some(dev) = devices.get(i).cloned() {
                                player.set_output_device(dev.clone());
                                persist.output_device = dev.name.clone();
                                current_device = dev.name;
                            }
                        }
                    }
                }
            }

            if let Some(action) = player_bar::draw(
                ui,
                bar_rect,
                &mut player,
                &mut playback,
                &mut seek_drag,
                &mut shuffle,
                &mut repeat,
                &mut mute,
                icon_font,
            ) {
                if !block_nav {
                    match action {
                        player_bar::Action::OpenQueue => {
                            mode = Mode::Queue;
                            selected_artist = None;
                            selection.clear();
                            context_menu.close();
                            main_scroll = 0;
                        }
                        player_bar::Action::TogglePlay => player.toggle_playback(),
                        player_bar::Action::Prev => {
                            if !playback.is_empty() {
                                playback.up();
                                if let Some(song) = playback.selected().cloned() {
                                    play(&mut player, &song, true);
                                }
                            }
                        }
                        player_bar::Action::Next => {
                            if !playback.is_empty() {
                                playback.down();
                                if let Some(song) = playback.selected().cloned() {
                                    play(&mut player, &song, true);
                                }
                            }
                        }
                        player_bar::Action::ToggleShuffle | player_bar::Action::CycleRepeat => {}
                    }
                }
            }

            // Right-click context menu (drawn above content, below palette).
            if let Some(cmd) = context_menu::draw(ui, &mut context_menu) {
                apply_menu_command(
                    cmd,
                    &mut player,
                    &mut playback,
                    &mut queue,
                    &mut playlists,
                    &mut selection,
                    &mut toast,
                    &mut mode,
                    &config.mu,
                );
            }

            // Command palette overlay (Ctrl+P / Ctrl+F).
            let shift = ui.window.modifiers().shift;
            if let Some(action) =
                command_palette::draw(ui, &mut palette, &db, &artists, shift)
            {
                apply_palette_action(
                    action,
                    &mut palette,
                    &mut player,
                    &mut playback,
                    &mut queue,
                    &mut mode,
                    &mut selected_artist,
                    &mut selection,
                    &mut main_scroll,
                    &mut toast,
                    &mut scan_handle,
                    &mut scan_timer,
                    &mut dots,
                    &persist,
                    &config,
                    &artists,
                    &db,
                );
            }

            // Toast (add-to-queue feedback, scan complete, etc.).
            if let Some(t) = &toast {
                let (win_w, _) = ui.window.content_size();
                if toast::draw(ui, t, bar_rect.y, win_w as i32) {
                    toast = None;
                }
            }
        });
    }

    persist.volume = player.volume();
    persist.queue = queue.iter().cloned().collect();
    if let Some(path) = playback.selected().map(|s| s.path.as_str()) {
        if let Some(i) = queue.iter().position(|s| s.path == path) {
            persist.index = i as u16;
        } else {
            persist.index = 0;
        }
    } else {
        persist.index = 0;
    }
    persist.elapsed = player.elapsed().as_secs_f32();
    let _ = persist.save();
}
