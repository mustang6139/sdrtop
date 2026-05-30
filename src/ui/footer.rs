use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::state::{InputMode, SdrMetrics};
use super::panel::Panel;

pub struct FooterPanel;

impl Panel for FooterPanel {
    fn name(&self) -> &'static str { "footer" }
    fn min_size(&self) -> (u16, u16) { (40, 3) }

    fn render(&self, f: &mut Frame, area: Rect, m: &SdrMetrics, theme: &crate::Theme, _focused: bool) {
        let (text, border_color) = if m.observer_mode {
            (
                " [Q] Quit  ·  [?] Help  (Observer Mode) ".to_string(),
                theme.observer,
            )
        } else {
            match m.input_mode {
                InputMode::FrequencyInput => (
                    format!(" Frequency (MHz): [{}▌]  [Enter] Confirm  [Esc] Cancel ", m.input_buf),
                    theme.status_warn,
                ),
                InputMode::SampleRateInput => (
                    format!(" Sample rate (2–20 MHz): [{}▌]  [Enter] Confirm  [Esc] Cancel ", m.input_buf),
                    theme.status_warn,
                ),
                InputMode::MarkerNameInput => {
                    let freq_str = m.pending_marker_freq
                        .map(|f| format!("{:.3} MHz", f as f64 / 1_000_000.0))
                        .unwrap_or_default();
                    (
                        format!(" Marker name at {}:  [{}▌]  [Enter] Confirm  [Esc] Cancel ", freq_str, m.input_buf),
                        theme.status_warn,
                    )
                }
                InputMode::Normal => {
                    if let Some(panel_name) = &m.focused_panel {
                        let mut parts: Vec<String> = m.focused_panel_bindings.iter()
                            .map(|(k, d)| format!("[{}] {}", k, d))
                            .collect();
                        parts.push("[Esc] Exit focus".to_string());
                        let bindings = parts.join("  ·  ");
                        (
                            format!(" {}  — {} ", bindings, panel_name),
                            theme.border_focused,
                        )
                    } else {
                        (
                            " [Q] Quit  [Space] RX  [↑↓] LNA  [[] VGA  [A] AMP  [F] Freq  [S] Rate  [R] Reset  [?] Help ".to_string(),
                            theme.border_dim,
                        )
                    }
                }
            }
        };

        f.render_widget(
            Paragraph::new(text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(border_color)),
                )
                .alignment(Alignment::Center),
            area,
        );
    }
}
