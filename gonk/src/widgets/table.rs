use tui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::Text,
    widgets::{Block, StatefulWidget, Widget},
};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Cell<'a> {
    content: Text<'a>,
    style: Style,
}

impl<'a> Cell<'a> {
    /// Set the `Style` of this cell.
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl<'a, T> From<T> for Cell<'a>
where
    T: Into<Text<'a>>,
{
    fn from(content: T) -> Cell<'a> {
        Cell {
            content: content.into(),
            style: Style::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Row<'a> {
    cells: Vec<Cell<'a>>,
    height: u16,
    style: Style,
    bottom_margin: u16,
}

impl<'a> Row<'a> {
    pub fn new<T>(cells: T) -> Self
    where
        T: IntoIterator,
        T::Item: Into<Cell<'a>>,
    {
        Self {
            height: 1,
            cells: cells.into_iter().map(|c| c.into()).collect(),
            style: Style::default(),
            bottom_margin: 0,
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn bottom_margin(mut self, margin: u16) -> Self {
        self.bottom_margin = margin;
        self
    }

    fn total_height(&self) -> u16 {
        self.height.saturating_add(self.bottom_margin)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Table<'a> {
    /// A block to wrap the widget in
    block: Option<Block<'a>>,
    /// Base style for the widget
    style: Style,
    /// Width constraints for each column
    widths: &'a [Constraint],
    /// Space between each column
    column_spacing: u16,
    /// Style used to render the selected row
    highlight_style: Style,
    /// Symbol in front of the selected rom
    highlight_symbol: Option<&'a str>,
    /// Optional header
    header: Option<Row<'a>>,
    /// Data to display in each row
    rows: Vec<Row<'a>>,
}

impl<'a> Table<'a> {
    pub fn new<T>(rows: T) -> Self
    where
        T: IntoIterator<Item = Row<'a>>,
    {
        Self {
            block: None,
            style: Style::default(),
            widths: &[],
            column_spacing: 1,
            highlight_style: Style::default(),
            highlight_symbol: None,
            header: None,
            rows: rows.into_iter().collect(),
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn header(mut self, header: Row<'a>) -> Self {
        self.header = Some(header);
        self
    }

    pub fn widths(mut self, widths: &'a [Constraint]) -> Self {
        let between_0_and_100 = |&w| match w {
            Constraint::Percentage(p) => p <= 100,
            _ => true,
        };
        assert!(
            widths.iter().all(between_0_and_100),
            "Percentages should be between 0 and 100 inclusively."
        );
        self.widths = widths;
        self
    }

    pub fn highlight_symbol(mut self, highlight_symbol: &'a str) -> Self {
        self.highlight_symbol = Some(highlight_symbol);
        self
    }

    fn get_columns_widths(&self, max_width: u16, has_selection: bool) -> Vec<u16> {
        let mut constraints = Vec::with_capacity(self.widths.len() * 2 + 1);
        if has_selection {
            let highlight_symbol_width = self.highlight_symbol.map(|s| s.len() as u16).unwrap_or(0);
            constraints.push(Constraint::Length(highlight_symbol_width));
        }
        for constraint in self.widths {
            constraints.push(*constraint);
            constraints.push(Constraint::Length(self.column_spacing));
        }
        if !self.widths.is_empty() {
            constraints.pop();
        }
        let mut chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(Rect {
                x: 0,
                y: 0,
                width: max_width,
                height: 1,
            });
        if has_selection {
            chunks.remove(0);
        }
        chunks.iter().step_by(2).map(|c| c.width).collect()
    }

    pub fn get_row_bounds(&self, selected: Option<usize>, terminal_height: u16) -> (usize, usize) {
        let mut real_end = 0;
        let mut height: usize = 0;
        let len = self.rows.len();
        let selection = selected.unwrap_or(0).min(len.saturating_sub(1));

        for item in self.rows.iter() {
            if height + item.height as usize > terminal_height as usize {
                break;
            }
            height += item.height as usize;
            real_end += 1;
        }

        if len <= height {
            (len - height, len)
        } else if height > 0 {
            let half = (height - 1) / 2;

            let (start, end) = if selection <= half {
                (0, real_end)
            } else if height % 2 == 0 {
                (selection - half, (selection + 2) + half)
            } else {
                (selection - half, (selection + 1) + half)
            };

            if end > len {
                (len - height, len)
            } else {
                (start, end)
            }
        } else {
            (0, 0)
        }
    }

    pub fn get_row_height(&self, area: Rect) -> u16 {
        let table_area = match &self.block {
            Some(b) => b.inner(area),
            None => area,
        };
        let mut rows_height = table_area.height;
        if let Some(ref header) = self.header {
            let max_header_height = table_area.height.min(header.total_height());
            rows_height = rows_height.saturating_sub(max_header_height);
        }
        rows_height
    }
}

#[derive(Debug, Clone, Default)]
pub struct TableState {
    selected: Option<usize>,
}

impl TableState {
    pub fn new(index: Option<usize>) -> Self {
        Self { selected: index }
    }
}

impl<'a> StatefulWidget for Table<'a> {
    type State = TableState;

    fn render(mut self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if area.area() == 0 {
            return;
        }
        buf.set_style(area, self.style);
        let table_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };

        let has_selection = state.selected.is_some();
        let columns_widths = self.get_columns_widths(table_area.width, has_selection);
        let highlight_symbol = self.highlight_symbol.unwrap_or("");
        let blank_symbol = " ".repeat(highlight_symbol.len());
        let mut current_height = 0;
        let mut rows_height = table_area.height;

        // Draw header
        if let Some(ref header) = self.header {
            let max_header_height = table_area.height.min(header.total_height());
            buf.set_style(
                Rect {
                    x: table_area.left(),
                    y: table_area.top(),
                    width: table_area.width,
                    height: table_area.height.min(header.height),
                },
                header.style,
            );
            let mut col = table_area.left();
            if has_selection {
                col += (highlight_symbol.len() as u16).min(table_area.width);
            }
            for (width, cell) in columns_widths.iter().zip(header.cells.iter()) {
                render_cell(
                    buf,
                    cell,
                    Rect {
                        x: col,
                        y: table_area.top(),
                        width: *width,
                        height: max_header_height,
                    },
                );
                col += *width + self.column_spacing;
            }
            current_height += max_header_height;
            rows_height = rows_height.saturating_sub(max_header_height);
        }

        // Draw rows
        if self.rows.is_empty() {
            return;
        }
        let (start, end) = self.get_row_bounds(state.selected, rows_height);

        for (i, table_row) in self
            .rows
            .iter_mut()
            .enumerate()
            .skip(start)
            .take(end - start)
        {
            let (row, col) = (table_area.top() + current_height, table_area.left());
            current_height += table_row.total_height();
            let table_row_area = Rect {
                x: col,
                y: row,
                width: table_area.width,
                height: table_row.height,
            };
            buf.set_style(table_row_area, table_row.style);
            let is_selected = state.selected.map(|s| s == i).unwrap_or(false);
            let table_row_start_col = if has_selection {
                let symbol = if is_selected {
                    highlight_symbol
                } else {
                    &blank_symbol
                };
                let (col, _) =
                    buf.set_stringn(col, row, symbol, table_area.width as usize, table_row.style);
                col
            } else {
                col
            };
            let mut col = table_row_start_col;
            for (width, cell) in columns_widths.iter().zip(table_row.cells.iter()) {
                render_cell(
                    buf,
                    cell,
                    Rect {
                        x: col,
                        y: row,
                        width: *width,
                        height: table_row.height,
                    },
                );
                col += *width + self.column_spacing;
            }
            if is_selected {
                buf.set_style(table_row_area, self.highlight_style);
            }
        }
    }
}

fn render_cell(buf: &mut Buffer, cell: &Cell, area: Rect) {
    buf.set_style(area, cell.style);
    for (i, spans) in cell.content.lines.iter().enumerate() {
        if i as u16 >= area.height {
            break;
        }
        buf.set_spans(area.x, area.y + i as u16, spans, area.width);
    }
}

impl<'a> Widget for Table<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = TableState::default();
        StatefulWidget::render(self, area, buf, &mut state);
    }
}
