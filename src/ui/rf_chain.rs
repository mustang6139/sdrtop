use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::hardware::device::compute_bb_filter_bw;
use crate::hardware::Device;
use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

pub struct RfChainPanel;

fn fmt_hz(hz: u32) -> String {
    if hz >= 1_000_000 {
        format!("{:.3} MHz", hz as f64 / 1_000_000.0)
    } else {
        format!("{} kHz", hz / 1_000)
    }
}

impl Panel for RfChainPanel {
    fn name(&self) -> &'static str { "rf_chain" }
    fn min_size(&self) -> (u16, u16) { (32, 12) }

    fn render(&self, f: &mut Frame, area: ratatui::layout::Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let border_color = if focused { theme.border_focused } else { theme.border_default };
        let block = Block::default()
            .title(" RF Chain ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let bb_bw = compute_bb_filter_bw(state.config_sample_rate);
        let total_gain = state.lna_gain as i32
            + state.vga_gain as i32
            + if state.amp_enabled { 14 } else { 0 };

        let cpld_span = match state.cpld_ok {
            Some(true)  => Span::styled("OK",       Style::default().fg(theme.status_ok)),
            Some(false) => Span::styled("MISMATCH", Style::default().fg(theme.status_crit).add_modifier(Modifier::BOLD)),
            None        => Span::styled("n/a",      Style::default().fg(theme.label)),
        };

        let lbl = Style::default().fg(theme.label);
        let val = Style::default().fg(theme.value);
        let hi  = Style::default().fg(theme.value_hi);

        let rows: &[Line] = &[
            Line::from(vec![
                Span::styled(format!("{:<12}", "Frequency"), lbl),
                Span::styled(format!("{:.3} MHz", state.frequency as f64 / 1_000_000.0), hi),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<12}", "Sample rate"), lbl),
                Span::styled(format!("{:.1} Msps", state.config_sample_rate / 1_000_000.0), val),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<12}", "BB filter"), lbl),
                Span::styled(fmt_hz(bb_bw), val),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<12}", "LNA gain"), lbl),
                Span::styled(format!("{} dB", state.lna_gain), val),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<12}", "VGA gain"), lbl),
                Span::styled(format!("{} dB", state.vga_gain), val),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<12}", "AMP"), lbl),
                Span::styled(
                    if state.amp_enabled { "ON  (+14 dB)" } else { "OFF" },
                    if state.amp_enabled { Style::default().fg(theme.status_warn) } else { val },
                ),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<12}", "Total gain"), lbl),
                Span::styled(format!("{} dB", total_gain), hi),
            ]),
            Line::from(vec![Span::raw("")]),
            Line::from(vec![
                Span::styled(format!("{:<12}", "Board"), lbl),
                Span::styled(Device::board_rev_name(state.board_rev), val),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<12}", "Firmware"), lbl),
                Span::styled(state.fw_version.clone(), val),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<12}", "USB API"), lbl),
                Span::styled(format!("{:#06x}", state.usb_api_version), val),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<12}", "CPLD"), lbl),
                cpld_span,
            ]),
        ];

        let n = rows.len().min(inner.height as usize);
        if n == 0 { return; }
        let constraints: Vec<Constraint> = (0..n).map(|_| Constraint::Length(1)).collect();
        let row_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        for (i, line) in rows.iter().take(n).enumerate() {
            f.render_widget(Paragraph::new(line.clone()), row_areas[i]);
        }
    }
}
