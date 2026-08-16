// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unused)]
use std::{
    collections::HashMap,
    thread::JoinHandle,
    time::{Duration, Instant},
};

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Id, Pos2, Rect, Sense, Shape, Stroke,
    StrokeKind, Vec2, pos2, vec2,
};
use mu_core::Album;
use zune_core::{bytestream::ZCursor, colorspace::ColorSpace, options::DecoderOptions};
use zune_jpeg::JpegDecoder;
use zune_png::PngDecoder;

const BODY: Color32 = Color32::from_rgb(11, 11, 12);
const SIDEBAR: Color32 = Color32::from_rgb(16, 16, 17);

const TEXT: Color32 = Color32::from_rgb(237, 233, 229);
const TEXT_SECONDARY: Color32 = Color32::from_rgb(202, 200, 196);
//Translucent colours are pre blended over their background in gamma 2.0 space to match mu_gui2.
const TEXT_TERTIARY: Color32 = Color32::from_rgb(183, 181, 177);
const TEXT_MUTED: Color32 = Color32::from_rgb(85, 84, 84);
const TEXT_FAINT: Color32 = Color32::from_rgb(64, 64, 63);

const BORDER_DIM: Color32 = Color32::from_rgb(27, 27, 28);

const TRACK_EMPTY: Color32 = Color32::from_rgb(84, 83, 81);

const ROW_HOVER: Color32 = Color32::from_rgb(47, 47, 46);
const ROW_HOVER_BODY: Color32 = Color32::from_rgb(46, 45, 44);
const PLAY_HOVER: Color32 = Color32::from_rgb(186, 181, 175);
const ROW_SELECTED: Color32 = Color32::from_rgb(32, 31, 32);

const ACCENT: Color32 = Color32::from_rgb(155, 132, 217);

const ICON_DIM: Color32 = Color32::from_rgb(155, 154, 150);

///Aptos: (ascender - descender + line gap) / units per em.
const LINE: f32 = 1.220703125;

const ALPHABET: &[&str] = &[
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z",
];

struct Anim {
    current: f32,
    target: f32,
    initial: f32,
    elapsed: f32,
}

impl Anim {
    fn new(value: f32) -> Self {
        Self {
            current: value,
            target: value,
            initial: value,
            elapsed: 0.0,
        }
    }

    fn update(&mut self, target: f32, duration: f32, dt: f32, out_cubic: bool) -> f32 {
        if self.target != target {
            self.initial = self.current;
            self.target = target;
            self.elapsed = 0.0;
        }

        if self.elapsed < duration {
            self.elapsed += dt;
            let t = (self.elapsed / duration).min(1.0);
            let t = if out_cubic {
                1.0 - (1.0 - t).powi(3)
            } else {
                -(std::f32::consts::PI * t).cos() / 2.0 + 0.5
            };
            self.current = self.initial + (self.target - self.initial) * t;
        } else {
            self.current = self.target;
        }

        self.current
    }
}

fn text(
    painter: &egui::Painter,
    rect: Rect,
    string: &str,
    size: f32,
    color: Color32,
    align: Align,
) -> f32 {
    if string.is_empty() {
        return 0.0;
    }

    let galley = painter.layout_no_wrap(string.to_string(), FontId::proportional(size), color);
    let width = galley.size().x;
    let x = match align {
        Align::Min => rect.left(),
        Align::Center => rect.left() + ((rect.width() - width) / 2.0).floor(),
        Align::Max => rect.right() - width,
    };
    let y = rect.top() + ((rect.height() - (size * LINE).round()) / 2.0).floor();
    painter.galley(pos2(x, y), galley, color);
    width
}

fn icon(painter: &egui::Painter, r: Rect, kind: &str, fill: Color32) {
    let palette = [fill, ICON_DIM, SIDEBAR];
    let side = r.width() as i32;
    let u = |v: i32| (v * side / 24) as f32;

    #[rustfmt::skip]
    let bars: &[[i32; 5]] = match kind {
        "Panel"    => &[[2,4,20,16,0], [4,6,16,12,2], [4,6,5,12,0]],
        "Library"  => &[[3,4,4,17,0], [9,4,4,17,0], [15,6,4,15,0], [19,7,3,14,1]],
        "Queue"    => &[[3,6,12,2,0], [3,11,12,2,0], [3,16,8,2,1]],
        "Playlist" => &[[3,6,15,2,0], [3,11,15,2,0], [3,16,9,2,1], [15,16,7,2,0], [17,14,2,7,0]],
        "Settings" => &[[5,3,2,18,1], [12,3,2,18,1], [19,3,2,18,1], [3,7,6,3,0], [10,13,6,3,0], [17,6,6,3,0]],
        "Shuffle"  => &[[4,7,13,2,0], [7,15,13,2,0]],
        "Pause"    => &[[7,4,4,16,0], [13,4,4,16,0]],
        "Volume"   => &[[3,9,4,6,0], [14,9,2,6,1]],
        _ => &[],
    };

    for &[x, y, w, h, c] in bars {
        let bar = Rect::from_min_size(
            pos2(r.left() + u(x), r.top() + u(y)),
            vec2(u(w).max(1.0), u(h).max(1.0)),
        );
        painter.rect_filled(bar, CornerRadius::same(1), palette[c as usize]);
    }

    let tri = |a: (i32, i32), b: (i32, i32), c: (i32, i32)| {
        let p = |(px, py): (i32, i32)| pos2(r.left() + u(px), r.top() + u(py));
        painter.add(Shape::convex_polygon(
            vec![p(a), p(b), p(c)],
            fill,
            Stroke::NONE,
        ));
    };

    match kind {
        "Queue" => tri((15, 11), (15, 20), (22, 15)),
        "Shuffle" => {
            tri((16, 4), (16, 12), (21, 8));
            tri((8, 12), (8, 20), (3, 16));
        }
        "Play" => tri((6, 3), (6, 21), (21, 12)),
        "Rewind" => {
            tri((11, 3), (11, 21), (1, 12));
            tri((22, 3), (22, 21), (12, 12));
        }
        "Forward" => {
            tri((13, 3), (13, 21), (23, 12));
            tri((2, 3), (2, 21), (12, 12));
        }
        "Volume" => tri((11, 3), (11, 21), (6, 12)),
        "Repeat" => {
            let ring =
                Rect::from_min_size(pos2(r.left() + u(3), r.top() + u(3)), vec2(u(18), u(18)));
            let stroke = Stroke::new(u(2).max(1.0), fill);
            //Draw the ring twice, clipped, so everything but the top right is covered.
            let left = Rect::from_min_size(r.min, vec2(u(12), u(24)));
            let bottom =
                Rect::from_min_size(pos2(r.left() + u(12), r.top() + u(9)), vec2(u(12), u(15)));
            for clip in [left, bottom] {
                painter.with_clip_rect(clip).circle_stroke(
                    ring.center(),
                    (ring.width() - stroke.width) / 2.0,
                    stroke,
                );
            }
            tri((12, 1), (12, 7), (18, 4));
        }
        _ => {}
    }
}

fn icon_button(
    ui: &mut egui::Ui,
    rect: Rect,
    kind: &str,
    fill: Color32,
    bg: Option<Color32>,
    hover: Color32,
    radius: u8,
    pad: f32,
) -> egui::Response {
    let response = ui.interact(rect, Id::new(("icon", kind)), Sense::click());
    let bg = if response.hovered() { Some(hover) } else { bg };

    if let Some(bg) = bg {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(radius), bg);
    }

    icon(ui.painter(), rect.shrink(pad), kind, fill);
    response
}

fn image_rect(painter: &egui::Painter, rect: Rect, texture: &egui::TextureHandle, radius: u8) {
    let mut shape =
        egui::epaint::RectShape::filled(rect, CornerRadius::same(radius), Color32::WHITE);
    shape.brush = Some(std::sync::Arc::new(egui::epaint::Brush {
        fill_texture_id: texture.id(),
        uv: Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
    }));
    painter.add(shape);
}

fn gradient(painter: &egui::Painter, rect: Rect, stops: &[(f32, Color32)]) {
    let mut mesh = egui::epaint::Mesh::default();
    for pair in stops.windows(2) {
        let (start, top) = pair[0];
        let (end, bottom) = pair[1];
        let y0 = rect.top() + rect.height() * start;
        let y1 = rect.top() + rect.height() * end;
        let index = mesh.vertices.len() as u32;
        for (y, color) in [(y0, top), (y1, bottom)] {
            for x in [rect.left(), rect.right()] {
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: pos2(x, y),
                    uv: egui::epaint::WHITE_UV,
                    color,
                });
            }
        }
        mesh.indices.extend_from_slice(&[
            index,
            index + 1,
            index + 2,
            index + 1,
            index + 2,
            index + 3,
        ]);
    }
    painter.add(Shape::mesh(mesh));
}

struct Sidebar {
    artists: Vec<String>,
    selected_artist: String,
    selected_mode: &'static str,
    current_letter: Option<char>,
    active: bool,
    update_library: bool,
    jump_to_letter: Option<char>,
    scroll: f32,
    max_scroll: f32,
    jump: Option<f32>,
    fade: Anim,
    hovered: bool,
}

fn draw_rail(ui: &mut egui::Ui, bounds: Rect, sidebar: &mut Sidebar) {
    ui.painter()
        .rect_filled(bounds, CornerRadius::ZERO, SIDEBAR);
    ui.painter().rect_filled(
        Rect::from_min_size(
            pos2(bounds.right() - 1.0, bounds.top()),
            vec2(1.0, bounds.height()),
        ),
        CornerRadius::ZERO,
        BORDER_DIM,
    );

    let x = bounds.left() + 11.0;
    let mut y = bounds.top() + 14.0;

    if icon_button(
        ui,
        Rect::from_min_size(pos2(x, y), vec2(34.0, 34.0)),
        "Panel",
        TEXT_TERTIARY,
        None,
        ROW_HOVER,
        8,
        7.0,
    )
    .clicked()
    {
        sidebar.active = true;
    }

    y += 34.0 + 4.0 + 6.0;

    for mode in ["Library", "Queue", "Playlist", "Settings"] {
        let selected = mode == sidebar.selected_mode;
        let clicked = icon_button(
            ui,
            Rect::from_min_size(pos2(x, y), vec2(34.0, 34.0)),
            mode,
            if selected { TEXT } else { TEXT_TERTIARY },
            if selected { Some(ROW_SELECTED) } else { None },
            if selected { ROW_SELECTED } else { ROW_HOVER },
            8,
            7.0,
        )
        .clicked();

        if clicked {
            sidebar.selected_mode = mode;
        }

        y += 38.0;
    }

    y += 10.0;
    ui.painter().rect_filled(
        Rect::from_min_size(pos2(bounds.left(), y), vec2(bounds.width(), 1.0)),
        CornerRadius::ZERO,
        BORDER_DIM,
    );
}

fn draw_sidebar(ui: &mut egui::Ui, bounds: Rect, sidebar: &mut Sidebar, dt: f32) {
    let painter = ui.painter().clone();
    painter.rect_filled(bounds, CornerRadius::ZERO, SIDEBAR);
    painter.rect_filled(
        Rect::from_min_size(
            pos2(bounds.right() - 1.0, bounds.top()),
            vec2(1.0, bounds.height()),
        ),
        CornerRadius::ZERO,
        BORDER_DIM,
    );

    let header = Rect::from_min_size(bounds.min, vec2(bounds.width(), 48.0));
    text(
        &painter,
        Rect::from_min_size(
            pos2(header.left() + 18.0, header.top() + 20.0),
            vec2(0.0, 8.0),
        ),
        "mu",
        16.0,
        TEXT,
        Align::Min,
    );

    if icon_button(
        ui,
        Rect::from_min_size(
            pos2(header.right() - 8.0 - 30.0, header.top() + 9.0),
            vec2(30.0, 30.0),
        ),
        "Panel",
        TEXT_TERTIARY,
        None,
        ROW_HOVER,
        6,
        5.0,
    )
    .clicked()
    {
        sidebar.active = false;
    }

    let mut y = bounds.top() + 48.0;
    for (mode, number) in [
        ("Library", "1"),
        ("Queue", "2"),
        ("Playlist", "3"),
        ("Settings", "4"),
    ] {
        let selected = mode == sidebar.selected_mode;
        let row = Rect::from_min_size(
            pos2(bounds.left() + 8.0, y),
            vec2(bounds.width() - 16.0, 36.0),
        );
        let response = ui.interact(row, Id::new(("mode", mode)), Sense::click());

        if response.hovered() {
            painter.rect_filled(row, CornerRadius::same(6), ROW_HOVER);
        } else if selected {
            painter.rect_filled(row, CornerRadius::same(6), ROW_SELECTED);
        }

        let inner = Rect::from_min_max(
            pos2(row.left() + 12.0, row.top()),
            pos2(row.right() - 12.0, row.bottom()),
        );
        text(
            &painter,
            inner,
            mode,
            16.0,
            if selected { TEXT } else { TEXT_TERTIARY },
            Align::Min,
        );
        text(
            &painter,
            inner,
            number,
            16.0,
            if selected { TEXT_MUTED } else { TEXT_FAINT },
            Align::Max,
        );

        if response.clicked() {
            sidebar.selected_mode = mode;
        }

        y += 38.0;
    }

    y += 8.0 - 2.0;
    painter.rect_filled(
        Rect::from_min_size(pos2(bounds.left(), y), vec2(bounds.width(), 1.0)),
        CornerRadius::ZERO,
        BORDER_DIM,
    );

    let rest = Rect::from_min_max(pos2(bounds.left(), y + 1.0), bounds.max);
    let artists = Rect::from_min_max(rest.min, pos2(rest.right() - 30.0, rest.bottom()));
    let mut alphabet = Rect::from_min_max(pos2(rest.right() - 30.0, rest.top()), rest.max);

    let list = Rect::from_min_max(
        pos2(artists.left() + 8.0, artists.top()),
        pos2(artists.right() - 8.0, artists.bottom()),
    );
    let mut scroll = egui::ScrollArea::vertical()
        .id_salt("artists")
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);

    if let Some(jump) = sidebar.jump.take() {
        scroll = scroll.vertical_scroll_offset(jump);
    }

    let mut jump_to = None;
    let mut current_letter = None;
    let selected_artist = sidebar.selected_artist.clone();
    let jump_target = sidebar.jump_to_letter.take();

    let output = ui
        .scope_builder(egui::UiBuilder::new().max_rect(list), |ui| {
            scroll.show(ui, |ui| {
                let origin = ui.max_rect().min;
                let width = list.width();
                let mut y = origin.y;
                let mut first_letter = ' ';

                for (index, artist) in sidebar.artists.iter().enumerate() {
                    let next = artist.chars().next().unwrap().to_ascii_uppercase();
                    if next != first_letter {
                        first_letter = next;
                        let header = Rect::from_min_size(pos2(origin.x, y), vec2(width, 31.0));

                        if current_letter.is_none() || header.top() <= list.top() {
                            current_letter = Some(first_letter);
                        }

                        if let Some(target) = jump_target
                            && first_letter == target.to_ascii_uppercase()
                            && jump_to.is_none()
                        {
                            jump_to = Some(y - origin.y);
                        }

                        text(
                            ui.painter(),
                            Rect::from_min_size(
                                pos2(header.left() + 12.0, header.top()),
                                vec2(width - 24.0, 31.0),
                            ),
                            &first_letter.to_string(),
                            12.0,
                            TEXT_MUTED,
                            Align::Min,
                        );
                        y += 31.0;
                    }

                    let row = Rect::from_min_size(pos2(origin.x, y), vec2(width, 36.0));
                    let response = ui.interact(row, Id::new(("artist", index)), Sense::click());
                    let selected = *artist == selected_artist;

                    if response.hovered() {
                        ui.painter()
                            .rect_filled(row, CornerRadius::same(6), ROW_HOVER);
                    } else if selected {
                        ui.painter()
                            .rect_filled(row, CornerRadius::same(6), ROW_SELECTED);
                    }

                    text(
                        ui.painter(),
                        Rect::from_min_size(
                            pos2(row.left() + 12.0, row.top()),
                            vec2(width - 24.0, 36.0),
                        ),
                        artist,
                        16.0,
                        TEXT,
                        Align::Min,
                    );

                    if response.clicked() {
                        sidebar.selected_artist = artist.clone();
                        sidebar.update_library = true;
                    }

                    y += 36.0;
                }

                ui.allocate_rect(
                    Rect::from_min_max(origin, pos2(origin.x + width, y)),
                    Sense::hover(),
                );
            })
        })
        .inner;

    sidebar.current_letter = current_letter;
    sidebar.scroll = output.state.offset.y;
    sidebar.max_scroll = (output.content_size.y - output.inner_rect.height()).max(0.0);

    if let Some(offset) = jump_to {
        sidebar.jump = Some(offset);
    }

    painter.rect_filled(
        Rect::from_min_size(alphabet.min, vec2(1.0, alphabet.height())),
        CornerRadius::ZERO,
        BORDER_DIM,
    );

    let response = ui.interact(alphabet, Id::new("alphabet"), Sense::click_and_drag());
    if let Some(pos) = response.interact_pointer_pos() {
        let raw = ((pos.y - alphabet.top()) / alphabet.height()).clamp(0.0, 1.0);
        let percentage = ((raw - 0.03) / 0.90).clamp(0.0, 1.0);
        sidebar.jump = Some(percentage * sidebar.max_scroll);
    }

    let fade = sidebar
        .fade
        .update(if response.hovered() { 1.0 } else { 0.0 }, 0.15, dt, false);

    if fade > 0.0
        && let Some(mouse) = ui.ctx().pointer_latest_pos()
    {
        let glow =
            |a: f32| Color32::from_rgba_unmultiplied(155, 132, 217, (a * fade * 255.0) as u8);
        let painter = painter.with_clip_rect(alphabet);

        gradient(
            &painter,
            Rect::from_min_size(
                pos2(alphabet.left(), mouse.y - 55.0),
                vec2(alphabet.width(), 110.0),
            ),
            &[
                (0.0, glow(0.0)),
                (0.21, glow(0.11)),
                (0.5, glow(0.30)),
                (0.79, glow(0.11)),
                (1.0, glow(0.0)),
            ],
        );

        gradient(
            &painter,
            Rect::from_min_size(pos2(alphabet.left(), mouse.y - 70.0), vec2(1.0, 140.0)),
            &[
                (0.0, glow(0.0)),
                (
                    0.5,
                    Color32::from_rgba_unmultiplied(199, 183, 240, (0.75 * fade * 255.0) as u8),
                ),
                (1.0, glow(0.0)),
            ],
        );
    }

    alphabet.min.x += 12.0;
    let mut y = alphabet.top() + ((alphabet.height() - 12.0 * 26.0) / 2.0).floor();

    if let Some(current) = sidebar.current_letter {
        for letter in ALPHABET {
            let color = if letter.chars().next() == Some(current) {
                ACCENT
            } else {
                TEXT_TERTIARY
            };
            text(
                &painter,
                Rect::from_min_size(pos2(alphabet.left(), y), vec2(alphabet.width(), 12.0)),
                letter,
                10.0,
                color,
                Align::Min,
            );
            y += 12.0;
        }
    }
}

struct Library {
    artist: String,
    total_tracks: usize,
    ///(Album, Song)
    playing_song: Option<(usize, usize)>,
    ///(Album, Song)
    selected_song: Option<(usize, usize)>,
    reset_scroll: bool,
}

struct Controls {
    ///Artist, Album, Song
    song: Option<(String, usize, usize)>,
    playing: bool,
    elapsed: f32,
    duration: f32,
    volume: u8,
}

fn draw_library(
    ui: &mut egui::Ui,
    bounds: Rect,
    albums: &[Album],
    library: &mut Library,
    controls: &mut Controls,
    player: &mut onmi::Player,
    textures: &HashMap<(String, String), egui::TextureHandle>,
) {
    let painter = ui.painter().clone();
    painter.rect_filled(bounds, CornerRadius::ZERO, BODY);

    let left = bounds.left() + 36.0;
    let right = bounds.right() - 36.0;

    text(
        &painter,
        Rect::from_min_size(pos2(left, bounds.top()), vec2(right - left, 51.0)),
        &library.artist,
        42.0,
        Color32::WHITE,
        Align::Min,
    );

    let header = format!("{} ALBUMS · {} TRACKS", albums.len(), library.total_tracks);
    text(
        &painter,
        Rect::from_min_size(pos2(left, bounds.top() + 55.0), vec2(right - left, 15.0)),
        &header,
        12.0,
        TEXT_MUTED,
        Align::Min,
    );

    painter.rect_filled(
        Rect::from_min_size(pos2(left, bounds.top() + 82.0), vec2(right - left, 1.0)),
        CornerRadius::ZERO,
        BORDER_DIM,
    );

    let list = Rect::from_min_max(
        pos2(left, bounds.top() + 95.0),
        pos2(right, bounds.bottom()),
    );
    let mut scroll = egui::ScrollArea::vertical()
        .id_salt("library")
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);

    if library.reset_scroll {
        library.reset_scroll = false;
        scroll = scroll.vertical_scroll_offset(0.0);
    }

    ui.scope_builder(egui::UiBuilder::new().max_rect(list), |ui| {
        scroll.show(ui, |ui| {
            let origin = ui.max_rect().min;
            let mut y = origin.y;

            for (ai, album) in albums.iter().enumerate() {
                let rows = album.songs.len() as f32;
                let height = (29.0 + 12.0 + rows * 36.0 + (rows - 1.0) * 2.0).max(148.0);
                let art = Rect::from_min_size(pos2(origin.x, y), vec2(148.0, 148.0));

                match textures.get(&(library.artist.clone(), album.title.clone())) {
                    Some(texture) => image_rect(ui.painter(), art, texture, 8),
                    None => {
                        ui.painter()
                            .rect_filled(art, CornerRadius::ZERO, BORDER_DIM);
                    }
                }

                let column_left = origin.x + 148.0 + 24.0;
                let column_right = list.right();

                let title_width = text(
                    ui.painter(),
                    Rect::from_min_size(
                        pos2(column_left, y),
                        vec2(column_right - column_left, 29.0),
                    ),
                    &album.title,
                    24.0,
                    Color32::WHITE,
                    Align::Min,
                );

                let tracks = if album.songs.len() > 1 {
                    "tracks"
                } else {
                    "track"
                };
                let year = format!("{} · {} {}", album.year(), album.songs.len(), tracks);
                text(
                    ui.painter(),
                    Rect::from_min_size(
                        pos2(column_left + title_width + 12.0, y + 8.0),
                        vec2(column_right - column_left, 20.0),
                    ),
                    &year,
                    16.0,
                    TEXT_MUTED,
                    Align::Min,
                );

                let mut row_y = y + 29.0 + 12.0;

                for (si, song) in album.songs.iter().enumerate() {
                    let playing = Some((ai, si)) == library.playing_song;
                    let selected = Some((ai, si)) == library.selected_song;
                    let row = Rect::from_min_max(
                        pos2(column_left, row_y),
                        pos2(column_right, row_y + 36.0),
                    );
                    let response = ui.interact(row, Id::new(("song", ai, si)), Sense::click());

                    if response.hovered() {
                        ui.painter()
                            .rect_filled(row, CornerRadius::same(12), ROW_HOVER_BODY);
                    } else if playing || selected {
                        ui.painter()
                            .rect_filled(row, CornerRadius::same(12), ROW_SELECTED);
                    }

                    let number = format!("{:02}", song.track_number);
                    let number_width = text(
                        ui.painter(),
                        Rect::from_min_size(pos2(row.left() + 12.0, row_y), vec2(0.0, 36.0)),
                        &number,
                        16.0,
                        if playing { ACCENT } else { TEXT_MUTED },
                        Align::Min,
                    );

                    text(
                        ui.painter(),
                        Rect::from_min_size(
                            pos2(row.left() + 12.0 + number_width + 12.0, row_y),
                            vec2(0.0, 36.0),
                        ),
                        &song.title,
                        16.0,
                        if playing {
                            Color32::WHITE
                        } else {
                            TEXT_SECONDARY
                        },
                        Align::Min,
                    );

                    let duration = Duration::from_secs_f32(song.duration);
                    let total_secs = duration.as_secs();
                    let hours = total_secs / 3600;
                    let minutes = (total_secs % 3600) / 60;
                    let seconds = total_secs % 60;
                    let duration = if hours > 0 {
                        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
                    } else {
                        format!("{:02}:{:02}", minutes, seconds)
                    };

                    text(
                        ui.painter(),
                        Rect::from_min_max(
                            pos2(row.left(), row_y),
                            pos2(row.right() - 12.0, row_y + 36.0),
                        ),
                        &duration,
                        16.0,
                        TEXT_MUTED,
                        Align::Max,
                    );

                    if response.clicked() {
                        library.selected_song = Some((ai, si));
                    }

                    if response.double_clicked() {
                        player.play_song(&song.path, Some(song.gain), true);
                        library.playing_song = Some((ai, si));
                        controls.song = Some((library.artist.clone(), ai, si));
                        controls.playing = true;
                    }

                    row_y += 38.0;
                }

                y += height;

                if (ai + 1) < albums.len() {
                    ui.painter().rect_filled(
                        Rect::from_min_size(pos2(origin.x, y + 24.0), vec2(list.width(), 1.0)),
                        CornerRadius::ZERO,
                        BORDER_DIM,
                    );
                    y += 49.0;
                }
            }

            ui.allocate_rect(
                Rect::from_min_max(origin, pos2(origin.x + list.width(), y)),
                Sense::hover(),
            );
        });
    });
}

fn draw_controls(
    ui: &mut egui::Ui,
    bounds: Rect,
    controls: &mut Controls,
    player: &mut onmi::Player,
    db: &mu_core::vdb::Database,
    textures: &HashMap<(String, String), egui::TextureHandle>,
) {
    let painter = ui.painter().clone();
    painter.rect_filled(bounds, CornerRadius::ZERO, SIDEBAR);
    painter.rect_filled(
        Rect::from_min_size(bounds.min, vec2(bounds.width(), 1.0)),
        CornerRadius::ZERO,
        BORDER_DIM,
    );

    let first = bounds.left() + (bounds.width() * 0.28).round();
    let second = bounds.left() + (bounds.width() * 0.72).round();
    let info = Rect::from_min_max(bounds.min, pos2(first, bounds.bottom()));
    let center = Rect::from_min_max(pos2(first, bounds.top()), pos2(second, bounds.bottom()));
    let extras = Rect::from_min_max(pos2(second, bounds.top()), bounds.max);

    if let Some((artist, ai, si)) = &controls.song {
        let albums = db.albums_by_artist(artist);
        let album = &albums[*ai];
        let song = &album.songs[*si];
        let art = Rect::from_min_size(
            pos2(info.left() + 16.0, info.top() + 18.0),
            vec2(48.0, 48.0),
        );

        match textures.get(&(artist.clone(), album.title.clone())) {
            Some(texture) => image_rect(&painter, art, texture, 6),
            None => {
                painter.rect_filled(art, CornerRadius::same(4), Color32::GRAY);
            }
        }

        let column = Rect::from_min_max(
            pos2(art.right() + 12.0, info.top() + 22.0),
            pos2(info.right(), info.top() + 62.0),
        );
        let painter = painter.with_clip_rect(info);

        text(
            &painter,
            Rect::from_min_size(column.min, vec2(column.width(), 20.0)),
            &song.title,
            16.0,
            TEXT,
            Align::Min,
        );

        let details = format!("{} · {}", song.artist, song.album);
        text(
            &painter,
            Rect::from_min_size(
                pos2(column.left(), column.top() + 20.0),
                vec2(column.width(), 17.0),
            ),
            &details,
            14.0,
            TEXT_MUTED,
            Align::Min,
        );
    }

    let buttons = Rect::from_min_size(
        pos2(
            center.left() + ((center.width() - 200.0) / 2.0).floor(),
            center.top() + 14.0,
        ),
        vec2(200.0, 32.0),
    );

    for (index, kind) in [
        "Shuffle",
        "Rewind",
        if controls.playing { "Pause" } else { "Play" },
        "Forward",
        "Repeat",
    ]
    .iter()
    .enumerate()
    {
        let rect = Rect::from_min_size(
            pos2(buttons.left() + index as f32 * 42.0, buttons.top()),
            vec2(32.0, 32.0),
        );

        let play = index == 2;
        let clicked = icon_button(
            ui,
            rect,
            kind,
            if play { SIDEBAR } else { TEXT_TERTIARY },
            if play { Some(TEXT) } else { None },
            if play { PLAY_HOVER } else { ROW_HOVER },
            if play { 16 } else { 8 },
            4.0,
        )
        .clicked();

        if clicked && play {
            if controls.playing {
                player.pause();
            } else {
                player.play();
            }
            controls.playing = !controls.playing;
        }
    }

    let seek = Rect::from_min_size(
        pos2(center.left(), center.top() + 52.0),
        vec2(center.width(), 20.0),
    );
    let track = Rect::from_min_size(
        pos2(seek.left() + 46.0, seek.top() + 8.0),
        vec2(seek.width() - 92.0, 4.0),
    );

    let elapsed = format!(
        "{:02}:{:02}",
        (controls.elapsed.max(0.0) as u32) / 60,
        (controls.elapsed.max(0.0) as u32) % 60
    );
    text(
        &painter,
        Rect::from_min_size(seek.min, vec2(36.0, 20.0)),
        &elapsed,
        13.0,
        TEXT_MUTED,
        Align::Max,
    );

    let duration = format!(
        "{:02}:{:02}",
        (controls.duration.max(0.0) as u32) / 60,
        (controls.duration.max(0.0) as u32) % 60
    );
    text(
        &painter,
        Rect::from_min_size(pos2(track.right() + 10.0, seek.top()), vec2(36.0, 20.0)),
        &duration,
        13.0,
        TEXT_MUTED,
        Align::Min,
    );

    painter.rect_filled(track, CornerRadius::same(2), TRACK_EMPTY);

    if controls.duration > 0.0 {
        painter.rect_filled(
            Rect::from_min_size(
                track.min,
                vec2(
                    (track.width() * controls.elapsed / controls.duration).max(0.0),
                    track.height(),
                ),
            ),
            CornerRadius::same(2),
            ACCENT,
        );
    }

    let outset = track.expand2(vec2(0.0, 12.0));
    let response = ui.interact(outset, Id::new("seek"), Sense::click_and_drag());

    if controls.playing
        && (response.clicked() || response.drag_stopped())
        && let Some(pos) = response.interact_pointer_pos()
    {
        let percentage = ((pos.x - outset.left()) / outset.width()).clamp(0.0, 1.0);
        player.seek_to(Duration::from_secs_f32(
            player.duration().as_secs_f32() * percentage,
        ));
    }

    let volume = format!("{}", controls.volume);
    text(
        &painter,
        Rect::from_min_size(
            pos2(extras.right() - 16.0 - 24.0, extras.top() + 32.0),
            vec2(24.0, 20.0),
        ),
        &volume,
        13.0,
        TEXT_MUTED,
        Align::Center,
    );

    let slider = Rect::from_min_size(
        pos2(
            extras.right() - 16.0 - 24.0 - 10.0 - 96.0,
            extras.top() + 40.0,
        ),
        vec2(96.0, 4.0),
    );
    painter.rect_filled(slider, CornerRadius::same(2), TRACK_EMPTY);
    painter.rect_filled(
        Rect::from_min_size(
            slider.min,
            vec2(
                slider.width() * controls.volume as f32 / 100.0,
                slider.height(),
            ),
        ),
        CornerRadius::same(2),
        ACCENT,
    );

    let outset = slider.expand2(vec2(0.0, 12.0));
    let response = ui.interact(outset, Id::new("volume"), Sense::click_and_drag());

    if let Some(pos) = response.interact_pointer_pos() {
        let percentage = ((pos.x - outset.left()) / outset.width()).clamp(0.0, 1.0);
        controls.volume = ((percentage * 100.0) as u8).clamp(0, 100);
        player.set_volume(controls.volume);
    }

    icon_button(
        ui,
        Rect::from_min_size(
            pos2(slider.left() - 10.0 - 32.0, extras.top() + 26.0),
            vec2(32.0, 32.0),
        ),
        "Volume",
        TEXT_TERTIARY,
        None,
        ROW_HOVER,
        8,
        4.0,
    );
}

const LANES: u32 = 0x00FF_00FF;
const WEIGHT_ONE: u32 = 256;

//Pixels are premultiplied RGBA, byte order matching egui::Color32.
fn decode(bytes: &[u8]) -> Option<(Vec<u32>, usize, usize)> {
    let mut out: Vec<u32> = Vec::new();

    let (width, height, channels) = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        let options = DecoderOptions::default()
            .png_set_add_alpha_channel(true)
            .png_set_strip_to_8bit(true);
        let mut decoder = PngDecoder::new_with_options(ZCursor::new(bytes), options);
        decoder.decode_headers().ok()?;
        let (width, height) = decoder.dimensions()?;
        let channels = decoder.output_buffer_size()? / (width * height);
        out.resize(width * height, 0);
        decoder
            .decode_into(unsafe {
                std::slice::from_raw_parts_mut(out.as_mut_ptr() as *mut u8, out.len() * 4)
            })
            .ok()?;
        (width, height, channels)
    } else if bytes.starts_with(&[0xFF, 0xD8]) {
        let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGBA);
        let mut decoder = JpegDecoder::new_with_options(ZCursor::new(bytes), options);
        decoder.decode_headers().ok()?;
        let info = decoder.info()?;
        let (width, height) = (info.width as usize, info.height as usize);
        let channels = decoder.output_buffer_size()? / (width * height);
        out.resize(width * height, 0);
        decoder
            .decode_into(unsafe {
                std::slice::from_raw_parts_mut(out.as_mut_ptr() as *mut u8, out.len() * 4)
            })
            .ok()?;
        (width, height, channels)
    } else {
        return None;
    };

    match channels {
        4 => {
            for pixel in out.iter_mut() {
                let [r, g, b, a] = pixel.to_ne_bytes();
                let mul = |v: u8| (v as u32 * a as u32 + 127) / 255;
                *pixel = u32::from_ne_bytes([mul(r) as u8, mul(g) as u8, mul(b) as u8, a]);
            }
        }
        2 => {
            for i in (0..width * height).rev() {
                let word = out[i / 2].to_ne_bytes();
                let (luma, alpha) = if i % 2 == 0 {
                    (word[0], word[1])
                } else {
                    (word[2], word[3])
                };
                let luma = ((luma as u32 * alpha as u32 + 127) / 255) as u8;
                out[i] = u32::from_ne_bytes([luma, luma, luma, alpha]);
            }
        }
        _ => return None,
    }

    Some((out, width, height))
}

fn resize(src: &[u32], src_w: usize, src_h: usize, width: usize, height: usize) -> Vec<u32> {
    assert!(width > 0 && height > 0 && src_w > 0 && src_h > 0);

    struct Axis {
        taps: usize,
        first: Vec<usize>,
        weights: Vec<u16>,
    }

    fn axis(source_len: usize, dest_len: usize) -> Axis {
        let ratio = source_len as f32 / dest_len as f32;
        let half = (ratio * 0.5).max(0.5);
        let taps = ((2.0 * half).ceil() as usize + 1).clamp(1, source_len);
        let last_window = source_len - taps;

        let mut first = Vec::with_capacity(dest_len);
        let mut weights = vec![0u16; dest_len * taps];
        let mut scratch = vec![0f32; taps];

        for i in 0..dest_len {
            let centre = (i as f32 + 0.5) * ratio;
            let (low, high) = (centre - half, centre + half);
            let begin = low.floor() as i64;
            let window = begin.clamp(0, last_window as i64) as usize;

            scratch.fill(0.0);
            let mut total = 0.0;
            for tap in 0..taps {
                let edge = begin + tap as i64;
                let weight = (((edge + 1) as f32).min(high) - (edge as f32).max(low)).max(0.0);
                let j = edge.clamp(0, source_len as i64 - 1) as usize;
                scratch[j.saturating_sub(window).min(taps - 1)] += weight;
                total += weight;
            }
            if total <= 0.0 {
                scratch[0] = 1.0;
                total = 1.0;
            }

            let normalize = WEIGHT_ONE as f32 / total;
            let (mut exact, mut assigned) = (0.0f32, 0u32);
            for tap in 0..taps {
                exact += scratch[tap] * normalize;
                let weight = (exact.round() as u32)
                    .saturating_sub(assigned)
                    .min(WEIGHT_ONE);
                weights[i * taps + tap] = weight as u16;
                assigned += weight;
            }
            first.push(window);
        }

        Axis {
            taps,
            first,
            weights,
        }
    }

    fn gather(source: &[u32], weights: &[u16], stride: usize) -> u32 {
        let (mut low, mut high) = (0u32, 0u32);
        for (tap, &weight) in weights.iter().enumerate() {
            let pixel = source[tap * stride];
            let weight = weight as u32;
            low += (pixel & LANES) * weight;
            high += ((pixel >> 8) & LANES) * weight;
        }
        ((low >> 8) & LANES) | (((high >> 8) & LANES) << 8)
    }

    //Halve repeatedly first so the filtered pass only ever sees a small source.
    let mut reduced: Vec<u32> = Vec::new();
    let (mut src_w, mut src_h) = (src_w, src_h);
    while src_w >= 2 * width && src_h >= 2 * height {
        let (half_w, half_h) = (src_w.div_ceil(2), src_h.div_ceil(2));
        let current: &[u32] = if reduced.is_empty() { src } else { &reduced };
        let mut next = vec![0u32; half_w * half_h];
        for y in 0..half_h {
            let top = (y * 2).min(src_h - 1) * src_w;
            let bottom = (y * 2 + 1).min(src_h - 1) * src_w;
            for (x, pixel) in next[y * half_w..][..half_w].iter_mut().enumerate() {
                let left = x * 2;
                let right = (left + 1).min(src_w - 1);
                let (a, b) = (current[top + left], current[top + right]);
                let (c, d) = (current[bottom + left], current[bottom + right]);
                let low = (a & LANES) + (b & LANES) + (c & LANES) + (d & LANES) + 0x0002_0002;
                let high = ((a >> 8) & LANES)
                    + ((b >> 8) & LANES)
                    + ((c >> 8) & LANES)
                    + ((d >> 8) & LANES)
                    + 0x0002_0002;
                *pixel = ((low >> 2) & LANES) | (((high >> 2) & LANES) << 8);
            }
        }
        reduced = next;
        src_w = half_w;
        src_h = half_h;
    }

    let pixels: &[u32] = if reduced.is_empty() { src } else { &reduced };
    if src_w == width && src_h == height {
        return pixels.to_vec();
    }
    let mut out = vec![0u32; width * height];

    let horizontal = axis(src_w, width);
    let vertical = axis(src_h, height);
    let row_start = vertical.first[0];
    let rows = (vertical.first[height - 1] + vertical.taps).min(src_h) - row_start;

    let mut scratch = vec![0u32; width * rows];
    for row in 0..rows {
        let source_row = &pixels[(row_start + row) * src_w..][..src_w];
        for (i, pixel) in scratch[row * width..][..width].iter_mut().enumerate() {
            let weights = &horizontal.weights[i * horizontal.taps..][..horizontal.taps];
            *pixel = gather(&source_row[horizontal.first[i]..], weights, 1);
        }
    }
    for y in 0..height {
        let weights = &vertical.weights[y * vertical.taps..][..vertical.taps];
        let top = vertical.first[y] - row_start;
        for (x, pixel) in out[y * width..][..width].iter_mut().enumerate() {
            *pixel = gather(&scratch[top * width + x..], weights, width);
        }
    }

    out
}

fn spawn_load_artwork(
    artist: String,
    albums: Vec<(String, String)>,
) -> JoinHandle<(String, Vec<(String, egui::ColorImage)>)> {
    std::thread::spawn(move || {
        let now = Instant::now();
        let threads = 16;
        let chunk = albums.len().div_ceil(threads).max(1);
        let mut artwork = Vec::new();

        std::thread::scope(|scope| {
            let mut handles = Vec::new();

            for albums in albums.chunks(chunk) {
                handles.push(scope.spawn(move || {
                    let mut decoded = Vec::new();

                    for (title, path) in albums {
                        //Use the first song for the whole album.
                        //Technically each track can have a different album cover.
                        if let Ok(song) = onmi::metadata(path, false, true)
                            && let Some(artwork) = song.artwork
                            && let Some((pixels, width, height)) = decode(&artwork.data)
                        {
                            let size = 512;
                            let pixels = resize(&pixels, width, height, size, size);
                            let bytes = unsafe {
                                std::slice::from_raw_parts(
                                    pixels.as_ptr() as *const u8,
                                    pixels.len() * 4,
                                )
                            };
                            decoded.push((
                                title.clone(),
                                egui::ColorImage::from_rgba_premultiplied([size, size], bytes),
                            ));
                        }
                    }

                    decoded
                }));
            }

            for handle in handles {
                artwork.extend(handle.join().unwrap());
            }
        });

        println!("Loaded {artist} in {}ms", now.elapsed().as_millis());

        (artist, artwork)
    })
}

fn main() {
    let now = Instant::now();
    let player = std::thread::spawn(move || {
        let now = Instant::now();
        let outputs = onmi::OutputDevices::new();
        let player = onmi::Player::new(outputs.default_device());
        println!("Loaded Player in {}ms", now.elapsed().as_millis());
        player
    });

    let db = std::thread::spawn(|| {
        let now = Instant::now();
        let config = mu_core::config_paths();
        let db = mu_core::vdb::Database::new(&config.database);
        let mut artists: Vec<String> = db.btree.keys().cloned().collect();
        artists.sort_by_key(|a| a.to_ascii_lowercase());
        println!("Loaded DB in {}ms", now.elapsed().as_millis());
        (db, artists)
    });

    let mut player = player.join().unwrap();
    let (mut db, artists) = db.join().unwrap();
    let mut artwork_task: Option<JoinHandle<(String, Vec<(String, egui::ColorImage)>)>> = None;
    let mut textures: HashMap<(String, String), egui::TextureHandle> = HashMap::new();

    println!("Loaded {}ms", now.elapsed().as_millis());

    let mut sidebar = Sidebar {
        selected_artist: String::from("Duster"),
        artists,
        selected_mode: "Library",
        current_letter: None,
        active: true,
        update_library: true,
        jump_to_letter: None,
        scroll: 0.0,
        max_scroll: 0.0,
        jump: None,
        fade: Anim::new(0.0),
        hovered: false,
    };

    let mut library = Library {
        artist: String::from("Duster"),
        total_tracks: db
            .albums_by_artist("Duster")
            .iter()
            .map(|a| a.songs.len())
            .sum(),
        playing_song: None,
        selected_song: None,
        reset_scroll: false,
    };

    let mut controls = Controls {
        song: None,
        playing: false,
        elapsed: 0.0,
        duration: 0.0,
        volume: player.volume(),
    };

    let mut width = Anim::new(280.0);
    let mut fonts = false;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1184.0, 741.0])
            .with_title("mu"),
        ..Default::default()
    };

    eframe::run_ui_native("mu", options, move |ui, _| {
        let ctx = ui.ctx().clone();

        if !fonts {
            fonts = true;
            let now = Instant::now();
            let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");
            let mut definitions = egui::FontDefinitions::empty();
            definitions.font_data.insert(
                "aptos".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(
                    std::fs::read(path.join("../../../neoui/fonts/Aptos.ttf")).unwrap(),
                )),
            );
            definitions.font_data.insert(
                "cjk".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(
                    std::fs::read(path.join("NotoSansCJK-Subset.otf")).unwrap(),
                )),
            );
            definitions.families.insert(
                egui::FontFamily::Proportional,
                vec!["aptos".to_owned(), "cjk".to_owned()],
            );
            definitions.families.insert(
                egui::FontFamily::Monospace,
                vec!["aptos".to_owned(), "cjk".to_owned()],
            );
            ctx.set_fonts(definitions);
            ctx.set_pixels_per_point(1.0);
            println!("Loaded Font in {}ms", now.elapsed().as_millis());
        }

        let keys: Vec<egui::Key> = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|event| match event {
                    egui::Event::Key {
                        key,
                        pressed: true,
                        repeat: false,
                        ..
                    } => Some(key.to_owned()),
                    _ => None,
                })
                .collect()
        });

        for key in keys {
            match key {
                egui::Key::Escape => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                egui::Key::Num1 => sidebar.selected_mode = "Library",
                egui::Key::Num2 => sidebar.selected_mode = "Queue",
                egui::Key::Num3 => sidebar.selected_mode = "Playlist",
                egui::Key::Num4 => sidebar.selected_mode = "Settings",
                egui::Key::Tab => sidebar.active = !sidebar.active,
                egui::Key::W => {
                    player.volume_up();
                    controls.volume = player.volume();
                }
                egui::Key::S => {
                    player.volume_down();
                    controls.volume = player.volume();
                }
                egui::Key::E => player.seek_forward(10.0),
                egui::Key::Q => player.seek_backward(10.0),
                egui::Key::A if let Some((artist, ai, si)) = &mut controls.song => {
                    if *si > 0 {
                        *si = si.saturating_sub(1);
                    } else if *ai > 0 {
                        *ai = ai.saturating_sub(1);
                        *si = db.albums_by_artist(artist)[*ai]
                            .songs
                            .len()
                            .saturating_sub(1);
                    }

                    let song = &db.albums_by_artist(artist)[*ai].songs[*si];
                    player.play_song(&song.path, Some(song.gain), true);
                    if &library.artist == artist {
                        library.playing_song = Some((*ai, *si));
                    }
                }
                egui::Key::D if let Some((artist, ai, si)) = &mut controls.song => {
                    let current_len = db.albums_by_artist(artist)[*ai]
                        .songs
                        .len()
                        .saturating_sub(1);
                    let album_len = db.albums_by_artist(artist).len();

                    if *si < current_len {
                        *si += 1;
                    } else if *ai < album_len {
                        *ai += 1;
                        *si = 0;
                    }

                    let song = &db.albums_by_artist(artist)[*ai].songs[*si];
                    player.play_song(&song.path, Some(song.gain), true);
                    if &library.artist == artist {
                        library.playing_song = Some((*ai, *si));
                    }
                }
                egui::Key::Space => {
                    player.toggle_playback();
                    controls.playing = !controls.playing;
                }
                _ => {}
            }

            //Sidebar jumps on alphabet key press.
            if sidebar.hovered
                && key.name().len() == 1
                && let Some(letter) = key.name().chars().next()
                && letter.is_ascii_alphabetic()
            {
                sidebar.jump_to_letter = Some(letter);
            }
        }

        if let Some(handle) = &artwork_task
            && handle.is_finished()
        {
            let (artist, artwork) = artwork_task.take().unwrap().join().unwrap();
            for (album, image) in artwork {
                let texture = ctx.load_texture(
                    format!("{artist}/{album}"),
                    image,
                    egui::TextureOptions {
                        mipmap_mode: Some(egui::TextureFilter::Linear),
                        ..egui::TextureOptions::LINEAR
                    },
                );
                textures.insert((artist.clone(), album), texture);
            }
        }

        //It's not as immediate, but easier than passing in db and library into sidebar.
        if sidebar.update_library {
            sidebar.update_library = false;
            sidebar.selected_mode = "Library";
            library.playing_song = None;
            library.selected_song = None;

            let artist = sidebar.selected_artist.clone();

            //Restore the playing song state.
            if let Some((a, ai, si)) = &controls.song
                && a == &artist
            {
                library.playing_song = Some((*ai, *si));
            }

            let albums: Vec<(String, String)> = db
                .albums_by_artist(&artist)
                .iter()
                .filter(|album| !textures.contains_key(&(artist.clone(), album.title.clone())))
                .filter_map(|album| {
                    album
                        .songs
                        .first()
                        .map(|song| (album.title.clone(), song.path.clone()))
                })
                .collect();

            if !albums.is_empty() {
                artwork_task = Some(spawn_load_artwork(artist.clone(), albums));
            }

            library.reset_scroll = true;
            library.artist = artist;
            library.total_tracks = db
                .albums_by_artist(&library.artist)
                .iter()
                .map(|a| a.songs.len())
                .sum();
        }

        controls.duration = player.duration().as_secs_f32();
        controls.elapsed = player.elapsed().as_secs_f32();

        let dt = ctx.input(|i| i.stable_dt);

        let screen = ui.max_rect();
        ui.painter().rect_filled(screen, CornerRadius::ZERO, BODY);

        let target = if sidebar.active { 280.0 } else { 56.0 };
        let sidebar_width = width.update(target, 0.15, dt, true).round();
        let bounds = Rect::from_min_size(screen.min, vec2(sidebar_width, screen.height()));
        let body = Rect::from_min_max(
            pos2(screen.left() + sidebar_width, screen.top()),
            screen.max,
        );

        sidebar.hovered = ui
            .ctx()
            .pointer_latest_pos()
            .is_some_and(|pos| bounds.contains(pos));

        if sidebar_width > 168.0 {
            draw_sidebar(ui, bounds, &mut sidebar, dt);
        } else {
            draw_rail(ui, bounds, &mut sidebar);
        }

        let controls_bounds = Rect::from_min_max(pos2(body.left(), body.bottom() - 84.0), body.max);
        let body = Rect::from_min_max(body.min, pos2(body.right(), body.bottom() - 84.0));

        match sidebar.selected_mode {
            "Library" => draw_library(
                ui,
                body,
                db.albums_by_artist(&library.artist),
                &mut library,
                &mut controls,
                &mut player,
                &textures,
            ),
            _ => {
                ui.painter().rect_filled(body, CornerRadius::ZERO, BODY);
            }
        }

        draw_controls(
            ui,
            controls_bounds,
            &mut controls,
            &mut player,
            &db,
            &textures,
        );

        let fading = sidebar_width > 168.0 && sidebar.fade.current != sidebar.fade.target;
        if width.current != width.target || fading {
            ctx.request_repaint();
        } else if artwork_task.is_some() {
            ctx.request_repaint_after(Duration::from_millis(50));
        } else if controls.playing {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    })
    .unwrap();
}
