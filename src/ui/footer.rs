use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Text},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::state::{InputMode, MicroView, SdrMetrics};
use super::panel::Panel;

const FOCUS_SEP:  &str = "  ·  ";
const NORMAL_SEP: &str = "  ";
const MAX_CONTENT_LINES: u16 = 5;

const NORMAL_ITEMS: &[&str] = &[
    "[Q] Quit", "[Space] RX", "[↑↓] LNA", "[[] VGA",
    "[A] AMP", "[F] Freq", "[S] Rate", "[R] Reset", "[?] Help", "[Tab] Hide",
];

/// Width (terminal columns) below which the preset name is shown in short form.
const NARROW_COLS: u16 = 60;

/// The lab preset family, in reserved number-key order. The footer shows these
/// as a navigation map when one of them is the active preset; only the ones
/// that actually exist (synced into `preset_names`) are listed.
const LAB_FAMILY: &[(&str, &str)] = &[
    ("5", "lab_iq"),
    ("6", "lab_rf"),
    ("7", "lab_timing"),
    ("8", "lab_signal"),
];

/// Display label for a preset in the footer. Narrow terminals get an
/// abbreviated form for the few long names; everything else passes through.
fn preset_label(name: &str, narrow: bool) -> &str {
    if narrow {
        match name {
            "spectrum_waterfall" => "spec+wf",
            "spectrum"           => "spec",
            "waterfall"          => "wf",
            other                => other,
        }
    } else {
        name
    }
}

/// Whether `name` belongs to the lab preset family.
fn is_lab_preset(name: &str) -> bool {
    LAB_FAMILY.iter().any(|(_, n)| *n == name)
}

/// Whether `name` is a micro ecosystem preset (entered via the `[0]` cycle).
fn is_micro_preset(name: &str) -> bool {
    name.starts_with("micro_")
}

/// Condensed footer for the micro ecosystem: the essential field keys plus the
/// `[0]▸{next}` hint and the `N/M` cycle position.
fn micro_items(view: MicroView, narrow: bool) -> Vec<String> {
    // Sweep is a future capability — not in the cycle yet.
    let sweep_active = false;
    let next  = view.next(sweep_active);
    let total = MicroView::total(sweep_active);
    let pos   = view.position();
    if narrow {
        vec![
            "[Q]".into(), "[Spc]".into(), "[↑↓]".into(),
            format!("[0]▸{}", next.label()),
            format!("{}/{}", pos, total),
        ]
    } else {
        vec![
            "[Q]".into(), "[Spc]RX".into(), "[↑↓]LNA".into(), "[[]VGA".into(), "[F]req".into(),
            format!("[0]▸{}", next.label()),
            format!("micro {}/{}", pos, total),
        ]
    }
}

/// Navigation map for the lab family: one entry per defined lab preset, with
/// the active one marked `▸`. Returns empty if none are available.
fn lab_map_items(active: &str, available: &[String]) -> Vec<String> {
    LAB_FAMILY.iter()
        .filter(|(_, name)| available.iter().any(|p| p == name))
        .map(|(key, name)| {
            if *name == active {
                format!("[{}]▸{}", key, name)
            } else {
                format!("[{}] {}", key, name)
            }
        })
        .collect()
}

/// The normal-mode footer items for the active preset:
/// - micro presets → a condensed field-key set with the `[0]` cycle hint;
/// - lab presets   → the fixed keys plus the lab navigation map;
/// - everything else → the fixed keys plus the `[P] {preset}` hint.
fn normal_items(active_preset: &str, available: &[String], micro_view: MicroView, available_width: u16) -> Vec<String> {
    let narrow = available_width < NARROW_COLS;
    if is_micro_preset(active_preset) {
        return micro_items(micro_view, narrow);
    }
    let mut items: Vec<String> = NORMAL_ITEMS.iter().map(|s| s.to_string()).collect();
    if is_lab_preset(active_preset) {
        items.extend(lab_map_items(active_preset, available));
    } else {
        items.push(format!("[P] {}", preset_label(active_preset, narrow)));
    }
    items
}

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

/// Public free function — called directly from the engine (bypasses dyn dispatch).
pub fn compute_footer_height(available_width: u16, state: &SdrMetrics) -> u16 {
    if !matches!(state.ui.input_mode, InputMode::Normal) || state.observer.active {
        return 3;
    }
    let inner_w = available_width.saturating_sub(2) as usize;
    let n = if state.ui.focused_panel.is_some() {
        count_lines(&focus_items(state), FOCUS_SEP, inner_w)
    } else {
        count_lines(&normal_items(&state.ui.active_preset, &state.ui.preset_names, state.ui.micro_view, available_width), NORMAL_SEP, inner_w)
    };
    (n as u16 + 2).min(MAX_CONTENT_LINES + 2).max(3)
}

pub struct FooterPanel;

impl Panel for FooterPanel {
    fn name(&self) -> &'static str { "footer" }
    fn min_size(&self) -> (u16, u16) { (40, 3) }

    fn preferred_height(&self, available_width: u16, state: &SdrMetrics) -> u16 {
        compute_footer_height(available_width, state)
    }

    fn render(&self, f: &mut Frame, area: Rect, m: &SdrMetrics, theme: &crate::Theme, _focused: bool) {
        // Clamp to the lines that actually fit — inner_h = area.height - 2 borders
        let inner_w = area.width.saturating_sub(2) as usize;
        let max_lines = area.height.saturating_sub(2) as usize;

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
                        wrapped.truncate(max_lines.max(1));
                        if let Some(last) = wrapped.last_mut() {
                            last.push_str(&format!("  — {}", panel_name));
                        }
                        (wrapped, theme.border_focused)
                    } else {
                        let items = normal_items(&m.ui.active_preset, &m.ui.preset_names, m.ui.micro_view, area.width);
                        let mut wrapped = wrap_items(&items, NORMAL_SEP, inner_w);
                        wrapped.truncate(max_lines.max(1));
                        (wrapped, theme.border_dim)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_items_splits_at_boundary() {
        let items = ["aaa", "bbb", "ccc"];
        // sep="  " (2), inner_w=7: "aaa  bbb"=8 > 7 → break after "aaa"
        let lines = wrap_items(&items, "  ", 7);
        assert_eq!(lines.len(), 3, "each item on its own line: {:?}", lines);
    }

    #[test]
    fn wrap_items_fits_all_on_one_line() {
        let items = ["aaa", "bbb"];
        // "aaa  bbb" = 8 chars, inner_w=10 → fits
        let lines = wrap_items(&items, "  ", 10);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "aaa  bbb");
    }

    #[test]
    fn normal_items_wrap_at_80_cols() {
        let n = count_lines(NORMAL_ITEMS, NORMAL_SEP, 78);
        assert!(n >= 2, "normal items at inner_w=78 should need >=2 lines, got {}", n);
    }

    #[test]
    fn normal_items_fit_at_200_cols() {
        let n = count_lines(NORMAL_ITEMS, NORMAL_SEP, 198);
        assert_eq!(n, 1, "normal items at inner_w=198 should fit on 1 line, got {}", n);
    }

    #[test]
    fn preset_label_abbreviates_when_narrow() {
        assert_eq!(preset_label("spectrum_waterfall", true), "spec+wf");
        assert_eq!(preset_label("spectrum_waterfall", false), "spectrum_waterfall");
        assert_eq!(preset_label("lab_iq", true), "lab_iq");
    }

    #[test]
    fn normal_items_appends_preset_entry() {
        let items = normal_items("main", &[], MicroView::Main, 120);
        assert_eq!(items.last().map(String::as_str), Some("[P] main"));
        assert_eq!(items.len(), NORMAL_ITEMS.len() + 1);
    }

    #[test]
    fn normal_items_uses_short_preset_when_narrow() {
        let items = normal_items("spectrum_waterfall", &[], MicroView::Main, 50);
        assert_eq!(items.last().map(String::as_str), Some("[P] spec+wf"));
    }

    #[test]
    fn micro_preset_shows_condensed_footer_with_next_and_position() {
        // From micro_main (Main), the [0] hint points at the next view (signal)
        // and the position reads 1/4.
        let items = normal_items("micro_main", &[], MicroView::Main, 120);
        assert!(items.iter().any(|i| i == "[0]▸signal"));
        assert!(items.iter().any(|i| i == "micro 1/4"));
        // No [P] hint and none of the long normal items in micro mode.
        assert!(items.iter().all(|i| !i.starts_with("[P]")));
        assert!(!items.contains(&"[R] Reset".to_string()));
    }

    #[test]
    fn micro_footer_narrow_is_more_compact() {
        let items = normal_items("micro_signal", &[], MicroView::Signal, 50);
        assert!(items.iter().any(|i| i == "[0]▸gain"));
        assert!(items.iter().any(|i| i == "2/4"));
    }

    #[test]
    fn lab_map_lists_only_available_presets_with_active_marked() {
        let available = vec!["lab_iq".to_string(), "lab_rf".to_string(), "lab_signal".to_string()];
        let map = lab_map_items("lab_rf", &available);
        // lab_timing [7] is not available → excluded.
        assert_eq!(map, vec!["[5] lab_iq", "[6]▸lab_rf", "[8] lab_signal"]);
    }

    #[test]
    fn normal_items_shows_lab_map_in_lab_preset() {
        let available = vec!["lab_iq".to_string(), "lab_rf".to_string()];
        let items = normal_items("lab_iq", &available, MicroView::Main, 120);
        // No [P] entry in lab mode; the map entries are appended instead.
        assert!(items.iter().all(|i| !i.starts_with("[P]")));
        assert!(items.contains(&"[5]▸lab_iq".to_string()));
        assert!(items.contains(&"[6] lab_rf".to_string()));
    }
}
