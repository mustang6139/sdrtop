pub mod footer;
pub mod gains;
pub mod header;
pub mod layout;
pub mod log;
pub mod telemetry;

// Stubs — populated in later phases
pub mod overlay;
pub mod sparkline;
pub mod spectrum;
pub mod waterfall;

use ratatui::Frame;

use crate::state::SdrMetrics;

pub fn draw(f: &mut Frame, m: &SdrMetrics, board_name: &str, fw: &str, serial: &str) {
    let chunks = layout::build(f.size());
    header::render(f, chunks.header, board_name, fw, serial);
    telemetry::render(f, chunks.body_left, m, board_name, serial);
    gains::render(f, chunks.body_right, m);
    log::render(f, chunks.log, m);
    footer::render(f, chunks.footer);
}
