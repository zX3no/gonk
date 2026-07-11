use mu_core::{vdb::*, *};
use neoui::*;
use onmi::{OutputDevices, Player};
use std::{
    fs,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Menu {
    File,
    Edit,
    View,
    Playback,
    Library,
    Help,
}

const fn dropdown_items(menu: Menu) -> &'static [&'static str] {
    match menu {
        Menu::File => &["New Project", "Open File...", "Save"],
        Menu::Edit => &["Undo", "Redo", "Cut"],
        Menu::View => &["Toggle Sidebar", "Zoom In"],
        Menu::Playback => &["Play / Pause", "Stop"],
        Menu::Library => &["Scan Folders..."],
        Menu::Help => &["Documentation", "About"],
    }
}

fn path(mut path: String) -> Option<std::path::PathBuf> {
    if path.contains("~") {
        path = path.replace("~", &user_profile_directory().unwrap());
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

fn main() {
    mini::defer_results!();
    mini::profile!();

    let config = config_paths();
    let mut persist = mu_core::settings::Settings::new(&config.settings).unwrap();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut scan_timer = Instant::now();
    let mut scan_handle = None;

    if !args.is_empty() {
        match args[0].as_str() {
            "add" => {
                if args.len() == 1 {
                    return println!("Usage: mu add <path>");
                }

                match path(args[1].clone()) {
                    Some(path) if path.exists() => {
                        persist.music_folder = path.to_string_lossy().to_string();
                        scan_handle =
                            Some(db::create(&persist.music_folder, config.database.clone()));
                        scan_timer = Instant::now();
                    }
                    _ => return println!("Invalid path."),
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
                println!("   mu [<command> <args>]");
                println!();
                println!("Options");
                println!("   add    <path> Add music to the library");
                println!("   reset         Reset the database");
                return;
            }
            _ if !args.is_empty() => return println!("Invalid command."),
            _ => (),
        }
    }

    //Prevents panic messages from being hidden.
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let mut stdout = std::io::stdout();
        let mut stdin = std::io::stdin();
        orig_hook(panic_info);
        std::process::exit(1);
    }));

    let index = (!persist.queue.is_empty()).then_some(persist.index as usize);
    let elapsed = persist.elapsed;
    let volume = persist.volume;
    let queue = persist.queue.clone();
    let mut songs = Index::new(queue, index);

    let outputs = OutputDevices::new();
    let devices = outputs.devices();
    let device = outputs
        .find(&persist.output_device)
        .unwrap_or(outputs.default_device());
    let mut player = Player::new(device.clone());

    // let mut settings = Settings::new(devices, device.name);

    //Takes ~5ms
    let db_path = config.database.clone();
    let db = std::thread::spawn(move || Database::new(&db_path));

    let mut last_tick = Instant::now();
    let mut ft = Instant::now();
    let mut dots: usize = 1;
    let mut help = false;
    let mut mute = false;
    let mut old_volume = 0;

    // player.set_volume(volume);
    // if let Some(song) = songs.selected() {
    //     play(&mut player, song, false);
    //     player.seek_to(Duration::from_secs_f32(elapsed));
    // }

    let mut db = db.join().unwrap();

    let mut ui = ui("mu", 1000, 700);
    ui.default_font_size = 13;

    let mut current_menu: Option<(Menu, Rect)> = None;
    let mut selected_song = 0;
    let mut volume = 0.5;
    let mut track_scroll = 0;
    let mut browser_scroll = 0;
    let mut seekbar_ratio = 0.0;

    let panel_bg = rgb(10, 10, 10);
    let border_color = rgb(45, 45, 45);
    let bar_color = rgb(66, 66, 66);
    let accent_blue = rgb(0, 102, 204);
    let text_dim = rgb(170, 170, 170);
    let menu_bg = rgb(25, 25, 25);
    let menu_hover = rgb(45, 45, 45);
    let items = [
        ("File", Menu::File),
        ("Edit", Menu::Edit),
        ("View", Menu::View),
        ("Playback", Menu::Playback),
        ("Library", Menu::Library),
        ("Help", Menu::Help),
    ];

    let artists = db.artists();

    let mut selected_artist = &String::new();

    let mut playlist: Index<Song> = Index::new(Vec::new(), None);

    while ui.window.open() {
        // if player.is_finished() && !songs.is_empty() {
        //     songs.down();
        //     if let Some(song) = songs.selected() {
        //         play(&mut player, &song, true)
        //     }
        // }
        if player.is_finished() && !playlist.is_empty() {
            playlist.down();
            if let Some(song) = playlist.selected() {
                play(&mut player, &song, true)
            }
        }

        ui.frame(|ui| {
            let (top_nav_rect, body) = ui.split_v(30);
            let (seekbar, body) = ui.split_rect_v(body, 24);
            let (sidebar_rect, track_rect) = ui.split_rect_h(body, 260);

            if let Some((menu, rect)) = current_menu {
                let item_style = style()
                    .width(180)
                    .padlr(12)
                    .padtb(8)
                    .bg(rgb(35, 35, 35))
                    .hover(rgb(60, 60, 60))
                    .align(Alignment::Left)
                    .depth(1);

                ui.flow_skip(style().x(rect.x).y(top_nav_rect.height), Flow::Down, |ui| {
                    for &item in dropdown_items(menu) {
                        if ui.item(item, false, item_style).clicked {
                            println!("{}", item);
                            current_menu = None;
                        }
                    }
                });

                //TODO: This could maybe be part of the response.
                if ui.lost_focus(rect) {
                    current_menu = None;
                }
            }

            ui.flow_right(bounds(top_nav_rect).bg(menu_bg), |ui| {
                for (label, menu) in items {
                    let state = ui.text(
                        label,
                        style()
                            .height(top_nav_rect.height)
                            .padl(14)
                            .padr(14)
                            .bg(menu_bg)
                            .hover(menu_hover),
                    );

                    if state.clicked {
                        if current_menu.is_some_and(|(cm, _)| cm == menu) {
                            current_menu = None;
                        } else {
                            current_menu = Some((menu, state.rect));
                        }
                    }
                }

                let bar = style().width(1).height(top_nav_rect.height).bg(bar_color);
                ui.rect(bar);
                ui.gap(120);
                ui.rect(bar);
                ui.gap(120);
                ui.rect(bar);
                ui.gap(120);
                ui.gap(-214);

                //Volume slider.
                {
                    let width = 200;
                    let height = top_nav_rect.height;
                    let rect = ui.walk_layout(width, height, 0).size;

                    ui.paint_rect(rect, bg(rgb(25, 25, 25)));

                    if let Some(percent) = ui.drag_percentage_x(rect) {
                        volume = percent;
                        //Don't go above 20% volume while testing :).
                        let volume = (volume * 20.0) as u8;
                        // eprintln!("Setting volume to {}", volume);
                        player.set_volume(volume);
                    }

                    let track_height = 6;
                    let cy = rect.y + height / 2;

                    ui.paint_triangle(
                        (rect.x, cy + track_height),
                        (rect.x + width, cy + track_height),
                        (rect.x + width, cy.saturating_sub(track_height)),
                        bg(black()),
                    );

                    let thumb_w = 12;
                    let thumb_h = 18;
                    let available_width = width.saturating_sub(thumb_w);
                    let thumb_x = rect.x + (volume * available_width as f32).round() as i32;
                    let thumb_y = rect.y + (height.saturating_sub(thumb_h)) / 2;
                    let thumb_color = rgb(0, 102, 204);

                    ui.paint_rect(
                        Rect::new(thumb_x, thumb_y, thumb_w, thumb_h),
                        bg(thumb_color),
                    );
                }
            });

            //Seekbar
            {
                ui.paint_rect(seekbar, bg(menu_bg));
                let inner = seekbar.inner(4, 6);
                ui.paint_rect(inner, bg(menu_bg).border(border_color));

                //Update during playback.
                let duration = player.duration().as_secs_f32();
                let elapsed = player.elapsed().as_secs_f32();
                let ratio = (elapsed.floor() / duration).clamp(0.0, 1.0);
                //TODO: This causes flicker since the player doesn't seek instantly :(
                seekbar_ratio = if ratio.is_nan() { 0.0 } else { ratio };

                //Only seek on release
                if ui.clicked(seekbar) {
                    if let Some((x, _)) = ui.window.mouse_pos() {
                        let x = (x as i32).saturating_sub(inner.x);
                        let ratio = (x as f32 / inner.width as f32).clamp(0.0, 1.0);
                        let pos = duration * ratio;
                        player.seek_to(Duration::from_secs_f32(pos));
                    }
                }

                //Update the scrollbar visually but don't update the player.
                if let Some(ratio) = ui.drag_percentage_x(inner) {
                    seekbar_ratio = ratio;
                }

                let x = inner.width as f32 * seekbar_ratio;
                let (w, h) = (11, 4);
                ui.paint_rect(
                    Rect::new(
                        x as i32,
                        seekbar.y + h / 2,
                        w,
                        seekbar.height.saturating_sub(h),
                    ),
                    bg(accent_blue),
                );
            }

            let row_style = style()
                .pad(8)
                .padl(12)
                .hover(rgb(35, 35, 35))
                .fill_width()
                .hover_border(rgb(90, 90, 90))
                .selected(rgb(82, 82, 82))
                .padl(12)
                .align(Alignment::Left)
                .selected_border(rgb(170, 170, 170));

            ui.scroll_view(
                bounds(sidebar_rect).bg(panel_bg),
                &mut browser_scroll,
                |ui| {
                    ui.text("All Music", style().fg(text_dim).pad(6));

                    for artist in &artists {
                        if ui.item(*artist, false, row_style).clicked {
                            selected_artist = artist;
                            playlist.clear();

                            let albums = db.albums_by_artist(artist);

                            for album in albums {
                                playlist.extend(album.songs.clone());
                            }

                            if !playlist.is_empty() {
                                playlist.select(Some(1));
                            }
                        }
                    }

                    ui.paint_rect(
                        sidebar_rect,
                        style().border(border_color).border_side(RIGHT),
                    );
                },
            );

            //This is kinda cursed.
            let (track_rect, scrollbar) = ui.split_rect_h(track_rect, Size::FillMinus(20));

            let state = ui.scroll_view(track_rect, &mut track_scroll, |ui| {
                ui.text(
                    selected_artist,
                    style()
                        .fg(accent_blue)
                        .font_size(14)
                        .padl(8)
                        .padb(4)
                        .height(24),
                );

                for (idx, track) in playlist.iter().enumerate() {
                    let label = format!("{}. {}", track.track_number, track.title);
                    if ui.item(label, idx == selected_song, row_style).clicked {
                        selected_song = idx;
                        play(&mut player, track, true);
                        // player.play_song(track.path, track.gain, true);
                    }
                }
            });

            {
                let s = scrollbar.inner(4, 0);
                let (y, h) = (s.y as f32, s.height as f32);
                let thumb_h = 80.0;
                // Calculate the exact space the bar can move within.
                let available_height = (h - thumb_h).max(0.0);
                let mut ratio = (track_scroll as f32 / state.max_scroll as f32).clamp(0.0, 1.0);

                if ui.dragged(scrollbar) {
                    // Offset the mouse position by half the bar height so the drag centers on the thumb.
                    let mousey = ui.mouse_position().y as f32 - y - (thumb_h / 2.0);
                    ratio = (mousey / available_height).clamp(0.0, 1.0);
                    track_scroll = (ratio * state.max_scroll as f32).round() as usize;
                }

                let y = s.y + (ratio * available_height).round() as i32;
                let thumb = Rect::new(s.x, y, s.width, thumb_h as i32);
                ui.paint_rect(thumb, bg(rgb(80, 80, 80)));
            }
        });
    }
}
