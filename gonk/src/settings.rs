use gonk_player::*;
use winter::*;

pub struct Settings {
    pub devices: Vec<Device>,
    pub index: Option<usize>,
    pub current_device: String,
}

impl Settings {
    pub fn new(wanted_device: &str) -> Self {
        let default = default_device();
        let devices = devices();
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

pub fn selected(settings: &Settings) -> Option<&str> {
    if let Some(index) = settings.index {
        if let Some(device) = settings.devices.get(index) {
            return Some(&device.name);
        }
    }
    None
}

pub fn up(settings: &mut Settings, amount: usize) {
    if settings.devices.is_empty() {
        return;
    }
    let Some(index) = settings.index else { return };
    settings.index = Some(gonk_core::up(settings.devices.len(), index, amount));
}

pub fn down(settings: &mut Settings, amount: usize) {
    if settings.devices.is_empty() {
        return;
    }
    let Some(index) = settings.index else { return };
    settings.index = Some(gonk_core::down(settings.devices.len(), index, amount));
}

//TODO: I liked the old item menu bold selections instead of white background.
//It doesn't work on most terminals though :(
pub fn draw(settings: &Settings, area: winter::Rect, buf: &mut winter::Buffer) {
    let mut items = Vec::new();
    for device in &settings.devices {
        let item = if device.name == settings.current_device {
            lines!(">> ".dim(), &device.name)
        } else {
            lines!("   ", &device.name)
        };
        items.push(item);
    }

    if let Some(index) = settings.index {
        items[index].style = Some(fg(Black).bg(White));
    }

    let list = list(&items).block(block().title("Output Device").title_margin(1));
    list.draw(area, buf, settings.index);
}
