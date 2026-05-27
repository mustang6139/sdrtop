use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::state::SdrMetrics;

pub fn render(f: &mut Frame, area: Rect, m: &SdrMetrics, board_name: &str, serial: &str) {
    let status_text = if m.hw_streaming { "STREAMING" } else { "IDLE" };
    let status_color = if m.hw_streaming { Color::Green } else { Color::Yellow };

    let info_text = format!(
        "Model:       {}\n\
         Serial:      {}\n\
         Status:      {}\n\n\
         Frequency:   {:.3} MHz\n\
         Sample Rate: {:.1} Msps (cfg)\n\
         Throughput:  {:.2} MB/s ({:.1} Msps actual)\n\
         AMP:         {}",
        board_name,
        serial,
        status_text,
        m.frequency as f64 / 1_000_000.0,
        m.config_sample_rate / 1_000_000.0,
        m.current_throughput_bps as f64 / (1024.0 * 1024.0),
        m.actual_sample_rate as f64 / 1_000_000.0,
        if m.amp_enabled { "ON" } else { "OFF" },
    );

    let panel = Paragraph::new(info_text)
        .block(
            Block::default()
                .title(" Telemetry ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(status_color)),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });
    f.render_widget(panel, area);
}
