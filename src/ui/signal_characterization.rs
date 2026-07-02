//! `signal_characterization` — the left column of the `lab_signal` preset's
//! redesign (DSN-2026-07).
//!
//! Phase 1 grows this into the RADIO HEADLINE / SIGNAL METRICS / ADJACENT CHANNEL
//! / SPECTRAL SHAPE / verdict instrument from the mockup, sharing the lab
//! side-panel vocabulary (`chrome::section` nameplates, `fit_spacers` airy fill,
//! `charts::gain_bar_colored` bars) exactly like `iq_diagnostics`,
//! `rf_chain`, and `timing_diagnostics`.
//!
//! This is the Step-1 stub: a framed, STALE-aware placeholder so the three-zone
//! layout can stand before the zones are filled.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

pub struct SignalCharacterizationPanel;

impl Panel for SignalCharacterizationPanel {
    fn name(&self) -> &'static str { "signal_characterization" }
    fn min_size(&self) -> (u16, u16) { (30, 12) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let stale = !state.radio.hw_streaming;
        let name_style = Style::default().fg(theme.label).add_modifier(Modifier::BOLD);
        let mut title = vec![Span::raw(" "), Span::styled("Signal Characterization", name_style)];
        if stale { title.push(Span::styled(" [STALE]", Style::default().fg(theme.stale))); }
        title.push(Span::raw(" "));

        let border = if focused { theme.border_focused } else if stale { theme.stale } else { theme.border_default };
        let block = Block::default()
            .title(Line::from(title))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border));
        let inner = block.inner(area);
        f.render_widget(block, area);
        if inner.width == 0 || inner.height == 0 { return; }

        // Skeleton: the zone nameplates the Phase-1 steps fill in. Uses the shared
        // `chrome::section` so it reads in the same family as the other lab rails.
        let iw = inner.width as usize;
        let mut lines: Vec<Line> = Vec::new();
        for (name, hint) in [
            ("RADIO HEADLINE", ""),
            ("SIGNAL METRICS", ""),
            ("ADJACENT CHANNEL", "ACPR"),
            ("SPECTRAL SHAPE", "60 s"),
        ] {
            lines.push(crate::ui::chrome::section(name, hint, iw, theme));
            lines.push(Line::raw(""));
        }
        crate::ui::chrome::fit_spacers(&mut lines, inner.height as usize);
        f.render_widget(Paragraph::new(lines), inner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_name_is_stable() {
        assert_eq!(SignalCharacterizationPanel.name(), "signal_characterization");
    }
}
