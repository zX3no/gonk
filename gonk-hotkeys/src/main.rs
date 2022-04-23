use gonk_core::{Config, GlobalHotkey, HOTKEY_CONFIG};
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
    let config: Config<GlobalHotkey> = Config::new(HOTKEY_CONFIG.as_path());
    let mut hk = Listener::<Event>::new();
    let mut client = Client::new();
    let data = config.data;

    hk.register_hotkey(
        data.volume_up.modifiers(),
        data.volume_up.key(),
        Event::VolUp,
    );
    hk.register_hotkey(
        data.volume_down.modifiers(),
        data.volume_down.key(),
        Event::VolDown,
    );
    hk.register_hotkey(data.previous.modifiers(), data.previous.key(), Event::Prev);
    hk.register_hotkey(data.next.modifiers(), data.next.key(), Event::Next);
    hk.register_hotkey(
        data.play_pause.modifiers(),
        data.play_pause.key(),
        Event::PlayPause,
    );
    hk.register_hotkey(data.quit.modifiers(), data.quit.key(), Event::Quit);

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
