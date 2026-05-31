use std::time::Instant;

/// Raw accumulators written by the hardware RX callback and consumed by the
/// polling task. Never visible to the UI layer.
#[derive(Clone, Default)]
pub(crate) struct Accumulators {
    pub drops:         u64,
    pub saturated:     u64,
    pub i_sum:         i64,
    pub q_sum:         i64,
    pub i_sq_sum:      u64,
    pub q_sq_sum:      u64,
    pub sample_count:  u64,
    pub jitter_sum_us: u64,
    pub jitter_count:  u64,
    pub iq_cross_sum:  i64,
    pub last_callback: Option<Instant>,
    pub iq_hist:       [u64; 32],
}
