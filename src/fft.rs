use std::sync::{Arc, Mutex};

use crossbeam_channel::Receiver;
use num_complex::Complex;
use rustfft::FftPlanner;

use crate::dsp::{self, WindowFn};
use crate::state::{FftFrame, SdrMetrics};

const DB_FLOOR: f32 = -160.0;

pub struct FftWorker {
    pub sample_rx: Receiver<Vec<u8>>,
    pub state: Arc<Mutex<SdrMetrics>>,
    pub fft_size: usize,
    pub window_fn: WindowFn,
    pub ema_alpha: f32,
    pub peak_decay_db: f32,
}

impl FftWorker {
    pub fn new(sample_rx: Receiver<Vec<u8>>, state: Arc<Mutex<SdrMetrics>>) -> Self {
        Self {
            sample_rx,
            state,
            fft_size: 2048,
            window_fn: WindowFn::Hann,
            ema_alpha: 0.2,
            peak_decay_db: 0.5,
        }
    }

    pub fn run(self) {
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(self.fft_size);
        let window = dsp::compute_window(self.window_fn, self.fft_size);
        let n = self.fft_size;

        // Pre-allocate all scratch buffers — reused every frame, zero heap churn
        let mut buf: Vec<u8> = Vec::new();
        let mut samples: Vec<Complex<f32>> = vec![Complex::default(); n];
        let mut mags:    Vec<f32>          = vec![0.0; n];
        let mut shifted: Vec<f32>          = vec![0.0; n];
        let mut smoothed: Vec<f32>         = vec![DB_FLOOR; n];
        let mut peak: Vec<f32>             = vec![DB_FLOOR; n];
        // scratch for noise floor partial sort (avoids O(n log n) full sort)
        let mut noise_scratch: Vec<f32>    = vec![0.0; n];
        // scratch for occupied-BW sort (avoids alloc per frame)
        let mut occ_scratch: Vec<(f32, usize)> = vec![(0.0, 0); n];
        let mut initialized = false;

        // Throttle state writes to ~30 fps — EMA runs on every frame for accuracy,
        // but the expensive analysis + state lock fires at display rate only.
        const UPDATE_INTERVAL: std::time::Duration = std::time::Duration::from_millis(33);
        let mut last_state_update = std::time::Instant::now()
            .checked_sub(UPDATE_INTERVAL)
            .unwrap_or_else(std::time::Instant::now);

        while let Ok(chunk) = self.sample_rx.recv() {
            buf.extend_from_slice(&chunk);

            let frame_bytes = n * 2;
            let mut buf_start = 0usize;

            while buf.len() - buf_start >= frame_bytes {
                let frame = &buf[buf_start..buf_start + frame_bytes];

                // Convert to windowed complex samples in-place
                for (i, (pair, &w)) in frame.chunks_exact(2).zip(window.iter()).enumerate() {
                    samples[i] = Complex {
                        re: pair[0] as i8 as f32 / 128.0 * w,
                        im: pair[1] as i8 as f32 / 128.0 * w,
                    };
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
                    let alpha = self.ema_alpha;
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
                    .map(|m| (m.frequency, m.config_sample_rate))
                    .unwrap_or((0, 0.0));

                // SNR: peak minus noise floor
                let peak_dbfs = smoothed.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let snr_db = (peak_dbfs - noise_floor).max(0.0);

                // Channel power: integrate all bins → dBFS
                let total_linear: f32 = smoothed.iter().map(|&b| 10f32.powf(b / 10.0)).sum();
                let channel_power_dbfs = if total_linear > 0.0 {
                    10.0 * total_linear.log10()
                } else {
                    f32::NEG_INFINITY
                };

                // 99% occupied BW using pre-allocated scratch
                let occupied_bw_hz = if total_linear > 0.0 && sample_rate > 0.0 {
                    let threshold = total_linear * 0.99;
                    for (i, &b) in smoothed.iter().enumerate() {
                        occ_scratch[i] = (10f32.powf(b / 10.0), i);
                    }
                    occ_scratch.sort_unstable_by(|a, b| {
                        b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal)
                    });
                    let mut acc = 0f32;
                    let mut min_bin = n;
                    let mut max_bin = 0usize;
                    for &(power, idx) in &occ_scratch {
                        acc += power;
                        min_bin = min_bin.min(idx);
                        max_bin = max_bin.max(idx);
                        if acc >= threshold { break; }
                    }
                    let bin_hz = sample_rate / n as f64;
                    ((max_bin.saturating_sub(min_bin) + 1) as f64 * bin_hz) as u64
                } else {
                    0u64
                };

                // Wrap in Arc once — shared cheaply between FftFrame and waterfall
                let bins_arc = Arc::new(smoothed.clone());
                let peak_arc = Arc::new(peak.clone());

                if let Ok(mut m) = self.state.lock() {
                    m.snr_db             = snr_db;
                    m.channel_power_dbfs = channel_power_dbfs;
                    m.occupied_bw_hz     = occupied_bw_hz;
                    m.last_fft_frame = Some(FftFrame {
                        bins_dbfs: Arc::clone(&bins_arc),
                        peak_hold: peak_arc,
                        noise_floor,
                        center_freq_hz,
                        sample_rate,
                        timestamp: std::time::Instant::now(),
                        snr_db,
                        channel_power_dbfs,
                        occupied_bw_hz,
                    });
                    m.waterfall.push(bins_arc);
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
