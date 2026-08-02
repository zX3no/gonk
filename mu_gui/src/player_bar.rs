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
    icon_font: Font,
) -> Option<Action> {
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

    ui.paint_rect(rect, style().bg(colors::PANEL));
    ui.paint_rect(
        Rect::new(rect.x, rect.y, rect.width, 1),
        style().bg(colors::LINE),
    );

    // Now playing
    {
        let pad = 12;
        let art = 52;
        let art_rect = Rect::new(left.x + pad, left.y + (left.height - art) / 2, art, art);
        paint_cover(ui, art_rect, 5);

        let info_x = art_rect.right() + 12;
        let info_w = left.right() - info_x - 8;
        let info = Rect::new(info_x, left.y + left.height / 2 - 16, info_w, 36);
        ui.place_down(bounds(info), |ui| {
            ui.text(
                title,
                style()
                    .fg(colors::TEXT)
                    .font_size(13)
                    .fill_width()
                    .height(18)
                    .align(Alignment::Left),
            );
            ui.text(
                artist,
                style()
                    .fg(colors::TEXT_MUTED)
                    .font_size(12)
                    .fill_width()
                    .height(16)
                    .align(Alignment::Left),
            );
        });

        if ui.clicked(left) && has_track {
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
        let transport = Rect::new(start_x, y, total_w, btn);

        let labels = [
            (icons::SHUFFLE, *shuffle, false),
            (icons::SKIP_PREV, false, false),
            (if playing { icons::PAUSE } else { icons::PLAY }, true, true),
            (icons::SKIP_NEXT, false, false),
            (
                icons::REPEAT,
                matches!(*repeat, RepeatMode::All | RepeatMode::One),
                false,
            ),
        ];

        ui.place_right(bounds(transport).align_flow(AlignFlow::Center), |ui| {
            for (i, (icon, on, is_play)) in labels.iter().enumerate() {
                let mut s = style()
                    .font(icon_font)
                    .font_size(if *is_play { 16 } else { 18 })
                    .width(btn)
                    .height(btn)
                    .radius(16)
                    .hover(colors::HOVER);
                s = if *is_play {
                    s.fg(colors::BG).bg(colors::TEXT).hover(colors::TEXT)
                } else if *on {
                    s.fg(colors::ACCENT_BRIGHT)
                } else {
                    s.fg(colors::TEXT_MUTED)
                };
                if ui.text(*icon, s).clicked {
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
                if i + 1 < labels.len() {
                    ui.gap(gap);
                }
            }
        });

        draw_seekbar(ui, seek_rect.inner(20, 4), player, seek_drag);
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

    let time = style()
        .fg(colors::TEXT_DIM)
        .font_size(11)
        .fill_width()
        .fill_height();
    ui.place_down(bounds(left), |ui| {
        ui.text(format_time(display_elapsed), time);
    });
    ui.place_down(bounds(right), |ui| {
        ui.text(
            format_time(if duration.is_finite() { duration } else { 0.0 }),
            time,
        );
    });

    let track_h = 4;
    let track = Rect::new(
        track_area.x,
        track_area.y + (track_area.height - track_h) / 2,
        track_area.width,
        track_h,
    );
    ui.paint_rect(track, style().bg(colors::LINE).radius(2));

    let head_travel = (track.width - head_d).max(0);
    let head_x = track.x + ((head_travel as f32) * ratio).round() as i32;
    let fill_w = (head_x + head_d / 2 - track.x).clamp(0, track.width);
    if fill_w > 0 {
        ui.paint_rect(
            Rect::new(track.x, track.y, fill_w, track.height),
            style().bg(colors::ACCENT_BRIGHT).radius(2),
        );
    }
    ui.paint_rect(
        Rect::new(
            head_x,
            track.y + track.height / 2 - head_d / 2,
            head_d,
            head_d,
        ),
        style().bg(colors::TEXT).radius((head_d / 2) as usize),
    );

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
    icon_font: Font,
) -> Option<u8> {
    let icon_w = 28;
    let (icon_rect, track_area) = ui.split_rect_h(rect, icon_w);

    ui.place_down(bounds(icon_rect), |ui| {
        ui.text(
            icons::VOLUME,
            style()
                .font(icon_font)
                .font_size(16)
                .fg(colors::TEXT_MUTED)
                .fill_width()
                .fill_height(),
        );
    });

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
