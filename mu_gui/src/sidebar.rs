use crate::theme::{colors, icons};
use crate::Mode;
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

    // Brand text only — no icon mark.
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
    let nav_items = [
        (Mode::Home, icons::HOME, "Home"),
        (Mode::Search, icons::SEARCH, "Search"),
        (Mode::Playlist, icons::PLAYLISTS, "Playlists"),
    ];

    for (item, icon, label) in nav_items {
        let active = matches!(
            (mode, &item),
            (Mode::Home, Mode::Home)
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
            r.width - 48,
            r.height,
            text_color,
            0,
            14,
            Alignment::Left,
            Padding::default(),
            0,
        );

        if ui.clicked(r) {
            action = Some(Action::Mode(item));
        }
        y += row_h + 2;
    }

    ui.paint_rect(
        Rect::new(nav_rect.x + 10, y + 4, nav_rect.width - 20, 1),
        style().bg(colors::LINE),
    );

    // Artist list — no "ARTISTS" heading.
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
        let pin = Rect::new(list_rect.x, list_rect.y, list_rect.width, 22);
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

fn sticky_letter(artists: &[String], scroll_y: usize) -> Option<char> {
    if artists.is_empty() {
        return None;
    }
    const ROW_H: usize = 30;
    const LETTER_H: usize = 22;
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
            y += LETTER_H;
            last = letter;
        }
        if y + ROW_H > scroll_y {
            sticky = letter;
            break;
        }
        y += ROW_H;
        sticky = letter;
    }
    Some(sticky)
}
