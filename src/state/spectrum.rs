use std::sync::Arc;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpectrumMarker {
    pub freq_hz: u64,
    pub label:   String,
}

#[derive(Clone)]
pub struct SpectrumState {
    pub step_hz:        u64,
    pub y_min:          f32,
    pub y_max:          f32,
    pub hold:           Option<Arc<Vec<f32>>>,
    pub cursor_freq:    Option<u64>,
    pub markers:        Vec<SpectrumMarker>,
    pub pending_marker: Option<u64>,
}
