use crate::Mode;
use crate::theme::{colors, icons};
use neoui::*;

pub const SIDEBAR_W: i32 = 232;

pub enum Action {
    Mode(Mode),
    Artist(String),
}

pub fn draw(
    ui: &mut FrameContext<'_, '_>,
    rect: Rect,
    mode: &Mode,
    artists: &[String],
    selected_artist: Option<&str>,
    artist_scroll: &mut Scroll,
    queue_len: usize,
    icon_font: usize,
) -> Option<Action> {
    let brand_h = 48;
    let nav_h = 44 * 3 + 16;
    let (brand_rect, rest) = ui.split_rect_v(rect, brand_h);
    let (nav_rect, list_rect) = ui.split_rect_v(rest, nav_h);

    ui.paint_rect(rect, style().bg(colors::PANEL));
    ui.paint_rect(
        Rect::new(rect.right() - 1, rect.y, 1, rect.height),
        style().bg(colors::LINE),
    );

    ui.place_down(bounds(brand_rect), |ui| {
        ui.text(
            "mu",
            style()
                .fg(colors::TEXT)
                .font_size(18)
                .padl(20)
                .fill_width()
                .fill_height()
                .align(Alignment::Left),
        );
    });

    let mut action = None;
    let nav_items: [(Mode, &str, &str, Option<String>); 3] = [
        (
            Mode::Queue,
            icons::QUEUE,
            "Queue",
            if queue_len > 0 {
                Some(queue_len.to_string())
            } else {
                None
            },
        ),
        (Mode::Playlist, icons::PLAYLISTS, "Playlists", None),
        (Mode::Settings, icons::SETTINGS, "Settings", None),
    ];

    ui.place_down(bounds(nav_rect).padlr(10).padt(4), |ui| {
        for (item, icon, label, badge) in nav_items {
            let active = matches!(
                (mode, &item),
                (Mode::Queue, Mode::Queue)
                    | (Mode::Playlist | Mode::PlaylistDetail { .. }, Mode::Playlist)
                    | (Mode::Settings, Mode::Settings)
            );
            let icon_color = if active {
                colors::ACCENT_BRIGHT
            } else {
                colors::TEXT_MUTED
            };
            let text_color = if active {
                colors::TEXT
            } else {
                colors::TEXT_MUTED
            };

            let row = ui.rect(
                style()
                    .fill_width()
                    .height(40)
                    .radius(7)
                    .bg(colors::PANEL)
                    .hover(colors::HOVER)
                    .selected(colors::ACCENT_DIM)
                    .is_selected(active),
            );

            ui.place_right(
                bounds(row.bounds)
                    .padl(10)
                    .padr(10)
                    .align_flow(AlignFlow::Center),
                |ui| {
                    ui.text(
                        icon,
                        style()
                            .font(icon_font)
                            .font_size(17)
                            .fg(icon_color)
                            .width(30)
                            .height(40),
                    );
                    let label_w = if badge.is_some() {
                        Size::FillMinus(38)
                    } else {
                        Size::Fill
                    };
                    ui.text(
                        label,
                        style()
                            .fg(text_color)
                            .font_size(14)
                            .width(label_w)
                            .height(40)
                            .align(Alignment::Left),
                    );
                    if let Some(badge) = badge {
                        ui.text(
                            badge,
                            style()
                                .fg(colors::ACCENT_BRIGHT)
                                .bg(colors::ACCENT_DIM)
                                .font_size(11)
                                .width(28)
                                .height(20)
                                .radius(10),
                        );
                    }
                },
            );

            if row.clicked {
                action = Some(Action::Mode(item));
            }
            ui.gap(2);
        }

        ui.gap(8);
        ui.rect(style().fill_width().height(1).bg(colors::LINE));
    });

    let row_style = style()
        .padlr(12)
        .padtb(7)
        .fill_width()
        .radius(7)
        .align(Alignment::Left)
        .fg(colors::TEXT)
        .hover(colors::HOVER)
        .selected(colors::ACCENT_DIM);

    let sticky = sticky_letter(artists, artist_scroll.offset as usize);
    let selected = selected_artist.map(|s| s.to_string());
    let mut artist_click = None;

    ui.scroll(bounds(list_rect).bg(colors::PANEL), artist_scroll, |ui| {
        let mut last_letter: Option<char> = None;
        for name in artists {
            let letter = name
                .chars()
                .next()
                .map(|c| c.to_ascii_uppercase())
                .unwrap_or('#');
            if last_letter != Some(letter) {
                last_letter = Some(letter);
                ui.text(
                    letter.to_string(),
                    style()
                        .fg(colors::TEXT_MUTED)
                        .font_size(11)
                        .padl(12)
                        .padt(6)
                        .padb(2)
                        .fill_width()
                        .align(Alignment::Left)
                        .bg(colors::PANEL),
                );
            }
            let active = selected.as_deref() == Some(name.as_str());
            if ui
                .item(format!("  {name}"), row_style.is_selected(active))
                .clicked
            {
                artist_click = Some(name.clone());
            }
        }
    });

    if let Some(letter) = sticky {
        let pin = Rect::new(
            list_rect.x,
            list_rect.y,
            list_rect.width,
            ARTIST_LETTER_H as i32,
        );
        ui.place_down(bounds(pin).bg(colors::PANEL).padl(12), |ui| {
            ui.text(
                letter.to_string(),
                style()
                    .fg(colors::TEXT_MUTED)
                    .font_size(11)
                    .fill_width()
                    .fill_height()
                    .align(Alignment::Left),
            );
        });
    }

    if action.is_none() {
        if let Some(name) = artist_click {
            action = Some(Action::Artist(name));
        }
    }
    action
}

// Heights must match the styles used in `draw` with the default UI font (Aptos)
// and `ui.default_font_size = 13`:
//   letter header: font 11, padt 6, padb 2 → new_line≈13 + 8 = 21
//   artist row:    font 13, padtb 7        → new_line≈16 + 14 = 30
const ARTIST_ROW_H: usize = 30;
const ARTIST_LETTER_H: usize = 21;

fn sticky_letter(artists: &[String], scroll_y: usize) -> Option<char> {
    if artists.is_empty() {
        return None;
    }
    let mut y = 0usize;
    let mut last = artists[0]
        .chars()
        .next()
        .map(|c| c.to_ascii_uppercase())
        .unwrap_or('#');
    let mut sticky = last;
    for name in artists {
        let letter = name
            .chars()
            .next()
            .map(|c| c.to_ascii_uppercase())
            .unwrap_or('#');
        if letter != last {
            y += ARTIST_LETTER_H;
            last = letter;
        }
        if y + ARTIST_ROW_H > scroll_y {
            sticky = letter;
            break;
        }
        y += ARTIST_ROW_H;
        sticky = letter;
    }
    Some(sticky)
}

/// Pixel scroll offset so `artists[index]` sits just below the sticky letter pin.
pub fn scroll_to_index(artists: &[String], index: usize) -> f32 {
    if artists.is_empty() || index >= artists.len() {
        return 0.0;
    }
    let mut y = 0usize;
    let mut last_letter: Option<char> = None;
    for (i, name) in artists.iter().enumerate() {
        let letter = name
            .chars()
            .next()
            .map(|c| c.to_ascii_uppercase())
            .unwrap_or('#');
        if last_letter != Some(letter) {
            y += ARTIST_LETTER_H;
            last_letter = Some(letter);
        }
        if i == index {
            // Leave room for the sticky letter overlay so the name stays visible.
            return y.saturating_sub(ARTIST_LETTER_H) as f32;
        }
        y += ARTIST_ROW_H;
    }
    0.0
}

/// First artist whose name starts with `prefix` (case-insensitive).
pub fn find_prefix(artists: &[String], prefix: &str) -> Option<usize> {
    if prefix.is_empty() {
        return None;
    }
    let prefix_lower = prefix.to_lowercase();
    artists
        .iter()
        .position(|name| name.to_lowercase().starts_with(&prefix_lower))
}
