use gonk_core::{vdb::Database, Album};
use gonk_core::{Index, Song};
use winter::*;

#[derive(PartialEq, Eq)]
pub enum Mode {
    Artist,
    Album,
    Song,
}

pub struct Browser {
    artists: Index<String>,
    albums: Index<Album>,
    ///Title, (disc, track)
    songs: Index<(String, (u8, u8))>,
    pub mode: Mode,
}

impl Browser {
    pub fn new(db: &Database) -> Self {
        let artists = Index::new(db.artists().into_iter().cloned().collect(), Some(0));
        let mut albums: Index<Album> = Index::default();
        let mut songs = Index::default();

        if let Some(artist) = artists.selected() {
            albums = Index::from(db.albums_by_artist(artist));
            if let Some(album) = albums.selected() {
                songs = Index::from(
                    album
                        .songs
                        .iter()
                        .map(|song| {
                            (
                                format!("{}. {}", song.track_number, song.title),
                                (song.disc_number, song.track_number),
                            )
                        })
                        .collect::<Vec<(String, (u8, u8))>>(),
                );
            }
        }

        Self {
            artists,
            albums,
            songs,
            mode: Mode::Artist,
        }
    }
}

pub fn up(browser: &mut Browser, db: &Database) {
    match browser.mode {
        Mode::Artist => browser.artists.up(),
        Mode::Album => browser.albums.up(),
        Mode::Song => browser.songs.up(),
    }
    update(browser, db);
}

pub fn down(browser: &mut Browser, db: &Database) {
    match browser.mode {
        Mode::Artist => browser.artists.down(),
        Mode::Album => browser.albums.down(),
        Mode::Song => browser.songs.down(),
    }
    update(browser, db);
}

pub fn left(browser: &mut Browser) {
    match browser.mode {
        Mode::Artist => (),
        Mode::Album => browser.mode = Mode::Artist,
        Mode::Song => browser.mode = Mode::Album,
    }
}

pub fn right(browser: &mut Browser) {
    match browser.mode {
        Mode::Artist => browser.mode = Mode::Album,
        Mode::Album => browser.mode = Mode::Song,
        Mode::Song => (),
    }
}

pub fn draw(
    browser: &mut Browser,
    area: winter::Rect,
    buf: &mut winter::Buffer,
    mouse: Option<(u16, u16)>,
) {
    let size = area.width / 3;
    let rem = area.width % 3;

    let chunks = layout!(
        area,
        Direction::Horizontal,
        Constraint::Length(size),
        Constraint::Length(size),
        Constraint::Length(size + rem)
    );

    if let Some((x, y)) = mouse {
        let rect = Rect {
            x,
            y,
            ..Default::default()
        };
        if rect.intersects(chunks[2]) {
            browser.mode = Mode::Song;
        } else if rect.intersects(chunks[1]) {
            browser.mode = Mode::Album;
        } else if rect.intersects(chunks[0]) {
            browser.mode = Mode::Artist;
        }
    }

    let artists: Vec<_> = browser.artists.iter().map(|a| lines!(a)).collect();
    let albums: Vec<_> = browser.albums.iter().map(|a| lines!(&a.title)).collect();
    let songs: Vec<_> = browser.songs.iter().map(|(s, _)| lines!(s)).collect();

    fn list<'a>(title: &'static str, items: Vec<Lines<'a>>, use_symbol: bool) -> List<'a> {
        let block = block(Some(title.bold()), Borders::ALL, Rounded).margin(1);
        let symbol = if use_symbol { ">" } else { " " };
        winter::list(Some(block), items, Some(symbol), None)
    }

    let artists = list("Aritst", artists, browser.mode == Mode::Artist);
    let albums = list("Album", albums, browser.mode == Mode::Album);
    let songs = list("Song", songs, browser.mode == Mode::Song);

    //TODO: Re-work list_state and index.
    artists.draw(chunks[0], buf, browser.artists.index());
    albums.draw(chunks[1], buf, browser.albums.index());
    songs.draw(chunks[2], buf, browser.songs.index());
}

pub fn refresh(browser: &mut Browser, db: &Database) {
    browser.mode = Mode::Artist;

    browser.artists = Index::new(db.artists().into_iter().cloned().collect(), Some(0));
    browser.albums = Index::default();
    browser.songs = Index::default();

    update_albums(browser, db);
}

pub fn update(browser: &mut Browser, db: &Database) {
    match browser.mode {
        Mode::Artist => update_albums(browser, db),
        Mode::Album => update_songs(browser, db),
        Mode::Song => (),
    }
}

pub fn update_albums(browser: &mut Browser, db: &Database) {
    //Update the album based on artist selection
    if let Some(artist) = browser.artists.selected() {
        browser.albums = Index::from(db.albums_by_artist(artist));
        update_songs(browser, db);
    }
}

pub fn update_songs(browser: &mut Browser, db: &Database) {
    if let Some(artist) = browser.artists.selected() {
        if let Some(album) = browser.albums.selected() {
            let songs: Vec<(String, (u8, u8))> = db
                .album(artist, &album.title)
                .songs
                .iter()
                .map(|song| {
                    (
                        format!("{}. {}", song.track_number, song.title),
                        (song.disc_number, song.track_number),
                    )
                })
                .collect();
            browser.songs = Index::from(songs);
        }
    }
}

pub fn get_selected(browser: &Browser, db: &Database) -> Vec<Song> {
    if let Some(artist) = browser.artists.selected() {
        if let Some(album) = browser.albums.selected() {
            if let Some((_, (disc, number))) = browser.songs.selected() {
                return match browser.mode {
                    Mode::Artist => db
                        .albums_by_artist(artist)
                        .iter()
                        .flat_map(|album| album.songs.iter().map(|song| song.clone().clone()))
                        .collect(),
                    Mode::Album => db.album(artist, &album.title).songs.to_vec(),
                    Mode::Song => {
                        vec![db.song(artist, &album.title, *disc, *number).clone()]
                    }
                };
            }
        }
    }
    todo!()
}
