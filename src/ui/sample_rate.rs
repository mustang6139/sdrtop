use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
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

pub struct SampleRatePanel;

/// Returns the smallest "nice" Msps step that fits ≤ canvas_height/2 ticks.
fn nice_step_msps(range_msps: f64, canvas_height: usize) -> f64 {
    let max_ticks = ((canvas_height / 2).max(1)) as f64;
    let min_step = (range_msps / max_ticks).max(0.001);
    // Extended to cover large ranges (e.g. if IDLE zeros contaminate)
    for &s in &[0.01_f64, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0] {
        if s >= min_step { return s; }
    }
    10.0
}

/// Renders the sample-rate Braille graph into `area` (no outer Block).
/// Uses the full `area` width — caller is responsible for positioning.
pub(crate) fn draw_sr_graph(
    f: &mut Frame,
    area: Rect,
    state: &SdrMetrics,
    theme: &crate::Theme,
) {
    if area.height < 3 || area.width < 8 { return; }

    let cfg_msps = state.radio.config_sample_rate / 1_000_000.0;

    let all_values: Vec<f64> = state.radio.sample_rate_history.iter()
        .map(|&sps| sps as f64 / 1_000_000.0)
        .collect();
    let streaming_values: Vec<f64> = all_values.iter().cloned()
        .filter(|&v| v > 0.1)
        .collect();

    let (min_msps, max_msps) = if streaming_values.is_empty() {
        (cfg_msps * 0.999, cfg_msps * 1.001)
    } else {
        let lo = streaming_values.iter().cloned().fold(f64::MAX, f64::min);
        let hi = streaming_values.iter().cloned().fold(0.0_f64, f64::max);
        (lo, hi)
    };
    let range = (max_msps - min_msps).max(0.0);

    let canvas_height = (area.height - 1) as usize;
    let step = nice_step_msps(range, canvas_height);
    let y_min = (min_msps / step).floor() * step;
    let y_max = ((max_msps / step).ceil() * step).max(y_min + step);
    let y_range = (y_max - y_min).max(1e-9);

    let y_axis_w = 6u16;

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
    let mut v = y_max;
    loop {
        let frac = (v - y_min) / y_range;
        let row = ((1.0 - frac) * ch as f64).round() as usize;
        if row < ch {
            y_lines[row] = Line::from(vec![
                Span::styled(format!("{:5.2}", v), Style::default().fg(theme.label)),
                Span::styled("┤", Style::default().fg(theme.border_default)),
            ]);
        }
        v -= step;
        if v < y_min - step * 0.01 { break; }
    }
    y_lines[ch] = Line::from(vec![
        Span::styled(format!("{:5.2}", y_min), Style::default().fg(theme.label)),
        Span::styled("└", Style::default().fg(theme.border_default)),
    ]);
    f.render_widget(Paragraph::new(y_lines), y_axis_area);

    let n = all_values.len();
    let max_n = THROUGHPUT_HISTORY_LEN as f64;
    let x_offset = max_n - n as f64;
    let canvas_w_braille = canvas_area.width as f64 * 2.0;
    let x_step = (max_n / canvas_w_braille).max(0.05);
    let graph_color = theme.observer;
    let y_min_c = y_min;
    let y_max_c = y_max;

    f.render_widget(
        Canvas::default()
            .x_bounds([0.0, max_n])
            .y_bounds([y_min_c, y_max_c])
            .paint(move |ctx| {
                for (i, &val) in all_values.iter().enumerate() {
                    let draw_val = if val < 0.1 { y_min_c } else { val };
                    let bar_left  = x_offset + i as f64;
                    let bar_right = x_offset + (i + 1) as f64;
                    let mut x = bar_left;
                    while x <= bar_right {
                        ctx.draw(&CanvasLine { x1: x, y1: y_min_c, x2: x, y2: draw_val, color: graph_color });
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

impl Panel for SampleRatePanel {
    fn name(&self) -> &'static str { "sample_rate" }
    fn min_size(&self) -> (u16, u16) { (40, 4) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, _focused: bool) {
        let cfg_msps = state.radio.config_sample_rate / 1_000_000.0;
        let block = Block::default()
            .title(format!(" SR  {:.2}  Msps ", cfg_msps))
            .title_alignment(Alignment::Right)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_dim));
        let inner = block.inner(area);
        f.render_widget(block, area);
        if inner.height < 3 || inner.width < 15 { return; }
        let graph_w = inner.width.min(50);
        let left_pad = inner.width - graph_w;
        let graph_rect = Rect { x: inner.x + left_pad, y: inner.y, width: graph_w, height: inner.height };
        draw_sr_graph(f, graph_rect, state, theme);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nice_step_msps_narrow_range_small_canvas() {
        assert_eq!(nice_step_msps(0.04, 4), 0.02);
    }

    #[test]
    fn nice_step_msps_wider_range() {
        assert_eq!(nice_step_msps(0.5, 4), 0.5);
    }

    #[test]
    fn nice_step_msps_zero_range() {
        assert_eq!(nice_step_msps(0.0, 4), 0.01);
    }

    #[test]
    fn nice_step_msps_tall_canvas() {
        assert_eq!(nice_step_msps(0.04, 10), 0.01);
    }

    #[test]
    fn nice_step_msps_large_range_covers_extended_steps() {
        // range=2.0, 4 rows → max_ticks=2, min_step=1.0 → step=1.0
        assert_eq!(nice_step_msps(2.0, 4), 1.0);
        // range=5.0, 4 rows → min_step=2.5 → step=5.0
        assert_eq!(nice_step_msps(5.0, 4), 5.0);
    }
}
