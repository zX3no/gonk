use crate::app::{App, Mode};

#[allow(unused_imports)]
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{
        Block, BorderType, Borders, Cell, Gauge, List, ListItem, Paragraph, Row, Table, Wrap,
    },
    Frame,
};

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .constraints([Constraint::Percentage(95), Constraint::Percentage(10)].as_ref())
        .split(f.size());

    let browser_chunks = match app.mode {
        Mode::Search => chunks.clone(),
        _ => Layout::default()
            .constraints([Constraint::Ratio(1, 1)].as_ref())
            .split(f.size()),
    };
    // let block = Block::default()
    //     .title("Block")
    //     .borders(Borders::ALL)
    //     .border_style(Style::default().fg(Color::White))
    //     .border_type(BorderType::Rounded)
    //     .style(Style::default());

    // f.render_widget(block.clone(), chunks[0]);
    // f.render_widget(block.clone(), chunks[1]);

    draw_browser(f, app, browser_chunks[0]);
    draw_queue(f, app, chunks[0]);

    draw_seeker(f, app, chunks[1]);

    match app.mode {
        Mode::Search => draw_search(f, app, chunks[1]),
        _ => (),
    }
}

fn draw_seeker<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 6), Constraint::Ratio(1, 2)])
        .split(area);

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("20 - Example Track"),
        )
        .gauge_style(
            Style::default()
                .fg(Color::White)
                .bg(Color::Black)
                .add_modifier(Modifier::ITALIC),
        )
        .percent(app.seeker_ratio)
        .label(app.seeker.clone());

    let y = f.size().height - 3;

    f.render_widget(gauge, Rect::new(chunks[1].x, y, chunks[1].width, 3));
}

fn draw_search<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 6), Constraint::Ratio(1, 2)])
        .split(area);

    let text = Spans::from(Span::styled(
        app.query.clone(),
        Style::default().add_modifier(Modifier::ITALIC),
    ));

    let query = Paragraph::new(text)
        .block(Block::default().title("Search").borders(Borders::ALL))
        .style(Style::default())
        .wrap(Wrap { trim: true });

    f.render_widget(query, chunks[0]);
}

fn draw_browser<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 6), Constraint::Ratio(1, 2)])
        .split(area);

    let items = app.browser.get_list_items();

    let list = List::new(items)
        .block(
            Block::default()
                .title(app.browser.title())
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
        .highlight_symbol(">>");

    f.render_stateful_widget(list, chunks[0], app.browser.get_selection());
}

fn draw_queue<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 6), Constraint::Ratio(1, 2)])
        .split(area);

    let items = app.queue.get_list_items();

    let list = List::new(items)
        .block(Block::default().title("List").borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::ITALIC)
                .bg(Color::White)
                .fg(Color::Black),
        )
        .highlight_symbol("> ");
    f.render_stateful_widget(list, chunks[1], app.queue.get_state());
}
