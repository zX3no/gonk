use crate::RepeatMode;
use crate::theme::{colors, format_time, icons, paint_cover};
use mu_core::{Index, Song};
use neoui::*;
use onmi::{Player, State};
use std::time::Duration;

pub const PLAYER_H: i32 = 90;

pub enum Action {
    /// Jump to where the current track is playing from (queue or library).
    GoToNowPlaying,
    TogglePlay,
    Prev,
    Next,
    ToggleShuffle,
    CycleRepeat,
}

pub fn draw(
    ui: &mut FrameContext<'_, '_>,
    rect: Rect,
    player: &mut Player,
    songs: &mut Index<Song>,
    seek_drag: &mut Option<f32>,
    shuffle: &mut bool,
    repeat: &mut RepeatMode,
    mute: &mut bool,
    icon_font: usize,
) -> Option<Action> {
    ui.paint_rect(rect, style().bg(colors::PANEL));
    ui.paint_rect(
        Rect::new(rect.x, rect.y, rect.width, 1),
        style().bg(colors::LINE),
    );

    let left_w = 280;
    let right_w = 160;
    let (left, rest) = ui.split_rect_h(rect, left_w);
    let (center, right) = ui.split_rect_h(rest, Size::FillMinus(right_w));

    let title = songs
        .selected()
        .map(|s| s.title.clone())
        .unwrap_or_else(|| "Not playing".into());
    let artist = songs
        .selected()
        .map(|s| s.artist.clone())
        .unwrap_or_default();
    let has_track = songs.selected().is_some();
    let mut action = None;

    // Now playing
    {
        let pad = 12;
        let art = 52;
        let art_rect = Rect::new(left.x + pad, left.y + (left.height - art) / 2, art, art);
        paint_cover(ui, art_rect, 5);

        let info_x = art_rect.right() + 12;
        let info_w = left.right() - info_x - 8;
        ui.paint_text(
            title,
            info_x,
            left.y + left.height / 2 - 16,
            info_w,
            18,
            colors::TEXT,
            0,
            13,
            Alignment::Left,
            Padding::default(),
            0,
        );
        ui.paint_text(
            artist,
            info_x,
            left.y + left.height / 2 + 2,
            info_w,
            16,
            colors::TEXT_MUTED,
            0,
            12,
            Alignment::Left,
            Padding::default(),
            0,
        );

        if ui.clicked(Rect::new(left.x, left.y, left.width, left.height)) && has_track {
            action = Some(Action::GoToNowPlaying);
        }
    }

    // Transport
    {
        let (transport_rect, seek_rect) = ui.split_rect_v(center, 44);
        let playing = player.state() == State::Playing;
        let btn = 32;
        let gap = 12;
        let total_w = btn * 5 + gap * 4;
        let start_x = transport_rect.x + (transport_rect.width - total_w) / 2;
        let y = transport_rect.y + (transport_rect.height - btn) / 2;

        let labels = [
            (icons::SHUFFLE, *shuffle),
            (icons::SKIP_PREV, false),
            (if playing { icons::PAUSE } else { icons::PLAY }, true),
            (icons::SKIP_NEXT, false),
            (
                icons::REPEAT,
                matches!(*repeat, RepeatMode::All | RepeatMode::One),
            ),
        ];

        for (i, (icon, on)) in labels.iter().enumerate() {
            let x = start_x + i as i32 * (btn + gap);
            let r = Rect::new(x, y, btn, btn);
            let is_play = i == 2;

            if is_play {
                ui.paint_rect(r, style().bg(colors::TEXT).radius(16));
                ui.paint_text(
                    *icon,
                    r.x,
                    r.y,
                    r.width,
                    r.height,
                    colors::BG,
                    icon_font,
                    16,
                    Alignment::Center,
                    Padding::default(),
                    0,
                );
            } else {
                let color = if *on {
                    colors::ACCENT_BRIGHT
                } else {
                    colors::TEXT_MUTED
                };
                if ui.hovered(r) {
                    ui.paint_rect(r, style().bg(colors::HOVER).radius(16));
                }
                ui.paint_text(
                    *icon,
                    r.x,
                    r.y,
                    r.width,
                    r.height,
                    color,
                    icon_font,
                    18,
                    Alignment::Center,
                    Padding::default(),
                    0,
                );
            }

            if ui.clicked(r) {
                match i {
                    0 => {
                        *shuffle = !*shuffle;
                        action = Some(Action::ToggleShuffle);
                    }
                    1 => action = Some(Action::Prev),
                    2 => action = Some(Action::TogglePlay),
                    3 => action = Some(Action::Next),
                    4 => {
                        *repeat = match *repeat {
                            RepeatMode::Off => RepeatMode::All,
                            RepeatMode::All => RepeatMode::One,
                            RepeatMode::One => RepeatMode::Off,
                        };
                        action = Some(Action::CycleRepeat);
                    }
                    _ => {}
                }
            }
        }

        // Seekbar
        let seek = seek_rect.inner(20, 4);
        draw_seekbar(ui, seek, player, seek_drag);
    }

    // Volume
    {
        let vol_rect = Rect::new(
            right.x + 16,
            right.y + (right.height - 24) / 2,
            right.width - 32,
            24,
        );
        if let Some(v) = draw_volume(ui, vol_rect, player, *mute, icon_font) {
            *mute = false;
            player.set_volume(v);
        }
    }

    action
}

fn draw_seekbar(
    ui: &mut FrameContext<'_, '_>,
    rect: Rect,
    player: &Player,
    seek_drag: &mut Option<f32>,
) {
    // Window close / tiny layouts can produce empty rects — skip safely.
    let time_w = 40;
    let head_d = 12;
    if rect.width < time_w * 2 + head_d || rect.height <= 0 {
        *seek_drag = None;
        return;
    }

    let duration = player.duration().as_secs_f32();
    let elapsed = player.elapsed().as_secs_f32();
    let live = if duration > 0.0 && duration.is_finite() {
        (elapsed / duration).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let ratio = seek_drag.unwrap_or(live).clamp(0.0, 1.0);
    let display_elapsed = if duration > 0.0 && duration.is_finite() {
        ratio * duration
    } else {
        0.0
    };

    let (left, rest) = ui.split_rect_h(rect, time_w);
    let (track_area, right) = ui.split_rect_h(rest, Size::FillMinus(time_w));
    if track_area.width <= 0 || track_area.height <= 0 {
        *seek_drag = None;
        return;
    }

    ui.paint_text(
        format_time(display_elapsed),
        left.x,
        left.y,
        left.width,
        left.height,
        colors::TEXT_DIM,
        0,
        11,
        Alignment::Center,
        Padding::default(),
        0,
    );
    ui.paint_text(
        format_time(if duration.is_finite() { duration } else { 0.0 }),
        right.x,
        right.y,
        right.width,
        right.height,
        colors::TEXT_DIM,
        0,
        11,
        Alignment::Center,
        Padding::default(),
        0,
    );

    // Basic track line + rounded scrub head.
    let track_h = 4;
    let track = Rect::new(
        track_area.x,
        track_area.y + (track_area.height - track_h) / 2,
        track_area.width,
        track_h,
    );
    ui.paint_rect(track, style().bg(colors::LINE).radius(2));

    // Keep the head fully within the track (avoids clamp min>max on bad sizes).
    let head_travel = (track.width - head_d).max(0);
    let head_x = track.x + ((head_travel as f32) * ratio).round() as i32;
    let fill_w = (head_x + head_d / 2 - track.x).clamp(0, track.width);
    if fill_w > 0 {
        ui.paint_rect(
            Rect::new(track.x, track.y, fill_w, track.height),
            style().bg(colors::ACCENT_BRIGHT).radius(2),
        );
    }
    let head = Rect::new(
        head_x,
        track.y + track.height / 2 - head_d / 2,
        head_d,
        head_d,
    );
    ui.paint_rect(head, style().bg(colors::TEXT).radius((head_d / 2) as usize));

    // Slightly taller hit target than the thin track.
    let hit = Rect::new(track.x, track_area.y, track.width, track_area.height);
    if ui.dragged(hit) {
        if let Some(pct) = ui.drag_percentage_x(hit) {
            *seek_drag = Some(pct.clamp(0.0, 1.0));
        }
    }
    if ui.released(hit) {
        if let Some(pct) = seek_drag.take().or_else(|| {
            if let Some((mx, _)) = ui.window.mouse_pos() {
                if hit.width > 0 {
                    let x = (mx as i32).saturating_sub(hit.x);
                    Some((x as f32 / hit.width as f32).clamp(0.0, 1.0))
                } else {
                    None
                }
            } else {
                None
            }
        }) {
            if duration > 0.0 && duration.is_finite() {
                player.seek_to(Duration::from_secs_f32(pct * duration));
            }
        }
    } else if ui.window.mouse_released(Mouse::Left) && !ui.dragged(hit) {
        *seek_drag = None;
    }

    if ui.clicked(hit) && seek_drag.is_none() {
        if let Some((mx, _)) = ui.window.mouse_pos() {
            if hit.width > 0 {
                let x = (mx as i32).saturating_sub(hit.x);
                let pct = (x as f32 / hit.width as f32).clamp(0.0, 1.0);
                if duration > 0.0 && duration.is_finite() {
                    player.seek_to(Duration::from_secs_f32(pct * duration));
                }
            }
        }
    }
}

fn draw_volume(
    ui: &mut FrameContext<'_, '_>,
    rect: Rect,
    player: &Player,
    mute: bool,
    icon_font: usize,
) -> Option<u8> {
    let icon_w = 28;
    let (icon_rect, track_area) = ui.split_rect_h(rect, icon_w);
    ui.paint_text(
        icons::VOLUME,
        icon_rect.x,
        icon_rect.y,
        icon_rect.width,
        icon_rect.height,
        colors::TEXT_MUTED,
        icon_font,
        16,
        Alignment::Center,
        Padding::default(),
        0,
    );

    let track = Rect::new(
        track_area.x,
        track_area.y + track_area.height / 2 - 2,
        track_area.width,
        4,
    );
    ui.paint_rect(track, style().bg(colors::LINE).radius(2));

    let vol = if mute {
        0.0
    } else {
        player.volume() as f32 / 100.0
    };
    let fill_w = ((track.width as f32) * vol.clamp(0.0, 1.0)).round() as i32;
    if fill_w > 0 {
        ui.paint_rect(
            Rect::new(track.x, track.y, fill_w, track.height),
            style().bg(colors::ACCENT_BRIGHT).radius(2),
        );
    }

    let hit = Rect::new(track.x, rect.y, track.width, rect.height);
    if let Some(pct) = ui.drag_percentage_x(hit) {
        return Some((pct * 100.0).round().clamp(0.0, 100.0) as u8);
    }
    None
}
