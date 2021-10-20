use gronk_indexer::database::Song;
use gronk_player::player::Player;
use tui::widgets::ListItem;

pub struct Queue {
    player: Player,
}
impl Queue {
    pub fn new() -> Self {
        Self {
            player: Player::new(),
        }
    }
    pub fn get_list_items(&self) -> Vec<ListItem<'static>> {
        Vec::new()
    }
    pub fn add(&mut self, song: Song) {
        self.player.add(song.clone());
    }
    pub fn get_queue(&self) -> Vec<String> {
        Vec::new()
    }
    pub fn get_seeker(&self) -> String {
        self.player.get_seeker()
    }
}
