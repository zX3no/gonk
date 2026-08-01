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

struct Sidebar<'a> {
    bounds: Rect,
    panel_left: &'a Image,
    artists: &'a [String],
    selected_artist: &'static str,
    selected_mode: &'static str,
    active: bool,
    artist_scroll: Scroll,
}

//TODO: Remove.
fn icon(ui: &mut FrameContext, kind: &str, r: Rect, color: u32) {
    let dim = with_alpha(color, 110);
    let u = |v: i32| v * r.width / 24;
    let mut bar = |x: i32, y: i32, w: i32, h: i32, c: u32| {
        let rect = Rect::new(r.x + u(x), r.y + u(y), u(w).max(1), u(h).max(1));
        ui.paint_rect(rect, bg(c).radius(1));
    };

    match kind {
        "Panel" => {
            bar(2, 4, 20, 16, color);
            bar(4, 6, 16, 12, SIDEBAR);
            bar(4, 6, 5, 12, color);
        }
        "Library" => {
            bar(3, 4, 4, 17, color);
            bar(9, 4, 4, 17, color);
            bar(15, 6, 4, 15, color);
            bar(19, 7, 3, 14, dim);
        }
        "Queue" => {
            bar(3, 6, 12, 2, color);
            bar(3, 11, 12, 2, color);
            bar(3, 16, 8, 2, dim);
            let (x, y) = (r.x + u(15), r.y + u(11));
            ui.paint_triangle((x, y), (x, y + u(9)), (x + u(7), y + u(4)), bg(color));
        }
        "Playlist" => {
            bar(3, 6, 15, 2, color);
            bar(3, 11, 15, 2, color);
            bar(3, 16, 9, 2, dim);
            bar(15, 16, 7, 2, color);
            bar(17, 14, 2, 7, color);
        }
        "Settings" => {
            bar(5, 3, 2, 18, dim);
            bar(12, 3, 2, 18, dim);
            bar(19, 3, 2, 18, dim);
            bar(3, 7, 6, 3, color);
            bar(10, 13, 6, 3, color);
            bar(17, 6, 6, 3, color);
        }
        _ => {}
    }
}

//TODO: Remove
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
            let mut btn = style().wh(34).radius(8).hover(ROW_HOVER);
            let toggle = ui.rect(btn);
            let panel = Rect::new(toggle.bounds.x + 7, toggle.bounds.y + 7, 20, 20);
            icon(ui, "Panel", panel, TEXT_TERTIARY);
            if toggle.clicked {
                sidebar.active = true;
            }

            ui.gap(6);

            for mode in ["Library", "Queue", "Playlist", "Settings"] {
                let selected = mode == sidebar.selected_mode;
                btn.bg = if selected { Some(ROW_SELECTED) } else { None };
                let state = ui.rect(btn);
                let color = if selected { TEXT } else { TEXT_TERTIARY };
                let b = Rect::new(state.bounds.x + 7, state.bounds.y + 7, 20, 20);
                icon(ui, mode, b, color);
                if state.clicked {
                    sidebar.selected_mode = mode;
                }
            }

            ui.gap(10);
            ui.rect(style().height(1).width(Size::Fill).bg(BORDER_DIM));
        },
    );
}

fn draw_sidebar<'a>(sidebar: &mut Sidebar<'a>, ui: &mut FrameContext<'_, 'a>) {
    let sb = style().fg(TEXT).font_size(16);
    ui.flow_down(
        bounds(sidebar.bounds)
            .bg(SIDEBAR)
            .border(BORDER_DIM)
            .border_side(RIGHT),
        |ui| {
            ui.flow_right(sb.pad(20), |ui| {
                ui.text("mu", sb);

                //TODO: Remove
                let frame = ui.current_frame_bounds();
                let size = ui.resolve_size(Size::FillMinus(-15), Flow::Right);
                let bounds = Rect::new(frame.x + size, frame.y, 20, 20);
                icon(ui, "Panel", bounds, TEXT_TERTIARY);
                if ui.clicked(bounds) {
                    sidebar.active = false;
                }
                // ui.place_left(style().fill_width().bleed().padr(15), |ui| {
                //     if ui
                //         .image(
                //             sidebar.panel_left,
                //             style().mar(5).radius(6).hover(ROW_HOVER),
                //         )
                //         .clicked
                //     {
                //         sidebar.active = false;
                //     }
                // });
            });

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
            ui.scroll(
                bounds(artist).padlr(8).elastic(true),
                &mut sidebar.artist_scroll,
                |ui| {
                    //Assuming artists is pre sorted alphabetically.
                    let mut first_letter = ' ';
                    let text = sb
                        .padlr(12)
                        .padtb(8)
                        .radius(6)
                        .align_left()
                        .fill_width()
                        .hover(ROW_HOVER);
                    let mut selected_text = text.bg(ROW_SELECTED);

                    for artist in ARTISTS {
                        let next = artist.chars().next().unwrap().to_ascii_uppercase();
                        if next != first_letter {
                            first_letter = next;
                            let letter = sb.padlr(12).padtb(8).font_size(12).fg(TEXT_MUTED);
                            ui.text(first_letter.to_string(), letter);
                        }
                        let sel = *artist == selected_artist;
                        ui.text(*artist, if sel { selected_text } else { text });
                    }
                },
            );

            let selected_letter = sidebar
                .selected_artist
                .chars()
                .next()
                .unwrap()
                .to_ascii_uppercase();

            ui.paint_rect(alphabet, style().border(BORDER_DIM).border_side(LEFT));
            let strip = alphabet;
            alphabet.x += 12;
            ui.flow_down(bounds(alphabet), |ui| {
                let letter = sb.font_size(10);
                let row = ui.measure_text("A", Font::default(), 10, None).height;
                ui.gap((ui.current_frame_bounds().height - row * 26) / 2);

                // let index = (selected_letter as i32 - 'A' as i32).clamp(0, 25);
                // let top = ui.current_frame_bounds().y + index * row;
                // let glow = Rect::new(strip.x + 1, top - row * 2, strip.width - 1, row * 5);
                // let (edge, core) = (with_alpha(TEXT, 1), with_alpha(TEXT, 8));
                // ui.paint_rect(
                //     Rect::new(glow.x, glow.y, glow.width, glow.height / 2),
                //     bg(edge).gradient(edge, core),
                // );
                // ui.paint_rect(
                //     Rect::new(
                //         glow.x,
                //         glow.y + glow.height / 2,
                //         glow.width,
                //         glow.height / 2,
                //     ),
                //     bg(core).gradient(core, edge),
                // );

                for &letter in ALPHABET {
                    let ch = letter.chars().next().unwrap();
                    let dist = (ch as i32 - selected_letter as i32).abs();
                    let color = match dist {
                        0 => ACCENT,
                        1 => TEXT,
                        _ => TEXT_FAINT,
                    };
                    ui.text(letter, sb.font_size(10).fg(color));
                }
                ui.current_frame_bounds().height
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
}

fn draw_controls(controls: &mut Controls, ui: &mut FrameContext<'_, '_>) {
    ui.flow_right(
        bounds(controls.bounds)
            .pad(16)
            .bg(SIDEBAR)
            .border(BORDER_DIM)
            .border_side(TOP),
        |ui| {
            //Current Song
            ui.flow_right(style(), |ui| {
                ui.rect(style().wh(34).bg(gray()));
                ui.gap(12);

                ui.flow_down(style(), |ui| {
                    ui.text("Light Years", style().fg(TEXT));
                    ui.text("Duster · Apex, Trance-Like", style().font_size(14).fg(TEXT_MUTED));
                });
            });

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
    };

    let mut library = Library {
        bounds: Rect::default(),
        artist: "Duster",
    };

    let mut controls = Controls {
        bounds: Rect::default(),
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

            let (body, con) = ui.split_rect_v(body, -64);
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
