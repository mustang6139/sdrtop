use ratatui::{
    layout::{Alignment, Rect},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, board_name: &str, fw: &str, serial: &str) {
    let header = Paragraph::new(format!(" {} | FW: {} | S/N: {} ", board_name, fw, serial))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
    f.render_widget(header, area);
}
