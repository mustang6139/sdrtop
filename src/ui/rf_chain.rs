use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::hardware::Device;
use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

pub struct RfChainPanel;

fn fmt_hz(hz: u32) -> String {
    if hz == 0        { return "---".to_string(); }
    if hz >= 1_000_000 {
        format!("{:.3} MHz", hz as f64 / 1_000_000.0)
    } else {
        format!("{} kHz", hz / 1_000)
    }
}

/// Cascade Noise Figure via Friis formula (result in dB).
///
/// HackRF One stage approximations:
///   AMP  — MGA-81563 front-end LNA: gain 14 dB, NF ~2.0 dB
///   LNA  — MAX2837 LNA: NF ~3.5 dB at max gain (40 dB), degrades ~0.15 dB
///          per dB of gain reduction (model: NF_LNA = 3.5 + (40−G)×0.15)
///   VGA  — MAX2837 baseband VGA: NF ~10 dB (contribution negligible at high LNA gain)
///
/// Friis: F_total = F₁ + (F₂−1)/G₁ + (F₃−1)/(G₁·G₂)  (all linear, → back to dB)
/// VGA gain is not a parameter — in a 3-stage cascade it does not appear in the
/// formula (there is no 4th stage whose noise it would need to suppress).
fn estimate_nf_db(amp_enabled: bool, lna_gain: u32) -> f64 {
    let lin = |db: f64| 10f64.powf(db / 10.0);

    let nf_lna = 3.5 + (40.0 - lna_gain as f64).max(0.0) * 0.15;
    let f_lna  = lin(nf_lna);
    let g_lna  = lin(lna_gain as f64);
    let f_vga  = lin(10.0);

    let f_total = if amp_enabled {
        let f_amp = lin(2.0);
        let g_amp = lin(14.0);
        f_amp + (f_lna - 1.0) / g_amp + (f_vga - 1.0) / (g_amp * g_lna)
    } else {
        f_lna + (f_vga - 1.0) / g_lna
    };

    10.0 * f_total.log10()
}

/// Minimum Detectable Signal in dBm.
///
/// MDS = kTB + NF  where kT = −174 dBm/Hz at 290 K.
/// Returns None when the BB filter bandwidth is unknown (0 Hz).
fn estimate_mds_dbm(bb_filter_hz: u32, nf_db: f64) -> Option<f64> {
    if bb_filter_hz == 0 { return None; }
    Some(-174.0 + 10.0 * (bb_filter_hz as f64).log10() + nf_db)
}

/// Returns `(text, severity)` where severity: 0 = OK, 1 = warn, 2 = crit.
fn gain_advice(hist: &[u64; 32]) -> (&'static str, u8) {
    let total: u64 = hist.iter().sum();
    if total == 0 { return ("no signal — start RX", 0); }
    let low:  u64 = hist[..8].iter().sum();
    let high: u64 = hist[24..].iter().sum();
    let low_pct  = low  * 100 / total;
    let high_pct = high * 100 / total;
    if high_pct > 10 {
        ("⬇ clipping — reduce gain", 2)
    } else if low_pct > 90 {
        ("⬆ weak — increase LNA +8 dB", 1)
    } else if low_pct > 70 {
        ("⬆ under-utilised — try +8 dB", 1)
    } else {
        ("✓ gain staging OK", 0)
    }
}

impl Panel for RfChainPanel {
    fn name(&self) -> &'static str { "rf_chain" }
    fn min_size(&self) -> (u16, u16) { (32, 15) }

    fn render(&self, f: &mut Frame, area: ratatui::layout::Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let stale = !state.radio.hw_streaming;
        let title = if stale { " RF Chain [STALE] " } else { " RF Chain " };
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

        let bb_bw = state.radio.bb_filter_hz;
        let total_gain = state.radio.lna_gain as i32
            + state.radio.vga_gain as i32
            + if state.radio.amp_enabled { 14 } else { 0 };

        let lbl  = Style::default().fg(theme.label);
        let val  = Style::default().fg(theme.value);
        let hi   = Style::default().fg(theme.value_hi);

        let (advice_text, advice_sev) = if stale {
            ("--- (RX not streaming)", 0u8)
        } else {
            gain_advice(&state.iq.iq_amplitude_hist)
        };
        let advice_color = if stale { theme.stale } else {
            match advice_sev {
                2 => theme.status_crit,
                1 => theme.status_warn,
                _ => theme.status_ok,
            }
        };

        // ADC utilisation gauge: fraction of samples in mid-range bins (8–23).
        // Show as stale (zero bar, stale color) when RX is not streaming.
        let (util_ratio, util_color) = if stale {
            (0.0, theme.stale)
        } else {
            let total: u64 = state.iq.iq_amplitude_hist.iter().sum();
            let mid: u64   = state.iq.iq_amplitude_hist[8..24].iter().sum();
            let ratio = if total > 0 { mid as f64 / total as f64 } else { 0.0 };
            let color = if ratio > 0.5      { theme.status_ok }
                        else if ratio > 0.2 { theme.status_warn }
                        else                { theme.status_crit };
            (ratio, color)
        };

        // Estimated cascade Noise Figure (Friis)
        let nf_db = estimate_nf_db(state.radio.amp_enabled, state.radio.lna_gain);
        let nf_color = if nf_db < 4.0      { theme.status_ok }
                       else if nf_db < 8.0 { theme.status_warn }
                       else                { theme.status_crit };

        // Frequency display
        let freq_str = format!("{:.3} MHz", state.radio.frequency as f64 / 1_000_000.0);

        // Gain chain: AMP[14] → LNA[xx] → VGA[xx] = total dB
        let chain_line = if state.radio.amp_enabled {
            Line::from(vec![
                Span::styled("AMP", lbl),
                Span::styled(format!("[{}]", 14), hi),
                Span::styled(" → ", lbl),
                Span::styled("LNA", lbl),
                Span::styled(format!("[{}]", state.radio.lna_gain), hi),
                Span::styled(" → ", lbl),
                Span::styled("VGA", lbl),
                Span::styled(format!("[{}]", state.radio.vga_gain), hi),
                Span::styled(format!(" = {} dB", total_gain), Style::default().fg(theme.value_hi)),
            ])
        } else {
            Line::from(vec![
                Span::styled("LNA", lbl),
                Span::styled(format!("[{}]", state.radio.lna_gain), hi),
                Span::styled(" → ", lbl),
                Span::styled("VGA", lbl),
                Span::styled(format!("[{}]", state.radio.vga_gain), hi),
                Span::styled(format!(" = {} dB", total_gain), Style::default().fg(theme.value_hi)),
            ])
        };

        let info_rows: &[Line] = &[
            Line::from(vec![
                Span::styled(format!("{:<13}", "Freq"),      lbl),
                Span::styled(freq_str, hi),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<13}", "BB filter"), lbl),
                Span::styled(fmt_hz(bb_bw), val),
            ]),
            Line::from(vec![Span::raw("")]),
            chain_line,
            Line::from(vec![
                Span::styled(format!("{:<13}", "Est. NF"),  lbl),
                Span::styled(format!("~{:.1} dB", nf_db),  Style::default().fg(nf_color)),
                Span::styled("  (Friis)", Style::default().fg(theme.border_dim)),
            ]),
            {
                let (mds_str, mds_color) = match estimate_mds_dbm(bb_bw, nf_db) {
                    Some(mds) => {
                        let color = if mds < -95.0      { theme.status_ok }
                                    else if mds < -85.0 { theme.status_warn }
                                    else                { theme.status_crit };
                        (format!("~{:.0} dBm", mds), color)
                    }
                    None => ("---".to_string(), theme.stale),
                };
                Line::from(vec![
                    Span::styled(format!("{:<13}", "MDS"),   lbl),
                    Span::styled(mds_str, Style::default().fg(mds_color)),
                ])
            },
            Line::from(vec![Span::raw("")]),
            Line::from(vec![
                Span::styled(format!("{:<13}", "Board"),   lbl),
                Span::styled(Device::board_rev_name(state.system.board_rev), Style::default().fg(theme.border_dim)),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<13}", "USB API"), lbl),
                Span::styled(format!("{:#06x}", state.system.usb_api_version), Style::default().fg(theme.border_dim)),
            ]),
            Line::from(vec![Span::raw("")]),
            Line::from(vec![
                Span::styled(advice_text, Style::default().fg(advice_color)),
            ]),
        ];

        // Reserve 1 row at the bottom for the ADC utilisation ▐ bar
        let n_info = info_rows.len().min(inner.height.saturating_sub(1) as usize);
        if inner.height < 2 { return; }

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(n_info as u16),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(inner);

        let row_constraints: Vec<Constraint> = (0..n_info).map(|_| Constraint::Length(1)).collect();
        let row_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(row_constraints)
            .split(sections[0]);
        for (i, line) in info_rows.iter().take(n_info).enumerate() {
            f.render_widget(Paragraph::new(line.clone()), row_areas[i]);
        }

        crate::ui::charts::draw_hbar(
            f, sections[2], util_ratio,
            "ADC util ",
            &format!("{:.0}%", util_ratio * 100.0),
            util_color, theme,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nf_amp_on_max_gain_is_near_amp_nf() {
        let nf = estimate_nf_db(true, 40);
        assert!(nf > 2.0 && nf < 3.0, "expected ~2.1 dB, got {:.2}", nf);
    }

    #[test]
    fn nf_amp_off_max_lna_gain_near_lna_nf() {
        let nf = estimate_nf_db(false, 40);
        assert!(nf > 3.4 && nf < 4.0, "expected ~3.5 dB, got {:.2}", nf);
    }

    #[test]
    fn nf_degrades_at_lower_lna_gain() {
        let nf_high = estimate_nf_db(false, 40);
        let nf_low  = estimate_nf_db(false,  8);
        assert!(nf_low > nf_high, "NF should be worse at lower LNA gain");
    }

    #[test]
    fn nf_amp_lowers_cascade_nf() {
        let nf_no_amp = estimate_nf_db(false, 24);
        let nf_amp    = estimate_nf_db(true,  24);
        assert!(nf_amp < nf_no_amp, "AMP should improve cascade NF");
    }

    #[test]
    fn gain_advice_clipping_is_crit() {
        let mut hist = [0u64; 32];
        hist[24] = 20; hist[0] = 80; // >10% in high bins
        let (_, sev) = gain_advice(&hist);
        assert_eq!(sev, 2);
    }

    #[test]
    fn gain_advice_weak_is_warn() {
        let mut hist = [0u64; 32];
        hist[0] = 95; hist[8] = 5; // >90% in low bins
        let (_, sev) = gain_advice(&hist);
        assert_eq!(sev, 1);
    }

    #[test]
    fn gain_advice_ok_is_zero() {
        let mut hist = [0u64; 32];
        hist[8] = 50; hist[16] = 50; // mid-range utilisation
        let (_, sev) = gain_advice(&hist);
        assert_eq!(sev, 0);
    }

    #[test]
    fn mds_none_when_bb_filter_zero() {
        assert!(estimate_mds_dbm(0, 3.5).is_none());
    }

    #[test]
    fn mds_10mhz_3_5db_nf() {
        // MDS = -174 + 10*log10(10_000_000) + 3.5 = -174 + 70 + 3.5 = -100.5 dBm
        let mds = estimate_mds_dbm(10_000_000, 3.5).unwrap();
        assert!((mds - (-100.5)).abs() < 0.1, "expected ~-100.5 dBm, got {:.1}", mds);
    }

    #[test]
    fn mds_improves_with_narrower_bw() {
        // Halving BW → -3 dB lower MDS (better sensitivity).
        let mds_wide   = estimate_mds_dbm(10_000_000, 3.5).unwrap();
        let mds_narrow = estimate_mds_dbm( 5_000_000, 3.5).unwrap();
        assert!((mds_wide - mds_narrow - 3.0).abs() < 0.1,
            "halving BW should improve MDS by 3 dB");
    }
}
