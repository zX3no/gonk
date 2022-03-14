use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, BorderType, Borders},
    Frame,
};

#[derive(Default)]
pub struct NewSearch {}

impl NewSearch {
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>) {
        let area = f.size();

        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
            .split(area);

        let h = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
            .split(v[0]);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        f.render_widget(block.clone(), h[0]);
        f.render_widget(block.clone(), h[1]);

        f.render_widget(block.clone(), v[1]);
    }
}
//The big item

//if the first result is an:
//artist - display the artist and their top songs on the side
//album - display the album and the songs in it
//songs - the main song and the next songs from that album

//display all other results bellow the main section
