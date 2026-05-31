use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::state::SdrMetrics;
use super::panel::Panel;

pub struct SignalStripPanel;

fn thresh(val: f64, ok: f64, warn: f64, theme: &crate::Theme) -> Color {
    if val < ok { theme.status_ok } else if val < warn { theme.status_warn } else { theme.status_crit }
}

fn snr_color(db: f32, theme: &crate::Theme) -> Color {
    if db >= 20.0 { theme.status_ok } else if db >= 10.0 { theme.status_warn } else { theme.status_crit }
}

fn sat_color(pct: f32, theme: &crate::Theme) -> Color {
    thresh(pct as f64, 1.0, 5.0, theme)
}

fn drop_color(drops: u64, theme: &crate::Theme) -> Color {
    thresh(drops as f64, 1.0, 10.0, theme)
}

fn jit_color(us: u64, theme: &crate::Theme) -> Color {
    thresh(us as f64, 100.0, 1000.0, theme)
}

fn cpu_color(pct: f32, theme: &crate::Theme) -> Color {
    thresh(pct as f64, 50.0, 80.0, theme)
}

fn fmt_occ(hz: u64) -> String {
    if hz == 0 { return "---".into(); }
    if hz >= 1_000_000 { format!("{:.2} MHz", hz as f64 / 1_000_000.0) }
    else if hz >= 1_000 { format!("{:.1} kHz", hz as f64 / 1_000.0) }
    else { format!("{} Hz", hz) }
}

impl Panel for SignalStripPanel {
    fn name(&self) -> &'static str { "signal_strip" }
    fn min_size(&self) -> (u16, u16) { (60, 3) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, _focused: bool) {
        let stale = state.waterfall.last_fft.as_ref()
            .map(|fr| fr.timestamp.elapsed().as_millis() > 500)
            .unwrap_or(true);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_dim));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let sep = Span::styled("  ·  ", Style::default().fg(theme.border_dim));
        let lbl = |s: &'static str| Span::styled(s, Style::default().fg(theme.label));
        let val = |s: String, c: Color| Span::styled(s, Style::default().fg(c));

        let snr_str = if stale { "---".into() } else { format!("{:.1} dB", state.signal.snr_db) };
        let snr_col = if stale { theme.stale } else { snr_color(state.signal.snr_db, theme) };

        let pwr_str = if stale || !state.signal.channel_power_dbfs.is_finite() {
            "--- dBFS".into()
        } else {
            format!("{:.1} dBFS", state.signal.channel_power_dbfs)
        };
        let pwr_col = if stale { theme.stale } else { theme.value };

        let occ_str = if stale { "---".into() } else { fmt_occ(state.signal.occupied_bw_hz) };
        let occ_col = if stale { theme.stale } else { theme.value };

        let line = Line::from(vec![
            Span::raw(" "),
            lbl("SNR "), val(snr_str, snr_col),
            sep.clone(),
            lbl("PWR "), val(pwr_str, pwr_col),
            sep.clone(),
            lbl("SAT "), val(format!("{:.1}%", state.signal.adc_saturation_pct),
                             sat_color(state.signal.adc_saturation_pct, theme)),
            sep.clone(),
            lbl("DROP "), val(format!("{}/s", state.signal.drops_per_sec),
                              drop_color(state.signal.drops_per_sec, theme)),
            sep.clone(),
            lbl("OCC "), val(occ_str, occ_col),
            sep.clone(),
            lbl("JIT "), val(format!("{} µs", state.iq.callback_jitter_us),
                             jit_color(state.iq.callback_jitter_us, theme)),
            sep.clone(),
            lbl("CPU "), val(format!("{:.1}%", state.system.process_cpu_pct),
                             cpu_color(state.system.process_cpu_pct, theme)),
            sep.clone(),
            lbl("RAM "), val(format!("{} MB", state.system.process_rss_mb), theme.value),
        ]);

        f.render_widget(Paragraph::new(line), inner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    #[test]
    fn snr_color_thresholds() {
        let t = Theme::sdr();
        assert_eq!(snr_color(25.0, &t), t.status_ok);
        assert_eq!(snr_color(15.0, &t), t.status_warn);
        assert_eq!(snr_color(5.0,  &t), t.status_crit);
    }

    #[test]
    fn sat_color_thresholds() {
        let t = Theme::sdr();
        assert_eq!(sat_color(0.5, &t), t.status_ok);
        assert_eq!(sat_color(2.0, &t), t.status_warn);
        assert_eq!(sat_color(8.0, &t), t.status_crit);
    }

    #[test]
    fn drop_color_thresholds() {
        let t = Theme::sdr();
        assert_eq!(drop_color(0,  &t), t.status_ok);
        assert_eq!(drop_color(5,  &t), t.status_warn);
        assert_eq!(drop_color(15, &t), t.status_crit);
    }

    #[test]
    fn fmt_occ_formats_correctly() {
        assert_eq!(fmt_occ(0),           "---");
        assert_eq!(fmt_occ(500),         "500 Hz");
        assert_eq!(fmt_occ(1_500),       "1.5 kHz");
        assert_eq!(fmt_occ(1_250_000),   "1.25 MHz");
    }
}
