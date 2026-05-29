use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::palette::{magnitude_to_color_themed, ColorDepth};
use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

const DB_MIN: f32 = -120.0;
const DB_MAX: f32 = 0.0;

pub struct WaterfallPanel;

impl WaterfallPanel {
    pub fn new() -> Self { Self }
}

impl Panel for WaterfallPanel {
    fn name(&self) -> &'static str { "waterfall" }
    fn min_size(&self) -> (u16, u16) { (40, 5) }
    fn focus_key(&self) -> Option<char> { Some('o') }
    fn focus_bindings(&self) -> &'static [(&'static str, &'static str)] {
        &[("W", "Pause/Resume"), ("Esc", "Exit focus")]
    }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let buf = &state.waterfall;
        let title = if buf.paused { " Waterfall [PAUSED] " } else { " Waterfall " };
        let border_color = if focused { theme.border_focused }
            else if buf.paused { theme.stale }
            else { theme.border_accent };

        if buf.rows.is_empty() {
            f.render_widget(
                Paragraph::new("Waiting for RX\u{2026}")
                    .block(
                        Block::default()
                            .title(title)
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(border_color)),
                    )
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(theme.label)),
                area,
            );
            return;
        }

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        f.render_widget(block, area);

        // Offset matches spectrum panel's dB-label column so both panels share the same x-axis
        const DB_COL: u16 = 6;
        let wf_area = Rect {
            x: inner.x + DB_COL,
            y: inner.y,
            width: inner.width.saturating_sub(DB_COL),
            height: inner.height,
        };
        let cols = wf_area.width as usize;
        if cols == 0 { return; }

        let rows_to_show = wf_area.height as usize;
        let depth = ColorDepth::detect();
        let mut lines: Vec<Line> = Vec::with_capacity(rows_to_show);

        for row_data in buf.rows.iter().take(rows_to_show) {
            let n = row_data.len();
            let mut spans: Vec<Span> = Vec::with_capacity(cols);
            for col in 0..cols {
                let bin_start = col * n / cols;
                let bin_end = (((col + 1) * n) / cols).max(bin_start + 1).min(n);
                let db = row_data[bin_start..bin_end]
                    .iter()
                    .cloned()
                    .fold(f32::NEG_INFINITY, f32::max);
                let color = magnitude_to_color_themed(db, DB_MIN, DB_MAX, depth, theme);
                spans.push(Span::styled(" ", Style::default().bg(color)));
            }
            lines.push(Line::from(spans));
        }

        f.render_widget(Paragraph::new(lines), wf_area);

        // dBFS color scale legend in the left column
        let legend_area = Rect { x: inner.x, y: inner.y, width: DB_COL, height: inner.height };
        let h = legend_area.height as usize;
        if h > 0 {
            let mut legend: Vec<Line> = Vec::with_capacity(h);
            for row in 0..h {
                let t = row as f32 / (h.saturating_sub(1)).max(1) as f32;
                let db = DB_MAX + (DB_MIN - DB_MAX) * t;
                let bar_color = magnitude_to_color_themed(db, DB_MIN, DB_MAX, depth, theme);
                let label = match row {
                    0 => format!("{:>+4} ", DB_MAX as i32),
                    r if r == h / 2 => format!("{:>+4} ", ((DB_MAX + DB_MIN) / 2.0) as i32),
                    r if r == h.saturating_sub(1) => format!("{:>+4} ", DB_MIN as i32),
                    _ => "     ".to_string(),
                };
                legend.push(Line::from(vec![
                    Span::styled("█", Style::default().fg(bar_color)),
                    Span::styled(label, Style::default().fg(theme.label)),
                ]));
            }
            f.render_widget(Paragraph::new(legend), legend_area);
        }
    }
}
