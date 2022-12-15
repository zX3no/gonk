use browser::Browser;
use crossterm::{event::*, terminal::*, *};
use gonk_core::{vdb, *};
use gonk_player::Player;
use playlist::{Mode as PlaylistMode, Playlist};
use queue::Queue;
use search::{Mode as SearchMode, Search};
use settings::Settings;
use std::{error::Error, fs, ptr::addr_of_mut};
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

pub trait Widget {
    fn up(&mut self);
    fn down(&mut self);
    fn left(&mut self);
    fn right(&mut self);
    fn draw(&mut self, f: &mut Frame, area: Rect, mouse_event: Option<MouseEvent>);
}

fn draw_log(f: &mut Frame) -> Rect {
    if let Some(msg) = log::last_message() {
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

static mut VDB: Lazy<vdb::Database> = Lazy::new(|| vdb::create().unwrap());

fn main() -> std::result::Result<(), Box<dyn Error + Send + Sync>> {
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
                if Path::new(&path).exists() {
                    persist.music_folder = path.clone();
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

    //Disable raw mode when the program panics.
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        disable_raw_mode().unwrap();
        execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
        orig_hook(panic_info);
        std::process::exit(1);
    }));

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture,
    )?;
    enable_raw_mode()?;
    terminal.clear()?;

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

    //TODO: Why does this need to exist?
    let mut queue = Queue::new(ui_index, addr_of_mut!(player));

    let mut browser = Browser::new();
    let mut playlist = Playlist::new();
    let mut settings = Settings::new(&persist.output_device);
    let mut search = Search::new();

    let mut mode = Mode::Browser;
    let mut last_tick = Instant::now();
    let mut dots: usize = 1;

    //If there are songs in the queue and the database isn't scanning, display the queue.
    if !player.songs.is_empty() && scan_handle.is_none() {
        mode = Mode::Queue;
    }

    let mut search_timeout = Instant::now();

    loop {
        profile!("loop");
        if let Some(handle) = &scan_handle {
            if handle.is_finished() {
                let handle = scan_handle.take().unwrap();
                let result = handle.join().unwrap();
                unsafe { *VDB = vdb::create()? };

                log::clear();

                match result {
                    db::ScanResult::Completed => {
                        log!(
                            "Finished adding {} files in {:.2} seconds.",
                            db::len(),
                            scan_timer.elapsed().as_secs_f32()
                        );
                    }
                    db::ScanResult::CompletedWithErrors(errors) => {
                        #[cfg(windows)]
                        let dir = "See %appdata%/gonk/gonk.log for details.";

                        #[cfg(unix)]
                        let dir = "See .config/gonk/gonk.log for details.";

                        let len = errors.len();
                        let s = if len == 1 { "" } else { "s" };

                        log!(
                            "Added {} files with {len} error{s}. {dir}",
                            db::len().saturating_sub(len)
                        );

                        let path = gonk_path().join("gonk.log");
                        let errors = errors.join("\n");
                        fs::write(path, errors)?;
                    }
                    db::ScanResult::FileInUse => {
                        log!("Could not update database, file in use.")
                    }
                }

                browser::refresh(&mut browser);
                search.results = Index::new(unsafe { vdb::search(&VDB, &search.query) }, Some(0));

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

            //Update the list of output devices.
            settings.update();

            last_tick = Instant::now();
        }

        //Update the UI index.
        queue.len = player.songs.len();

        player.update();

        let input_search = search.mode == SearchMode::Search && mode == Mode::Search;
        let input_playlist = playlist.mode == PlaylistMode::Popup && mode == Mode::Playlist;

        let input = match mode {
            Mode::Browser => &mut browser as &mut dyn Widget,
            Mode::Queue => &mut queue as &mut dyn Widget,
            Mode::Search => &mut search as &mut dyn Widget,
            Mode::Playlist => &mut playlist as &mut dyn Widget,
            Mode::Settings => &mut settings as &mut dyn Widget,
        };

        let _ = terminal.draw(|f| {
            let top = draw_log(f);
            input.draw(f, top, None);
        });

        if !event::poll(Duration::from_millis(2))? {
            continue;
        }

        match event::read()? {
            Event::Key(event) => {
                let shift = event.modifiers == KeyModifiers::SHIFT;
                let control = event.modifiers == KeyModifiers::CONTROL;

                match event.code {
                    KeyCode::Char('c') if control => break,
                    KeyCode::Char(c) if input_search => {
                        //Handle ^W as control backspace.
                        if control && c == 'w' {
                            search::on_backspace(&mut search, true);
                        } else if search_timeout.elapsed() < Duration::from_millis(350) {
                            //I have mixed feelings about this.
                            match c {
                                '1' => mode = Mode::Queue,
                                '2' => mode = Mode::Browser,
                                '3' => mode = Mode::Playlist,
                                '4' => mode = Mode::Settings,
                                '/' => (),
                                _ => {
                                    search.query.push(c);
                                    search.query_changed = true;
                                }
                            };
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
                            if persist.music_folder.is_empty() {
                                gonk_core::log!(
                                    "Nothing to scan! Add a folder with 'gonk add /path/'"
                                );
                            } else {
                                scan_handle = Some(db::create(persist.music_folder.clone()));
                                scan_timer = Instant::now();
                                playlist.lists = Index::from(gonk_core::playlist::playlists()?);
                            }
                        }
                    }
                    KeyCode::Char('q') => player.seek_backward(),
                    KeyCode::Char('e') => player.seek_foward(),
                    KeyCode::Char('a') => player.prev(),
                    KeyCode::Char('d') => player.next(),
                    KeyCode::Char('w') => {
                        player.volume_up();
                        persist.volume = player.volume();
                    }
                    KeyCode::Char('s') => {
                        player.volume_down();
                        persist.volume = player.volume();
                    }
                    KeyCode::Char('/') => {
                        if mode == Mode::Search {
                            if search.mode == SearchMode::Select {
                                search.results.select(None);
                                search.mode = SearchMode::Search;
                            }
                        } else {
                            search_timeout = Instant::now();
                            mode = Mode::Search;
                        }
                    }
                    KeyCode::Tab => {
                        terminal.clear()?;
                        mode = match mode {
                            Mode::Browser | Mode::Settings | Mode::Search => Mode::Queue,
                            Mode::Queue | Mode::Playlist => Mode::Browser,
                        };
                    }
                    KeyCode::Esc => match mode {
                        Mode::Search => match search.mode {
                            search::Mode::Search => {
                                if let search::Mode::Search = search.mode {
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
                                if let Some(song) = player.songs.get(index) {
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
                    KeyCode::Char('1') => mode = Mode::Queue,
                    KeyCode::Char('2') => mode = Mode::Browser,
                    KeyCode::Char('3') => mode = Mode::Playlist,
                    KeyCode::Char('4') => mode = Mode::Settings,
                    KeyCode::F(1) => queue::constraint(&mut queue, 0, shift),
                    KeyCode::F(2) => queue::constraint(&mut queue, 1, shift),
                    KeyCode::F(3) => queue::constraint(&mut queue, 2, shift),
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
            Event::FocusGained => (),
            Event::FocusLost => (),
            Event::Mouse(mouse_event) => match mouse_event.kind {
                MouseEventKind::ScrollUp => input.up(),
                MouseEventKind::ScrollDown => input.down(),
                MouseEventKind::Down(_) => {
                    terminal.draw(|f| {
                        let top = draw_log(f);
                        input.draw(f, top, Some(mouse_event))
                    })?;
                }
                _ => (),
            },
            Event::Resize(_, _) => (),
            Event::Paste(_) => (),
        }

        //End of loop
    }

    persist.queue = (*player.songs).to_vec();
    persist.index = player.songs.index().unwrap_or(0) as u16;
    persist.elapsed = player.elapsed().as_secs_f32();
    persist.save()?;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    gonk_core::profiler::print();

    Ok(())
}
