//! `timing_panel` — host-side stream-timing diagnostics for the `lab_timing`
//! preset (`[8]`).
//!
//! Five stacked zones: callback timing (period + drift), jitter (std-dev, p95/p99/
//! max + sparkline), sample-rate accuracy, throughput (mean / std + sparkline),
//! and an errors line with the overall `TimingQuality` verdict. All values are
//! sourced from `state.timing`, which the rx poll task rebuilds each window.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::state::{SdrMetrics, TimingQuality};
use super::micro_common::{sparkline, status_badge};
use super::panel::Panel;

pub struct TimingPanel;

/// Inline sparkline width in cells.
const SPARK_W: usize = 18;

impl Panel for TimingPanel {
    fn name(&self) -> &'static str { "timing_panel" }
    fn min_size(&self) -> (u16, u16) { (40, 12) }
    fn focus_key(&self) -> Option<char> { Some('t') }
    fn focus_bindings(&self) -> &'static [(&'static str, &'static str)] {
        &[("R", "Reset jitter peak"), ("C", "Clear history")]
    }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let border = if focused { theme.border_focused } else { theme.border_default };
        // Title: leading 'T' highlighted as the focus-key indicator ([T]).
        let key_style = Style::default().fg(theme.value_hi).add_modifier(Modifier::BOLD);
        let title_line = Line::from(vec![
            Span::raw(" "),
            Span::styled("T", key_style),
            Span::raw("iming "),
        ]);
        let block = Block::default()
            .title(title_line)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border));
        let inner = block.inner(area);
        f.render_widget(block, area);
        if inner.width == 0 || inner.height == 0 { return; }

        let stale = !state.radio.hw_streaming;
        let t = &state.timing;
        let lbl  = |s: &'static str| Span::styled(s, Style::default().fg(theme.label));
        let val  = |s: String| Span::styled(s, Style::default().fg(theme.value));
        let dash = || Span::styled("---".to_string(), Style::default().fg(theme.stale));
        let section = |s: &'static str| Line::from(vec![
            Span::raw(" "),
            Span::styled(s, Style::default().fg(theme.value_hi).add_modifier(Modifier::BOLD)),
        ]);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // 0 header + section
                Constraint::Length(1), // 1 period
                Constraint::Length(1), // 2 drift
                Constraint::Length(1), // 3 jitter
                Constraint::Length(1), // 4 p95/p99/max
                Constraint::Length(1), // 5 jitter sparkline
                Constraint::Length(1), // 6 blank
                Constraint::Length(1), // 7 SAMPLE RATE
                Constraint::Length(1), // 8 SR
                Constraint::Length(1), // 9 SR drift
                Constraint::Length(1), // 10 blank
                Constraint::Length(1), // 11 THROUGHPUT
                Constraint::Length(1), // 12 TP
                Constraint::Length(1), // 13 TP sparkline
                Constraint::Length(1), // 14 blank
                Constraint::Length(1), // 15 ERRORS
                Constraint::Length(1), // 16 errors line
                Constraint::Length(1), // 17 blank
                Constraint::Length(1), // 18 overall
                Constraint::Min(0),
            ])
            .split(inner);

        // ── Header: status badge + first section title.
        let [dot, word] = status_badge(state, theme);
        f.render_widget(Paragraph::new(Line::from(vec![
            Span::raw(" "), dot, word, Span::raw("   "),
            Span::styled("CALLBACK TIMING", Style::default().fg(theme.value_hi).add_modifier(Modifier::BOLD)),
        ])), rows[0]);

        // Period: measured vs expected.
        let period_line = if stale || t.cb_period_us == 0 {
            vec![Span::raw(" "), lbl("Period   "), dash()]
        } else {
            vec![
                Span::raw(" "), lbl("Period   "), val(fmt_us(t.cb_period_us)),
                Span::raw("  "),
                Span::styled(format!("(exp {})", fmt_us(t.cb_period_expected)), Style::default().fg(theme.stale)),
            ]
        };
        f.render_widget(Paragraph::new(Line::from(period_line)), rows[1]);

        // Period drift (ppm).
        let drift_line = if stale || t.cb_period_us == 0 {
            vec![Span::raw(" "), lbl("Drift    "), dash()]
        } else {
            vec![Span::raw(" "), lbl("Drift    "), ppm_span(t.cb_period_delta_ppm, theme)]
        };
        f.render_widget(Paragraph::new(Line::from(drift_line)), rows[2]);

        // Jitter std-dev.
        let jitter_line = if stale {
            vec![Span::raw(" "), lbl("Jitter   "), dash()]
        } else {
            vec![Span::raw(" "), lbl("Jitter   "), val(format!("±{} µs", t.cb_jitter_us))]
        };
        f.render_widget(Paragraph::new(Line::from(jitter_line)), rows[3]);

        // p95 / p99 (current window) + session peak (reset with [R] in focus mode).
        let pct_line = if stale {
            vec![Span::raw(" "), lbl("         "), dash()]
        } else {
            vec![
                Span::raw(" "), lbl("         "),
                Span::styled(
                    format!("p95 {}  p99 {}  peak {} µs", t.jitter_p95_us, t.jitter_p99_us, t.jitter_session_max_us),
                    Style::default().fg(theme.value),
                ),
            ]
        };
        f.render_widget(Paragraph::new(Line::from(pct_line)), rows[4]);

        // Jitter sparkline (µs history shared with the IQ diagnostics panel).
        let jitter_hist: Vec<f64> = state.iq.jitter_history.iter().map(|&v| v as f64).collect();
        let jspark = if stale { String::new() } else { sparkline(&jitter_hist, SPARK_W) };
        f.render_widget(Paragraph::new(Line::from(vec![
            Span::raw(" "), Span::styled(jspark, Style::default().fg(theme.value)),
        ])), rows[5]);

        // ── Sample rate.
        f.render_widget(Paragraph::new(section("SAMPLE RATE")), rows[7]);
        let cfg_msps = state.radio.config_sample_rate / 1_000_000.0;
        let sr_line = if stale || state.radio.actual_sample_rate == 0 {
            vec![Span::raw(" "), lbl("Rate     "), val(format!("{:.3} MHz", cfg_msps)), Span::raw("  "), dash()]
        } else {
            let act_msps = state.radio.actual_sample_rate as f64 / 1_000_000.0;
            vec![
                Span::raw(" "), lbl("Rate     "),
                val(format!("{:.3} → {:.3} MHz", cfg_msps, act_msps)),
            ]
        };
        f.render_widget(Paragraph::new(Line::from(sr_line)), rows[8]);
        let sr_drift = if stale || state.radio.actual_sample_rate == 0 {
            vec![Span::raw(" "), lbl("Drift    "), dash()]
        } else {
            vec![Span::raw(" "), lbl("Drift    "), ppm_span(t.sr_delta_ppm, theme)]
        };
        f.render_widget(Paragraph::new(Line::from(sr_drift)), rows[9]);

        // ── Throughput.
        f.render_widget(Paragraph::new(section("THROUGHPUT")), rows[11]);
        let tp_line = if stale {
            vec![Span::raw(" "), lbl("Rate     "), dash()]
        } else {
            vec![
                Span::raw(" "), lbl("Rate     "),
                val(format!("{:.1} MB/s", t.throughput_mean_mbps)),
                Span::raw("  "),
                Span::styled(format!("σ {:.2}", t.throughput_std_mbps), Style::default().fg(theme.stale)),
            ]
        };
        f.render_widget(Paragraph::new(Line::from(tp_line)), rows[12]);
        let tp_hist: Vec<f64> = state.radio.throughput_history.iter().map(|&v| v as f64).collect();
        let tspark = if stale { String::new() } else { sparkline(&tp_hist, SPARK_W) };
        f.render_widget(Paragraph::new(Line::from(vec![
            Span::raw(" "), Span::styled(tspark, Style::default().fg(theme.value)),
        ])), rows[13]);

        // ── Errors + overall verdict.
        f.render_widget(Paragraph::new(section("ERRORS")), rows[15]);
        let drops = state.signal.drops_per_sec;
        let usb = state.signal.usb_errors_session;
        let err_line = if stale {
            vec![Span::raw(" "), lbl("         "), dash()]
        } else {
            vec![
                Span::raw(" "), lbl("Drops    "),
                Span::styled(format!("{}/s", drops),
                    Style::default().fg(if drops == 0 { theme.status_ok } else { theme.status_crit })),
                Span::raw("   "), lbl("USB "),
                Span::styled(format!("{}", usb),
                    Style::default().fg(if usb == 0 { theme.value } else { theme.status_warn })),
            ]
        };
        f.render_widget(Paragraph::new(Line::from(err_line)), rows[16]);

        // Overall verdict — dimmed when idle, otherwise color-coded by severity.
        let overall = if stale {
            Line::from(vec![Span::raw(" "), Span::styled("○ IDLE — RX stopped", Style::default().fg(theme.stale))])
        } else {
            let q = t.timing_quality;
            let color = quality_color(q, theme);
            let mark = if q.severity() == 0 { "✓" } else { "⚠" };
            Line::from(vec![
                Span::raw(" "),
                Span::styled(format!("{} {}", mark, q.label()),
                    Style::default().fg(color).add_modifier(Modifier::BOLD)),
            ])
        };
        f.render_widget(Paragraph::new(overall), rows[18]);
    }
}

/// Microseconds rendered as `ms` once they pass 1000 µs, else plain `µs`.
fn fmt_us(us: u64) -> String {
    if us >= 1_000 { format!("{:.3} ms", us as f64 / 1_000.0) } else { format!("{} µs", us) }
}

/// Signed ppm value, colored by absolute magnitude (green / yellow / red).
fn ppm_span(ppm: i64, theme: &crate::Theme) -> Span<'static> {
    let mag = ppm.unsigned_abs();
    let color = if mag < 50 { theme.status_ok } else if mag < 200 { theme.status_warn } else { theme.status_crit };
    Span::styled(format!("{:+} ppm", ppm), Style::default().fg(color))
}

fn quality_color(q: TimingQuality, theme: &crate::Theme) -> ratatui::style::Color {
    match q.severity() {
        0 => theme.status_ok,
        1 => theme.value_hi,
        2 => theme.status_warn,
        _ => theme.status_crit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_us_switches_to_ms() {
        assert_eq!(fmt_us(450), "450 µs");
        assert_eq!(fmt_us(13_107), "13.107 ms");
    }

    #[test]
    fn quality_color_matches_severity() {
        let t = crate::theme::Theme::sdr();
        assert_eq!(quality_color(TimingQuality::Excellent, &t), t.status_ok);
        assert_eq!(quality_color(TimingQuality::Marginal, &t), t.status_warn);
        assert_eq!(quality_color(TimingQuality::Poor, &t), t.status_crit);
    }

    #[test]
    fn ppm_span_color_thresholds() {
        let t = crate::theme::Theme::sdr();
        assert_eq!(ppm_span(10, &t).style.fg, Some(t.status_ok));
        assert_eq!(ppm_span(-120, &t).style.fg, Some(t.status_warn));
        assert_eq!(ppm_span(600, &t).style.fg, Some(t.status_crit));
    }
}
