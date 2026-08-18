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

    ui.flow_down(bounds(queue.bounds).padlr(36).padtb(12).bg(BODY), |ui| {
        //
        ui.text("Queue", style().font_size(42));

        let subtext = ui.fmt(format_args!(
            "{} tracks · 34 min remaining",
            queue.songs.len()
        ));
        ui.text(subtext, font_size(14).fg(TEXT_MUTED));

        ui.gap(12);
        ui.rect(style().fill_width().height(1).bg(BORDER_DIM));
        ui.gap(12);

        ui.flow_scroll(style().gap(4), &mut queue.scroll, |ui| {
            //
            for song in &queue.songs {
                ui.flow_right(
                    style()
                        .radius(8)
                        .gap(12)
                        .height(48 + 12)
                        .fill_width()
                        .align_flow(AlignFlow::Center)
                        .padlr(12)
                        .hover(ROW_SELECTED),
                    |ui| {
                        //TODO: Should probably split this 
                        // let a = ui.split_hs(ui.current_frame_bounds(), [0.3]);
                        ui.text(
                            ui.fmt(format_args!("{}", song.track_number)),
                            style().fg(TEXT_FAINT),
                        );
                        ui.rect(style().wh(36).bg(BORDER_DIM));
                        ui.text(
                            ui.fmt(format_args!("{}", song.title)),
                            style().width(0.7).align_left().fg(TEXT),
                        );

                        let txt = ui.fmt(format_args!("{} · {}", song.artist, song.album));
                        ui.text(txt, style().fg(TEXT_TERTIARY));

                        let t = time(ui, song.duration);
                        ui.text(t, style().fg(TEXT_MUTED).align_right());

                        ui.rect(style().w(4).h(12).bg(BORDER))
                    },
                );
            }
        });
    });
}

pub fn handle(ui: FrameContext) {
    // let ball = style().wh(2).fg(TEXT_SECONDARY).radius(2);
    // ui.circle()
    // ui.rect()
}