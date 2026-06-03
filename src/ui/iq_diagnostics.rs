use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

pub struct IqDiagnosticsPanel;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn irr_perfect_balance_is_high() {
        // α=1, θ=0 → den=0 → clamped to 99.9 dB
        let irr = image_rejection_db(0.0, 0.0);
        assert!((irr - 99.9).abs() < 0.01, "got {:.1}", irr);
    }

    #[test]
    fn irr_amp_only_0_5db() {
        // 0.5 dB amplitude imbalance, 0° phase → IRR ≈ 32 dB
        let irr = image_rejection_db(0.5, 0.0);
        assert!(irr > 30.0 && irr < 35.0, "expected ~32 dB, got {:.1}", irr);
    }

    #[test]
    fn irr_phase_only_2deg() {
        // 0 dB amplitude, 2° phase imbalance:
        // α=1, cosθ≈0.99939 → IRR = 10·log10(3.999/0.00122) ≈ 35.2 dB
        let irr = image_rejection_db(0.0, 2.0);
        assert!(irr > 34.0 && irr < 37.0, "expected ~35.2 dB, got {:.1}", irr);
    }

    #[test]
    fn irr_worsens_with_more_imbalance() {
        let irr_low  = image_rejection_db(0.5, 1.0);
        let irr_high = image_rejection_db(3.0, 5.0);
        assert!(irr_low > irr_high, "more imbalance should give worse IRR");
    }

    #[test]
    fn dc_spike_typical_values() {
        // dc_mag = 0.005 → spike = 20*log10(0.005) ≈ -46 dBFS
        let s = dc_spike_dbfs(0.005).unwrap();
        assert!((s - (-46.0)).abs() < 0.2, "expected ~-46 dBFS, got {:.1}", s);
    }

    #[test]
    fn dc_spike_zero_is_none() {
        assert!(dc_spike_dbfs(0.0).is_none());
    }
}

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

fn irr_color(irr_db: f64, theme: &crate::Theme) -> Color {
    if irr_db >= 30.0      { theme.status_ok   }
    else if irr_db >= 20.0 { theme.status_warn }
    else                   { theme.status_crit }
}

fn spike_color(spike_dbfs: f64, theme: &crate::Theme) -> Color {
    if spike_dbfs < -40.0      { theme.status_ok   }
    else if spike_dbfs < -20.0 { theme.status_warn }
    else                       { theme.status_crit }
}

/// Image Rejection Ratio from IQ amplitude and phase imbalance.
///
/// Exact formula for a direct-conversion quadrature receiver:
///   IRR = 10·log₁₀( (1 + α² + 2α·cosθ) / (1 + α² − 2α·cosθ) )
/// where α = linear amplitude ratio, θ = phase error in radians.
/// Returns 99.9 dB when imbalances are negligible (den ≈ 0 → IRR → ∞).
fn image_rejection_db(amp_imbalance_db: f32, phase_imbalance_deg: f32) -> f64 {
    let alpha = 10f64.powf(amp_imbalance_db as f64 / 20.0);
    let theta = phase_imbalance_deg as f64 * std::f64::consts::PI / 180.0;
    let num = 1.0 + alpha * alpha + 2.0 * alpha * theta.cos();
    let den = 1.0 + alpha * alpha - 2.0 * alpha * theta.cos();
    if den <= 1e-12 { return 99.9; }
    10.0 * (num / den).log10()
}

/// DC spike level in dBFS: how tall the centre-frequency spike is in the spectrum.
///   DC spike = 20·log₁₀(dc_magnitude)
/// Returns None when dc_mag is zero (no spike).
fn dc_spike_dbfs(dc_mag: f64) -> Option<f64> {
    if dc_mag <= 0.0 { return None; }
    Some(20.0 * dc_mag.log10())
}

impl Panel for IqDiagnosticsPanel {
    fn name(&self) -> &'static str { "iq_diagnostics" }
    fn min_size(&self) -> (u16, u16) { (30, 12) }
    fn focus_key(&self) -> Option<char> { Some('i') }
    fn focus_bindings(&self) -> &'static [(&'static str, &'static str)] {
        &[("C", "Snapshot to log")]
    }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let stale = !state.radio.hw_streaming;
        // Title: leading 'I' highlighted as the focus-key indicator ([I]).
        let key_style = Style::default().fg(theme.value_hi).add_modifier(Modifier::BOLD);
        let mut title_spans = vec![
            Span::raw(" "),
            Span::styled("I", key_style),
            Span::raw("Q Diagnostics"),
        ];
        if stale {
            title_spans.push(Span::styled(" [STALE]", Style::default().fg(theme.stale)));
        }
        title_spans.push(Span::raw(" "));
        let title_line = Line::from(title_spans);
        let border_color = if focused { theme.border_focused }
            else if stale { theme.stale }
            else { theme.border_default };
        let block = Block::default()
            .title(title_line)
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

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // [0] DC offset I
                Constraint::Length(1), // [1] DC offset Q
                Constraint::Length(1), // [2] DC magnitude bar
                Constraint::Length(1), // [3] DC spike dBFS
                Constraint::Length(1), // [4] blank
                Constraint::Length(1), // [5] IQ amplitude imbalance
                Constraint::Length(1), // [6] IQ phase imbalance
                Constraint::Length(1), // [7] Image Rejection Ratio
                Constraint::Length(1), // [8] hint
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

        // DC magnitude bar — f64 precision (see comment on dc_mag below)
        let dc_mag   = (state.iq.dc_offset_i as f64).hypot(state.iq.dc_offset_q as f64);
        let dc_ratio = (dc_mag / 0.05).min(1.0);
        let dc_color = offset_color(dc_mag as f32, theme);
        crate::ui::charts::draw_hbar(
            f, rows[2], dc_ratio,
            "DC mag ",
            &format!("{:.4}", dc_mag),
            dc_color, theme,
        );

        // DC spike level: how tall the DC spike appears in the spectrum
        let spike = dc_spike_dbfs(dc_mag);
        let (spike_str, spike_col) = match spike {
            Some(s) => (format!("{:.1} dBFS", s), spike_color(s, theme)),
            None    => ("---".to_string(), theme.label),
        };
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("DC spike      ", lbl),
                Span::styled(spike_str, Style::default().fg(spike_col)),
            ])),
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

        // Image Rejection Ratio — the key quadrature quality metric
        let irr = image_rejection_db(state.iq.iq_imbalance_db, state.iq.phase_imbalance_deg);
        let irr_str = if irr >= 60.0 {
            "> 60 dB".to_string()
        } else {
            format!("{:.1} dB", irr)
        };
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("IRR           ", lbl),
                Span::styled(irr_str, Style::default().fg(irr_color(irr, theme))),
            ])),
            rows[7],
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
        let hint_color = if amp_abs > 3.0 || phase_abs > 5.0 {
            theme.status_crit
        } else if dc_mag as f32 > 0.02 || amp_abs > 1.0 || phase_abs > 2.0 {
            theme.status_warn
        } else {
            theme.label
        };
        f.render_widget(
            Paragraph::new(Span::styled(hint, Style::default().fg(hint_color))),
            rows[8],
        );
    }
}
