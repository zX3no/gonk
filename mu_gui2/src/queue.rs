use crate::*;
use mu_core::{Database, SongId};

pub const ROW_HEIGHT: i32 = 60;
pub const ROW_GAP: i32 = 4;
pub const ROW_STEP: i32 = ROW_HEIGHT + ROW_GAP;
/// How long a row takes to slide into a new slot, and to settle after a drop.
pub const SLIDE: f32 = 0.18;
/// Distance from the edge of the list where dragging starts scrolling.
pub const EDGE: i32 = 56;
pub const EDGE_SPEED: f32 = 900.0;

pub struct Drag {
    pub index: usize,
    /// Distance from the top of the row to the mouse when it was grabbed.
    pub grab: i32,
}

pub fn next(current: usize, queue: &mut Queue, db: &Database, player: &mut onmi::Player) {
    if queue.songs.is_empty() {
        return;
    }
    let next = (current + 1) % queue.songs.len();
    queue.playing_song = Some(next);
    if let Some(song) = db.song(queue.songs[next]) {
        player.play_song(&song.path, Some(song.gain), true);
    }
}

pub fn prev(current: usize, queue: &mut Queue, db: &Database, player: &mut onmi::Player) {
    if queue.songs.is_empty() {
        return;
    }
    let len = queue.songs.len();
    let prev = (current + len - 1) % len;
    queue.playing_song = Some(prev);
    if let Some(song) = db.song(queue.songs[prev]) {
        player.play_song(&song.path, Some(song.gain), true);
    }
}

pub struct Queue<'a> {
    /// Queuing playlists and artists have different behaviour.
    /// Since the user can play different songs inside the library album view.
    /// We don't want to update the queue everytime the user plays a new song
    /// only when the user changes the artist.
    pub playing_artist: Option<&'a str>,
    pub songs: Vec<SongId>,
    /// songs[index]
    pub playing_song: Option<usize>,
    pub bounds: Rect,
    pub scroll: Scroll,
    pub drag: Option<Drag>,
}

pub fn draw_queue<'a>(ui: &mut FrameContext<'_, 'a>, queue: &mut Queue, db: &'a Database) {
    ui.flow_down(
        flow().bounds(queue.bounds).padlr(36).padtb(12).bg(BODY),
        |ui| {
            ui.text("Queue", text().font_size(42));

            // let remaining_secs: f32 = queue
            //     .songs
            //     .iter()
            //     .skip(queue.playing_song.unwrap_or(0))
            //     .filter_map(|&id| db.song(id).map(|s| s.duration))
            //     .sum();
            // let remaining_mins = (remaining_secs / 60.0).round() as u32;
            // let subtext = ui.fmt(format_args!(
            //     "{} tracks · {} min remaining",
            //     queue.songs.len(),
            //     remaining_mins
            // ));
            let subtext = ui.fmt(format_args!("{} tracks", queue.songs.len(),));
            ui.text(subtext, text().font_size(14).fg(TEXT_MUTED));

            ui.gap(12);
            ui.rect(rect().fillw().height(1).bg(BORDER_DIM));
            ui.gap(12);

            let view = ui.current_frame_bounds();
            let mouse = ui.mouse_position().y;
            let held = ui.window.mouse_down(Mouse::Left);
            let dt = ui.dt;
            // Fractional slot the carried row sits at.
            let mut carried = 0.0;

            if let Some(d) = &mut queue.drag {
                if !held {
                    queue.drag = None;
                } else {
                    let low = view.y as f32;
                    let high = (view.bottom() - ROW_HEIGHT).max(view.y) as f32;
                    let y = ((mouse - d.grab) as f32).clamp(low, high);

                    if mouse >= view.y && mouse <= view.bottom() {
                        if mouse < view.y + EDGE {
                            let over = (view.y + EDGE - mouse) as f32 / EDGE as f32;
                            queue.scroll.offset -= over * over * EDGE_SPEED * dt;
                        } else if mouse > view.bottom() - EDGE {
                            let over = (mouse - (view.bottom() - EDGE)) as f32 / EDGE as f32;
                            queue.scroll.offset += over * over * EDGE_SPEED * dt;
                        }
                        let content = queue.songs.len() as i32 * ROW_STEP;
                        let max_scroll = (content - view.height).max(0) as f32;
                        queue.scroll.offset = queue.scroll.offset.clamp(0.0, max_scroll);

                        let centre =
                            y as i32 + ROW_HEIGHT / 2 - view.y + queue.scroll.offset as i32;
                        let last = queue.songs.len().saturating_sub(1) as i32;
                        let target = (centre / ROW_STEP).clamp(0, last) as usize;
                        if target != d.index {
                            let song_id = queue.songs.remove(d.index);
                            queue.songs.insert(target, song_id);
                            if queue.playing_song == Some(d.index) {
                                queue.playing_song = Some(target);
                            } else if let Some(cur) = queue.playing_song {
                                if d.index < cur && target >= cur {
                                    queue.playing_song = Some(cur - 1);
                                } else if d.index > cur && target <= cur {
                                    queue.playing_song = Some(cur + 1);
                                }
                            }
                            d.index = target;
                        }
                    }

                    carried = (y - view.y as f32 + queue.scroll.offset) / ROW_STEP as f32;
                    ui.animating = true;
                }
            }

            let drag = &mut queue.drag;
            let songs = &queue.songs;
            let scroll = queue.scroll.offset + queue.scroll.stretch;

            ui.flow_scroll(flow().elastic(true).gap(ROW_GAP), &mut queue.scroll, |ui| {
                for (i, &song_id) in songs.iter().enumerate() {
                    let Some(song) = db.song(song_id) else {
                        continue;
                    };
                    let dragging = drag.as_ref().is_some_and(|d| d.index == i);
                    let slot = ui.with_id(song_id, |ui| {
                        ui.animate_f32(
                            if dragging { carried } else { i as f32 },
                            if dragging { 0.0 } else { SLIDE },
                            Ease::OutCubic,
                        )
                    });
                    let y = view.y + (slot * ROW_STEP as f32 - scroll) as i32;

                    // The hole the floating row left behind.
                    if dragging {
                        ui.place_down(
                            flow()
                                .x(view.x)
                                .y(view.y + i as i32 * ROW_STEP - scroll as i32)
                                .width(Size::FillMinus(36))
                                .height(ROW_HEIGHT)
                                .radius(8)
                                .bg(GRAY_900),
                            |_| {},
                        );
                    }

                    ui.flow_right(
                        flow()
                            .y(y)
                            .fillw()
                            .height(ROW_HEIGHT)
                            .depth(if dragging { 1 } else { 0 })
                            .children_center(),
                        |ui| {
                            let selected =
                                Some(song_id) == queue.playing_song.map(|s| queue.songs[s]);
                            ui.flow_right(
                                flow()
                                    .radius(8)
                                    .gap(12)
                                    .width(-36)
                                    .height(ROW_HEIGHT)
                                    .children_center()
                                    .padlr(12)
                                    .bg(if selected { ROW_SELECTED } else { BODY })
                                    .hover(if drag.is_some() { BODY } else { ROW_SELECTED }),
                                |ui| {
                                    ui.text(
                                        ui.fmt(format_args!("{:02}", song.track_number)),
                                        text().w(20).fg(TEXT_FAINT),
                                    );
                                    if let Some((pixels, width, height)) = db.artwork(song) {
                                        ui.image(
                                            Image {
                                                width,
                                                height,
                                                pixels,
                                            },
                                            image().wh(36),
                                        );
                                    } else {
                                        ui.rect(rect().wh(36).radius(4).bg(BORDER_DIM));
                                    }

                                    let [title_col, artist_col, _] =
                                        ui.split_hs(ui.current_frame_bounds(), [0.48, 0.38, 0.14]);

                                    ui.text(
                                        ui.fmt(format_args!("{}", song.title)),
                                        text()
                                            .width(title_col.width)
                                            .content_left()
                                            .fg(TEXT)
                                            .clip(true),
                                    );

                                    ui.text(
                                        ui.fmt(format_args!("{} · {}", song.artist, song.album)),
                                        text()
                                            .width(artist_col.width)
                                            .content_left()
                                            .fg(TEXT_TERTIARY)
                                            .clip(true),
                                    );

                                    let t = time(ui, song.duration);
                                    ui.text(t, text().fg(TEXT_MUTED));
                                },
                            );

                            ui.gap(12);

                            let dot = circle().wh(2).bg(TEXT_FAINT);
                            let handle = ui.flow_down(
                                flow()
                                    .height(3 * 2 + 2 * 3) // 12px
                                    .gap(3)
                                    .hover(GRAY_800)
                                    .radius(4)
                                    .mar(6),
                                |ui| {
                                    for _ in 0..3 {
                                        ui.flow_right(flow().gap(3), |ui| {
                                            ui.circle(dot);
                                            ui.circle(dot);
                                        });
                                    }
                                },
                            );

                            if drag.is_none()
                                && ui.dragged(handle.bounds)
                                && let Some(start) = ui.left_mouse_start
                            {
                                *drag = Some(Drag {
                                    index: i,
                                    grab: start.y - y,
                                });
                            }
                        },
                    );
                }
            });
        },
    );
}
