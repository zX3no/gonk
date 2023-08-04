#![allow(unused)]
use browser::Browser;
use gonk_core::{vdb::Database, *};
use gonk_player::{Player, Wasapi};
use once_cell::sync::Lazy;
use playlist::{Mode as PlaylistMode, Playlist};
use queue::Queue;
use search::{Mode as SearchMode, Search};
use settings::Settings;
use std::{
    error::Error,
    fs,
    io::{stdout, Stdout, Write},
    path::Path,
    ptr::addr_of_mut,
    time::{Duration, Instant},
};
use winter::{symbols::line::ROUNDED, *};

mod browser;
mod playlist;
mod queue;
mod search;
mod settings;

const NUMBER: Color = Color::Green;
const TITLE: Color = Color::Cyan;
const ALBUM: Color = Color::Magenta;
const ARTIST: Color = Color::Blue;
const SEEKER: Color = Color::White;

static HELP: Lazy<[Row; 29]> = Lazy::new(|| {
    [
        row![text!("Move Up", fg(Cyan)), "K / UP"],
        row![text!("Move Down", fg(Cyan)), "J / Down"],
        row![text!("Move Left", fg(Cyan)), "H / Left"],
        row![text!("Move Right", fg(Cyan)), "L / Right"],
        row![text!("Volume Up", fg(Green)), "W"],
        row![text!("Volume Down", fg(Green)), "S"],
        row![text!("Mute", fg(Green)), "Z"],
        row![text!("Play/Pause", fg(Magenta)), "Space"],
        row![text!("Previous", fg(Magenta)), "A"],
        row![text!("Next", fg(Magenta)), "D"],
        row![text!("Seek -10s", fg(Magenta)), "Q"],
        row![text!("Seek 10s", fg(Magenta)), "E"],
        row![text!("Queue", fg(Blue)), "1"],
        row![text!("Browser", fg(Blue)), "2"],
        row![text!("Playlists", fg(Blue)), "3"],
        row![text!("Settings", fg(Blue)), "4"],
        row![text!("Search", fg(Blue)), "/"],
        row![text!("Exit Search", fg(Blue)), "Escape"],
        row![text!("Add song to queue", fg(Cyan)), "Enter"],
        row![text!("Add song to playlist", fg(Cyan)), "Shift + Enter"],
        row![text!("Move song margin", fg(Green)), "F1 / Shift + F1"],
        row![text!("Move album margin", fg(Green)), "F2 / Shift + F2"],
        row![text!("Move artist margin", fg(Green)), "F3 / Shift + F3"],
        row![text!("Update database", fg(Yellow)), "U"],
        row![text!("Quit player", fg(Yellow)), "Ctrl + C"],
        row![text!("Clear queue", fg(Red)), "C"],
        row![text!("Clear except playing", fg(Red)), "Shift + C"],
        row![text!("Delete song/playlist", fg(Red)), "X"],
        row![text!("Delete without confirmation", fg(Red)), "Shift + X"],
    ]
});

#[derive(PartialEq, Eq, Clone)]
pub enum Mode {
    Browser,
    Queue,
    Playlist,
    Settings,
    Search,
}

fn draw_log(area: Rect, buf: &mut Buffer) -> Rect {
    if let Some(msg) = log::last_message() {
        let area = layout!(
            area,
            Direction::Vertical,
            Constraint::Min(2),
            Constraint::Length(3)
        );
        lines([text![msg]])
            .block(None, Borders::ALL, Rounded)
            .draw(area[1], buf);
        area[0]
    } else {
        area
    }
}

const SEARCH_MARGIN: (u16, u16) = (6, 8);

fn main() -> std::result::Result<(), Box<dyn Error>> {
    let mut persist = gonk_core::settings::Settings::new();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut scan_timer = Instant::now();
    let mut scan_handle = None;

    if !args.is_empty() {
        match args[0].as_str() {
            "add" => {
                if args.len() == 1 {
                    return Ok(println!("Usage: gonk add <path>"));
                }
                let path = args[1..].join(" ");
                let Ok(path) = fs::canonicalize(path) else {
                    return Ok(println!("Invalid path."));
                };
                let Some(path) = path.to_str() else {
                    return Ok(println!("Invalid path."));
                };
                if Path::new(&path).exists() {
                    persist.music_folder = path.to_string();
                    scan_handle = Some(db::create(path));
                    scan_timer = Instant::now();
                } else {
                    return Ok(println!("Invalid path."));
                }
            }
            "reset" => {
                return match gonk_core::db::reset() {
                    Ok(_) => Ok(println!("Database reset!")),
                    Err(e) => Ok(println!("Failed to reset database! {e}")),
                };
            }
            "help" | "--help" => {
                println!("Usage");
                println!("   gonk [<command> <args>]");
                println!();
                println!("Options");
                println!("   add   <path>  Add music to the library");
                println!("   reset         Reset the database");
                return Ok(());
            }
            _ if !args.is_empty() => return Ok(println!("Invalid command.")),
            _ => (),
        }
    }

    let (output_handle, input_handle) = handles();
    let (width, height) = info(output_handle).window_size;

    let mut viewport = Rect::new(0, 0, width, height);
    let mut buffers: [Buffer; 2] = [Buffer::empty(viewport), Buffer::empty(viewport)];
    let mut current = 0;

    //Prevents panic messages from being hidden.
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let mut stdout = stdout();
        // disable_raw_mode();
        // disable_mouse_caputure();
        leave_alternate_screen(&mut stdout);
        show_cursor(&mut stdout);
        stdout.flush().unwrap();

        orig_hook(panic_info);
        std::process::exit(1);
    }));

    let mut stdout = stdout();
    winter::init(&mut stdout);

    let index = if persist.queue.is_empty() {
        None
    } else {
        Some(persist.index as usize)
    };

    let songs = Index::new(persist.queue.clone(), index);
    let ui_index = index.unwrap_or(0);
    let mut player = Player::new(
        &persist.output_device,
        persist.volume,
        songs,
        persist.elapsed,
    );

    let mut db = Database::new();
    let mut queue = Queue::new(ui_index, addr_of_mut!(player));
    let mut browser = Browser::new(&db);
    let mut playlist = Playlist::new()?;
    let mut settings = Settings::new(&persist.output_device);
    let mut search = Search::new();
    let mut mode = Mode::Browser;
    let mut last_tick = Instant::now();
    let mut dots: usize = 1;

    //If there are songs in the queue and the database isn't scanning, display the queue.
    if !player.songs.is_empty() && scan_handle.is_none() {
        mode = Mode::Queue;
    }

    let mut help = false;

    //Stores previous mode for search.
    let mut prev_mode = Mode::Search;

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
                        fs::write(path, errors)?;
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
                log!("Scanning for files{}", ".".repeat(dots));
            }

            //Update the time elapsed.
            persist.index = player.songs.index().unwrap_or(0) as u16;
            persist.elapsed = player.elapsed().as_secs_f32();
            persist.queue = player.songs.to_vec();
            persist.save()?;

            //Update the list of output devices
            //TODO: I don't like how this is done.
            settings.devices = Wasapi::devices();
            let mut index = settings.index.unwrap_or(0);
            if index >= settings.devices.len() {
                index = settings.devices.len().saturating_sub(1);
                settings.index = Some(index);
            }

            last_tick = Instant::now();
        }

        //Update the UI index.
        queue.len = player.songs.len();

        //TODO: This is very slow and causes scrolling to feel laggy.
        player.update();

        let input_playlist = playlist.mode == PlaylistMode::Popup && mode == Mode::Playlist;

        //Draw widgets
        let mut draw = || {
            let buf = &mut buffers[current];
            let area = draw_log(viewport, buf);

            //TODO: Handle mouse events.
            // let event = if searching { None } else { Some(mouse_event) };

            //TODO: Remove mouse_event from draw.
            match mode {
                Mode::Browser => browser::draw(&mut browser, area, buf, None),
                Mode::Settings => settings::draw(&mut settings, area, buf),
                Mode::Queue => queue::draw(&mut queue, area, buf, None),
                Mode::Playlist => playlist::draw(&mut playlist, area, buf, None, &mut stdout),
                Mode::Search => search::draw(&mut search, area, buf, None, &db),
            }

            if help {
                let area = area.inner(SEARCH_MARGIN);
                buf.clear(area);
                let widths = [Constraint::Percentage(50), Constraint::Percentage(50)];

                //TODO: This is hard to read because the gap between command and key is large.
                let header = header![text!("Command", bold()), text!("Key", bold())];
                let table = table(
                    Some(header),
                    Some(block(Some("Help:".into()), ALL, Rounded)),
                    &widths,
                    HELP.clone(),
                    None,
                    style(),
                );
                table.draw(area, buf, None);
            }
        };

        draw();

        //Handle events
        'events: {
            //TODO: I feel like event handling and audio playback should be on different thread.
            let Some((event, state)) = poll(Duration::from_millis(2)) else {
                break 'events;
            };

            let shift = state.shift();
            let control = state.ctrl();

            match event {
                Event::LeftMouse if !help => {
                    // draw();

                    //FIXME: Copy-paste drawing. Need to swap to custom tui library.
                    //tui-rs has private fields which makes the api to inflexable for my use.
                    // terminal.draw(|f| {
                    //     let top = draw_log(f);
                    //     let event = if searching { None } else { Some(mouse_event) };

                    //     match mode {
                    //         Mode::Browser => browser::draw(&mut browser, f, top, event),
                    //         Mode::Queue => queue::draw(&mut queue, f, top, event),
                    //         Mode::Playlist => playlist::draw(&mut playlist, f, top, event),
                    //         Mode::Settings => settings::draw(&mut settings, f, top),
                    //     }

                    //     if searching {
                    //         search::draw(&mut search, f, top, Some(mouse_event), &db);
                    //     }
                    // })?;
                }
                Event::ScrollUp => match mode {
                    Mode::Browser => browser::up(&mut browser, &db),
                    Mode::Queue => queue::up(&mut queue),
                    Mode::Playlist => playlist::up(&mut playlist),
                    Mode::Settings => settings::up(&mut settings),
                    Mode::Search => search::up(&mut search),
                },
                Event::ScrollDown => match mode {
                    Mode::Browser => browser::down(&mut browser, &db),
                    Mode::Queue => queue::down(&mut queue),
                    Mode::Playlist => playlist::down(&mut playlist),
                    Mode::Settings => settings::down(&mut settings),
                    Mode::Search => search::down(&mut search),
                },
                Event::Char('c') if control => break 'outer,
                _ if help => match event {
                    Event::Char('?') | Event::Char('/') | Event::Escape => help = false,
                    _ => {}
                },
                Event::Char('?') => help = true,
                //TODO: Fix search cursor.
                Event::Char('/') => {
                    if mode != Mode::Search {
                        prev_mode = mode;
                        mode = Mode::Search;
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
                Event::Char(c) if search.mode == SearchMode::Search && mode == Mode::Search => {
                    //Handle ^W as control backspace.
                    if control && c == 'w' {
                        search::on_backspace(&mut search, true);
                    } else {
                        search.query.push(c);
                        search.query_changed = true;
                    }
                }
                Event::Char(c) if input_playlist => {
                    if control && c == 'w' {
                        playlist::on_backspace(&mut playlist, true);
                    } else {
                        playlist.changed = true;
                        playlist.search_query.push(c);
                    }
                }
                Event::Space => player.toggle_playback(),
                Event::Char('C') => {
                    player.clear_except_playing();
                    queue.ui.select(Some(0));
                }
                Event::Char('c') => {
                    player.clear();
                    queue.ui.select(Some(0));
                }
                Event::Char('x') => match mode {
                    Mode::Queue => {
                        if let Some(i) = queue.ui.index() {
                            player.delete_index(i);

                            //Sync the UI index.
                            let len = player.songs.len().saturating_sub(1);
                            if i > len {
                                queue.ui.select(Some(len));
                            }
                        }
                    }
                    Mode::Playlist => {
                        playlist::delete(&mut playlist, false);
                    }
                    _ => (),
                },
                Event::Char('X') if mode == Mode::Playlist => {
                    playlist::delete(&mut playlist, false)
                }
                Event::Char('u') if mode == Mode::Browser || mode == Mode::Playlist => {
                    if scan_handle.is_none() {
                        if persist.music_folder.is_empty() {
                            gonk_core::log!("Nothing to scan! Add a folder with 'gonk add /path/'");
                        } else {
                            scan_handle = Some(db::create(persist.music_folder.clone()));
                            scan_timer = Instant::now();
                            playlist.lists = Index::from(gonk_core::playlist::playlists());
                        }
                    }
                }
                Event::Char('z') => player.mute(),
                Event::Char('q') => player.seek_backward(),
                Event::Char('e') => player.seek_foward(),
                Event::Char('a') => player.prev(),
                Event::Char('d') => player.next(),
                Event::Char('w') => {
                    player.volume_up();
                    persist.volume = player.volume();
                }
                Event::Char('s') => {
                    player.volume_down();
                    persist.volume = player.volume();
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
                Event::Escape if mode == Mode::Search => match search.mode {
                    SearchMode::Search => {
                        search.query = String::new();
                        search.query_changed = true;
                        mode = prev_mode.clone();
                    }
                    SearchMode::Select => {
                        search.mode = SearchMode::Search;
                        search.results.select(None);
                        search.query = String::new();
                        search.query_changed = true;
                    }
                },
                Event::Enter if shift => match mode {
                    Mode::Browser => {
                        playlist::add(&mut playlist, browser::get_selected(&browser, &db));
                        mode = Mode::Playlist
                    }
                    Mode::Queue => {
                        if let Some(index) = queue.ui.index() {
                            if let Some(song) = player.songs.get(index) {
                                playlist::add(&mut playlist, vec![song.clone()]);
                                mode = Mode::Playlist;
                            }
                        }
                    }
                    _ => {}
                },
                Event::Enter => match mode {
                    Mode::Browser => {
                        player.add(browser::get_selected(&browser, &db));
                    }
                    Mode::Queue => {
                        if let Some(i) = queue.ui.index() {
                            player.play_index(i);
                        }
                    }
                    Mode::Settings => {
                        if let Some(device) = settings::selected(&mut settings) {
                            let device = device.to_string();
                            player.set_output_device(&device);
                            settings.current_device = device.clone();
                            persist.output_device = device.clone();
                        }
                    }
                    Mode::Playlist => playlist::on_enter(&mut playlist, &mut player),
                    Mode::Search => {
                        if shift {
                            if let Some(songs) = search::on_enter(&mut search, &db) {
                                playlist::add(
                                    &mut playlist,
                                    songs.iter().map(|song| song.clone().clone()).collect(),
                                );
                                mode = Mode::Playlist;
                            }
                        } else {
                            if let Some(songs) = search::on_enter(&mut search, &db) {
                                //Swap to the queue so people can see what they added.
                                mode = Mode::Queue;
                                let songs: Vec<Song> =
                                    songs.iter().map(|song| song.clone().clone()).collect();
                                player.add(songs);
                            }
                        }
                    }
                },
                Event::Backspace => {
                    if mode == Mode::Playlist {
                        playlist::on_backspace(&mut playlist, control)
                    } else if mode == Mode::Search {
                        match search.mode {
                            SearchMode::Search if !search.query.is_empty() => {
                                if control {
                                    search.query.clear();
                                } else {
                                    search.query.pop();
                                }

                                search.query_changed = true;
                            }
                            SearchMode::Search => (),
                            SearchMode::Select => {
                                search.results.select(None);
                                search.mode = SearchMode::Search;
                            }
                        }
                    }
                }
                Event::Char('1') => mode = Mode::Queue,
                Event::Char('2') => mode = Mode::Browser,
                Event::Char('3') => mode = Mode::Playlist,
                Event::Char('4') => mode = Mode::Settings,
                Event::Function(1) => queue::constraint(&mut queue, 0, shift),
                Event::Function(2) => queue::constraint(&mut queue, 1, shift),
                Event::Function(3) => queue::constraint(&mut queue, 2, shift),
                Event::Up | Event::Char('k') => match mode {
                    Mode::Browser => browser::up(&mut browser, &db),
                    Mode::Queue => queue::up(&mut queue),
                    Mode::Playlist => playlist::up(&mut playlist),
                    Mode::Settings => settings::up(&mut settings),
                    Mode::Search => search::up(&mut search),
                },
                Event::Down | Event::Char('j') => match mode {
                    Mode::Browser => browser::down(&mut browser, &db),
                    Mode::Queue => queue::down(&mut queue),
                    Mode::Playlist => playlist::down(&mut playlist),
                    Mode::Settings => settings::down(&mut settings),
                    Mode::Search => search::down(&mut search),
                },
                Event::Left | Event::Char('h') => match mode {
                    Mode::Browser => browser::left(&mut browser),
                    Mode::Queue => {}
                    Mode::Playlist => playlist::left(&mut playlist),
                    Mode::Settings => {}
                    Mode::Search => {}
                },
                Event::Right | Event::Char('l') => match mode {
                    Mode::Browser => browser::right(&mut browser),
                    Mode::Queue => {}
                    Mode::Playlist => playlist::right(&mut playlist),
                    Mode::Settings => {}
                    Mode::Search => {}
                },
                _ => {}
            }
        }

        //Calculate difference and draw to the terminal.
        let previous_buffer = &buffers[1 - current];
        let current_buffer = &buffers[current];
        let diff = previous_buffer.diff(current_buffer);
        buffer::draw(&mut stdout, diff);

        //Swap buffers
        buffers[1 - current].reset();
        current = 1 - current;

        //Update the viewport area.
        //TODO: I think there is a resize event that might be better.
        let (width, height) = info(output_handle).window_size;
        viewport = Rect::new(0, 0, width, height);

        //Resize
        if buffers[current].area != viewport {
            buffers[current].resize(viewport);
            buffers[1 - current].resize(viewport);

            // Reset the back buffer to make sure the next update will redraw everything.
            buffers[1 - current].reset();
            clear(&mut stdout);
        }
    }

    persist.queue = player.songs.to_vec();
    persist.index = player.songs.index().unwrap_or(0) as u16;
    persist.elapsed = player.elapsed().as_secs_f32();
    persist.save()?;

    uninit(&mut stdout);

    gonk_core::profiler::print();

    Ok(())
}
