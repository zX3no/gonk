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
        self.get_queue()
            .iter()
            .map(|song| ListItem::new(song.clone()))
            .collect()
    }
    pub fn add(&mut self, song: Song) {
        self.player.add(song);
    }
    pub fn get_queue(&self) -> Vec<String> {
        self.player.get_queue()
    }
    pub fn get_seeker(&self) -> String {
        self.player.get_seeker()
    }
    pub fn next(&self) {
        self.player.next();
    }
    pub fn prev(&self) {
        self.player.previous();
    }
    pub fn clear(&self) {
        self.player.clear_queue();
        self.player.stop()
    }
}
