use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossterm::event::KeyCode;
use ratatui::{backend::Backend, Terminal};

use std::path::PathBuf;

use crate::config::{AppConfig, DisplayConfig, LayoutConfig, RadioConfig};
use crate::event::{AppEvent, EventStream};
use crate::fft::FftWorker;
use crate::hardware::{self, RxContext};
use crate::state::{
    InputMode, SdrMetrics, DEFAULT_FREQUENCY, DEFAULT_LNA_GAIN, DEFAULT_SAMPLE_RATE,
    DEFAULT_VGA_GAIN, THROUGHPUT_HISTORY_LEN,
};
use crate::ui;

pub struct App {
    state: Arc<Mutex<SdrMetrics>>,
    #[allow(dead_code)]
    device: Arc<hardware::Device>,
    #[allow(dead_code)]
    rx_ctx: Arc<RxContext>,
    config_path: Option<PathBuf>,
    events: EventStream,
    show_help: bool,
    engine: ui::LayoutEngine,
}

impl App {
    pub fn new(cfg: AppConfig, config_path: Option<PathBuf>) -> anyhow::Result<Self> {
        let device = Arc::new(hardware::Device::open()?);

        let board_id = device.board_id()?;
        let board_name = device.board_name(board_id);
        let fw_version = device.version()?;
        let serial = device.serial_number()?;
        let board_rev       = device.board_rev().unwrap_or(0xFE);
        let usb_api_version = device.usb_api_version().unwrap_or(0);
        let cpld_ok: Option<bool> = None; // hackrf_cpld_checksum not in this libhackrf version

        let startup_results = [
            device.set_frequency(cfg.radio.frequency_hz),
            device.set_sample_rate(cfg.radio.sample_rate),
            device.set_lna_gain(cfg.radio.lna_gain),
            device.set_vga_gain(cfg.radio.vga_gain),
            device.set_amp_enable(cfg.radio.amp_enabled),
        ];

        let state = Arc::new(Mutex::new(SdrMetrics {
            frequency: cfg.radio.frequency_hz,
            config_sample_rate: cfg.radio.sample_rate,
            actual_sample_rate: 0,
            lna_gain: cfg.radio.lna_gain,
            vga_gain: cfg.radio.vga_gain,
            amp_enabled: cfg.radio.amp_enabled,
            rx_enabled: false,
            hw_streaming: false,
            bytes_since_last_poll: 0,
            last_poll_time: Instant::now(),
            current_throughput_bps: 0,
            throughput_history: VecDeque::with_capacity(THROUGHPUT_HISTORY_LEN),
            log: VecDeque::new(),
            input_mode: InputMode::Normal,
            input_buf: String::new(),

            drops_per_sec: 0,
            total_drops_session: 0,
            drop_history: VecDeque::with_capacity(THROUGHPUT_HISTORY_LEN),

            adc_saturation_pct: 0.0,
            adc_saturation_peak: 0.0,
            saturation_history: VecDeque::with_capacity(THROUGHPUT_HISTORY_LEN),

            iq_imbalance_db: 0.0,
            dc_offset_i: 0.0,
            dc_offset_q: 0.0,

            callback_jitter_us: 0,

            process_cpu_pct: 0.0,
            process_rss_mb: 0,
            last_fft_frame: None,
            waterfall: crate::state::WaterfallBuffer::new(cfg.display.waterfall_max_rows),

            board_rev,
            usb_api_version,
            cpld_ok,
            snr_db:             0.0,
            channel_power_dbfs: f32::NEG_INFINITY,
            occupied_bw_hz:     0,
            iq_amplitude_hist:  [0u64; 32],

            usb_errors_session: 0,

            acc_drops: 0,
            acc_saturated: 0,
            acc_i_sum: 0,
            acc_q_sum: 0,
            acc_i_sq_sum: 0,
            acc_q_sq_sum: 0,
            acc_sample_count: 0,
            acc_jitter_sum_us: 0,
            acc_jitter_count: 0,
            acc_last_callback_us: None,
            acc_iq_hist:          [0u64; 32],
        }));

        {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            m.push_log(format!("Connected: {} | Serial: {}", board_name, serial));
            m.push_log(format!("Firmware: {}", fw_version));
            m.push_log(format!("Board: {} | USB API: {:#06x}",
                hardware::Device::board_rev_name(board_rev), usb_api_version));
            if cpld_ok == Some(false) {
                m.push_log("WARNING: CPLD checksum mismatch!");
            }
            let names = ["frequency", "sample rate", "LNA gain", "VGA gain", "amp"];
            for (result, name) in startup_results.iter().zip(names.iter()) {
                if let Err(e) = result {
                    m.push_log(format!("Startup: failed to set {}: {}", name, e));
                }
            }
        }

        let (sample_tx, sample_rx) = crossbeam_channel::bounded::<Vec<u8>>(4);

        let rx_ctx = Arc::new(RxContext {
            metrics: Arc::clone(&state),
            sample_tx,
        });

        // FftWorker runs on a real OS thread — it's CPU-bound blocking work
        let fft_state = Arc::clone(&state);
        std::thread::spawn(move || {
            FftWorker::new(sample_rx, fft_state).run();
        });

        let rx_ctx_bg = Arc::clone(&rx_ctx);
        let state_bg = Arc::clone(&state);
        let device_bg = Arc::clone(&device);
        tokio::spawn(async move {
            let mut hw_rx_active = false;

            loop {
                let now = Instant::now();

                // Detect unexpected streaming stop (USB error, cable issue, device reset).
                // Must check before the rx_enabled/hw_rx_active gate below, otherwise
                // hw_rx_active stays true and neither branch triggers — app gets stuck.
                if hw_rx_active && !device_bg.is_streaming() {
                    let _ = device_bg.stop_rx();
                    hw_rx_active = false;
                    let mut m = state_bg.lock().unwrap_or_else(|e| e.into_inner());
                    m.rx_enabled = false;
                    m.hw_streaming = false;
                    m.push_log("WARNING: Streaming stopped unexpectedly — press [Space] to restart");
                }

                {
                    let mut m = state_bg.lock().unwrap_or_else(|e| e.into_inner());
                    let elapsed_ms = now.duration_since(m.last_poll_time).as_millis() as u64;
                    let bytes = m.bytes_since_last_poll;
                    m.bytes_since_last_poll = 0;
                    m.last_poll_time = now;

                    m.hw_streaming = device_bg.is_streaming();

                    if let Some(bps) = (bytes * 1000).checked_div(elapsed_ms) {
                        m.current_throughput_bps = bps;
                        // 2 bytes per IQ sample (8-bit I + 8-bit Q)
                        m.actual_sample_rate = (m.current_throughput_bps / 2) as u32;
                        let throughput_kb = m.current_throughput_bps / 1024;
                        if m.throughput_history.len() >= THROUGHPUT_HISTORY_LEN {
                            m.throughput_history.pop_front();
                        }
                        m.throughput_history.push_back(throughput_kb);
                    }
                    if let Some(dps) = (m.acc_drops * 1000).checked_div(elapsed_ms) {
                        m.drops_per_sec = dps;
                    }
                    let drops_snapshot = m.drops_per_sec;
                    if m.drop_history.len() >= THROUGHPUT_HISTORY_LEN { m.drop_history.pop_front(); }
                    m.drop_history.push_back(drops_snapshot);

                    // Snapshot and reset accumulators atomically
                    let acc_drops       = m.acc_drops;
                    let acc_saturated   = m.acc_saturated;
                    let acc_i_sum       = m.acc_i_sum;
                    let acc_q_sum       = m.acc_q_sum;
                    let acc_i_sq_sum    = m.acc_i_sq_sum;
                    let acc_q_sq_sum    = m.acc_q_sq_sum;
                    let acc_samples     = m.acc_sample_count;
                    let acc_jitter_sum  = m.acc_jitter_sum_us;
                    let acc_jitter_cnt  = m.acc_jitter_count;
                    m.acc_drops           = 0;
                    m.acc_saturated       = 0;
                    m.acc_i_sum           = 0;
                    m.acc_q_sum           = 0;
                    m.acc_i_sq_sum        = 0;
                    m.acc_q_sq_sum        = 0;
                    m.acc_sample_count    = 0;
                    m.acc_jitter_sum_us   = 0;
                    m.acc_jitter_count    = 0;

                    // IQ amplitude histogram snapshot
                    m.iq_amplitude_hist = m.acc_iq_hist;
                    m.acc_iq_hist = [0u64; 32];

                    // ADC saturation % — each IQ pair has 2 bytes that can saturate
                    let saturable = acc_samples * 2;
                    m.adc_saturation_pct = if saturable > 0 {
                        (acc_saturated as f32 / saturable as f32) * 100.0
                    } else {
                        0.0
                    };
                    if m.adc_saturation_pct > m.adc_saturation_peak {
                        m.adc_saturation_peak = m.adc_saturation_pct;
                    }
                    let sat_snapshot = m.adc_saturation_pct;
                    if m.saturation_history.len() >= THROUGHPUT_HISTORY_LEN { m.saturation_history.pop_front(); }
                    m.saturation_history.push_back(sat_snapshot);

                    // IQ diagnostics — float math only here, never in rx_callback
                    if acc_samples > 0 {
                        let n = acc_samples as f64;
                        m.dc_offset_i = (acc_i_sum as f64 / n / 128.0) as f32;
                        m.dc_offset_q = (acc_q_sum as f64 / n / 128.0) as f32;
                        let i_rms = (acc_i_sq_sum as f64 / n).sqrt();
                        let q_rms = (acc_q_sq_sum as f64 / n).sqrt();
                        if q_rms > 0.0 {
                            m.iq_imbalance_db = (20.0 * (i_rms / q_rms).log10()) as f32;
                        }
                    }

                    // Callback jitter: rolling mean of inter-callback intervals
                    if let Some(jitter) = acc_jitter_sum.checked_div(acc_jitter_cnt) {
                        m.callback_jitter_us = jitter;
                    }

                    // Suppress unused variable warnings — these are read by the panels
                    let _ = acc_drops;
                }

                let rx_enabled = state_bg.lock().unwrap_or_else(|e| e.into_inner()).rx_enabled;
                if rx_enabled && !hw_rx_active {
                    let user_param = Arc::as_ptr(&rx_ctx_bg) as *mut libc::c_void;
                    match device_bg.start_rx(hardware::rx_callback, user_param) {
                        Ok(()) => {
                            hw_rx_active = true;
                            state_bg.lock().unwrap_or_else(|e| e.into_inner()).push_log("RX streaming started");
                        }
                        Err(e) => {
                            let msg = format!("Error starting RX: {}", e);
                            let mut m = state_bg.lock().unwrap_or_else(|e| e.into_inner());
                            m.rx_enabled = false;
                            m.push_log(msg);
                        }
                    }
                } else if !rx_enabled && hw_rx_active {
                    let result = device_bg.stop_rx();
                    hw_rx_active = false;
                    let mut m = state_bg.lock().unwrap_or_else(|e| e.into_inner());
                    match result {
                        Ok(()) => m.push_log("RX streaming stopped"),
                        Err(e) => m.push_log(format!("Error stopping RX: {}", e)),
                    }
                }

                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        });

        // System resource polling — reads /proc every 1 s, independent of hardware task
        let sys_state = Arc::clone(&state);
        tokio::spawn(async move {
            let ticks_per_sec = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as f64;
            let mut last_ticks: u64 = 0;
            let mut last_time = Instant::now();

            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                if let Some((total_ticks, rss_mb)) = read_process_stats() {
                    let elapsed = last_time.elapsed().as_secs_f64();
                    let tick_delta = total_ticks.saturating_sub(last_ticks) as f64;
                    let cpu_pct = if elapsed > 0.0 && ticks_per_sec > 0.0 {
                        (tick_delta / ticks_per_sec / elapsed * 100.0).min(100.0) as f32
                    } else {
                        0.0
                    };
                    last_ticks = total_ticks;
                    last_time = Instant::now();
                    if let Ok(mut m) = sys_state.lock() {
                        m.process_cpu_pct = cpu_pct;
                        m.process_rss_mb  = rss_mb;
                    }
                }
            }
        });

        // Build panel registry
        let mut registry = ui::PanelRegistry::new();
        registry.register(ui::HeaderPanel {
            board_name: board_name.clone(),
            fw_version: fw_version.clone(),
            serial: serial.clone(),
        });
        registry.register(ui::TelemetryPanel {
            board_name: board_name.clone(),
            serial: serial.clone(),
        });
        registry.register(ui::GainsPanel);
        registry.register(ui::LogPanel);
        registry.register(ui::FooterPanel);
        registry.register(ui::HardwareHealthPanel);
        registry.register(ui::IqDiagnosticsPanel);
        registry.register(ui::SystemResourcesPanel);
        registry.register(ui::SpectrumPanel);
        registry.register(ui::WaterfallPanel::new());
        registry.register(ui::RfChainPanel);
        registry.register(ui::SignalMetricsPanel);
        registry.register(ui::IqHistogramPanel);

        let mut engine = ui::LayoutEngine::new(LayoutConfig::default_config(), registry);
        engine.set_preset(&cfg.display.active_preset);

        Ok(Self {
            state,
            device,
            rx_ctx,
            config_path,
            events: EventStream::new(Duration::from_millis(100)),
            show_help: false,
            engine,
        })
    }

    fn save_config(&self) {
        let Some(path) = &self.config_path else { return };
        let (freq, rate, lna, vga, amp, wf_rows) = {
            let m = self.state.lock().unwrap_or_else(|e| e.into_inner());
            (m.frequency, m.config_sample_rate, m.lna_gain,
             m.vga_gain, m.amp_enabled, m.waterfall.max_rows)
        };
        let cfg = AppConfig {
            radio: RadioConfig {
                frequency_hz: freq,
                sample_rate:  rate,
                lna_gain:     lna,
                vga_gain:     vga,
                amp_enabled:  amp,
            },
            display: DisplayConfig {
                active_preset:      self.engine.active_preset().to_string(),
                waterfall_max_rows: wf_rows,
            },
        };
        let _ = cfg.save(path);
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        loop {
            let m = self.state.lock().unwrap_or_else(|e| e.into_inner()).clone();
            terminal.draw(|f| {
                self.engine.draw(f, &m);
                if self.show_help {
                    ui::overlay::render_help(f);
                }
            })?;

            match self.events.recv() {
                AppEvent::Key(key) => {
                    let input_mode = self.state.lock().unwrap_or_else(|e| e.into_inner()).input_mode.clone();
                    match input_mode {
                        InputMode::Normal => match key.code {
                            KeyCode::Char('q') => {
                                self.save_config();
                                return Ok(());
                            }
                            KeyCode::Char(' ') => {
                                let mut m = self.state.lock().unwrap_or_else(|e| e.into_inner());
                                m.rx_enabled = !m.rx_enabled;
                            }
                            KeyCode::Char('r') => {
                                let results = [
                                    self.device.set_lna_gain(DEFAULT_LNA_GAIN),
                                    self.device.set_vga_gain(DEFAULT_VGA_GAIN),
                                    self.device.set_frequency(DEFAULT_FREQUENCY),
                                    self.device.set_sample_rate(DEFAULT_SAMPLE_RATE),
                                    self.device.set_amp_enable(false),
                                ];
                                let mut m = self.state.lock().unwrap_or_else(|e| e.into_inner());
                                m.reset_to_defaults();
                                for r in results {
                                    if let Err(e) = r {
                                        m.push_log(format!("Reset error: {}", e));
                                    }
                                }
                            }
                            KeyCode::Char('f') => {
                                let mut m = self.state.lock().unwrap_or_else(|e| e.into_inner());
                                m.input_mode = InputMode::FrequencyInput;
                                m.input_buf.clear();
                                m.push_log("Enter frequency in MHz, then press Enter");
                            }
                            KeyCode::Char('?') => {
                                self.show_help = !self.show_help;
                            }
                            KeyCode::Char('p') => {
                                self.engine.cycle_preset();
                                let name = self.engine.active_preset().to_string();
                                self.state.lock().unwrap_or_else(|e| e.into_inner()).push_log(format!("Preset: {}", name));
                            }
                            KeyCode::Char('1') => {
                                self.engine.set_preset("minimal");
                                self.state.lock().unwrap_or_else(|e| e.into_inner()).push_log("Preset: minimal");
                            }
                            KeyCode::Char('2') => {
                                self.engine.set_preset("monitoring");
                                self.state.lock().unwrap_or_else(|e| e.into_inner()).push_log("Preset: monitoring");
                            }
                            KeyCode::Char('3') => {
                                self.engine.set_preset("spectrum");
                                self.state.lock().unwrap_or_else(|e| e.into_inner()).push_log("Preset: spectrum");
                            }
                            KeyCode::Char('4') => {
                                self.engine.set_preset("waterfall");
                                self.state.lock().unwrap_or_else(|e| e.into_inner()).push_log("Preset: waterfall");
                            }
                            KeyCode::Char('5') => {
                                self.engine.set_preset("spectrum_waterfall");
                                self.state.lock().unwrap_or_else(|e| e.into_inner()).push_log("Preset: spectrum+waterfall");
                            }
                            KeyCode::Char('6') => {
                                self.engine.set_preset("lab");
                                self.state.lock().unwrap_or_else(|e| e.into_inner()).push_log("Preset: lab");
                            }
                            KeyCode::Char('w') => {
                                let mut m = self.state.lock().unwrap_or_else(|e| e.into_inner());
                                m.waterfall.paused = !m.waterfall.paused;
                                let s = if m.waterfall.paused { "paused" } else { "resumed" };
                                m.push_log(format!("Waterfall {}", s));
                            }
                            KeyCode::Up => {
                                let (gain, result) = {
                                    let m = self.state.lock().unwrap_or_else(|e| e.into_inner());
                                    let new_gain = (m.lna_gain + 8).min(40);
                                    let result = self.device.set_lna_gain(new_gain);
                                    (new_gain, result)
                                };
                                let mut m = self.state.lock().unwrap_or_else(|e| e.into_inner());
                                match result {
                                    Ok(()) => {
                                        m.lna_gain = gain;
                                        m.push_log(format!("LNA gain → {} dB", gain));
                                    }
                                    Err(e) => m.push_log(format!("LNA gain error: {}", e)),
                                }
                            }
                            KeyCode::Down => {
                                let (gain, result) = {
                                    let m = self.state.lock().unwrap_or_else(|e| e.into_inner());
                                    let new_gain = m.lna_gain.saturating_sub(8);
                                    let result = self.device.set_lna_gain(new_gain);
                                    (new_gain, result)
                                };
                                let mut m = self.state.lock().unwrap_or_else(|e| e.into_inner());
                                match result {
                                    Ok(()) => {
                                        m.lna_gain = gain;
                                        m.push_log(format!("LNA gain → {} dB", gain));
                                    }
                                    Err(e) => m.push_log(format!("LNA gain error: {}", e)),
                                }
                            }
                            KeyCode::Char('[') => {
                                let (gain, result) = {
                                    let m = self.state.lock().unwrap_or_else(|e| e.into_inner());
                                    let new_gain = m.vga_gain.saturating_sub(2);
                                    let result = self.device.set_vga_gain(new_gain);
                                    (new_gain, result)
                                };
                                let mut m = self.state.lock().unwrap_or_else(|e| e.into_inner());
                                match result {
                                    Ok(()) => {
                                        m.vga_gain = gain;
                                        m.push_log(format!("VGA gain → {} dB", gain));
                                    }
                                    Err(e) => m.push_log(format!("VGA gain error: {}", e)),
                                }
                            }
                            KeyCode::Char(']') => {
                                let (gain, result) = {
                                    let m = self.state.lock().unwrap_or_else(|e| e.into_inner());
                                    let new_gain = (m.vga_gain + 2).min(62);
                                    let result = self.device.set_vga_gain(new_gain);
                                    (new_gain, result)
                                };
                                let mut m = self.state.lock().unwrap_or_else(|e| e.into_inner());
                                match result {
                                    Ok(()) => {
                                        m.vga_gain = gain;
                                        m.push_log(format!("VGA gain → {} dB", gain));
                                    }
                                    Err(e) => m.push_log(format!("VGA gain error: {}", e)),
                                }
                            }
                            KeyCode::Char('a') => {
                                let (enabled, result) = {
                                    let m = self.state.lock().unwrap_or_else(|e| e.into_inner());
                                    let new_state = !m.amp_enabled;
                                    let result = self.device.set_amp_enable(new_state);
                                    (new_state, result)
                                };
                                let mut m = self.state.lock().unwrap_or_else(|e| e.into_inner());
                                match result {
                                    Ok(()) => {
                                        m.amp_enabled = enabled;
                                        m.push_log(format!(
                                            "AMP {}",
                                            if enabled { "ON" } else { "OFF" }
                                        ));
                                    }
                                    Err(e) => m.push_log(format!("AMP error: {}", e)),
                                }
                            }
                            _ => {}
                        },
                        InputMode::FrequencyInput => match key.code {
                            KeyCode::Esc => {
                                let mut m = self.state.lock().unwrap_or_else(|e| e.into_inner());
                                m.input_mode = InputMode::Normal;
                                m.input_buf.clear();
                                m.push_log("Frequency input cancelled");
                            }
                            KeyCode::Backspace => {
                                self.state.lock().unwrap_or_else(|e| e.into_inner()).input_buf.pop();
                            }
                            KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => {
                                self.state.lock().unwrap_or_else(|e| e.into_inner()).input_buf.push(c);
                            }
                            KeyCode::Enter => {
                                let (freq_hz, result) = {
                                    let m = self.state.lock().unwrap_or_else(|e| e.into_inner());
                                    match m.input_buf.parse::<f64>() {
                                        Ok(mhz) if mhz > 0.0 => {
                                            let hz = (mhz * 1_000_000.0) as u64;
                                            let result = self.device.set_frequency(hz);
                                            (Some(hz), Some(result))
                                        }
                                        _ => (None, None),
                                    }
                                };
                                let mut m = self.state.lock().unwrap_or_else(|e| e.into_inner());
                                match (freq_hz, result) {
                                    (Some(hz), Some(Ok(()))) => {
                                        m.frequency = hz;
                                        m.input_mode = InputMode::Normal;
                                        m.input_buf.clear();
                                        m.push_log(format!(
                                            "Frequency set to {:.3} MHz",
                                            hz as f64 / 1_000_000.0
                                        ));
                                    }
                                    (Some(_), Some(Err(e))) => {
                                        m.push_log(format!("Frequency error: {}", e));
                                    }
                                    _ => {
                                        let bad = m.input_buf.clone();
                                        m.push_log(format!("Invalid frequency: '{}'", bad));
                                    }
                                }
                            }
                            _ => {}
                        },
                    }
                }
                AppEvent::Tick => {}
            }
        }
    }
}

fn read_process_stats() -> Option<(u64, u64)> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let after_comm = stat.rsplit_once(')')?.1;
    let fields: Vec<&str> = after_comm.split_whitespace().collect();
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let rss_kb: u64 = status
        .lines()
        .find(|l| l.starts_with("VmRSS:"))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()?;
    Some((utime + stime, rss_kb / 1024))
}

#[cfg(test)]
mod tests {
    #[test]
    fn proc_stat_field_indices() {
        // Verify that field offsets after ')' are correct: utime=11, stime=12
        // Simulate a realistic /proc/self/stat line
        let fake = "1234 (my process) S 1 1 1 0 -1 4194304 0 0 0 0 42 7 0 0 20 0 1 0 0 0 0";
        let after_comm = fake.rsplit_once(')').unwrap().1;
        let fields: Vec<&str> = after_comm.split_whitespace().collect();
        assert_eq!(fields.get(11), Some(&"42"), "utime at index 11");
        assert_eq!(fields.get(12), Some(&"7"),  "stime at index 12");
    }

    #[test]
    fn iq_imbalance_zero_for_balanced() {
        let n = 1000_f64;
        let i_rms = (500_000_f64 / n).sqrt();
        let q_rms = (500_000_f64 / n).sqrt();
        let imbalance = (20.0 * (i_rms / q_rms).log10()) as f32;
        assert!(imbalance.abs() < 0.001, "expected ~0, got {}", imbalance);
    }

    #[test]
    fn iq_imbalance_positive_when_i_stronger() {
        let n = 1000_f64;
        let i_rms = (800_000_f64 / n).sqrt();
        let q_rms = (200_000_f64 / n).sqrt();
        let imbalance = (20.0 * (i_rms / q_rms).log10()) as f32;
        assert!(imbalance > 0.0, "expected positive, got {}", imbalance);
    }

    #[test]
    fn adc_saturation_pct_full() {
        let acc_saturated = 200_u64;
        let acc_samples   = 100_u64; // IQ pairs
        let saturable     = acc_samples * 2;
        let pct = (acc_saturated as f32 / saturable as f32) * 100.0;
        assert!((pct - 100.0).abs() < 0.01, "expected 100%, got {}", pct);
    }
}
