use crate::theme::{colors, paint_cover};
use mu_core::vdb::Database;
use mu_core::Song;
use neoui::*;

const COVER: i32 = 140;
const HEADER_H: i32 = 96;
/// Vertical space between album sections.
const ALBUM_GAP: i32 = 28;

pub enum Action {
    MissingArtist,
    PlayAlbum { songs: Vec<Song>, index: usize },
    Append(Song),
}

pub fn draw(
    ui: &mut FrameContext<'_, '_>,
    rect: Rect,
    db: &Database,
    artists: &[String],
    artist: &str,
    playing_path: Option<&str>,
    selected_path: &mut Option<String>,
    scroll: &mut usize,
) -> Option<Action> {
    if !artists.iter().any(|a| a == artist) {
        return Some(Action::MissingArtist);
    }

    let albums = db.albums_by_artist(artist);
    // (title, year, songs) — year from tags when available (0 = unknown).
    let album_data: Vec<(String, u16, Vec<Song>)> = albums
        .iter()
        .map(|a| (a.title.clone(), a.year(), a.songs.clone()))
        .collect();
    let artist_owned = artist.to_string();
    let total_tracks: usize = album_data.iter().map(|(_, _, s)| s.len()).sum();
    let album_count = album_data.len();
    let shift = ui.window.modifiers().shift;
    let playing = playing_path.map(|s| s.to_string());
    let mut action = None;

    let (header_rect, list_rect) = ui.split_rect_v(rect, HEADER_H);

    // Sticky header — title + meta only (no Play all).
    ui.paint_rect(header_rect, style().bg(colors::BG));
    ui.flow_skip(bounds(header_rect), Flow::Down, |ui| {
        ui.text(
            artist_owned.clone(),
            style()
                .fg(colors::TEXT)
                .font_size(36)
                .padl(40)
                .padt(28)
                .padb(6)
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
                .padb(12)
                .fill_width()
                .align(Alignment::Left),
        );
    });

    ui.paint_rect(
        Rect::new(list_rect.x, list_rect.y, list_rect.width, 1),
        style().bg(colors::LINE),
    );

    ui.scroll_view(bounds(list_rect).bg(colors::BG), scroll, |ui| {
        ui.gap(16);

        for (i, (album_title, year, songs)) in album_data.iter().enumerate() {
            // Title + meta + tracks (≈28px per track row).
            let body_h = 28 + 20 + songs.len() as i32 * 28;
            let card_h = COVER.max(body_h);
            let subtext = if *year > 0 {
                format!("{year} · {} tracks", songs.len())
            } else {
                format!("{} tracks", songs.len())
            };

            ui.flow_right(
                style()
                    .padl(40)
                    .padr(40)
                    .height(card_h)
                    .fill_width(),
                |ui| {
                    let layout = ui.walk_layout(COVER, COVER, 0);
                    let cover_rect = Rect::new(layout.paint_x, layout.paint_y, COVER, COVER);
                    paint_cover(ui, cover_rect, 8);
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
                                .padb(10)
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

                        for (ti, song) in songs.iter().enumerate() {
                            let is_playing = playing.as_deref() == Some(song.path.as_str());
                            let is_selected =
                                selected_path.as_deref() == Some(song.path.as_str());
                            let label = if is_playing {
                                format!("♪  {}.  {}", song.track_number, song.title)
                            } else {
                                format!("    {}.  {}", song.track_number, song.title)
                            };
                            let state = ui.item(label, is_selected, row_style);
                            if state.clicked {
                                *selected_path = Some(song.path.clone());
                                if shift {
                                    action = Some(Action::Append(song.clone()));
                                } else if state.double_clicked {
                                    action = Some(Action::PlayAlbum {
                                        songs: songs.clone(),
                                        index: ti,
                                    });
                                }
                            }
                        }
                    });
                },
            );

            // Explicit gap between albums (and after last for scroll padding).
            if i + 1 < album_data.len() {
                ui.gap(ALBUM_GAP);
                // Hairline separator centered in the gap zone.
                ui.flow_right(style().padl(40).padr(40).fill_width().height(1), |ui| {
                    let layout = ui.walk_layout(ui.current_frame_bounds().width, 1, 0);
                    ui.paint_rect(layout.size, style().bg(colors::LINE));
                });
                ui.gap(ALBUM_GAP);
            } else {
                ui.gap(40);
            }
        }
    });

    action
}
