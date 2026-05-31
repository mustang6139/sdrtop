use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::hardware::{self, Device, RxContext};
use crate::state::{SdrMetrics, THROUGHPUT_HISTORY_LEN};

/// Polls the HackRF device every 200 ms:
///   - starts / stops RX in response to `state.rx_enabled`
///   - computes throughput, drop rate, ADC saturation, IQ metrics, jitter
///   - writes results back to `state`
pub fn spawn_rx_task(
    state: Arc<Mutex<SdrMetrics>>,
    device: Arc<Device>,
    rx_ctx: Arc<RxContext>,
) {
    tokio::spawn(async move {
        let mut hw_rx_active = false;

        loop {
            let now = Instant::now();

            if hw_rx_active && !device.is_streaming() {
                let _ = device.stop_rx();
                hw_rx_active = false;
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                m.radio.rx_enabled = false;
                m.radio.hw_streaming = false;
                m.push_log("WARNING: Streaming stopped unexpectedly — press [Space] to restart");
            }

            {
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                let elapsed_ms = now.duration_since(m.radio.last_poll_time).as_millis() as u64;
                let bytes = m.radio.bytes_since_last_poll;
                m.radio.bytes_since_last_poll = 0;
                m.radio.last_poll_time = now;

                m.radio.hw_streaming = device.is_streaming();

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

                let acc_drops      = m.acc.drops;
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

                let saturable = acc_samples * 2;
                m.signal.adc_saturation_pct = if saturable > 0 {
                    (acc_saturated as f32 / saturable as f32) * 100.0
                } else {
                    0.0
                };
                if m.signal.adc_saturation_pct > m.signal.adc_saturation_peak {
                    m.signal.adc_saturation_peak = m.signal.adc_saturation_pct;
                }
                let sat_snapshot = m.signal.adc_saturation_pct;
                if m.signal.saturation_history.len() >= THROUGHPUT_HISTORY_LEN { m.signal.saturation_history.pop_front(); }
                m.signal.saturation_history.push_back(sat_snapshot);

                if acc_samples > 0 {
                    let n      = acc_samples as f64;
                    let mean_i = acc_i_sum as f64 / n;
                    let mean_q = acc_q_sum as f64 / n;
                    let var_i  = (acc_i_sq_sum as f64 / n - mean_i * mean_i).max(0.0);
                    let var_q  = (acc_q_sq_sum as f64 / n - mean_q * mean_q).max(0.0);

                    m.iq.dc_offset_i = (mean_i / 128.0) as f32;
                    m.iq.dc_offset_q = (mean_q / 128.0) as f32;

                    // Amplitude imbalance: AC RMS ratio (DC-subtracted stddev)
                    let i_ac = var_i.sqrt();
                    let q_ac = var_q.sqrt();
                    if q_ac > 0.0 {
                        m.iq.iq_imbalance_db = (20.0 * (i_ac / q_ac).log10()) as f32;
                    }

                    // Phase imbalance: sin(θ) = 2·cov(I,Q) / (var(I) + var(Q))
                    let cov_iq = acc_cross_sum as f64 / n - mean_i * mean_q;
                    let denom  = var_i + var_q;
                    if denom > 0.0 {
                        let sin_theta = (2.0 * cov_iq / denom).clamp(-1.0, 1.0);
                        m.iq.phase_imbalance_deg = (sin_theta.asin() * 180.0 / std::f64::consts::PI) as f32;
                    }
                }

                let usb_now = m.signal.usb_errors_session;
                let usb_delta = usb_now.saturating_sub(m.signal.usb_errors_last_poll);
                m.signal.usb_errors_last_poll = usb_now;
                if m.signal.usb_error_history.len() >= crate::state::THROUGHPUT_HISTORY_LEN {
                    m.signal.usb_error_history.pop_front();
                }
                m.signal.usb_error_history.push_back(usb_delta);

                if acc_jitter_cnt > 0 {
                    let mean = acc_jitter_sum / acc_jitter_cnt;
                    let sq_mean = acc_jitter_sq / acc_jitter_cnt;
                    let variance = sq_mean.saturating_sub(mean.saturating_mul(mean));
                    let stddev = (variance as f64).sqrt() as u64;
                    m.iq.cb_period_us = mean;
                    m.iq.cb_jitter_us = stddev;
                    if m.iq.jitter_history.len() >= crate::state::THROUGHPUT_HISTORY_LEN {
                        m.iq.jitter_history.pop_front();
                    }
                    m.iq.jitter_history.push_back(stddev);
                }

                let cap = rx_ctx.sample_tx.capacity().unwrap_or(4);
                m.iq.buf_fill_pct = if cap > 0 {
                    (rx_ctx.sample_tx.len() as f32 / cap as f32) * 100.0
                } else {
                    0.0
                };

                let _ = acc_drops;
            }

            let rx_enabled = state.lock().unwrap_or_else(|e| e.into_inner()).radio.rx_enabled;
            if rx_enabled && !hw_rx_active {
                let user_param = Arc::as_ptr(&rx_ctx) as *mut libc::c_void;
                match device.start_rx(hardware::rx_callback, user_param) {
                    Ok(()) => {
                        hw_rx_active = true;
                        state.lock().unwrap_or_else(|e| e.into_inner()).push_log("RX streaming started");
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
                match result {
                    Ok(()) => m.push_log("RX streaming stopped"),
                    Err(e) => m.push_log(format!("Error stopping RX: {}", e)),
                }
            }

            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    });
}
