use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::hardware::{RxContext, SdrDevice};
use crate::state::{SdrMetrics, THROUGHPUT_HISTORY_LEN};

/// Polls the HackRF device every 200 ms:
///   - starts / stops RX in response to `state.rx_enabled`
///   - computes throughput, drop rate, ADC saturation, IQ metrics, jitter
///   - writes results back to `state`
pub fn spawn_rx_task(
    state: Arc<Mutex<SdrMetrics>>,
    device: Arc<dyn SdrDevice>,
    rx_ctx: Arc<RxContext>,
) {
    tokio::spawn(async move {
        let mut hw_rx_active = false;
        // Throttles SNR history sampling to ~500 ms regardless of the 200 ms poll.
        let mut last_snr_push = Instant::now();
        // Online Welford accumulator for throughput (binary MB/s), reset each RX
        // session so the timing panel reports per-session mean / std-dev.
        let mut tp_count: u64 = 0;
        let mut tp_mean:  f64 = 0.0;
        let mut tp_m2:    f64 = 0.0;

        loop {
            // Single is_streaming() call per iteration — result used for both the
            // unexpected-stop check and the hw_streaming state update.
            let hw_streaming = device.is_streaming();
            let now = Instant::now();

            if hw_rx_active && !hw_streaming {
                let _ = device.stop_rx();
                hw_rx_active = false;
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                m.radio.rx_enabled = false;
                m.radio.hw_streaming = false;
                m.radio.rx_start_time = None;
                m.push_log("WARNING: Streaming stopped unexpectedly — press [Space] to restart");
            }

            // Lock block 1: snapshot + reset accumulators, do integer computations.
            // Floating-point transcendentals (sqrt, log10, asin) run outside the lock.
            let (acc_i_sum, acc_q_sum, acc_i_sq_sum, acc_q_sq_sum,
                 acc_cross_sum, acc_samples, acc_jitter_sum, acc_jitter_sq, acc_jitter_cnt,
                 cal_snap, acc_peak_amp) = {
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                let elapsed_ms = now.duration_since(m.radio.last_poll_time).as_millis() as u64;
                let bytes = m.radio.bytes_since_last_poll;
                m.radio.bytes_since_last_poll = 0;
                m.radio.last_poll_time = now;
                m.radio.hw_streaming = hw_streaming;

                if let Some(bps) = (bytes * 1000).checked_div(elapsed_ms) {
                    m.radio.current_throughput_bps = bps;
                    m.radio.actual_sample_rate = (m.radio.current_throughput_bps / 2) as u32;
                    let throughput_kb = m.radio.current_throughput_bps / 1024;
                    if m.radio.throughput_history.len() >= THROUGHPUT_HISTORY_LEN {
                        m.radio.throughput_history.pop_front();
                    }
                    m.radio.throughput_history.push_back(throughput_kb);
                    let actual_sr = m.radio.actual_sample_rate as u64;
                    if m.radio.sample_rate_history.len() >= THROUGHPUT_HISTORY_LEN {
                        m.radio.sample_rate_history.pop_front();
                    }
                    m.radio.sample_rate_history.push_back(actual_sr);
                }
                if let Some(dps) = (m.acc.drops * 1000).checked_div(elapsed_ms) {
                    m.signal.drops_per_sec = dps;
                }
                let drops_snapshot = m.signal.drops_per_sec;
                if m.signal.drop_history.len() >= THROUGHPUT_HISTORY_LEN { m.signal.drop_history.pop_front(); }
                m.signal.drop_history.push_back(drops_snapshot);

                let acc_saturated  = m.acc.saturated;
                let acc_i_sum      = m.acc.i_sum;
                let acc_q_sum      = m.acc.q_sum;
                let acc_i_sq_sum   = m.acc.i_sq_sum;
                let acc_q_sq_sum   = m.acc.q_sq_sum;
                let acc_cross_sum  = m.acc.iq_cross_sum;
                let acc_samples    = m.acc.sample_count;
                let acc_jitter_sum = m.acc.jitter_sum_us;
                let acc_jitter_sq  = m.acc.jitter_sq_sum;
                let acc_jitter_cnt = m.acc.jitter_count;
                m.acc.drops         = 0;
                m.acc.saturated     = 0;
                m.acc.i_sum         = 0;
                m.acc.q_sum         = 0;
                m.acc.i_sq_sum      = 0;
                m.acc.q_sq_sum      = 0;
                m.acc.iq_cross_sum  = 0;
                m.acc.sample_count  = 0;
                m.acc.jitter_sum_us = 0;
                m.acc.jitter_sq_sum = 0;
                m.acc.jitter_count  = 0;

                m.iq.iq_amplitude_hist = m.acc.iq_hist;
                m.acc.iq_hist = [0u64; 32];
                m.iq.adc_signed_hist = m.acc.adc_signed_hist;
                m.acc.adc_signed_hist = [0u64; 32];
                let acc_peak_amp = m.acc.peak_amp;
                m.acc.peak_amp = 0;
                m.signal.adc_clip_events = acc_saturated;

                let saturable = acc_samples * 2;
                m.signal.adc_saturation_pct = if saturable > 0 {
                    (acc_saturated as f32 / saturable as f32) * 100.0
                } else { 0.0 };
                if m.signal.adc_saturation_pct > m.signal.adc_saturation_peak {
                    m.signal.adc_saturation_peak = m.signal.adc_saturation_pct;
                }
                let sat_snapshot = m.signal.adc_saturation_pct;
                if m.signal.saturation_history.len() >= THROUGHPUT_HISTORY_LEN { m.signal.saturation_history.pop_front(); }
                m.signal.saturation_history.push_back(sat_snapshot);
                // Remember the moment of a real clip so the rail can show a fading
                // "last clip Xs" memory (decays in render; nothing flickers here).
                if sat_snapshot >= crate::state::SAT_CLIP_PCT {
                    m.signal.last_clip_at = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).ok();
                }

                let usb_now = m.signal.usb_errors_session;
                let usb_delta = usb_now.saturating_sub(m.signal.usb_errors_last_poll);
                m.signal.usb_errors_last_poll = usb_now;
                if m.signal.usb_error_history.len() >= crate::state::THROUGHPUT_HISTORY_LEN {
                    m.signal.usb_error_history.pop_front();
                }
                m.signal.usb_error_history.push_back(usb_delta);

                let cap = rx_ctx.sample_tx.capacity().unwrap_or(4);
                m.iq.buf_fill_pct = if cap > 0 {
                    rx_ctx.sample_tx.len() as f32 / cap as f32 * 100.0
                } else { 0.0 };
                let buf_sample = (m.iq.buf_fill_pct * 10.0) as u64;
                if m.iq.buf_fill_history.len() >= crate::state::THROUGHPUT_HISTORY_LEN {
                    m.iq.buf_fill_history.pop_front();
                }
                m.iq.buf_fill_history.push_back(buf_sample);

                // Snapshot the live correction state so the displayed impairment can
                // be reported as the residual AFTER correction (agreeing with the
                // corrected scope/constellation), not the raw hardware figure.
                let cal_snap = m.iq.cal;

                (acc_i_sum, acc_q_sum, acc_i_sq_sum, acc_q_sq_sum,
                 acc_cross_sum, acc_samples, acc_jitter_sum, acc_jitter_sq, acc_jitter_cnt,
                 cal_snap, acc_peak_amp)
            };

            // Floating-point IQ and jitter metrics — computed outside the lock.
            let dc_i:              f32;
            let dc_q:              f32;
            let dc_i_raw:          f32;          // mean I/Q in raw sample units
            let dc_q_raw:          f32;          //   (the amount DC-block subtracts)
            let iq_corr:           (f32, f32);   // candidate auto-cal Q-row coeffs
            let iq_imbalance_db:   Option<f32>;
            let phase_imbalance:   Option<f32>;
            let cb_period_us:      Option<u64>;
            let cb_jitter_us:      Option<u64>;
            let adc_peak_dbfs:     f32;           // loudest sample, dBFS (Lab RF loading)
            let adc_rms_dbfs:      f32;           // full-bandwidth RMS, dBFS

            if acc_samples > 0 {
                let n      = acc_samples as f64;
                let mean_i = acc_i_sum as f64 / n;
                let mean_q = acc_q_sum as f64 / n;
                let var_i  = (acc_i_sq_sum as f64 / n - mean_i * mean_i).max(0.0);
                let var_q  = (acc_q_sq_sum as f64 / n - mean_q * mean_q).max(0.0);
                let cov_iq = acc_cross_sum as f64 / n - mean_i * mean_q;

                // ADC loading: peak from the loudest sample, RMS from the total I/Q
                // power, both referenced to full scale (128 counts). −120 dBFS floor.
                adc_peak_dbfs = if acc_peak_amp > 0 {
                    20.0 * (acc_peak_amp as f32 / 128.0).log10()
                } else { -120.0 };
                adc_rms_dbfs = {
                    let p = (var_i + var_q) / (128.0 * 128.0);
                    if p > 0.0 { (10.0 * p.log10()) as f32 } else { -120.0 }
                };

                // Candidate auto-cal coefficients + DC to subtract are always derived
                // from the RAW moments — that is what [C]/[D] capture and apply.
                iq_corr  = crate::signal::iq_correction_coeffs(var_i, var_q, cov_iq);
                dc_i_raw = mean_i as f32;
                dc_q_raw = mean_q as f32;

                // Displayed impairment is the RESIDUAL after the active correction:
                // transform the moments by the applied Q-row when calibrated, and
                // subtract the blocked DC. With no correction these are the raw values.
                let (ev_i, ev_q, ecov) = if cal_snap.cal_applied {
                    crate::signal::corrected_moments(
                        var_i, var_q, cov_iq, cal_snap.c_qi as f64, cal_snap.c_qq as f64)
                } else {
                    (var_i, var_q, cov_iq)
                };
                let (emean_i, emean_q) = if cal_snap.dc_block_on || cal_snap.cal_applied {
                    (mean_i - cal_snap.dc_i_raw as f64, mean_q - cal_snap.dc_q_raw as f64)
                } else {
                    (mean_i, mean_q)
                };
                dc_i = (emean_i / 128.0) as f32;
                dc_q = (emean_q / 128.0) as f32;
                let i_ac = ev_i.sqrt();
                let q_ac = ev_q.sqrt();
                iq_imbalance_db = if q_ac > 0.0 {
                    Some((20.0 * (i_ac / q_ac).log10()) as f32)
                } else { None };
                let denom = ev_i + ev_q;
                phase_imbalance = if denom > 0.0 {
                    let sin_theta = (2.0 * ecov / denom).clamp(-1.0, 1.0);
                    Some((sin_theta.asin() * 180.0 / std::f64::consts::PI) as f32)
                } else { None };
            } else {
                dc_i = 0.0; dc_q = 0.0;
                dc_i_raw = 0.0; dc_q_raw = 0.0; iq_corr = (0.0, 1.0);
                iq_imbalance_db = None; phase_imbalance = None;
                adc_peak_dbfs = -120.0; adc_rms_dbfs = -120.0;
            }

            if acc_jitter_cnt > 0 {
                let mean     = acc_jitter_sum / acc_jitter_cnt;
                let sq_mean  = acc_jitter_sq  / acc_jitter_cnt;
                let variance = sq_mean.saturating_sub(mean.saturating_mul(mean));
                cb_period_us = Some(mean);
                cb_jitter_us = Some((variance as f64).sqrt() as u64);
            } else {
                cb_period_us = None;
                cb_jitter_us = None;
            }

            // Lock block 2: write computed IQ / jitter results, then read rx_enabled
            // in the same critical section to avoid a separate third lock.
            let rx_enabled = {
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                if acc_samples > 0 {
                    m.iq.dc_offset_i = dc_i;
                    m.iq.dc_offset_q = dc_q;
                    m.signal.adc_peak_dbfs = adc_peak_dbfs;
                    m.signal.adc_rms_dbfs  = adc_rms_dbfs;
                    if let Some(v) = iq_imbalance_db  { m.iq.iq_imbalance_db      = v; }
                    if let Some(v) = phase_imbalance   { m.iq.phase_imbalance_deg  = v; }

                    // DC-block tracks the live DC estimate so it follows slow drift.
                    if m.iq.cal.dc_block_on || m.iq.cal.cal_applied {
                        m.iq.cal.dc_i_raw = dc_i_raw;
                        m.iq.cal.dc_q_raw = dc_q_raw;
                    }
                    // [C] pressed → capture the correction matrix from this window
                    // (a one-shot snapshot; it stays fixed until the next auto-cal).
                    if m.iq.cal.cal_pending {
                        m.iq.cal.c_qi = iq_corr.0;
                        m.iq.cal.c_qq = iq_corr.1;
                        m.iq.cal.dc_i_raw = dc_i_raw;
                        m.iq.cal.dc_q_raw = dc_q_raw;
                        m.iq.cal.cal_applied = true;
                        m.iq.cal.cal_pending = false;
                        m.iq.cal.last_cal_at = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).ok();
                        let irr = crate::signal::image_rejection_db(
                            m.iq.iq_imbalance_db, m.iq.phase_imbalance_deg);
                        m.push_log(format!(
                            "IQ auto-cal applied \u{2014} quadrature corrected (was IRR {irr:.1} dB)"));
                    }
                }
                if let (Some(period), Some(jitter)) = (cb_period_us, cb_jitter_us) {
                    m.iq.cb_period_us = period;
                    m.iq.cb_jitter_us = jitter;
                    if m.iq.jitter_history.len() >= crate::state::THROUGHPUT_HISTORY_LEN {
                        m.iq.jitter_history.pop_front();
                    }
                    m.iq.jitter_history.push_back(jitter);
                }
                // Sample SNR / PWR / NF / SAT into their trend histories at ~500 ms
                // while streaming — one cadence and depth so the command rail's four
                // SIGNAL traces fill and align together.
                if hw_streaming && now.duration_since(last_snr_push) >= Duration::from_millis(500) {
                    last_snr_push = now;
                    let cap = crate::state::SNR_HISTORY_LEN;
                    let push = |h: &mut std::collections::VecDeque<f32>, v: f32| {
                        if h.len() >= cap { h.pop_front(); }
                        h.push_back(v);
                    };
                    // Read snapshots first so the mutable history borrows below
                    // don't overlap an immutable read of `m.signal`.
                    let snr = m.signal.peak_to_nf_db;
                    let pwr = m.signal.channel_power_dbfs;
                    let sat = m.signal.adc_saturation_pct;
                    let nf  = m.waterfall.last_fft.as_ref().map(|f| f.noise_floor);
                    // IRR from the freshly-written imbalance, via the shared helper
                    // the Lab IQ panel also uses (so trend and read-out agree).
                    let irr = crate::signal::image_rejection_db(
                        m.iq.iq_imbalance_db, m.iq.phase_imbalance_deg) as f32;
                    push(&mut m.signal.snr_history, snr);
                    if pwr.is_finite() { push(&mut m.signal.pwr_history, pwr); }
                    if let Some(nf) = nf { push(&mut m.signal.nf_history, nf); }
                    push(&mut m.signal.sat_history, sat);
                    push(&mut m.iq.irr_history, irr);
                }

                // Timing accuracy: fold this window's throughput into the running
                // Welford accumulator, then rebuild the TimingState snapshot from
                // the latest jitter / sample-rate / drop measurements.
                if hw_streaming {
                    let mbps = m.radio.current_throughput_bps as f64 / 1024.0 / 1024.0;
                    tp_count += 1;
                    let delta = mbps - tp_mean;
                    tp_mean += delta / tp_count as f64;
                    tp_m2  += delta * (mbps - tp_mean);
                }
                let tp_std = if tp_count > 1 { (tp_m2 / (tp_count - 1) as f64).sqrt() } else { 0.0 };
                let jitter_snapshot: Vec<u64> = m.iq.jitter_history.iter().copied().collect();
                // Carry the session jitter peak across the wholesale rebuild — it is
                // reset on RX start and by the timing panel's [R] focus binding.
                let prev_peak = m.timing.jitter_session_max_us;
                m.timing = crate::state::TimingState::compute(
                    m.iq.cb_period_us,
                    m.radio.config_sample_rate,
                    device.capabilities().samples_per_transfer,
                    &jitter_snapshot,
                    m.iq.cb_jitter_us,
                    m.radio.actual_sample_rate,
                    m.signal.drops_per_sec,
                    tp_mean,
                    tp_std,
                );
                m.timing.jitter_session_max_us = prev_peak.max(m.timing.jitter_max_us);

                m.radio.rx_enabled
            };
            if rx_enabled && !hw_rx_active {
                match device.start_rx(Arc::clone(&rx_ctx)) {
                    Ok(()) => {
                        hw_rx_active = true;
                        // Fresh per-session throughput statistics.
                        tp_count = 0; tp_mean = 0.0; tp_m2 = 0.0;
                        let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                        m.radio.rx_start_time = Some(Instant::now());
                        m.timing.jitter_session_max_us = 0;
                        m.push_log("RX streaming started");
                    }
                    Err(e) => {
                        let msg = format!("Error starting RX: {}", e);
                        let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                        m.radio.rx_enabled = false;
                        m.push_log(msg);
                    }
                }
            } else if !rx_enabled && hw_rx_active {
                let result = device.stop_rx();
                hw_rx_active = false;
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                m.radio.rx_start_time = None;
                match result {
                    Ok(()) => m.push_log("RX streaming stopped"),
                    Err(e) => m.push_log(format!("Error stopping RX: {}", e)),
                }
            }

            // ── Lab RF continuous auto-gain (AGC-lite) ──────────────────────────
            // Only when the [A] latch is set, streaming, and on a cascade-capable
            // radio. Re-centres the ADC peak when it drifts out of the comfortable
            // window, jumping LNA/VGA to the same staging target the one-shot uses.
            // Device sets run with no lock held; at the rails the target equals the
            // current gain, so there is no action (and no log spam).
            if hw_streaming && hw_rx_active && acc_samples > 0 {
                let (latched, friis, lna, vga) = {
                    let m = state.lock().unwrap_or_else(|e| e.into_inner());
                    (m.lab.rf_autotrack, m.caps.friis_applicable, m.radio.lna_gain, m.radio.vga_gain)
                };
                if latched && friis && !(-12.0..=-4.0).contains(&adc_peak_dbfs) {
                    let (lna_t, vga_t) =
                        crate::ui::rf_calc::staging_target(adc_peak_dbfs as f64, lna, vga);
                    if (lna_t, vga_t) != (lna, vga) {
                        let r1 = if lna_t != lna { device.set_lna_gain(lna_t) } else { Ok(()) };
                        let r2 = if vga_t != vga { device.set_vga_gain(vga_t) } else { Ok(()) };
                        let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                        match (r1, r2) {
                            (Ok(()), Ok(())) => {
                                m.radio.lna_gain = lna_t;
                                m.radio.vga_gain = vga_t;
                                m.push_log(format!(
                                    "Auto-gain track \u{2192} LNA {lna_t} \u{00b7} VGA {vga_t} dB"));
                            }
                            _ => m.push_log("Auto-gain track: device error".to_string()),
                        }
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    });
}
