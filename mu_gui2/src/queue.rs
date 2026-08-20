use crate::*;
use mu_core::Song;

pub struct Queue {
    pub songs: Vec<Song>,
    pub playing_song: Option<usize>,
    pub bounds: Rect,
    pub scroll: Scroll,
}

pub fn draw_queue(ui: &mut FrameContext, queue: &mut Queue) {
    const ITEM_HEIGHT: usize = 48;

    ui.flow_down(
        flow().bounds(queue.bounds).padlr(36).padtb(12).bg(BODY),
        |ui| {
            //
            ui.text("Queue", text().font_size(42));

            let subtext = ui.fmt(format_args!(
                "{} tracks · 34 min remaining",
                queue.songs.len()
            ));
            ui.text(subtext, text().font_size(14).fg(TEXT_MUTED));

            ui.gap(12);
            ui.rect(rect().fillw().height(1).bg(BORDER_DIM));
            ui.gap(12);

            ui.flow_scroll(flow().gap(4), &mut queue.scroll, |ui| {
                //
                for song in &queue.songs {
                    ui.flow_right(flow().height(60).children_center(), |ui| {
                        ui.flow_right(
                            flow()
                                .radius(8)
                                .gap(12)
                                .height(60)
                                .children_center()
                                .padlr(12)
                                .hover(ROW_SELECTED),
                            |ui| {
                                ui.text(
                                    ui.fmt(format_args!("{:02}", song.track_number)),
                                    text().w(20).fg(TEXT_FAINT),
                                );
                                ui.rect(rect().wh(36).radius(4).bg(BORDER_DIM));

                                let [title_col, artist_col, meta_col] =
                                    ui.split_hs(ui.current_frame_bounds(), [0.48, 0.38, 0.14]);

                                ui.text(
                                    ui.fmt(format_args!("{}", song.title)),
                                    text()
                                        .width(title_col.width)
                                        .content_left()
                                        .fg(TEXT)
                                        .clip(true),
                                );

                                let txt = ui.fmt(format_args!("{} · {}", song.artist, song.album));
                                ui.text(
                                    txt,
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

                        //TODO: Hover color for dots.
                        //Need to retain the handle state.
                        let dot = circle().wh(2).bg(TEXT_FAINT);
                        let handle = ui.flow_down(
                            flow()
                                .height(3 * 2 + 2 * 3) // 12px
                                .gap(3)
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
                    });
                }
            });
        },
    );
}
