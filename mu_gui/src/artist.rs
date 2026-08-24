use crate::context_menu::{self, ContextMenu, MenuCommand};
use crate::selection::PathSelection;
use crate::theme::{colors, paint_cover};
use mu_core::Song;
use mu_core::vdb::Database;
use neoui::*;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};

const COVER: i32 = 140;
/// Pixel size stored for covers (2× logical for hi-dpi).
const COVER_PX: usize = (COVER * 2) as usize;
const HEADER_H: i32 = 96;
const ALBUM_GAP: i32 = 28;

pub enum Action {
    MissingArtist,
    /// Play through the artist discography starting at `index` (does not touch the queue).
    PlayDiscography {
        songs: Vec<Song>,
        index: usize,
    },
}

pub fn load_covers(db: &mut Database, artist: &str) {
    mini::profile!();
    let Some(albums) = db.btree.get_mut(artist) else {
        return;
    };
    albums
        .par_iter_mut()
        //This should really by async.
        .for_each(|album| {
            let Some(song) = album.songs.first_mut() else {
                return;
            };
            match &song.artwork {
                Some(mu_core::db::Artwork::Decoded(..)) => {}
                Some(mu_core::db::Artwork::Compressed(art)) => {
                    if let Ok((pixels, width, height)) = neoui::image::decode(&art.data) {
                        let pixels = if width == COVER_PX && height == COVER_PX {
                            pixels
                        } else {
                            neoui::image::resize(
                                Image {
                                    pixels: &pixels,
                                    width,
                                    height,
                                },
                                COVER_PX,
                                COVER_PX,
                            )
                        };
                        song.artwork = Some(mu_core::db::Artwork::Decoded(
                            pixels.into_boxed_slice(),
                            COVER_PX,
                            COVER_PX,
                        ));
                    }
                }
                None => {
                    if let Ok(meta) = onmi::metadata(&song.path, false, true) {
                        if let Some(art) = meta.artwork {
                            if let Ok((pixels, width, height)) = neoui::image::decode(&art.data) {
                                let pixels = if width == COVER_PX && height == COVER_PX {
                                    pixels
                                } else {
                                    neoui::image::resize(
                                        Image {
                                            pixels: &pixels,
                                            width,
                                            height,
                                        },
                                        COVER_PX,
                                        COVER_PX,
                                    )
                                };
                                song.artwork = Some(mu_core::db::Artwork::Decoded(
                                    pixels.into_boxed_slice(),
                                    COVER_PX,
                                    COVER_PX,
                                ));
                            }
                        }
                    }
                }
            }
        });
}

pub fn draw<'a>(
    ui: &mut FrameContext<'_, 'a>,
    rect: Rect,
    db: &'a Database,
    artists: &[String],
    artist: &str,
    playing_path: Option<&str>,
    selection: &mut PathSelection,
    menu: &mut ContextMenu,
    scroll: &mut Scroll,
) -> Option<Action> {
    if !artists.iter().any(|a| a == artist) {
        return Some(Action::MissingArtist);
    }

    let albums = db.albums_by_artist(artist);
    let total_tracks: usize = albums.iter().map(|a| a.songs.len()).sum();
    let album_count = albums.len();
    let shift = ui.window.modifiers().shift;
    let ctrl = ui.window.modifiers().ctrl;
    let mut action = None;

    let ordered_paths: Vec<&str> = albums
        .iter()
        .flat_map(|a| a.songs.iter().map(|s| s.path.as_str()))
        .collect();

    let get_all_songs = || -> Vec<Song> {
        albums
            .iter()
            .flat_map(|a| a.songs.iter().cloned())
            .collect()
    };

    let (header_rect, list_rect) = ui.split_rect_v(rect, HEADER_H);

    ui.place_down(flow().bounds(header_rect).bg(colors::BG), |ui| {
        ui.text(
            artist.to_string(),
            text()
                .fg(colors::TEXT)
                .font_size(32)
                .padl(40)
                .padt(28)
                .padb(4)
                .fillw()
                .content(Alignment::Left),
        );
        let txt = ui.fmt(format_args!(
            "{} album{} · {} track{}",
            album_count,
            if album_count == 1 { "" } else { "s" },
            total_tracks,
            if total_tracks == 1 { "" } else { "s" }
        ));
        ui.text(
            txt,
            text()
                .fg(colors::TEXT_MUTED)
                .font_size(13)
                .padl(40)
                .fillw()
                .content(Alignment::Left),
        );
    });

    ui.paint_rect(
        Rect::new(list_rect.x, list_rect.y, list_rect.width, 1),
        neoui::rect().bg(colors::LINE),
    );

    ui.scroll(flow().bounds(list_rect).bg(colors::BG), scroll, |ui| {
        ui.gap(16);

        for (i, album) in albums.iter().enumerate() {
            let year = album.year();
            let body_h = 28 + 20 + album.songs.len() as i32 * 28;
            let card_h = COVER.max(body_h);
            let subtext = if year > 0 {
                ui.fmt(format_args!("{year} · {} tracks", album.songs.len()))
            } else {
                ui.fmt(format_args!("{} tracks", album.songs.len()))
            };

            ui.flow_right(flow().padlr(40).height(card_h).fillw(), |ui| {
                let layout = ui.walk_layout(COVER, COVER, 0, None);
                let cover_rect = Rect::new(layout.paint_x, layout.paint_y, COVER, COVER);
                if let Some(first) = album.songs.first()
                    && let Some(mu_core::db::Artwork::Decoded(pixels, width, height)) =
                        &first.artwork
                {
                    let img = Image {
                        pixels,
                        width: *width,
                        height: *height,
                    };
                    ui.paint_image(cover_rect, img, image().radius(8));
                } else {
                    paint_cover(ui, cover_rect, 8);
                }
                if ui.double_clicked(cover_rect) && !album.songs.is_empty() {
                    if let Some(index) =
                        ordered_paths.iter().position(|p| *p == album.songs[0].path)
                    {
                        action = Some(Action::PlayDiscography {
                            songs: get_all_songs(),
                            index,
                        });
                    }
                }
                // Right-click cover → album menu.
                if let Some((mx, my)) = context_menu::right_click_at(ui, cover_rect) {
                    if let Some(first) = album.songs.first() {
                        let index = ordered_paths
                            .iter()
                            .position(|p| *p == first.path)
                            .unwrap_or(0);
                        menu.open_at(
                            mx,
                            my,
                            vec![
                                (
                                    "Play from album".into(),
                                    MenuCommand::Play {
                                        songs: get_all_songs(),
                                        index,
                                    },
                                ),
                                (
                                    "Add album to queue".into(),
                                    MenuCommand::AddToQueue(album.songs.clone()),
                                ),
                            ],
                        );
                    }
                }
                ui.gap(26);

                ui.flow_down(flow().fillw().height(card_h), |ui| {
                    ui.text(
                        album.title.to_string(),
                        text()
                            .fg(colors::TEXT)
                            .font_size(20)
                            .padb(2)
                            .fillw()
                            .content(Alignment::Left),
                    );
                    ui.text(
                        subtext,
                        text()
                            .fg(colors::TEXT_DIM)
                            .font_size(12)
                            .padb(8)
                            .padt(2)
                            .fillw()
                            .content(Alignment::Left),
                    );

                    let row_style = text()
                        .padlr(8)
                        .padtb(5)
                        .fillw()
                        .radius(6)
                        .content(Alignment::Left)
                        .hover(colors::HOVER)
                        .fg(colors::TEXT)
                        .selected(colors::ACCENT_DIM);

                    for song in album.songs.iter() {
                        let is_playing = playing_path == Some(song.path.as_str());
                        let is_selected = selection.contains(&song.path);
                        // ASCII only — default UI font has no ♪ glyph (rendered as ?).
                        let label = if is_playing {
                            ui.fmt(format_args!(">  {}.  {}", song.track_number, song.title))
                        } else {
                            ui.fmt(format_args!("   {}.  {}", song.track_number, song.title))
                        };
                        let state = ui.text(label, row_style.is_selected(is_selected));
                        if state.double_clicked {
                            selection.select_only(song.path.clone());
                            if let Some(index) = ordered_paths.iter().position(|p| *p == song.path)
                            {
                                action = Some(Action::PlayDiscography {
                                    songs: get_all_songs(),
                                    index,
                                });
                            }
                        } else if state.clicked {
                            selection.click(&ordered_paths, song.path.clone(), shift, ctrl);
                        }
                        if let Some((mx, my)) = context_menu::right_click_at(ui, state.bounds) {
                            open_song_menu(
                                menu,
                                selection,
                                &ordered_paths,
                                &get_all_songs(),
                                &album.songs,
                                &song.path,
                                mx,
                                my,
                            );
                        }
                    }
                });
            });

            if i + 1 < albums.len() {
                ui.gap(ALBUM_GAP);
                ui.flow_right(flow().padlr(40).fillw().height(1), |ui| {
                    ui.rect(neoui::rect().fillw().height(1).bg(colors::LINE));
                });
                ui.gap(ALBUM_GAP);
            } else {
                ui.gap(40);
            }
        }
    });

    action
}

fn open_song_menu<S: AsRef<str>>(
    menu: &mut ContextMenu,
    selection: &mut PathSelection,
    ordered_paths: &[S],
    all_songs: &[Song],
    album_songs: &[Song],
    path: &str,
    mx: i32,
    my: i32,
) {
    if !selection.contains(path) {
        selection.select_only(path.to_string());
    }
    let selected = selection.collect_songs(all_songs);
    let n = selected.len();
    let play_index = ordered_paths
        .iter()
        .position(|p| p.as_ref() == path)
        .unwrap_or(0);

    let add_label = if n <= 1 {
        "Add to queue".to_string()
    } else {
        format!("Add {n} to queue")
    };

    menu.open_at(
        mx,
        my,
        vec![
            (
                "Play".into(),
                MenuCommand::Play {
                    songs: all_songs.to_vec(),
                    index: play_index,
                },
            ),
            (add_label, MenuCommand::AddToQueue(selected)),
            (
                "Add album to queue".into(),
                MenuCommand::AddToQueue(album_songs.to_vec()),
            ),
        ],
    );
}
