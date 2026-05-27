use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct Chunks {
    pub header: Rect,
    pub body_left: Rect,
    pub body_right: Rect,
    pub log: Rect,
    pub footer: Rect,
}

pub fn build(size: Rect) -> Chunks {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(7),
            Constraint::Length(3),
        ])
        .split(size);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(outer[1]);

    Chunks {
        header: outer[0],
        body_left: body[0],
        body_right: body[1],
        log: outer[2],
        footer: outer[3],
    }
}
