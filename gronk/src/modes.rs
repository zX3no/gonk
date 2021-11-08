#[derive(Debug, Clone)]
pub struct Mode {
    pub current: UiMode,
    prev: UiMode,
}

impl Mode {
    pub fn new() -> Self {
        Self {
            current: UiMode::Browser,
            prev: UiMode::Queue,
        }
    }
    pub fn toggle(&mut self) {
        if self.current == UiMode::Browser {
            self.current = UiMode::Queue;
            self.prev = UiMode::Browser;
        } else {
            self.current = UiMode::Browser;
            self.prev = UiMode::Queue;
        }
    }
    pub fn update(&mut self, current: UiMode) {
        self.prev = self.current.clone();
        self.current = current;
    }
    pub fn flip(&mut self) {
        let (c, p) = (self.current.clone(), self.prev.clone());
        self.current = p;
        self.prev = c;
    }
}

impl PartialEq<UiMode> for Mode {
    fn eq(&self, other: &UiMode) -> bool {
        &self.current == other
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UiMode {
    Browser,
    Queue,
    Search,
}

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
