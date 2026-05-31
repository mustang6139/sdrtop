use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Gauge, Sparkline},
    Frame,
};

use crate::state::SdrMetrics;
use super::panel::Panel;

pub struct GainsPanel;

impl Panel for GainsPanel {
    fn name(&self) -> &'static str { "gains" }
    fn min_size(&self) -> (u16, u16) { (20, 12) }

    fn render(&self, f: &mut Frame, area: Rect, m: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(area);

        let border_style = Style::default().fg(if focused { theme.border_focused } else { theme.border_dim });

        let lna_gauge = Gauge::default()
            .block(
                Block::default()
                    .title(format!(" LNA Gain: {} dB ", m.radio.lna_gain))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(border_style),
            )
            .gauge_style(Style::default().fg(theme.value_hi).add_modifier(Modifier::ITALIC))
            .percent(((m.radio.lna_gain as f32 / 40.0) * 100.0) as u16);
        f.render_widget(lna_gauge, chunks[0]);

        let vga_gauge = Gauge::default()
            .block(
                Block::default()
                    .title(format!(" VGA Gain: {} dB ", m.radio.vga_gain))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(border_style),
            )
            .gauge_style(Style::default().fg(theme.value).add_modifier(Modifier::ITALIC))
            .percent(((m.radio.vga_gain as f32 / 62.0) * 100.0) as u16);
        f.render_widget(vga_gauge, chunks[1]);

        let sr_gauge = Gauge::default()
            .block(
                Block::default()
                    .title(format!(
                        " Sample Rate: {:.1} Msps ",
                        m.radio.actual_sample_rate as f64 / 1_000_000.0
                    ))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(border_style),
            )
            .gauge_style(Style::default().fg(theme.status_ok).add_modifier(Modifier::ITALIC))
            .percent(((m.radio.actual_sample_rate as f32 / 20_000_000.0) * 100.0).min(100.0) as u16);
        f.render_widget(sr_gauge, chunks[2]);

        let sparkline_data: Vec<u64> = m.radio.throughput_history.iter().cloned().collect();
        let sparkline_max = sparkline_data.iter().cloned().max().unwrap_or(0).max(1);
        let sparkline = Sparkline::default()
            .block(
                Block::default()
                    .title(format!(" USB Throughput (KB/s, peak: {}) ", sparkline_max))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(border_style),
            )
            .data(&sparkline_data)
            .max(sparkline_max)
            .style(Style::default().fg(theme.status_ok));
        f.render_widget(sparkline, chunks[3]);
    }
}
