use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, Sparkline},
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

pub struct HardwareHealthPanel;

fn threshold_color(value: f64, warn: f64, crit: f64) -> Color {
    if value >= crit      { Color::Red    }
    else if value >= warn { Color::Yellow }
    else                  { Color::Green  }
}

impl Panel for HardwareHealthPanel {
    fn name(&self) -> &'static str { "hardware_health" }
    fn min_size(&self) -> (u16, u16) { (30, 12) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics) {
        let block = Block::default()
            .title(" Hardware Health ")
            .borders(Borders::ALL);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);

        let drop_color = threshold_color(state.drops_per_sec as f64, 1.0, 10.0);
        f.render_widget(
            Paragraph::new(Span::styled(
                format!(
                    "Drops: {}/s  (session total: {})",
                    state.drops_per_sec, state.total_drops_session
                ),
                Style::default().fg(drop_color),
            )),
            rows[0],
        );
        let drop_data: Vec<u64> = state.drop_history.iter().cloned().collect();
        f.render_widget(
            Sparkline::default()
                .data(&drop_data)
                .style(Style::default().fg(drop_color)),
            rows[1],
        );

        let sat_color = threshold_color(state.adc_saturation_pct as f64, 1.0, 5.0);
        f.render_widget(
            Paragraph::new(Span::styled(
                format!(
                    "ADC sat: {:.1}%  (peak: {:.1}%)",
                    state.adc_saturation_pct, state.adc_saturation_peak
                ),
                Style::default().fg(sat_color),
            )),
            rows[2],
        );
        let sat_data: Vec<u64> = state.saturation_history.iter()
            .map(|v| *v as u64)
            .collect();
        f.render_widget(
            Sparkline::default()
                .data(&sat_data)
                .style(Style::default().fg(sat_color)),
            rows[3],
        );

        let jitter_color = threshold_color(state.callback_jitter_us as f64, 500.0, 2000.0);
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("Jitter: {} µs (inter-callback mean)", state.callback_jitter_us),
                Style::default().fg(jitter_color),
            )),
            rows[4],
        );

        let usb_color = if state.usb_errors_session > 0 { Color::Red } else { Color::Green };
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("USB errors: {} (session)", state.usb_errors_session),
                Style::default().fg(usb_color),
            )),
            rows[5],
        );
    }
}
