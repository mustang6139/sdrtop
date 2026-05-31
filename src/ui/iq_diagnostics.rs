use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph},
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

fn phase_color(abs_deg: f32, theme: &crate::Theme) -> Color {
    if abs_deg > 5.0      { theme.status_crit }
    else if abs_deg > 2.0 { theme.status_warn }
    else                  { theme.status_ok   }
}

impl Panel for IqDiagnosticsPanel {
    fn name(&self) -> &'static str { "iq_diagnostics" }
    fn min_size(&self) -> (u16, u16) { (30, 8) }

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

        if stale {
            f.render_widget(
                Paragraph::new(Span::styled("---", Style::default().fg(theme.label))),
                inner,
            );
            return;
        }

        // Layout: 5 text rows + gap + 2 gauge rows
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // DC offset I
                Constraint::Length(1), // DC offset Q
                Constraint::Length(1), // DC magnitude gauge label
                Constraint::Length(2), // DC magnitude gauge
                Constraint::Length(1), // blank
                Constraint::Length(1), // IQ amplitude imbalance
                Constraint::Length(1), // IQ phase imbalance
                Constraint::Length(1), // hint
                Constraint::Min(0),
            ])
            .split(inner);

        let lbl = Style::default().fg(theme.label);

        // DC offsets
        let i_color = offset_color(state.iq.dc_offset_i.abs(), theme);
        let q_color = offset_color(state.iq.dc_offset_q.abs(), theme);
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("DC  I ", lbl),
                Span::styled(format!("{:+.4}", state.iq.dc_offset_i), Style::default().fg(i_color)),
            ])),
            rows[0],
        );
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("    Q ", lbl),
                Span::styled(format!("{:+.4}", state.iq.dc_offset_q), Style::default().fg(q_color)),
            ])),
            rows[1],
        );

        // DC magnitude gauge (0 = perfect, 0.05 = very bad)
        let dc_mag = (state.iq.dc_offset_i.hypot(state.iq.dc_offset_q)) as f64;
        let dc_ratio = (dc_mag / 0.05).min(1.0);
        let dc_color = offset_color(dc_mag as f32, theme);
        f.render_widget(
            Paragraph::new(Span::styled("DC magnitude", lbl)),
            rows[2],
        );
        f.render_widget(
            Gauge::default()
                .label(format!("{:.4}", dc_mag))
                .ratio(dc_ratio)
                .style(Style::default().fg(dc_color)),
            rows[3],
        );

        // IQ amplitude imbalance
        let amp_abs = state.iq.iq_imbalance_db.abs();
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("Amp imbalance ", lbl),
                Span::styled(
                    format!("{:+.2} dB", state.iq.iq_imbalance_db),
                    Style::default().fg(imbalance_color(amp_abs, theme)),
                ),
            ])),
            rows[5],
        );

        // IQ phase imbalance
        let phase_abs = state.iq.phase_imbalance_deg.abs();
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("Phase imbal   ", lbl),
                Span::styled(
                    format!("{:+.2}°", state.iq.phase_imbalance_deg),
                    Style::default().fg(phase_color(phase_abs, theme)),
                ),
            ])),
            rows[6],
        );

        // Contextual hint
        let hint = if amp_abs > 3.0 || phase_abs > 5.0 {
            "⚠ IQ mismatch — consider calibration"
        } else if dc_mag > 0.02 {
            "⚠ high DC offset — DC spike likely"
        } else if amp_abs > 1.0 || phase_abs > 2.0 {
            "minor imbalance — acceptable"
        } else {
            "✓ IQ quality OK"
        };
        let hint_color = if amp_abs > 3.0 || phase_abs > 5.0 || dc_mag as f32 > 0.02 {
            theme.status_warn
        } else {
            theme.label
        };
        f.render_widget(
            Paragraph::new(Span::styled(hint, Style::default().fg(hint_color))),
            rows[7],
        );
    }
}
