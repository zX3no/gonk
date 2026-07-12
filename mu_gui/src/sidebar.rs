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
    artist_scroll: &mut usize,
    queue_len: usize,
    icon_font: usize,
) -> Option<Action> {
    ui.paint_rect(rect, style().bg(colors::PANEL));
    ui.paint_rect(
        Rect::new(rect.right() - 1, rect.y, 1, rect.height),
        style().bg(colors::LINE),
    );

    let brand_h = 48;
    let nav_h = 44 * 3 + 16;
    let (brand_rect, rest) = ui.split_rect_v(rect, brand_h);
    let (nav_rect, list_rect) = ui.split_rect_v(rest, nav_h);

    ui.paint_text(
        "mu",
        brand_rect.x + 20,
        brand_rect.y,
        brand_rect.width - 20,
        brand_rect.height,
        colors::TEXT,
        0,
        18,
        Alignment::Left,
        Padding::default(),
        0,
    );

    let mut action = None;
    let mut y = nav_rect.y + 4;
    let row_h = 40;
    let pad_x = 10;

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
        (Mode::Search, icons::SEARCH, "Search", None),
        (Mode::Playlist, icons::PLAYLISTS, "Playlists", None),
    ];

    for (item, icon, label, badge) in nav_items {
        let active = matches!(
            (mode, &item),
            (Mode::Queue, Mode::Queue)
                | (Mode::Search, Mode::Search)
                | (Mode::Playlist | Mode::PlaylistDetail { .. }, Mode::Playlist)
        );

        let r = Rect::new(nav_rect.x + pad_x, y, nav_rect.width - pad_x * 2, row_h);
        let bg = if active {
            colors::ACCENT_DIM
        } else if ui.hovered(r) {
            colors::HOVER
        } else {
            colors::PANEL
        };
        ui.paint_rect(r, style().bg(bg).radius(7));

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

        ui.paint_text(
            icon,
            r.x + 10,
            r.y,
            24,
            r.height,
            icon_color,
            icon_font,
            17,
            Alignment::Center,
            Padding::default(),
            0,
        );
        ui.paint_text(
            label,
            r.x + 40,
            r.y,
            r.width - 80,
            r.height,
            text_color,
            0,
            14,
            Alignment::Left,
            Padding::default(),
            0,
        );

        if let Some(badge) = badge {
            let bw = 28;
            let bh = 20;
            let br = Rect::new(r.right() - bw - 10, r.y + (r.height - bh) / 2, bw, bh);
            ui.paint_rect(br, style().bg(colors::ACCENT_DIM).radius(10));
            ui.paint_text(
                badge,
                br.x,
                br.y,
                br.width,
                br.height,
                colors::ACCENT_BRIGHT,
                0,
                11,
                Alignment::Center,
                Padding::default(),
                0,
            );
        }

        if ui.clicked(r) {
            action = Some(Action::Mode(item));
        }
        y += row_h + 2;
    }

    ui.paint_rect(
        Rect::new(nav_rect.x + 10, y + 4, nav_rect.width - 20, 1),
        style().bg(colors::LINE),
    );

    let row_style = style()
        .padlr(12)
        .padtb(7)
        .fill_width()
        .radius(7)
        .align(Alignment::Left)
        .fg(colors::TEXT)
        .hover(colors::HOVER)
        .selected(colors::ACCENT_DIM);

    let sticky = sticky_letter(artists, *artist_scroll);
    let selected = selected_artist.map(|s| s.to_string());
    let mut artist_click = None;

    ui.scroll_view(bounds(list_rect).bg(colors::PANEL), artist_scroll, |ui| {
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
            if ui.item(format!("  {name}"), active, row_style).clicked {
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
        ui.paint_rect(pin, style().bg(colors::PANEL));
        ui.paint_text(
            letter.to_string(),
            pin.x + 12,
            pin.y,
            pin.width - 12,
            pin.height,
            colors::TEXT_MUTED,
            0,
            11,
            Alignment::Left,
            Padding::default(),
            1,
        );
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
pub fn scroll_to_index(artists: &[String], index: usize) -> usize {
    if artists.is_empty() || index >= artists.len() {
        return 0;
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
            return y.saturating_sub(ARTIST_LETTER_H);
        }
        y += ARTIST_ROW_H;
    }
    0
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
