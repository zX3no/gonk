use crate::*;
use mu_core::{Database, SongId};

pub struct Controls {
    pub current_song: Option<SongId>,
    pub bounds: Rect,
    pub playing: bool,
    pub shuffle: bool,
    pub repeat: bool,
    pub muted: bool,
    pub elapsed: f32,
    pub duration: f32,
    pub volume: u8,
}

pub fn draw_controls<'a>(
    controls: &mut Controls,
    player: &mut onmi::Player,
    ui: &mut FrameContext<'_, 'a>,
    db: &'a Database,
) {
    ui.paint_rect(
        controls.bounds,
        rect().bg(SIDEBAR).border(BORDER_DIM).border_side(TOP),
    );

    let [info, center, extras] = ui.split_hs(controls.bounds, [0.28, 0.44, 0.28]);
    if let Some(song_id) = controls.current_song
        && let Some(song) = db.song(song_id)
    {
        ui.flow_right(
            flow()
                .bounds(info)
                .padlr(16)
                .gap(12)
                .children_center()
                .clip(true),
            |ui| {
                if let Some((pixels, width, height)) = db.artwork(song) {
                    let img = Image {
                        width,
                        height,
                        pixels,
                    };
                    ui.image(img, image().radius(6).wh(48));
                } else {
                    //TODO: Better placeholder
                    ui.rect(rect().wh(48).radius(4).bg(BORDER_DIM));
                }
                ui.flow_down(flow().height(40), |ui| {
                    let title = ui.fmt(format_args!("{}", song.title));
                    ui.text(title, text().fg(TEXT));
                    let txt = ui.fmt(format_args!("{} · {}", song.artist, song.album));
                    ui.text(txt, text().font_size(14).fg(TEXT_MUTED));
                });
            },
        );
    }

    let t = text().w(36).font_size(13).fg(TEXT_MUTED);
    let btn = rect()
        .wh(32)
        .pad(4)
        .radius(8)
        .hover(ROW_HOVER)
        .fg(TEXT_TERTIARY);

    ui.flow_down(
        flow()
            .bounds(center)
            .padtb(12)
            .gap(4)
            .children_center()
            .clip(true),
        |ui| {
            ui.flow_right(
                flow().w(200).clip(true).h(36).gap(10).children_center(),
                |ui| {
                    icon(ui, "Shuffle", btn);
                    icon(ui, "Rewind", btn);
                    if icon(
                        ui,
                        if controls.playing { "Pause" } else { "Play" },
                        btn.bg(TEXT).hover(PLAY_HOVER).fg(SIDEBAR).radius(16),
                    )
                    .clicked
                    {
                        if controls.playing {
                            player.pause();
                        } else {
                            player.play();
                        }
                        controls.playing = !controls.playing;
                    }
                    icon(ui, "Forward", btn);
                    icon(ui, "Repeat", btn);
                },
            );

            ui.flow_right(flow().clip(true).h(20).gap(10).children_center(), |ui| {
                let elapsed = time(ui, controls.elapsed);
                ui.text(elapsed, t.content_right());
                let track = ui.rect(rect().w(Size::FillMinus(46)).h(4).bg(TRACK_EMPTY).radius(2));

                //Outset the seekbar verticall so it's easier to drag.
                let outset = track.bounds.outer(0, 12);

                //TODO: Dragged only works with released mouse input
                //So we have to duplicate the logic here.
                if controls.playing
                    && let Some(release) = ui.left_mouse_release
                    && release.intersects(ui.hit(outset))
                {
                    let x = ui.mouse_position().x.saturating_sub(outset.x);
                    let pos = (x as f32 / outset.width as f32).clamp(0.0, 1.0);
                    let pos = player.duration().as_secs_f32() * pos;
                    player.seek_to(Duration::from_secs_f32(pos));
                }

                let duration = time(ui, controls.duration);
                ui.text(duration, t.content_left());

                ui.paint_rect(
                    track.bounds.width(
                        (track.bounds.width as f32 * controls.elapsed / controls.duration) as i32,
                    ),
                    rect().bg(ACCENT).radius(2),
                );
            });
        },
    );

    ui.flow_left(
        flow()
            .bounds(extras)
            .padlr(16)
            .gap(10)
            .clip(true)
            .children_center(),
        |ui| {
            let volume = ui.fmt(format_args!("{}", controls.volume));
            ui.text(volume, text().w(24).font_size(13).fg(TEXT_MUTED));
            let slider = ui.rect(rect().w(96).h(4).radius(2).bg(TRACK_EMPTY));
            ui.paint_rect(
                slider
                    .bounds
                    .width((slider.bounds.width as f32 * controls.volume as f32 / 100.0) as i32),
                rect().bg(ACCENT).radius(2),
            );
            let outset = slider.bounds.outer(0, 12);
            if let Some(pos) = ui.drag_percentage_x(outset) {
                controls.volume = ((pos * 100.0) as u8).clamp(0, 100);
                player.set_volume(controls.volume);
            }
            icon(ui, "Volume", btn);
        },
    );
}
