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

        let mut buf: Vec<u8> = Vec::new();
        let mut smoothed: Vec<f32> = Vec::new();
        let mut peak: Vec<f32> = Vec::new();

        while let Ok(chunk) = self.sample_rx.recv() {
            buf.extend_from_slice(&chunk);

            while buf.len() >= self.fft_size * 2 {
                // Convert to windowed complex samples
                let mut samples: Vec<Complex<f32>> = buf[..self.fft_size * 2]
                    .chunks_exact(2)
                    .zip(window.iter())
                    .map(|(pair, &w)| Complex {
                        re: pair[0] as i8 as f32 / 128.0 * w,
                        im: pair[1] as i8 as f32 / 128.0 * w,
                    })
                    .collect();
                buf.drain(..self.fft_size * 2);

                fft.process(&mut samples);

                // Magnitude → dBFS; normalize by fft_size for size-independent scale
                let mags: Vec<f32> = samples
                    .iter()
                    .map(|z| {
                        let norm = z.norm() / self.fft_size as f32;
                        if norm > 0.0 { 20.0 * norm.log10() } else { DB_FLOOR }
                    })
                    .collect();

                // fftshift: rotate by N/2 so DC lands at center
                let n = mags.len();
                let mut shifted = Vec::with_capacity(n);
                shifted.extend_from_slice(&mags[n / 2..]);
                shifted.extend_from_slice(&mags[..n / 2]);

                // EMA smoothing
                if smoothed.is_empty() {
                    smoothed = shifted.clone();
                } else {
                    let alpha = self.ema_alpha;
                    for (s, &new) in smoothed.iter_mut().zip(shifted.iter()) {
                        *s = alpha * new + (1.0 - alpha) * *s;
                    }
                }

                // Peak hold with per-frame decay
                if peak.is_empty() {
                    peak = smoothed.clone();
                } else {
                    let decay = self.peak_decay_db;
                    for (p, &s) in peak.iter_mut().zip(smoothed.iter()) {
                        *p = (*p - decay).max(s);
                    }
                }

                // Noise floor: mean of bottom 10% of bins
                let mut sorted = smoothed.clone();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let count = (sorted.len() / 10).max(1);
                let noise_floor = sorted[..count].iter().sum::<f32>() / count as f32;

                // Read freq/rate while unlocked from the FFT loop
                let (center_freq_hz, sample_rate) = self
                    .state
                    .lock()
                    .map(|m| (m.frequency, m.config_sample_rate))
                    .unwrap_or((0, 0.0));

                // SNR: current peak minus noise floor
                let peak_dbfs = smoothed.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let snr_db = (peak_dbfs - noise_floor).max(0.0);

                // Channel power: integrate all bins in linear domain → dBFS
                let total_linear: f32 = smoothed.iter()
                    .map(|&b| 10f32.powf(b / 10.0))
                    .sum();
                let channel_power_dbfs = if total_linear > 0.0 {
                    10.0 * total_linear.log10()
                } else {
                    f32::NEG_INFINITY
                };

                // 99% occupied BW: span of bins containing 99% of total power
                let occupied_bw_hz = if total_linear > 0.0 && sample_rate > 0.0 {
                    let threshold = total_linear * 0.99;
                    let mut indexed: Vec<(f32, usize)> = smoothed.iter()
                        .enumerate()
                        .map(|(i, &b)| (10f32.powf(b / 10.0), i))
                        .collect();
                    indexed.sort_unstable_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
                    let mut acc = 0f32;
                    let mut min_bin = smoothed.len();
                    let mut max_bin = 0usize;
                    for (power, idx) in &indexed {
                        acc += power;
                        min_bin = min_bin.min(*idx);
                        max_bin = max_bin.max(*idx);
                        if acc >= threshold { break; }
                    }
                    let bin_hz = sample_rate / smoothed.len() as f64;
                    ((max_bin.saturating_sub(min_bin) + 1) as f64 * bin_hz) as u64
                } else {
                    0u64
                };

                if let Ok(mut m) = self.state.lock() {
                    m.snr_db             = snr_db;
                    m.channel_power_dbfs = channel_power_dbfs;
                    m.occupied_bw_hz     = occupied_bw_hz;
                    m.last_fft_frame = Some(FftFrame {
                        bins_dbfs: smoothed.clone(),
                        peak_hold: peak.clone(),
                        noise_floor,
                        center_freq_hz,
                        sample_rate,
                        timestamp: std::time::Instant::now(),
                        snr_db,
                        channel_power_dbfs,
                        occupied_bw_hz,
                    });
                    m.waterfall.push(smoothed.clone());
                }
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
