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

pub fn load_covers(db: &mut Database, artist: &str) -> Vec<Option<Image>> {
    mini::profile!();
    let Some(albums) = db.btree.get_mut(artist) else {
        return Vec::new();
    };
    albums
        .par_iter_mut()
        //This should really by async.
        .map(|album| {
            let song = album.songs.first_mut()?;
            if song.artwork.is_none() {
                if let Ok(meta) = onmi::metadata(&song.path, false, true) {
                    song.artwork = meta.artwork;
                }
            }
            let art = song.artwork.as_ref()?;
            Image::decode(&art.data)
                .ok()
                .map(|img| img.thumbnail(COVER_PX))
        })
        .collect()
}

pub fn draw<'a>(
    ui: &mut FrameContext<'_, 'a>,
    rect: Rect,
    db: &Database,
    artists: &[String],
    artist: &str,
    covers: &'a [Option<Image>],
    playing_path: Option<&str>,
    selection: &mut PathSelection,
    menu: &mut ContextMenu,
    scroll: &mut usize,
) -> Option<Action> {
    if !artists.iter().any(|a| a == artist) {
        return Some(Action::MissingArtist);
    }

    let albums = db.albums_by_artist(artist);
    let album_data: Vec<(String, u16, Vec<Song>)> = albums
        .iter()
        .map(|a| {
            let songs = a.songs.iter().cloned().collect();
            (a.title.clone(), a.year(), songs)
        })
        .collect();
    let artist_owned = artist.to_string();
    let total_tracks: usize = album_data.iter().map(|(_, _, s)| s.len()).sum();
    let album_count = album_data.len();
    let shift = ui.window.modifiers().shift;
    let ctrl = ui.window.modifiers().ctrl;
    let playing = playing_path.map(|s| s.to_string());
    let mut action = None;

    let ordered_paths: Vec<String> = album_data
        .iter()
        .flat_map(|(_, _, songs)| songs.iter().map(|s| s.path.clone()))
        .collect();
    let all_songs: Vec<Song> = album_data
        .iter()
        .flat_map(|(_, _, songs)| songs.iter().cloned())
        .collect();

    let (header_rect, list_rect) = ui.split_rect_v(rect, HEADER_H);

    ui.place_down(bounds(header_rect).bg(colors::BG), |ui| {
        ui.text(
            artist_owned.clone(),
            style()
                .fg(colors::TEXT)
                .font_size(32)
                .padl(40)
                .padt(28)
                .padb(4)
                .fill_width()
                .align(Alignment::Left),
        );
        ui.text(
            format!(
                "{} album{} · {} track{}",
                album_count,
                if album_count == 1 { "" } else { "s" },
                total_tracks,
                if total_tracks == 1 { "" } else { "s" }
            ),
            style()
                .fg(colors::TEXT_MUTED)
                .font_size(13)
                .padl(40)
                .fill_width()
                .align(Alignment::Left),
        );
    });

    ui.paint_rect(
        Rect::new(list_rect.x, list_rect.y, list_rect.width, 1),
        style().bg(colors::LINE),
    );

    ui.scroll(bounds(list_rect).bg(colors::BG), scroll, |ui| {
        ui.gap(16);

        for (i, (album_title, year, songs)) in album_data.iter().enumerate() {
            let body_h = 28 + 20 + songs.len() as i32 * 28;
            let card_h = COVER.max(body_h);
            let subtext = if *year > 0 {
                format!("{year} · {} tracks", songs.len())
            } else {
                format!("{} tracks", songs.len())
            };
            let album_songs = songs.clone();

            ui.flow_right(
                style().padl(40).padr(40).height(card_h).fill_width(),
                |ui| {
                    let layout = ui.walk_layout(COVER, COVER, 0);
                    let cover_rect = Rect::new(layout.paint_x, layout.paint_y, COVER, COVER);
                    if let Some(Some(img)) = covers.get(i) {
                        ui.paint_image(
                            cover_rect,
                            img,
                            style().image_fit(ImageFit::Stretch).radius(8),
                        );
                    } else {
                        paint_cover(ui, cover_rect, 8);
                    }
                    if ui.double_clicked(cover_rect) && !album_songs.is_empty() {
                        if let Some(index) =
                            ordered_paths.iter().position(|p| p == &album_songs[0].path)
                        {
                            action = Some(Action::PlayDiscography {
                                songs: all_songs.clone(),
                                index,
                            });
                        }
                    }
                    // Right-click cover → album menu.
                    if let Some((mx, my)) = context_menu::right_click_at(ui, cover_rect) {
                        if let Some(first) = album_songs.first() {
                            let index = ordered_paths
                                .iter()
                                .position(|p| p == &first.path)
                                .unwrap_or(0);
                            menu.open_at(
                                mx,
                                my,
                                vec![
                                    (
                                        "Play from album".into(),
                                        MenuCommand::Play {
                                            songs: all_songs.clone(),
                                            index,
                                        },
                                    ),
                                    (
                                        "Add album to queue".into(),
                                        MenuCommand::AddToQueue(album_songs.clone()),
                                    ),
                                ],
                            );
                        }
                    }
                    ui.gap(26);

                    ui.flow_down(style().fill_width().height(card_h), |ui| {
                        ui.text(
                            album_title.clone(),
                            style()
                                .fg(colors::TEXT)
                                .font_size(20)
                                .padb(2)
                                .fill_width()
                                .align(Alignment::Left),
                        );
                        ui.text(
                            subtext.clone(),
                            style()
                                .fg(colors::TEXT_DIM)
                                .font_size(12)
                                .padb(8)
                                .padt(2)
                                .fill_width()
                                .align(Alignment::Left),
                        );

                        let row_style = style()
                            .padlr(8)
                            .padtb(5)
                            .fill_width()
                            .radius(6)
                            .align(Alignment::Left)
                            .hover(colors::HOVER)
                            .fg(colors::TEXT)
                            .selected(colors::ACCENT_DIM);

                        for song in songs.iter() {
                            let is_playing = playing.as_deref() == Some(song.path.as_str());
                            let is_selected = selection.contains(&song.path);
                            // ASCII only — default UI font has no ♪ glyph (rendered as ?).
                            let label = if is_playing {
                                format!(">  {}.  {}", song.track_number, song.title)
                            } else {
                                format!("   {}.  {}", song.track_number, song.title)
                            };
                            let state = ui.item(label, row_style.is_selected(is_selected));
                            if state.double_clicked {
                                selection.select_only(song.path.clone());
                                if let Some(index) =
                                    ordered_paths.iter().position(|p| p == &song.path)
                                {
                                    action = Some(Action::PlayDiscography {
                                        songs: all_songs.clone(),
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
                                    &all_songs,
                                    &album_songs,
                                    &song.path,
                                    mx,
                                    my,
                                );
                            }
                        }
                    });
                },
            );

            if i + 1 < album_data.len() {
                ui.gap(ALBUM_GAP);
                ui.flow_right(style().padl(40).padr(40).fill_width().height(1), |ui| {
                    ui.rect(style().fill_width().height(1).bg(colors::LINE));
                });
                ui.gap(ALBUM_GAP);
            } else {
                ui.gap(40);
            }
        }
    });

    action
}

fn open_song_menu(
    menu: &mut ContextMenu,
    selection: &mut PathSelection,
    ordered_paths: &[String],
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
    let play_index = ordered_paths.iter().position(|p| p == path).unwrap_or(0);

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
