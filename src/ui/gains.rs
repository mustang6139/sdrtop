use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Gauge, Sparkline},
    Frame,
};

use crate::state::SdrMetrics;

pub fn render(f: &mut Frame, area: Rect, m: &SdrMetrics) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    let lna_gauge = Gauge::default()
        .block(
            Block::default()
                .title(format!(" LNA Gain: {} dB ", m.lna_gain))
                .borders(Borders::ALL),
        )
        .gauge_style(
            Style::default()
                .fg(Color::Cyan)
                .bg(Color::Black)
                .add_modifier(Modifier::ITALIC),
        )
        // LNA valid range: 0–40 dB in 8 dB steps
        .percent(((m.lna_gain as f32 / 40.0) * 100.0) as u16);
    f.render_widget(lna_gauge, chunks[0]);

    let vga_gauge = Gauge::default()
        .block(
            Block::default()
                .title(format!(" VGA Gain: {} dB ", m.vga_gain))
                .borders(Borders::ALL),
        )
        .gauge_style(
            Style::default()
                .fg(Color::Magenta)
                .bg(Color::Black)
                .add_modifier(Modifier::ITALIC),
        )
        // VGA valid range: 0–62 dB in 2 dB steps
        .percent(((m.vga_gain as f32 / 62.0) * 100.0) as u16);
    f.render_widget(vga_gauge, chunks[1]);

    let sr_gauge = Gauge::default()
        .block(
            Block::default()
                .title(format!(
                    " Sample Rate: {:.1} Msps ",
                    m.actual_sample_rate as f64 / 1_000_000.0
                ))
                .borders(Borders::ALL),
        )
        .gauge_style(
            Style::default()
                .fg(Color::Yellow)
                .bg(Color::Black)
                .add_modifier(Modifier::ITALIC),
        )
        // HackRF One max: 20 Msps
        .percent(((m.actual_sample_rate as f32 / 20_000_000.0) * 100.0).min(100.0) as u16);
    f.render_widget(sr_gauge, chunks[2]);

    let sparkline_data: Vec<u64> = m.throughput_history.iter().cloned().collect();
    let sparkline_max = sparkline_data.iter().cloned().max().unwrap_or(0).max(1);
    let sparkline = Sparkline::default()
        .block(
            Block::default()
                .title(format!(" USB Throughput (KB/s, peak: {}) ", sparkline_max))
                .borders(Borders::ALL),
        )
        .data(&sparkline_data)
        .max(sparkline_max)
        .style(Style::default().fg(Color::Green));
    f.render_widget(sparkline, chunks[3]);
}
