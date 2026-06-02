use std::collections::VecDeque;
use std::sync::Arc;

pub const LOG_MAX_ENTRIES: usize = 100;

#[derive(Clone, PartialEq)]
pub enum InputMode {
    Normal,
    FrequencyInput,
    SampleRateInput,
    MarkerNameInput,
}

#[derive(Clone)]
pub struct UiState {
    pub input_mode:             InputMode,
    pub input_buf:              String,
    pub focused_panel:          Option<String>,
    pub focused_panel_bindings: &'static [(&'static str, &'static str)],
    /// Name of the engine's active preset, synced each frame before draw so the
    /// footer can show it. The engine owns the authoritative value; this is a
    /// render-time mirror.
    pub active_preset:          String,
    pub log:                    VecDeque<Arc<str>>,
}

impl UiState {
    pub fn push_log(&mut self, msg: impl Into<String>) {
        if self.log.len() >= LOG_MAX_ENTRIES {
            self.log.pop_front();
        }
        self.log.push_back(Arc::from(msg.into()));
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            input_mode:             InputMode::Normal,
            input_buf:              String::new(),
            focused_panel:          None,
            focused_panel_bindings: &[],
            active_preset:          String::new(),
            log:                    VecDeque::new(),
        }
    }
}
