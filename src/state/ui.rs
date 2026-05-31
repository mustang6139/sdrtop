use std::collections::VecDeque;

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
    pub log:                    VecDeque<String>,
}

impl UiState {
    pub fn push_log(&mut self, msg: impl Into<String>) {
        if self.log.len() >= LOG_MAX_ENTRIES {
            self.log.pop_front();
        }
        self.log.push_back(msg.into());
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            input_mode:             InputMode::Normal,
            input_buf:              String::new(),
            focused_panel:          None,
            focused_panel_bindings: &[],
            log:                    VecDeque::new(),
        }
    }
}
