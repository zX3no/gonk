use neoui::*;

pub mod colors {
    use neoui::rgb;

    pub const BG: u32 = rgb(0x14, 0x14, 0x13);
    pub const PANEL: u32 = rgb(0x19, 0x19, 0x18);
    pub const PANEL_RAISED: u32 = rgb(0x21, 0x21, 0x20);
    pub const HOVER: u32 = rgb(0x26, 0x26, 0x25);
    pub const LINE: u32 = rgb(0x2c, 0x2c, 0x2a);
    pub const TEXT: u32 = rgb(0xf1, 0xf0, 0xee);
    pub const TEXT_MUTED: u32 = rgb(0xc4, 0xc2, 0xbe);
    pub const TEXT_DIM: u32 = rgb(0x8a, 0x87, 0x82);
    pub const ACCENT: u32 = rgb(0x84, 0x57, 0xe8);
    pub const ACCENT_BRIGHT: u32 = rgb(0xa1, 0x84, 0xf0);
    pub const ACCENT_DIM: u32 = rgb(0x26, 0x23, 0x2b);
}

pub mod icons {
    pub const SEARCH: &str = "\u{e8b6}";
    pub const PLAYLISTS: &str = "\u{e05f}";
    /// queue_music
    pub const QUEUE: &str = "\u{e03d}";
    pub const PLAY: &str = "\u{e037}";
    pub const PAUSE: &str = "\u{e034}";
    pub const SKIP_PREV: &str = "\u{e045}";
    pub const SKIP_NEXT: &str = "\u{e044}";
    pub const SHUFFLE: &str = "\u{e043}";
    pub const REPEAT: &str = "\u{e040}";
    pub const VOLUME: &str = "\u{e050}";
}

pub const MATERIAL_ICONS: &[u8] = include_bytes!("../assets/fonts/MaterialIcons-Regular.ttf");

pub fn load_icon_font() -> fontdue::Font {
    fontdue::Font::from_bytes(MATERIAL_ICONS, fontdue::FontSettings::default())
        .expect("Material Icons font")
}

pub fn format_time(secs: f32) -> String {
    if !secs.is_finite() || secs < 0.0 {
        return "—:——".to_string();
    }
    let total = secs.floor() as u32;
    let m = total / 60;
    let s = total % 60;
    format!("{m}:{s:02}")
}

pub fn paint_cover(ui: &mut FrameContext<'_, '_>, rect: Rect, radius: usize) {
    ui.paint_rect(
        rect,
        style()
            .bg(colors::PANEL_RAISED)
            .border(colors::LINE)
            .radius(radius),
    );
    let inner = rect.inner(4, 4);
    if inner.width > 0 && inner.height > 0 {
        ui.paint_rect(
            inner,
            style()
                .bg(colors::ACCENT_DIM)
                .radius(radius.saturating_sub(2)),
        );
    }
}
