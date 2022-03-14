use gonk_database::Colors;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

#[derive(Default)]
pub struct NewSearch {}

impl NewSearch {
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>, colors: &Colors) {
        let area = f.size();

        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Percentage(40),
                    Constraint::Percentage(60),
                ]
                .as_ref(),
            )
            .split(area);

        let h = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
            .split(v[1]);

        self.draw_textbox(f, v[0]);

        self.draw_main_result(f, h[0]);
        self.draw_associated_result(f, h[1]);

        self.draw_other_results(f, v[2], colors);

        self.handle_cursor(f);
    }
    fn handle_cursor<B: Backend>(&self, f: &mut Frame<B>) {
        // let area = f.size();
        ////Move the cursor position when typing
        // if let SearchMode::Search = self.mode {
        //     if self.results.is_none() && self.query.is_empty() {
        //         f.set_cursor(1, 1);
        //     } else {
        //         let mut len = self.query_len();
        //         if len > area.width {
        //             len = area.width;
        //         }
        //         f.set_cursor(len + 1, 1);
        //     }
        // }
        f.set_cursor(("badbadnotgood".len() + 1) as u16, 1);
    }
    fn draw_textbox<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let p = Paragraph::new(vec![Spans::from("badbadnotgood")])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .alignment(Alignment::Left);
        f.render_widget(p, area);
    }
    fn draw_main_result<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let p = Paragraph::new(vec![
            Spans::from(Span::styled(
                "Badbadnotgood",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Spans::from(""),
            Spans::from(Span::styled(
                "Artist",
                Style::default().add_modifier(Modifier::DIM | Modifier::ITALIC),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .alignment(Alignment::Left);
        f.render_widget(p, area);
    }
    fn draw_associated_result<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let h = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(area);

        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(h[0]);

        let v1 = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(h[1]);

        let p = Paragraph::new(vec![Spans::from("Test")])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .alignment(Alignment::Left);

        f.render_widget(p.clone(), v[0]);
        f.render_widget(p.clone(), v[1]);
        f.render_widget(p.clone(), v1[0]);
        f.render_widget(p.clone(), v1[1]);
    }
    fn draw_other_results<B: Backend>(&self, f: &mut Frame<B>, area: Rect, colors: &Colors) {
        let items = vec![
            Row::new(vec![
                Cell::from(" Name").style(Style::default().fg(colors.title)),
                Cell::from(" Album").style(Style::default().fg(colors.album)),
                Cell::from(" Artist").style(Style::default().fg(colors.artist)),
            ]),
            Row::new(vec![Cell::from("")]),
            Row::new(vec![
                Cell::from(" Name").style(Style::default().fg(colors.title)),
                Cell::from(" Album").style(Style::default().fg(colors.album)),
                Cell::from(" Artist").style(Style::default().fg(colors.artist)),
            ]),
        ];

        let t = Table::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .widths(&[
                Constraint::Percentage(43),
                Constraint::Percentage(29),
                Constraint::Percentage(27),
            ])
            .highlight_symbol("> ");

        f.render_widget(t, area);
    }
}
//The big item

//if the first result is an:
//artist - display the artist and their top songs on the side
//album - display the album and the songs in it
//songs - the main song and the next songs from that album

//display all other results bellow the main section
