#![allow(unused)]
use neoui::*;

const BODY: u32 = hex("#0b0b0c");
const SIDEBAR: u32 = hex("#101011");

const TEXT: u32 = hex("#ece9e4");
const TEXT_SECONDARY: u32 = hex("#ece9e4d9");
const TEXT_TERTIARY: u32 = hex("#ece9e499");
const TEXT_MUTED: u32 = hex("#ece9e46b");
const TEXT_FAINT: u32 = hex("#ece9e44d");

const BORDER: u32 = hex("#ece9e41a");
const BORDER_SUBTLE: u32 = hex("#ece9e412");
const TRACK_EMPTY: u32 = hex("#ece9e41f");
const TRACK_FILL: u32 = hex("#ece9e4a6");
const KNOB: u32 = hex("#ece9e4");

const ROW_HOVER: u32 = hex("#ece9e409");
const ROW_PLAYING: u32 = hex("#ece9e40a");
const ROW_SELECTED: u32 = hex("#ece9e412");
const ROW_ACTIVE: u32 = hex("#ece9e41a");

const ACCENT: u32 = hex("#9b84d9");
const ACCENT_HOVER: u32 = hex("#ad98e2");
const ACCENT_PRESSED: u32 = hex("#8871c6");
const ACCENT_SOFT: u32 = hex("#9b84d938");

fn main() {
    let mut ui = ui("mu", 1200, 780);
    ui.default_font_size = 13;

    while ui.window.open() {
        if ui.window.pressed(Key::Escape) {
            ui.window.close();
        }

        ui.frame(|ui| {
            let (sidebar, body) = ui.split_h(280);

            let sb = style().font_size(16);
            ui.flow_down(bounds(sidebar).bg(SIDEBAR).pad(12), |ui| {
                ui.flow_right(sb, |ui| {
                    ui.text("mu", sb);
                    ui.text("[]", sb.fill_width().align_right())
                });

                ui.flow_down(sb.gap(12), |ui| {
                    ui.flow_right(sb, |ui| {});

                    let mut item = |t: &'static str, i: &'static str| {
                        ui.flow_right(sb, |ui| {
                            ui.text(t, sb);
                            ui.text(i, sb.fill_width().align(Alignment::Right));
                        })
                    };

                    item("Library", "1");
                    item("Queue", "2");
                    item("Playlist", "3");
                    item("Settings", "4");
                });
            });

            ui.flow_down(bounds(body).bg(BODY), |ui| {})

            // ui.paint_rect(left, bg(red()));
            // ui.paint_rect(right, bg(gray()));
        });
    }
}
