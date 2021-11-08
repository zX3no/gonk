#[derive(Debug)]
pub enum BrowserMode {
    Artist,
    Album,
    Song,
}

impl BrowserMode {
    pub fn next(&mut self) {
        match self {
            BrowserMode::Artist => *self = BrowserMode::Album,
            BrowserMode::Album => *self = BrowserMode::Song,
            BrowserMode::Song => (),
        }
    }
    pub fn prev(&mut self) {
        match self {
            BrowserMode::Artist => (),
            BrowserMode::Album => *self = BrowserMode::Artist,
            BrowserMode::Song => *self = BrowserMode::Album,
        }
    }
}

#[derive(Debug)]
pub enum UiMode {
    Browser,
    Queue,
    Search,
}

impl UiMode {
    pub fn toggle(&mut self) {
        match self {
            UiMode::Browser => *self = UiMode::Queue,
            UiMode::Queue => *self = UiMode::Browser,
            UiMode::Search => *self = UiMode::Queue,
        }
    }
}
