use gonk_player::{Device, Wasapi};
use winter::*;

pub struct Settings {
    pub devices: Vec<Device>,
    pub index: Option<usize>,
    pub current_device: String,
}

impl Settings {
    pub fn new(wanted_device: &str) -> Self {
        let default = Wasapi::default_device();
        let devices = Wasapi::devices();
        let current_device = if devices.iter().any(|device| device.name == wanted_device) {
            wanted_device.to_string()
        } else {
            default.name.to_string()
        };

        Self {
            index: if devices.is_empty() { None } else { Some(0) },
            devices,
            current_device,
        }
    }
}

pub fn selected(settings: &mut Settings) -> Option<&str> {
    if let Some(index) = settings.index {
        if let Some(device) = settings.devices.get(index) {
            return Some(&device.name);
        }
    }
    None
}

pub fn up(settings: &mut Settings) {
    if settings.devices.is_empty() {
        return;
    }

    match settings.index {
        Some(0) => settings.index = Some(settings.devices.len() - 1),
        Some(n) => settings.index = Some(n - 1),
        None => (),
    }
}

pub fn down(settings: &mut Settings) {
    if settings.devices.is_empty() {
        return;
    }

    match settings.index {
        Some(n) if n + 1 < settings.devices.len() => settings.index = Some(n + 1),
        Some(_) => settings.index = Some(0),
        None => (),
    }
}

//TODO: I liked the old item menu bold selections instead of white background.
//It doesn't work on most terminals though :(
pub fn draw<'a>(settings: &'a mut Settings, area: winter::Rect, buf: &mut winter::Buffer) {
    let mut items = Vec::new();
    for device in &settings.devices {
        let item = if device.name == settings.current_device {
            lines([text!(">> ", dim()), text!(&device.name)], None, None)
        } else {
            lines([text!("   "), text!(&device.name)], None, None)
        };
        items.push(item);
    }

    if let Some(index) = settings.index {
        items[index].style = Some(fg(Black).bg(White));
    }

    let list = list(
        Some(block(
            Some("Output Device".into()),
            1,
            Borders::ALL,
            BorderType::Rounded,
            style(),
        )),
        &items,
        None,
        None,
    );

    list.draw(area, buf, &mut list_state(settings.index));
}
