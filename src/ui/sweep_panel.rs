//! `sweep_panel` — the frequency-scanner display for the `lab_sweep` preset.
//!
//! Renders the latest completed `SweepFrame` as a vertical dBFS bar plot with the
//! sweep band on the x-axis, a band-plan label row underneath, and a status line.
//! The cursor and peak/mean toggle are driven from the panel's focus mode.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::band_plan::{band_at, BAND_PLAN};
use crate::ui::panel::Panel;

pub struct SweepPanel;

/// dBFS window for the vertical axis.
const Y_MIN: f32 = -100.0;
const Y_MAX: f32 = 0.0;
/// Width of the left dBFS-label gutter.
const AXIS_W: u16 = 5;

impl Panel for SweepPanel {
    fn name(&self) -> &'static str { "sweep_panel" }
    fn min_size(&self) -> (u16, u16) { (40, 10) }
    fn focus_key(&self) -> Option<char> { Some('g') }
    fn focus_bindings(&self) -> &'static [(&'static str, &'static str)] {
        &[("←/→", "Cursor"), ("S/E", "Start/End"), ("M", "Peak/Mean"), ("+/-", "Dwell"), ("Enter", "Tune here")]
    }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let sw = &state.sweep;
        let border = if focused { theme.border_focused } else { theme.border_default };

        // Title: the scanner band + step + dwell + cycle, with 'G' as the focus key.
        let key_style = Style::default().fg(theme.value_hi).add_modifier(Modifier::BOLD);
        let step_mhz = sw.config.effective_step_hz(state.radio.config_sample_rate) as f64 / 1e6;
        let title = Line::from(vec![
            Span::raw(" Sweep ["),
            Span::styled("G", key_style),
            Span::styled(
                format!(
                    "]  {:.1}–{:.1} MHz · step {:.1} MHz · dwell {} ms · cycle #{} ",
                    sw.config.start_hz as f64 / 1e6,
                    sw.config.stop_hz as f64 / 1e6,
                    step_mhz,
                    sw.config.dwell_ms,
                    sw.cycle_count,
                ),
                Style::default().fg(theme.label),
            ),
        ]);
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border));
        let inner = block.inner(area);
        f.render_widget(block, area);
        if inner.width <= AXIS_W + 2 || inner.height < 4 { return; }

        // Rows: plot area, x-axis labels, band-plan labels, status.
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),    // plot
                Constraint::Length(1), // freq axis
                Constraint::Length(1), // band plan
                Constraint::Length(1), // status
            ])
            .split(inner);

        let Some(frame) = sw.current_frame.as_ref() else {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    if state.radio.hw_streaming { "  Scanning… first cycle in progress" } else { "  Waiting — open lab_sweep with RX available" },
                    Style::default().fg(theme.stale),
                ))),
                rows[0],
            );
            return;
        };

        let plot = rows[0];
        let plot_w = plot.width.saturating_sub(AXIS_W) as usize;
        let plot_h = plot.height as usize;
        if plot_w == 0 || plot_h == 0 { return; }

        let curve = frame.project(plot_w, sw.show_peak);

        // dBFS bar plot: for each cell row (top = Y_MAX), mark columns that reach it.
        let span_db = (Y_MAX - Y_MIN).max(1.0);
        let mut plot_lines: Vec<Line> = Vec::with_capacity(plot_h);
        for r in 0..plot_h {
            // Row r maps to a dBFS threshold (row 0 = top = high dBFS).
            let row_frac = 1.0 - (r as f32 + 0.5) / plot_h as f32;
            let threshold = Y_MIN + row_frac * span_db;
            // Left gutter: a dBFS label every few rows.
            let label = if r == 0 {
                format!("{:>4} ", Y_MAX as i32)
            } else if r == plot_h - 1 {
                format!("{:>4} ", Y_MIN as i32)
            } else if plot_h >= 5 && r == plot_h / 2 {
                format!("{:>4} ", ((Y_MAX + Y_MIN) / 2.0) as i32)
            } else {
                "     ".to_string()
            };
            let mut bar = String::with_capacity(plot_w);
            for &v in &curve {
                bar.push(if v.is_finite() && v >= threshold { '█' } else { ' ' });
            }
            plot_lines.push(Line::from(vec![
                Span::styled(label, Style::default().fg(theme.label)),
                Span::styled(bar, Style::default().fg(theme.value_hi)),
            ]));
        }
        f.render_widget(Paragraph::new(plot_lines), plot);

        // Cursor marker overlaid as a status (drawn in the status line below).
        let cursor_hz = sw.cursor_frac.map(|fr| frame.freq_at_fraction(fr));

        // X-axis: start / mid / stop MHz, left-padded past the gutter.
        let axis = format!(
            "{}{:<width$}{:^midw$}{:>endw$}",
            " ".repeat(AXIS_W as usize),
            format!("{:.0}", frame.start_hz as f64 / 1e6),
            format!("{:.0}", (frame.start_hz + frame.stop_hz) as f64 / 2e6),
            format!("{:.0} MHz", frame.stop_hz as f64 / 1e6),
            width = plot_w / 3,
            midw = plot_w / 3,
            endw = plot_w - 2 * (plot_w / 3),
        );
        f.render_widget(Paragraph::new(Line::from(Span::styled(axis, Style::default().fg(theme.label)))), rows[1]);

        // Band-plan label row: place each overlapping band's name at its centre x.
        f.render_widget(Paragraph::new(band_plan_line(frame.start_hz, frame.stop_hz, plot_w, AXIS_W as usize, theme)), rows[2]);

        // Status line: cursor readout, else the cycle summary.
        let status = match cursor_hz {
            Some(hz) => {
                let frac = sw.cursor_frac.unwrap_or(0.0);
                let bucket = ((frac * plot_w as f64) as usize).min(plot_w.saturating_sub(1));
                let level = curve.get(bucket).copied().unwrap_or(f32::NEG_INFINITY);
                let level_str = if level.is_finite() { format!("{:.1} dBFS", level) } else { "—".to_string() };
                let band = band_at(hz).map(|b| format!("  [{}]", b)).unwrap_or_default();
                Line::from(vec![
                    Span::styled(" Cursor ", Style::default().fg(theme.label)),
                    Span::styled(format!("{:.3} MHz", hz as f64 / 1e6), Style::default().fg(theme.value_hi).add_modifier(Modifier::BOLD)),
                    Span::raw("  "),
                    Span::styled(level_str, Style::default().fg(theme.value)),
                    Span::styled(band, Style::default().fg(theme.status_ok)),
                ])
            }
            None => Line::from(vec![
                Span::styled(" pos ", Style::default().fg(theme.label)),
                Span::styled(format!("{}/{}", sw.positions_done, sw.positions_total), Style::default().fg(theme.value)),
                Span::styled("  ·  cycle ", Style::default().fg(theme.label)),
                Span::styled(format!("#{} ({:.1}s)", frame.cycle_count, frame.cycle_duration_ms as f64 / 1000.0), Style::default().fg(theme.value)),
                Span::styled("  ·  ", Style::default().fg(theme.label)),
                Span::styled(if sw.show_peak { "PEAK" } else { "MEAN" }, Style::default().fg(theme.value_hi)),
                Span::styled(format!("  ·  {:.0}s ago", frame.timestamp.elapsed().as_secs_f64()), Style::default().fg(theme.stale)),
                Span::styled("  ·  focus [G] for cursor", Style::default().fg(theme.stale)),
            ]),
        };
        f.render_widget(Paragraph::new(status), rows[3]);
    }
}

/// Build the band-plan label row: each known band overlapping `[start, stop]`
/// gets its name placed at its centre x (within the plot area, after the gutter).
fn band_plan_line(start_hz: u64, stop_hz: u64, plot_w: usize, gutter: usize, theme: &crate::Theme) -> Line<'static> {
    let mut row = vec![' '; gutter + plot_w];
    if stop_hz > start_hz {
        let span = (stop_hz - start_hz) as f64;
        for &(bs, be, name) in BAND_PLAN {
            if be <= start_hz || bs >= stop_hz { continue; }
            let centre = (bs.max(start_hz) + be.min(stop_hz)) / 2;
            let frac = (centre - start_hz) as f64 / span;
            let col = gutter + ((frac * plot_w as f64) as usize).min(plot_w.saturating_sub(1));
            // Place the name starting at `col`, not overwriting earlier labels.
            for (k, ch) in name.chars().enumerate() {
                let idx = col + k;
                if idx < row.len() && row[idx] == ' ' {
                    row[idx] = ch;
                }
            }
        }
    }
    Line::from(Span::styled(row.into_iter().collect::<String>(), Style::default().fg(theme.status_ok)))
}
