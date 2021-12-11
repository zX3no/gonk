use gronk_search::{ItemType, SearchEngine, SearchItem};

fn main() {
    let query = "test";
    let items = vec![
        SearchItem::song("test", 0),
        SearchItem::song("test song", 0),
        SearchItem::song("test drive", 0),
        SearchItem::song("dont test me", 0),
        SearchItem::song("test ;lskjdf", 0),
        SearchItem::song("testing drive", 0),
        SearchItem::album("teaser", "sus"),
        SearchItem::song("why does this not work and why are you test me", 0),
        SearchItem::song("why does this not work and why are you testing me", 0),
        //don't show up
        SearchItem::song("fortnite", 0),
        SearchItem::song("among us", 0),
        SearchItem::artist("JPEGMAFIA"),
    ];

    let mut engine = SearchEngine::new();
    engine.insert_vec(items);

    for result in engine.search(query) {
        match result.item_type {
            ItemType::Song => (),
            ItemType::Album => (),
            ItemType::Artist => (),
        }
        println!("{}", result);
    }
}
