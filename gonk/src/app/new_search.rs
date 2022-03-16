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
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
            .split(v[1]);

        self.draw_textbox(f, v[0]);

        self.draw_main_result_artist(f, h[0]);
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
        f.set_cursor(("badbad".len() + 1) as u16, 1);
    }
    fn draw_textbox<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let p = Paragraph::new(vec![Spans::from("badbad")])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .alignment(Alignment::Left);
        f.render_widget(p, area);
    }
    fn draw_main_result_artist<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let p = Paragraph::new(vec![
            Spans::from(Span::styled(
                "BADBADNOTGOOD ",
                Style::default().add_modifier(Modifier::ITALIC),
            )),
            Spans::from(""),
            Spans::from("IV"),
            Spans::from("Talk Memory"),
            Spans::from("Test Album"),
            // Spans::from(Span::styled(
            //     "IV ",
            //     Style::default().add_modifier(Modifier::ITALIC),
            // )),
            // Spans::from(Span::styled(
            //     "Talk Memory ",
            //     Style::default().add_modifier(Modifier::ITALIC),
            // )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Artist"),
        )
        .alignment(Alignment::Left);
        f.render_widget(p, area);
    }
    fn draw_associated_result<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let h = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(area);

        let p = Paragraph::new(vec![
            Spans::from(Span::styled(
                "IV ",
                Style::default().add_modifier(Modifier::ITALIC),
            )),
            Spans::from(""),
            Spans::from("1. Test Song"),
            Spans::from("2. Test Song"),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Album"),
        )
        .alignment(Alignment::Left);

        let p1 = Paragraph::new(vec![
            Spans::from(Span::styled(
                "Talk Memory ",
                Style::default().add_modifier(Modifier::ITALIC),
            )),
            Spans::from(""),
            Spans::from("1. Track One"),
            Spans::from("2. Track Two"),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Album"),
        )
        .alignment(Alignment::Left);

        f.render_widget(p, h[0]);
        f.render_widget(p1, h[1]);
    }
    fn draw_other_results<B: Backend>(&self, f: &mut Frame<B>, area: Rect, _colors: &Colors) {
        // let items = vec![
        //     Row::new(vec![
        //         Cell::from(" Name").style(Style::default().fg(colors.title)),
        //         Cell::from(" Album").style(Style::default().fg(colors.album)),
        //         Cell::from(" Artist").style(Style::default().fg(colors.artist)),
        //     ]),
        //     Row::new(vec![Cell::from("")]),
        //     Row::new(vec![
        //         Cell::from(" Name").style(Style::default().fg(colors.title)),
        //         Cell::from(" Album").style(Style::default().fg(colors.album)),
        //         Cell::from(" Artist").style(Style::default().fg(colors.artist)),
        //     ]),
        // ];

        let items = vec![
            Row::new(vec![Cell::from("BADBADNOTGOOD - Artist")
                .style(Style::default().add_modifier(Modifier::ITALIC))]),
            Row::new(vec![
                Cell::from("Bald Head Girl"),
                Cell::from("Steroids (Crouching Tiger Hidden"),
                Cell::from("Death Grips"),
            ]),
            Row::new(vec![
                Cell::from("BALD!"),
                Cell::from("EP"),
                Cell::from("JEPGMAFIA"),
            ]),
            Row::new(vec![
                Cell::from("BALD!"),
                Cell::from("LP!"),
                Cell::from("JPEGMAFIA"),
            ]),
        ];

        let t = Table::new(items)
            .header(
                Row::new(vec![
                    Cell::from("Name"),
                    Cell::from("Album"),
                    Cell::from("Artist"),
                ])
                .bottom_margin(1),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("Results"),
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
