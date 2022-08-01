use browser::Browser;
use crossterm::{event::*, terminal::*, *};
use gonk_database::Index;
use gonk_player::Player;
use playlist::{Mode as PlaylistMode, Playlist};
use queue::Queue;
use search::{Mode as SearchMode, Search};
use settings::Settings;
use status_bar::StatusBar;
use std::{
    io::{stdout, Stdout},
    path::Path,
    thread,
    time::{Duration, Instant},
};
use tui::{backend::CrosstermBackend, layout::*, style::Color, Terminal};

mod browser;
mod log;
mod playlist;
mod queue;
mod search;
mod settings;
mod status_bar;
mod widgets;

type Frame<'a> = tui::Frame<'a, CrosstermBackend<Stdout>>;

pub struct Colors {
    pub number: Color,
    pub title: Color,
    pub album: Color,
    pub artist: Color,
    pub seeker: Color,
}

const COLORS: Colors = Colors {
    number: Color::Green,
    title: Color::Cyan,
    album: Color::Magenta,
    artist: Color::Blue,
    seeker: Color::White,
};

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

fn save_queue(player: &Player) {
    gonk_database::update_queue(
        &player.songs.data,
        player.songs.index().unwrap_or(0) as u16,
        player.elapsed().as_secs_f32(),
    );
}

fn main() {
    if gonk_database::init().is_err() {
        return println!("Database is corrupted! Please close all instances of gonk then relaunch or run `gonk reset`.");
    }

    log::init();
    let mut handle = None;

    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        match args[0].as_str() {
            "add" => {
                if args.len() == 1 {
                    return println!("Usage: gonk add <path>");
                }
                let path = args[1..].join(" ");
                if Path::new(&path).exists() {
                    gonk_database::update_music_folder(path.as_str());
                    handle = Some(gonk_database::scan(path));
                } else {
                    return println!("Invalid path.");
                }
            }
            "reset" => {
                return match gonk_database::reset() {
                    Ok(_) => println!("Files reset!"),
                    Err(e) => println!("Failed to reset database! {}", e),
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

    let (songs, index, elapsed) = gonk_database::get_queue();
    let songs = Index::new(songs, index);
    let volume = gonk_database::volume();
    let device = gonk_database::get_output_device();
    let player = thread::spawn(move || Player::new(device, volume, songs, elapsed));

    let mut browser = Browser::new();
    let mut queue = Queue::new();
    let mut status_bar = StatusBar::new();
    let mut playlist = Playlist::new();
    let mut settings = Settings::new();
    let mut search = Search::new();

    let mut mode = Mode::Browser;
    let mut last_tick = Instant::now();
    let mut busy = false;

    //TODO: Re-time if using another thread is faster after the rework.
    let mut player = player.join().unwrap();

    //If there are songs in the queue, display the queue.
    if !player.songs.is_empty() {
        mode = Mode::Queue;
    }

    loop {
        if let Some(h) = &handle {
            if h.is_finished() {
                browser::refresh(&mut browser);
                search::refresh_cache(&mut search);
                search::refresh_results(&mut search);
                handle = None;
            } else {
                busy = true;
            }
        } else {
            busy = false;
        }

        if last_tick.elapsed() >= Duration::from_millis(200) {
            //Update the status_bar at a constant rate.
            status_bar::update(&mut status_bar, busy, &player);
            last_tick = Instant::now();
        }

        queue.len = player.songs.len();

        match player.update() {
            Ok(_) => (),
            Err(e) => log!("{}", e),
        };

        terminal
            .draw(|f| {
                let area = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(2), Constraint::Length(3)])
                    .split(f.size());

                let (top, bottom) =
                    if status_bar.hidden || player.songs.is_empty() && log::message().is_none() {
                        (f.size(), Rect::default())
                    } else {
                        (area[0], area[1])
                    };

                match mode {
                    Mode::Browser => browser::draw(&browser, top, f),
                    Mode::Queue => queue::draw(&mut queue, &mut player, f, None),
                    Mode::Search => search::draw(&mut search, top, f),
                    Mode::Playlist => playlist::draw(&mut playlist, top, f),
                    Mode::Settings => settings::draw(&mut settings, top, f),
                };

                if log::message().is_some() {
                    log::draw(bottom, f);
                } else if mode != Mode::Queue {
                    status_bar::draw(&mut status_bar, bottom, f, busy, &player);
                }
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
                                search.query.push(c);
                                search.query_changed = true;
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
                        KeyCode::Char(' ') => match player.toggle_playback() {
                            Ok(_) => (),
                            Err(e) => log!("{}", e),
                        },
                        KeyCode::Char('C') if shift => {
                            player.clear_except_playing();
                            queue.ui.select(Some(0));
                        }
                        KeyCode::Char('c') => {
                            player.clear();
                            queue.ui.select(Some(0));
                        }
                        KeyCode::Char('x') => match mode {
                            Mode::Queue => queue::delete(&mut queue, &mut player),
                            Mode::Playlist => playlist::delete(&mut playlist, false),
                            _ => (),
                        },
                        KeyCode::Char('X') => {
                            if let Mode::Playlist = mode {
                                playlist::delete(&mut playlist, true)
                            }
                        }
                        KeyCode::Char('u') if mode == Mode::Browser => {
                            let folder = gonk_database::get_music_folder().to_string();
                            handle = Some(gonk_database::scan(folder));
                        }
                        KeyCode::Char('q') => match player.seek_by(-10.0) {
                            Ok(_) => (),
                            Err(e) => log!("{}", e),
                        },
                        KeyCode::Char('e') => match player.seek_by(10.0) {
                            Ok(_) => (),
                            Err(e) => log!("{}", e),
                        },
                        KeyCode::Char('a') => match player.previous() {
                            Ok(_) => (),
                            Err(e) => log!("{}", e),
                        },
                        KeyCode::Char('d') => match player.next() {
                            Ok(_) => (),
                            Err(e) => log!("{}", e),
                        },
                        KeyCode::Char('w') => {
                            player.volume_up();
                            gonk_database::update_volume(player.volume);
                        }
                        KeyCode::Char('s') => {
                            player.volume_down();
                            gonk_database::update_volume(player.volume);
                        }
                        //TODO: Rework mode changing buttons
                        KeyCode::Char('`') => {
                            status_bar.hidden = !status_bar.hidden;
                        }
                        KeyCode::Char(',') => mode = Mode::Playlist,
                        KeyCode::Char('.') => mode = Mode::Settings,
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
                            mode = match mode {
                                Mode::Browser | Mode::Settings | Mode::Search => Mode::Queue,
                                Mode::Queue | Mode::Playlist => Mode::Browser,
                            };
                        }
                        KeyCode::Esc => match mode {
                            Mode::Search => search::on_escape(&mut search),
                            Mode::Playlist => playlist::on_escape(&mut playlist),
                            _ => (),
                        },
                        KeyCode::Enter if shift => match mode {
                            Mode::Browser => {
                                let songs = browser::get_selected(&browser);
                                playlist::add_to_playlist(&mut playlist, &songs);
                                mode = Mode::Playlist;
                            }
                            Mode::Queue => {
                                if let Some(song) = player.songs.selected() {
                                    playlist::add_to_playlist(&mut playlist, &[song.clone()]);
                                    mode = Mode::Playlist;
                                }
                            }
                            Mode::Search => {
                                if let Some(songs) = search::on_enter(&mut search) {
                                    playlist::add_to_playlist(&mut playlist, &songs);
                                    mode = Mode::Playlist;
                                }
                            }
                            _ => (),
                        },
                        KeyCode::Enter => match mode {
                            Mode::Browser => {
                                let songs = browser::get_selected(&browser);
                                match player.add_songs(&songs) {
                                    Ok(_) => (),
                                    Err(e) => log!("{}", e),
                                }

                                save_queue(&player);
                            }
                            Mode::Queue => {
                                if let Some(i) = queue.ui.index() {
                                    match player.play_index(i) {
                                        Ok(_) => (),
                                        Err(e) => log!("{}", e),
                                    }
                                }
                            }
                            Mode::Search => {
                                if let Some(songs) = search::on_enter(&mut search) {
                                    match player.add_songs(&songs) {
                                        Ok(_) => (),
                                        Err(e) => log!("{}", e),
                                    }

                                    save_queue(&player);
                                }
                            }
                            Mode::Settings => settings::on_enter(&mut settings, &mut player),
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
                Event::Mouse(event) => match event.kind {
                    MouseEventKind::ScrollUp => input.up(),
                    MouseEventKind::ScrollDown => input.down(),
                    MouseEventKind::Down(_) => {
                        if let Mode::Queue = mode {
                            terminal
                                .draw(|f| queue::draw(&mut queue, &mut player, f, Some(event)))
                                .unwrap();
                        }
                    }
                    _ => (),
                },
                Event::Resize(..) => (),
            }
        }
    }

    save_queue(&player);

    disable_raw_mode().unwrap();
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .unwrap();
}
