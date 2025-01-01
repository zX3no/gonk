use browser::Browser;
use gonk_core::{vdb::*, *};
use gonk_player::*;
use mini::defer_results;
use playlist::{Mode as PlaylistMode, Playlist};
use queue::Queue;
use search::{Mode as SearchMode, Search};
use settings::Settings;
use std::{
    fs,
    time::{Duration, Instant},
};
use winter::*;

mod browser;
mod help;
mod playlist;
mod queue;
mod search;
mod settings;

const JUMP_AMOUNT: usize = 3;
const FRAME_TIME: f32 = 1000.0 / 300.0;

const NUMBER: Color = Color::Green;
const TITLE: Color = Color::Cyan;
const ALBUM: Color = Color::Magenta;
const ARTIST: Color = Color::Blue;
const SEEKER: Color = Color::White;

#[derive(PartialEq, Eq, Clone)]
pub enum Mode {
    Browser,
    Queue,
    Playlist,
    Settings,
    Search,
}

fn draw(
    winter: &mut Winter,
    mode: &Mode,
    browser: &mut Browser,
    settings: &Settings,
    queue: &mut Queue,
    playlist: &mut Playlist,
    search: &mut Search,
    cursor: &mut Option<(u16, u16)>,
    songs: &mut Index<Song>,
    db: &Database,
    mouse: Option<(u16, u16)>,
    help: bool,
    mute: bool,
) {
    let viewport = winter.viewport;
    let buf = winter.buffer();
    let area = if let Some(msg) = log::last_message() {
        let length = 3;
        let fill = viewport.height.saturating_sub(length);
        let area = layout(viewport, Vertical, &[Length(fill), Length(length)]);
        lines!(msg).block(block()).draw(area[1], buf);
        area[0]
    } else {
        viewport
    };

    //Hide the cursor when it's not needed.
    match mode {
        Mode::Search | Mode::Playlist => {}
        _ => *cursor = None,
    }

    match mode {
        Mode::Browser => browser::draw(browser, area, buf, mouse),
        Mode::Settings => settings::draw(settings, area, buf),
        Mode::Queue => queue::draw(queue, area, buf, mouse, songs, mute),
        Mode::Playlist => *cursor = playlist::draw(playlist, area, buf, mouse),
        Mode::Search => *cursor = search::draw(search, area, buf, mouse, db),
    }

    if help {
        if let Ok(area) = area.inner(8, 6) {
            let widths = [Constraint::Percentage(50), Constraint::Percentage(50)];

            //TODO: This is hard to read because the gap between command and key is large.
            let header = header!["Command".bold(), "Key".bold()];
            let table = table(help::HELP.clone(), &widths)
                .header(header)
                .block(block().title("Help:"));
            buf.clear(area);
            table.draw(area, buf, None);
        }
    }
}

fn path(mut path: String) -> Option<std::path::PathBuf> {
    if path.contains("~") {
        path = path.replace("~", &user_profile_directory().unwrap());
    }
    fs::canonicalize(path).ok()
}

fn main() {
    defer_results!();
    let mut persist = gonk_core::settings::Settings::new().unwrap();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut scan_timer = Instant::now();
    let mut scan_handle = None;

    if !args.is_empty() {
        match args[0].as_str() {
            "add" => {
                if args.len() == 1 {
                    return println!("Usage: gonk add <path>");
                }

                match path(args[1].clone()) {
                    Some(path) if path.exists() => {
                        persist.music_folder = path.to_string_lossy().to_string();
                        scan_handle = Some(db::create(&persist.music_folder));
                        scan_timer = Instant::now();
                    }
                    _ => return println!("Invalid path."),
                }
            }
            "reset" => {
                return match gonk_core::db::reset() {
                    Ok(_) => println!("Database reset!"),
                    Err(e) => println!("Failed to reset database! {e}"),
                };
            }
            "help" | "--help" => {
                println!("Usage");
                println!("   gonk [<command> <args>]");
                println!();
                println!("Options");
                println!("   add    <path> Add music to the library");
                println!("   reset         Reset the database");
                println!("   buffer <size> Set a custom ring buffer size");
                return;
            }
            "b" | "buffer" | "--buffer" | "--b" => match args.get(1) {
                Some(rb_size) => unsafe {
                    gonk_player::RB_SIZE = rb_size.parse::<usize>().unwrap()
                },
                None => {
                    println!("Please enter a valid ring buffer size `buffer <size>`.");
                    return;
                }
            },
            _ if !args.is_empty() => return println!("Invalid command."),
            _ => (),
        }
    }

    //Prevents panic messages from being hidden.
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let mut stdout = std::io::stdout();
        let mut stdin = std::io::stdin();
        uninit(&mut stdout, &mut stdin);
        orig_hook(panic_info);
        std::process::exit(1);
    }));

    let po = persist.output_device.clone();
    let thread = std::thread::spawn(move || {
        let device_list = devices();
        let default_device = default_device();
        let device = device_list
            .iter()
            .find(|d| d.name == po)
            .unwrap_or(&default_device)
            .clone();
        spawn_audio_threads(device.clone());

        Settings::new(device_list.clone(), device.name.clone())
    });

    let mut winter = Winter::new();
    let index = (!persist.queue.is_empty()).then_some(persist.index as usize);

    set_volume(persist.volume);

    let mut songs = Index::new(persist.queue.clone(), index);
    if let Some(song) = songs.selected() {
        play_song(song);
        pause();
        seek(persist.elapsed);
    }

    let mut db = Database::new();
    let mut browser = Browser::new(&db);

    //Everything here initialises quickly.
    let mut queue = Queue::new(index.unwrap_or(0));
    let mut playlist = Playlist::new().unwrap();
    let mut search = Search::new();
    let mut mode = Mode::Browser;
    let mut last_tick = Instant::now();
    let mut ft = Instant::now();
    let mut dots: usize = 1;
    let mut help = false;
    let mut prev_mode = Mode::Search; //Used for search.
    let mut mute = false;
    let mut old_volume = 0;
    let mut cursor: Option<(u16, u16)> = None;
    let mut shift;
    let mut control;

    let mut settings = thread.join().unwrap();

    //If there are songs in the queue and the database isn't scanning, display the queue.
    if !songs.is_empty() && scan_handle.is_none() {
        mode = Mode::Queue;
    }

    macro_rules! up {
        () => {{
            let amount = if shift { JUMP_AMOUNT } else { 1 };
            match mode {
                Mode::Browser => browser::up(&mut browser, &db, amount),
                Mode::Queue => queue::up(&mut queue, &mut songs, amount),
                Mode::Playlist => playlist::up(&mut playlist, amount),
                Mode::Settings => settings::up(&mut settings, amount),
                Mode::Search => search.results.up_n(amount),
            }
        }};
    }

    macro_rules! down {
        () => {{
            let amount = if shift { JUMP_AMOUNT } else { 1 };
            match mode {
                Mode::Browser => browser::down(&mut browser, &db, amount),
                Mode::Queue => queue::down(&mut queue, &mut songs, amount),
                Mode::Playlist => playlist::down(&mut playlist, amount),
                Mode::Settings => settings::down(&mut settings, amount),
                Mode::Search => search.results.down_n(amount),
            }
        }};
    }

    macro_rules! left {
        () => {
            match mode {
                Mode::Browser => browser::left(&mut browser),
                Mode::Playlist => playlist::left(&mut playlist),
                _ => {}
            }
        };
    }

    macro_rules! right {
        () => {
            match mode {
                Mode::Browser => browser::right(&mut browser),
                Mode::Playlist => playlist::right(&mut playlist),
                _ => {}
            }
        };
    }

    'outer: loop {
        if let Some(handle) = &scan_handle {
            if handle.is_finished() {
                let handle = scan_handle.take().unwrap();
                let result = handle.join().unwrap();

                db = Database::new();
                log::clear();

                match result {
                    db::ScanResult::Completed => {
                        log!(
                            "Finished adding {} files in {:.2} seconds.",
                            db.len,
                            scan_timer.elapsed().as_secs_f32()
                        );
                    }
                    db::ScanResult::CompletedWithErrors(errors) => {
                        let dir = "See %appdata%/gonk/gonk.log for details.";
                        let len = errors.len();
                        let s = if len == 1 { "" } else { "s" };

                        log!(
                            "Added {} files with {len} error{s}. {dir}",
                            db.len.saturating_sub(len)
                        );

                        let path = gonk_path().join("gonk.log");
                        let errors = errors.join("\n");
                        fs::write(path, errors).unwrap();
                    }
                    db::ScanResult::FileInUse => {
                        log!("Could not update database, file in use.")
                    }
                }

                browser::refresh(&mut browser, &db);
                search.results = Index::new(db.search(&search.query), None);

                //No need to reset scan_timer since it's reset with new scans.
                scan_handle = None;
            }
        }

        if last_tick.elapsed() >= Duration::from_millis(150) {
            if scan_handle.is_some() {
                if dots < 3 {
                    dots += 1;
                } else {
                    dots = 1;
                }
                log!(
                    "Scanning {} for files{}",
                    //Remove the UNC \\?\ from the path.
                    &persist.music_folder.replace("\\\\?\\", ""),
                    ".".repeat(dots)
                );
            }

            //Update the time elapsed.
            persist.index = songs.index().unwrap_or(0) as u16;
            persist.elapsed = elapsed().as_secs_f32();
            persist.queue = songs.to_vec();
            persist.save().unwrap();

            //Update the list of output devices
            settings.devices = devices();
            let mut index = settings.index.unwrap_or(0);
            if index >= settings.devices.len() {
                index = settings.devices.len().saturating_sub(1);
                settings.index = Some(index);
            }

            last_tick = Instant::now();
        }

        //Play the next song if the current is finished.
        if gonk_player::play_next() && !songs.is_empty() {
            songs.down();
            if let Some(song) = songs.selected() {
                play_song(song);
            }
        }

        let input_playlist = playlist.mode == PlaylistMode::Popup && mode == Mode::Playlist;
        let empty = songs.is_empty();

        draw(
            &mut winter,
            &mode,
            &mut browser,
            &settings,
            &mut queue,
            &mut playlist,
            &mut search,
            &mut cursor,
            &mut songs,
            &db,
            None,
            help,
            mute,
        );

        'events: {
            let Some((event, state)) = winter.poll() else {
                break 'events;
            };

            shift = state.shift();
            control = state.control();

            match event {
                Event::LeftMouse(x, y) if !help => {
                    draw(
                        &mut winter,
                        &mode,
                        &mut browser,
                        &settings,
                        &mut queue,
                        &mut playlist,
                        &mut search,
                        &mut cursor,
                        &mut songs,
                        &db,
                        Some((x, y)),
                        help,
                        mute,
                    );
                }
                Event::ScrollUp => up!(),
                Event::ScrollDown => down!(),
                Event::Backspace if mode == Mode::Playlist => {
                    playlist::on_backspace(&mut playlist, control);
                }
                Event::Char('c') if control => break 'outer,
                Event::Char('?') | Event::Char('/') | Event::Escape if help => help = false,
                Event::Char('?') if mode != Mode::Search => help = true,
                Event::Char('/') => {
                    if mode != Mode::Search {
                        prev_mode = mode;
                        mode = Mode::Search;
                        search.query_changed = true;
                    } else {
                        match search.mode {
                            SearchMode::Search if search.query.is_empty() => {
                                mode = prev_mode.clone();
                            }
                            SearchMode::Search => {
                                search.query.push('/');
                                search.query_changed = true;
                            }
                            SearchMode::Select => {
                                search.mode = SearchMode::Search;
                                search.results.select(None);
                            }
                        }
                    }
                }
                Event::Char('a') if control => {
                    queue.range = Some(0..songs.len());
                }
                Event::Backspace if mode == Mode::Search => {
                    search::on_backspace(&mut search, control, shift);
                }
                //Handle ^W as control backspace.
                Event::Char('w') if control && mode == Mode::Search => {
                    search::on_backspace(&mut search, control, shift);
                }
                Event::Char(c) if search.mode == SearchMode::Search && mode == Mode::Search => {
                    search.query.push(c);
                    search.query_changed = true;
                }
                Event::Escape if mode == Mode::Search => {
                    search.query = String::new();
                    search.query_changed = true;
                    search.mode = SearchMode::Search;
                    mode = prev_mode.clone();
                    search.results.select(None);
                }
                Event::Tab if mode == Mode::Search => {
                    mode = prev_mode.clone();
                }
                Event::Char(c) if input_playlist => {
                    if control && c == 'w' {
                        playlist::on_backspace(&mut playlist, true);
                    } else {
                        playlist.changed = true;
                        playlist.search_query.push(c);
                    }
                }
                Event::Char(' ') => toggle_playback(),
                Event::Char('C') => {
                    clear_except_playing(&mut songs);
                    queue.set_index(0);
                }
                Event::Char('c') => {
                    gonk_player::clear(&mut songs);
                }
                Event::Char('x') => match mode {
                    Mode::Queue => {
                        if let Some(i) = queue.index() {
                            gonk_player::delete(&mut songs, i);

                            //Sync the UI index.
                            let len = songs.len().saturating_sub(1);
                            if i > len {
                                queue.set_index(len);
                            }
                        }
                    }
                    Mode::Playlist => {
                        playlist::delete(&mut playlist, false);
                    }
                    _ => (),
                },
                //Force delete -> Shift + X.
                Event::Char('X') if mode == Mode::Playlist => playlist::delete(&mut playlist, true),
                Event::Char('u') if mode == Mode::Browser || mode == Mode::Playlist => {
                    if scan_handle.is_none() {
                        if persist.music_folder.is_empty() {
                            gonk_core::log!("Nothing to scan! Add a folder with 'gonk add /path/'");
                        } else {
                            scan_handle = Some(db::create(&persist.music_folder));
                            scan_timer = Instant::now();
                            playlist.lists = Index::from(gonk_core::playlist::playlists());
                        }
                    }
                }
                Event::Char('z') => {
                    if mute {
                        mute = false;
                        set_volume(old_volume)
                    } else {
                        mute = true;
                        old_volume = get_volume();
                        set_volume(0);
                    }
                }
                Event::Char('q') => seek_backward(),
                Event::Char('e') => seek_foward(),
                Event::Char('a') => {
                    songs.up();
                    if let Some(song) = songs.selected() {
                        play_song(song);
                    }
                }
                Event::Char('d') => {
                    songs.down();
                    if let Some(song) = songs.selected() {
                        play_song(song);
                    }
                }
                Event::Char('w') => {
                    volume_up();
                    persist.volume = get_volume();
                }
                Event::Char('s') => {
                    volume_down();
                    persist.volume = get_volume();
                }
                Event::Escape if mode == Mode::Playlist => {
                    if playlist.delete {
                        playlist.yes = true;
                        playlist.delete = false;
                    } else if let playlist::Mode::Popup = playlist.mode {
                        playlist.mode = playlist::Mode::Playlist;
                        playlist.search_query = String::new();
                        playlist.changed = true;
                    }
                }
                Event::Tab if mode != Mode::Search => {
                    prev_mode = mode.clone();
                    mode = Mode::Search;
                }
                Event::Enter if mode == Mode::Browser && shift => {
                    playlist::add(&mut playlist, browser::get_selected(&browser, &db));
                    mode = Mode::Playlist
                }
                Event::Enter if mode == Mode::Browser => {
                    songs.extend(browser::get_selected(&browser, &db));
                }
                Event::Enter if mode == Mode::Queue && shift => {
                    if let Some(range) = &queue.range {
                        let mut playlist_songs = Vec::new();

                        for index in range.start..=range.end {
                            if let Some(song) = songs.get(index) {
                                playlist_songs.push(song.clone());
                            }
                        }

                        playlist::add(&mut playlist, playlist_songs);
                        mode = Mode::Playlist;
                    }
                }
                Event::Enter if mode == Mode::Queue => {
                    if let Some(i) = queue.index() {
                        songs.select(Some(i));
                        play_song(&songs[i]);
                    }
                }
                Event::Enter if mode == Mode::Settings => {
                    if let Some(device) = settings::selected(&settings) {
                        let device = device.to_string();
                        set_output_device(&device);
                        settings.current_device = device.clone();
                        persist.output_device = device.clone();
                    }
                }
                Event::Enter if mode == Mode::Playlist => {
                    playlist::on_enter(&mut playlist, &mut songs, shift);
                }
                Event::Enter if mode == Mode::Search && shift => {
                    if let Some(songs) = search::on_enter(&mut search, &db) {
                        playlist::add(
                            &mut playlist,
                            songs.iter().map(|song| song.clone().clone()).collect(),
                        );
                        mode = Mode::Playlist;
                    }
                }
                Event::Enter if mode == Mode::Search => {
                    if let Some(s) = search::on_enter(&mut search, &db) {
                        //Swap to the queue so people can see what they added.
                        mode = Mode::Queue;
                        songs.extend(s.iter().cloned());
                    }
                }
                Event::Char('1') => mode = Mode::Queue,
                Event::Char('2') => mode = Mode::Browser,
                Event::Char('3') => mode = Mode::Playlist,
                Event::Char('4') => mode = Mode::Settings,
                Event::Function(1) => queue::constraint(&mut queue, 0, shift),
                Event::Function(2) => queue::constraint(&mut queue, 1, shift),
                Event::Function(3) => queue::constraint(&mut queue, 2, shift),
                Event::Up | Event::Char('k') | Event::Char('K') => up!(),
                Event::Down | Event::Char('j') | Event::Char('J') => down!(),
                Event::Left | Event::Char('h') | Event::Char('H') => left!(),
                Event::Right | Event::Char('l') | Event::Char('L') => right!(),
                _ => {}
            }
        }

        //New songs were added.
        if empty && !songs.is_empty() {
            queue.set_index(0);
            songs.select(Some(0));
            if let Some(song) = songs.selected() {
                play_song(song);
            }
        }

        winter.draw();

        //Move cursor
        if let Some((x, y)) = cursor {
            show_cursor(&mut winter.stdout);
            move_to(&mut winter.stdout, x, y);
        } else {
            hide_cursor(&mut winter.stdout);
        }

        winter.flush().unwrap();

        let frame = ft.elapsed().as_secs_f32() * 1000.0;
        if frame < FRAME_TIME {
            std::thread::sleep(Duration::from_secs_f32((FRAME_TIME - frame) / 1000.0));
            ft = Instant::now();
        } else {
            ft = Instant::now();
        }
    }

    persist.queue = songs.to_vec();
    persist.index = songs.index().unwrap_or(0) as u16;
    persist.elapsed = elapsed().as_secs_f32();
    persist.save().unwrap();
}
