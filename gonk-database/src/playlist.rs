//FIXME: All queries are out of date and broken.
use crate::*;

pub fn add(name: &str, ids: &[usize]) {
    let name = name.replace('\'', r"''");
    let queries: Vec<String> = ids
        .iter()
        .map(|id| {
            format!(
                "INSERT INTO playlist (song_id, name) VALUES ('{}', '{}');",
                id, name
            )
        })
        .collect();
    let query = format!("BEGIN;\n{}\nCOMMIT;", queries.join("\n"));

    conn().execute_batch(&query).unwrap();
}

pub fn get_names() -> Vec<String> {
    // let conn = conn();
    // let mut stmt = conn.prepare("SELECT DISTINCT name FROM playlist").unwrap();

    // stmt.query_map([], |row| row.get(0))
    //     .unwrap()
    //     .flatten()
    //     .collect()
    Vec::new()
}

pub fn get(playlist_name: &str) -> (Vec<usize>, Vec<usize>) {
    let conn = conn();
    let mut stmt = conn
        .prepare("SELECT rowid, song_id FROM playlist WHERE name = ?")
        .unwrap();

    let ids: Vec<_> = stmt
        .query_map([playlist_name], |row| {
            Ok((row.get(0).unwrap(), row.get(1).unwrap()))
        })
        .unwrap()
        .flatten()
        .collect();

    let row_ids: Vec<_> = ids.iter().map(|id| id.0).collect();
    let song_ids: Vec<_> = ids.iter().map(|id| id.1).collect();
    (row_ids, song_ids)
}

pub fn remove_id(id: usize) {
    conn()
        .execute("DELETE FROM playlist WHERE rowid = ?", [id])
        .unwrap();
}
pub fn remove(name: &str) {
    conn()
        .execute("DELETE FROM playlist WHERE name = ?", [name])
        .unwrap();
}
