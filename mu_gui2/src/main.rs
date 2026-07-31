#![allow(unused)]
use neoui::*;

const BODY: u32 = hex("#0b0b0c");
const SIDEBAR: u32 = hex("#101011");

const TEXT: u32 = hex("#ece9e4");
const TEXT_SECONDARY: u32 = hex("#ece9e4d9");
const TEXT_TERTIARY: u32 = hex("#ece9e499");
// const TEXT_MUTED: u32 = hex("#ece9e46b");
// const TEXT_FAINT: u32 = hex("#ece9e44d");
const TEXT_MUTED: u32 = hex("#555454");
// const TEXT_MUTED: u32 = hex("#4D4D4C");
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

const ARTISTS: &[&str] = &[
    "Arca",
    "BADBADNOTGOOD",
    "Björk",
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
    "Eminem",
    "Flawed Mangoes",
    "Floating Points, Pharoah Sanders & The London Symphony Orchestra",
    "Flume",
    "Flying Lotus",
    "Funeral Diner",
    "Godspeed You! Black Emperor",
    "Gospel",
    "Hans Zimmer",
    "Iglooghost",
    "J-E-T-S",
    "JPEGMAFIA",
    "Jakey",
    "John Coltrane",
    "Joji",
    "Julie",
    "Kai Whiston",
    "Kanazu Tomoyuki",
    "Kanye West",
    "Kendrick Lamar",
    "Koan Sound",
    "Komorebi",
    "LINGUA IGNOTA",
    "Lena Raine",
    "Machine Girl",
    "Machinedrum",
    "Madvillain",
    "Massive Attack",
    "Medasin",
    "Memo Boy",
    "Men I Trust",
    "Mick Gordon",
    "Miles Davis",
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
    "STEINS;GATE",
    "Sam Gellaitry",
    "Seatbelts",
    "Sinjin Hawke",
    "Sinjin Hawke & Zora Jones",
    "Slauson Malone",
    "Slint",
    "Steve Reich",
    "Sweet Trip",
    "Tera Melos",
    "The Comet Is Coming",
    "Title Fight",
    "Toby Fox",
    "Travis Scott",
    "Tyler, The Creator",
    "Various Artists",
    "Yussef Dayes",
    "beabadoobee",
    "black midi",
    "eightiesheadachetape",
    "foxtails",
    "kinoue64",
    "mage tears",
    "mouse on the keys",
    "my bloody valentine",
    "william crooks",
    "Øneheart", //TODO: This should be converted to an O.
];

const ALPHABET: &[&str] = &[
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z",
];

struct Sidebar<'a> {
    panel_left: &'a Image,
    selected_artist: &'static str,
    bounds: Rect,
    active: bool,
    artist_scroll: Scroll,
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
                ui.place_left(style().fill_width().bleed().padr(15), |ui| {
                    if ui
                        .image(
                            sidebar.panel_left,
                            style().mar(5).radius(6).hover(ROW_HOVER),
                        )
                        .clicked
                    {
                        sidebar.active = false;
                    }
                });
            });

            ui.flow_down(sb.gap(2).padlr(8), |ui| {
                let mut item = |t: &'static str, i: &'static str, s: bool| {
                    //TODO: Should use impl IntoColor to allow for Option or u32.
                    // sel.bg(if s { Some(ROW_SELECTED) } else { None });

                    let mut sel = sb.padlr(12).padtb(8).radius(6).hover(ROW_HOVER);
                    sel.bg = if s { Some(ROW_SELECTED) } else { None };
                    let text = sb.fg(if s { TEXT } else { TEXT_TERTIARY });
                    let ntext = sb
                        .fg(if s { TEXT_MUTED } else { TEXT_FAINT })
                        .fill_width()
                        .align_right();

                    ui.flow_right(sel, |ui| {
                        ui.text(t, text);
                        ui.text(i, ntext);
                    })
                };

                item("Library", "1", true);
                item("Queue", "2", false);
                item("Playlist", "3", false);
                item("Settings", "4", false);
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
                let row = ui.measure_text("A", 0, 10, None).height;
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

fn main() {
    let mut ui = ui("mu", 1200, 780);
    ui.default_font_size = 13;

    let panel_left = Image::open("assets/panel-left.png").unwrap().thumbnail(18);
    let mut sidebar = Sidebar {
        panel_left: &panel_left,
        selected_artist: "Duster",
        bounds: Rect::default(),
        artist_scroll: Scroll::new(),
        active: true,
    };

    while ui.window.open() {
        if ui.window.pressed(Key::Escape) {
            ui.window.close();
        }

        ui.frame(|ui| {
            let (sb, body) = ui.split_h(280);
            sidebar.bounds = sb;

            if sidebar.active {
                draw_sidebar(&mut sidebar, ui);
            }

            // ui.flow_down(bounds(body).bg(BODY), |ui| {})

            // ui.paint_rect(left, bg(red()));
            // ui.paint_rect(right, bg(gray()));
        });
    }
}
