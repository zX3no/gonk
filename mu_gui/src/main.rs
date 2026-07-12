use mu_core::{vdb::*, *};
use neoui::*;
use onmi::{OutputDevices, Player};
use std::{
    fs,
    time::{Duration, Instant},
};

mod artist;
mod command_palette;
mod home;
mod player_bar;
mod playlist;
mod queue;
mod search;
mod settings;
mod sidebar;
mod theme;
mod toast;

use command_palette::CommandPalette;
use search::Search;
use theme::colors;
use toast::Toast;

const PLAYER_H: i32 = player_bar::PLAYER_H;
const SIDEBAR_W: i32 = sidebar::SIDEBAR_W;

#[derive(PartialEq, Eq, Clone)]
pub enum Mode {
    Home,
    Search,
    Playlist,
    PlaylistDetail { name: String },
    Artist { name: String },
    Queue,
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

fn replace_and_play(player: &mut Player, songs: &mut Index<Song>, list: Vec<Song>, index: usize) {
    if list.is_empty() {
        return;
    }
    let idx = index.min(list.len() - 1);
    *songs = Index::new(list, Some(idx));
    if let Some(song) = songs.selected().cloned() {
        play(player, &song, true);
    }
}

fn append(player: &mut Player, songs: &mut Index<Song>, list: Vec<Song>) {
    if list.is_empty() {
        return;
    }
    let was_empty = songs.is_empty();
    let start = songs.len();
    songs.extend(list);
    if was_empty {
        songs.select(Some(start));
        if let Some(song) = songs.selected().cloned() {
            play(player, &song, true);
        }
    }
}

fn refresh_artists(db: &Database) -> Vec<String> {
    db.artists().into_iter().cloned().collect()
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

    let index = (!persist.queue.is_empty()).then_some(persist.index as usize);
    let elapsed = persist.elapsed;
    let volume = persist.volume;
    let mut songs = Index::new(persist.queue.clone(), index);

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
    if let Some(song) = songs.selected() {
        play(&mut player, song, false);
        if elapsed > 0.0 {
            player.seek_to(Duration::from_secs_f32(elapsed));
        }
    }

    let mut mode = Mode::Home;
    let mut prev_mode = Mode::Home;
    let mut search = Search::new();
    let mut palette = CommandPalette::new();
    let mut toast: Option<Toast> = None;
    let mut selected_artist: Option<String> = None;
    let mut list_selected_path: Option<String> = None;
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
                        mode = Mode::Home;
                    }
                }
                if let Mode::Artist { name } = &mode {
                    if !artists.iter().any(|a| a == name) {
                        mode = Mode::Home;
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
            persist.index = songs.index().unwrap_or(0) as u16;
            persist.elapsed = player.elapsed().as_secs_f32();
            persist.queue = songs.iter().cloned().collect();
            let _ = persist.save();

            devices = outputs.devices();
            last_tick = Instant::now();
        }

        if player.is_finished() && !songs.is_empty() {
            match repeat {
                RepeatMode::One => {
                    if let Some(song) = songs.selected().cloned() {
                        play(&mut player, &song, true);
                    }
                }
                RepeatMode::Off | RepeatMode::All => {
                    songs.down();
                    if let Some(song) = songs.selected().cloned() {
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
                        match action {
                            command_palette::Action::RescanDatabase => {
                                palette.close();
                                start_scan(
                                    &mut scan_handle,
                                    &mut scan_timer,
                                    &mut dots,
                                    &mut toast,
                                    &persist,
                                    &config,
                                );
                            }
                            command_palette::Action::Play(song) => {
                                palette.close();
                                replace_and_play(&mut player, &mut songs, vec![song], 0);
                            }
                            command_palette::Action::Append(song) => {
                                palette.close();
                                append(&mut player, &mut songs, vec![song]);
                            }
                            command_palette::Action::OpenArtist(name) => {
                                palette.close();
                                if artists.iter().any(|a| a == &name) {
                                    selected_artist = Some(name.clone());
                                    mode = Mode::Artist { name };
                                    list_selected_path = None;
                                    main_scroll = 0;
                                }
                            }
                            command_palette::Action::Close => palette.close(),
                        }
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
                    mode = Mode::Home;
                    selected_artist = None;
                    list_selected_path = None;
                    main_scroll = 0;
                }
                if window.pressed(Key::Char('2')) {
                    prev_mode = mode.clone();
                    mode = Mode::Search;
                    selected_artist = None;
                    list_selected_path = None;
                    search.focused = true;
                    main_scroll = 0;
                }
                if window.pressed(Key::Char('3')) {
                    mode = Mode::Playlist;
                    selected_artist = None;
                    list_selected_path = None;
                    main_scroll = 0;
                }
                if window.pressed(Key::Char('4')) {
                    mode = Mode::Queue;
                    selected_artist = None;
                    list_selected_path = None;
                    main_scroll = 0;
                }
                if window.pressed(Key::Char('5')) {
                    mode = Mode::Settings;
                    selected_artist = None;
                    list_selected_path = None;
                    main_scroll = 0;
                }
                if window.pressed(Key::Char('/')) {
                    prev_mode = mode.clone();
                    mode = Mode::Search;
                    selected_artist = None;
                    list_selected_path = None;
                    search.focused = true;
                    main_scroll = 0;
                }

                if window.pressed(Key::Space) {
                    player.toggle_playback();
                }
                if window.pressed(Key::Char('A')) || window.pressed(Key::Char('a')) {
                    if !songs.is_empty() {
                        songs.up();
                        if let Some(song) = songs.selected().cloned() {
                            play(&mut player, &song, true);
                        }
                    }
                }
                if window.pressed(Key::Char('D')) || window.pressed(Key::Char('d')) {
                    if !songs.is_empty() {
                        songs.down();
                        if let Some(song) = songs.selected().cloned() {
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
                    if shift {
                        if let Some(idx) = songs.index() {
                            let keep = songs.get(idx).cloned();
                            songs.clear();
                            if let Some(song) = keep {
                                songs.push(song);
                                songs.select(Some(0));
                            }
                        }
                    } else {
                        songs.clear();
                        songs.select(None);
                        player.stop();
                    }
                }
                if window.pressed(Key::Char('X')) || window.pressed(Key::Char('x')) {
                    if let Some(idx) = songs.index() {
                        songs.remove_and_move(idx);
                        if let Some(song) = songs.selected().cloned() {
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

            let (body, bar_rect) = ui.split_v(Size::FillMinus(PLAYER_H));
            let (sidebar_rect, main_rect) = ui.split_rect_h(body, SIDEBAR_W);

            if let Some(action) = sidebar::draw(
                ui,
                sidebar_rect,
                &mode,
                &artists,
                selected_artist.as_deref(),
                &mut artist_scroll,
                icon_font,
            ) {
                if !palette_open {
                    match action {
                        sidebar::Action::Mode(m) => {
                            mode = m;
                            selected_artist = None;
                            list_selected_path = None;
                            main_scroll = 0;
                            if matches!(mode, Mode::Search) {
                                search.focused = true;
                            }
                        }
                        sidebar::Action::Artist(name) => {
                            if artists.iter().any(|a| a == &name) {
                                selected_artist = Some(name.clone());
                                mode = Mode::Artist { name };
                                list_selected_path = None;
                                main_scroll = 0;
                            }
                        }
                    }
                }
            }

            ui.paint_rect(main_rect, style().bg(colors::BG));
            let playing_path = songs.selected().map(|s| s.path.clone());

            match &mode.clone() {
                Mode::Home => {
                    let _ = home::draw(
                        ui,
                        main_rect,
                        &artists,
                        songs.selected().map(|s| s.title.as_str()),
                        &mut main_scroll,
                    );
                }
                Mode::Search => {
                    if let Some(action) =
                        search::draw(ui, main_rect, &mut search, &db, &artists, &mut main_scroll)
                    {
                        if !palette_open {
                            match action {
                                search::Action::OpenArtist(name) => {
                                    search.focused = false;
                                    selected_artist = Some(name.clone());
                                    mode = Mode::Artist { name };
                                    list_selected_path = None;
                                    main_scroll = 0;
                                }
                                search::Action::Play(song) => {
                                    search.focused = false;
                                    replace_and_play(&mut player, &mut songs, vec![song], 0);
                                }
                                search::Action::Append(song) => {
                                    search.focused = false;
                                    append(&mut player, &mut songs, vec![song]);
                                }
                            }
                        }
                    }
                }
                Mode::Playlist => {
                    if let Some(action) =
                        playlist::draw_list(ui, main_rect, &playlists, &mut main_scroll)
                    {
                        if !palette_open {
                            if let playlist::Action::OpenDetail(name) = action {
                                mode = Mode::PlaylistDetail { name };
                                list_selected_path = None;
                                main_scroll = 0;
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
                        &mut list_selected_path,
                        &mut main_scroll,
                    ) {
                        if !palette_open {
                            match action {
                                playlist::Action::Back => {
                                    mode = Mode::Playlist;
                                    list_selected_path = None;
                                    main_scroll = 0;
                                }
                                playlist::Action::Play { songs: list, index } => {
                                    replace_and_play(&mut player, &mut songs, list, index);
                                }
                                playlist::Action::Append(song) => {
                                    append(&mut player, &mut songs, vec![song]);
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
                        &mut list_selected_path,
                        &mut main_scroll,
                    ) {
                        if !palette_open {
                            match action {
                                artist::Action::MissingArtist => {
                                    selected_artist = None;
                                    list_selected_path = None;
                                    mode = Mode::Home;
                                    log!("Artist not found: {name}");
                                }
                                artist::Action::PlayAlbum { songs: list, index } => {
                                    replace_and_play(&mut player, &mut songs, list, index);
                                }
                                artist::Action::Append(song) => {
                                    append(&mut player, &mut songs, vec![song]);
                                }
                            }
                        }
                    }
                }
                Mode::Queue => {
                    if let Some(queue::Action::PlayIndex(i)) =
                        queue::draw(ui, main_rect, &songs, &mut list_selected_path, &mut main_scroll)
                    {
                        if !palette_open {
                            songs.select(Some(i));
                            if let Some(song) = songs.selected().cloned() {
                                play(&mut player, &song, true);
                            }
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
                        if !palette_open {
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
                &mut songs,
                &mut seek_drag,
                &mut shuffle,
                &mut repeat,
                &mut mute,
                icon_font,
            ) {
                if !palette_open {
                    match action {
                        player_bar::Action::OpenQueue => {
                            mode = Mode::Queue;
                            selected_artist = None;
                            list_selected_path = None;
                            main_scroll = 0;
                        }
                        player_bar::Action::TogglePlay => player.toggle_playback(),
                        player_bar::Action::Prev => {
                            if !songs.is_empty() {
                                songs.up();
                                if let Some(song) = songs.selected().cloned() {
                                    play(&mut player, &song, true);
                                }
                            }
                        }
                        player_bar::Action::Next => {
                            if !songs.is_empty() {
                                songs.down();
                                if let Some(song) = songs.selected().cloned() {
                                    play(&mut player, &song, true);
                                }
                            }
                        }
                        player_bar::Action::ToggleShuffle | player_bar::Action::CycleRepeat => {}
                    }
                }
            }

            // Command palette overlay (Ctrl+P / Ctrl+F).
            let shift = ui.window.modifiers().shift;
            if let Some(action) =
                command_palette::draw(ui, &mut palette, &db, &artists, shift)
            {
                match action {
                    command_palette::Action::RescanDatabase => {
                        palette.close();
                        start_scan(
                            &mut scan_handle,
                            &mut scan_timer,
                            &mut dots,
                            &mut toast,
                            &persist,
                            &config,
                        );
                    }
                    command_palette::Action::Play(song) => {
                        palette.close();
                        replace_and_play(&mut player, &mut songs, vec![song], 0);
                    }
                    command_palette::Action::Append(song) => {
                        palette.close();
                        append(&mut player, &mut songs, vec![song]);
                    }
                    command_palette::Action::OpenArtist(name) => {
                        palette.close();
                        if artists.iter().any(|a| a == &name) {
                            selected_artist = Some(name.clone());
                            mode = Mode::Artist { name };
                            list_selected_path = None;
                            main_scroll = 0;
                        }
                    }
                    command_palette::Action::Close => palette.close(),
                }
            }

            // Scan-complete toast (bottom-right, above player bar).
            if let Some(t) = &toast {
                let (win_w, _) = ui.window.content_size();
                if toast::draw(ui, t, bar_rect.y, win_w as i32) {
                    toast = None;
                }
            }
        });
    }

    persist.volume = player.volume();
    persist.index = songs.index().unwrap_or(0) as u16;
    persist.elapsed = player.elapsed().as_secs_f32();
    persist.queue = songs.iter().cloned().collect();
    let _ = persist.save();
}
