use std::collections::VecDeque;

/// ADC saturation at or above this percent records a clip event for the Command
/// Rail's alert-memory. Aligned with the SAT "warn" colour — real clipping that's
/// worth remembering, not measurement noise.
pub const SAT_CLIP_PCT: f32 = 10.0;

/// Adjacent-channel offset from centre for the ACPR measurement, fixed at the
/// mockup's FM-broadcast spacing. This is sdrtop's own display convention, not
/// a regulatory channel-plan value — it applies to whatever the measured
/// occupied bandwidth turns out to be. Shared by the FFT worker (which computes
/// the ratio) and the characterization panel (which labels the adjacent-band
/// frequency from it), so the two never drift apart.
pub const ACPR_OFFSET_HZ: f64 = 200_000.0;

/// A rough modulation estimate for the signal at centre. Honest by design: a
/// bandwidth heuristic (see [`classify`]), not a demodulating classifier. The
/// demod phase refines it (e.g. WFM confirmed by a 19 kHz pilot lock).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Modulation {
    /// No clear carrier to characterize (weak signal), or a shape that does not
    /// fit the known bands.
    #[default]
    Unknown,
    /// Wide-band FM broadcast (~180 kHz occupied).
    Wfm,
    /// Narrow-band FM voice / data (~11–30 kHz).
    Nfm,
    /// Amplitude modulation / narrow voice (< 11 kHz).
    Am,
}

impl Modulation {
    /// Short badge label for the banner / headline: `WFM` / `NFM` / `AM` / `—`.
    pub fn label(self) -> &'static str {
        match self {
            Modulation::Wfm     => "WFM",
            Modulation::Nfm     => "NFM",
            Modulation::Am      => "AM",
            Modulation::Unknown => "\u{2014}",
        }
    }

    /// Whether a modulation was confidently classified (not the no-signal fallback).
    pub fn is_known(self) -> bool { !matches!(self, Modulation::Unknown) }
}

/// Minimum peak-to-noise (dB) for [`classify`] to commit to a modulation. Below
/// this there is no clear carrier at centre, so it reports [`Modulation::Unknown`]
/// rather than labelling noise.
pub const CLASSIFY_MIN_SNR_DB: f32 = 10.0;

/// Estimate the modulation of the signal at centre from its 99% occupied
/// bandwidth, gated on signal presence. Deliberately conservative: a bandwidth
/// heuristic, so the wide/narrow split is trustworthy while the AM vs NFM boundary
/// is a best guess the demod phase can sharpen.
pub fn classify(snr_db: f32, occupied_bw_hz: u64) -> Modulation {
    if snr_db < CLASSIFY_MIN_SNR_DB || occupied_bw_hz == 0 {
        return Modulation::Unknown;
    }
    match occupied_bw_hz {
        bw if bw >= 100_000 => Modulation::Wfm,
        bw if bw >= 11_000  => Modulation::Nfm,
        _                   => Modulation::Am,
    }
}

#[derive(Clone)]
pub struct SignalState {
    pub drops_per_sec:       u64,
    pub total_drops_session: u64,
    pub drop_history:        VecDeque<u64>,
    pub adc_saturation_pct:  f32,
    pub adc_saturation_peak: f32,
    pub saturation_history:  VecDeque<f32>,
    pub peak_to_nf_db:       f32,
    pub channel_power_dbfs:  f32,
    pub occupied_bw_hz:      u64,
    /// Adjacent-channel power ratio, dB relative to the in-channel power, at
    /// ±[`ACPR_OFFSET_HZ`]. `f32::NEG_INFINITY` when there is nothing to compare
    /// against yet (no occupied bandwidth) or a band falls outside the captured
    /// span — never a guessed number.
    pub acpr_lower_db:       f32,
    pub acpr_upper_db:       f32,
    /// Absolute level (dBFS) of the louder — worse — of the two adjacent bands.
    /// Paired with `acpr_lower_db` / `acpr_upper_db`; same undefined sentinel.
    pub adj_carrier_dbfs:    f32,
    pub usb_errors_session:   u64,
    pub usb_errors_last_poll: u64,
    pub usb_error_history:    std::collections::VecDeque<u64>,
    /// Recent SNR (peak/noise-floor) samples, pushed by the rx poll task roughly
    /// every 500 ms while streaming. Powers the micro_signal trend arrow.
    pub snr_history:          VecDeque<f32>,
    /// Recent channel-power (dBFS) samples — pushed alongside `snr_history` at the
    /// same ~500 ms cadence. Powers the command rail's PWR sparkline + trend.
    pub pwr_history:          VecDeque<f32>,
    /// Recent noise-floor (dBFS) samples — pushed alongside `snr_history`. Powers
    /// the command rail's NF sparkline + trend.
    pub nf_history:           VecDeque<f32>,
    /// Recent ADC-saturation (%) samples — pushed alongside `snr_history` at the
    /// same ~500 ms / [`crate::state::SNR_HISTORY_LEN`] depth so the command rail's
    /// SAT trace fills like the other three. Distinct from [`Self::saturation_history`],
    /// which feeds the health panels' mini-graph at the 200 ms / 64-deep cadence.
    pub sat_history:          VecDeque<f32>,
    /// Unix-epoch second of the most recent ADC clip (saturation ≥ [`SAT_CLIP_PCT`]),
    /// for the rail's fading "last clip Xs" alert-memory. `None` = none this session.
    pub last_clip_at:         Option<u64>,
    /// Rough modulation estimate for the signal at centre, refreshed each display
    /// frame by the FFT worker via [`classify`]. Drives the lab_signal headline /
    /// banner and, later, the demod panel's mode-adaptive view.
    pub modulation:           Modulation,
    /// ADC loading for the Lab RF bench, refreshed each ~200 ms window: the loudest
    /// sample (`adc_peak_dbfs`), the full-bandwidth RMS level (`adc_rms_dbfs`, total
    /// I/Q power vs full scale — distinct from the in-channel `channel_power_dbfs`),
    /// and the clipped-sample count in the last window (`adc_clip_events`).
    pub adc_peak_dbfs:        f32,
    pub adc_rms_dbfs:         f32,
    pub adc_clip_events:      u64,
}

impl SignalState {
    /// Short-term SNR trend in dB: mean of the most recent half of
    /// `snr_history` minus the mean of the older half. Positive means the
    /// signal is strengthening. `None` until there are enough samples.
    pub fn snr_delta(&self) -> Option<f32> {
        let n = self.snr_history.len();
        if n < 4 { return None; }
        let half = n / 2;
        let older:  f32 = self.snr_history.iter().take(half).sum::<f32>() / half as f32;
        let recent: f32 = self.snr_history.iter().skip(n - half).sum::<f32>() / half as f32;
        Some(recent - older)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_history(samples: &[f32]) -> SignalState {
        let mut s = SignalState {
            drops_per_sec: 0, total_drops_session: 0, drop_history: VecDeque::new(),
            adc_saturation_pct: 0.0, adc_saturation_peak: 0.0, saturation_history: VecDeque::new(),
            peak_to_nf_db: 0.0, channel_power_dbfs: 0.0, occupied_bw_hz: 0,
            acpr_lower_db: f32::NEG_INFINITY, acpr_upper_db: f32::NEG_INFINITY,
            adj_carrier_dbfs: f32::NEG_INFINITY,
            usb_errors_session: 0, usb_errors_last_poll: 0, usb_error_history: VecDeque::new(),
            snr_history: VecDeque::new(), pwr_history: VecDeque::new(), nf_history: VecDeque::new(),
            sat_history: VecDeque::new(),
            last_clip_at: None,
            modulation: Modulation::Unknown,
            adc_peak_dbfs: 0.0, adc_rms_dbfs: 0.0, adc_clip_events: 0,
        };
        s.snr_history.extend(samples.iter().copied());
        s
    }

    #[test]
    fn classify_gates_on_signal_presence() {
        // Weak carrier or no occupancy → no guess.
        assert_eq!(classify(5.0, 180_000), Modulation::Unknown);
        assert_eq!(classify(40.0, 0),      Modulation::Unknown);
    }

    #[test]
    fn classify_bands_by_occupied_bandwidth() {
        assert_eq!(classify(40.0, 180_000), Modulation::Wfm);
        assert_eq!(classify(40.0, 100_000), Modulation::Wfm); // wide boundary
        assert_eq!(classify(40.0, 15_000),  Modulation::Nfm);
        assert_eq!(classify(40.0, 11_000),  Modulation::Nfm); // narrow-FM boundary
        assert_eq!(classify(40.0, 8_000),   Modulation::Am);
    }

    #[test]
    fn modulation_labels_and_known_flag() {
        assert_eq!(Modulation::Wfm.label(), "WFM");
        assert_eq!(Modulation::Unknown.label(), "\u{2014}");
        assert!(Modulation::Nfm.is_known());
        assert!(!Modulation::Unknown.is_known());
    }

    #[test]
    fn snr_delta_none_with_too_few_samples() {
        assert_eq!(with_history(&[10.0, 12.0, 14.0]).snr_delta(), None);
    }

    #[test]
    fn snr_delta_positive_when_rising() {
        // older half avg = 10, recent half avg = 20 → +10
        let d = with_history(&[10.0, 10.0, 20.0, 20.0]).snr_delta().unwrap();
        assert!((d - 10.0).abs() < 1e-6, "got {d}");
    }

    #[test]
    fn snr_delta_negative_when_falling() {
        let d = with_history(&[20.0, 20.0, 12.0, 12.0]).snr_delta().unwrap();
        assert!((d + 8.0).abs() < 1e-6, "got {d}");
    }
}
