use crate::index::Index;
use crossterm::event::KeyModifiers;
use gronk_types::Song;
use rodio::Player;
use std::time::Duration;

pub struct Queue {
    pub ui: Index<bool>,
    pub list: Index<Song>,
    pub constraint: [u16; 4],
    //TODO: is there a better way of doing this?
    pub clicked_pos: Option<(u16, u16)>,
    player: Player,
    skip_fix: bool,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            ui: Index::default(),
            list: Index::default(),
            constraint: [8, 42, 24, 26],
            clicked_pos: None,
            player: Player::new(),
            skip_fix: false,
        }
    }
    pub fn volume_up(&mut self) {
        self.player.change_volume(true);
    }
    pub fn volume_down(&mut self) {
        self.player.change_volume(false);
    }
    pub fn play(&self) {
        if self.player.is_paused() {
            self.player.toggle_playback();
        }
    }
    pub fn play_pause(&self) {
        self.player.toggle_playback();
    }
    pub fn seeker(&self) -> f64 {
        self.player.seeker()
    }
    pub fn update(&mut self) {
        if !self.player.is_done() && self.skip_fix {
            self.next();
        }
        if self.player.is_done() {
            self.skip_fix = true;
        } else {
            self.skip_fix = false;
        }
    }
    pub fn prev(&mut self) {
        self.list.up();
        self.play_selected();
    }
    pub fn next(&mut self) {
        self.list.down();
        self.play_selected();
    }
    pub fn clear(&mut self) {
        self.list = Index::default();
        self.player.stop();
    }
    pub fn up(&mut self) {
        self.ui.up_with_len(self.list.len());
    }
    pub fn down(&mut self) {
        self.ui.down_with_len(self.list.len());
    }
    pub fn add(&mut self, mut songs: Vec<Song>) {
        //clippy will tell you this is wrong :/
        if self.list.is_empty() {
            self.list.append(&mut songs);
            self.list.select(Some(0));
            self.ui.select(Some(0));
            self.play_selected();
        } else {
            self.list.append(&mut songs);
        }
    }
    pub fn select(&mut self) {
        //TODO: remove redundant if let
        if let Some(index) = self.ui.index() {
            self.list.select(Some(index));
            self.play_selected();
        }
    }
    pub fn delete_selected(&mut self) {
        if let Some(index) = self.ui.index() {
            self.list.remove(index);
            if let Some(playing) = self.list.index() {
                let len = self.list.len();

                if len == 0 {
                    self.clear();
                } else if playing == index && index == 0 {
                    self.list.select(Some(0));
                } else if playing == index && len == index {
                    self.list.select(Some(len - 1));
                } else if index < playing {
                    self.list.select(Some(playing - 1));
                }

                let end = self.list.len().saturating_sub(1);
                if index > end {
                    self.ui.select(Some(end));
                }
                if index < playing {
                    self.play_selected();
                }
            };
        }
    }
    pub fn play_selected(&mut self) {
        if let Some(item) = self.list.selected() {
            self.player.play(&item.path);
        } else {
            self.player.stop();
        }
    }
    pub fn seek_fw(&mut self) {
        self.play();
        self.player.seek_fw();
    }
    pub fn seek_bw(&mut self) {
        self.play();
        self.player.seek_bw();
    }
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
    pub fn is_playing(&self) -> bool {
        !self.player.is_paused()
    }
    pub fn duration(&self) -> Option<Duration> {
        if self.is_empty() {
            None
        } else {
            self.player.duration()
        }
    }
    pub fn elapsed(&self) -> Duration {
        self.player.elapsed()
    }
    pub fn seek_to(&self, new_time: f64) {
        self.player.seek_to(Duration::from_secs_f64(new_time));
    }
    pub fn get_playing(&self) -> Option<&Song> {
        self.list.selected()
    }
    pub fn get_volume_percent(&self) -> u16 {
        self.player.volume_percent()
    }
    pub fn move_constraint(&mut self, arg: char, modifier: KeyModifiers) {
        //1 is 48, '1' - 49 = 0
        let i = (arg as usize) - 49;
        if modifier == KeyModifiers::SHIFT && self.constraint[i] != 0 {
            self.constraint[i] = self.constraint[i].saturating_sub(1);
            self.constraint[i + 1] += 1;
        } else if self.constraint[i + 1] != 0 {
            self.constraint[i] += 1;
            self.constraint[i + 1] = self.constraint[i + 1].saturating_sub(1);
        }

        for n in &mut self.constraint {
            if *n > 100 {
                *n = 100;
            }
        }
        if self.constraint.iter().sum::<u16>() != 100 {
            panic!("{:?}", self.constraint);
        }
    }
}
