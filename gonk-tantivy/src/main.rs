use gonk_core::sqlite;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::Deserialize;
use std::fs::{self};
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index, IndexReader, ReloadPolicy};

#[derive(Deserialize)]
pub struct Song {
    name: String,
    album: String,
    artist: String,
    path: String,
    id: u64,
}

pub struct Database {
    schema: Schema,
    reader: IndexReader,
    index: Index,
}

impl Database {
    pub fn new() -> tantivy::Result<Self> {
        let mut schema_builder = Schema::builder();

        let numeric_options = NumericOptions::default().set_stored();

        schema_builder.add_text_field("name", TEXT | STORED);
        schema_builder.add_text_field("album", TEXT | STORED);
        schema_builder.add_text_field("artist", TEXT | STORED);
        schema_builder.add_text_field("path", STORED);
        schema_builder.add_u64_field("id", numeric_options);

        let schema = schema_builder.build();

        let path = Path::new("db");
        let index = if path.exists() {
            Index::open_in_dir(path)
        } else {
            fs::create_dir(path)?;
            Index::create_in_dir(&path, schema.clone())
        }?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()?;

        Ok(Self {
            schema,
            reader,
            index,
        })
    }
    pub fn add_songs(&self) -> tantivy::Result<()> {
        let songs = sqlite::get_all_songs();

        let schema = &self.schema;
        let mut index_writer = self.index.writer(50_000_000)?;

        let name = schema.get_field("name").unwrap();
        let album = schema.get_field("album").unwrap();
        let artist = schema.get_field("artist").unwrap();
        let id = schema.get_field("id").unwrap();

        songs.into_par_iter().for_each(|(i, song)| {
            index_writer
                .add_document(doc!(
                name => song.name,
                album => song.album,
                artist => song.artist,
                id => i as u64,
                ))
                .unwrap();
        });

        index_writer.commit()?;
        Ok(())
    }
    pub fn search(&self, query: &str) -> tantivy::Result<Vec<Song>> {
        let Self {
            reader,
            index,
            schema,
        } = self;

        let searcher = reader.searcher();
        let name = schema.get_field("name").unwrap();
        let album = schema.get_field("album").unwrap();
        let artist = schema.get_field("artist").unwrap();

        let query_parser = QueryParser::for_index(index, vec![name, album, artist]);
        let query = query_parser.parse_query(query)?;

        let top_docs = searcher.search(&query, &TopDocs::with_limit(50))?;

        let songs: Vec<Song> = top_docs
            .into_iter()
            .map(|(_, doc_address)| {
                let retrieved_doc = searcher.doc(doc_address).unwrap();
                let json = schema.to_json(&retrieved_doc);
                serde_json::from_str(&json).unwrap()
            })
            .collect();

        Ok(songs)
    }
}

fn main() -> tantivy::Result<()> {
    let db = Database::new()?;
    // db.add_songs()?;
    db.search("You will never know")?;

    Ok(())
}
