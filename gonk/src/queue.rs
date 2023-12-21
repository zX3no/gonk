use crate::{ALBUM, ARTIST,  NUMBER, SEEKER, TITLE};
use gonk_core::{log, Index, Song};
use winter::*;

pub struct Queue {
    pub constraint: [u16; 4],
    pub index: Option<usize>,
}

impl Queue {
    pub fn new(index: usize) -> Self {
        Self {
            constraint: [6, 37, 31, 26],
            index: Some(index),
        }
    }
}

#[cfg(test)]
mod tests {
    use gonk_core::*;

    #[test]
    fn test() {
        //index is zero indexed, length is not.
        assert_eq!(up(10, 1, 1), 0);
        assert_eq!(up(8, 7, 5), 2);

        assert_eq!(up(8, 0, 5), 4);

        assert_eq!(down(8, 7, 5), 4);

        assert_eq!(down(8, 1, 5), 6);
    }
}

pub fn up(queue: &mut Queue, songs: &mut Index<Song>, amount: usize) {
    if let Some(index) = &mut queue.index {
        *index = gonk_core::up(songs.len(), *index, amount);
    }
}

pub fn down(queue: &mut Queue, songs: &Index<Song>, amount: usize) {
    if let Some(index) = &mut queue.index {
        *index = gonk_core::down(songs.len(), *index, amount);
    }
}

pub fn draw(
    queue: &mut Queue,
    viewport: winter::Rect,
    buf: &mut winter::Buffer,
    mouse: Option<(u16, u16)>,
    songs: &mut Index<Song>,
    mute: bool,
) {
    let fill = viewport.height.saturating_sub(3 + 3);
    let area = layout(
        viewport,
        Direction::Vertical,
        &[
            Constraint::Length(3),
            Constraint::Length(fill),
            Constraint::Length(3),
            // Constraint::Length(3),
        ],
    );

    //Header
    block()
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
        .title(if songs.is_empty() {
            "Stopped"
        } else if gonk_player::is_paused() {
            "Paused"
        } else {
            "Playing"
        })
        .title_margin(1)
        .draw(area[0], buf);

    if !songs.is_empty() {
        //Title
        if let Some(song) = songs.selected() {
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

            let top = lines![
                text!("─│ {}", pad_front),
                artist.fg(ARTIST),
                " ─ ",
                album.fg(ALBUM),
                text!("{} │─", pad_back)
            ];
            top.align(Center).draw(area[0], buf);

            let bottom = lines!(title.fg(TITLE));
            let mut area = area[0];
            if area.height > 1 {
                area.y += 1;
                bottom.align(Center).draw(area, buf)
            }
        }
    }

    let volume: Lines<'_> = if mute {
        "Mute─╮".into()
    } else {
        text!("Vol: {}%─╮", gonk_player::get_volume()).into()
    };
    volume.align(Right).draw(area[0], buf);

    let mut row_bounds = None;

    //Body
    if songs.is_empty() {
        let block = if log::last_message().is_some() {
            block().borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
        } else {
            block().borders(Borders::LEFT | Borders::RIGHT)
        };
        block.draw(area[1], buf);
    } else {
        let (songs, player_index, ui_index) = (&songs, songs.index(), queue.index);

        let mut rows: Vec<Row> = songs
            .iter()
            .map(|song| {
                row![
                    text!(),
                    song.track_number.to_string().fg(NUMBER),
                    song.title.as_str().fg(TITLE),
                    song.album.as_str().fg(ALBUM),
                    song.artist.as_str().fg(ARTIST)
                ]
            })
            .collect();

        if let Some(player_index) = player_index {
            if let Some(song) = songs.get(player_index) {
                if let Some(ui_index) = ui_index {
                    //Currently playing song
                    if ui_index == player_index {
                        rows[player_index] = row![
                            ">>".fg(White).dim().bold(),
                            song.track_number.to_string().bg(NUMBER).fg(Black).dim(),
                            song.title.as_str().bg(TITLE).fg(Black).dim(),
                            song.album.as_str().bg(ALBUM).fg(Black).dim(),
                            song.artist.as_str().bg(ARTIST).fg(Black).dim()
                        ];
                    } else {
                        rows[player_index] = row![
                            ">>".fg(White).dim().bold(),
                            song.track_number.to_string().fg(NUMBER),
                            song.title.as_str().fg(TITLE),
                            song.album.as_str().fg(ALBUM),
                            song.artist.as_str().fg(ARTIST)
                        ]
                    };

                    //Current selection
                    if ui_index != player_index {
                        if let Some(song) = songs.get(ui_index) {
                            //TODO: Something is not working here.
                            rows[ui_index] = row![
                                text!(),
                                song.track_number.to_string().fg(Black).bg(NUMBER).dim(),
                                song.title.as_str().fg(Black).bg(TITLE).dim(),
                                song.album.as_str().fg(Black).bg(ALBUM).dim(),
                                song.artist.as_str().fg(Black).bg(ARTIST).dim()
                            ];
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
        let block = block().borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM);
        //TODO: Header style.
        let header = header![
            text!(),
            "#".bold(),
            "Title".bold(),
            "Album".bold(),
            "Artist".bold()
        ];
        let table = table(rows, &con).header(header).block(block).spacing(1);
        table.draw(area[1], buf, ui_index);
        row_bounds = Some(table.get_row_bounds(ui_index, table.get_row_height(area[1])));
    };

    if log::last_message().is_none() {
        //Seeker
        if songs.is_empty() {
            return block()
                .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
                .draw(area[2], buf);
        }

        let elapsed = gonk_player::elapsed().as_secs_f32();
        let duration = gonk_player::duration().as_secs_f32();

        let seeker = format!(
            "{:02}:{:02}/{:02}:{:02}",
            (elapsed / 60.0).floor(),
            (elapsed % 60.0) as u64,
            (duration / 60.0).floor(),
            (duration % 60.0) as u64,
        );

        let ratio = elapsed.floor() / duration;
        let ratio = if ratio.is_nan() {
            0.0
        } else {
            ratio.clamp(0.0, 1.0)
        };

        let block = block();
        guage(Some(block), ratio, seeker.into(), bg(SEEKER), style()).draw(area[2], buf);
    }

    //Don't handle mouse input when the queue is empty.
    if songs.is_empty() {
        return;
    }

    //Handle mouse input.
    if let Some((x, y)) = mouse {
        let header_height = 5;
        let size = viewport;

        //Mouse support for the seek bar.
        if (size.height - 3 == y || size.height - 2 == y || size.height - 1 == y)
            && size.height > 15
        {
            let ratio = x as f32 / size.width as f32;
            let duration = gonk_player::duration().as_secs_f32();
            gonk_player::seek(duration * ratio);
        }

        //Mouse support for the queue.
        if let Some((start, _)) = row_bounds {
            //Check if you clicked on the header.
            if y >= header_height {
                let index = (y - header_height) as usize + start;

                //Make sure you didn't click on the seek bar
                //and that the song index exists.
                if index < songs.len()
                    && ((size.height < 15 && y < size.height.saturating_sub(1))
                        || y < size.height.saturating_sub(3))
                {
                    queue.index = Some(index);
                }
            }
        }
    }
}

pub fn constraint(queue: &mut Queue, row: usize, shift: bool) {
    if shift && queue.constraint[row] != 0 {
        //Move row back.
        queue.constraint[row + 1] += 1;
        queue.constraint[row] = queue.constraint[row].saturating_sub(1);
    } else if queue.constraint[row + 1] != 0 {
        //Move row forward.
        queue.constraint[row] += 1;
        queue.constraint[row + 1] = queue.constraint[row + 1].saturating_sub(1);
    }

    debug_assert!(
        queue.constraint.iter().sum::<u16>() == 100,
        "Constraint went out of bounds: {:?}",
        queue.constraint
    );
}
