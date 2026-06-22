use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::charts::{gain_bar_colored, null_meter};
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

        let iw = inner.width as usize;
        let dim = theme.border_dim;
        let lbl_st = Style::default().fg(theme.label);

        // Fixed label field (3-wide) + a right value budget, so every meter/bar
        // starts and ends at the same column and the readings line up. The meter
        // (arrows + track) and the gradient bar share one visual width (`field_w`).
        const LEAD: usize = 5;       // " LBL " = space + 3 + space
        const VALUE_W: usize = 10;
        let field_w = iw.saturating_sub(LEAD + 1 + VALUE_W).max(8);
        let track_w = field_w.saturating_sub(2); // null_meter adds 2 arrow columns

        // ├╴ SECTION ╶──── nameplate — the same language as the command rail.
        let section = |name: &str| {
            let label = name.to_uppercase();
            let used = label.chars().count() + 5;
            Line::from(vec![
                Span::styled("├╴ ".to_string(), Style::default().fg(dim)),
                Span::styled(label, Style::default().fg(theme.label).add_modifier(Modifier::BOLD)),
                Span::styled(" ╶".to_string(), Style::default().fg(dim)),
                Span::styled("─".repeat(iw.saturating_sub(used)), Style::default().fg(dim)),
            ])
        };
        // " LBL " + bipolar null-meter + "  value"
        let meter_row = |label: &str, value: f64, full_scale: f64, color: Color,
                         val_str: String| -> Line<'static> {
            let mut spans = vec![
                Span::raw(" "),
                Span::styled(format!("{label:<3}"), lbl_st),
                Span::raw(" "),
            ];
            spans.extend(null_meter(value, full_scale, track_w, color, dim));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(val_str, Style::default().fg(color).add_modifier(Modifier::BOLD)));
            Line::from(spans)
        };
        // " LBL " + gradient quality bar + "  value" (frac 0..1 maps the fill).
        let bar_row = |label: &str, frac: f64, lo: Color, hi: Color, val_color: Color,
                       val_str: String| -> Line<'static> {
            let v = (frac.clamp(0.0, 1.0) * 1000.0) as u32;
            let mut spans = vec![
                Span::raw(" "),
                Span::styled(format!("{label:<3}"), lbl_st),
                Span::raw(" "),
            ];
            spans.extend(gain_bar_colored(v, 1000, field_w, lo, hi, dim));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(val_str, Style::default().fg(val_color).add_modifier(Modifier::BOLD)));
            Line::from(spans)
        };

        let mut lines: Vec<Line> = Vec::new();

        // --- DC OFFSET ---------------------------------------------------------
        // I / Q offsets are deviations from zero → null-meters; magnitude is a
        // green→red quality bar; the spike is a plain level readout.
        lines.push(section("DC offset"));
        let i_color = offset_color(state.iq.dc_offset_i.abs(), theme);
        let q_color = offset_color(state.iq.dc_offset_q.abs(), theme);
        lines.push(meter_row("I", state.iq.dc_offset_i as f64, 0.05, i_color,
                             format!("{:+.4}", state.iq.dc_offset_i)));
        lines.push(meter_row("Q", state.iq.dc_offset_q as f64, 0.05, q_color,
                             format!("{:+.4}", state.iq.dc_offset_q)));

        let dc_mag   = (state.iq.dc_offset_i as f64).hypot(state.iq.dc_offset_q as f64);
        let dc_color = offset_color(dc_mag as f32, theme);
        lines.push(bar_row("MAG", dc_mag / 0.05, theme.status_ok, theme.status_crit,
                           dc_color, format!("{dc_mag:.4}")));

        let spike = dc_spike_dbfs(dc_mag);
        let (spike_str, spike_col) = match spike {
            Some(s) => (format!("{s:.1} dBFS"), spike_color(s, theme)),
            None    => ("—".to_string(), theme.label),
        };
        lines.push(Line::from(vec![
            Span::raw(" "),
            Span::styled("SPK", lbl_st),
            Span::raw("  "),
            Span::styled(spike_str, Style::default().fg(spike_col)),
        ]));

        lines.push(Line::raw(""));

        // --- QUADRATURE --------------------------------------------------------
        // Amplitude / phase imbalance are deviations from balance → null-meters;
        // IRR (higher = better) is a red→green quality bar.
        lines.push(section("Quadrature"));
        let amp_abs = state.iq.iq_imbalance_db.abs();
        lines.push(meter_row("AMP", state.iq.iq_imbalance_db as f64, 4.0,
                             imbalance_color(amp_abs, theme),
                             format!("{:+.2} dB", state.iq.iq_imbalance_db)));
        let phase_abs = state.iq.phase_imbalance_deg.abs();
        lines.push(meter_row("PHA", state.iq.phase_imbalance_deg as f64, 6.0,
                             phase_color(phase_abs, theme),
                             format!("{:+.2}\u{b0}", state.iq.phase_imbalance_deg)));

        let irr = image_rejection_db(state.iq.iq_imbalance_db, state.iq.phase_imbalance_deg);
        let irr_str = if irr >= 60.0 { "> 60 dB".to_string() } else { format!("{irr:.1} dB") };
        lines.push(bar_row("IRR", irr / 60.0, theme.status_crit, theme.status_ok,
                           irr_color(irr, theme), irr_str));

        lines.push(Line::raw(""));

        // --- verdict -----------------------------------------------------------
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
            theme.status_ok
        };
        lines.push(Line::from(vec![
            Span::raw(" "),
            Span::styled(hint, Style::default().fg(hint_color)),
        ]));

        f.render_widget(Paragraph::new(lines), inner);
    }
}
