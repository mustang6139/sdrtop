use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent};

use crate::hardware;
use crate::state::{InputMode, SdrMetrics, SpectrumMarker};
use crate::ui::{self, spectrum::{fmt_spectrum_step, next_spectrum_step, prev_spectrum_step}};
use crate::ui::waterfall::{next_wf_stride, prev_wf_stride};

pub enum KeyAction {
    Continue,
    Quit,
}

pub fn handle_key(
    key: KeyEvent,
    state: &Arc<Mutex<SdrMetrics>>,
    device: Option<&Arc<hardware::Device>>,
    engine: &mut ui::LayoutEngine,
    show_help: &mut bool,
    focus_keys: &HashMap<char, &'static str>,
) -> KeyAction {
    let input_mode = state.lock().unwrap_or_else(|e| e.into_inner()).ui.input_mode.clone();
    match input_mode {
        InputMode::Normal        => handle_normal(key, state, device, engine, show_help, focus_keys),
        InputMode::FrequencyInput  => { handle_freq_input(key, state, device); KeyAction::Continue }
        InputMode::SampleRateInput => { handle_sr_input(key, state, device);   KeyAction::Continue }
        InputMode::MarkerNameInput => { handle_marker_input(key, state);        KeyAction::Continue }
    }
}

// ── Normal mode ───────────────────────────────────────────────────────────────

fn handle_normal(
    key: KeyEvent,
    state: &Arc<Mutex<SdrMetrics>>,
    device: Option<&Arc<hardware::Device>>,
    engine: &mut ui::LayoutEngine,
    show_help: &mut bool,
    focus_keys: &HashMap<char, &'static str>,
) -> KeyAction {
    let focused = engine.focused_panel_name().map(|s| s.to_string());

    match focused.as_deref() {
        Some("spectrum")  => handle_spectrum_focus(key, state, device, engine, show_help, focus_keys),
        Some("waterfall") => handle_waterfall_focus(key, state, engine, show_help, focus_keys),
        _                 => handle_global(key, state, device, engine, show_help, focus_keys),
    }
}

// ── Spectrum focus keys ───────────────────────────────────────────────────────

fn handle_spectrum_focus(
    key: KeyEvent,
    state: &Arc<Mutex<SdrMetrics>>,
    device: Option<&Arc<hardware::Device>>,
    engine: &mut ui::LayoutEngine,
    show_help: &mut bool,
    focus_keys: &HashMap<char, &'static str>,
) -> KeyAction {
    match key.code {
        KeyCode::Left => {
            if let Some(device) = device {
                let (new_freq, result) = {
                    let m = state.lock().unwrap_or_else(|e| e.into_inner());
                    let f = m.radio.frequency.saturating_sub(m.spectrum.step_hz).max(1_000_000);
                    (f, device.set_frequency(f))
                };
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                match result {
                    Ok(()) => m.radio.frequency = new_freq,
                    Err(e) => m.push_log(format!("Tune error: {}", e)),
                }
            }
        }
        KeyCode::Right => {
            if let Some(device) = device {
                let (new_freq, result) = {
                    let m = state.lock().unwrap_or_else(|e| e.into_inner());
                    let f = (m.radio.frequency + m.spectrum.step_hz).min(6_000_000_000);
                    (f, device.set_frequency(f))
                };
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                match result {
                    Ok(()) => m.radio.frequency = new_freq,
                    Err(e) => m.push_log(format!("Tune error: {}", e)),
                }
            }
        }
        KeyCode::Char('[') => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            let new_step = prev_spectrum_step(m.spectrum.step_hz);
            m.spectrum.step_hz = new_step;
            m.push_log(format!("Step → {}", fmt_spectrum_step(new_step)));
        }
        KeyCode::Char(']') => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            let new_step = next_spectrum_step(m.spectrum.step_hz);
            m.spectrum.step_hz = new_step;
            m.push_log(format!("Step → {}", fmt_spectrum_step(new_step)));
        }
        KeyCode::Up => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            let new_min = (m.spectrum.y_min + 10.0).min(m.spectrum.y_max - 20.0);
            m.spectrum.y_min = new_min;
            let ymax = m.spectrum.y_max;
            m.push_log(format!("Zoom: {:.0}…{:.0} dBFS", new_min, ymax));
        }
        KeyCode::Down => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            let new_min = (m.spectrum.y_min - 10.0).max(-120.0);
            m.spectrum.y_min = new_min;
            let ymax = m.spectrum.y_max;
            m.push_log(format!("Zoom: {:.0}…{:.0} dBFS", new_min, ymax));
        }
        KeyCode::Char('j') => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            let step = m.spectrum.step_hz;
            m.spectrum.cursor_freq = Some(match m.spectrum.cursor_freq {
                Some(f) => f.saturating_sub(step).max(1_000_000),
                None    => m.radio.frequency,
            });
        }
        KeyCode::Char('k') => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            let step = m.spectrum.step_hz;
            m.spectrum.cursor_freq = Some(match m.spectrum.cursor_freq {
                Some(f) => (f + step).min(6_000_000_000),
                None    => m.radio.frequency,
            });
        }
        KeyCode::Char('m') => {
            let (marker_freq, existing_idx) = {
                let m = state.lock().unwrap_or_else(|e| e.into_inner());
                let freq = if let Some(f) = m.spectrum.cursor_freq {
                    f
                } else if let Some(frame) = &m.waterfall.last_fft {
                    let peak_bin = frame.bins_dbfs.iter()
                        .enumerate()
                        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                        .map(|(i, _)| i)
                        .unwrap_or(frame.bins_dbfs.len() / 2);
                    let left_hz = m.radio.frequency as f64 - frame.sample_rate / 2.0;
                    (left_hz + peak_bin as f64 / frame.bins_dbfs.len() as f64 * frame.sample_rate).round() as u64
                } else {
                    m.radio.frequency
                };
                let step = m.spectrum.step_hz;
                let idx = m.spectrum.markers.iter().position(|mk| {
                    (mk.freq_hz as i64 - freq as i64).unsigned_abs() < step
                });
                (freq, idx)
            };
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(idx) = existing_idx {
                let removed = m.spectrum.markers.remove(idx);
                m.push_log(format!("Marker removed: {}", removed.label));
            } else {
                m.spectrum.pending_marker = Some(marker_freq);
                m.ui.input_mode = InputMode::MarkerNameInput;
                m.ui.input_buf.clear();
                m.push_log(format!(
                    "Name this marker at {:.3} MHz (Enter = confirm, empty = auto-label)",
                    marker_freq as f64 / 1_000_000.0
                ));
            }
        }
        // All other keys fall through to global handler
        _ => return handle_global(key, state, device, engine, show_help, focus_keys),
    }
    KeyAction::Continue
}

// ── Waterfall focus keys ──────────────────────────────────────────────────────

fn handle_waterfall_focus(
    key: KeyEvent,
    state: &Arc<Mutex<SdrMetrics>>,
    engine: &mut ui::LayoutEngine,
    show_help: &mut bool,
    focus_keys: &HashMap<char, &'static str>,
) -> KeyAction {
    match key.code {
        KeyCode::Up => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            let new_min = (m.waterfall.db_min + 10.0).min(-20.0);
            m.waterfall.db_min = new_min;
            m.push_log(format!("Waterfall zoom: {:.0}…0 dBFS", new_min));
        }
        KeyCode::Down => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            let new_min = (m.waterfall.db_min - 10.0).max(-120.0);
            m.waterfall.db_min = new_min;
            m.push_log(format!("Waterfall zoom: {:.0}…0 dBFS", new_min));
        }
        KeyCode::Char('[') => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            let new_stride = prev_wf_stride(m.waterfall.buffer.row_stride);
            m.waterfall.buffer.set_row_stride(new_stride);
            m.push_log(format!("Waterfall: ×{} frames/row", new_stride));
        }
        KeyCode::Char(']') => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            let new_stride = next_wf_stride(m.waterfall.buffer.row_stride);
            m.waterfall.buffer.set_row_stride(new_stride);
            m.push_log(format!("Waterfall: ×{} frames/row", new_stride));
        }
        KeyCode::Char('m') => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            m.waterfall.cursor_freq = if m.waterfall.cursor_freq.is_some() {
                None
            } else {
                Some(m.radio.frequency)
            };
        }
        KeyCode::Left => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(cf) = m.waterfall.cursor_freq {
                m.waterfall.cursor_freq = Some(cf.saturating_sub(m.spectrum.step_hz).max(1_000_000));
            }
        }
        KeyCode::Right => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(cf) = m.waterfall.cursor_freq {
                m.waterfall.cursor_freq = Some((cf + m.spectrum.step_hz).min(6_000_000_000));
            }
        }
        KeyCode::Char('j') => {
            state.lock().unwrap_or_else(|e| e.into_inner()).waterfall.scroll_offset += 1;
        }
        KeyCode::Char('k') => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            m.waterfall.scroll_offset = m.waterfall.scroll_offset.saturating_sub(1);
        }
        _ => return handle_global_no_device(key, state, engine, show_help, focus_keys),
    }
    KeyAction::Continue
}

// ── Global keys (no panel focus) ─────────────────────────────────────────────

fn handle_global(
    key: KeyEvent,
    state: &Arc<Mutex<SdrMetrics>>,
    device: Option<&Arc<hardware::Device>>,
    engine: &mut ui::LayoutEngine,
    show_help: &mut bool,
    focus_keys: &HashMap<char, &'static str>,
) -> KeyAction {
    match key.code {
        KeyCode::Esc => {
            if engine.focused_panel_name().is_some() {
                engine.clear_focus();
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                m.ui.focused_panel = None;
                m.ui.focused_panel_bindings = &[];
                m.spectrum.cursor_freq = None;
                m.waterfall.scroll_offset = 0;
                m.waterfall.cursor_freq = None;
            }
        }
        KeyCode::Char('q') => return KeyAction::Quit,
        KeyCode::Char(' ') if device.is_some() => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            m.radio.rx_enabled = !m.radio.rx_enabled;
        }
        KeyCode::Char('r') => {
            use crate::state::{DEFAULT_FREQUENCY, DEFAULT_LNA_GAIN, DEFAULT_SAMPLE_RATE, DEFAULT_VGA_GAIN};
            if let Some(device) = device {
                let results = [
                    device.set_lna_gain(DEFAULT_LNA_GAIN),
                    device.set_vga_gain(DEFAULT_VGA_GAIN),
                    device.set_frequency(DEFAULT_FREQUENCY),
                    device.set_sample_rate(DEFAULT_SAMPLE_RATE),
                    device.set_amp_enable(false),
                ];
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                if results.iter().all(|r| r.is_ok()) {
                    m.reset_to_defaults();
                } else {
                    for r in &results {
                        if let Err(e) = r { m.push_log(format!("Reset error: {}", e)); }
                    }
                }
            }
        }
        KeyCode::Char('f') if device.is_some() => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            m.ui.input_mode = InputMode::FrequencyInput;
            m.ui.input_buf.clear();
            m.push_log("Enter frequency in MHz, then press Enter");
        }
        KeyCode::Char('s') if device.is_some() => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            m.ui.input_mode = InputMode::SampleRateInput;
            m.ui.input_buf.clear();
            m.push_log("Enter sample rate in MHz (2–20), then press Enter");
        }
        KeyCode::Char('?') => *show_help = !*show_help,
        KeyCode::Char('p') => {
            engine.cycle_preset();
            let name = engine.active_preset().to_string();
            state.lock().unwrap_or_else(|e| e.into_inner()).push_log(format!("Preset: {}", name));
        }
        KeyCode::Char('1') => { engine.set_preset("main");             state.lock().unwrap_or_else(|e| e.into_inner()).push_log("Preset: main"); }
        KeyCode::Char('2') => { engine.set_preset("spectrum");         state.lock().unwrap_or_else(|e| e.into_inner()).push_log("Preset: spectrum"); }
        KeyCode::Char('3') => { engine.set_preset("waterfall");        state.lock().unwrap_or_else(|e| e.into_inner()).push_log("Preset: waterfall"); }
        KeyCode::Char('4') => { engine.set_preset("spectrum_waterfall"); state.lock().unwrap_or_else(|e| e.into_inner()).push_log("Preset: spectrum+waterfall"); }
        KeyCode::Char('5') => { engine.set_preset("monitoring");       state.lock().unwrap_or_else(|e| e.into_inner()).push_log("Preset: monitoring"); }
        KeyCode::Char('6') => { engine.set_preset("lab");              state.lock().unwrap_or_else(|e| e.into_inner()).push_log("Preset: lab"); }
        KeyCode::Char('w') => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            m.waterfall.buffer.paused = !m.waterfall.buffer.paused;
            let s = if m.waterfall.buffer.paused { "paused" } else { "resumed" };
            m.push_log(format!("Waterfall {}", s));
        }
        KeyCode::Char('h') => {
            let held = {
                let m = state.lock().unwrap_or_else(|e| e.into_inner());
                m.waterfall.last_fft.as_ref().map(|fr| Arc::clone(&fr.bins_dbfs))
            };
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            if m.spectrum.hold.is_some() {
                m.spectrum.hold = None;
                m.push_log("Hold: off");
            } else if let Some(bins) = held {
                m.spectrum.hold = Some(bins);
                m.push_log("Hold: on — ghost spectrum frozen");
            }
        }
        KeyCode::Up => {
            if let Some(device) = device {
                let (gain, result) = {
                    let m = state.lock().unwrap_or_else(|e| e.into_inner());
                    let new_gain = (m.radio.lna_gain + 8).min(40);
                    (new_gain, device.set_lna_gain(new_gain))
                };
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                match result {
                    Ok(()) => { m.radio.lna_gain = gain; m.push_log(format!("LNA gain → {} dB", gain)); }
                    Err(e) => m.push_log(format!("LNA gain error: {}", e)),
                }
            }
        }
        KeyCode::Down => {
            if let Some(device) = device {
                let (gain, result) = {
                    let m = state.lock().unwrap_or_else(|e| e.into_inner());
                    let new_gain = m.radio.lna_gain.saturating_sub(8);
                    (new_gain, device.set_lna_gain(new_gain))
                };
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                match result {
                    Ok(()) => { m.radio.lna_gain = gain; m.push_log(format!("LNA gain → {} dB", gain)); }
                    Err(e) => m.push_log(format!("LNA gain error: {}", e)),
                }
            }
        }
        KeyCode::Char('[') => {
            if let Some(device) = device {
                let (gain, result) = {
                    let m = state.lock().unwrap_or_else(|e| e.into_inner());
                    let new_gain = m.radio.vga_gain.saturating_sub(2);
                    (new_gain, device.set_vga_gain(new_gain))
                };
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                match result {
                    Ok(()) => { m.radio.vga_gain = gain; m.push_log(format!("VGA gain → {} dB", gain)); }
                    Err(e) => m.push_log(format!("VGA gain error: {}", e)),
                }
            }
        }
        KeyCode::Char(']') => {
            if let Some(device) = device {
                let (gain, result) = {
                    let m = state.lock().unwrap_or_else(|e| e.into_inner());
                    let new_gain = (m.radio.vga_gain + 2).min(62);
                    (new_gain, device.set_vga_gain(new_gain))
                };
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                match result {
                    Ok(()) => { m.radio.vga_gain = gain; m.push_log(format!("VGA gain → {} dB", gain)); }
                    Err(e) => m.push_log(format!("VGA gain error: {}", e)),
                }
            }
        }
        KeyCode::Char('a') => {
            if let Some(device) = device {
                let (enabled, result) = {
                    let m = state.lock().unwrap_or_else(|e| e.into_inner());
                    let new_state = !m.radio.amp_enabled;
                    (new_state, device.set_amp_enable(new_state))
                };
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                match result {
                    Ok(()) => {
                        m.radio.amp_enabled = enabled;
                        m.push_log(format!("AMP {}", if enabled { "ON" } else { "OFF" }));
                    }
                    Err(e) => m.push_log(format!("AMP error: {}", e)),
                }
            }
        }
        KeyCode::Char(c) => {
            if let Some(&panel_name) = focus_keys.get(&c) {
                if engine.is_panel_visible(panel_name) {
                    engine.focus(panel_name);
                    let bindings = engine.get_panel_bindings(panel_name);
                    let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                    m.ui.focused_panel = Some(panel_name.to_string());
                    m.ui.focused_panel_bindings = bindings;
                }
            }
        }
        _ => {}
    }
    KeyAction::Continue
}

fn handle_global_no_device(
    key: KeyEvent,
    state: &Arc<Mutex<SdrMetrics>>,
    engine: &mut ui::LayoutEngine,
    show_help: &mut bool,
    focus_keys: &HashMap<char, &'static str>,
) -> KeyAction {
    handle_global(key, state, None, engine, show_help, focus_keys)
}

// ── Text input modes ──────────────────────────────────────────────────────────

fn handle_freq_input(
    key: KeyEvent,
    state: &Arc<Mutex<SdrMetrics>>,
    device: Option<&Arc<hardware::Device>>,
) {
    match key.code {
        KeyCode::Esc => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            m.ui.input_mode = InputMode::Normal;
            m.ui.input_buf.clear();
            m.push_log("Frequency input cancelled");
        }
        KeyCode::Backspace => { state.lock().unwrap_or_else(|e| e.into_inner()).ui.input_buf.pop(); }
        KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => {
            state.lock().unwrap_or_else(|e| e.into_inner()).ui.input_buf.push(c);
        }
        KeyCode::Enter => {
            if let Some(device) = device {
                let (freq_hz, result) = {
                    let m = state.lock().unwrap_or_else(|e| e.into_inner());
                    match m.ui.input_buf.parse::<f64>() {
                        Ok(mhz) if mhz > 0.0 => {
                            let hz = (mhz * 1_000_000.0) as u64;
                            (Some(hz), Some(device.set_frequency(hz)))
                        }
                        _ => (None, None),
                    }
                };
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                match (freq_hz, result) {
                    (Some(hz), Some(Ok(()))) => {
                        m.radio.frequency = hz;
                        m.ui.input_mode = InputMode::Normal;
                        m.ui.input_buf.clear();
                        m.push_log(format!("Frequency set to {:.3} MHz", hz as f64 / 1_000_000.0));
                    }
                    (Some(_), Some(Err(e))) => m.push_log(format!("Frequency error: {}", e)),
                    _ => {
                        let bad = m.ui.input_buf.clone();
                        m.push_log(format!("Invalid frequency: '{}'", bad));
                    }
                }
            }
        }
        _ => {}
    }
}

fn handle_sr_input(
    key: KeyEvent,
    state: &Arc<Mutex<SdrMetrics>>,
    device: Option<&Arc<hardware::Device>>,
) {
    match key.code {
        KeyCode::Esc => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            m.ui.input_mode = InputMode::Normal;
            m.ui.input_buf.clear();
            m.push_log("Sample rate input cancelled");
        }
        KeyCode::Backspace => { state.lock().unwrap_or_else(|e| e.into_inner()).ui.input_buf.pop(); }
        KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => {
            state.lock().unwrap_or_else(|e| e.into_inner()).ui.input_buf.push(c);
        }
        KeyCode::Enter => {
            if let Some(device) = device {
                let (rate_hz, result) = {
                    let m = state.lock().unwrap_or_else(|e| e.into_inner());
                    match m.ui.input_buf.parse::<f64>() {
                        Ok(mhz) if (2.0..=20.0).contains(&mhz) => {
                            let hz = mhz * 1_000_000.0;
                            (Some(hz), Some(device.set_sample_rate(hz)))
                        }
                        _ => (None, None),
                    }
                };
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                match (rate_hz, result) {
                    (Some(hz), Some(Ok(()))) => {
                        m.radio.config_sample_rate = hz;
                        m.ui.input_mode = InputMode::Normal;
                        m.ui.input_buf.clear();
                        m.push_log(format!("Sample rate set to {:.1} MHz", hz / 1_000_000.0));
                    }
                    (Some(_), Some(Err(e))) => m.push_log(format!("Sample rate error: {}", e)),
                    _ => {
                        let bad = m.ui.input_buf.clone();
                        m.push_log(format!("Invalid sample rate: '{}' (valid: 2–20 MHz)", bad));
                    }
                }
            }
        }
        _ => {}
    }
}

fn handle_marker_input(key: KeyEvent, state: &Arc<Mutex<SdrMetrics>>) {
    match key.code {
        KeyCode::Esc => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            m.ui.input_mode = InputMode::Normal;
            m.ui.input_buf.clear();
            m.spectrum.pending_marker = None;
            m.push_log("Marker cancelled");
        }
        KeyCode::Backspace => { state.lock().unwrap_or_else(|e| e.into_inner()).ui.input_buf.pop(); }
        KeyCode::Char(c) => { state.lock().unwrap_or_else(|e| e.into_inner()).ui.input_buf.push(c); }
        KeyCode::Enter => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(freq) = m.spectrum.pending_marker.take() {
                let label = if m.ui.input_buf.trim().is_empty() {
                    format!("M{}", m.spectrum.markers.len() + 1)
                } else {
                    m.ui.input_buf.trim().to_string()
                };
                m.push_log(format!("Marker: {} → {:.3} MHz", label, freq as f64 / 1_000_000.0));
                m.spectrum.markers.push(SpectrumMarker { freq_hz: freq, label });
            }
            m.ui.input_mode = InputMode::Normal;
            m.ui.input_buf.clear();
        }
        _ => {}
    }
}
