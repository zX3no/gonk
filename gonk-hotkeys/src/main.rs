use gonk_core::HotkeyConfig;
use gonk_server::Client;
use win_hotkey::Listener;

#[derive(Debug)]
enum Event {
    PlayPause,
    Next,
    Prev,
    VolUp,
    VolDown,
    Quit,
}

fn main() {
    let config = HotkeyConfig::new();
    let mut hk = Listener::<Event>::new();
    let mut client = Client::new();

    hk.register_hotkey(
        config.volume_up.modifiers(),
        config.volume_up.key(),
        Event::VolUp,
    );
    hk.register_hotkey(
        config.volume_down.modifiers(),
        config.volume_down.key(),
        Event::VolDown,
    );
    hk.register_hotkey(
        config.previous.modifiers(),
        config.previous.key(),
        Event::Prev,
    );
    hk.register_hotkey(config.next.modifiers(), config.next.key(), Event::Next);
    hk.register_hotkey(
        config.play_pause.modifiers(),
        config.play_pause.key(),
        Event::PlayPause,
    );
    hk.register_hotkey(config.quit.modifiers(), config.quit.key(), Event::Quit);

    loop {
        if let Some(event) = hk.listen() {
            println!("Event::{:?}", event);
            match event {
                Event::VolUp => {
                    client.volume_up();
                }
                Event::VolDown => {
                    client.volume_down();
                }
                Event::PlayPause => client.toggle_playback(),
                Event::Prev => client.prev(),
                Event::Next => client.next(),
                Event::Quit => return,
            }
        }
    }
}
