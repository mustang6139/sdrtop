use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

pub struct IqDiagnosticsPanel;

fn offset_color(abs_val: f32, theme: &crate::Theme) -> Color {
    if abs_val > 0.02       { theme.status_crit }
    else if abs_val > 0.005 { theme.status_warn }
    else                    { theme.status_ok   }
}

fn imbalance_color(abs_db: f32, theme: &crate::Theme) -> Color {
    if abs_db > 3.0      { theme.status_crit }
    else if abs_db > 1.0 { theme.status_warn }
    else                 { theme.status_ok   }
}

impl Panel for IqDiagnosticsPanel {
    fn name(&self) -> &'static str { "iq_diagnostics" }
    fn min_size(&self) -> (u16, u16) { (30, 6) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let stale = !state.radio.hw_streaming && !state.observer.active;
        let title = if stale { " IQ Diagnostics [STALE] " } else { " IQ Diagnostics " };
        let border_color = if focused { theme.border_focused }
            else if stale { theme.stale }
            else { theme.border_default };
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);

        if stale {
            f.render_widget(
                Paragraph::new(Span::styled("---", Style::default().fg(theme.label))),
                rows[0],
            );
            f.render_widget(
                Paragraph::new(Span::styled("---", Style::default().fg(theme.label))),
                rows[1],
            );
            return;
        }

        let max_offset = state.iq.dc_offset_i.abs().max(state.iq.dc_offset_q.abs());
        f.render_widget(
            Paragraph::new(Span::styled(
                format!(
                    "DC offset  I: {:+.4}  Q: {:+.4}",
                    state.iq.dc_offset_i, state.iq.dc_offset_q
                ),
                Style::default().fg(offset_color(max_offset, theme)),
            )),
            rows[0],
        );

        let abs_imbalance = state.iq.iq_imbalance_db.abs();
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("IQ imbalance: {:+.2} dB", state.iq.iq_imbalance_db),
                Style::default().fg(imbalance_color(abs_imbalance, theme)),
            )),
            rows[1],
        );

        let hint = if abs_imbalance < 1.0        { "OK — channels balanced" }
            else if state.iq.iq_imbalance_db > 0.0  { "I channel stronger" }
            else                                  { "Q channel stronger" };
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("  \u{2192} {}", hint),
                Style::default().fg(theme.label),
            )),
            rows[2],
        );
    }
}
