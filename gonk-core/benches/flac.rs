use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gonk_core::{read_metadata, Song};
use winwalk::DirEntry;

fn custom(files: &[DirEntry]) -> Vec<Result<Song, String>> {
    files
        .iter()
        .map(|file| match read_metadata(&file.path) {
            Ok(metadata) => {
                let track_number = metadata
                    .get("TRACKNUMBER")
                    .unwrap_or(&String::from("1"))
                    .parse()
                    .unwrap_or(1);

                let disc_number = metadata
                    .get("DISCNUMBER")
                    .unwrap_or(&String::from("1"))
                    .parse()
                    .unwrap_or(1);

                let mut gain = 0.0;
                if let Some(db) = metadata.get("REPLAYGAIN_TRACK_GAIN") {
                    let g = db.replace(" dB", "");
                    if let Ok(db) = g.parse::<f32>() {
                        gain = 10.0f32.powf(db / 20.0);
                    }
                }

                let artist = match metadata.get("ALBUMARTIST") {
                    Some(artist) => artist.as_str(),
                    None => match metadata.get("ARTIST") {
                        Some(artist) => artist.as_str(),
                        None => "Unknown Artist",
                    },
                };

                let album = match metadata.get("ALBUM") {
                    Some(album) => album.as_str(),
                    None => "Unknown Album",
                };

                let title = match metadata.get("TITLE") {
                    Some(title) => title.as_str(),
                    None => "Unknown Title",
                };

                Ok(Song {
                    title: title.to_string(),
                    album: album.to_string(),
                    artist: artist.to_string(),
                    disc_number,
                    track_number,
                    path: file
                        .path
                        .to_str()
                        .ok_or("Invalid UTF-8 in path.")?
                        .to_string(),
                    gain,
                })
            }
            Err(err) => Err(format!("Error: ({err}) @ {}", file.path.to_string_lossy())),
        })
        .collect()
}

fn symphonia(files: &[DirEntry]) -> Vec<Result<Song, String>> {
    use std::fs::File;
    use symphonia::{
        core::{formats::FormatOptions, io::*, meta::*, probe::Hint},
        default::get_probe,
    };
    files
        .iter()
        .map(|entry| {
            let file = match File::open(&entry.path) {
                Ok(file) => file,
                Err(err) => {
                    return Err(format!("Error: ({err}) @ {}", entry.path.to_string_lossy()))
                }
            };

            let mss = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());

            let mut probe = match get_probe().format(
                &Hint::new(),
                mss,
                &FormatOptions::default(),
                &MetadataOptions {
                    limit_visual_bytes: Limit::Maximum(1),
                    ..Default::default()
                },
            ) {
                Ok(probe) => probe,
                Err(err) => {
                    return Err(format!("Error: ({err}) @ {}", entry.path.to_string_lossy()))?
                }
            };

            let mut title = String::from("Unknown Title");
            let mut album = String::from("Unknown Album");
            let mut artist = String::from("Unknown Artist");
            let mut track_number = 1;
            let mut disc_number = 1;
            let mut gain = 0.0;

            let mut metadata_revision = probe.format.metadata();
            let mut metadata = probe.metadata.get();
            let mut m = None;

            if let Some(metadata) = metadata_revision.skip_to_latest() {
                m = Some(metadata);
            };

            if let Some(metadata) = &mut metadata {
                if let Some(metadata) = metadata.skip_to_latest() {
                    m = Some(metadata)
                };
            }

            if let Some(metadata) = m {
                for tag in metadata.tags() {
                    if let Some(std_key) = tag.std_key {
                        match std_key {
                            StandardTagKey::AlbumArtist => artist = tag.value.to_string(),
                            StandardTagKey::Artist if artist == "Unknown Artist" => {
                                artist = tag.value.to_string()
                            }
                            StandardTagKey::Album => album = tag.value.to_string(),
                            StandardTagKey::TrackTitle => title = tag.value.to_string(),
                            StandardTagKey::TrackNumber => {
                                let num = tag.value.to_string();
                                if let Some((num, _)) = num.split_once('/') {
                                    track_number = num.parse().unwrap_or(1);
                                } else {
                                    track_number = num.parse().unwrap_or(1);
                                }
                            }
                            StandardTagKey::DiscNumber => {
                                let num = tag.value.to_string();
                                if let Some((num, _)) = num.split_once('/') {
                                    disc_number = num.parse().unwrap_or(1);
                                } else {
                                    disc_number = num.parse().unwrap_or(1);
                                }
                            }
                            StandardTagKey::ReplayGainTrackGain => {
                                let tag = tag.value.to_string();
                                let (_, value) =
                                    tag.split_once(' ').ok_or("Invalid replay gain.")?;
                                let db = value.parse().unwrap_or(0.0);
                                gain = 10.0f32.powf(db / 20.0);
                            }
                            _ => (),
                        }
                    }
                }
            }

            Ok(Song {
                title,
                album,
                artist,
                disc_number,
                track_number,
                path: entry
                    .path
                    .to_str()
                    .ok_or("Invalid UTF-8 in path.")?
                    .to_string(),
                gain,
            })
        })
        .collect()
}

const PATH: &str = "D:\\OneDrive\\Music";

fn flac(c: &mut Criterion) {
    let mut group = c.benchmark_group("flac");
    group.sample_size(10);

    let paths: Vec<winwalk::DirEntry> = winwalk::walkdir(PATH, 0)
        .into_iter()
        .flatten()
        .filter(|entry| match entry.path.extension() {
            Some(ex) => {
                matches!(ex.to_str(), Some("flac"))
            }
            None => false,
        })
        .collect();

    group.bench_function("custom", |b| {
        b.iter(|| {
            custom(black_box(&paths));
        });
    });

    group.bench_function("symphonia", |b| {
        b.iter(|| {
            symphonia(black_box(&paths));
        });
    });

    group.finish();
}

criterion_group!(benches, flac);
criterion_main!(benches);
