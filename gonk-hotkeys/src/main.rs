use gonk_database::HotkeyConfig;
use gonk_server::Client;
use win_hotkey::{keys, modifiers, Listener};

#[derive(Debug, Clone)]
enum HotkeyEvent {
    PlayPause,
    Next,
    Prev,
    VolUp,
    VolDown,
}

fn main() {
    let config = HotkeyConfig::new();
    let mut hk = Listener::<HotkeyEvent>::new();
    let mut client = Client::new();

    hk.register_hotkey(
        config.volume_up.modifiers(),
        config.volume_up.key(),
        HotkeyEvent::VolUp,
    );
    hk.register_hotkey(
        config.volume_down.modifiers(),
        config.volume_down.key(),
        HotkeyEvent::VolDown,
    );
    hk.register_hotkey(
        config.previous.modifiers(),
        config.previous.key(),
        HotkeyEvent::Prev,
    );
    hk.register_hotkey(
        config.next.modifiers(),
        config.next.key(),
        HotkeyEvent::Next,
    );
    hk.register_hotkey(modifiers::SHIFT, keys::ESCAPE, HotkeyEvent::PlayPause);
    loop {
        if let Some(event) = hk.listen() {
            match event {
                HotkeyEvent::VolUp => {
                    client.volume_up();
                }
                HotkeyEvent::VolDown => {
                    client.volume_down();
                }
                HotkeyEvent::PlayPause => client.toggle_playback(),
                HotkeyEvent::Prev => client.prev(),
                HotkeyEvent::Next => client.next(),
            }
        }
    }
}
