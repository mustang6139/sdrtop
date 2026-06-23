/// Ring-buffer capacity for the IQ constellation (number of normalised sample pairs).
/// Oldest pairs are discarded when this limit is reached.
pub const CONSTELLATION_CAP: usize = 1024;

#[derive(Clone)]
pub struct IqState {
    pub iq_imbalance_db:    f32,
    pub dc_offset_i:        f32,
    pub dc_offset_q:        f32,
    pub cb_period_us:        u64,
    pub cb_jitter_us:        u64,
    pub jitter_history:      std::collections::VecDeque<u64>,
    pub iq_amplitude_hist:   [u64; 32],
    pub buf_fill_pct:        f32,
    pub buf_fill_history:    std::collections::VecDeque<u64>,
    pub phase_imbalance_deg: f32,
    /// IRR (image-rejection ratio, dB) trend history for the Lab IQ diagnostics
    /// sparkline. Sampled at the same ~500 ms cadence and [`SNR_HISTORY_LEN`] depth
    /// as the command-rail SIGNAL traces so a full panel-width sweep ≈ 60 s.
    pub irr_history:         std::collections::VecDeque<f32>,
    /// Decimated I/Q sample ring buffer for the 2-D constellation display.
    /// Values are normalised to [-1, 1] (divided by 128). Written in the RX
    /// hot-path at a 1 : [`CONST_DECIMATE`] decimation; oldest pairs are
    /// evicted once the buffer reaches [`CONSTELLATION_CAP`].
    pub constellation: std::collections::VecDeque<(f32, f32)>,
}
