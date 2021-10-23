use gronk_indexer::database::Song;
use gronk_player::player::Player;
use tui::widgets::{ListItem, ListState};

pub struct Queue {
    player: Player,
    state: ListState,
}
impl Queue {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            player: Player::new(),
            state,
        }
    }
    pub fn get_list_items(&self) -> Vec<ListItem<'static>> {
        self.get_queue()
            .iter()
            .map(|song| ListItem::new(song.clone()))
            .collect()
    }
    pub fn get_state(&mut self) -> &mut ListState {
        //todo store the state in the queue
        &mut self.state
    }
    pub fn add(&mut self, songs: Vec<Song>) {
        self.player.add(songs);
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
    pub fn pause(&self) {
        self.player.toggle_playback();
    }
    pub fn volume_down(&self) {
        self.player.volume(-0.005);
    }
    pub fn volume_up(&self) {
        self.player.volume(0.005);
    }
}
