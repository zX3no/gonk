mod database;
use std::time::Instant;

use database::Database;

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
    let database = Database::new();

    let t = T::start();
    database.find_album("Clear Tamei");
    t.end();
}
