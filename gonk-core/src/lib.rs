pub use crate::{
    index::Index,
    song::Song,
    sqlite::Database,
    toml::{Bind, Colors, GlobalHotkey, Hotkey, Key, Modifier, Toml},
};
use static_init::dynamic;
use std::path::PathBuf;

mod index;
mod song;
mod toml;

pub mod sqlite;

#[dynamic]
pub static GONK_DIR: PathBuf = {
    let gonk = dirs::config_dir().unwrap().join("gonk");
    if !gonk.exists() {
        std::fs::create_dir_all(&gonk).unwrap();
    }
    gonk
};

#[dynamic]
pub static DB_DIR: PathBuf = GONK_DIR.join("gonk.db");

#[dynamic]
pub static TOML_DIR: PathBuf = GONK_DIR.join("gonk.toml");

#[cfg(test)]
mod tests {

    #[test]
    fn add_songs_to_database() {
        let db = crate::Database::default();
        db.add_dirs(&[String::from("D:\\Music")]);

        loop {
            if db.needs_update() {
                break;
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_keybindings() {
        use crate::{Bind, Key, Modifier};
        use global_hotkeys::{keys, modifiers};

        let b = Bind {
            key: Key::from("A"),
            modifiers: Some(vec![Modifier::ALT, Modifier::SHIFT]),
        };
        assert_eq!(Bind::new("A").key(), 'A' as u32);
        assert_eq!(Bind::new("TAB").key(), keys::TAB);
        assert_eq!(b.modifiers(), modifiers::ALT | modifiers::SHIFT);
    }
}
