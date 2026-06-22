use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Line as CanvasLine},
        Paragraph,
    },
    Frame,
};

use crate::state::THROUGHPUT_HISTORY_LEN;

/// Full-block horizontal bar — same visual language as the header's LNA/VGA gain bars.
/// Renders into a single terminal row: `label ████░░░░ value_str`
pub fn draw_hbar(
    f: &mut Frame,
    area: Rect,
    ratio: f64,
    label: &str,
    value_str: &str,
    color: Color,
    theme: &crate::Theme,
) {
    let ratio   = ratio.clamp(0.0, 1.0);
    let label_w = label.chars().count() as u16;
    let val_w   = (value_str.chars().count() + 1) as u16; // +1 space separator
    let bar_w   = area.width.saturating_sub(label_w + val_w) as usize;
    let filled  = (ratio * bar_w as f64).round() as usize;
    let empty   = bar_w.saturating_sub(filled);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(label.to_string(),     Style::default().fg(theme.label)),
            Span::styled("█".repeat(filled),    Style::default().fg(color)),
            Span::styled("░".repeat(empty),     Style::default().fg(theme.border_dim)),
            Span::raw(" "),
            Span::styled(value_str.to_string(), Style::default().fg(color)),
        ])),
        area,
    );
}

/// A one-line inline sparkline from the most recent `width` samples, drawn with
/// the `▁▂▃▄▅▆▇█` ramp and auto-scaled to the window's own min..max. Returns a
/// string of exactly `width` columns (left-padded with spaces when there are
/// fewer samples than `width`). Non-finite samples render as a gap. Used by the
/// command rail's metric rows.
pub fn sparkline(data: &[f32], width: usize) -> String {
    const RAMP: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    if width == 0 { return String::new(); }
    if data.is_empty() { return " ".repeat(width); }

    let start  = data.len().saturating_sub(width);
    let window = &data[start..];
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    for &v in window {
        if v.is_finite() { lo = lo.min(v); hi = hi.max(v); }
    }
    if !lo.is_finite() || !hi.is_finite() { return " ".repeat(width); }
    let range = (hi - lo).max(1e-6);

    let mut s = String::with_capacity(width);
    for _ in 0..width.saturating_sub(window.len()) { s.push(' '); }
    for &v in window {
        if !v.is_finite() { s.push(' '); continue; }
        let t = (((v - lo) / range) * 7.0).round().clamp(0.0, 7.0) as usize;
        s.push(RAMP[t]);
    }
    s
}

/// EMA smoothing — alpha near 1 = responsive, near 0 = smooth. Empty input → empty vec.
pub fn ema_smooth(data: &[f32], alpha: f32) -> Vec<f32> {
    if data.is_empty() { return Vec::new(); }
    let mut out = Vec::with_capacity(data.len());
    let mut s = data[0];
    out.push(s);
    for &v in &data[1..] {
        s = alpha * v + (1.0 - alpha) * s;
        out.push(s);
    }
    out
}

pub(crate) const EIGHTHS: [char; 9] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];

/// Continuous horizontal ⅛-block bar: `val/max_val` fill ratio across `n` columns.
/// Returns `(filled_part, empty_part)` — drop-in replacement for the legacy `▮▯` gain bar.
pub fn eighth_block_bar(val: u32, max_val: u32, n: usize) -> (String, String) {
    if n == 0 || max_val == 0 { return (String::new(), " ".repeat(n)); }
    let total_eighths = ((val as f64 / max_val as f64) * n as f64 * 8.0).round() as usize;
    let full = total_eighths / 8;
    let rem  = total_eighths % 8;
    let mut filled = "█".repeat(full.min(n));
    if full < n && rem > 0 { filled.push(EIGHTHS[rem]); }
    let empty_start = if rem > 0 { full + 1 } else { full };
    let empty = " ".repeat(n.saturating_sub(empty_start));
    (filled, empty)
}

/// 2-row braille filled-area mini-scope. Returns `[top_row, bot_row]`, each exactly `width`
/// Unicode characters wide. Renders from the most recent 2×width samples, auto-scaled.
pub fn mini_braille_scope(data: &[f32], width: usize) -> [String; 2] {
    if width == 0 { return [String::new(), String::new()]; }

    let n_samples = width * 2;
    let start = data.len().saturating_sub(n_samples);
    let window = &data[start..];

    let (lo, hi) = window.iter().copied()
        .filter(|v| v.is_finite())
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), v| (lo.min(v), hi.max(v)));
    let range = if lo.is_finite() && hi.is_finite() && (hi - lo) > 1e-6 {
        hi - lo
    } else {
        return [" ".repeat(width), " ".repeat(width)];
    };

    let level = |v: f32| -> usize {
        if !v.is_finite() { return 0; }
        ((v - lo) / range * 8.0).round().clamp(0.0, 8.0) as usize
    };

    // Returns (top_bits, bot_bits) for the left dot-column of a braille cell.
    // Fills upward from dot 7 (bottom-left) → dot 1 (top-left) for levels 1–4,
    // then the same pattern in the top text row for levels 5–8.
    let left_bits = |lv: usize| -> (u8, u8) {
        let (mut t, mut b) = (0u8, 0u8);
        if lv >= 1 { b |= 0x40; } // dot 7
        if lv >= 2 { b |= 0x04; } // dot 3
        if lv >= 3 { b |= 0x02; } // dot 2
        if lv >= 4 { b |= 0x01; } // dot 1
        if lv >= 5 { t |= 0x40; }
        if lv >= 6 { t |= 0x04; }
        if lv >= 7 { t |= 0x02; }
        if lv >= 8 { t |= 0x01; }
        (t, b)
    };
    let right_bits = |lv: usize| -> (u8, u8) {
        let (mut t, mut b) = (0u8, 0u8);
        if lv >= 1 { b |= 0x80; } // dot 8
        if lv >= 2 { b |= 0x20; } // dot 6
        if lv >= 3 { b |= 0x10; } // dot 5
        if lv >= 4 { b |= 0x08; } // dot 4
        if lv >= 5 { t |= 0x80; }
        if lv >= 6 { t |= 0x20; }
        if lv >= 7 { t |= 0x10; }
        if lv >= 8 { t |= 0x08; }
        (t, b)
    };

    let mut top = String::with_capacity(width * 3);
    let mut bot = String::with_capacity(width * 3);

    for col in 0..width {
        let li = col * 2;
        let ri = li + 1;
        let lv = if li < window.len() { level(window[li]) } else { 0 };
        let rv = if ri < window.len() { level(window[ri]) } else { 0 };
        let (lt, lb) = left_bits(lv);
        let (rt, rb) = right_bits(rv);
        top.push(char::from_u32(0x2800 + (lt | rt) as u32).unwrap_or(' '));
        bot.push(char::from_u32(0x2800 + (lb | rb) as u32).unwrap_or(' '));
    }

    [top, bot]
}

/// Single-row braille filled-area mini-scope. Returns exactly `width` braille
/// chars, each encoding 2 time samples (left/right dot columns) at 0..4 vertical
/// levels, auto-scaled to the most recent 2×width samples' min..max. Used by the
/// command rail's framed metric boxes (one braille row inside a `┌─┐` border).
pub fn mini_braille_row(data: &[f32], width: usize) -> String {
    if width == 0 { return String::new(); }

    let n_samples = width * 2;
    let start = data.len().saturating_sub(n_samples);
    let window = &data[start..];

    let (lo, hi) = window.iter().copied()
        .filter(|v| v.is_finite())
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), v| (lo.min(v), hi.max(v)));
    let range = if lo.is_finite() && hi.is_finite() && (hi - lo) > 1e-6 {
        hi - lo
    } else {
        return " ".repeat(width);
    };

    let level = |v: f32| -> usize {
        if !v.is_finite() { return 0; }
        ((v - lo) / range * 4.0).round().clamp(0.0, 4.0) as usize
    };
    // Fill upward within a single braille cell: left column dots 7,3,2,1; right
    // column dots 8,6,5,4 (bottom→top).
    let left_bits = |lv: usize| -> u8 {
        let mut b = 0u8;
        if lv >= 1 { b |= 0x40; } // dot 7
        if lv >= 2 { b |= 0x04; } // dot 3
        if lv >= 3 { b |= 0x02; } // dot 2
        if lv >= 4 { b |= 0x01; } // dot 1
        b
    };
    let right_bits = |lv: usize| -> u8 {
        let mut b = 0u8;
        if lv >= 1 { b |= 0x80; } // dot 8
        if lv >= 2 { b |= 0x20; } // dot 6
        if lv >= 3 { b |= 0x10; } // dot 5
        if lv >= 4 { b |= 0x08; } // dot 4
        b
    };

    let mut s = String::with_capacity(width * 3);
    for col in 0..width {
        let li = col * 2;
        let ri = li + 1;
        let lv = if li < window.len() { level(window[li]) } else { 0 };
        let rv = if ri < window.len() { level(window[ri]) } else { 0 };
        s.push(char::from_u32(0x2800 + (left_bits(lv) | right_bits(rv)) as u32).unwrap_or(' '));
    }
    s
}

/// Canvas filled-column graph — same style as the spectrum panel (filled columns + outline).
/// Accepts a plain `&[u64]` slice. Scales automatically to the data maximum.
pub fn draw_mini_graph(f: &mut Frame, area: Rect, data: &[u64], color: Color) {
    if area.height == 0 || area.width < 2 || data.is_empty() { return; }

    let values: Vec<f64> = data.iter().map(|&v| v as f64).collect();
    let n       = values.len();
    let max_val = values.iter().cloned().fold(0.0_f64, f64::max).max(1.0);
    let max_n   = THROUGHPUT_HISTORY_LEN as f64;
    let x_off   = max_n - n as f64;

    f.render_widget(
        Canvas::default()
            .x_bounds([0.0, max_n])
            .y_bounds([0.0, max_val])
            .paint(move |ctx| {
                // Filled columns
                for (i, &val) in values.iter().enumerate() {
                    let x = x_off + i as f64;
                    ctx.draw(&CanvasLine { x1: x, y1: 0.0, x2: x, y2: val, color });
                }
                // Outline connecting column tops
                for i in 1..n {
                    ctx.draw(&CanvasLine {
                        x1: x_off + (i - 1) as f64, y1: values[i - 1],
                        x2: x_off +  i      as f64, y2: values[i],
                        color,
                    });
                }
            }),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // Every sparkline char is single-column, so chars().count() == display width.
    #[test]
    fn sparkline_is_always_width_columns() {
        assert_eq!(sparkline(&[], 6).chars().count(), 6);
        assert_eq!(sparkline(&[1.0], 6).chars().count(), 6);
        assert_eq!(sparkline(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], 6).chars().count(), 6);
        assert_eq!(sparkline(&[1.0; 3], 0).chars().count(), 0);
    }

    #[test]
    fn sparkline_maps_extremes_to_ramp_ends() {
        // A rising ramp ends low→high: first non-space is ▁, last is █.
        let s: Vec<char> = sparkline(&[0.0, 1.0, 2.0, 3.0], 4).chars().collect();
        assert_eq!(s[0], '▁');
        assert_eq!(s[3], '█');
    }

    #[test]
    fn sparkline_flat_series_does_not_panic_or_spike() {
        // Equal samples → range floored to 1e-6, all map to the lowest bar.
        let s = sparkline(&[-76.3; 6], 6);
        assert!(s.chars().all(|c| c == '▁'));
    }

    #[test]
    fn sparkline_left_pads_when_too_few_samples() {
        let s = sparkline(&[5.0, 9.0], 6);
        assert!(s.starts_with("    "), "got {s:?}");
    }

    #[test]
    fn ema_smooth_single_sample_is_itself() {
        assert_eq!(ema_smooth(&[5.0f32], 0.5), vec![5.0f32]);
    }

    #[test]
    fn ema_smooth_alpha_one_is_passthrough() {
        let data = vec![1.0f32, 3.0, 2.0, 5.0];
        assert_eq!(ema_smooth(&data, 1.0), data);
    }

    #[test]
    fn ema_smooth_empty_returns_empty() {
        assert!(ema_smooth(&[], 0.5).is_empty());
    }

    #[test]
    fn mini_braille_scope_always_width_chars() {
        let [t, b] = mini_braille_scope(&[], 6);
        assert_eq!(t.chars().count(), 6);
        assert_eq!(b.chars().count(), 6);
        let [t, b] = mini_braille_scope(&[1.0; 20], 6);
        assert_eq!(t.chars().count(), 6);
        assert_eq!(b.chars().count(), 6);
        let [t, b] = mini_braille_scope(&[1.0; 3], 0);
        assert_eq!(t.chars().count(), 0);
        assert_eq!(b.chars().count(), 0);
    }

    #[test]
    fn mini_braille_scope_empty_data_returns_spaces() {
        let [t, b] = mini_braille_scope(&[], 4);
        assert!(t.chars().all(|c| c == ' '), "top: {t:?}");
        assert!(b.chars().all(|c| c == ' '), "bot: {b:?}");
    }

    #[test]
    fn mini_braille_scope_rising_ramp_fills_upward() {
        // Ramp 0→8: all chars must be valid braille codepoints (U+2800..=U+28FF).
        let data: Vec<f32> = (0..=16).map(|i| i as f32).collect();
        let [t, b] = mini_braille_scope(&data, 4);
        for c in t.chars().chain(b.chars()) {
            assert!(c as u32 >= 0x2800 && c as u32 <= 0x28FF, "non-braille char: {c:?}");
        }
    }

    #[test]
    fn mini_braille_row_always_width_chars() {
        assert_eq!(mini_braille_row(&[], 6).chars().count(), 6);
        assert_eq!(mini_braille_row(&[1.0; 20], 6).chars().count(), 6);
        assert_eq!(mini_braille_row(&[1.0; 3], 0).chars().count(), 0);
    }

    #[test]
    fn mini_braille_row_empty_data_returns_spaces() {
        assert!(mini_braille_row(&[], 4).chars().all(|c| c == ' '));
        // A flat series has no range → spaces (nothing to plot).
        assert!(mini_braille_row(&[-50.0; 8], 4).chars().all(|c| c == ' '));
    }

    #[test]
    fn mini_braille_row_rising_ramp_is_braille() {
        let data: Vec<f32> = (0..=16).map(|i| i as f32).collect();
        for c in mini_braille_row(&data, 4).chars() {
            assert!(c as u32 >= 0x2800 && c as u32 <= 0x28FF, "non-braille char: {c:?}");
        }
    }

    #[test]
    fn eighth_block_bar_zero_val_all_empty() {
        let (f, e) = eighth_block_bar(0, 100, 10);
        assert!(f.is_empty(), "filled should be empty for val=0");
        assert_eq!(e, " ".repeat(10));
    }

    #[test]
    fn eighth_block_bar_full_val_all_filled() {
        let (f, e) = eighth_block_bar(100, 100, 10);
        assert_eq!(f, "█".repeat(10));
        assert!(e.is_empty());
    }

    #[test]
    fn eighth_block_bar_total_width_is_n() {
        for val in [0u32, 25, 50, 75, 100] {
            let (f, e) = eighth_block_bar(val, 100, 8);
            let total = f.chars().count() + e.chars().count();
            assert_eq!(total, 8, "val={val}");
        }
    }

    #[test]
    fn eighth_block_bar_fractional_end_uses_eighth_char() {
        // val=1, max=16, n=8: total_eighths = round(0.0625 * 64) = 4 → '▌'
        let (f, e) = eighth_block_bar(1, 16, 8);
        assert!(f.contains('▌'), "expected '▌', got {f:?}");
        assert_eq!(f.chars().count() + e.chars().count(), 8);
    }
}
