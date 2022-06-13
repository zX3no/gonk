use miniserde::{json, Deserialize, Serialize};
use static_init::dynamic;
use std::fs;

#[derive(Serialize, Deserialize, Debug)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Gray,
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    White,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Colors {
    pub number: Color,
    pub name: Color,
    pub album: Color,
    pub artist: Color,
    pub seeker: Color,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Json {
    paths: Vec<String>,
    output_device: String,
    volume: u8,
    color: Colors,
}
/*
[config]
paths = ["D:/OneDrive/Music"]
output_device = "OUT 1-4 (BEHRINGER UMC 404HD 192k)"
volume = 55

[colors]
number = "Green"
name = "Cyan"
album = "Magenta"
artist = "Blue"
seeker = "White"
*/

pub fn write_config() {
    let j = json::to_string(&*JSON);
    let p = jsonxf::pretty_print(&j).unwrap();
    fs::write("gonk.json", p);
}

pub fn read_config() -> miniserde::Result<Json> {
    let json = if let Ok(file) = fs::read_to_string("gonk.json") {
        json::from_str(&file)?
    } else {
        let color = Colors {
            number: Color::Green,
            name: Color::Cyan,
            album: Color::Magenta,
            artist: Color::Blue,
            seeker: Color::White,
        };
        Json {
            paths: Vec::new(),
            output_device: String::new(),
            volume: 15,
            color,
        }
    };
    Ok(json)
}

#[dynamic]
static JSON: Json = read_config().unwrap();
