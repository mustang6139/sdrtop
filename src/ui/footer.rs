use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Text},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::state::{InputMode, SdrMetrics};
use super::panel::Panel;

const FOCUS_SEP:  &str = "  ·  ";
const NORMAL_SEP: &str = "  ";
const MAX_CONTENT_LINES: u16 = 5;

const NORMAL_ITEMS: &[&str] = &[
    "[Q] Quit", "[Space] RX", "[↑↓] LNA", "[[] VGA",
    "[A] AMP", "[F] Freq", "[S] Rate", "[R] Reset", "[?] Help", "[Tab] Hide",
];

/// Break `items` into lines where no line exceeds `inner_w` display columns.
fn wrap_items<S: AsRef<str>>(items: &[S], sep: &str, inner_w: usize) -> Vec<String> {
    let sep_w = sep.chars().count();
    let mut lines: Vec<String> = Vec::new();
    let mut cur: Vec<&str>     = Vec::new();
    let mut cur_w = 0usize;

    for item in items {
        let s  = item.as_ref();
        let iw = s.chars().count();
        let needed = if cur.is_empty() { iw } else { sep_w + iw };
        if !cur.is_empty() && inner_w > 0 && cur_w + needed > inner_w {
            lines.push(cur.join(sep));
            cur   = vec![s];
            cur_w = iw;
        } else {
            cur.push(s);
            cur_w += needed;
        }
    }
    if !cur.is_empty() { lines.push(cur.join(sep)); }
    if lines.is_empty() { lines.push(String::new()); }
    lines
}

fn count_lines<S: AsRef<str>>(items: &[S], sep: &str, inner_w: usize) -> usize {
    wrap_items(items, sep, inner_w).len()
}

pub struct FooterPanel;

impl Panel for FooterPanel {
    fn name(&self) -> &'static str { "footer" }
    fn min_size(&self) -> (u16, u16) { (40, 3) }

    fn preferred_height(&self, available_width: u16, state: &SdrMetrics) -> u16 {
        if !matches!(state.ui.input_mode, InputMode::Normal) || state.observer.active {
            return 3;
        }
        let inner_w = available_width.saturating_sub(2) as usize;
        let n = if state.ui.focused_panel.is_some() {
            let items = focus_items(state);
            count_lines(&items, FOCUS_SEP, inner_w)
        } else {
            count_lines(NORMAL_ITEMS, NORMAL_SEP, inner_w)
        };
        (n as u16 + 2).min(MAX_CONTENT_LINES + 2).max(3)
    }

    fn render(&self, f: &mut Frame, area: Rect, m: &SdrMetrics, theme: &crate::Theme, _focused: bool) {
        let inner_w = area.width.saturating_sub(2) as usize;

        let (lines, border_color): (Vec<String>, _) = if m.observer.active {
            (
                vec!["[Q] Quit  ·  [?] Help  (Observer Mode)".to_string()],
                theme.observer,
            )
        } else {
            match m.ui.input_mode {
                InputMode::FrequencyInput => (
                    vec![format!(" Frequency (MHz): [{}▌]  [Enter] Confirm  [Esc] Cancel", m.ui.input_buf)],
                    theme.status_warn,
                ),
                InputMode::SampleRateInput => (
                    vec![format!(" Sample rate (2–20 MHz): [{}▌]  [Enter] Confirm  [Esc] Cancel", m.ui.input_buf)],
                    theme.status_warn,
                ),
                InputMode::MarkerNameInput => {
                    let freq_str = m.spectrum.pending_marker
                        .map(|f| format!("{:.3} MHz", f as f64 / 1_000_000.0))
                        .unwrap_or_default();
                    (
                        vec![format!(" Marker name at {}:  [{}▌]  [Enter] Confirm  [Esc] Cancel", freq_str, m.ui.input_buf)],
                        theme.status_warn,
                    )
                }
                InputMode::Normal => {
                    if let Some(panel_name) = &m.ui.focused_panel {
                        let items = focus_items(m);
                        let mut wrapped = wrap_items(&items, FOCUS_SEP, inner_w);
                        if let Some(last) = wrapped.last_mut() {
                            last.push_str(&format!("  — {}", panel_name));
                        }
                        (wrapped, theme.border_focused)
                    } else {
                        (wrap_items(NORMAL_ITEMS, NORMAL_SEP, inner_w), theme.border_dim)
                    }
                }
            }
        };

        let text = Text::from_iter(lines.into_iter().map(Line::raw));
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

/// Build the ordered items list for focus-mode footer.
fn focus_items(m: &SdrMetrics) -> Vec<String> {
    let mut items: Vec<String> = m.ui.focused_panel_bindings.iter()
        .map(|(k, d)| format!("[{}] {}", k, d))
        .collect();
    items.push("[Tab] Hide".to_string());
    items.push("[Esc] Exit focus".to_string());
    items
}
