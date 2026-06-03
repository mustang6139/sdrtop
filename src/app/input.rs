use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent};

use crate::hardware;
use crate::state::{InputMode, MicroView, SdrMetrics, SpectrumMarker};
use crate::ui::{self, spectrum::{fmt_spectrum_step, next_spectrum_step, prev_spectrum_step}};
use crate::ui::waterfall::{next_wf_stride, prev_wf_stride, next_wf_zoom, prev_wf_zoom};

fn fmt_bw(hz: u64) -> String {
    if hz >= 1_000_000 { format!("{:.1} MHz", hz as f64 / 1_000_000.0) }
    else if hz >= 1_000 { format!("{} kHz", hz / 1_000) }
    else                { format!("{} Hz", hz) }
}

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
    show_footer: &mut bool,
    focus_keys: &HashMap<char, &'static str>,
) -> KeyAction {
    let input_mode = state.lock().unwrap_or_else(|e| e.into_inner()).ui.input_mode.clone();
    match input_mode {
        InputMode::Normal        => handle_normal(key, state, device, engine, show_help, show_footer, focus_keys),
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
    show_footer: &mut bool,
    focus_keys: &HashMap<char, &'static str>,
) -> KeyAction {
    let focused = engine.focused_panel_name().map(|s| s.to_string());

    match focused.as_deref() {
        Some("spectrum")        => handle_spectrum_focus(key, state, device, engine, show_help, show_footer, focus_keys),
        Some("waterfall")       => handle_waterfall_focus(key, state, engine, show_help, show_footer, focus_keys),
        Some("iq_diagnostics")  => handle_iq_focus(key, state, device, engine, show_help, show_footer, focus_keys),
        Some("hardware_health") => handle_health_focus(key, state, device, engine, show_help, show_footer, focus_keys),
        Some("timing_panel")    => handle_timing_focus(key, state, device, engine, show_help, show_footer, focus_keys),
        _                       => handle_global(key, state, device, engine, show_help, show_footer, focus_keys),
    }
}

// ── Spectrum focus keys ───────────────────────────────────────────────────────

fn handle_spectrum_focus(
    key: KeyEvent,
    state: &Arc<Mutex<SdrMetrics>>,
    device: Option<&Arc<hardware::Device>>,
    engine: &mut ui::LayoutEngine,
    show_help: &mut bool,
    show_footer: &mut bool,
    focus_keys: &HashMap<char, &'static str>,
) -> KeyAction {
    match key.code {
        KeyCode::Left => {
            if let Some(device) = device {
                let new_freq = {
                    let m = state.lock().unwrap_or_else(|e| e.into_inner());
                    m.radio.frequency.saturating_sub(m.spectrum.step_hz).max(1_000_000)
                };
                let result = device.set_frequency(new_freq);
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                match result {
                    Ok(()) => m.radio.frequency = new_freq,
                    Err(e) => m.push_log(format!("Tune error: {}", e)),
                }
            }
        }
        KeyCode::Right => {
            if let Some(device) = device {
                let new_freq = {
                    let m = state.lock().unwrap_or_else(|e| e.into_inner());
                    (m.radio.frequency + m.spectrum.step_hz).min(6_000_000_000)
                };
                let result = device.set_frequency(new_freq);
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
        KeyCode::Char('b') => {
            const BW_STEPS: &[u64] = &[6_250, 12_500, 25_000, 50_000, 100_000, 200_000, 500_000];
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            let cursor = m.spectrum.cursor_freq.unwrap_or(m.radio.frequency);
            let step   = m.spectrum.step_hz;
            if let Some(mk) = m.spectrum.markers.iter_mut()
                .min_by_key(|mk| (mk.freq_hz as i64 - cursor as i64).unsigned_abs())
                .filter(|mk| (mk.freq_hz as i64 - cursor as i64).unsigned_abs() < step * 4)
            {
                let next = match mk.channel_bw_hz {
                    None      => Some(BW_STEPS[0]),
                    Some(cur) => {
                        let idx = BW_STEPS.iter().position(|&b| b == cur);
                        idx.and_then(|i| BW_STEPS.get(i + 1)).copied()
                    }
                };
                mk.channel_bw_hz = next;
                mk.measured_bw_hz = None;
                let msg = match next {
                    Some(bw) => format!("Marker '{}' channel BW → {}", mk.label, fmt_bw(bw)),
                    None     => format!("Marker '{}' channel BW cleared", mk.label),
                };
                m.push_log(msg);
            } else {
                m.push_log("No marker near cursor — place one with [M] first");
            }
        }
        // All other keys fall through to global handler
        _ => return handle_global(key, state, device, engine, show_help, show_footer, focus_keys),
    }
    KeyAction::Continue
}

// ── Waterfall focus keys ──────────────────────────────────────────────────────

fn handle_waterfall_focus(
    key: KeyEvent,
    state: &Arc<Mutex<SdrMetrics>>,
    engine: &mut ui::LayoutEngine,
    show_help: &mut bool,
    show_footer: &mut bool,
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
        KeyCode::Char('+') | KeyCode::Char('=') => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            let new_zoom = next_wf_zoom(m.waterfall.hz_zoom);
            m.waterfall.hz_zoom = new_zoom;
            m.push_log(format!("Waterfall zoom: ×{}", new_zoom));
        }
        KeyCode::Char('-') => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            let new_zoom = prev_wf_zoom(m.waterfall.hz_zoom);
            m.waterfall.hz_zoom = new_zoom;
            if new_zoom == 1 {
                m.push_log("Waterfall zoom: off".to_string());
            } else {
                m.push_log(format!("Waterfall zoom: ×{}", new_zoom));
            }
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
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            let max = m.waterfall.buffer.rows.len() / 2;
            m.waterfall.scroll_offset = (m.waterfall.scroll_offset + 1).min(max);
        }
        KeyCode::Char('k') => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            m.waterfall.scroll_offset = m.waterfall.scroll_offset.saturating_sub(1);
        }
        _ => return handle_global_no_device(key, state, engine, show_help, show_footer, focus_keys),
    }
    KeyAction::Continue
}

// ── Lab panel focus keys ──────────────────────────────────────────────────────
//
// Each lab panel's focus mode adds only panel-specific actions; every other key
// falls through to the global handler (so Esc, Space, gain, etc. keep working).
// `rf_chain` deliberately has no focus mode — its gain controls are already the
// global [↑↓]/[[]]/[A]/[R] bindings, so a focus mode would only duplicate them.

/// `iq_diagnostics` focus: `[C]` logs a one-line snapshot of the current IQ
/// balance figures as a reference capture.
fn handle_iq_focus(
    key: KeyEvent,
    state: &Arc<Mutex<SdrMetrics>>,
    device: Option<&Arc<hardware::Device>>,
    engine: &mut ui::LayoutEngine,
    show_help: &mut bool,
    show_footer: &mut bool,
    focus_keys: &HashMap<char, &'static str>,
) -> KeyAction {
    if let KeyCode::Char('c') = key.code {
        let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
        let (dci, dcq, imb, ph) = (m.iq.dc_offset_i, m.iq.dc_offset_q, m.iq.iq_imbalance_db, m.iq.phase_imbalance_deg);
        m.push_log(format!(
            "IQ snapshot — DC I:{:+.3} Q:{:+.3} · imbalance {:+.1} dB · phase {:+.1}°",
            dci, dcq, imb, ph
        ));
        return KeyAction::Continue;
    }
    handle_global(key, state, device, engine, show_help, show_footer, focus_keys)
}

/// `hardware_health` focus (`[V]`): `[R]` resets the session drop counter, `[C]`
/// clears the trend sparkline histories.
fn handle_health_focus(
    key: KeyEvent,
    state: &Arc<Mutex<SdrMetrics>>,
    device: Option<&Arc<hardware::Device>>,
    engine: &mut ui::LayoutEngine,
    show_help: &mut bool,
    show_footer: &mut bool,
    focus_keys: &HashMap<char, &'static str>,
) -> KeyAction {
    match key.code {
        KeyCode::Char('r') => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            m.signal.total_drops_session = 0;
            m.push_log("Session drop counter reset");
        }
        KeyCode::Char('c') => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            m.signal.drop_history.clear();
            m.signal.saturation_history.clear();
            m.signal.usb_error_history.clear();
            m.iq.buf_fill_history.clear();
            m.system.cpu_history.clear();
            m.push_log("Health trend history cleared");
        }
        _ => return handle_global(key, state, device, engine, show_help, show_footer, focus_keys),
    }
    KeyAction::Continue
}

/// `timing_panel` focus (`[T]`): `[R]` resets the session jitter peak, `[C]`
/// clears the jitter / throughput / sample-rate histories.
fn handle_timing_focus(
    key: KeyEvent,
    state: &Arc<Mutex<SdrMetrics>>,
    device: Option<&Arc<hardware::Device>>,
    engine: &mut ui::LayoutEngine,
    show_help: &mut bool,
    show_footer: &mut bool,
    focus_keys: &HashMap<char, &'static str>,
) -> KeyAction {
    match key.code {
        KeyCode::Char('r') => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            m.timing.jitter_session_max_us = 0;
            m.push_log("Jitter session peak reset");
        }
        KeyCode::Char('c') => {
            let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
            m.iq.jitter_history.clear();
            m.radio.throughput_history.clear();
            m.radio.sample_rate_history.clear();
            m.push_log("Timing trend history cleared");
        }
        _ => return handle_global(key, state, device, engine, show_help, show_footer, focus_keys),
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
    show_footer: &mut bool,
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
                let (sr_result, bb_bw) = match device.set_sample_rate(DEFAULT_SAMPLE_RATE) {
                    Ok(bw) => (Ok(()), bw),
                    Err(e) => (Err(e), crate::hardware::compute_bb_filter_bw(DEFAULT_SAMPLE_RATE)),
                };
                let results = [
                    device.set_lna_gain(DEFAULT_LNA_GAIN),
                    device.set_vga_gain(DEFAULT_VGA_GAIN),
                    device.set_frequency(DEFAULT_FREQUENCY),
                    sr_result,
                    device.set_amp_enable(false),
                ];
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                if results.iter().all(|r| r.is_ok()) {
                    m.reset_to_defaults();
                    m.radio.bb_filter_hz = bb_bw;
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
        KeyCode::Tab       => *show_footer = !*show_footer,
        KeyCode::Char('p') => {
            engine.cycle_preset();
            let name = engine.active_preset().to_string();
            state.lock().unwrap_or_else(|e| e.into_inner()).push_log(format!("Preset: {}", name));
        }
        KeyCode::Char('1') => { engine.set_preset("main");             state.lock().unwrap_or_else(|e| e.into_inner()).push_log("Preset: main"); }
        KeyCode::Char('2') => { engine.set_preset("spectrum");         state.lock().unwrap_or_else(|e| e.into_inner()).push_log("Preset: spectrum"); }
        KeyCode::Char('3') => { engine.set_preset("waterfall");        state.lock().unwrap_or_else(|e| e.into_inner()).push_log("Preset: waterfall"); }
        KeyCode::Char('4') => { engine.set_preset("spectrum_waterfall"); state.lock().unwrap_or_else(|e| e.into_inner()).push_log("Preset: spectrum+waterfall"); }
        // Lab family on [5]–[8]. Each lights up automatically once its preset is
        // defined; until then it logs without switching.
        KeyCode::Char('5') => { try_set_preset(engine, state, "lab_iq"); }
        KeyCode::Char('6') => { try_set_preset(engine, state, "lab_rf"); }
        KeyCode::Char('7') => { try_set_preset(engine, state, "lab_timing"); }
        KeyCode::Char('8') => { try_set_preset(engine, state, "lab_signal"); }
        // [9] reserved for the future lab_sweep (Phase 6); pre-wired so it activates
        // the moment that preset exists.
        KeyCode::Char('9') => { try_set_preset(engine, state, "lab_sweep"); }
        KeyCode::Char('0') => { cycle_micro(engine, state); }
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
                let new_gain = { let m = state.lock().unwrap_or_else(|e| e.into_inner()); (m.radio.lna_gain + 8).min(40) };
                let result = device.set_lna_gain(new_gain);
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                match result {
                    Ok(()) => { m.radio.lna_gain = new_gain; m.push_log(format!("LNA gain → {} dB", new_gain)); }
                    Err(e) => m.push_log(format!("LNA gain error: {}", e)),
                }
            }
        }
        KeyCode::Down => {
            if let Some(device) = device {
                let new_gain = { let m = state.lock().unwrap_or_else(|e| e.into_inner()); m.radio.lna_gain.saturating_sub(8) };
                let result = device.set_lna_gain(new_gain);
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                match result {
                    Ok(()) => { m.radio.lna_gain = new_gain; m.push_log(format!("LNA gain → {} dB", new_gain)); }
                    Err(e) => m.push_log(format!("LNA gain error: {}", e)),
                }
            }
        }
        KeyCode::Char('[') => {
            if let Some(device) = device {
                let new_gain = { let m = state.lock().unwrap_or_else(|e| e.into_inner()); m.radio.vga_gain.saturating_sub(2) };
                let result = device.set_vga_gain(new_gain);
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                match result {
                    Ok(()) => { m.radio.vga_gain = new_gain; m.push_log(format!("VGA gain → {} dB", new_gain)); }
                    Err(e) => m.push_log(format!("VGA gain error: {}", e)),
                }
            }
        }
        KeyCode::Char(']') => {
            if let Some(device) = device {
                let new_gain = { let m = state.lock().unwrap_or_else(|e| e.into_inner()); (m.radio.vga_gain + 2).min(62) };
                let result = device.set_vga_gain(new_gain);
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                match result {
                    Ok(()) => { m.radio.vga_gain = new_gain; m.push_log(format!("VGA gain → {} dB", new_gain)); }
                    Err(e) => m.push_log(format!("VGA gain error: {}", e)),
                }
            }
        }
        KeyCode::Char('a') => {
            if let Some(device) = device {
                let new_state = { let m = state.lock().unwrap_or_else(|e| e.into_inner()); !m.radio.amp_enabled };
                let result = device.set_amp_enable(new_state);
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                match result {
                    Ok(()) => {
                        m.radio.amp_enabled = new_state;
                        m.push_log(format!("AMP {}", if new_state { "ON" } else { "OFF" }));
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
    show_footer: &mut bool,
    focus_keys: &HashMap<char, &'static str>,
) -> KeyAction {
    handle_global(key, state, None, engine, show_help, show_footer, focus_keys)
}

/// Switch to `name` if the preset is defined, otherwise log that it is not yet
/// available. This keeps the number-key framework (`[6]`–`[9]`, `[0]`) in place
/// before the presets themselves exist, so each one activates the moment it is
/// added to the layout config.
fn try_set_preset(engine: &mut ui::LayoutEngine, state: &Arc<Mutex<SdrMetrics>>, name: &str) -> KeyAction {
    let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
    if engine.has_preset(name) {
        engine.set_preset(name);
        m.push_log(format!("Preset: {}", name));
    } else {
        m.push_log(format!("Preset '{}' not yet available", name));
    }
    KeyAction::Continue
}

/// The `[0]` micro-ecosystem cycle. Entering from a non-micro preset lands on
/// `micro_main`; pressing `[0]` again while already in a micro preset advances
/// to the next view. A target whose preset is not yet defined is logged and
/// skipped (micro_view does not advance), so the cycle never strands the user on
/// a blank view while the micro presets are still being built out.
fn cycle_micro(engine: &mut ui::LayoutEngine, state: &Arc<Mutex<SdrMetrics>>) {
    // Sweep is a future capability — not part of the cycle yet.
    const SWEEP_ACTIVE: bool = false;
    let in_micro = engine.active_preset().starts_with("micro_");
    let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
    let target = if in_micro { m.ui.micro_view.next(SWEEP_ACTIVE) } else { MicroView::Main };
    if engine.has_preset(target.preset_name()) {
        m.ui.micro_view = target;
        engine.set_preset(target.preset_name());
        m.push_log(format!("Micro: {} ({}/{})", target.label(), target.position(), MicroView::total(SWEEP_ACTIVE)));
    } else {
        m.push_log(format!("Preset '{}' not yet available", target.preset_name()));
    }
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
                let freq_hz: Option<u64> = {
                    let m = state.lock().unwrap_or_else(|e| e.into_inner());
                    m.ui.input_buf.parse::<f64>().ok()
                        .filter(|&mhz| mhz > 0.0)
                        .map(|mhz| (mhz * 1_000_000.0) as u64)
                };
                let result = freq_hz.map(|hz| device.set_frequency(hz));
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
                let rate_hz: Option<f64> = {
                    let m = state.lock().unwrap_or_else(|e| e.into_inner());
                    m.ui.input_buf.parse::<f64>().ok()
                        .filter(|&mhz| (2.0..=20.0).contains(&mhz))
                        .map(|mhz| mhz * 1_000_000.0)
                };
                // Release lock before calling device — hackrf_set_sample_rate is a
                // blocking USB control transfer; holding the mutex here deadlocks the
                // rx_callback thread that needs the same lock to return.
                let result = rate_hz.map(|hz| device.set_sample_rate(hz));
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                match (rate_hz, result) {
                    (Some(hz), Some(Ok(bw))) => {
                        m.radio.config_sample_rate = hz;
                        m.radio.bb_filter_hz = bw;
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
                m.spectrum.markers.push(SpectrumMarker { freq_hz: freq, label, channel_bw_hz: None, measured_bw_hz: None });
            }
            m.ui.input_mode = InputMode::Normal;
            m.ui.input_buf.clear();
        }
        _ => {}
    }
}
