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

pub enum SearchMode {
    Search,
    Select,
}
impl SearchMode {
    pub fn next(&mut self) {
        match self {
            SearchMode::Search => *self = SearchMode::Select,
            SearchMode::Select => *self = SearchMode::Search,
        }
    }

    pub fn reset(&mut self) {
        *self = SearchMode::Search;
    }
}
