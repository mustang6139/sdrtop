use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Line as CanvasLine},
        Block, BorderType, Borders, Paragraph,
    },
    Frame,
};

use crate::state::{SdrMetrics, THROUGHPUT_HISTORY_LEN};
use super::panel::Panel;

pub struct ThroughputPanel;

/// Returns the smallest "nice" MB/s step that fits ≤ canvas_height/2 ticks.
fn nice_step_mb(peak_mb: f64, canvas_height: usize) -> f64 {
    let max_ticks = ((canvas_height / 2).max(1)) as f64;
    let min_step = peak_mb / max_ticks;
    for &s in &[0.5_f64, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0] {
        if s >= min_step { return s; }
    }
    100.0
}

/// Renders the USB throughput Braille graph (y-axis + canvas + bottom axis) into `area`.
/// Does NOT draw an outer Block — the caller is responsible for positioning.
pub(crate) fn draw_usb_graph(
    f: &mut Frame,
    area: Rect,
    state: &SdrMetrics,
    theme: &crate::Theme,
) {
    if area.height < 3 || area.width < 8 { return; }

    let peak_kb = state.radio.throughput_history.iter().copied().max().unwrap_or(0);
    let peak_mb = (peak_kb as f64 / 1024.0).max(1.0);
    let canvas_height = (area.height - 1) as usize;
    let step = nice_step_mb(peak_mb, canvas_height);
    let y_max_mb = ((peak_mb * 1.1 / step).ceil() * step).max(step);

    let label_chars = format!("{:.0}", y_max_mb).len();
    let y_axis_w = (label_chars + 1) as u16;

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(y_axis_w), Constraint::Min(1)])
        .split(area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(cols[1]);

    let y_axis_area = cols[0];
    let canvas_area = rows[0];
    let axis_area   = rows[1];

    let ch = canvas_area.height as usize;
    let mut y_lines: Vec<Line<'static>> = vec![Line::from(""); ch + 1];
    let mut v = y_max_mb;
    loop {
        let row = ((1.0 - v / y_max_mb) * ch as f64).round() as usize;
        if row < ch {
            let label = format!("{:>width$.0}", v, width = label_chars);
            y_lines[row] = Line::from(vec![
                Span::styled(label, Style::default().fg(theme.label)),
                Span::styled("┤", Style::default().fg(theme.border_default)),
            ]);
        }
        v -= step;
        if v < step * 0.01 { break; }
    }
    let bottom_label = format!("{:>width$.0}", 0_f64, width = label_chars);
    y_lines[ch] = Line::from(vec![
        Span::styled(bottom_label, Style::default().fg(theme.label)),
        Span::styled("└", Style::default().fg(theme.border_default)),
    ]);
    f.render_widget(Paragraph::new(y_lines), y_axis_area);

    let history: Vec<f64> = state.radio.throughput_history.iter()
        .map(|&kb| kb as f64 / 1024.0)
        .collect();
    let n = history.len();
    let max_n = THROUGHPUT_HISTORY_LEN as f64;
    let x_offset = max_n - n as f64;
    let canvas_w_braille = canvas_area.width as f64 * 2.0;
    let x_step = (max_n / canvas_w_braille).max(0.05);
    let graph_color = theme.status_ok;

    f.render_widget(
        Canvas::default()
            .x_bounds([0.0, max_n])
            .y_bounds([0.0, y_max_mb])
            .paint(move |ctx| {
                for (i, &val) in history.iter().enumerate() {
                    let bar_left  = x_offset + i as f64;
                    let bar_right = x_offset + (i + 1) as f64;
                    let mut x = bar_left;
                    while x <= bar_right {
                        ctx.draw(&CanvasLine { x1: x, y1: 0.0, x2: x, y2: val, color: graph_color });
                        x += x_step;
                    }
                }
            }),
        canvas_area,
    );

    let dash_w = axis_area.width as usize;
    f.render_widget(
        Paragraph::new(Span::styled(
            "─".repeat(dash_w),
            Style::default().fg(theme.border_default),
        )),
        axis_area,
    );
}

impl Panel for ThroughputPanel {
    fn name(&self) -> &'static str { "throughput" }
    fn min_size(&self) -> (u16, u16) { (40, 5) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, _focused: bool) {
        let block = Block::default()
            .title(" USB Throughput ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_dim));
        let inner = block.inner(area);
        f.render_widget(block, area);
        draw_usb_graph(f, inner, state, theme);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nice_step_mb_small_canvas_coarse_step() {
        assert_eq!(nice_step_mb(20.0, 4), 10.0);
    }

    #[test]
    fn nice_step_mb_medium_canvas() {
        assert_eq!(nice_step_mb(20.0, 8), 5.0);
    }

    #[test]
    fn nice_step_mb_large_canvas() {
        assert_eq!(nice_step_mb(20.0, 20), 2.0);
    }

    #[test]
    fn nice_step_mb_low_throughput() {
        assert_eq!(nice_step_mb(5.0, 4), 5.0);
    }

    #[test]
    fn nice_step_mb_idle_peak() {
        assert_eq!(nice_step_mb(0.1, 4), 0.5);
    }
}
