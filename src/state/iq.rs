#[derive(Clone)]
pub struct IqState {
    pub iq_imbalance_db:    f32,
    pub dc_offset_i:        f32,
    pub dc_offset_q:        f32,
    pub callback_jitter_us:  u64,
    pub iq_amplitude_hist:   [u64; 32],
    pub buf_fill_pct:        f32,
    pub phase_imbalance_deg: f32,
}
