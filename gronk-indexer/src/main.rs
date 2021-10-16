mod database;
use std::{fs::File, io::Write, path::PathBuf, time::Instant};

use database::Database;
use hashbrown::HashMap;

use crate::database::{Album, Artist, Song};
struct T {
    now: Instant,
}
impl T {
    pub fn start() -> Self {
        T {
            now: Instant::now(),
        }
    }
    pub fn end(&self) {
        println!("{:?}", self.now.elapsed());
    }
}

fn main() {
    // let database = Database::create(r"D:\OneDrive\Music\");
    // database.save();
    let database = Database::read();
    let t = T::start();
    // database.find_artist("Iglooghost");
    database.find_song("Talk Meaning");
    t.end();
}
