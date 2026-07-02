//! `signal_characterization` — the left column of the `lab_signal` preset's
//! redesign (DSN-2026-07).
//!
//! An airy read-out of what the signal at centre *is* and how clean it is, built
//! as a single Line stack fitted with `chrome::fit_spacers` and grouped by the
//! shared `chrome::section` nameplates, exactly like `iq_diagnostics`,
//! `rf_chain`, and `timing_diagnostics`:
//!
//!   1. RADIO HEADLINE   — the peak/noise figure + a status lamp.
//!   2. SIGNAL METRICS   — channel power, peak (+freq), noise floor, occupied BW,
//!      peak hold.
//!   3. ADJACENT CHANNEL — ACPR L/R, a badness-fill bar per side plus the
//!      absolute level of the louder adjacent band.
//!   4. SPECTRAL SHAPE   — C/N trend + crest (Step 7, skeleton for now).
//!
//! Every scalar comes from the latest coherent FFT frame (`state.waterfall.last_fft`),
//! so the numbers agree with the bonded spectrum beside it.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::state::{FftFrame, SdrMetrics};
use crate::ui::chrome::{field, section};
use crate::ui::panel::Panel;

pub struct SignalCharacterizationPanel;

/// Label-column width — clears the longest label ("Channel power" = 13) plus a gap.
const FIELD_W: usize = 14;

/// Status-lamp / headline colour by peak-to-noise: clean ≥ 20 dB, usable ≥ 10 dB,
/// else weak. Same thresholds the signal strip and micro views already use.
fn snr_color(snr: f32, theme: &crate::Theme) -> Color {
    if snr >= 20.0 { theme.status_ok } else if snr >= 10.0 { theme.status_warn } else { theme.status_crit }
}

/// `92.800 MHz` / `1.234500 GHz` — the same precise readout the lab marker bar uses.
fn fmt_freq(hz: u64) -> String {
    if hz >= 1_000_000_000 { format!("{:.6} GHz", hz as f64 / 1e9) }
    else                   { format!("{:.3} MHz", hz as f64 / 1e6) }
}

/// Occupied-bandwidth readout: MHz / kHz / Hz by magnitude.
fn fmt_bw(hz: u64) -> String {
    if hz >= 1_000_000      { format!("{:.3} MHz", hz as f64 / 1e6) }
    else if hz >= 1_000     { format!("{:.1} kHz", hz as f64 / 1e3) }
    else                    { format!("{hz} Hz") }
}

/// The ACPR bar's own display floor — not a regulatory spectral-mask limit (we
/// don't assert one), just how far down this gauge reads before showing a fully
/// clean, empty bar. A ratio at 0 dB (touching the carrier) reads full/red.
const ACPR_BAR_FLOOR_DB: f32 = -80.0;

/// Map an ACPR ratio to a ⅛-block badness bar: more fill = closer to the
/// carrier = worse (green→red, same grading the timing deadline bars use). No
/// reference tick — unlike the timing budget bar, there is no verified
/// regulatory ACPR threshold to mark, so the bar shows the measurement only.
fn acpr_bar(db: f32, bar_w: usize, theme: &crate::Theme) -> Vec<Span<'static>> {
    let clamped = db.clamp(ACPR_BAR_FLOOR_DB, 0.0);
    let badness = ((clamped - ACPR_BAR_FLOOR_DB) * 10.0).round() as u32;
    let max_badness = ((0.0 - ACPR_BAR_FLOOR_DB) * 10.0).round() as u32;
    crate::ui::charts::gain_bar_colored(badness, max_badness, bar_w, theme.status_ok, theme.status_crit, theme.border_dim)
}

/// The strongest live bin as `(level_dbfs, freq_hz)`, mapping the bin index back to
/// frequency across the captured span. `None` for an empty frame.
fn peak_bin(fr: &FftFrame) -> Option<(f32, u64)> {
    let bins = &fr.bins_dbfs;
    let n = bins.len();
    if n == 0 { return None; }
    let mut idx = 0usize;
    let mut best = f32::NEG_INFINITY;
    for (i, &v) in bins.iter().enumerate() {
        if v > best { best = v; idx = i; }
    }
    let left = fr.center_freq_hz as f64 - fr.sample_rate / 2.0;
    let span_frac = if n > 1 { idx as f64 / (n - 1) as f64 } else { 0.0 };
    let freq = (left + span_frac * fr.sample_rate).max(0.0).round() as u64;
    Some((best, freq))
}

impl Panel for SignalCharacterizationPanel {
    fn name(&self) -> &'static str { "signal_characterization" }
    fn min_size(&self) -> (u16, u16) { (30, 12) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        // FFT-driven panel: stale the instant the latest frame ages past the shared
        // 500 ms threshold (or there is no frame yet), like the other signal views.
        let frame = state.waterfall.last_fft.as_ref();
        let stale = frame.map(|fr| fr.timestamp.elapsed().as_millis() > 500).unwrap_or(true);

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
        let iw = inner.width as usize;

        let val = Style::default().fg(theme.value);
        let dim = Style::default().fg(theme.stale);
        let dash = || Span::styled("---".to_string(), dim);

        let mut lines: Vec<Line> = Vec::new();

        // ── RADIO HEADLINE ──────────────────────────────────────────────────
        lines.push(section("RADIO HEADLINE", "", iw, theme));
        if let Some(fr) = frame.filter(|_| !stale) {
            let snr = fr.peak_to_nf_db;
            let col = snr_color(snr, theme);
            let mut hspans = vec![
                Span::raw(" "),
                Span::styled(format!("{snr:.1}"), Style::default().fg(col).add_modifier(Modifier::BOLD)),
                Span::styled(" dB", val),
                Span::styled("  peak / noise", dim),
            ];
            // MOD badge — the classifier's estimate of what's at centre.
            if state.signal.modulation.is_known() {
                hspans.push(Span::styled(
                    format!("   {}", state.signal.modulation.label()),
                    Style::default().fg(theme.value_hi).add_modifier(Modifier::BOLD),
                ));
            }
            hspans.push(Span::styled("   \u{25cf}", Style::default().fg(col)));
            lines.push(Line::from(hspans));
        } else {
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled("\u{25cb} IDLE \u{2014} RX stopped", dim),
            ]));
        }

        lines.push(Line::raw(""));

        // ── SIGNAL METRICS ──────────────────────────────────────────────────
        lines.push(section("SIGNAL METRICS", "", iw, theme));
        let metric = |name: &str, body: Vec<Span<'static>>| -> Line<'static> {
            let mut spans = vec![field(name, FIELD_W, theme)];
            spans.extend(body);
            Line::from(spans)
        };
        if let Some(fr) = frame.filter(|_| !stale) {
            lines.push(metric("Channel power", if fr.channel_power_dbfs.is_finite() {
                vec![Span::styled(format!("{:.1} dBFS", fr.channel_power_dbfs), val)]
            } else { vec![dash()] }));

            lines.push(metric("Peak", match peak_bin(fr) {
                Some((lvl, hz)) => vec![
                    Span::styled(format!("{lvl:.1} dBFS"), val),
                    Span::styled(format!("   {}", fmt_freq(hz)), dim),
                ],
                None => vec![dash()],
            }));

            lines.push(metric("Noise floor", vec![Span::styled(format!("{:.1} dBFS", fr.noise_floor), val)]));

            lines.push(metric("Occupied BW", if fr.occupied_bw_hz > 0 {
                vec![
                    Span::styled(fmt_bw(fr.occupied_bw_hz), val),
                    Span::styled("   99% power", dim),
                ]
            } else { vec![dash()] }));

            let ph = fr.peak_hold.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            lines.push(metric("Peak hold", if ph.is_finite() {
                vec![Span::styled(format!("{ph:.1} dBFS"), val)]
            } else { vec![dash()] }));
        } else {
            for name in ["Channel power", "Peak", "Noise floor", "Occupied BW", "Peak hold"] {
                lines.push(metric(name, vec![dash()]));
            }
        }

        lines.push(Line::raw(""));

        // ── ADJACENT CHANNEL (ACPR) ───────────────────────────────────────────
        lines.push(section("ADJACENT CHANNEL", "ACPR", iw, theme));
        let sig = &state.signal;
        if !stale && sig.acpr_lower_db.is_finite() {
            const LABEL_W: usize = 8;
            for (label, db) in [("L -200k", sig.acpr_lower_db), ("R +200k", sig.acpr_upper_db)] {
                let value_str = format!("{db:.1} dB");
                // lead(1) + label(8) + gap(1) + bar + gap(1) + value
                let bar_w = iw.saturating_sub(1 + LABEL_W + 1 + 1 + value_str.chars().count()).max(6);
                let mut spans = vec![Span::styled(format!(" {label:<LABEL_W$}"), Style::default().fg(theme.label))];
                spans.extend(acpr_bar(db, bar_w, theme));
                spans.push(Span::styled(format!(" {value_str}"), val));
                lines.push(Line::from(spans));
            }
            let adj_freq = if let Some(fr) = frame {
                if sig.acpr_upper_db >= sig.acpr_lower_db {
                    fr.center_freq_hz + crate::state::ACPR_OFFSET_HZ as u64
                } else {
                    fr.center_freq_hz.saturating_sub(crate::state::ACPR_OFFSET_HZ as u64)
                }
            } else { 0 };
            lines.push(metric("Adj carrier", vec![
                Span::styled(format!("{:.1} dBFS", sig.adj_carrier_dbfs), val),
                Span::styled(format!("   {}", fmt_freq(adj_freq)), dim),
            ]));
        } else {
            lines.push(metric("L -200k", vec![dash()]));
            lines.push(metric("R +200k", vec![dash()]));
        }

        lines.push(Line::raw(""));

        // ── SPECTRAL SHAPE ─────────────────────────────────────────────────
        lines.push(section("SPECTRAL SHAPE", "60 s", iw, theme));
        if !stale {
            // C/N trend: reuses `snr_history` (already fed ~500 ms by the rx poll
            // task, [`crate::state::SNR_HISTORY_LEN`] = 120 deep → 60 s), the same
            // ring the Command Rail's SNR trace and the micro views read. No new
            // state — C/N ≈ peak/noise = SNR.
            const LABEL: &str = "C/N trend";
            const TREND_ANN_W: usize = 10; // budget for "±NN.N dB"
            let snr_hist: Vec<f32> = sig.snr_history.iter().copied().collect();
            let spark_w = iw.saturating_sub(1 + LABEL.chars().count() + 1 + TREND_ANN_W).max(1);
            let (spark, p2p) = crate::ui::micro_common::spark_minmax(&snr_hist, spark_w);
            if !spark.is_empty() {
                let ann = format!("\u{00b1}{:.1} dB", p2p / 2.0);
                let used = 1 + LABEL.chars().count() + 1 + spark.chars().count() + 1 + ann.chars().count();
                let pad = iw.saturating_sub(used).max(1);
                lines.push(Line::from(vec![
                    Span::raw(" "),
                    Span::styled(LABEL, Style::default().fg(theme.label)),
                    Span::raw(" "),
                    Span::styled(spark, val),
                    Span::raw(" ".repeat(pad)),
                    Span::styled(ann, dim),
                ]));
            } else {
                lines.push(Line::from(vec![Span::raw(" "), Span::styled(LABEL, Style::default().fg(theme.label)), Span::raw("  "), dash()]));
            }

            // Crest / PAPR: reuses the exact ADC-loading model from the Lab RF
            // bench (`rf_calc::adc_loading`) rather than re-deriving peak-minus-rms
            // — full-bandwidth ADC crest factor, the same honest proxy the RF lab
            // already shows for "constant-envelope vs peaky".
            let n: u64 = state.iq.adc_signed_hist.iter().sum();
            let load = crate::ui::rf_calc::adc_loading(
                sig.adc_peak_dbfs as f64, sig.adc_rms_dbfs as f64, sig.adc_clip_events, n);
            lines.push(metric("Crest / PAPR", vec![Span::styled(format!("{:.1} dB", load.crest_db), val)]));
        } else {
            lines.push(metric("C/N trend", vec![dash()]));
            lines.push(metric("Crest / PAPR", vec![dash()]));
        }

        crate::ui::chrome::fit_spacers(&mut lines, inner.height as usize);
        f.render_widget(Paragraph::new(lines), inner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Instant;

    fn frame(bins: Vec<f32>, center: u64, sr: f64) -> FftFrame {
        FftFrame {
            peak_hold: Arc::new(bins.clone()),
            bins_dbfs: Arc::new(bins),
            noise_floor: -90.0,
            center_freq_hz: center,
            sample_rate: sr,
            timestamp: Instant::now(),
            peak_to_nf_db: 40.0,
            channel_power_dbfs: -22.0,
            occupied_bw_hz: 180_000,
            enbw_hz: 1_000.0,
        }
    }

    #[test]
    fn panel_name_is_stable() {
        assert_eq!(SignalCharacterizationPanel.name(), "signal_characterization");
    }

    #[test]
    fn peak_bin_maps_index_to_frequency() {
        // 5 bins across 2 MHz centred at 100 MHz → span 99..101 MHz. Peak at the
        // centre bin (idx 2) maps back to the centre frequency.
        let fr = frame(vec![-80.0, -60.0, -10.0, -60.0, -80.0], 100_000_000, 2_000_000.0);
        let (lvl, hz) = peak_bin(&fr).unwrap();
        assert!((lvl + 10.0).abs() < 1e-6, "peak level is the max bin");
        assert_eq!(hz, 100_000_000, "centre bin → centre frequency");
    }

    #[test]
    fn peak_bin_edge_bins_hit_span_ends() {
        let fr = frame(vec![-10.0, -80.0, -80.0, -80.0, -80.0], 100_000_000, 2_000_000.0);
        let (_, hz) = peak_bin(&fr).unwrap();
        assert_eq!(hz, 99_000_000, "first bin → left edge of the span");
    }

    #[test]
    fn peak_bin_empty_frame_is_none() {
        let fr = frame(vec![], 100_000_000, 2_000_000.0);
        assert!(peak_bin(&fr).is_none());
    }

    #[test]
    fn snr_color_thresholds() {
        let t = crate::theme::Theme::sdr();
        assert_eq!(snr_color(25.0, &t), t.status_ok);
        assert_eq!(snr_color(15.0, &t), t.status_warn);
        assert_eq!(snr_color(5.0, &t), t.status_crit);
    }

    #[test]
    fn acpr_bar_width_matches_bar_w() {
        let t = crate::theme::Theme::sdr();
        let spans = acpr_bar(-38.0, 24, &t);
        let w: usize = spans.iter().map(|s| s.content.chars().count()).sum();
        assert_eq!(w, 24);
    }

    #[test]
    fn acpr_bar_touching_carrier_is_full_red() {
        let t = crate::theme::Theme::sdr();
        let spans = acpr_bar(0.0, 10, &t);
        assert_eq!(spans.last().unwrap().style.fg, Some(t.status_crit));
    }

    #[test]
    fn acpr_bar_below_floor_is_empty() {
        let t = crate::theme::Theme::sdr();
        let spans = acpr_bar(-95.0, 10, &t);
        let s: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(s.chars().all(|c| c == ' '), "below the display floor reads as clean/empty: {s:?}");
    }

    #[test]
    fn fmt_bw_picks_units() {
        assert_eq!(fmt_bw(180_000), "180.0 kHz");
        assert_eq!(fmt_bw(1_500_000), "1.500 MHz");
        assert_eq!(fmt_bw(400), "400 Hz");
    }
}
