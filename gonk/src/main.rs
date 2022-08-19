#![warn(clippy::pedantic)]
#![allow(
    clippy::wildcard_imports,
    clippy::float_cmp,
    clippy::cast_lossless,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::match_same_arms
)]
use browser::Browser;
use crossterm::{event::*, terminal::*, *};
use gonk_core::log;
use gonk_core::Index;
use gonk_player::Player;
use playlist::{Mode as PlaylistMode, Playlist};
use queue::Queue;
use search::{Mode as SearchMode, Search};
use settings::Settings;
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
    gonk_core::update_queue(
        &player.songs.data,
        player.songs.index().unwrap_or(0) as u16,
        player.elapsed().as_secs_f32(),
    );
}

fn save_queue_state(player: &Player) {
    gonk_core::update_queue_state(
        player.songs.index().unwrap_or(0) as u16,
        player.elapsed().as_secs_f32(),
    );
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
    gonk_core::init();

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
                    gonk_core::update_music_folder(path.as_str());
                    scan_handle = Some(gonk_core::scan(path));
                } else {
                    return println!("Invalid path.");
                }
            }
            "reset" => {
                return match gonk_core::reset() {
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

    let (songs, index, elapsed) = gonk_core::get_queue();
    let songs = Index::new(songs, index);
    let volume = gonk_core::volume();
    let device = gonk_core::output_device();
    let ui_index = index.unwrap_or(0);
    let mut player = Player::new(device, volume, songs, elapsed);

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

    //If there are songs in the queue and the database isn't scanning, display the queue.
    if !player.songs.is_empty() && scan_handle.is_none() {
        mode = Mode::Queue;
    }

    loop {
        if let Some(h) = &scan_handle {
            if h.is_finished() {
                browser::refresh(&mut browser);
                search::refresh_cache(&mut search);
                search::refresh_results(&mut search);

                if let Some(time) = scan_timer {
                    log!(
                        "Finished adding {} files in {:.2} seconds.",
                        gonk_core::len(),
                        time.elapsed().as_secs_f32()
                    );
                }

                scan_timer = None;
                scan_handle = None;
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

        if last_tick.elapsed() >= Duration::from_millis(150) {
            if busy && scan_timer.is_some() {
                if dots < 3 {
                    dots += 1;
                } else {
                    dots = 1;
                }
                log!("Scanning for files{}", ".".repeat(dots));
            }
            last_tick = Instant::now();
        }

        queue.len = player.songs.len();

        if player.update() {
            save_queue_state(&player);
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
                            save_queue(&player);
                        }
                        KeyCode::Char('c') => {
                            player.clear();
                            queue.ui.select(Some(0));
                            save_queue(&player);
                        }
                        KeyCode::Char('x') => match mode {
                            Mode::Queue => {
                                queue::delete(&mut queue, &mut player);
                                save_queue(&player);
                            }
                            Mode::Playlist => {
                                playlist::delete(&mut playlist, false);
                                save_queue(&player);
                            }
                            _ => (),
                        },
                        KeyCode::Char('X') => {
                            if let Mode::Playlist = mode {
                                playlist::delete(&mut playlist, true);
                            }
                        }
                        KeyCode::Char('u') if mode == Mode::Browser || mode == Mode::Playlist => {
                            let folder = gonk_core::music_folder().to_string();
                            scan_handle = Some(gonk_core::scan(folder));
                            playlist.playlists = Index::from(gonk_core::playlists());
                        }
                        KeyCode::Char('q') => player.seek_backward(),
                        KeyCode::Char('e') => player.seek_foward(),
                        KeyCode::Char('a') => {
                            player.prev();
                            save_queue_state(&player);
                        }
                        KeyCode::Char('d') => {
                            player.next();
                            save_queue_state(&player);
                        }
                        KeyCode::Char('w') => {
                            player.volume_up();
                            gonk_core::save_volume(player.volume);
                        }
                        KeyCode::Char('s') => {
                            player.volume_down();
                            gonk_core::save_volume(player.volume);
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
                                playlist::add(&mut playlist, &songs);
                                mode = Mode::Playlist;
                            }
                            Mode::Queue => {
                                if let Some(song) = player.songs.selected() {
                                    playlist::add(&mut playlist, &[song.clone()]);
                                    mode = Mode::Playlist;
                                }
                            }
                            Mode::Search => {
                                if let Some(songs) = search::on_enter(&mut search) {
                                    playlist::add(&mut playlist, &songs);
                                    mode = Mode::Playlist;
                                }
                            }
                            _ => (),
                        },
                        KeyCode::Enter => match mode {
                            Mode::Browser => {
                                let songs = browser::get_selected(&browser);
                                match player.add(&songs) {
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
                                    match player.add(&songs) {
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
                            terminal
                                .draw(|f| queue::draw(&mut queue, &mut player, f, Some(event)))
                                .unwrap();
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
                _ => (),
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
