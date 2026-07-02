use std::sync::{Arc, Mutex};

use crossbeam_channel::Receiver;
use num_complex::Complex;
use rustfft::FftPlanner;

use super::dsp::{self, WindowFn};
use crate::hardware::SampleFormat;
use crate::state::{FftFrame, SdrMetrics};

const DB_FLOOR: f32 = -160.0;

pub struct FftWorker {
    pub sample_rx: Receiver<Vec<u8>>,
    pub state: Arc<Mutex<SdrMetrics>>,
    pub fft_size: usize,
    pub window_fn: WindowFn,
    pub ema_alpha: f32,
    pub peak_decay_db: f32,
    /// How to decode the raw bytes — set from the active device's capabilities.
    pub format: SampleFormat,
}

impl FftWorker {
    pub fn new(sample_rx: Receiver<Vec<u8>>, state: Arc<Mutex<SdrMetrics>>, format: SampleFormat) -> Self {
        Self {
            sample_rx,
            state,
            fft_size: 2048,
            window_fn: WindowFn::Hann,
            ema_alpha: 0.2,
            peak_decay_db: 0.5,
            format,
        }
    }

    pub fn run(self) {
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(self.fft_size);
        let window = dsp::compute_window(self.window_fn, self.fft_size);
        let n = self.fft_size;

        // ENBW coefficient: N × Σ(w²) / (Σ(w))² — exact for whatever window is used.
        // Hann ≈ 1.5, Hamming ≈ 1.36, Blackman ≈ 1.73.
        let w_sum_sq: f64 = window.iter().map(|&w| (w as f64).powi(2)).sum();
        let w_sum:    f64 = window.iter().map(|&w| w as f64).sum();
        let enbw_coeff = n as f64 * w_sum_sq / (w_sum * w_sum);

        // Pre-allocate all scratch buffers — reused every frame, zero heap churn
        let mut buf: Vec<u8> = Vec::new();
        let mut samples: Vec<Complex<f32>> = vec![Complex::default(); n];
        let mut mags:    Vec<f32>          = vec![0.0; n];
        let mut shifted: Vec<f32>          = vec![0.0; n];
        let mut smoothed: Vec<f32>         = vec![DB_FLOOR; n];
        let mut peak: Vec<f32>             = vec![DB_FLOOR; n];
        // scratch for noise floor partial sort (avoids O(n log n) full sort)
        let mut noise_scratch: Vec<f32>    = vec![0.0; n];
        // linear power per bin — computed once per display frame, reused for channel
        // power, occupied BW, and per-marker BW (avoids repeated 10^(x/10) powf calls)
        let mut linear: Vec<f32>           = vec![0.0; n];
        let mut initialized = false;

        // Throttle state writes to ~30 fps — EMA runs on every frame for accuracy,
        // but the expensive analysis + state lock fires at display rate only.
        const UPDATE_INTERVAL: std::time::Duration = std::time::Duration::from_millis(33);
        // Pace the drawn spectrum to the *visible* waterfall: each waterfall
        // character row packs this many data rows (half-block ▀), so the spectrum
        // refreshes once per visible line rather than every FFT frame, keeping the
        // two panels moving in lockstep. Signal metrics stay at full rate.
        const ROWS_PER_WATERFALL_LINE: u32 = 2;
        // Never let the drawn frame age past the panels' 500 ms STALE threshold,
        // even at large frames/row strides.
        const SPECTRUM_STALE_GUARD: std::time::Duration = std::time::Duration::from_millis(400);
        let mut last_state_update = std::time::Instant::now()
            .checked_sub(UPDATE_INTERVAL)
            .unwrap_or_else(std::time::Instant::now);
        let mut rows_since_spectrum: u32 = 0;
        let mut last_spectrum_update = std::time::Instant::now()
            .checked_sub(SPECTRUM_STALE_GUARD)
            .unwrap_or_else(std::time::Instant::now);
        // Live EMA factor: refreshed once per display frame from the lab's `AVG ×N`
        // control (alpha = 1/N), so trace averaging is adjustable without rebuilding
        // the worker. Starts at the worker's configured default.
        let mut current_alpha = self.ema_alpha;

        while let Ok(chunk) = self.sample_rx.recv() {
            buf.extend_from_slice(&chunk);

            let frame_bytes = n * 2;
            let mut buf_start = 0usize;

            while buf.len() - buf_start >= frame_bytes {
                let frame = &buf[buf_start..buf_start + frame_bytes];

                // Convert to windowed complex samples in-place. Branch the
                // byte→sample decode once per frame, not per sample.
                match self.format {
                    SampleFormat::Int8 => {
                        for (i, (pair, &w)) in frame.chunks_exact(2).zip(window.iter()).enumerate() {
                            samples[i] = Complex {
                                re: pair[0] as i8 as f32 / 128.0 * w,
                                im: pair[1] as i8 as f32 / 128.0 * w,
                            };
                        }
                    }
                    SampleFormat::Uint8 => {
                        for (i, (pair, &w)) in frame.chunks_exact(2).zip(window.iter()).enumerate() {
                            samples[i] = Complex {
                                re: (pair[0] as f32 - 127.5) / 127.5 * w,
                                im: (pair[1] as f32 - 127.5) / 127.5 * w,
                            };
                        }
                    }
                }
                buf_start += frame_bytes;

                fft.process(&mut samples);

                // Magnitude → dBFS in-place
                let n_f32 = n as f32;
                for (i, z) in samples.iter().enumerate() {
                    let norm = z.norm() / n_f32;
                    mags[i] = if norm > 0.0 { 20.0 * norm.log10() } else { DB_FLOOR };
                }

                // fftshift in-place
                shifted[..n / 2].copy_from_slice(&mags[n / 2..]);
                shifted[n / 2..].copy_from_slice(&mags[..n / 2]);

                // EMA smoothing + peak hold — run on every frame for accurate averaging
                if !initialized {
                    smoothed.copy_from_slice(&shifted);
                    peak.copy_from_slice(&shifted);
                    initialized = true;
                } else {
                    let alpha = current_alpha;
                    let one_minus = 1.0 - alpha;
                    for (s, &new) in smoothed.iter_mut().zip(shifted.iter()) {
                        *s = alpha * new + one_minus * *s;
                    }
                    let decay = self.peak_decay_db;
                    for (p, &s) in peak.iter_mut().zip(smoothed.iter()) {
                        *p = (*p - decay).max(s);
                    }
                }

                // Throttle: skip expensive analysis + state update until next display frame
                if last_state_update.elapsed() < UPDATE_INTERVAL {
                    continue;
                }
                last_state_update = std::time::Instant::now();

                // Noise floor: mean of bottom 10% via partial sort — O(n) average
                noise_scratch.copy_from_slice(&smoothed);
                let nf_count = (n / 10).max(1);
                noise_scratch.select_nth_unstable_by(nf_count - 1, |a, b| {
                    a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                });
                let noise_floor = noise_scratch[..nf_count].iter().sum::<f32>() / nf_count as f32;

                // Read freq/rate while unlocked from the FFT loop
                let (center_freq_hz, sample_rate) = self
                    .state
                    .lock()
                    .map(|m| (m.radio.frequency, m.radio.config_sample_rate))
                    .unwrap_or((0, 0.0));

                // SNR: peak minus noise floor
                let peak_dbfs = smoothed.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let peak_to_nf_db = (peak_dbfs - noise_floor).max(0.0);

                // Single linear-power pass — 10^(x/10) is expensive; compute once,
                // reuse for channel power, occupied BW, and per-marker BW below.
                for (l, &s) in linear.iter_mut().zip(smoothed.iter()) {
                    *l = 10f32.powf(s / 10.0);
                }
                let total_linear: f32 = linear.iter().sum();

                // Channel power: integrate all bins → dBFS
                let channel_power_dbfs = if total_linear > 0.0 {
                    10.0 * total_linear.log10()
                } else {
                    f32::NEG_INFINITY
                };

                // 99% occupied BW — ITU-R SM.328 cumulative method:
                // exclude the bottom 0.5% and top 0.5% of total power,
                // return the frequency span between those two cut-off points.
                let occupied_bw_hz = if total_linear > 0.0 && sample_rate > 0.0 {
                    let lo_thresh = total_linear * 0.005;
                    let hi_thresh = total_linear * 0.995;
                    let bin_hz    = sample_rate / n as f64;
                    let mut acc   = 0f32;
                    let mut lo_bin = 0usize;
                    let mut hi_bin = n - 1;
                    for (i, &lin) in linear.iter().enumerate() {
                        acc += lin;
                        if acc < lo_thresh { lo_bin = i; }
                        if acc < hi_thresh { hi_bin = i; }
                    }
                    ((hi_bin.saturating_sub(lo_bin) + 1) as f64 * bin_hz) as u64
                } else {
                    0u64
                };

                if let Ok(mut m) = self.state.lock() {
                    // Refresh the averaging factor from the lab control (cheap read
                    // under the lock we already hold for the result write-back).
                    current_alpha = m.lab.ema_alpha();
                    m.signal.peak_to_nf_db      = peak_to_nf_db;
                    m.signal.channel_power_dbfs = channel_power_dbfs;
                    m.signal.occupied_bw_hz     = occupied_bw_hz;
                    m.signal.modulation         = crate::state::classify(peak_to_nf_db, occupied_bw_hz);

                    // Per-marker occupied BW within each marker's channel window
                    if sample_rate > 0.0 {
                        let bin_hz  = sample_rate / n as f64;
                        let left_hz = center_freq_hz as f64 - sample_rate / 2.0;
                        let right_hz = left_hz + sample_rate;
                        for mk in m.spectrum.markers.iter_mut() {
                            if let Some(ch_bw) = mk.channel_bw_hz {
                                let mf = mk.freq_hz as f64;
                                // Skip markers outside the current band
                                if mf < left_hz || mf > right_hz {
                                    mk.measured_bw_hz = None;
                                    continue;
                                }
                                let lo_hz  = mf - ch_bw as f64 / 2.0;
                                let hi_hz  = mf + ch_bw as f64 / 2.0;
                                let lo_bin = ((lo_hz - left_hz) / bin_hz).floor().max(0.0) as usize;
                                let hi_bin = ((hi_hz - left_hz) / bin_hz).ceil().min((n - 1) as f64) as usize;
                                if lo_bin <= hi_bin && hi_bin < n {
                                    let lin_slice = &linear[lo_bin..=hi_bin];
                                    let tot: f32 = lin_slice.iter().sum();
                                    if tot > 0.0 {
                                        let lo_t = tot * 0.005;
                                        let hi_t = tot * 0.995;
                                        let mut acc = 0f32;
                                        let mut lo_b = 0usize;
                                        let mut hi_b = lin_slice.len() - 1;
                                        for (i, &lin) in lin_slice.iter().enumerate() {
                                            acc += lin;
                                            if acc < lo_t { lo_b = i; }
                                            if acc < hi_t { hi_b = i; }
                                        }
                                        mk.measured_bw_hz = Some(((hi_b.saturating_sub(lo_b) + 1) as f64 * bin_hz) as u64);
                                    }
                                } else {
                                    mk.measured_bw_hz = None;
                                }
                            }
                        }
                    }

                    // Advance the waterfall every display frame.
                    let row_materialized = m.waterfall.buffer.push(&smoothed);
                    if row_materialized {
                        rows_since_spectrum += 1;
                    }

                    // Refresh the drawn spectrum once per visible waterfall line
                    // (or sooner if it would otherwise age toward STALE). This is
                    // the only display-paced write; the metrics above ran at full
                    // rate.
                    if m.waterfall.last_fft.is_none()
                        || rows_since_spectrum >= ROWS_PER_WATERFALL_LINE
                        || last_spectrum_update.elapsed() >= SPECTRUM_STALE_GUARD
                    {
                        rows_since_spectrum = 0;
                        last_spectrum_update = std::time::Instant::now();

                        // Reclaim the Vec allocations from the previous FftFrame
                        // before overwriting it.  We hold the mutex, so refcount == 1
                        // and try_unwrap is guaranteed to succeed — no heap alloc.
                        let (mut bins_vec, mut peak_vec) = match m.waterfall.last_fft.take() {
                            Some(old) => (
                                Arc::try_unwrap(old.bins_dbfs).unwrap_or_else(|_| vec![0.0_f32; n]),
                                Arc::try_unwrap(old.peak_hold).unwrap_or_else(|_| vec![0.0_f32; n]),
                            ),
                            None => (vec![0.0_f32; n], vec![0.0_f32; n]),
                        };
                        bins_vec.copy_from_slice(&smoothed);
                        peak_vec.copy_from_slice(&peak);
                        let bins_arc = Arc::new(bins_vec);
                        let peak_arc = Arc::new(peak_vec);

                        m.waterfall.last_fft = Some(FftFrame {
                            bins_dbfs: bins_arc,
                            peak_hold: peak_arc,
                            noise_floor,
                            center_freq_hz,
                            sample_rate,
                            timestamp: std::time::Instant::now(),
                            peak_to_nf_db,
                            channel_power_dbfs,
                            occupied_bw_hz,
                            enbw_hz: enbw_coeff * sample_rate / n as f64,
                        });
                    }
                }
            }

            // Single drain per received chunk instead of one per FFT frame
            if buf_start > 0 {
                buf.drain(..buf_start);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fftshift_dc_at_center() {
        let n = 8usize;
        let mags: Vec<f32> = (0..n).map(|i| i as f32).collect();
        let mut shifted = Vec::with_capacity(n);
        shifted.extend_from_slice(&mags[n / 2..]);
        shifted.extend_from_slice(&mags[..n / 2]);
        // shifted = [4,5,6,7,0,1,2,3]; DC (was 0) is now at index 4 = N/2
        assert_eq!(shifted[n / 2], 0.0, "DC should be at index N/2 after shift");
        assert_eq!(shifted[0], 4.0);
    }

    #[test]
    fn magnitude_floor_for_zero_input() {
        let z = Complex { re: 0.0f32, im: 0.0f32 };
        let norm = z.norm() / 2048.0f32;
        let db = if norm > 0.0 { 20.0 * norm.log10() } else { DB_FLOOR };
        assert_eq!(db, DB_FLOOR);
    }

    #[test]
    fn iq_byte_i8_max_converts_correctly() {
        let byte: u8 = 0x7F;
        let f = byte as i8 as f32 / 128.0;
        assert!((f - 0.9921875).abs() < 1e-6, "got {}", f);
    }

    #[test]
    fn iq_byte_i8_min_converts_correctly() {
        let byte: u8 = 0x80;
        let f = byte as i8 as f32 / 128.0;
        assert!((f - (-1.0)).abs() < 1e-6, "got {}", f);
    }

    #[test]
    fn iq_byte_uint8_converts_correctly() {
        // RTL-SDR unsigned-8-bit decode around the 127.5 DC bias maps the byte
        // range symmetrically onto [-1, 1]: 0x00 → -1.0, 0x80 → ~0, 0xFF → +1.0.
        let lo = (0x00u8 as f32 - 127.5) / 127.5;
        let mid = (0x80u8 as f32 - 127.5) / 127.5;
        let hi = (0xFFu8 as f32 - 127.5) / 127.5;
        assert!((lo - (-1.0)).abs() < 1e-6, "lo = {}", lo);
        assert!(mid.abs() < 0.01, "mid = {}", mid);
        assert!((hi - 1.0).abs() < 1e-6, "hi = {}", hi);
    }

    #[test]
    fn ema_converges_to_new_value() {
        let mut s = 0.0f32;
        let target = 1.0f32;
        let alpha = 0.5f32;
        for _ in 0..20 {
            s = alpha * target + (1.0 - alpha) * s;
        }
        assert!(s > 0.99, "EMA should converge to target, got {}", s);
    }

    #[test]
    fn snr_is_peak_minus_noise() {
        let peak_dbfs: f32 = -30.0;
        let noise_floor: f32 = -90.0;
        let snr = (peak_dbfs - noise_floor).max(0.0);
        assert!((snr - 60.0).abs() < 0.001);
    }

    #[test]
    fn channel_power_two_equal_bins() {
        let bins = [-60.0f32, -60.0];
        let total: f32 = bins.iter().map(|&b| 10f32.powf(b / 10.0)).sum();
        let power = 10.0 * total.log10();
        // Two -60 dBFS bins → -60 + 10*log10(2) ≈ -56.99 dBFS
        assert!((power - (-56.99)).abs() < 0.02);
    }

    #[test]
    fn channel_power_zero_signal_is_neg_inf() {
        let total_linear: f32 = 0.0;
        let power = if total_linear > 0.0 { 10.0 * total_linear.log10() } else { f32::NEG_INFINITY };
        assert!(power.is_infinite() && power < 0.0);
    }
}
