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
                let acc_samples    = m.acc.sample_count;
                let acc_jitter_sum = m.acc.jitter_sum_us;
                let acc_jitter_cnt = m.acc.jitter_count;
                m.acc.drops         = 0;
                m.acc.saturated     = 0;
                m.acc.i_sum         = 0;
                m.acc.q_sum         = 0;
                m.acc.i_sq_sum      = 0;
                m.acc.q_sq_sum      = 0;
                m.acc.sample_count  = 0;
                m.acc.jitter_sum_us = 0;
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
                    let n = acc_samples as f64;
                    m.iq.dc_offset_i = (acc_i_sum as f64 / n / 128.0) as f32;
                    m.iq.dc_offset_q = (acc_q_sum as f64 / n / 128.0) as f32;
                    let i_rms = (acc_i_sq_sum as f64 / n).sqrt();
                    let q_rms = (acc_q_sq_sum as f64 / n).sqrt();
                    if q_rms > 0.0 {
                        m.iq.iq_imbalance_db = (20.0 * (i_rms / q_rms).log10()) as f32;
                    }
                }

                if let Some(jitter) = acc_jitter_sum.checked_div(acc_jitter_cnt) {
                    m.iq.callback_jitter_us = jitter;
                }

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
