use browser::Browser;
use crossterm::{event::*, terminal::*, *};
use gonk_core::*;
use gonk_player::Player;
use playlist::{Mode as PlaylistMode, Playlist};
use queue::Queue;
use search::{Mode as SearchMode, Search};
use settings::Settings;
use std::fs;
use std::{
    io::{stdout, Stdout},
    path::Path,
    time::{Duration, Instant},
};
use tui::widgets::Block;
use tui::widgets::BorderType;
use tui::widgets::Borders;
use tui::widgets::Paragraph;
use tui::{backend::CrosstermBackend, layout::*, style::Color, Terminal};

mod browser;
mod playlist;
mod queue;
mod search;
mod settings;
mod widgets;

type Frame<'a> = tui::Frame<'a, CrosstermBackend<Stdout>>;

const NUMBER: Color = Color::Green;
const TITLE: Color = Color::Cyan;
const ALBUM: Color = Color::Magenta;
const ARTIST: Color = Color::Blue;
const SEEKER: Color = Color::White;

#[derive(PartialEq, Eq)]
pub enum Mode {
    Browser,
    Queue,
    Search,
    Playlist,
    Settings,
}

pub trait Input {
    fn up(&mut self);
    fn down(&mut self);
    fn left(&mut self);
    fn right(&mut self);
}

fn draw_log(f: &mut Frame) -> Rect {
    if let Some(msg) = log::message() {
        let area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(2), Constraint::Length(3)])
            .split(f.size());

        f.render_widget(
            Paragraph::new(msg).alignment(Alignment::Left).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            ),
            area[1],
        );

        area[0]
    } else {
        f.size()
    }
}

fn main() {
    let mut scan_handle = None;

    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        match args[0].as_str() {
            "add" => {
                if args.len() == 1 {
                    return println!("Usage: gonk add <path>");
                }
                let path = args[1..].join(" ");
                if Path::new(&path).exists() {
                    Database::update_music_folder(path.as_str());
                    scan_handle = Some(Database::scan(path));
                } else {
                    return println!("Invalid path.");
                }
            }
            "reset" => {
                return match unsafe { Database::reset() } {
                    Ok(_) => println!("Files reset!"),
                    Err(e) => println!("Failed to reset database! {e}"),
                };
            }
            "help" | "--help" => {
                println!("Usage");
                println!("   gonk [<command> <args>]");
                println!();
                println!("Options");
                println!("   add   <path>  Add music to the library");
                println!("   reset         Reset the database");
                return;
            }
            _ if !args.is_empty() => return println!("Invalid command."),
            _ => (),
        }
    }

    //Disable raw mode when the program panics.
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        disable_raw_mode().unwrap();
        execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
        orig_hook(panic_info);
        std::process::exit(1);
    }));

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout())).unwrap();
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture,
    )
    .unwrap();
    enable_raw_mode().unwrap();
    terminal.clear().unwrap();

    let (songs, index, elapsed) = Database::get_saved_queue();

    let songs = Index::new(songs, index);

    let volume = Database::volume();
    let device = Database::output_device();
    let ui_index = index.unwrap_or(0);
    let mut player = Player::new(device, volume, songs, elapsed);
    let mut player_clone = player.songs.data.clone();

    let mut queue = Queue::new(ui_index);
    let mut browser = Browser::new();
    let mut playlist = Playlist::new();
    let mut settings = Settings::new();
    let mut search = Search::new();

    let mut mode = Mode::Browser;
    let mut last_tick = Instant::now();
    let mut busy = false;
    let mut dots: usize = 1;
    let mut scan_timer: Option<Instant> = None;
    let mut focused = true;

    //If there are songs in the queue and the database isn't scanning, display the queue.
    if !player.songs.is_empty() && scan_handle.is_none() {
        mode = Mode::Queue;
    }

    let mut end_scan = false;

    loop {
        if let Some(h) = &scan_handle {
            if h.is_finished() {
                end_scan = true;
            } else {
                busy = true;

                if scan_timer.is_none() {
                    scan_timer = Some(Instant::now());
                    log!("Scanning for files.");
                }
            }
        } else {
            busy = false;
        }

        if end_scan {
            let h = scan_handle.take().unwrap();
            let result = h.join().unwrap();

            log::clear();

            if let Some(time) = scan_timer {
                match result {
                    ScanResult::Completed => {
                        log!(
                            "Finished adding {} files in {:.2} seconds.",
                            Database::len(),
                            time.elapsed().as_secs_f32()
                        );
                    }
                    ScanResult::CompletedWithErrors(errors) => {
                        #[cfg(windows)]
                        let dir = "See %appdata%/gonk/gonk.log for details.";

                        #[cfg(unix)]
                        let dir = "See .config/gonk/gonk.log for details.";

                        let len = errors.len();

                        let s = if len == 1 { "" } else { "s" };

                        log!(
                            "Added {} files with {len} error{s}. {dir}",
                            Database::len() - len,
                        );

                        let path = gonk_path().join("gonk.log");
                        let errors = errors.join("\n");
                        fs::write(path, errors).unwrap();
                    }
                    ScanResult::Incomplete(error) => log!("{error}"),
                }
            }

            browser::refresh(&mut browser);
            search.results.data = Database::search(&search.query);

            scan_timer = None;
            scan_handle = None;
            end_scan = false;
        }

        if last_tick.elapsed() >= Duration::from_millis(150) {
            if busy && scan_timer.is_some() {
                if dots < 3 {
                    dots += 1;
                } else {
                    dots = 1;
                }
                log!("Scanning for files{}", ".".repeat(dots));
            }

            //Update the time elapsed.
            Database::update_queue_state(
                player.songs.index().unwrap_or(0) as u16,
                player.elapsed().as_secs_f32(),
            );

            //Update the list of output devices.
            settings.update();

            last_tick = Instant::now();
        }

        //Update the UI index.
        queue.len = player.songs.len();

        if player.is_finished() {
            player.next();
        }

        if player.songs.data != player_clone {
            player_clone = player.songs.data.clone();
            Database::save_queue(
                &player.songs.data,
                player.songs.index().unwrap_or(0) as u16,
                player.elapsed().as_secs_f32(),
            );
        }

        terminal
            .draw(|f| {
                let top = draw_log(f);
                match mode {
                    Mode::Browser => browser::draw(&mut browser, top, f, None),
                    Mode::Queue => queue::draw(&mut queue, &mut player, f, None),
                    Mode::Search => search::draw(&mut search, top, f, None),
                    Mode::Playlist => playlist::draw(&mut playlist, top, f, None),
                    Mode::Settings => settings::draw(&mut settings, top, f),
                };
            })
            .unwrap();

        let input_search = search.mode == SearchMode::Search && mode == Mode::Search;
        let input_playlist = playlist.mode == PlaylistMode::Popup && mode == Mode::Playlist;

        let input = match mode {
            Mode::Browser => &mut browser as &mut dyn Input,
            Mode::Queue => &mut queue as &mut dyn Input,
            Mode::Search => &mut search as &mut dyn Input,
            Mode::Playlist => &mut playlist as &mut dyn Input,
            Mode::Settings => &mut settings as &mut dyn Input,
        };

        if event::poll(Duration::from_millis(2)).unwrap() {
            match event::read().unwrap() {
                Event::Key(event) => {
                    let shift = event.modifiers == KeyModifiers::SHIFT;
                    let control = event.modifiers == KeyModifiers::CONTROL;

                    match event.code {
                        KeyCode::Char('c') if control => break,
                        KeyCode::Char(c) if input_search => {
                            //Handle ^W as control backspace.
                            if control && c == 'w' {
                                search::on_backspace(&mut search, true);
                            } else {
                                //Sometimes users will open the search when the meant to open playlist or settings.
                                //This will cause them to search for ',' or '.'.
                                //I can't think of any songs that would start with a comma or period so just change modes instead.
                                //Before you would need to exit from the search with tab or escape and then change to settings/playlist mode.
                                match c {
                                    ',' if search.query.is_empty() => mode = Mode::Settings,
                                    '.' if search.query.is_empty() => mode = Mode::Playlist,
                                    '/' if search.query.is_empty() => (),
                                    _ => {
                                        search.query.push(c);
                                        search.query_changed = true;
                                    }
                                };
                            }
                        }
                        KeyCode::Char(c) if input_playlist => {
                            if control && c == 'w' {
                                playlist::on_backspace(&mut playlist, true);
                            } else {
                                playlist.changed = true;
                                playlist.search_query.push(c);
                            }
                        }
                        KeyCode::Char(' ') => player.toggle_playback(),
                        KeyCode::Char('C') if shift => {
                            player.clear_except_playing();
                            queue.ui.select(Some(0));
                        }
                        KeyCode::Char('c') => {
                            player.clear();
                            queue.ui.select(Some(0));
                        }
                        KeyCode::Char('x') => match mode {
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
                        KeyCode::Char('X') => {
                            if let Mode::Playlist = mode {
                                playlist::delete(&mut playlist, true);
                            }
                        }
                        KeyCode::Char('u') if mode == Mode::Browser || mode == Mode::Playlist => {
                            if scan_handle.is_none() {
                                let folder = Database::music_folder().to_string();
                                if folder.is_empty() {
                                    //TODO: I saw this flash by but I can't replicate it.
                                    gonk_core::log!(
                                        "Nothing to scan! Add a folder with 'gonk add /path/'"
                                    );
                                } else {
                                    scan_handle = Some(Database::scan(folder));
                                    playlist.playlists = Index::from(gonk_core::playlists());
                                }
                            }
                        }
                        KeyCode::Char('q') => player.seek_backward(),
                        KeyCode::Char('e') => player.seek_foward(),
                        KeyCode::Char('a') => player.prev(),
                        KeyCode::Char('d') => player.next(),
                        KeyCode::Char('w') => {
                            player.volume_up();
                            Database::save_volume(player.volume());
                        }
                        KeyCode::Char('s') => {
                            player.volume_down();
                            Database::save_volume(player.volume());
                        }
                        KeyCode::Char(',') => mode = Mode::Settings,
                        KeyCode::Char('.') => mode = Mode::Playlist,
                        KeyCode::Char('/') => {
                            if mode == Mode::Search {
                                if search.mode == SearchMode::Select {
                                    search.results.select(None);
                                    search.mode = SearchMode::Search;
                                }
                            } else {
                                mode = Mode::Search;
                            }
                        }
                        KeyCode::Tab => {
                            terminal.clear().unwrap();
                            mode = match mode {
                                Mode::Browser | Mode::Settings | Mode::Search => Mode::Queue,
                                Mode::Queue | Mode::Playlist => Mode::Browser,
                            };
                        }
                        KeyCode::Esc => match mode {
                            Mode::Search => match search.mode {
                                search::Mode::Search => {
                                    if let search::Mode::Search = search.mode {
                                        // search.query.clear();
                                        // search.query_changed = true;
                                        mode = Mode::Queue;
                                    }
                                }
                                search::Mode::Select => {
                                    search.mode = search::Mode::Search;
                                    search.results.select(None);
                                }
                            },
                            Mode::Playlist => {
                                if playlist.delete {
                                    playlist.yes = true;
                                    playlist.delete = false;
                                } else if let playlist::Mode::Popup = playlist.mode {
                                    playlist.mode = playlist::Mode::Playlist;
                                    playlist.search_query = String::new();
                                    playlist.changed = true;
                                } else {
                                    mode = Mode::Browser;
                                }
                            }
                            Mode::Browser => mode = Mode::Queue,
                            Mode::Queue => (),
                            Mode::Settings => mode = Mode::Queue,
                        },
                        KeyCode::Enter if shift => match mode {
                            Mode::Browser => {
                                let songs: Vec<Song> = browser::get_selected(&browser)
                                    .into_iter()
                                    .cloned()
                                    .collect();
                                playlist::add(&mut playlist, &songs);
                                mode = Mode::Playlist;
                            }
                            Mode::Queue => {
                                if let Some(index) = queue.ui.index() {
                                    if let Some(song) = player.songs.data.get(index) {
                                        playlist::add(&mut playlist, &[song.clone()]);
                                        mode = Mode::Playlist;
                                    }
                                }
                            }
                            Mode::Search => {
                                if let Some(songs) = search::on_enter(&mut search) {
                                    let songs: Vec<Song> = songs.into_iter().cloned().collect();
                                    playlist::add(&mut playlist, &songs);
                                    mode = Mode::Playlist;
                                }
                            }
                            _ => (),
                        },
                        KeyCode::Enter => match mode {
                            Mode::Browser => {
                                let songs = browser::get_selected(&browser)
                                    .into_iter()
                                    .cloned()
                                    .collect();
                                player.add(songs);
                            }
                            Mode::Queue => {
                                if let Some(i) = queue.ui.index() {
                                    player.play_index(i);
                                }
                            }
                            Mode::Search => {
                                if let Some(songs) = search::on_enter(&mut search) {
                                    let songs = songs.into_iter().cloned().collect();
                                    player.add(songs);
                                }
                            }
                            Mode::Settings => {
                                if let Some(device) = settings.devices.selected() {
                                    player.set_output_device(device);
                                    settings.current_device = (*device).to_string();
                                }
                            }
                            Mode::Playlist => playlist::on_enter(&mut playlist, &mut player),
                        },
                        KeyCode::Backspace => match mode {
                            Mode::Search => search::on_backspace(&mut search, control),
                            Mode::Playlist => playlist::on_backspace(&mut playlist, control),
                            _ => (),
                        },
                        KeyCode::Up => input.up(),
                        KeyCode::Down => input.down(),
                        KeyCode::Left => input.left(),
                        KeyCode::Right => input.right(),
                        KeyCode::Char('1' | '!') => {
                            queue::constraint(&mut queue, 0, shift);
                        }
                        KeyCode::Char('2' | '@') => {
                            queue::constraint(&mut queue, 1, shift);
                        }
                        KeyCode::Char('3' | '#') => {
                            queue::constraint(&mut queue, 2, shift);
                        }
                        KeyCode::Char(c) => match c {
                            'h' => input.left(),
                            'j' => input.down(),
                            'k' => input.up(),
                            'l' => input.right(),
                            _ => (),
                        },
                        _ => (),
                    }
                }
                Event::FocusGained => focused = true,
                Event::FocusLost => focused = false,
                Event::Mouse(event) => match event.kind {
                    MouseEventKind::ScrollUp => input.up(),
                    MouseEventKind::ScrollDown => input.down(),
                    MouseEventKind::Down(_) => match mode {
                        Mode::Browser => {
                            terminal
                                .draw(|f| {
                                    let top = draw_log(f);
                                    browser::draw(&mut browser, top, f, Some(event));
                                })
                                .unwrap();
                        }
                        Mode::Queue => {
                            if focused {
                                terminal
                                    .draw(|f| queue::draw(&mut queue, &mut player, f, Some(event)))
                                    .unwrap();
                            }
                        }
                        Mode::Playlist => {
                            terminal
                                .draw(|f| {
                                    let top = draw_log(f);
                                    playlist::draw(&mut playlist, top, f, Some(event));
                                })
                                .unwrap();
                        }
                        Mode::Search => {
                            terminal
                                .draw(|f| {
                                    let top = draw_log(f);
                                    search::draw(&mut search, top, f, Some(event));
                                })
                                .unwrap();
                        }
                        Mode::Settings => (),
                    },
                    _ => (),
                },
                Event::Resize(_, _) => (),
                Event::Paste(_) => (),
            }
        }
    }

    Database::save_queue(
        &player.songs.data,
        player.songs.index().unwrap_or(0) as u16,
        player.elapsed().as_secs_f32(),
    );

    disable_raw_mode().unwrap();
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .unwrap();

    gonk_core::profiler::print();
}
