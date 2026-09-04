use mu_core::db::album_year;

use crate::*;
pub struct Library<'a> {
    pub bounds: Rect,
    pub artist: &'a str,
    pub total_tracks: usize,
    ///(Album, Song)
    pub playing_song: Option<(usize, usize)>,
    ///(Album, Song)
    pub selected_song: Option<(usize, usize)>,
    pub scroll: Scroll,
    pub update_playing: bool,
}

pub fn draw_library<'a, 'b: 'a>(
    albums: &'a [mu_core::Song],
    library: &mut Library<'b>,
    ui: &mut FrameContext<'_, 'a>,
) {
    ui.flow_down(
        flow().bounds(library.bounds).padtb(12).padlr(36).bg(BODY),
        |ui| {
            ui.text(library.artist, text().font_size(42));
            //TODO: Add letter spacing?
            ui.gap(4);
            let header = ui.fmt(format_args!(
                "{} ALBUMS · {} TRACKS",
                albums.len(),
                library.total_tracks,
            ));
            ui.text(header, text().font_size(12).fg(TEXT_MUTED));
            ui.gap(12);
            ui.rect(rect().fillw().height(1).bg(BORDER_DIM));
            ui.gap(12);

            //TODO: Make mouse scroll better on elastic so it doesn't need to be disabled.
            let scroll_style = flow();

            // #[cfg(not(target_os = "windows"))]
            let scroll_style = flow().elastic(true);

            ui.scroll(scroll_style, &mut library.scroll, |ui| {
                let title_height = ui
                    .measure_text("A", Font::default(), 24, None, i32::MAX)
                    .height;
                let mut rendered = 0;

                let albums = albums
                    .chunk_by(|a, b| a.album == b.album)
                    .map(|chunk| (&chunk[0].album, chunk))
                    .enumerate();

                // let mut total_albums = 0;
                for (ai, (album, songs)) in albums {
                    let rows = songs.len() as i32;
                    let row_gap = 2;
                    let row_height = 36;
                    let title_gap = 12;
                    let height =
                        (title_height + title_gap + rows * row_height + (rows - 1) * row_gap)
                            .max(148);

                    ui.flow_right(flow().height(height).fillw(), |ui| {
                        rendered += 1;
                        //In terms of API, images are always user retained.
                        //It's a bit painful to deal with for the current use case.
                        //Each library page can have an unbounded number of images.

                        if let Some(first) = &songs.first()
                            && let Some(Artwork::Decoded(pixels, width, height)) = &first.artwork
                        {
                            let img = Image {
                                width: *width,
                                height: *height,
                                pixels,
                            };
                            ui.image(img, image().radius(8).wh(148));
                        } else {
                            //TODO: Better placeholder
                            ui.rect(rect().wh(148).bg(BORDER_DIM));
                        }

                        ui.gap(24);

                        ui.flow_down(flow(), |ui| {
                            let tracks = if songs.len() > 1 { "tracks" } else { "track" };
                            let year = ui.fmt(format_args!(
                                "{} · {} {}",
                                album_year(songs),
                                songs.len(),
                                tracks
                            ));
                            ui.lines(
                                [
                                    line(album, text().font_size(24).padr(12)),
                                    line(year, text().font_size(16).fg(TEXT_MUTED)),
                                ],
                                text().height(title_height),
                            );

                            ui.gap(title_gap);

                            let s = text()
                                .content_left()
                                .font_size(16)
                                .radius(12)
                                .padlr(6)
                                .padtb(4);

                            let row = flow()
                                .radius(12)
                                .padlr(6)
                                .padtb(4)
                                .hover(ROW_HOVER)
                                .height(row_height);

                            for (si, song) in songs.iter().enumerate() {
                                let playing = Some((ai, si)) == library.playing_song;
                                let selected = Some((ai, si)) == library.selected_song;

                                let row = if playing {
                                    row.bg(ROW_SELECTED)
                                } else if selected {
                                    //TODO: Maybe a dimmer version of this?
                                    row.bg(ROW_SELECTED)
                                } else {
                                    row
                                };

                                let song_row = ui.flow_right(row, |ui| {
                                    let track_number =
                                        ui.fmt(format_args!("{:02}", song.track_number));

                                    let number_color = if playing { ACCENT } else { TEXT_MUTED };
                                    ui.text(track_number, s.fg(number_color));

                                    let title_style =
                                        if playing { s } else { s.fg(TEXT_SECONDARY) };
                                    ui.text(&song.title, title_style);

                                    let duration = Duration::from_secs_f32(song.duration);
                                    let total_secs = duration.as_secs();
                                    let hours = total_secs / 3600;
                                    let minutes = (total_secs % 3600) / 60;
                                    let seconds = total_secs % 60;

                                    let duration = if hours > 0 {
                                        ui.fmt(format_args!(
                                            "{:02}:{:02}:{:02}",
                                            hours, minutes, seconds
                                        ))
                                    } else {
                                        ui.fmt(format_args!("{:02}:{:02}", minutes, seconds))
                                    };

                                    ui.text(duration, s.fg(TEXT_MUTED).fillw().content_right());
                                });

                                if song_row.clicked {
                                    library.selected_song = Some((ai, si));
                                }

                                if song_row.double_clicked {
                                    library.playing_song = Some((ai, si));
                                    library.update_playing = true;
                                }

                                if (si + 1) < songs.len() {
                                    ui.gap(row_gap)
                                }
                            }
                        });
                    });

                    //TODO: How to get album lengths without running lazy iter twice.
                    // if (ai + 1) < total_albums {
                    //     ui.gap(24);
                    //     ui.rect(rect().fillw().bg(BORDER_DIM).h(1));
                    //     ui.gap(24);
                    // }
                    // println!("rendered {}/{} albums", rendered, albums.len());
                }
            });
        },
    );
}
