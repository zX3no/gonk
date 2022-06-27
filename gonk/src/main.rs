use browser::Browser;
use crossterm::{event::*, terminal::*, *};
use gonk_database::{query, Database, State};
use gonk_player::Player;
use playlist::{Mode as PlaylistMode, Playlist};
use queue::Queue;
use search::{Mode as SearchMode, Search};
use settings::Settings;
use status_bar::StatusBar;
use std::{
    io::{stdout, Stdout},
    path::Path,
    sync::mpsc,
    time::{Duration, Instant},
};
use tui::{backend::CrosstermBackend, layout::*, style::Color, Terminal};

mod browser;
mod playlist;
mod queue;
mod search;
mod settings;
mod status_bar;
mod widgets;

type Frame<'a> = tui::Frame<'a, CrosstermBackend<Stdout>>;

//TODO: Cleanup colors
pub struct Colors {
    pub number: Color,
    pub name: Color,
    pub album: Color,
    pub artist: Color,
    pub seeker: Color,
}

const COLORS: Colors = Colors {
    number: Color::Green,
    name: Color::Cyan,
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

fn main() {
    let mut db = Database::default();
    let args: Vec<String> = std::env::args().skip(1).collect();

    if !args.is_empty() {
        match args[0].as_str() {
            "add" => {
                if args.len() == 1 {
                    return println!("Usage: gonk add <path>");
                }

                let path = args[1..].join(" ");
                //TODO: This might silently scan a directory but not add anything.
                //Might be confusing.
                if Path::new(&path).exists() {
                    db.add_path(&path);
                } else {
                    return println!("Invalid path.");
                }
            }
            //TODO: Add numbers to each path
            //so users can just write: gonk rm 3
            "rm" => {
                if args.len() == 1 {
                    return println!("Usage: gonk rm <path>");
                }

                let path = args[1..].join(" ");
                match query::remove_folder(&path) {
                    Ok(_) => return println!("Deleted path: {}", path),
                    Err(e) => return println!("{e}"),
                };
            }
            "list" => {
                return for path in query::folders() {
                    println!("{path}");
                };
            }
            "reset" => {
                return match gonk_database::reset() {
                    Ok(_) => println!("Files reset!"),
                    Err(e) => println!("{}", e),
                }
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

    //Player takes a while so off-load it to another thread.
    let (s, r) = mpsc::channel();

    //TODO: figure out why database is crashing
    let songs = query::get_cache();
    let volume = query::volume();

    std::thread::spawn(move || {
        let player = Player::new(volume, &songs);
        s.send(player).unwrap();
    });

    //Terminal.
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout())).unwrap();
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture,
    )
    .unwrap();
    enable_raw_mode().unwrap();
    terminal.clear().unwrap();

    //13ms
    let mut search = Search::new();
    let mut settings = Settings::new();
    let mut browser = Browser::new();
    let mut queue = Queue::new();
    let mut status_bar = StatusBar::new();
    let mut playlist = Playlist::new();

    let mut mode = Mode::Browser;

    let mut busy = false;
    let mut last_tick = Instant::now();

    let mut player = r.recv().unwrap();

    loop {
        match db.state() {
            State::Busy => busy = true,
            State::Idle => busy = false,
            State::NeedsUpdate => {
                browser::refresh(&mut browser);
                search::refresh_cache(&mut search);
                search::refresh_results(&mut search);
            }
        }

        if last_tick.elapsed() >= Duration::from_millis(200) {
            //Update the status_bar at a constant rate.
            status_bar::update(&mut status_bar, busy, &player);
            last_tick = Instant::now();
        }

        queue.len = player.songs.len();
        player.update();

        //Draw
        terminal
            .draw(|f| {
                let area = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(2), Constraint::Length(3)])
                    .split(f.size());

                let (top, bottom) = if status_bar.hidden {
                    (f.size(), area[1])
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

                if mode != Mode::Queue {
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
                                search.query_changed = true;
                                search.query.push(c);
                            }
                        }
                        KeyCode::Char(c) if input_playlist => {
                            if control && c == 'w' {
                                playlist::on_backspace(&mut playlist, true);
                            } else {
                                playlist.changed = true;
                                playlist.search.push(c);
                            }
                        }
                        KeyCode::Char(' ') => player.toggle_playback(),
                        KeyCode::Char('c') if shift => {
                            player.clear_except_playing();
                            queue.ui.select(Some(0));
                        }
                        KeyCode::Char('c') => {
                            player.clear();
                            queue.ui.select(Some(0));
                        }
                        KeyCode::Char('x') => match mode {
                            Mode::Queue => queue::delete(&mut queue, &mut player),
                            Mode::Playlist => playlist::delete(&mut playlist),
                            _ => (),
                        },
                        KeyCode::Char('u') if mode == Mode::Browser => db.refresh(),
                        KeyCode::Char('q') => player.seek_by(-10.0),
                        KeyCode::Char('e') => player.seek_by(10.0),
                        KeyCode::Char('a') => player.prev_song(),
                        KeyCode::Char('d') => player.next_song(),
                        KeyCode::Char('w') => player.volume_up(),
                        KeyCode::Char('s') => player.volume_down(),
                        KeyCode::Char('r') => player.randomize(),
                        //TODO: Rework mode changing buttons
                        KeyCode::Char('`') => {
                            status_bar.hidden = !status_bar.hidden;
                        }
                        KeyCode::Char(',') => mode = Mode::Playlist,
                        KeyCode::Char('.') => mode = Mode::Settings,
                        KeyCode::Char('/') => mode = Mode::Search,
                        KeyCode::Tab => {
                            mode = match mode {
                                Mode::Browser | Mode::Settings | Mode::Search => Mode::Queue,
                                Mode::Queue | Mode::Playlist => Mode::Browser,
                            };
                        }
                        KeyCode::Esc => match mode {
                            Mode::Search => {
                                search::on_escape(&mut search, &mut mode);
                            }
                            Mode::Settings => mode = Mode::Queue,
                            Mode::Playlist => playlist::on_escape(&mut playlist, &mut mode),
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
                            _ => (),
                        },
                        KeyCode::Enter => match mode {
                            Mode::Browser => {
                                let songs = browser::get_selected(&browser);
                                player.add_songs(&songs);
                            }
                            Mode::Queue => {
                                if let Some(i) = queue.ui.index() {
                                    player.play_song(i);
                                }
                            }
                            Mode::Search => search::on_enter(&mut search, &mut player),
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

    query::set_volume(player.volume);

    let ids: Vec<usize> = player.songs.data.iter().flat_map(|song| song.id).collect();
    query::cache(&ids);

    disable_raw_mode().unwrap();
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .unwrap();
}
