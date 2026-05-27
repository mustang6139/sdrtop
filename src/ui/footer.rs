use ratatui::{
    layout::{Alignment, Rect},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect) {
    let footer =
        Paragraph::new(" [Q] Quit | [SPACE] Start/Stop RX | [R] Reset to defaults ")
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center);
    f.render_widget(footer, area);
}
