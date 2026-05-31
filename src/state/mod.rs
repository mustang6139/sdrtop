mod acc;
mod iq;
mod observer;
mod radio;
mod signal;
mod spectrum;
mod system;
mod ui;
mod waterfall;

pub(crate) use acc::Accumulators;
pub use iq::IqState;
pub use observer::ObserverState;
pub use radio::RadioState;
pub use signal::SignalState;
pub use spectrum::{SpectrumMarker, SpectrumState};
pub use system::SystemState;
pub use ui::{InputMode, UiState};
pub use waterfall::{FftFrame, WaterfallState};

pub const THROUGHPUT_HISTORY_LEN: usize = 64;
pub const DEFAULT_LNA_GAIN: u32 = 16;
pub const DEFAULT_VGA_GAIN: u32 = 20;
pub const DEFAULT_FREQUENCY: u64 = 2_400_000_000;
pub const DEFAULT_SAMPLE_RATE: f64 = 10_000_000.0;

#[derive(Clone)]
pub struct SdrMetrics {
    pub radio:    RadioState,
    pub signal:   SignalState,
    pub iq:       IqState,
    pub observer: ObserverState,
    pub spectrum: SpectrumState,
    pub waterfall: WaterfallState,
    pub system:   SystemState,
    pub ui:       UiState,
    pub(crate) acc: Accumulators,
}

impl SdrMetrics {
    pub fn push_log(&mut self, msg: impl Into<String>) {
        self.ui.push_log(msg);
    }

    pub fn reset_to_defaults(&mut self) {
        self.radio.lna_gain           = DEFAULT_LNA_GAIN;
        self.radio.vga_gain           = DEFAULT_VGA_GAIN;
        self.radio.amp_enabled        = false;
        self.radio.frequency          = DEFAULT_FREQUENCY;
        self.radio.config_sample_rate = DEFAULT_SAMPLE_RATE;
        self.push_log("Settings reset to defaults");
    }
}
