use std::collections::VecDeque;

#[derive(Clone)]
pub struct SignalState {
    pub drops_per_sec:       u64,
    pub total_drops_session: u64,
    pub drop_history:        VecDeque<u64>,
    pub adc_saturation_pct:  f32,
    pub adc_saturation_peak: f32,
    pub saturation_history:  VecDeque<f32>,
    pub snr_db:              f32,
    pub channel_power_dbfs:  f32,
    pub occupied_bw_hz:      u64,
    pub usb_errors_session:   u64,
    pub usb_errors_last_poll: u64,
    pub usb_error_history:    std::collections::VecDeque<u64>,
}
