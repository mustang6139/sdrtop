use ratatui::{
    layout::{Alignment, Rect},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::state::SdrMetrics;

pub fn render(f: &mut Frame, area: Rect, m: &SdrMetrics) {
    let log_lines: Vec<&str> = m.log.iter().map(|s| s.as_str()).collect();
    let log_text = log_lines.join("\n");
    let panel = Paragraph::new(log_text)
        .block(Block::default().title(" Log ").borders(Borders::ALL))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });
    f.render_widget(panel, area);
}
