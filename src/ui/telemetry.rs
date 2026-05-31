use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

use crate::state::SdrMetrics;

pub fn render(f: &mut Frame, area: Rect, m: &SdrMetrics, board_name: &str, serial: &str, theme: &crate::Theme) {
    let status_text = if m.radio.hw_streaming { "STREAMING" } else { "IDLE" };
    let status_color = if m.radio.hw_streaming { theme.status_ok } else { theme.status_warn };

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
        m.radio.frequency as f64 / 1_000_000.0,
        m.radio.config_sample_rate / 1_000_000.0,
        m.radio.current_throughput_bps as f64 / 1_000_000.0,
        m.radio.actual_sample_rate as f64 / 1_000_000.0,
        if m.radio.amp_enabled { "ON" } else { "OFF" },
    );

    let panel = Paragraph::new(info_text)
        .block(
            Block::default()
                .title(" Telemetry ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(status_color)),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });
    f.render_widget(panel, area);
}

use super::panel::Panel;

pub struct TelemetryPanel {
    pub board_name: String,
    pub serial: String,
}

impl Panel for TelemetryPanel {
    fn name(&self) -> &'static str { "telemetry" }
    fn min_size(&self) -> (u16, u16) { (30, 10) }
    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, _focused: bool) {
        render(f, area, state, &self.board_name, &self.serial, theme);
    }
}
