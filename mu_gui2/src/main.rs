#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unused)]
use std::{
    thread::JoinHandle,
    time::{Duration, Instant},
};

use mu_core::{Album, db::Artwork};
use neoui::*;

pub mod settings;
pub use settings::*;
pub mod queue;
pub use queue::*;

pub const GRAY_50: u32 = hex("#f9f6f4");
pub const GRAY_100: u32 = hex("#ede9e5");
pub const GRAY_200: u32 = hex("#cbc7c4");
pub const GRAY_300: u32 = hex("#aaa6a2");
pub const GRAY_400: u32 = hex("#827f7c");
pub const GRAY_500: u32 = hex("#5c5957");
pub const GRAY_600: u32 = hex("#403f3e");
pub const GRAY_700: u32 = hex("#2a2929");
pub const GRAY_800: u32 = hex("#1b1b1c");
pub const GRAY_850: u32 = hex("#201f20");
pub const GRAY_900: u32 = hex("#101011");
pub const GRAY_950: u32 = hex("#0b0b0c");

pub const ACCENT_50: u32 = hex("#f6f3ff");
pub const ACCENT_100: u32 = hex("#e6e0fd");
pub const ACCENT_200: u32 = hex("#cdc2f4");
pub const ACCENT_300: u32 = hex("#b9a8ee");
pub const ACCENT_400: u32 = hex("#ad98e2");
pub const ACCENT_500: u32 = hex("#9b84d9");
pub const ACCENT_600: u32 = hex("#8871c6");
pub const ACCENT_700: u32 = hex("#69559b");
pub const ACCENT_800: u32 = hex("#463968");
pub const ACCENT_900: u32 = hex("#261f39");
pub const ACCENT_950: u32 = hex("#100d1b");

pub const GRAY_100_A4: u32 = hex("#ede9e509");
pub const GRAY_100_A8: u32 = hex("#ede9e512");
pub const GRAY_100_A10: u32 = hex("#ede9e51a");
pub const GRAY_100_A15: u32 = hex("#ede9e51f");
pub const GRAY_100_A60: u32 = hex("#ede9e599");
pub const GRAY_100_A65: u32 = hex("#ede9e5a6");
pub const ACCENT_A22: u32 = hex("#9b84d938");

pub const BODY: u32 = GRAY_950;
pub const SIDEBAR: u32 = GRAY_900;
pub const ROW_SELECTED: u32 = GRAY_850;
pub const BORDER_DIM: u32 = GRAY_800;
pub const TEXT_FAINT: u32 = GRAY_600;
pub const TEXT_MUTED: u32 = GRAY_500;
pub const TEXT_TERTIARY: u32 = GRAY_100_A60;
pub const PLAY_HOVER: u32 = GRAY_300;
pub const TEXT_SECONDARY: u32 = GRAY_200;
pub const TEXT: u32 = GRAY_100;
pub const KNOB: u32 = GRAY_100;

pub const BORDER: u32 = GRAY_100_A10;
pub const BORDER_SUBTLE: u32 = GRAY_100_A8;
pub const TRACK_EMPTY: u32 = GRAY_100_A15;
pub const TRACK_FILL: u32 = GRAY_100_A65;
pub const ROW_HOVER: u32 = GRAY_100_A4;

pub const ACCENT: u32 = ACCENT_500;
pub const ACCENT_HOVER: u32 = ACCENT_400;
pub const ACCENT_PRESSED: u32 = ACCENT_600;
pub const ACCENT_SOFT: u32 = ACCENT_A22;

const ALPHABET: &[&str] = &[
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z",
];

fn icon(ui: &mut FrameContext, kind: &str, style: RectStyle) -> State {
    ui.widget(24, 24, &style.layout, &style.paint, |ui, r, _, depth| {
        let fill = style.paint.fg.unwrap_or(white());
        let palette = [fill, with_alpha(fill, 110), SIDEBAR];
        let u = |v: i32| v * r.width / 24;

        #[rustfmt::skip]
        let bars: &[[i32; 5]] = match kind {
            "Panel"    => &[[2,4,20,16,0], [4,6,16,12,2], [4,6,5,12,0]],
            "Library"  => &[[3,4,4,17,0], [9,4,4,17,0], [15,6,4,15,0], [19,7,3,14,1]],
            "Queue"    => &[[3,6,12,2,0], [3,11,12,2,0], [3,16,8,2,1]],
            "Playlist" => &[[3,6,15,2,0], [3,11,15,2,0], [3,16,9,2,1], [15,16,7,2,0], [17,14,2,7,0]],
            "Settings" => &[[5,3,2,18,1], [12,3,2,18,1], [19,3,2,18,1], [3,7,6,3,0], [10,13,6,3,0], [17,6,6,3,0]],
            "Shuffle"  => &[[4,7,13,2,0], [7,15,13,2,0]],
            "Pause"    => &[[7,4,4,16,0], [13,4,4,16,0]],
            "Volume"   => &[[3,9,4,6,0], [14,9,2,6,1]],
            _ => &[],
        };

        for &[x, y, w, h, c] in bars {
            let bar = Rect::new(r.x + u(x), r.y + u(y), u(w).max(1), u(h).max(1));
            ui.paint_rect(bar, rect().bg(palette[c as usize]).radius(1).depth(depth));
        }

        let (x, y) = (r.x, r.y);
        let tri = |ui: &mut FrameContext, a: (i32, i32), b: (i32, i32), c: (i32, i32)| {
            let p = |(px, py): (i32, i32)| (x + u(px), y + u(py));
            ui.paint_triangle(p(a), p(b), p(c), rect().bg(fill).depth(depth));
        };
        match kind {
            "Queue" => tri(ui, (15, 11), (15, 20), (22, 15)),
            "Shuffle" => {
                tri(ui, (16, 4), (16, 12), (21, 8));
                tri(ui, (8, 12), (8, 20), (3, 16));
            }
            "Play" => tri(ui, (6, 3), (6, 21), (21, 12)),
            "Rewind" => {
                tri(ui, (11, 3), (11, 21), (1, 12));
                tri(ui, (22, 3), (22, 21), (12, 12));
            }
            "Forward" => {
                tri(ui, (13, 3), (13, 21), (23, 12));
                tri(ui, (2, 3), (2, 21), (12, 12));
            }
            "Volume" => tri(ui, (11, 3), (11, 21), (6, 12)),
            "Repeat" => {
                let ring = Rect::new(x + u(3), y + u(3), u(18), u(18));
                let stroke = rect()
                    .border(fill)
                    .border_thickness(u(2).max(1) as usize)
                    .radius(u(9).max(1) as usize)
                    .depth(depth);
                //Draw the ring twice, clipped, so everything but the top right is covered.
                ui.clipped(Rect::new(x, y, u(12), u(24)), |ui| ui.paint_rect(ring, stroke));
                ui.clipped(Rect::new(x + u(12), y + u(9), u(12), u(15)), |ui| {
                    ui.paint_rect(ring, stroke)
                });
                tri(ui, (12, 1), (12, 7), (18, 4));
            }
            _ => {}
        }
    })
}

struct Sidebar<'a> {
    bounds: Rect,
    // panel_left: &'a Image,
    artists: &'a [String],
    selected_artist: &'a str,
    selected_mode: &'a str,
    current_letter: Option<char>,
    active: bool,
    update_library: bool,
    artist_scroll: Scroll,
    jump_to_letter: Option<char>,
}

fn draw_rail(sidebar: &mut Sidebar, ui: &mut FrameContext) {
    ui.flow_down(
        flow()
            .bounds(sidebar.bounds)
            .bg(SIDEBAR)
            .border(BORDER_DIM)
            .border_side(RIGHT)
            .padtb(14)
            .padlr(11)
            .gap(4),
        |ui| {
            let btn = rect()
                .wh(34)
                .pad(7)
                .radius(8)
                .hover(ROW_HOVER)
                .selected(ROW_SELECTED);

            if icon(ui, "Panel", btn.fg(TEXT_TERTIARY)).clicked {
                sidebar.active = true;
            }

            ui.gap(6);

            for mode in ["Library", "Queue", "Playlist", "Settings"] {
                let selected = mode == sidebar.selected_mode;
                let btn = btn
                    .is_selected(selected)
                    .fg(if selected { TEXT } else { TEXT_TERTIARY });
                if icon(ui, mode, btn).clicked {
                    sidebar.selected_mode = mode;
                }
            }

            ui.gap(10);
            ui.rect(rect().height(1).width(Size::Fill).bg(BORDER_DIM));
        },
    );
}

fn draw_sidebar<'a, 'b: 'a>(sidebar: &mut Sidebar<'b>, ui: &mut FrameContext<'_, 'a>) {
    let sb = text().fg(TEXT).font_size(16);
    let state = ui.flow_down(
        flow()
            .bounds(sidebar.bounds)
            .bg(SIDEBAR)
            .border(BORDER_DIM)
            .border_side(RIGHT),
        |ui| {
            ui.flow_right(
                flow()
                    .padtb(20)
                    .padl(18)
                    .padr(10)
                    .height(48)
                    .children_center(),
                |ui| {
                    ui.text("mu", sb);
                    ui.gap(-28);
                    let btn = rect()
                        .wh(30)
                        .pad(5)
                        .radius(6)
                        .hover(ROW_HOVER)
                        .fg(TEXT_TERTIARY);
                    if icon(ui, "Panel", btn).clicked {
                        sidebar.active = false;
                    }
                },
            );

            ui.flow_down(flow().gap(2).padlr(8), |ui| {
                let mut item = |t: &'static str, i: &'static str| {
                    //TODO: Should use impl IntoColor to allow for Option or u32.
                    // sel.bg(if s { Some(ROW_SELECTED) } else { None });
                    let selected = t == sidebar.selected_mode;
                    let mut sel = flow().padlr(12).padtb(8).radius(6).hover(ROW_HOVER);
                    sel.paint.bg = if selected { Some(ROW_SELECTED) } else { None };
                    let text = sb.fg(if selected { TEXT } else { TEXT_TERTIARY });
                    let ntext = sb
                        .fg(if selected { TEXT_MUTED } else { TEXT_FAINT })
                        .fillw()
                        .content_right();

                    if ui
                        .flow_right(sel, |ui| {
                            ui.text(t, text);
                            ui.text(i, ntext);
                        })
                        .clicked
                    {
                        sidebar.selected_mode = t;
                    }
                };

                item("Library", "1");
                item("Queue", "2");
                item("Playlist", "3");
                item("Settings", "4");
                ui.gap(8);
            });

            ui.rect(rect().height(1).width(Size::Fill).bg(BORDER_DIM));

            let (artist, mut alphabet) = ui.split_h(-30);
            let selected_artist = sidebar.selected_artist;
            let top_of_artist_view = artist.y;
            let jump_target = sidebar.jump_to_letter.take();
            let mut jump_offset = None;

            let scroll_state = ui.scroll(
                flow().bounds(artist).padlr(8).elastic(true),
                &mut sidebar.artist_scroll,
                |ui| {
                    //Assuming artists is pre sorted alphabetically.
                    let mut first_letter = ' ';
                    // let mut top_letter = None;
                    sidebar.current_letter = None;
                    let text = sb
                        .padlr(12)
                        .padtb(8)
                        .radius(6)
                        .content_left()
                        .fillw()
                        .hover(ROW_HOVER);
                    let selected_text = text.bg(ROW_SELECTED);

                    for artist in sidebar.artists {
                        let next = artist.chars().next().unwrap().to_ascii_uppercase();
                        if next != first_letter {
                            first_letter = next;
                            let l = sb.padlr(12).padtb(8).font_size(12).fg(TEXT_MUTED);
                            let frame = ui.current_frame();
                            if sidebar.current_letter.is_none()
                                || frame.cursor_y - frame.scroll_y <= top_of_artist_view
                            {
                                sidebar.current_letter = Some(first_letter);
                            }
                            if let Some(target) = jump_target
                                && first_letter == target.to_ascii_uppercase()
                                && jump_offset.is_none()
                            {
                                jump_offset = Some((frame.cursor_y - frame.inner_bounds.y) as f32);
                            }
                            ui.text(first_letter.to_string(), l);
                        }
                        let sel = *artist == selected_artist;
                        let state = ui.text(artist, if sel { selected_text } else { text });
                        if state.clicked {
                            sidebar.selected_artist = artist;
                            sidebar.update_library = true;
                        }
                    }
                },
            );

            if let Some(offset) = jump_offset {
                //TODO: This should not allow for jumping out of bounds.
                //TODO: Should also have some momentum when jumping around.
                //Currently just a fixed jump.
                sidebar.artist_scroll.jump(offset);
            }

            ui.paint_rect(alphabet, rect().border(BORDER_DIM).border_side(LEFT));

            if let Some(raw_pct) = ui.drag_percentage_y(alphabet) {
                let pct = ((raw_pct - 0.03) / 0.90).clamp(0.0, 1.0);
                sidebar
                    .artist_scroll
                    .jump(pct * scroll_state.max_scroll as f32);
            }

            let hovered = ui.hovered(alphabet);
            let fade = ui.animate_f32(if hovered { 1.0 } else { 0.0 }, 0.15, Ease::InOutSine);
            let my = ui.mouse_position().y;
            let glow = |a: f32| rgba(155, 132, 217, (a * fade * 255.0) as u8);
            if fade > 0.0 {
                ui.place_down(flow().bounds(alphabet).clip(true), |ui| {
                    ui.gradient(
                        rect()
                            .x(alphabet.x)
                            .y(my.saturating_sub(55))
                            .width(alphabet.width)
                            .height(110),
                        180.0,
                    )
                    .stop(0.0, glow(0.0))
                    .stop(0.21, glow(0.11))
                    .stop(0.5, glow(0.30))
                    .stop(0.79, glow(0.11))
                    .stop(1.0, glow(0.0));

                    ui.gradient(rect().x(alphabet.x).y(my - 70).width(1).height(140), 180.0)
                        .stop(0.0, glow(0.0))
                        .stop(0.5, rgba(199, 183, 240, (0.75 * fade * 255.0) as u8))
                        .stop(1.0, glow(0.0));
                });
            }
            alphabet.x += 12;
            ui.flow_down(flow().bounds(alphabet), |ui| {
                let row = ui
                    .measure_text("A", Font::default(), 10, None, i32::MAX)
                    .height;
                ui.gap((ui.current_frame_bounds().height - row * 26) / 2);

                let Some(current_letter) = sidebar.current_letter else {
                    return;
                };

                for &letter in ALPHABET {
                    let ch = letter.chars().next().unwrap();
                    let dist = (ch as i32 - current_letter as i32).abs();
                    let color = match dist {
                        0 => ACCENT,
                        // 1 => TEXT,
                        _ => TEXT_TERTIARY,
                    };
                    ui.text(letter, sb.font_size(10).fg(color));
                }
            });
        },
    );

    if state.hovered {
        for key in ui.window.pressed_keys() {
            match key {
                Key::Char(c) => sidebar.jump_to_letter = Some(*c),
                _ => {}
            }
        }
    }
}

struct Library<'a> {
    bounds: Rect,
    artist: &'a str,
    total_tracks: usize,
    ///(Album, Song)
    playing_song: Option<(usize, usize)>,
    ///(Album, Song)
    selected_song: Option<(usize, usize)>,
    scroll: Scroll,
    update_playing: bool,
}

fn draw_library<'a, 'b: 'a>(
    albums: &'a [mu_core::Album],
    library: &mut Library<'b>,
    // controls: &mut Controls,
    ui: &mut FrameContext<'_, 'a>,
) {
    ui.flow_down(
        flow().bounds(library.bounds).padtb(12).padlr(36).bg(BODY),
        |ui| {
            ui.text(library.artist, text().font_size(42));
            //TODO: Add letter spacing?
            ui.gap(4);
            let header = ui.fmt(format_args!(
                "{} ALBUMS · {} TRACKS",
                albums.len(),
                library.total_tracks,
            ));
            ui.text(header, text().font_size(12).fg(TEXT_MUTED));
            ui.gap(12);
            ui.rect(rect().fillw().height(1).bg(BORDER_DIM));
            ui.gap(12);

            //TODO: Make mouse scroll better on elastic so it doesn't need to be disabled.
            let scroll_style = flow();

            // #[cfg(not(target_os = "windows"))]
            let scroll_style = flow().elastic(true);

            ui.scroll(scroll_style, &mut library.scroll, |ui| {
                let title_height = ui
                    .measure_text("A", Font::default(), 24, None, i32::MAX)
                    .height;
                let mut rendered = 0;

                for (ai, album) in albums.iter().enumerate() {
                    let rows = album.songs.len() as i32;
                    let row_gap = 2;
                    let row_height = 36;
                    let title_gap = 12;
                    let height =
                        (title_height + title_gap + rows * row_height + (rows - 1) * row_gap)
                            .max(148);

                    ui.flow_right(flow().height(height).fillw(), |ui| {
                        rendered += 1;
                        //In terms of API, images are always user retained.
                        //It's a bit painful to deal with for the current use case.
                        //Each library page can have an unbounded number of images.

                        if let Some(first) = &album.songs.first()
                            && let Some(Artwork::Decoded(pixels, width, height)) = &first.artwork
                        {
                            let img = Image {
                                width: *width,
                                height: *height,
                                pixels,
                            };
                            ui.image(img, image().radius(8).wh(148));
                        } else {
                            //TODO: Better placeholder
                            ui.rect(rect().wh(148).bg(BORDER_DIM));
                        }

                        ui.gap(24);

                        ui.flow_down(flow(), |ui| {
                            let tracks = if album.songs.len() > 1 {
                                "tracks"
                            } else {
                                "track"
                            };
                            let year = ui.fmt(format_args!(
                                "{} · {} {}",
                                album.year(),
                                album.songs.len(),
                                tracks
                            ));
                            ui.lines(
                                [
                                    line(&album.title, text().font_size(24).padr(12)),
                                    line(year, text().font_size(16).fg(TEXT_MUTED)),
                                ],
                                text().height(title_height),
                            );

                            ui.gap(title_gap);

                            let s = text()
                                .content_left()
                                .font_size(16)
                                .radius(12)
                                .padlr(6)
                                .padtb(4);

                            let row = flow()
                                .radius(12)
                                .padlr(6)
                                .padtb(4)
                                .hover(ROW_HOVER)
                                .height(row_height);

                            for (si, song) in album.songs.iter().enumerate() {
                                let playing = Some((ai, si)) == library.playing_song;
                                let selected = Some((ai, si)) == library.selected_song;

                                let row = if playing {
                                    row.bg(ROW_SELECTED)
                                } else if selected {
                                    //TODO: Maybe a dimmer version of this?
                                    row.bg(ROW_SELECTED)
                                } else {
                                    row
                                };

                                let song_row = ui.flow_right(row, |ui| {
                                    let track_number =
                                        ui.fmt(format_args!("{:02}", song.track_number));

                                    let number_color = if playing { ACCENT } else { TEXT_MUTED };
                                    ui.text(track_number, s.fg(number_color));

                                    let title_style =
                                        if playing { s } else { s.fg(TEXT_SECONDARY) };
                                    ui.text(&song.title, title_style);

                                    let duration = Duration::from_secs_f32(song.duration);
                                    let total_secs = duration.as_secs();
                                    let hours = total_secs / 3600;
                                    let minutes = (total_secs % 3600) / 60;
                                    let seconds = total_secs % 60;

                                    let duration = if hours > 0 {
                                        ui.fmt(format_args!(
                                            "{:02}:{:02}:{:02}",
                                            hours, minutes, seconds
                                        ))
                                    } else {
                                        ui.fmt(format_args!("{:02}:{:02}", minutes, seconds))
                                    };

                                    ui.text(duration, s.fg(TEXT_MUTED).fillw().content_right());
                                });

                                if song_row.clicked {
                                    library.selected_song = Some((ai, si));
                                }

                                if song_row.double_clicked {
                                    library.update_playing = true;
                                    library.playing_song = Some((ai, si));
                                }

                                if (si + 1) < album.songs.len() {
                                    ui.gap(row_gap)
                                }
                            }
                        });
                    });

                    if (ai + 1) < albums.len() {
                        ui.gap(24);
                        ui.rect(rect().fillw().bg(BORDER_DIM).h(1));
                        ui.gap(24);
                    }
                    // println!("rendered {}/{} albums", rendered, albums.len());
                }
            });
        },
    );
}

struct Controls {
    ///Artist, Album, Song
    song: Option<(String, usize, usize)>,
    bounds: Rect,
    playing: bool,
    shuffle: bool,
    repeat: bool,
    muted: bool,
    elapsed: f32,
    duration: f32,
    volume: u8,
}

fn time<'a>(ui: &mut FrameContext<'_, 'a>, t: f32) -> &'a str {
    let total_seconds = t.max(0.0) as u32;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    ui.fmt(format_args!("{:02}:{:02}", minutes, seconds))
}

fn draw_controls<'a>(
    controls: &mut Controls,
    player: &mut onmi::Player,
    db: &'a mu_core::vdb::Database,
    ui: &mut FrameContext<'_, 'a>,
) {
    ui.paint_rect(
        controls.bounds,
        rect().bg(SIDEBAR).border(BORDER_DIM).border_side(TOP),
    );

    let [info, center, extras] = ui.split_hs(controls.bounds, [0.28, 0.44, 0.28]);
    if let Some((artist, ai, si)) = &controls.song {
        let albums = db.albums_by_artist(artist);
        let song = &albums[*ai].songs[*si];
        let artwork = albums[*ai]
            .songs
            .first()
            .and_then(|song| song.artwork.as_ref());

        ui.flow_right(
            flow()
                .bounds(info)
                .padlr(16)
                .gap(12)
                .children_center()
                .clip(true),
            |ui| {
                if let Some(Artwork::Decoded(pixels, width, height)) = artwork {
                    let img = Image {
                        width: *width,
                        height: *height,
                        pixels,
                    };
                    ui.image(img, image().radius(6).wh(48));
                } else {
                    //TODO: Better placeholder
                    ui.rect(rect().wh(48).radius(4).bg(gray()));
                }
                ui.flow_down(flow().height(40), |ui| {
                    ui.text(&song.title, text().fg(TEXT));
                    let txt = ui.fmt(format_args!("{} · {}", song.artist, song.album));
                    ui.text(txt, text().font_size(14).fg(TEXT_MUTED));
                });
            },
        );
    }

    let t = text().w(36).font_size(13).fg(TEXT_MUTED);
    let btn = rect()
        .wh(32)
        .pad(4)
        .radius(8)
        .hover(ROW_HOVER)
        .fg(TEXT_TERTIARY);

    ui.flow_down(
        flow()
            .bounds(center)
            .padtb(12)
            .gap(4)
            .children_center()
            .clip(true),
        |ui| {
            ui.flow_right(
                flow().w(200).clip(true).h(36).gap(10).children_center(),
                |ui| {
                    icon(ui, "Shuffle", btn);
                    icon(ui, "Rewind", btn);
                    if icon(
                        ui,
                        if controls.playing { "Pause" } else { "Play" },
                        btn.bg(TEXT).hover(PLAY_HOVER).fg(SIDEBAR).radius(16),
                    )
                    .clicked
                    {
                        if controls.playing {
                            player.pause();
                        } else {
                            player.play();
                        }
                        controls.playing = !controls.playing;
                    }
                    icon(ui, "Forward", btn);
                    icon(ui, "Repeat", btn);
                },
            );

            ui.flow_right(flow().clip(true).h(20).gap(10).children_center(), |ui| {
                let elapsed = time(ui, controls.elapsed);
                ui.text(elapsed, t.content_right());
                let track = ui.rect(rect().w(Size::FillMinus(46)).h(4).bg(TRACK_EMPTY).radius(2));

                //Outset the seekbar verticall so it's easier to drag.
                let outset = track.bounds.outer(0, 12);

                //TODO: Dragged only works with released mouse input
                //So we have to duplicate the logic here.
                if controls.playing
                    && let Some(release) = ui.left_mouse_release
                    && release.intersects(ui.hit(outset))
                {
                    let x = ui.mouse_position().x.saturating_sub(outset.x);
                    let pos = (x as f32 / outset.width as f32).clamp(0.0, 1.0);
                    let pos = player.duration().as_secs_f32() * pos;
                    player.seek_to(Duration::from_secs_f32(pos));
                }

                let duration = time(ui, controls.duration);
                ui.text(duration, t.content_left());

                ui.paint_rect(
                    track.bounds.width(
                        (track.bounds.width as f32 * controls.elapsed / controls.duration) as i32,
                    ),
                    rect().bg(ACCENT).radius(2),
                );
            });
        },
    );

    ui.flow_left(
        flow()
            .bounds(extras)
            .padlr(16)
            .gap(10)
            .clip(true)
            .children_center(),
        |ui| {
            let volume = ui.fmt(format_args!("{}", controls.volume));
            ui.text(volume, text().w(24).font_size(13).fg(TEXT_MUTED));
            let slider = ui.rect(rect().w(96).h(4).radius(2).bg(TRACK_EMPTY));
            ui.paint_rect(
                slider
                    .bounds
                    .width((slider.bounds.width as f32 * controls.volume as f32 / 100.0) as i32),
                rect().bg(ACCENT).radius(2),
            );
            let outset = slider.bounds.outer(0, 12);
            if let Some(pos) = ui.drag_percentage_x(outset) {
                controls.volume = ((pos * 100.0) as u8).clamp(0, 100);
                player.set_volume(controls.volume);
            }
            icon(ui, "Volume", btn);
        },
    );
}

fn spawn_load_artwork(artist: String, mut albums: Vec<Album>) -> JoinHandle<(String, Vec<Album>)> {
    std::thread::spawn(move || {
        let now = Instant::now();
        // let threads = std::thread::available_parallelism().map_or(16, |n| n.get());
        let threads = 16;
        let chunk = albums.len().div_ceil(threads).max(1);

        std::thread::scope(|scope| {
            for albums in albums.chunks_mut(chunk) {
                scope.spawn(move || {
                    for a in albums {
                        //Use the first song for the whole album.
                        //Technically each track can have a different album cover.
                        if let Some(first) = a.songs.first_mut()
                            && first.artwork.is_none()
                            && let Ok(s) = onmi::metadata(&first.path, false, true)
                            && let Some(artwork) = s.artwork
                        {
                            //TODO: The point of the thumbnails is to allow users to cache a downscaled version
                            //Currently even though the image is being rendered at 120x120px.
                            //We need a high resolution version stored ???
                            if let Ok((pixels, width, height)) = image::decode(&artwork.data) {
                                let size = 512;
                                let pixels = image::resize(
                                    Image {
                                        pixels: &pixels,
                                        width,
                                        height,
                                    },
                                    size,
                                    size,
                                );
                                first.artwork =
                                    Some(Artwork::Decoded(pixels.into_boxed_slice(), size, size));
                            }
                        }
                    }
                });
            }
        });

        println!("Loaded {artist} in {}ms", now.elapsed().as_millis());

        (artist, albums)
    })
}

//TODO: Add tailwind style font size and padding builders.
//Allow the user to customize them.
//Currently keeping track of all the sizings is very difficult.
fn main() {
    defer_results!();

    // let config = mu_core::config_paths();
    // let s = mu_core::db::create("/Users/bay/Music/gdrive", config.database);
    // s.join().unwrap();

    let now = Instant::now();
    let player = std::thread::spawn(move || {
        let now = Instant::now();
        let outputs = onmi::OutputDevices::new();
        let player = onmi::Player::new(outputs.default_device());
        println!("Loaded Player in {}ms", now.elapsed().as_millis());
        player
    });

    let mut font = Some(std::thread::spawn(|| {
        let now = Instant::now();
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fonts")
            .join("NotoSansCJK-Subset.otf");
        let font = std::fs::read(path).unwrap();
        let f = fontdue::Font::from_bytes(font, fontdue::FontSettings::default()).unwrap();
        println!("Loaded Font in {}ms", now.elapsed().as_millis());
        f
    }));

    let db = std::thread::spawn(|| {
        let now = Instant::now();
        let config = mu_core::config_paths();
        let db = mu_core::vdb::Database::new(&config.database);
        let mut artists: Vec<String> = db.btree.keys().cloned().collect();
        artists.sort_by_key(|a| a.to_ascii_lowercase());
        println!("Loaded DB in {}ms", now.elapsed().as_millis());
        (db, artists)
    });

    let mut ui = ui("mu", 1200, 780);
    ui.default_font_size = 16;
    // ui.debug_damage = true;

    let mut player = player.join().unwrap();
    let (mut db, artists) = db.join().unwrap();

    let mut artwork_task: Option<JoinHandle<(String, Vec<Album>)>> = None;

    if let Some(albums) = db.btree.get("Duster").cloned() {
        artwork_task = Some(spawn_load_artwork("Duster".to_string(), albums));
    }

    println!("Loaded {}ms", now.elapsed().as_millis());

    let mut sidebar = Sidebar {
        bounds: Rect::default(),
        selected_artist: "Duster",
        artists: &artists,
        artist_scroll: Scroll::new(),
        active: true,
        // update_library: true,
        // selected_mode: "Library",
        update_library: false,
        selected_mode: "Queue",
        current_letter: None,
        jump_to_letter: None,
    };

    let mut library = Library {
        scroll: Scroll::new(),
        bounds: Rect::default(),
        total_tracks: db
            .albums_by_artist("Duster")
            .iter()
            .map(|a| a.songs.len())
            .sum(),
        artist: "Duster",
        playing_song: None,
        selected_song: None,
        update_playing: false,
    };

    let mut controls = Controls {
        song: None,
        bounds: Rect::default(),
        playing: false,
        shuffle: false,
        repeat: false,
        muted: false,
        elapsed: 0.0,
        duration: 0.0,
        volume: player.volume(),
    };

    let mut queue = Queue {
        // songs: db.albums_by_artist("Duster")[0].songs.clone(),
        songs: db
            .albums_by_artist("Duster")
            .iter()
            .map(|a| a.songs.clone())
            .flatten()
            .collect(),
        playing_song: None,
        bounds: Rect::default(),
        scroll: Scroll::new(),
        drag: None,
        playing_artist: Some("Duster"),
    };

    while ui.window.open() {
        if ui.window.pressed(Key::Escape) {
            ui.window.close();
        }

        for key in ui.window.pressed_keys() {
            match *key {
                Key::Char('1') => sidebar.selected_mode = "Library",
                Key::Char('2') => sidebar.selected_mode = "Queue",
                Key::Char('3') => sidebar.selected_mode = "Playlist",
                Key::Char('4') => sidebar.selected_mode = "Settings",
                Key::Tab => sidebar.active = !sidebar.active,
                //TODO: Should only trigger when library is focused.
                //Sidebar jumps on alphabet key press.
                Key::Char('W') => {
                    player.volume_up();
                    controls.volume = player.volume();
                }
                Key::Char('S') => {
                    player.volume_down();
                    controls.volume = player.volume()
                }
                Key::Char('E') => player.seek_forward(10.0),
                Key::Char('Q') => player.seek_backward(10.0),
                Key::Char('A') if let Some(current) = queue.playing_song => {
                    prev(current, &mut queue, &mut player);
                }
                Key::Char('D') if let Some(current) = queue.playing_song => {
                    next(current, &mut queue, &mut player);
                }
                Key::Space => {
                    player.toggle_playback();
                    controls.playing = !controls.playing
                }
                _ => {}
            }
        }

        if let Some(handle) = &artwork_task {
            if handle.is_finished() {
                let (artist, albums) = artwork_task.take().unwrap().join().unwrap();
                db.btree.insert(artist, albums);
            }
        }

        if let Some(f) = &font {
            if f.is_finished() {
                let font = font.take().unwrap().join().unwrap();
                ui.add_font_fallback(font);
            }
        }

        if library.update_playing {
            library.update_playing = false;

            let (ai, si) = library.selected_song.unwrap();
            let albums = db.albums_by_artist(library.artist);

            // How can we unify / simplify this a bit more?
            // queue.playing_song = todo!();

            //User is playing a different artist now.
            if queue.playing_artist != Some(library.artist) {
                queue.playing_artist = Some(library.artist);
                queue.songs = albums
                    .iter()
                    .flat_map(|a| a.songs.iter().cloned())
                    .collect();
            }

            let song = albums.get(ai).map(|a| a.songs.get(si)).flatten().unwrap();
            player.play_song(&song.path, Some(song.gain), true);

            //Library can change the selected artist so must clone here.
            //Also db is used mutabled so cannot borrow outside of the frame.
            controls.song = Some((library.artist.to_string(), ai, si));
            controls.playing = true;
        }

        //It's not as immediate, but easier than passing in db and library into sidebar.
        if sidebar.update_library {
            sidebar.update_library = false;
            sidebar.selected_mode = "Library";
            library.playing_song = None;
            library.selected_song = None;

            let artist = sidebar.selected_artist.to_string();

            //Restore the playing song state.
            if let Some((a, ai, si)) = &controls.song {
                if a == &artist {
                    library.playing_song = Some((*ai, *si));
                }
            }

            if let Some(albums) = db.btree.get(&artist).cloned() {
                artwork_task = Some(spawn_load_artwork(artist, albums));
            }

            library.scroll = Scroll::new();
            library.artist = sidebar.selected_artist;
            library.total_tracks = db
                .albums_by_artist(sidebar.selected_artist)
                .iter()
                .map(|a| a.songs.len())
                .sum();
        }

        if player.is_finished()
            && let Some((artist, ai, si)) = &mut controls.song
        {
            let current_len = db.albums_by_artist(artist)[*ai]
                .songs
                .len()
                .saturating_sub(1);
            let album_len = db.albums_by_artist(artist).len();

            if *si < current_len {
                *si += 1;
            } else if *ai < album_len.saturating_sub(1) {
                *ai += 1;
                *si = 0;
            } else {
                *ai = 0;
                *si = 0;
            }

            let song = &db.albums_by_artist(artist)[*ai].songs[*si];
            player.play_song(&song.path, Some(song.gain), true);
            if library.artist == artist {
                library.playing_song = Some((*ai, *si));
            }
        }

        controls.duration = player.duration().as_secs_f32();
        controls.elapsed = player.elapsed().as_secs_f32();

        ui.frame(|ui| {
            let target = if sidebar.active { 280.0 } else { 56.0 };
            let width = ui.animate_f32(target, 0.15, Ease::OutCubic) as i32;
            let (sb, body) = ui.split_h(width);
            sidebar.bounds = sb;

            if width > 168 {
                draw_sidebar(&mut sidebar, ui);
            } else {
                draw_rail(&mut sidebar, ui);
            }

            let (body, con) = ui.split_rect_v(body, -84);
            library.bounds = body;
            queue.bounds = body;
            controls.bounds = con;

            match sidebar.selected_mode {
                "Library" => draw_library(db.albums_by_artist(library.artist), &mut library, ui),
                "Queue" => draw_queue(ui, &mut queue, &db),
                "Playlist" => {}
                "Settings" => {}
                _ => unreachable!(),
            }

            draw_controls(&mut controls, &mut player, &db, ui);
        });
    }
}
