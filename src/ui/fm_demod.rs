//! `fm_demod` — the right column of the `lab_signal` preset's redesign
//! (DSN-2026-07): the FM MPX · DEMOD instrument.
//!
//! Phase 1 leaves this a MOD-classifier-driven placeholder: a status headline
//! (`idle_status`) that reads "NO SIGNAL" or "DEMOD IDLE — {MOD} detected" —
//! neither is a fault, so both read dim/neutral, never a warning colour — above
//! the real nameplate sections (MPX BASEBAND / PILOT · STEREO / DEVIATION / RDS /
//! AUDIO) held empty. Phase 2 fills those sections in place with the recovered
//! WFM baseband; later NFM / AM slot into the same panel via a single
//! `match modulation` dispatch — the analogue of how the app already adapts to
//! HackRF vs RTL front-ends.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::state::{Modulation, SdrMetrics};
use crate::ui::panel::Panel;

pub struct FmDemodPanel;

/// The Phase-1 status headline: `(mark, headline, detail)`. Both possible states
/// are dim/neutral (`theme.stale`) — an idle demod isn't a fault, the same
/// framing `signal_characterization`'s own "IDLE — RX stopped" uses.
fn idle_status(modulation: Modulation) -> (&'static str, &'static str, &'static str) {
    if modulation.is_known() {
        ("\u{25cb}", "DEMOD IDLE",
         "Carrier detected \u{2014} demodulation lands in a later phase.")
    } else {
        ("\u{25cb}", "NO SIGNAL",
         "Tune to a broadcast station and centre it to characterize.")
    }
}

impl Panel for FmDemodPanel {
    fn name(&self) -> &'static str { "fm_demod" }
    fn min_size(&self) -> (u16, u16) { (28, 12) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let stale = !state.radio.hw_streaming;
        let name_style = Style::default().fg(theme.label).add_modifier(Modifier::BOLD);
        let mut title = vec![Span::raw(" "), Span::styled("FM MPX \u{00b7} Demod", name_style)];
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

        let iw = inner.width as usize;
        let dim = Style::default().fg(theme.stale);
        let lbl = Style::default().fg(theme.label);
        let mut lines: Vec<Line> = Vec::new();

        // ── Status headline ────────────────────────────────────────────────
        if stale {
            lines.push(Line::from(vec![Span::raw(" "), Span::styled("\u{25cb} IDLE \u{2014} RX stopped", dim)]));
        } else {
            let (mark, headline, detail) = idle_status(state.signal.modulation);
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled(format!("{mark} {headline}"), Style::default().fg(theme.stale).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(vec![Span::raw(" "), Span::styled(detail, lbl)]));
        }
        lines.push(Line::raw(""));

        // ── Demod zone nameplates (Phase 2/3 fill these in place) ─────────
        // Uses the shared `chrome::section` so it reads in the same family as
        // the other lab rails.
        for (name, hint) in [
            ("MPX BASEBAND", "0-57 kHz"),
            ("PILOT / STEREO", "19 kHz"),
            ("DEVIATION", "75 kHz max"),
            ("RDS", "57 kHz"),
            ("AUDIO", ""),
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
        assert_eq!(FmDemodPanel.name(), "fm_demod");
    }

    #[test]
    fn idle_status_reads_no_signal_when_modulation_unknown() {
        let (_, headline, _) = idle_status(Modulation::Unknown);
        assert_eq!(headline, "NO SIGNAL");
    }

    #[test]
    fn idle_status_reads_demod_idle_when_modulation_known() {
        for m in [Modulation::Wfm, Modulation::Nfm, Modulation::Am] {
            let (_, headline, _) = idle_status(m);
            assert_eq!(headline, "DEMOD IDLE", "modulation={m:?}");
        }
    }
}
