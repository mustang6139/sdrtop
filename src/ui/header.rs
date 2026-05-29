use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::state::SdrMetrics;
use super::panel::Panel;

pub struct HeaderPanel;

/// Returns (filled_chars, empty_chars). Both strings use block elements;
/// their char counts always sum to `width`.
fn gain_bar(gain: u32, max_gain: u32, width: usize) -> (String, String) {
    let filled = ((gain as f32 / max_gain as f32) * width as f32).round() as usize;
    let filled = filled.min(width);
    ("█".repeat(filled), "░".repeat(width - filled))
}

/// Returns the number of space characters needed between the fw-version field
/// and the right-aligned "AMP … USB …" section in the top band.
/// All length arguments are in terminal columns (chars, not bytes).
fn top_band_gap(board_name_len: usize, badge_len: usize, fw_value_len: usize, inner_width: u16) -> usize {
    // left side: " " + " DeviceName " + "  " + " BADGE " + "  " + "fw " + fw_val
    let left = 1 + (2 + board_name_len) + 2 + badge_len + 2 + 3 + fw_value_len;
    // right side: "AMP XXX  ·  USB XXXXXXXXX  " = 4+3+5+4+9+2 = 27 chars
    let right = 27usize;
    (inner_width as usize).saturating_sub(left + right)
}

fn top_band_line(state: &SdrMetrics, theme: &crate::Theme, inner_width: u16) -> Line<'static> {
    use ratatui::style::Color;

    // --- Status badge ---
    let (badge_text, badge_bg, badge_fg): (&str, Color, Color) = if state.observer_mode {
        (" ◈ OBSERVER ", theme.observer, Color::Rgb(4, 6, 15))
    } else if state.hw_streaming {
        (" ● RX ", theme.status_ok, Color::Rgb(3, 15, 6))
    } else {
        (" ○ IDLE ", theme.status_warn, Color::Rgb(10, 7, 0))
    };
    let badge_len = badge_text.chars().count();

    // --- Firmware version ---
    let fw_val: String = if state.observer_mode {
        "—".to_string()
    } else {
        state.fw_version.clone()
    };
    let fw_color = if state.observer_mode { theme.label } else { theme.value };
    let fw_len = fw_val.chars().count();

    // --- AMP value (always 3 terminal columns) ---
    let (amp_val, amp_color) = if state.observer_mode {
        ("—  ".to_string(), theme.label)
    } else if state.amp_enabled {
        ("ON ".to_string(), theme.value_hi)
    } else {
        ("OFF".to_string(), theme.label)
    };

    // --- USB value (always 9 terminal columns) ---
    let (usb_val, usb_color) = if state.hw_streaming && state.current_throughput_bps > 0 {
        let mb = state.current_throughput_bps as f64 / 1_000_000.0;
        (format!("{:4.1} MB/s", mb), theme.value)
    } else {
        ("—        ".to_string(), theme.label)  // 1 + 8 spaces = 9 chars
    };

    // --- Gap ---
    let board_len = state.board_name.chars().count();
    let gap = top_band_gap(board_len, badge_len, fw_len, inner_width);

    Line::from(vec![
        Span::raw(" "),
        Span::styled(
            format!(" {} ", state.board_name),
            Style::default()
                .fg(theme.value_hi)
                .bg(Color::Rgb(20, 25, 38))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            badge_text.to_string(),
            Style::default().fg(badge_fg).bg(badge_bg).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("fw ", Style::default().fg(theme.label)),
        Span::styled(fw_val, Style::default().fg(fw_color)),
        Span::raw(" ".repeat(gap)),
        Span::styled("AMP ", Style::default().fg(theme.label)),
        Span::styled(amp_val, Style::default().fg(amp_color)),
        Span::raw("  ·  "),
        Span::styled("USB ", Style::default().fg(theme.label)),
        Span::styled(usb_val, Style::default().fg(usb_color)),
        Span::raw("  "),
    ])
}

/// Builds the ├──── FREQUENCY ────┤ line.
/// `outer_width` is the FULL panel width (including border chars), not the inner width.
/// The returned Line must be rendered at the outer Rect so ├/┤ overwrite the │ border chars.
fn separator_line(theme: &crate::Theme, outer_width: u16) -> Line<'static> {
    let label = " FREQUENCY ";
    let label_len = label.len();  // 11 ASCII chars
    let fill = (outer_width as usize).saturating_sub(2 + label_len);  // 2 for ├ and ┤
    let left_fill = fill / 2;
    let right_fill = fill - left_fill;
    Line::from(vec![
        Span::styled("├", Style::default().fg(theme.border_dim)),
        Span::styled("─".repeat(left_fill), Style::default().fg(theme.border_default)),
        Span::styled(label, Style::default().fg(theme.border_dim)),
        Span::styled("─".repeat(right_fill), Style::default().fg(theme.border_default)),
        Span::styled("┤", Style::default().fg(theme.border_dim)),
    ])
}

/// Frequency · sample-rate · LNA bar · VGA bar.
/// Total char count is 78 for a standard 80-col terminal (inner_width=78).
/// Layout (chars): 3 + 8 + 1 + 3 + 4 + 3 + 4 + 5 + 4 + 4 + 8 + 1 + 2 + 3 + 4 + 4 + 8 + 1 + 2 + 3 + 2 = 78
fn bottom_band_line(state: &SdrMetrics, theme: &crate::Theme) -> Line<'static> {
    let active = state.hw_streaming && !state.observer_mode;

    // Frequency: right-padded to 8 chars (covers 0.000–9999.999 MHz)
    let freq_str = format!("{:8.3}", state.frequency as f64 / 1_000_000.0);
    // Sample rate: right-padded to 4 chars (HackRF range 2.0–20.0 Msps)
    let sr_str = format!("{:4.1}", state.config_sample_rate / 1_000_000.0);
    // Gain values: right-padded to 2 chars
    let lna_str = format!("{:2}", state.lna_gain);
    let vga_str = format!("{:2}", state.vga_gain);

    let (lna_filled, lna_empty) = gain_bar(state.lna_gain, 40, 8);
    let (vga_filled, vga_empty) = gain_bar(state.vga_gain, 62, 8);

    let freq_color = if state.observer_mode { theme.label } else { theme.border_accent };
    let val_color  = if active { theme.value } else { theme.label };
    let lna_color  = if active { theme.status_ok } else { theme.label };
    let vga_color  = if active { theme.status_warn } else { theme.label };
    let dim        = theme.border_dim;

    Line::from(vec![
        Span::raw("   "),
        Span::styled(freq_str, Style::default().fg(freq_color).add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::styled("MHz", Style::default().fg(theme.label)),
        Span::raw("    "),
        Span::styled("SR ", Style::default().fg(theme.label)),
        Span::styled(sr_str, Style::default().fg(val_color)),
        Span::styled(" Msps", Style::default().fg(theme.label)),
        Span::raw("    "),
        Span::styled("LNA ", Style::default().fg(theme.label)),
        Span::styled(lna_filled, Style::default().fg(lna_color)),
        Span::styled(lna_empty, Style::default().fg(dim)),
        Span::raw(" "),
        Span::styled(lna_str, Style::default().fg(val_color)),
        Span::styled(" dB", Style::default().fg(theme.label)),
        Span::raw("    "),
        Span::styled("VGA ", Style::default().fg(theme.label)),
        Span::styled(vga_filled, Style::default().fg(vga_color)),
        Span::styled(vga_empty, Style::default().fg(dim)),
        Span::raw(" "),
        Span::styled(vga_str, Style::default().fg(val_color)),
        Span::styled(" dB", Style::default().fg(theme.label)),
        Span::raw("  "),
    ])
}

impl Panel for HeaderPanel {
    fn name(&self) -> &'static str { "header" }
    fn min_size(&self) -> (u16, u16) { (60, 5) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, _focused: bool) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_dim));
        let inner = block.inner(area);
        f.render_widget(block, area);

        // inner.height == 3 when area.height == 5
        // Row positions (absolute y):
        //   inner.y     → top band
        //   inner.y + 1 → separator (rendered at outer width to overwrite the │ border chars)
        //   inner.y + 2 → bottom band

        let top_area = Rect { x: inner.x, y: inner.y,     width: inner.width, height: 1 };
        let sep_area = Rect { x: area.x,  y: inner.y + 1, width: area.width,  height: 1 };
        let bot_area = Rect { x: inner.x, y: inner.y + 2, width: inner.width, height: 1 };

        f.render_widget(Paragraph::new(top_band_line(state, theme, inner.width)), top_area);
        f.render_widget(Paragraph::new(separator_line(theme, area.width)), sep_area);
        f.render_widget(Paragraph::new(bottom_band_line(state, theme)), bot_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gain_bar_zero_gain_all_empty() {
        let (filled, empty) = gain_bar(0, 40, 8);
        assert_eq!(filled, "");
        assert_eq!(empty, "░░░░░░░░");
    }

    #[test]
    fn gain_bar_full_gain_all_filled() {
        let (filled, empty) = gain_bar(40, 40, 8);
        assert_eq!(filled, "████████");
        assert_eq!(empty, "");
    }

    #[test]
    fn gain_bar_half_gain() {
        let (filled, empty) = gain_bar(20, 40, 8);
        // 20/40 = 50% → 4 chars
        assert_eq!(filled.chars().count(), 4);
        assert_eq!(empty.chars().count(), 4);
    }

    #[test]
    fn gain_bar_total_always_equals_width() {
        for gain in [0u32, 1, 16, 20, 40] {
            let (f, e) = gain_bar(gain, 40, 8);
            assert_eq!(f.chars().count() + e.chars().count(), 8,
                "gain={gain}: filled({}) + empty({}) != 8", f.chars().count(), e.chars().count());
        }
    }

    #[test]
    fn top_band_gap_rx_state() {
        // HackRF One (len=10), badge " ● RX " (len=6), fw "2024.02.1" (len=9), inner=78
        assert_eq!(top_band_gap(10, 6, 9, 78), 16);
    }

    #[test]
    fn top_band_gap_idle_state() {
        // badge " ○ IDLE " is 2 chars wider than RX → gap shrinks by 2
        assert_eq!(top_band_gap(10, 8, 9, 78), 14);
    }

    #[test]
    fn top_band_gap_observer_state() {
        // badge " ◈ OBSERVER " (len=12), fw "—" (len=1)
        assert_eq!(top_band_gap(10, 12, 1, 78), 18);
    }
}
