use crate::conn;

pub fn total_songs() -> usize {
    let conn = conn();
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM song").unwrap();
    stmt.query_row([], |row| row.get(0)).unwrap()
}
