use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpectrumMarker {
    pub freq_hz: u64,
    pub label:   String,
}

#[derive(Clone)]
pub struct WaterfallBuffer {
    pub rows: VecDeque<Vec<f32>>,
    pub max_rows: usize,
    pub paused: bool,
}

impl WaterfallBuffer {
    pub fn new(max_rows: usize) -> Self {
        Self { rows: VecDeque::new(), max_rows, paused: false }
    }

    pub fn push(&mut self, bins: Vec<f32>) {
        if self.paused || self.max_rows == 0 { return; }
        if self.rows.len() >= self.max_rows {
            self.rows.pop_back();
        }
        self.rows.push_front(bins);
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct FftFrame {
    /// fftshifted, EMA-smoothed magnitude spectrum in dBFS
    pub bins_dbfs: Vec<f32>,
    /// decaying peak hold, same length as bins_dbfs
    pub peak_hold: Vec<f32>,
    /// mean dBFS of the bottom 10% of bins (noise estimate)
    pub noise_floor: f32,
    pub center_freq_hz: u64,
    pub sample_rate: f64,
    pub timestamp: std::time::Instant,
    pub snr_db: f32,
    pub channel_power_dbfs: f32,
    pub occupied_bw_hz: u64,
}

pub const THROUGHPUT_HISTORY_LEN: usize = 64;

#[derive(Clone, PartialEq)]
pub enum InputMode {
    Normal,
    FrequencyInput,
    SampleRateInput,
    MarkerNameInput,
}
pub const LOG_MAX_ENTRIES: usize = 100;

pub const DEFAULT_LNA_GAIN: u32 = 16;
pub const DEFAULT_VGA_GAIN: u32 = 20;
pub const DEFAULT_FREQUENCY: u64 = 2_400_000_000;
pub const DEFAULT_SAMPLE_RATE: f64 = 10_000_000.0;

#[derive(Clone)]
pub struct SdrMetrics {
    pub frequency: u64,
    pub config_sample_rate: f64,
    pub actual_sample_rate: u32,
    pub lna_gain: u32,
    pub vga_gain: u32,
    pub amp_enabled: bool,
    // User-desired RX state (toggled by Space); separate from hw_streaming
    pub rx_enabled: bool,
    // Actual hardware streaming state, updated by the polling task
    pub hw_streaming: bool,
    pub bytes_since_last_poll: u64,
    pub last_poll_time: std::time::Instant,
    pub current_throughput_bps: u64,
    // Throughput history in KB/s for sparkline display
    pub throughput_history: VecDeque<u64>,
    // actual_sample_rate (samples/sec) per poll tick
    pub sample_rate_history: VecDeque<u64>,
    // In-app log messages (replaces eprintln! while TUI is active)
    pub log: VecDeque<String>,
    pub input_mode: InputMode,
    pub input_buf: String,

    // --- Derived metrics (written by polling task, read by UI) ---
    pub drops_per_sec: u64,
    pub total_drops_session: u64,
    pub drop_history: VecDeque<u64>,

    pub adc_saturation_pct: f32,
    pub adc_saturation_peak: f32,
    pub saturation_history: VecDeque<f32>,

    pub iq_imbalance_db: f32,
    pub dc_offset_i: f32,
    pub dc_offset_q: f32,

    pub callback_jitter_us: u64,

    pub process_cpu_pct: f32,
    pub process_rss_mb: u64,
    pub last_fft_frame: Option<FftFrame>,
    pub waterfall: WaterfallBuffer,

    // --- Hardware identity (read once at startup) ---
    pub board_name: String,
    #[allow(dead_code)]
    pub serial: String,
    pub fw_version: String,
    pub board_rev: u8,
    pub usb_api_version: u16,
    pub cpld_ok: Option<bool>,

    // --- Signal quality (written by FftWorker per frame) ---
    pub snr_db: f32,
    pub channel_power_dbfs: f32,
    pub occupied_bw_hz: u64,

    // --- IQ amplitude histogram (snapshot from accumulator, read by UI) ---
    pub iq_amplitude_hist: [u64; 32],

    // USB transfer errors (valid_length == 0 from libhackrf) — session total
    pub usb_errors_session: u64,

    // --- Observer mode (device physically present but in use by another process) ---
    pub observer_mode: bool,
    pub observer_device: Option<String>,    // "HackRF One · Great Scott Gadgets"
    pub observer_serial: Option<String>,
    pub observer_usb: Option<String>,       // "High Speed (480 Mbit/s) · 500 mA · Bus 1, Port 3"
    pub observer_connected: Option<String>, // "1h 23m 14s"
    pub observer_owner: Option<String>,     // "sdrpp (PID 12345)"
    pub observer_cmdline: Option<String>,
    pub observer_owner_cpu_pct: f32,
    pub observer_owner_ram_mb: u64,
    pub observer_owner_uptime: Option<String>,

    // --- Panel focus (set by app event loop, read by footer) ---
    pub focused_panel: Option<String>,
    pub focused_panel_bindings: &'static [(&'static str, &'static str)],

    // --- Spectrum focus controls ---
    pub spectrum_step_hz:      u64,
    pub pending_marker_freq:   Option<u64>,
    pub spectrum_y_min:        f32,
    pub spectrum_y_max:        f32,
    pub spectrum_hold:         Option<Vec<f32>>,
    pub spectrum_cursor_freq:  Option<u64>,
    pub spectrum_markers:      Vec<SpectrumMarker>,

    // --- Accumulators (written by rx_callback, reset by polling task) ---
    pub acc_drops: u64,
    pub acc_saturated: u64,
    pub acc_i_sum: i64,
    pub acc_q_sum: i64,
    pub acc_i_sq_sum: u64,
    pub acc_q_sq_sum: u64,
    pub acc_sample_count: u64,
    pub acc_jitter_sum_us: u64,
    pub acc_jitter_count: u64,
    pub acc_last_callback_us: Option<std::time::Instant>,
    pub acc_iq_hist: [u64; 32],
}

impl SdrMetrics {
    pub fn push_log(&mut self, msg: impl Into<String>) {
        if self.log.len() >= LOG_MAX_ENTRIES {
            self.log.pop_front();
        }
        self.log.push_back(msg.into());
    }

    pub fn reset_to_defaults(&mut self) {
        self.lna_gain = DEFAULT_LNA_GAIN;
        self.vga_gain = DEFAULT_VGA_GAIN;
        self.amp_enabled = false;
        self.frequency = DEFAULT_FREQUENCY;
        self.config_sample_rate = DEFAULT_SAMPLE_RATE;
        self.push_log("Settings reset to defaults");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_adds_newest_row_first() {
        let mut buf = WaterfallBuffer::new(4);
        buf.push(vec![1.0, 2.0]);
        buf.push(vec![3.0, 4.0]);
        assert_eq!(buf.rows[0], vec![3.0, 4.0], "newest row should be at index 0");
        assert_eq!(buf.rows[1], vec![1.0, 2.0]);
    }

    #[test]
    fn push_respects_max_rows() {
        let mut buf = WaterfallBuffer::new(3);
        for i in 0..5u32 {
            buf.push(vec![i as f32]);
        }
        assert_eq!(buf.rows.len(), 3, "should not exceed max_rows");
    }

    #[test]
    fn paused_ignores_push() {
        let mut buf = WaterfallBuffer::new(4);
        buf.paused = true;
        buf.push(vec![1.0, 2.0]);
        assert!(buf.rows.is_empty(), "paused buffer should not accept new rows");
    }
}
