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

fn snr_color(db: f32, theme: &crate::Theme) -> Color {
    if db >= 20.0 { theme.status_ok } else if db >= 10.0 { theme.status_warn } else { theme.status_crit }
}

fn sat_color(pct: f32, theme: &crate::Theme) -> Color {
    if pct < 1.0 { theme.status_ok } else if pct < 5.0 { theme.status_warn } else { theme.status_crit }
}

fn drop_color(drops: u64, theme: &crate::Theme) -> Color {
    if drops == 0 { theme.status_ok } else if drops < 10 { theme.status_warn } else { theme.status_crit }
}

fn buf_color(pct: f32, theme: &crate::Theme) -> Color {
    if pct < 50.0 { theme.status_ok } else if pct < 80.0 { theme.status_warn } else { theme.status_crit }
}

fn iq_color(db: f32, theme: &crate::Theme) -> Color {
    if db.abs() < 1.0 { theme.status_ok } else if db.abs() < 3.0 { theme.status_warn } else { theme.status_crit }
}

fn fmt_rbw(hz: f64) -> String {
    if hz >= 1_000.0 { format!("{:.1} kHz", hz / 1_000.0) }
    else { format!("{:.0} Hz", hz) }
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

        let snr_str = if stale { "---".into() } else { format!("{:.1} dB", state.signal.peak_to_nf_db) };
        let snr_col = if stale { theme.stale } else { snr_color(state.signal.peak_to_nf_db, theme) };

        let pwr_str = if stale || !state.signal.channel_power_dbfs.is_finite() {
            "---".into()
        } else {
            format!("{:.1} dBFS", state.signal.channel_power_dbfs)
        };
        let pwr_col = if stale { theme.stale } else { theme.value };

        let (nf_str, nf_col) = match state.waterfall.last_fft.as_ref().filter(|_| !stale) {
            Some(fr) => (format!("{:.1} dBFS", fr.noise_floor), theme.value),
            None     => ("---".into(), theme.stale),
        };

        let (rbw_str, rbw_col) = match state.waterfall.last_fft.as_ref().filter(|_| !stale) {
            Some(fr) if fr.enbw_hz > 0.0 => (fmt_rbw(fr.enbw_hz), theme.value),
            _ => ("---".into(), theme.stale),
        };

        let iq_str = format!("{:+.1} dB", state.iq.iq_imbalance_db);
        let iq_col = iq_color(state.iq.iq_imbalance_db, theme);

        let buf_str = format!("{:.0}%", state.iq.buf_fill_pct);
        let buf_col = buf_color(state.iq.buf_fill_pct, theme);

        let line = Line::from(vec![
            Span::raw(" "),
            lbl("P/NF "), val(snr_str, snr_col),
            sep.clone(),
            lbl("PWR "),  val(pwr_str, pwr_col),
            sep.clone(),
            lbl("NF "),   val(nf_str, nf_col),
            sep.clone(),
            lbl("SAT "),  val(format!("{:.1}%", state.signal.adc_saturation_pct),
                              sat_color(state.signal.adc_saturation_pct, theme)),
            sep.clone(),
            lbl("DROP "), val(format!("{}/s", state.signal.drops_per_sec),
                              drop_color(state.signal.drops_per_sec, theme)),
            sep.clone(),
            lbl("BUF "),  val(buf_str, buf_col),
            sep.clone(),
            lbl("IQ "),   val(iq_str, iq_col),
            sep.clone(),
            lbl("RBW "),  val(rbw_str, rbw_col),
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
