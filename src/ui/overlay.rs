use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render_help(f: &mut Frame) {
    let area = centered_rect(62, 32, f.size());

    let text = "\
 [Q]        Quit\n\
 [SPACE]    Start / Stop RX\n\
 [↑] [↓]    LNA gain  +8 / −8 dB  (0–40 dB)\n\
 [[] []]    VGA gain  −2 / +2 dB  (0–62 dB)\n\
 [A]        Toggle AMP\n\
 [F]        Enter frequency (MHz)\n\
 [S]        Enter sample rate (2–20 MHz)\n\
 [R]        Reset all to defaults\n\
 [P]        Cycle presets\n\
 [1]        Preset: main\n\
 [2]        Preset: spectrum\n\
 [3]        Preset: waterfall\n\
 [4]        Preset: spectrum+waterfall\n\
 [5]/[6]/[7]/[8] Lab: IQ / RF / timing / signal\n\
 [0]        Micro field-mode (press again to cycle)\n\
 [W]        Pause / resume waterfall\n\
 [E]        Focus spectrum panel (expand / zoom)\n\
   Esc      Exit spectrum focus\n\
 [I]/[V]/[T] Focus lab panel: IQ / vitals / timing\n\
 [?]        Toggle this help\n\
 [Tab]      Toggle footer bar\n\
\n\
 --theme <name>:  sdr | nord | dracula | gruvbox | catppuccin | solarized\n\
\n\
 In frequency / sample rate input mode:\n\
   digits / .    type value\n\
   Backspace     delete last char\n\
   Enter         confirm\n\
   Esc           cancel\
";

    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .title(" Help — press [?] to close ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .alignment(Alignment::Left),
        area,
    );
}

fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(r.height.saturating_sub(height) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(r.width.saturating_sub(width) / 2),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(vertical[1])[1]
}
