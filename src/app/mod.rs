mod builder;
pub mod input;

use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ratatui::{backend::Backend, Terminal};

use crate::config::{AppConfig, DisplayConfig, RadioConfig};
use crate::event::{AppEvent, EventStream};
use crate::hardware::{self, RxContext};
use crate::state::SdrMetrics;
use crate::ui;

pub struct App {
    pub(super) state:       Arc<Mutex<SdrMetrics>>,
    pub(super) device:      Option<Arc<hardware::Device>>,
    #[allow(dead_code)]
    pub(super) rx_ctx:      Option<Arc<RxContext>>,
    pub(super) config_path: Option<PathBuf>,
    pub(super) events:      EventStream,
    pub(super) show_help:   bool,
    pub(super) show_footer: bool,
    pub(super) engine:      ui::LayoutEngine,
    pub(super) theme:       crate::Theme,
    pub(super) focus_keys:  HashMap<char, &'static str>,
}

impl App {
    pub fn new(cfg: AppConfig, config_path: Option<PathBuf>, device_index: usize) -> anyhow::Result<Self> {
        match hardware::Device::open_at(device_index) {
            Ok(raw_device) => Self::new_normal(cfg, config_path, raw_device),
            Err(open_err) => {
                let Some(sysinfo) = hardware::sysfs::find_hackrf() else {
                    return Err(open_err);
                };
                Self::new_observer(cfg, config_path, sysinfo)
            }
        }
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        const FRAME_DURATION: Duration = Duration::from_millis(33);
        let mut last_draw = Instant::now();

        self.draw(terminal)?;

        loop {
            let needs_redraw = match self.events.recv() {
                AppEvent::Key(key) => {
                    match input::handle_key(
                        key,
                        &self.state,
                        self.device.as_ref(),
                        &mut self.engine,
                        &mut self.show_help,
                        &mut self.show_footer,
                        &self.focus_keys,
                    ) {
                        input::KeyAction::Quit => {
                            self.save_config();
                            return Ok(());
                        }
                        input::KeyAction::Continue => {}
                    }
                    last_draw.elapsed() >= FRAME_DURATION
                }
                AppEvent::Tick => true,
            };

            if needs_redraw {
                self.draw(terminal)?;
                last_draw = Instant::now();
            }
        }
    }

    fn draw<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        let mut m = self.state.lock().unwrap_or_else(|e| e.into_inner()).clone();
        // Mirror the engine's active preset into the cloned snapshot so the
        // footer can render it without reaching into the engine.
        m.ui.active_preset = self.engine.active_preset().to_string();
        let hide_footer = !self.show_footer
            && m.ui.input_mode == crate::state::InputMode::Normal;
        self.engine.set_panel_hidden("footer", hide_footer);
        terminal.draw(|f| {
            self.engine.draw(f, &m, &self.theme);
            if self.show_help { ui::overlay::render_help(f); }
        })?;
        Ok(())
    }

    fn save_config(&self) {
        if self.device.is_none() { return; }
        let Some(path) = &self.config_path else { return };
        let (freq, rate, lna, vga, amp, wf_rows, markers) = {
            let m = self.state.lock().unwrap_or_else(|e| e.into_inner());
            (m.radio.frequency, m.radio.config_sample_rate, m.radio.lna_gain,
             m.radio.vga_gain, m.radio.amp_enabled, m.waterfall.buffer.max_rows,
             m.spectrum.markers.clone())
        };
        let cfg = AppConfig {
            radio: RadioConfig { frequency_hz: freq, sample_rate: rate, lna_gain: lna, vga_gain: vga, amp_enabled: amp },
            display: DisplayConfig {
                active_preset:      self.engine.active_preset().to_string(),
                waterfall_max_rows: wf_rows,
                spectrum_markers:   markers,
            },
            theme: crate::config::ThemeConfig { base: self.theme.name.to_string(), ..Default::default() },
        };
        let _ = cfg.save(path);
    }
}

#[cfg(test)]
mod tests {
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
        let acc_samples   = 100_u64;
        let saturable     = acc_samples * 2;
        let pct = (acc_saturated as f32 / saturable as f32) * 100.0;
        assert!((pct - 100.0).abs() < 0.01, "expected 100%, got {}", pct);
    }
}
