use std::borrow::Cow;

use crate::{ALBUM, ARTIST, NUMBER, SEEKER, TITLE};
use gonk_core::{log, Index};
use gonk_player::Player;
use unicode_width::UnicodeWidthStr;
use winter::*;

pub struct Queue {
    pub ui: Index<()>,
    pub constraint: [u16; 4],
    pub len: usize,
    pub player: *mut Player,
}

impl Queue {
    pub fn new(ui_index: usize, player: *mut Player) -> Self {
        Self {
            ui: Index::new(Vec::new(), Some(ui_index)),
            constraint: [6, 37, 31, 26],
            len: 0,
            player,
        }
    }
}

pub fn up(queue: &mut Queue) {
    queue.ui.up_with_len(queue.len);
}

pub fn down(queue: &mut Queue) {
    queue.ui.down_with_len(queue.len);
}

pub fn draw(queue: &mut Queue, area: winter::Rect, buf: &mut winter::Buffer, mouse: Option<Event>) {
    let player = unsafe { queue.player.as_mut().unwrap() };

    let area = layout![
        area,
        Direction::Vertical,
        Constraint::Length(3),
        Constraint::Min(10),
        Constraint::Length(3)
    ];

    //Header
    block(
        Some(
            if player.songs.is_empty() {
                "Stopped"
            } else if player.is_playing() {
                "Playing"
            } else {
                "Paused"
            }
            .into(),
        ),
        Borders::TOP | Borders::LEFT | Borders::RIGHT,
        BorderType::Rounded,
    )
    .margin(1)
    .draw(area[0], buf);

    if !player.songs.is_empty() {
        //Title
        if let Some(song) = player.songs.selected() {
            let mut artist = song.artist.trim_end().to_string();
            let mut album = song.album.trim_end().to_string();
            let mut title = song.title.trim_end().to_string();
            let max_width = area[0].width.saturating_sub(30) as usize;
            let separator_width = "-| - |-".width();

            if max_width == 0 || max_width < separator_width {
                return;
            }

            while artist.width() + album.width() + separator_width > max_width {
                if artist.width() > album.width() {
                    artist.pop();
                } else {
                    album.pop();
                }
            }

            while title.width() > max_width {
                title.pop();
            }

            let n = title
                .width()
                .saturating_sub(artist.width() + album.width() + 3);
            let rem = n % 2;
            let pad_front = " ".repeat(n / 2);
            let pad_back = " ".repeat(n / 2 + rem);

            let top = lines_s![
                format!("─│ {pad_front}"),
                style(),
                artist,
                fg(ARTIST),
                " ─ ",
                style(),
                album,
                fg(ALBUM),
                format!("{pad_back} │─"),
                style()
            ];
            top.align(Center).draw(area[0], buf);

            let bottom = lines_s![title, fg(TITLE)];
            let mut area = area[0];
            if area.height > 1 {
                area.y += 1;
                bottom.align(Center).draw(area, buf)
            }
        }
    }

    let volume = if player.mute {
        lines!("Mute─╮")
    } else {
        lines!(format!("Vol: {}%─╮", player.volume()))
    };
    volume.align(Right).draw(area[0], buf);

    //Body
    let row_bounds = if log::last_message().is_some() && player.songs.is_empty() {
        block(
            None,
            Borders::LEFT | Borders::RIGHT | Borders::BOTTOM,
            BorderType::Rounded,
        )
        .draw(area[1], buf);
        None
    } else if player.songs.is_empty() {
        block(None, Borders::LEFT | Borders::RIGHT, BorderType::Rounded).draw(area[1], buf);
        None
    } else {
        let (songs, player_index, ui_index) =
            (&player.songs, player.songs.index(), queue.ui.index());

        let mut rows: Vec<Row> = songs
            .iter()
            .map(|song| {
                row![
                    text!(""),
                    text!(song.track_number.to_string(), fg(NUMBER)),
                    text!(song.title.as_str(), fg(TITLE)),
                    text!(song.album.as_str(), fg(ALBUM)),
                    text!(song.artist.as_str(), fg(ARTIST))
                ]
            })
            .collect();

        if let Some(player_index) = player_index {
            if let Some(song) = songs.get(player_index) {
                if let Some(ui_index) = ui_index {
                    //Currently playing song
                    let row = if ui_index == player_index {
                        rows[player_index] = row![
                            text!(">>", fg(White).dim().bold()),
                            text!(song.track_number.to_string(), bg(NUMBER).fg(Black)),
                            text!(song.title.as_str(), bg(TITLE).fg(Black)),
                            text!(song.album.as_str(), bg(ALBUM).fg(Black)),
                            text!(song.artist.as_str(), bg(ARTIST).fg(Black))
                        ];
                    } else {
                        // row![
                        //     text!(">>", fg(White).dim().bold()),
                        //     text!(song.track_number.to_string(), fg(NUMBER)),
                        //     text!(song.title.as_str(), fg(TITLE)),
                        //     text!(song.album.as_str(), fg(ALBUM)),
                        //     text!(song.artist.as_str(), fg(ARTIST))
                        // ]
                        rows[player_index].columns[0] = text!(">>", fg(White).dim().bold()).into();
                    };

                    // items.remove(player_index);
                    // items.insert(player_index, row);

                    //Current selection
                    if ui_index != player_index {
                        if let Some(song) = songs.get(ui_index) {
                            rows[ui_index] = row![
                                text!(""),
                                text!(song.track_number.to_string(), fg(Black).bg(NUMBER)),
                                text!(song.title.as_str(), fg(Black).bg(TITLE)),
                                text!(song.album.as_str(), fg(Black).bg(ALBUM)),
                                text!(song.artist.as_str(), fg(Black).bg(ARTIST))
                            ];
                            // let row = Row::new(vec![
                            //     Cell::default(),
                            //     Cell::from(song.track_number.to_string())
                            //         .style(Style::default().bg(NUMBER)),
                            //     Cell::from(song.title.as_str()).style(Style::default().bg(TITLE)),
                            //     Cell::from(song.album.as_str()).style(Style::default().bg(ALBUM)),
                            //     Cell::from(song.artist.as_str()).style(Style::default().bg(ARTIST)),
                            // ])
                            // .style(Style::default().fg(Color::Black));
                            // items.remove(ui_index);
                            // items.insert(ui_index, row);
                        }
                    }
                }
            }
        }

        let con = [
            Constraint::Length(2),
            Constraint::Percentage(queue.constraint[0]),
            Constraint::Percentage(queue.constraint[1]),
            Constraint::Percentage(queue.constraint[2]),
            Constraint::Percentage(queue.constraint[3]),
        ];
        let block = block(
            None,
            Borders::LEFT | Borders::RIGHT | Borders::BOTTOM,
            BorderType::Rounded,
        );
        let header = header!["", "#", "Title", "Album", "Artist"];
        let table = table(Some(header), Some(block), &con, rows, None, style());
        table.draw(area[1], buf, &mut table_state(ui_index));

        let row_bounds = Some(table.get_row_bounds(ui_index, table.get_row_height(area[1])));
        row_bounds

        // let t = Table::new(&items)
        //     .header(
        //         Row::new(["", "#", "Title", "Album", "Artist"])
        //             .style(
        //                 Style::default()
        //                     .fg(Color::White)
        //                     .add_modifier(Modifier::BOLD),
        //             )
        //             .bottom_margin(1),
        //     )
        //     .block(
        //         Block::default()
        //             .borders()
        //             .border_type(BorderType::Rounded),
        //     )
        //     .widths(&con);
    };

    // if log::last_message().is_none() {
    //     //Seeker
    //     let area = chunks[2];
    //     if player.songs.is_empty() {
    //         return f.render_widget(
    //             Block::default()
    //                 .border_type(BorderType::Rounded)
    //                 .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT),
    //             area,
    //         );
    //     }

    //     let elapsed = player.elapsed().as_secs_f64();
    //     let duration = player.duration().as_secs_f64();

    //     let seeker = format!(
    //         "{:02}:{:02}/{:02}:{:02}",
    //         (elapsed / 60.0).floor(),
    //         (elapsed % 60.0) as u64,
    //         (duration / 60.0).floor(),
    //         (duration % 60.0) as u64,
    //     );

    //     let ratio = elapsed.floor() / duration;
    //     let ratio = if ratio.is_nan() {
    //         0.0
    //     } else {
    //         ratio.clamp(0.0, 1.0)
    //     };

    //     f.render_widget(
    //         Gauge::default()
    //             .block(
    //                 Block::default()
    //                     .border_type(BorderType::Rounded)
    //                     .borders(Borders::ALL),
    //             )
    //             .gauge_style(Style::default().fg(SEEKER))
    //             .ratio(ratio)
    //             .label(seeker),
    //         area,
    //     );
    // }

    // //Don't handle mouse input when the queue is empty.
    // if player.songs.is_empty() {
    //     return;
    // }

    // //Handle mouse input.
    // if let Some(event) = mouse_event {
    //     let (x, y) = (event.column, event.row);
    //     let header_height = 5;

    //     let size = f.size();

    //     //Mouse support for the seek bar.
    //     if (size.height - 3 == y || size.height - 2 == y || size.height - 1 == y)
    //         && size.height > 15
    //     {
    //         let ratio = x as f32 / size.width as f32;
    //         let duration = player.duration().as_secs_f32();
    //         player.seek(duration * ratio);
    //     }

    //     //Mouse support for the queue.
    //     if let Some((start, _)) = row_bounds {
    //         //Check if you clicked on the header.
    //         if y >= header_height {
    //             let index = (y - header_height) as usize + start;

    //             //Make sure you didn't click on the seek bar
    //             //and that the song index exists.
    //             if index < player.songs.len()
    //                 && ((size.height < 15 && y < size.height.saturating_sub(1))
    //                     || y < size.height.saturating_sub(3))
    //             {
    //                 queue.ui.select(Some(index));
    //             }
    //         }
    //     }
    // }
}

// pub fn draw(queue: &mut Queue, f: &mut Frame, _: Rect, mouse_event: Option<MouseEvent>) {
// }

// pub fn constraint(queue: &mut Queue, row: usize, shift: bool) {
//     if shift && queue.constraint[row] != 0 {
//         //Move row back.
//         queue.constraint[row + 1] += 1;
//         queue.constraint[row] = queue.constraint[row].saturating_sub(1);
//     } else if queue.constraint[row + 1] != 0 {
//         //Move row forward.
//         queue.constraint[row] += 1;
//         queue.constraint[row + 1] = queue.constraint[row + 1].saturating_sub(1);
//     }

//     debug_assert!(
//         queue.constraint.iter().sum::<u16>() == 100,
//         "Constraint went out of bounds: {:?}",
//         queue.constraint
//     );
// }
