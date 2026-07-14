use crate::theme::colors;
use neoui::*;
use onmi::Device;

pub enum Action {
    SelectDevice(usize),
}

pub fn draw(
    ui: &mut FrameContext<'_, '_>,
    rect: Rect,
    devices: &[Device],
    current_device: &str,
    music_folder: &str,
    scroll: &mut usize,
) -> Option<Action> {
    let names: Vec<String> = devices.iter().map(|d| d.name.clone()).collect();
    let current = current_device.to_string();
    let folder = music_folder.to_string();
    let mut action = None;

    ui.scroll(bounds(rect).bg(colors::BG), scroll, |ui| {
        ui.text(
            "Settings",
            style()
                .fg(colors::TEXT)
                .font_size(28)
                .padl(40)
                .padt(34)
                .padb(20)
                .fill_width()
                .align(Alignment::Left),
        );
        ui.text(
            "Output device",
            style()
                .fg(colors::TEXT_DIM)
                .font_size(12)
                .padl(40)
                .padb(8)
                .fill_width()
                .align(Alignment::Left),
        );

        let row = style()
            .pad(10)
            .padl(40)
            .padr(40)
            .fill_width()
            .radius(7)
            .align(Alignment::Left)
            .hover(colors::HOVER)
            .fg(colors::TEXT)
            .selected(colors::ACCENT_DIM);

        for (i, name) in names.iter().enumerate() {
            let active = name == &current;
            let label = if active {
                format!("●  {name}")
            } else {
                format!("   {name}")
            };
            if ui.item(label, row.is_selected(active)).clicked {
                action = Some(Action::SelectDevice(i));
            }
        }

        ui.gap(24);
        ui.text(
            format!("Music folder: {folder}"),
            style()
                .fg(colors::TEXT_MUTED)
                .font_size(13)
                .padl(40)
                .fill_width()
                .align(Alignment::Left),
        );
        ui.text(
            "Scan: U · Ctrl+P → Rescan · Ctrl+F song search (or CLI: mu_gui add <path>)",
            style()
                .fg(colors::TEXT_DIM)
                .font_size(12)
                .padl(40)
                .padt(8)
                .fill_width()
                .align(Alignment::Left),
        );
    });
    action
}
