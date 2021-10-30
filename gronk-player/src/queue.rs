use std::path::PathBuf;

use gronk_indexer::database::Song;

#[derive(Debug, Clone)]
pub struct Queue {
    pub songs: Vec<Song>,
    pub now_playing: Option<Song>,
    pub index: Option<usize>,
}
impl Queue {
    pub fn new() -> Self {
        Self {
            songs: Vec::new(),
            now_playing: None,
            index: None,
        }
    }
    pub fn test() -> Self {
        Self {
            songs: vec![Song::from(PathBuf::from("music/2.flac"))],
            now_playing: None,
            index: None,
        }
    }
    pub fn next_song(&mut self) -> Option<PathBuf> {
        let (now_playing, index, songs) = (&mut self.now_playing, &mut self.index, &self.songs);

        if let Some(song) = now_playing {
            if let Some(index) = index {
                if let Some(next_song) = songs.get(*index + 1) {
                    *song = next_song.clone();
                    *index += 1;
                } else if let Some(next_song) = songs.first() {
                    *song = next_song.clone();
                    *index = 0;
                }
            }
            let song = song.clone();
            Some(song.path)
        } else {
            None
        }
    }
    pub fn prev_song(&mut self) -> Option<PathBuf> {
        let (now_playing, index, queue) = (&mut self.now_playing, &mut self.index, &self.songs);

        if let Some(song) = now_playing {
            if let Some(index) = index {
                if *index != 0 {
                    if let Some(next_song) = queue.get(*index - 1) {
                        *song = next_song.clone();
                        *index -= 1;
                    }
                } else if let Some(next_song) = queue.last() {
                    *song = next_song.clone();
                    *index = queue.len() - 1;
                }
            }
            let song = song.clone();
            Some(song.path)
        } else {
            None
        }
    }
}
