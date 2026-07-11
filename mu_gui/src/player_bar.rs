use crate::theme::{colors, format_time, icons, paint_cover};
use crate::RepeatMode;
use mu_core::{Index, Song};
use neoui::*;
use onmi::{Player, State};
use std::time::Duration;

pub const PLAYER_H: i32 = 90;

pub enum Action {
    OpenQueue,
    TogglePlay,
    Prev,
    Next,
    ToggleShuffle,
    CycleRepeat,
}

const BAR_COUNT: usize = 48;

fn bar_height(i: usize, max_h: i32) -> i32 {
    let mut seed = (i as u32).wrapping_mul(16807).wrapping_add(7);
    seed = seed.wrapping_mul(16807) % 2147483647;
    let r = seed as f32 / 2147483647.0;
    (6.0 + r * (max_h as f32 - 6.0).max(1.0)) as i32
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
            action = Some(Action::OpenQueue);
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
            (
                if playing { icons::PAUSE } else { icons::PLAY },
                true,
            ),
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
    let duration = player.duration().as_secs_f32();
    let elapsed = player.elapsed().as_secs_f32();
    let live = if duration > 0.0 && duration.is_finite() {
        (elapsed / duration).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let ratio = seek_drag.unwrap_or(live);
    let display_elapsed = if duration > 0.0 && duration.is_finite() {
        ratio * duration
    } else {
        0.0
    };

    let time_w = 40;
    let (left, rest) = ui.split_rect_h(rect, time_w);
    let (wave, right) = ui.split_rect_h(rest, Size::FillMinus(time_w));

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

    let bar_gap = 2;
    let usable = wave.width.saturating_sub(bar_gap * (BAR_COUNT as i32 - 1));
    let bar_w = (usable / BAR_COUNT as i32).max(2);
    let max_h = (wave.height - 4).max(8);
    let mut x = wave.x;
    for i in 0..BAR_COUNT {
        let h = bar_height(i, max_h);
        let y = wave.y + (wave.height - h) / 2;
        let played = (i as f32 / BAR_COUNT as f32) < ratio;
        ui.paint_rect(
            Rect::new(x, y, bar_w, h),
            style()
                .bg(if played {
                    colors::ACCENT_BRIGHT
                } else {
                    colors::LINE
                })
                .radius(1),
        );
        x += bar_w + bar_gap;
    }

    if ui.dragged(wave) {
        if let Some(pct) = ui.drag_percentage_x(wave) {
            *seek_drag = Some(pct);
        }
    }
    if ui.released(wave) {
        if let Some(pct) = seek_drag.take().or_else(|| {
            if let Some((mx, _)) = ui.window.mouse_pos() {
                let x = (mx as i32).saturating_sub(wave.x);
                Some((x as f32 / wave.width as f32).clamp(0.0, 1.0))
            } else {
                None
            }
        }) {
            if duration > 0.0 && duration.is_finite() {
                player.seek_to(Duration::from_secs_f32(pct * duration));
            }
        }
    } else if ui.window.mouse_released(Mouse::Left) && !ui.dragged(wave) {
        *seek_drag = None;
    }

    if ui.clicked(wave) && seek_drag.is_none() {
        if let Some((mx, _)) = ui.window.mouse_pos() {
            let x = (mx as i32).saturating_sub(wave.x);
            let pct = (x as f32 / wave.width as f32).clamp(0.0, 1.0);
            if duration > 0.0 && duration.is_finite() {
                player.seek_to(Duration::from_secs_f32(pct * duration));
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
