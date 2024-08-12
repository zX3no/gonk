use crate::JUMP_AMOUNT;
use std::sync::LazyLock;
use winter::*;

//TODO: Add scrolling to the help menu.
//TODO: Improve visability, it's hard to tell which option matches which command.
//TODO: Do I have a widget for adding lines?
pub static HELP: LazyLock<[Row; 32]> = LazyLock::new(|| {
    [
        row!["Move Up".fg(Cyan), "K / UP"],
        row!["Move Down".fg(Cyan), "J / Down"],
        row!["Move Left".fg(Cyan), "H / Left"],
        row!["Move Right".fg(Cyan), "L / Right"],
        row![text!("Move Up {}", JUMP_AMOUNT).fg(Cyan), "Shift + K / UP"],
        row![
            text!("Move Down {}", JUMP_AMOUNT).fg(Cyan),
            "Shift + J / Down"
        ],
        row!["Volume Up".fg(Green), "W"],
        row!["Volume Down".fg(Green), "S"],
        row!["Mute".fg(Green), "Z"],
        row!["Play/Pause".fg(Magenta), "Space"],
        row!["Previous".fg(Magenta), "A"],
        row!["Next".fg(Magenta), "D"],
        row!["Seek -10s".fg(Magenta), "Q"],
        row!["Seek 10s".fg(Magenta), "E"],
        row!["Queue".fg(Blue), "1"],
        row!["Browser".fg(Blue), "2"],
        row!["Playlists".fg(Blue), "3"],
        row!["Settings".fg(Blue), "4"],
        row!["Search".fg(Blue), "/"],
        row!["Exit Search".fg(Blue), "Escape | Tab"],
        row!["Select all".fg(Cyan), "Control + A"],
        row!["Add song to queue".fg(Cyan), "Enter"],
        row!["Add selection to playlist".fg(Cyan), "Shift + Enter"],
        row!["Move song margin".fg(Green), "F1 / Shift + F1"],
        row!["Move album margin".fg(Green), "F2 / Shift + F2"],
        row!["Move artist margin".fg(Green), "F3 / Shift + F3"],
        row!["Update database".fg(Yellow), "U"],
        row!["Quit player".fg(Yellow), "Ctrl + C"],
        row!["Clear queue".fg(Red), "C"],
        row!["Clear except playing".fg(Red), "Shift + C"],
        row!["Delete song/playlist".fg(Red), "X"],
        row!["Delete without confirmation".fg(Red), "Shift + X"],
    ]
});
