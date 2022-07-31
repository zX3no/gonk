use tui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::{Block, Widget},
};

#[derive(Debug, Clone)]
pub struct Gauge<'a> {
    block: Option<Block<'a>>,
    ratio: f64,
    label: Option<Span<'a>>,
    style: Style,
    gauge_style: Style,
}

impl<'a> Default for Gauge<'a> {
    fn default() -> Gauge<'a> {
        Gauge {
            block: None,
            ratio: 0.0,
            label: None,
            style: Style::default(),
            gauge_style: Style::default(),
        }
    }
}

impl<'a> Gauge<'a> {
    pub fn block(mut self, block: Block<'a>) -> Gauge<'a> {
        self.block = Some(block);
        self
    }

    pub fn ratio(mut self, ratio: f64) -> Gauge<'a> {
        assert!(
            (0.0..=1.0).contains(&ratio),
            "Ratio should be between 0 and 1 inclusively."
        );
        self.ratio = ratio;
        self
    }

    pub fn label<T>(mut self, label: T) -> Gauge<'a>
    where
        T: Into<Span<'a>>,
    {
        self.label = Some(label.into());
        self
    }

    pub fn style(mut self, style: Style) -> Gauge<'a> {
        self.style = style;
        self
    }

    pub fn gauge_style(mut self, style: Style) -> Gauge<'a> {
        self.gauge_style = style;
        self
    }
}

impl<'a> Widget for Gauge<'a> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, self.style);
        let gauge_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };
        buf.set_style(gauge_area, self.gauge_style);
        if gauge_area.height < 1 {
            return;
        }

        // compute label value and its position
        // label is put at the center of the gauge_area
        let label = {
            let pct = f64::round(self.ratio * 100.0);
            self.label
                .unwrap_or_else(|| Span::from(format!("{}%", pct)))
        };
        let clamped_label_width = gauge_area.width.min(label.width() as u16);
        let label_col = gauge_area.left() + (gauge_area.width - clamped_label_width) / 2;
        let label_row = gauge_area.top() + gauge_area.height / 2;

        // the gauge will be filled proportionally to the ratio
        let filled_width = f64::from(gauge_area.width) * self.ratio;
        let end = gauge_area.left() + filled_width.round() as u16;
        for y in gauge_area.top()..gauge_area.bottom() {
            // render the filled area (left to end)
            for x in gauge_area.left()..end {
                // spaces are needed to apply the background styling
                buf.get_mut(x, y)
                    .set_symbol(" ")
                    .set_fg(self.gauge_style.bg.unwrap_or(Color::Black))
                    .set_bg(self.gauge_style.fg.unwrap_or(Color::Reset));
            }
        }
        // set the span
        buf.set_span(label_col, label_row, &label, clamped_label_width);
    }
}
