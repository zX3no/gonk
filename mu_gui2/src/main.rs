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

fn main() {
    let mut ui = ui("mu", 1200, 780);
    ui.default_font_size = 13;

    const ARTISTS: &[&str] = &[
        "black midi",
        "Bonobo",
        "C418",
        "Clarance Clarity",
        "Covet",
        "Daft Punk",
        "Dorian Concept",
        "Duster",
    ];

    let selected_aritst = "Duster";

    while ui.window.open() {
        if ui.window.pressed(Key::Escape) {
            ui.window.close();
        }

        ui.frame(|ui| {
            let (sidebar, body) = ui.split_h(280);

            let sb = style().fg(TEXT).font_size(16);
            ui.flow_down(bounds(sidebar).bg(SIDEBAR).pad(12), |ui| {
                ui.flow_right(sb.pad(12), |ui| {
                    ui.text("mu", sb);
                    ui.text("[]", sb.fill_width().align_right())
                });

                ui.flow_down(sb.gap(2), |ui| {
                    ui.flow_right(sb, |ui| {});

                    let mut item = |t: &'static str, i: &'static str, s: bool| {
                        //TODO: Should use impl IntoColor to allow for Option or u32.
                        // sel.bg(if s { Some(ROW_SELECTED) } else { None });

                        let mut sel = sb.padlr(12).padtb(8).radius(6);
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

                ui.flow_down(style(), |ui| {
                    //Assuming artists is pre sorted alphabetically.
                    let mut first_letter = String::new();
                    let text = sb.padlr(12).padtb(8);
                    let mut selected_text =
                        text.bg(ROW_SELECTED).align_left().fill_width().radius(6);

                    for artist in ARTISTS {
                        let next = artist[..1].to_ascii_uppercase();
                        if next != first_letter {
                            first_letter = next;
                            let letter = text.font_size(12).fg(TEXT_MUTED);
                            ui.text(first_letter.clone(), letter);
                        }
                        let sel = *artist == selected_aritst;
                        ui.text(*artist, if sel { selected_text } else { text });
                    }
                });
            });

            ui.flow_down(bounds(body).bg(BODY), |ui| {})

            // ui.paint_rect(left, bg(red()));
            // ui.paint_rect(right, bg(gray()));
        });
    }
}
