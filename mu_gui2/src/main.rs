#![allow(unused)]
use neoui::*;

const BODY: u32 = hex("#0b0b0c");
const SIDEBAR: u32 = hex("#101011");

const TEXT: u32 = hex("#EDE9E5");
const TEXT_SECONDARY: u32 = hex("#CAC8C4");
const TEXT_TERTIARY: u32 = hex("#ece9e499");
const TEXT_MUTED: u32 = hex("#555454");
const TEXT_FAINT: u32 = hex("#40403F");

const BORDER: u32 = hex("#ece9e41a");
const BORDER_SUBTLE: u32 = hex("#ece9e412");

const BORDER_DIM: u32 = hex("#1B1B1C");

const TRACK_EMPTY: u32 = hex("#ece9e41f");
const TRACK_FILL: u32 = hex("#ece9e4a6");
const KNOB: u32 = hex("#ece9e4");

const ROW_HOVER: u32 = hex("#ece9e409");
// const ROW_PLAYING: u32 = hex("#ece9e40a");
// const ROW_SELECTED: u32 = hex("#ece9e412");
// const ROW_ACTIVE: u32 = hex("#ece9e41a");
const ROW_SELECTED: u32 = hex("#201F20");

const ACCENT: u32 = hex("#9b84d9");
const ACCENT_HOVER: u32 = hex("#ad98e2");
const ACCENT_PRESSED: u32 = hex("#8871c6");
const ACCENT_SOFT: u32 = hex("#9b84d938");

const ARTISTS: &[&'static str] = &[
    "Arca",
    "BADBADNOTGOOD",
    "beabadoobee",
    "Björk",
    "black midi",
    "Bonobo",
    "C418",
    "Clarence Clarity",
    "Clown Core",
    "Corea",
    "Covet",
    "Daft Punk",
    "Death Grips",
    "Dorian Concept",
    "Duster",
    "EDEN",
    "eightiesheadachetape",
    "Eminem",
    "Flawed Mangoes",
    "Floating Points, Pharoah Sanders & The London Symphony Orchestra",
    "Flume",
    "Flying Lotus",
    "foxtails",
    "Funeral Diner",
    "Godspeed You! Black Emperor",
    "Gospel",
    "Hans Zimmer",
    "Iglooghost",
    "J-E-T-S",
    "Jakey",
    "John Coltrane",
    "Joji",
    "JPEGMAFIA",
    "Julie",
    "Kai Whiston",
    "Kanazu Tomoyuki",
    "Kanye West",
    "Kendrick Lamar",
    "kinoue64",
    "Koan Sound",
    "Komorebi",
    "Lena Raine",
    "LINGUA IGNOTA",
    "Machine Girl",
    "Machinedrum",
    "Madvillain",
    "mage tears",
    "Massive Attack",
    "Medasin",
    "Memo Boy",
    "Men I Trust",
    "Mick Gordon",
    "Miles Davis",
    "mouse on the keys",
    "my bloody valentine",
    "Nas",
    "Nirvana",
    "Nujabes",
    "Oli XL",
    "Otuka",
    "PinkPantheress",
    "Pokelawls",
    "Portishead",
    "Portraits of Past",
    "Puma Blue",
    "Rachel's",
    "Radiohead",
    "Ramin Djawadi",
    "Ryo Fukui",
    "Ryuichi Sakamoto",
    "Sam Gellaitry",
    "Seatbelts",
    "Sinjin Hawke",
    "Sinjin Hawke & Zora Jones",
    "Slauson Malone",
    "Slint",
    "STEINS;GATE",
    "Steve Reich",
    "Sweet Trip",
    "Tera Melos",
    "The Comet Is Coming",
    "Title Fight",
    "Toby Fox",
    "Travis Scott",
    "Tyler, The Creator",
    "Various Artists",
    "william crooks",
    "Yussef Dayes",
    "Øneheart", //Should be sorted as an O.
];

const ALPHABET: &[&str] = &[
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z",
];

fn icon(ui: &mut FrameContext, kind: &str, style: Style) -> State {
    ui.widget(24, 24, style, |ui, r, _, depth| {
        let fill = style.fg.unwrap_or(white());
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
            let rect = Rect::new(r.x + u(x), r.y + u(y), u(w).max(1), u(h).max(1));
            ui.paint_rect(rect, bg(palette[c as usize]).radius(1).depth(depth));
        }

        let (x, y) = (r.x, r.y);
        let tri = |ui: &mut FrameContext, a: (i32, i32), b: (i32, i32), c: (i32, i32)| {
            let p = |(px, py): (i32, i32)| (x + u(px), y + u(py));
            ui.paint_triangle(p(a), p(b), p(c), bg(fill).depth(depth));
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
                let stroke = Style::default()
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
    panel_left: &'a Image,
    artists: &'a [String],
    selected_artist: &'static str,
    selected_mode: &'static str,
    current_letter: Option<char>,
    active: bool,
    artist_scroll: Scroll,
}

fn draw_rail(sidebar: &mut Sidebar, ui: &mut FrameContext) {
    ui.flow_down(
        bounds(sidebar.bounds)
            .bg(SIDEBAR)
            .border(BORDER_DIM)
            .border_side(RIGHT)
            .padtb(14)
            .padlr(11)
            .gap(4),
        |ui| {
            let btn = style()
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
            ui.rect(style().height(1).width(Size::Fill).bg(BORDER_DIM));
        },
    );
}

fn draw_sidebar(sidebar: &mut Sidebar, ui: &mut FrameContext) {
    let sb = style().fg(TEXT).font_size(16);
    ui.flow_down(
        bounds(sidebar.bounds)
            .bg(SIDEBAR)
            .border(BORDER_DIM)
            .border_side(RIGHT),
        |ui| {
            ui.flow_right(
                sb.padtb(20)
                    .padl(18)
                    .padr(10)
                    .height(48)
                    .align_flow(AlignFlow::Center),
                |ui| {
                    ui.text("mu", sb);
                    ui.gap(-28);
                    let btn = style()
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

            ui.flow_down(sb.gap(2).padlr(8), |ui| {
                let mut item = |t: &'static str, i: &'static str| {
                    //TODO: Should use impl IntoColor to allow for Option or u32.
                    // sel.bg(if s { Some(ROW_SELECTED) } else { None });
                    let selected = t == sidebar.selected_mode;
                    let mut sel = sb.padlr(12).padtb(8).radius(6).hover(ROW_HOVER);
                    sel.bg = if selected { Some(ROW_SELECTED) } else { None };
                    let text = sb.fg(if selected { TEXT } else { TEXT_TERTIARY });
                    let ntext = sb
                        .fg(if selected { TEXT_MUTED } else { TEXT_FAINT })
                        .fill_width()
                        .align_right();

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

            ui.rect(style().height(1).width(Size::Fill).bg(BORDER_DIM));

            let (artist, mut alphabet) = ui.split_h(-30);
            let selected_artist = sidebar.selected_artist;
            let top_of_artist_view = artist.y;

            ui.scroll(
                bounds(artist).padlr(8).elastic(true),
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
                        .align_left()
                        .fill_width()
                        .hover(ROW_HOVER);
                    let selected_text = text.bg(ROW_SELECTED);

                    for artist in ARTISTS {
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
                            ui.text(first_letter.to_string(), l);
                        }
                        let sel = *artist == selected_artist;
                        ui.text(*artist, if sel { selected_text } else { text });
                    }
                },
            );

            ui.paint_rect(alphabet, style().border(BORDER_DIM).border_side(LEFT));
            let strip = alphabet;
            let hovered = ui.hovered(strip);
            let fade = ui.animate_f32(if hovered { 1.0 } else { 0.0 }, 0.15, Ease::InOutSine);
            let my = ui.mouse_position().y;
            let glow = |a: f32| rgba(155, 132, 217, (a * fade * 255.0) as u8);
            ui.place_down(bounds(strip).clip(true), |ui| {
                ui.gradient(
                    style()
                        .x(strip.x)
                        .y(my.saturating_sub(55))
                        .width(strip.width)
                        .height(110),
                    180.0,
                )
                .stop(0.0, glow(0.0))
                .stop(0.21, glow(0.11))
                .stop(0.5, glow(0.30))
                .stop(0.79, glow(0.11))
                .stop(1.0, glow(0.0));

                ui.gradient(style().x(strip.x).y(my - 70).width(1).height(140), 180.0)
                    .stop(0.0, glow(0.0))
                    .stop(0.5, rgba(199, 183, 240, (0.75 * fade * 255.0) as u8))
                    .stop(1.0, glow(0.0));
            });
            alphabet.x += 12;
            ui.flow_down(bounds(alphabet), |ui| {
                let row = ui.measure_text("A", Font::default(), 10, None).height;
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
}

struct Library<'a> {
    bounds: Rect,
    artist: &'a str,
}

fn draw_library<'a>(library: &mut Library<'a>, ui: &mut FrameContext<'_, 'a>) {
    ui.flow_down(bounds(library.bounds).padlr(36).padtb(24).bg(BODY), |ui| {
        ui.text(library.artist, style().font_size(42));
        //TODO: Add letter spacing?
        ui.gap(4);
        ui.text(
            "10 ALBUMS · 102 TRACKS · 6.1 GB LOCAL",
            style().font_size(12).fg(TEXT_MUTED),
        );
        ui.gap(12);
        ui.rect(style().fill_width().height(1).bg(BORDER_DIM));
        ui.gap(12);
        ui.flow_right(style(), |ui| {
            ui.rect(style().wh(120).bg(gray()));
            ui.gap(24);
            ui.flow_down(style(), |ui| {
                ui.line(
                    [
                        text("Apex, Trance-Like", style().font_size(24).padr(12)),
                        text("1998 · 2 tracks", style().font_size(16).fg(TEXT_MUTED)),
                    ],
                    style(),
                );
                ui.gap(12);
                let song = style()
                    .align_left()
                    .font_size(16)
                    .radius(12)
                    .padlr(6)
                    .padtb(4);

                let dur = song.fill_width().fg(TEXT_MUTED).align_right();

                ui.flow_right(song.bg(ROW_SELECTED).hover(ROW_HOVER), |ui| {
                    ui.text("01", song.fg(ACCENT));
                    ui.text("Light Years", song);
                    //TODO: Times are not aligned??
                    ui.text("4:12", dur);
                });

                ui.flow_right(song.hover(ROW_HOVER), |ui| {
                    ui.text("02", song.fg(TEXT_MUTED));
                    ui.text("Four Hours", song.fg(TEXT_SECONDARY));
                    ui.text("3:38", dur);
                });
            });
        });
    });
}

struct Controls {
    bounds: Rect,
    playing: bool,
    shuffle: bool,
    repeat: bool,
    muted: bool,
    elapsed: f32,
    duration: f32,
    volume: f32,
}

fn time(t: f32) -> String {
    let total_seconds = t.max(0.0) as u32;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}", minutes, seconds)
}

fn draw_controls(controls: &mut Controls, ui: &mut FrameContext<'_, '_>) {
    ui.paint_rect(
        controls.bounds,
        bg(SIDEBAR).border(BORDER_DIM).border_side(TOP),
    );

    let [info, center, extras] = ui.split_cols(controls.bounds, [0.28, 0.44, 0.28]);

    ui.flow_right(
        bounds(info)
            .padlr(16)
            .gap(12)
            .align_flow(AlignFlow::Center)
            .clip(true),
        |ui| {
            ui.rect(style().wh(48).radius(4).bg(gray()));
            ui.flow_down(style().height(40), |ui| {
                ui.text("Light Years", style().fg(TEXT));
                ui.text(
                    "Duster · Apex, Trance-Like",
                    style().font_size(14).fg(TEXT_MUTED),
                );
            });
        },
    );

    let t = style().w(36).font_size(13).fg(TEXT_MUTED);
    let btn = style()
        .wh(32)
        .pad(4)
        .radius(8)
        .hover(ROW_HOVER)
        .fg(TEXT_TERTIARY);

    ui.flow_down(
        bounds(center)
            .padtb(12)
            .gap(4)
            .align_flow(AlignFlow::Center)
            .clip(true),
        |ui| {
            ui.flow_right(
                style()
                    .w(200)
                    .clip(true)
                    .h(36)
                    .gap(10)
                    .align_flow(AlignFlow::Center),
                |ui| {
                    icon(ui, "Shuffle", btn);
                    icon(ui, "Rewind", btn);
                    icon(
                        ui,
                        if controls.playing { "Pause" } else { "Play" },
                        btn.bg(TEXT).fg(SIDEBAR).radius(16),
                    );
                    icon(ui, "Forward", btn);
                    icon(ui, "Repeat", btn);
                },
            );

            ui.flow_right(
                style()
                    .clip(true)
                    .h(20)
                    .gap(10)
                    .align_flow(AlignFlow::Center),
                |ui| {
                    ui.text(time(controls.elapsed), t.align_right());
                    let track = ui.rect(
                        style()
                            .w(Size::FillMinus(46))
                            .h(4)
                            .radius(2)
                            .bg(TRACK_EMPTY),
                    );
                    ui.text(time(controls.duration), t.align_left());

                    ui.paint_rect(
                        Rect::new(
                            track.bounds.x,
                            track.bounds.y,
                            (track.bounds.width as f32 * controls.elapsed / controls.duration)
                                as i32,
                            track.bounds.height,
                        ),
                        bg(ACCENT).radius(2),
                    );
                },
            );
        },
    );

    ui.flow_left(
        bounds(extras)
            .padlr(16)
            .gap(10)
            .clip(true)
            .align_flow(AlignFlow::Center),
        |ui| {
            ui.text(
                format!("{}", (controls.volume * 100.0).clamp(0.0, 100.0).round()),
                style().w(24).font_size(13).fg(TEXT_MUTED),
            );
            ui.rect(style().w(96).h(4).radius(2).bg(TRACK_EMPTY));
            icon(ui, "Volume", btn);
        },
    );
}

//TODO: Add tailwind style font size and padding builders.
//Allow the user to customize them.
//Currently keeping track of all the sizings is very difficult.
fn main() {
    let mut ui = ui("mu", 1200, 780);
    ui.default_font_size = 16;

    // Skip slow db loading for now.
    // let config = mu_core::config_paths();
    // let db = mu_core::vdb::Database::new(&config.database);
    // let mut artists: Vec<String> = db.btree.keys().cloned().collect();
    // artists.sort_by_key(|a| a.to_ascii_lowercase());

    let panel_left = Image::open("assets/panel-left.png").unwrap().thumbnail(18);
    let mut sidebar = Sidebar {
        bounds: Rect::default(),
        panel_left: &panel_left,
        selected_artist: "Duster",
        artists: &[],
        // artists: &artists,
        artist_scroll: Scroll::new(),
        active: true,
        selected_mode: "Library",
        current_letter: None,
    };

    let mut library = Library {
        bounds: Rect::default(),
        artist: "Duster",
    };

    let mut controls = Controls {
        bounds: Rect::default(),
        playing: false,
        shuffle: false,
        repeat: false,
        muted: false,
        elapsed: 132.0,
        duration: 252.0,
        volume: 0.32,
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
                _ => {}
            }
        }

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
            controls.bounds = con;

            match sidebar.selected_mode {
                "Library" => {
                    draw_library(&mut library, ui);
                }
                "Queue" => {}
                "Playlist" => {}
                "Settings" => {}
                _ => unreachable!(),
            }

            draw_controls(&mut controls, ui);
        });
    }
}
