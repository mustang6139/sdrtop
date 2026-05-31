use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, BorderType, Borders},
    Frame,
};

use crate::state::SdrMetrics;
use super::panel::Panel;

pub struct UsbSrPanel;

impl Panel for UsbSrPanel {
    fn name(&self) -> &'static str { "usb_sr" }
    fn min_size(&self) -> (u16, u16) { (60, 5) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, _focused: bool) {
        let cfg_msps = state.radio.config_sample_rate / 1_000_000.0;
        let block = Block::default()
            .title(format!(" USB Throughput  ·  SR {:.2} Msps ", cfg_msps))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_dim));
        let inner = block.inner(area);
        f.render_widget(block, area);

        if inner.height < 3 || inner.width < 20 { return; }

        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(inner);

        crate::ui::throughput::draw_usb_graph(f, cols[0], state, theme);
        crate::ui::sample_rate::draw_sr_graph(f, cols[1], state, theme);
    }
}
