use ratatui::{layout::Rect, Frame};
use crate::state::SdrMetrics;

pub trait Panel: Send + Sync {
    fn name(&self) -> &'static str;
    #[allow(dead_code)]
    fn min_size(&self) -> (u16, u16);
    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool);

    /// Single character that activates panel-focus mode for this panel.
    /// Returns `None` for panels that don't support focus mode.
    fn focus_key(&self) -> Option<char> { None }

    /// Keybindings shown in the footer when this panel is focused.
    /// Each entry: (key_label, description). Empty by default.
    /// Do NOT include Esc or Tab — the footer appends those automatically.
    fn focus_bindings(&self) -> &'static [(&'static str, &'static str)] { &[] }

    /// Preferred rendered height in rows, given the available terminal width and current state.
    /// Used by the layout engine for top/bottom panels. Default: 3 (1 content + 2 borders).
    fn preferred_height(&self, _available_width: u16, _state: &SdrMetrics) -> u16 { 3 }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyPanel;

    impl Panel for DummyPanel {
        fn name(&self) -> &'static str { "dummy" }
        fn min_size(&self) -> (u16, u16) { (10, 3) }
        fn render(&self, _f: &mut Frame, _area: Rect, _state: &SdrMetrics, _theme: &crate::Theme, _focused: bool) {}
    }

    #[test]
    fn panel_name_and_min_size() {
        let p = DummyPanel;
        assert_eq!(p.name(), "dummy");
        assert_eq!(p.min_size(), (10, 3));
    }
}
