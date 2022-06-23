use crate::{Frame, Input};
use tui::layout::Rect;

//TODO

#[derive(Default)]
pub struct Settings {}

impl Settings {}

impl Input for Settings {
    fn up(&mut self) {}

    fn down(&mut self) {}

    fn left(&mut self) {}

    fn right(&mut self) {}
}

#[allow(unused)]
pub fn draw(settings: &mut Settings, area: Rect, f: &mut Frame) {}
