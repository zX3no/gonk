mod database;
use std::time::Instant;

use database::Database;

fn main() {
    let database = Database::new(r"D:\OneDrive\Music");
}
