use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

use crate::state::SdrMetrics;

pub fn render(f: &mut Frame, area: Rect, m: &SdrMetrics, theme: &crate::Theme) {
    let log_lines: Vec<&str> = m.ui.log.iter().map(|s| s.as_str()).collect();
    let log_text = log_lines.join("\n");
    let panel = Paragraph::new(log_text)
        .block(
            Block::default()
                .title(" Log ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border_dim)),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });
    f.render_widget(panel, area);
}

use super::panel::Panel;

pub struct LogPanel;

impl Panel for LogPanel {
    fn name(&self) -> &'static str { "log" }
    fn min_size(&self) -> (u16, u16) { (20, 7) }
    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, _focused: bool) {
        render(f, area, state, theme);
    }
}
