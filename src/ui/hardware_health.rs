use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

pub struct HardwareHealthPanel;

fn threshold_color(value: f64, warn: f64, crit: f64, theme: &crate::Theme) -> Color {
    if value >= crit      { theme.status_crit }
    else if value >= warn { theme.status_warn }
    else                  { theme.status_ok   }
}

impl Panel for HardwareHealthPanel {
    fn name(&self) -> &'static str { "hardware_health" }
    fn min_size(&self) -> (u16, u16) { (30, 18) }
    fn focus_key(&self) -> Option<char> { Some('v') }
    fn focus_bindings(&self) -> &'static [(&'static str, &'static str)] {
        &[("R", "Reset drop counter"), ("C", "Clear history")]
    }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let stale = !state.radio.hw_streaming;
        // Title "Hardware Vitals" with 'V' highlighted as the focus-key indicator
        // ([V]) — 'v' was chosen over 'h' to avoid clashing with the global hold key.
        let key_style = Style::default().fg(theme.value_hi).add_modifier(Modifier::BOLD);
        let mut title_spans = vec![
            Span::raw(" Hardware "),
            Span::styled("V", key_style),
            Span::raw("itals"),
        ];
        if stale {
            title_spans.push(Span::styled(" [STALE]", Style::default().fg(theme.stale)));
        }
        title_spans.push(Span::raw(" "));
        let title_line = Line::from(title_spans);
        let border_color = if focused { theme.border_focused }
            else if stale { theme.stale }
            else { theme.border_default };
        let block = Block::default()
            .title(title_line)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // [0] drops label
                Constraint::Length(2), // [1] drops graph
                Constraint::Length(1), // [2] sat label
                Constraint::Length(2), // [3] sat graph
                Constraint::Length(1), // [4] CPU + RAM label
                Constraint::Length(2), // [5] CPU graph
                Constraint::Length(1), // [6] USB label
                Constraint::Length(2), // [7] USB graph
                Constraint::Length(1), // [8] sample rate label
                Constraint::Length(1), // [9] BUF fill label
                Constraint::Length(2), // [10] BUF fill graph
                Constraint::Min(0),
            ])
            .split(inner);

        let drop_color = threshold_color(state.signal.drops_per_sec as f64, 1.0, 10.0, theme);
        f.render_widget(
            Paragraph::new(Span::styled(
                format!(
                    "Drops: {}/s  (session total: {})",
                    state.signal.drops_per_sec, state.signal.total_drops_session
                ),
                Style::default().fg(drop_color),
            )),
            rows[0],
        );
        let drop_data: Vec<u64> = state.signal.drop_history.iter().cloned().collect();
        crate::ui::charts::draw_mini_graph(f, rows[1], &drop_data, drop_color);

        let sat_color = threshold_color(state.signal.adc_saturation_pct as f64, 1.0, 5.0, theme);
        f.render_widget(
            Paragraph::new(Span::styled(
                format!(
                    "ADC sat: {:.1}%  (peak: {:.1}%)",
                    state.signal.adc_saturation_pct, state.signal.adc_saturation_peak
                ),
                Style::default().fg(sat_color),
            )),
            rows[2],
        );
        let sat_data: Vec<u64> = state.signal.saturation_history.iter()
            .map(|v| (*v * 1000.0) as u64)  // millipercent — preserves sub-1% values on graph
            .collect();
        crate::ui::charts::draw_mini_graph(f, rows[3], &sat_data, sat_color);

        let cpu = state.system.process_cpu_pct;
        let cpu_color = threshold_color(cpu as f64, 50.0, 80.0, theme);
        let rss_color = threshold_color(state.system.process_rss_mb as f64, 200.0, 400.0, theme);
        let lbl = Style::default().fg(theme.label);
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("CPU: ", lbl),
                Span::styled(format!("{:.1}%", cpu), Style::default().fg(cpu_color)),
                Span::styled("   RAM: ", lbl),
                Span::styled(format!("{} MB", state.system.process_rss_mb), Style::default().fg(rss_color)),
            ])),
            rows[4],
        );
        let cpu_data: Vec<u64> = state.system.cpu_history.iter().cloned().collect();
        crate::ui::charts::draw_mini_graph(f, rows[5], &cpu_data, cpu_color);

        // Color by recent rate (last poll delta), not session total — avoids
        // permanently-crit display after a single historic error.
        let usb_recent: u64 = state.signal.usb_error_history.iter().sum();
        let usb_color = if usb_recent > 0 { theme.status_crit }
            else if state.signal.usb_errors_session > 0 { theme.status_warn }
            else { theme.status_ok };
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("USB errors: {} (session)", state.signal.usb_errors_session),
                Style::default().fg(usb_color),
            )),
            rows[6],
        );
        let usb_err_data: Vec<u64> = state.signal.usb_error_history.iter().cloned().collect();
        crate::ui::charts::draw_mini_graph(f, rows[7], &usb_err_data, usb_color);

        // Sample rate: configured (always known) vs actually measured.
        let cfg_sr = state.radio.config_sample_rate / 1_000_000.0;
        let act_sr = state.radio.actual_sample_rate as f64 / 1_000_000.0;
        let (sr_text, sr_color) = if state.radio.actual_sample_rate == 0 || cfg_sr <= 0.0 {
            // Not streaming (or not yet measured): show the configured rate, dash the actual.
            (format!("SR  {:.3} → --- MHz", cfg_sr), theme.label)
        } else {
            let delta_pct = ((act_sr - cfg_sr) / cfg_sr * 100.0).abs();
            let color = if delta_pct < 2.0       { theme.status_ok }
                        else if delta_pct < 10.0 { theme.status_warn }
                        else                     { theme.status_crit };
            (format!("SR  {:.3} → {:.3} MHz  ({:+.1}%)", cfg_sr, act_sr, act_sr - cfg_sr), color)
        };
        f.render_widget(
            Paragraph::new(Span::styled(sr_text, Style::default().fg(sr_color))),
            rows[8],
        );

        // Buffer fill history
        let buf_color = if state.iq.buf_fill_pct >= 80.0 { theme.status_crit }
                        else if state.iq.buf_fill_pct >= 50.0 { theme.status_warn }
                        else { theme.status_ok };
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("BUF fill: ", lbl),
                Span::styled(
                    if stale { "---".to_string() } else { format!("{:.0}%", state.iq.buf_fill_pct) },
                    Style::default().fg(if stale { theme.stale } else { buf_color }),
                ),
            ])),
            rows[9],
        );
        let buf_data: Vec<u64> = state.iq.buf_fill_history.iter().cloned().collect();
        crate::ui::charts::draw_mini_graph(f, rows[10], &buf_data, buf_color);
    }
}
