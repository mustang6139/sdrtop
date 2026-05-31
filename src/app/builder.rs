use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::config::{AppConfig, LayoutConfig};
use crate::event::EventStream;
use crate::hardware;
use crate::signal::FftWorker;
use crate::state::{
    Accumulators, IqState, ObserverState, RadioState, SdrMetrics,
    SignalState, SpectrumState, SystemState, UiState, WaterfallState,
    THROUGHPUT_HISTORY_LEN,
};
use crate::tasks;
use crate::ui;

use super::App;

impl App {
    fn build_ui(
        board_name: &str,
        serial: &str,
        active_preset: &str,
    ) -> (ui::LayoutEngine, HashMap<char, &'static str>) {
        let mut registry = ui::PanelRegistry::new();
        registry.register(ui::HeaderPanel);
        registry.register(ui::TelemetryPanel {
            board_name: board_name.to_string(),
            serial: serial.to_string(),
        });
        registry.register(ui::GainsPanel);
        registry.register(ui::ThroughputPanel);
        registry.register(ui::SampleRatePanel);
        registry.register(ui::SignalStripPanel);
        registry.register(ui::UsbSrPanel);
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
        registry.register(ui::ObserverPanel);

        let mut focus_keys: HashMap<char, &'static str> = HashMap::new();
        for panel in registry.panels_iter() {
            if let Some(key) = panel.focus_key() {
                focus_keys.insert(key, panel.name());
            }
        }

        let mut engine = ui::LayoutEngine::new(LayoutConfig::default_config(), registry);
        engine.set_preset(active_preset);
        (engine, focus_keys)
    }

    pub(super) fn new_normal(
        cfg: AppConfig,
        config_path: Option<PathBuf>,
        raw_device: hardware::Device,
    ) -> anyhow::Result<Self> {
        let device = Arc::new(raw_device);

        let board_id     = device.board_id()?;
        let board_name   = device.board_name(board_id);
        let fw_version   = device.version()?;
        let serial       = device.serial_number()?;
        let board_rev    = device.board_rev().unwrap_or(0xFE);
        let usb_api_ver  = device.usb_api_version().unwrap_or(0);
        let cpld_ok: Option<bool> = None;
        let theme        = cfg.build_theme();

        let startup_results = [
            device.set_frequency(cfg.radio.frequency_hz),
            device.set_sample_rate(cfg.radio.sample_rate),
            device.set_lna_gain(cfg.radio.lna_gain),
            device.set_vga_gain(cfg.radio.vga_gain),
            device.set_amp_enable(cfg.radio.amp_enabled),
        ];

        let state = Arc::new(Mutex::new(SdrMetrics {
            radio: RadioState {
                frequency:           cfg.radio.frequency_hz,
                config_sample_rate:  cfg.radio.sample_rate,
                actual_sample_rate:  0,
                lna_gain:            cfg.radio.lna_gain,
                vga_gain:            cfg.radio.vga_gain,
                amp_enabled:         cfg.radio.amp_enabled,
                rx_enabled:          false,
                hw_streaming:        false,
                bytes_since_last_poll: 0,
                last_poll_time:      Instant::now(),
                current_throughput_bps: 0,
                throughput_history:  std::collections::VecDeque::with_capacity(THROUGHPUT_HISTORY_LEN),
                sample_rate_history: std::collections::VecDeque::with_capacity(THROUGHPUT_HISTORY_LEN),
            },
            signal: SignalState {
                drops_per_sec: 0, total_drops_session: 0,
                drop_history: std::collections::VecDeque::with_capacity(THROUGHPUT_HISTORY_LEN),
                adc_saturation_pct: 0.0, adc_saturation_peak: 0.0,
                saturation_history: std::collections::VecDeque::with_capacity(THROUGHPUT_HISTORY_LEN),
                snr_db: 0.0, channel_power_dbfs: f32::NEG_INFINITY,
                occupied_bw_hz: 0, usb_errors_session: 0,
            },
            iq: IqState { iq_imbalance_db: 0.0, dc_offset_i: 0.0, dc_offset_q: 0.0, callback_jitter_us: 0, iq_amplitude_hist: [0u64; 32] },
            observer: ObserverState::default(),
            spectrum: SpectrumState {
                step_hz: 100_000, y_min: -120.0, y_max: 0.0,
                hold: None, cursor_freq: None,
                markers: cfg.display.spectrum_markers.clone(), pending_marker: None,
            },
            waterfall: WaterfallState::new(cfg.display.waterfall_max_rows),
            system: SystemState {
                board_name: board_name.clone(), serial: serial.clone(),
                fw_version: fw_version.clone(), board_rev,
                usb_api_version: usb_api_ver, cpld_ok,
                process_cpu_pct: 0.0, process_rss_mb: 0,
            },
            ui:  UiState::default(),
            acc: Accumulators::default(),
        }));

        {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            m.push_log(format!("Connected: {} | Serial: {}", board_name, serial));
            m.push_log(format!("Firmware: {}", fw_version));
            m.push_log(format!("Board: {} | USB API: {:#06x}",
                hardware::Device::board_rev_name(board_rev), usb_api_ver));
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
        let rx_ctx = Arc::new(hardware::RxContext { metrics: Arc::clone(&state), sample_tx });

        let fft_state = Arc::clone(&state);
        std::thread::spawn(move || FftWorker::new(sample_rx, fft_state).run());

        tasks::spawn_rx_task(Arc::clone(&state), Arc::clone(&device), Arc::clone(&rx_ctx));
        tasks::spawn_sys_resource_task(Arc::clone(&state));

        let (engine, focus_keys) = Self::build_ui(&board_name, &serial, &cfg.display.active_preset);

        Ok(Self {
            state,
            device: Some(device),
            rx_ctx: Some(rx_ctx),
            config_path,
            events: EventStream::new(Duration::from_millis(100)),
            show_help: false,
            engine,
            theme,
            focus_keys,
        })
    }

    pub(super) fn new_observer(
        cfg: AppConfig,
        config_path: Option<PathBuf>,
        sysinfo: hardware::sysfs::HackRfSysInfo,
    ) -> anyhow::Result<Self> {
        let board_name = sysinfo.product.clone();
        let serial     = sysinfo.serial.clone();
        let theme      = cfg.build_theme();

        let state = Arc::new(Mutex::new(SdrMetrics {
            radio: RadioState {
                frequency:           cfg.radio.frequency_hz,
                config_sample_rate:  cfg.radio.sample_rate,
                actual_sample_rate:  0,
                lna_gain:            cfg.radio.lna_gain,
                vga_gain:            cfg.radio.vga_gain,
                amp_enabled:         cfg.radio.amp_enabled,
                rx_enabled:          false,
                hw_streaming:        false,
                bytes_since_last_poll: 0,
                last_poll_time:      Instant::now(),
                current_throughput_bps: 0,
                throughput_history:  std::collections::VecDeque::with_capacity(THROUGHPUT_HISTORY_LEN),
                sample_rate_history: std::collections::VecDeque::with_capacity(THROUGHPUT_HISTORY_LEN),
            },
            signal: SignalState {
                drops_per_sec: 0, total_drops_session: 0,
                drop_history: std::collections::VecDeque::with_capacity(THROUGHPUT_HISTORY_LEN),
                adc_saturation_pct: 0.0, adc_saturation_peak: 0.0,
                saturation_history: std::collections::VecDeque::with_capacity(THROUGHPUT_HISTORY_LEN),
                snr_db: 0.0, channel_power_dbfs: f32::NEG_INFINITY,
                occupied_bw_hz: 0, usb_errors_session: 0,
            },
            iq: IqState { iq_imbalance_db: 0.0, dc_offset_i: 0.0, dc_offset_q: 0.0, callback_jitter_us: 0, iq_amplitude_hist: [0u64; 32] },
            observer: ObserverState {
                active: true,
                device: Some(format!("{} · {}", sysinfo.product, sysinfo.manufacturer)),
                serial: Some(sysinfo.serial.clone()),
                usb: Some(format!(
                    "High Speed ({} Mbit/s) · {} · Bus {}, Dev {}",
                    sysinfo.speed_mbits, sysinfo.max_power, sysinfo.bus, sysinfo.dev
                )),
                connected: sysinfo.connected_secs.map(tasks::fmt_duration),
                ..Default::default()
            },
            spectrum: SpectrumState {
                step_hz: 100_000, y_min: -120.0, y_max: 0.0,
                hold: None, cursor_freq: None, markers: vec![], pending_marker: None,
            },
            waterfall: WaterfallState::new(cfg.display.waterfall_max_rows),
            system: SystemState {
                board_name: board_name.clone(), serial: serial.clone(),
                fw_version: "Observer Mode".to_string(),
                board_rev: 0xFE, usb_api_version: 0, cpld_ok: None,
                process_cpu_pct: 0.0, process_rss_mb: 0,
            },
            ui:  UiState::default(),
            acc: Accumulators::default(),
        }));

        {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            m.push_log(format!("Observer Mode: {} (Serial: {})", board_name, serial));
            m.push_log("Device is in use by another process — hardware controls disabled");
        }

        tasks::spawn_observer_task(Arc::clone(&state), sysinfo.bus, sysinfo.dev);
        tasks::spawn_sys_resource_task(Arc::clone(&state));

        let (engine, focus_keys) = Self::build_ui(&board_name, &serial, "observer");

        Ok(Self {
            state,
            device: None,
            rx_ctx: None,
            config_path,
            events: EventStream::new(Duration::from_millis(100)),
            show_help: false,
            engine,
            theme,
            focus_keys,
        })
    }
}
