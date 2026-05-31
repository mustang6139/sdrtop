use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

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

fn gain_advice(hist: &[u64; 32]) -> (&'static str, bool) {
    let total: u64 = hist.iter().sum();
    if total == 0 { return ("no signal — start RX", false); }
    let low:  u64 = hist[..8].iter().sum();
    let high: u64 = hist[24..].iter().sum();
    let low_pct  = low  * 100 / total;
    let high_pct = high * 100 / total;
    if high_pct > 10 {
        ("⬇ clipping — reduce gain", true)
    } else if low_pct > 90 {
        ("⬆ weak — increase LNA +8 dB", false)
    } else if low_pct > 70 {
        ("⬆ under-utilised — try +8 dB", false)
    } else {
        ("✓ gain staging OK", false)
    }
}

impl Panel for RfChainPanel {
    fn name(&self) -> &'static str { "rf_chain" }
    fn min_size(&self) -> (u16, u16) { (32, 10) }

    fn render(&self, f: &mut Frame, area: ratatui::layout::Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let border_color = if focused { theme.border_focused } else { theme.border_default };
        let block = Block::default()
            .title(" RF Chain ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let bb_bw = state.radio.bb_filter_hz;
        let total_gain = state.radio.lna_gain as i32
            + state.radio.vga_gain as i32
            + if state.radio.amp_enabled { 14 } else { 0 };

        let cpld_span = match state.system.cpld_ok {
            Some(true)  => Span::styled("OK",       Style::default().fg(theme.status_ok)),
            Some(false) => Span::styled("MISMATCH", Style::default().fg(theme.status_crit).add_modifier(Modifier::BOLD)),
            None        => Span::styled("n/a",      Style::default().fg(theme.label)),
        };

        let lbl  = Style::default().fg(theme.label);
        let val  = Style::default().fg(theme.value);
        let hi   = Style::default().fg(theme.value_hi);

        let (advice_text, is_warn) = gain_advice(&state.iq.iq_amplitude_hist);
        let advice_color = if is_warn { theme.status_crit } else { theme.status_ok };

        // ADC utilisation gauge: fraction of samples in mid-range bins (8–23)
        let total: u64 = state.iq.iq_amplitude_hist.iter().sum();
        let mid: u64   = state.iq.iq_amplitude_hist[8..24].iter().sum();
        let util_ratio = if total > 0 { mid as f64 / total as f64 } else { 0.0 };
        let util_color = if util_ratio > 0.5 { theme.status_ok }
            else if util_ratio > 0.2         { theme.status_warn }
            else                             { theme.status_crit };

        let info_rows: &[Line] = &[
            Line::from(vec![
                Span::styled(format!("{:<13}", "BB filter"),  lbl),
                Span::styled(fmt_hz(bb_bw), val),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<13}", "Total gain"), lbl),
                Span::styled(format!("{} dB", total_gain), hi),
            ]),
            Line::from(vec![Span::raw("")]),
            Line::from(vec![
                Span::styled(format!("{:<13}", "Board"),   lbl),
                Span::styled(Device::board_rev_name(state.system.board_rev), val),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<13}", "USB API"), lbl),
                Span::styled(format!("{:#06x}", state.system.usb_api_version), val),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<13}", "CPLD"),    lbl),
                cpld_span,
            ]),
            Line::from(vec![Span::raw("")]),
            Line::from(vec![
                Span::styled(advice_text, Style::default().fg(advice_color)),
            ]),
        ];

        // Reserve 1 row at the bottom for the ADC utilisation ▐ bar
        let n_info = info_rows.len().min(inner.height.saturating_sub(1) as usize);
        if inner.height < 2 { return; }

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(n_info as u16),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(inner);

        let row_constraints: Vec<Constraint> = (0..n_info).map(|_| Constraint::Length(1)).collect();
        let row_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(row_constraints)
            .split(sections[0]);
        for (i, line) in info_rows.iter().take(n_info).enumerate() {
            f.render_widget(Paragraph::new(line.clone()), row_areas[i]);
        }

        crate::ui::charts::draw_hbar(
            f, sections[2], util_ratio,
            "ADC util ",
            &format!("{:.0}%", util_ratio * 100.0),
            util_color, theme,
        );
    }
}
