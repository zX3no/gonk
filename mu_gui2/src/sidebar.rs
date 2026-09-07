use crate::*;

pub struct Sidebar<'a> {
    pub bounds: Rect,
    pub artists: &'a [String],
    pub selected_artist: &'a str,
    pub selected_mode: &'a str,
    pub current_letter: Option<char>,
    pub active: bool,
    pub update_library: bool,
    pub artist_scroll: Scroll,
    pub jump_to_letter: Option<char>,
}

pub fn draw_rail(sidebar: &mut Sidebar, ui: &mut FrameContext) {
    ui.flow_down(
        flow()
            .bounds(sidebar.bounds)
            .bg(SIDEBAR)
            .border(BORDER_DIM)
            .border_side(RIGHT)
            .padtb(14)
            .padlr(11)
            .gap(4),
        |ui| {
            let btn = rect()
                .wh(34)
                .pad(7)
                .radius(8)
                .hover(ROW_HOVER)
                .selected(ROW_SELECTED);

            if icon(ui, "Panel", btn.fg(TEXT_TERTIARY)).clicked {
                sidebar.active = true;
            }

            ui.gap(6);

            for mode in ["Library", "Queue", "Playlist", "Settings"] {
                let selected = mode == sidebar.selected_mode;
                let btn = btn
                    .is_selected(selected)
                    .fg(if selected { TEXT } else { TEXT_TERTIARY });
                if icon(ui, mode, btn).clicked {
                    sidebar.selected_mode = mode;
                }
            }

            ui.gap(10);
            ui.rect(rect().height(1).width(Size::Fill).bg(BORDER_DIM));
        },
    );
}

pub fn draw_sidebar<'a, 'b: 'a>(
    sidebar: &mut Sidebar<'b>,
    ui: &mut FrameContext<'_, 'a>,
) {
    let sb = text().fg(TEXT).font_size(16);
    let state = ui.flow_down(
        flow()
            .bounds(sidebar.bounds)
            .bg(SIDEBAR)
            .border(BORDER_DIM)
            .border_side(RIGHT),
        |ui| {
            ui.flow_right(
                flow()
                    .padtb(20)
                    .padl(18)
                    .padr(10)
                    .height(48)
                    .children_center(),
                |ui| {
                    ui.text("mu", sb);
                    ui.gap(-28);
                    let btn = rect()
                        .wh(30)
                        .pad(5)
                        .radius(6)
                        .hover(ROW_HOVER)
                        .fg(TEXT_TERTIARY);
                    if icon(ui, "Panel", btn.fg(TEXT_TERTIARY)).clicked {
                        sidebar.active = false;
                    }
                },
            );

            ui.flow_down(flow().gap(2).padlr(8), |ui| {
                let mut item = |t: &'static str, i: &'static str| {
                    //TODO: Should use impl IntoColor to allow for Option or u32.
                    // sel.bg(if s { Some(ROW_SELECTED) } else { None });
                    let selected = t == sidebar.selected_mode;
                    let mut sel = flow().padlr(12).padtb(8).radius(6).hover(ROW_HOVER);
                    sel.paint.bg = if selected { Some(ROW_SELECTED) } else { None };
                    let text = sb.fg(if selected { TEXT } else { TEXT_TERTIARY });
                    let ntext = sb
                        .fg(if selected { TEXT_MUTED } else { TEXT_FAINT })
                        .fillw()
                        .content_right();

                    if ui
                        .flow_right(sel, |ui| {
                            ui.text(t, text);
                            ui.text(i, ntext);
                        })
                        .clicked
                    {
                        sidebar.selected_mode = t;
                    }
                };

                item("Library", "1");
                item("Queue", "2");
                item("Playlist", "3");
                item("Settings", "4");
                ui.gap(8);
            });

            ui.rect(rect().height(1).width(Size::Fill).bg(BORDER_DIM));

            let (artist, mut alphabet) = ui.split_h(-30);
            let selected_artist = sidebar.selected_artist;
            let top_of_artist_view = artist.y;
            let jump_target = sidebar.jump_to_letter.take();
            let mut jump_offset = None;

            let scroll_state = ui.scroll(
                flow().bounds(artist).padlr(8).elastic(true),
                &mut sidebar.artist_scroll,
                |ui| {
                    //Assuming artists is pre sorted alphabetically.
                    let mut first_letter = ' ';
                    // let mut top_letter = None;
                    sidebar.current_letter = None;
                    let text = sb
                        .padlr(12)
                        .padtb(8)
                        .radius(6)
                        .content_left()
                        .fillw()
                        .hover(ROW_HOVER);
                    let selected_text = text.bg(ROW_SELECTED);

                    for artist in sidebar.artists {
                        let next = artist.chars().next().unwrap().to_ascii_uppercase();
                        if next != first_letter {
                            first_letter = next;
                            let l = sb.padlr(12).padtb(8).font_size(12).fg(TEXT_MUTED);
                            let frame = ui.current_frame();
                            if sidebar.current_letter.is_none()
                                || frame.cursor_y - frame.scroll_y <= top_of_artist_view
                            {
                                sidebar.current_letter = Some(first_letter);
                            }
                            if let Some(target) = jump_target
                                && first_letter == target.to_ascii_uppercase()
                                && jump_offset.is_none()
                            {
                                jump_offset = Some((frame.cursor_y - frame.inner_bounds.y) as f32);
                            }
                            ui.text(first_letter.to_string(), l);
                        }
                        let sel = artist.as_str() == selected_artist;
                        let state = ui.text(artist, if sel { selected_text } else { text });
                        if state.clicked {
                            sidebar.selected_artist = artist;
                            sidebar.update_library = true;
                        }
                    }
                },
            );

            if let Some(offset) = jump_offset {
                //TODO: This should not allow for jumping out of bounds.
                //TODO: Should also have some momentum when jumping around.
                //Currently just a fixed jump.
                sidebar.artist_scroll.jump(offset);
            }

            ui.paint_rect(alphabet, rect().border(BORDER_DIM).border_side(LEFT));

            if let Some(raw_pct) = ui.drag_percentage_y(alphabet) {
                let pct = ((raw_pct - 0.03) / 0.90).clamp(0.0, 1.0);
                sidebar
                    .artist_scroll
                    .jump(pct * scroll_state.max_scroll as f32);
            }

            let hovered = ui.hovered(alphabet);
            let fade = ui.animate_f32(if hovered { 1.0 } else { 0.0 }, 0.15, Ease::InOutSine);
            let my = ui.mouse_position().y;
            let glow = |a: f32| rgba(155, 132, 217, (a * fade * 255.0) as u8);
            if fade > 0.0 {
                ui.place_down(flow().bounds(alphabet).clip(true), |ui| {
                    ui.gradient(
                        rect()
                            .x(alphabet.x)
                            .y(my.saturating_sub(55))
                            .width(alphabet.width)
                            .height(110),
                        180.0,
                    )
                    .stop(0.0, glow(0.0))
                    .stop(0.21, glow(0.11))
                    .stop(0.5, glow(0.30))
                    .stop(0.79, glow(0.11))
                    .stop(1.0, glow(0.0));

                    ui.gradient(rect().x(alphabet.x).y(my - 70).width(1).height(140), 180.0)
                        .stop(0.0, glow(0.0))
                        .stop(0.5, rgba(199, 183, 240, (0.75 * fade * 255.0) as u8))
                        .stop(1.0, glow(0.0));
                });
            }
            alphabet.x += 12;
            ui.flow_down(flow().bounds(alphabet), |ui| {
                let row = ui
                    .measure_text("A", Font::default(), 10, None, i32::MAX)
                    .height;
                ui.gap((ui.current_frame_bounds().height - row * 26) / 2);

                let Some(current_letter) = sidebar.current_letter else {
                    return;
                };

                for &letter in ALPHABET {
                    let ch = letter.chars().next().unwrap();
                    let dist = (ch as i32 - current_letter as i32).abs();
                    let color = match dist {
                        0 => ACCENT,
                        // 1 => TEXT,
                        _ => TEXT_TERTIARY,
                    };
                    ui.text(letter, sb.font_size(10).fg(color));
                }
            });
        },
    );

    if state.hovered {
        for key in ui.window.pressed_keys() {
            match key {
                Key::Char(c) => sidebar.jump_to_letter = Some(*c),
                _ => {}
            }
        }
    }
}