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

/// Linear blend of two colours along `t∈[0,1]`. Falls back to `a` for non-Rgb
/// inputs (the SDR theme is truecolor, so the Rgb arm is what actually runs).
fn lerp_color(a: Color, b: Color, t: f64) -> Color {
    let t = t.clamp(0.0, 1.0);
    match (a, b) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => Color::Rgb(
            (r1 as f64 + (r2 as f64 - r1 as f64) * t) as u8,
            (g1 as f64 + (g2 as f64 - g1 as f64) * t) as u8,
            (b1 as f64 + (b2 as f64 - b1 as f64) * t) as u8,
        ),
        _ => a,
    }
}

/// Continuous ⅛-block gain bar with a position-based gradient. Same fill maths as
/// [`eighth_block_bar`], but returns per-column [`Span`]s so the filled part shades
/// from `lo` (left) to `hi` (right); empty columns get `empty_col`. The colour of a
/// column is fixed by its position across the full `n`-wide bar — the fill just
/// reveals more of the gradient. Used by the command rail's GAIN section.
pub fn gain_bar_colored(val: u32, max_val: u32, n: usize,
                        lo: Color, hi: Color, empty_col: Color) -> Vec<Span<'static>> {
    if n == 0 { return Vec::new(); }
    if max_val == 0 {
        return vec![Span::styled(" ".repeat(n), Style::default().fg(empty_col))];
    }
    let total_eighths = ((val as f64 / max_val as f64) * n as f64 * 8.0).round() as usize;
    let full = total_eighths / 8;
    let rem  = total_eighths % 8;
    let denom = (n - 1).max(1) as f64;

    let mut spans = Vec::with_capacity(n);
    for x in 0..n {
        let (ch, filled) = if x < full {
            ('█', true)
        } else if x == full && rem > 0 {
            (EIGHTHS[rem], true)
        } else {
            (' ', false)
        };
        let col = if filled { lerp_color(lo, hi, x as f64 / denom) } else { empty_col };
        spans.push(Span::styled(ch.to_string(), Style::default().fg(col)));
    }
    spans
}

/// Bipolar "null meter": a centre-zero track where a coloured needle (`●`) deviates
/// from the centre tick (`┃`) by `value / full_scale`, with the span between centre
/// and needle filled (`▓`). `width` is the track width; the returned spans add `◄`/`►`
/// end arrows, so the total width is `width + 2`. The needle and fill carry `color`
/// (the caller's severity colour); the track, centre and arrows are `dim`. Ideal for
/// deviation-from-ideal readings (DC offset, IQ amplitude / phase imbalance).
pub fn null_meter(value: f64, full_scale: f64, width: usize,
                  color: Color, dim: Color) -> Vec<Span<'static>> {
    let w = width.max(3);
    let center = w / 2;
    let frac = if full_scale > 0.0 { (value / full_scale).clamp(-1.0, 1.0) } else { 0.0 };
    let needle = ((center as f64 + frac * center as f64).round() as isize)
        .clamp(0, w as isize - 1) as usize;
    let (lo, hi) = (center.min(needle), center.max(needle));

    let mut spans = Vec::with_capacity(w + 2);
    spans.push(Span::styled("◄".to_string(), Style::default().fg(dim)));
    for x in 0..w {
        let (ch, col) = if x == needle {
            ('●', color)
        } else if x == center {
            ('┃', dim)
        } else if x > lo && x < hi {
            ('▓', color) // filled deviation between centre and needle
        } else {
            ('·', dim)    // empty track
        };
        spans.push(Span::styled(ch.to_string(), Style::default().fg(col)));
    }
    spans.push(Span::styled("►".to_string(), Style::default().fg(dim)));
    spans
}

/// Single-row braille **line trace** — an oscilloscope-style connected curve (not a
/// filled area). Returns exactly `width` braille chars, each holding 2 time samples
/// (left/right dot columns) over 4 vertical levels, auto-scaled to the most recent
/// 2×width samples' min..max. Consecutive samples are joined by filling the dot rows
/// between their levels, so the trace reads as a continuous line. Empty/flat input
/// renders as spaces. Used by the command rail's SIGNAL metric rows.
pub fn mini_braille_line(data: &[f32], width: usize) -> String {
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

    // Per-sample level: 0 (bottom) .. 3 (top) within one braille cell; -1 = no data.
    let lvl_at = |i: usize| -> i32 {
        match window.get(i) {
            Some(v) if v.is_finite() => (((v - lo) / range) * 3.0).round().clamp(0.0, 3.0) as i32,
            _ => -1,
        }
    };
    // Dot bits per column side, indexed by level 0 (bottom) .. 3 (top).
    const LEFT:  [u8; 4] = [0x40, 0x04, 0x02, 0x01]; // dots 7,3,2,1
    const RIGHT: [u8; 4] = [0x80, 0x20, 0x10, 0x08]; // dots 8,6,5,4

    let mut s = String::with_capacity(width * 3);
    for col in 0..width {
        let li = col * 2;
        let ri = li + 1;
        let mut bits = 0u8;
        // Left column: light the level, plus the rows up to the previous sample so
        // the segment between them is connected.
        let ll = lvl_at(li);
        if ll >= 0 {
            let prev = if li > 0 { lvl_at(li - 1).max(0) } else { ll };
            for l in ll.min(prev)..=ll.max(prev) { bits |= LEFT[l as usize]; }
        }
        // Right column: connect to the left sample of this same cell.
        let rl = lvl_at(ri);
        if rl >= 0 {
            let prev = lvl_at(li).max(0); // li is finite whenever ri is
            for l in rl.min(prev)..=rl.max(prev) { bits |= RIGHT[l as usize]; }
        }
        s.push(char::from_u32(0x2800 + bits as u32).unwrap_or(' '));
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
    fn mini_braille_line_always_width_chars() {
        assert_eq!(mini_braille_line(&[], 6).chars().count(), 6);
        assert_eq!(mini_braille_line(&[1.0; 20], 6).chars().count(), 6);
        assert_eq!(mini_braille_line(&[1.0; 3], 0).chars().count(), 0);
    }

    #[test]
    fn mini_braille_line_empty_or_flat_returns_spaces() {
        assert!(mini_braille_line(&[], 4).chars().all(|c| c == ' '));
        // A flat series has no range → spaces (nothing to plot).
        assert!(mini_braille_line(&[-50.0; 8], 4).chars().all(|c| c == ' '));
    }

    #[test]
    fn mini_braille_line_rising_ramp_is_braille() {
        let data: Vec<f32> = (0..=16).map(|i| i as f32).collect();
        for c in mini_braille_line(&data, 4).chars() {
            assert!(c as u32 >= 0x2800 && c as u32 <= 0x28FF, "non-braille char: {c:?}");
        }
    }

    #[test]
    fn mini_braille_line_sparse_history_fills_from_left() {
        // Only 4 samples for a width-6 (12-sample) trace: data on the left, the
        // empty right tail has no dots (blank braille U+2800).
        let s: Vec<char> = mini_braille_line(&[0.0, 1.0, 2.0, 3.0], 6).chars().collect();
        assert!(s[0] as u32 > 0x2800, "left end should carry dots, got {:?}", s[0]);
        assert_eq!(s[5] as u32, 0x2800, "right end has no data yet");
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

    #[test]
    fn lerp_color_endpoints_exact() {
        let a = Color::Rgb(0, 0, 0);
        let b = Color::Rgb(200, 100, 50);
        assert_eq!(lerp_color(a, b, 0.0), a);
        assert_eq!(lerp_color(a, b, 1.0), b);
        // Clamps out-of-range t to the endpoints.
        assert_eq!(lerp_color(a, b, -1.0), a);
        assert_eq!(lerp_color(a, b, 2.0), b);
    }

    #[test]
    fn lerp_color_midpoint_blends() {
        let mid = lerp_color(Color::Rgb(0, 0, 0), Color::Rgb(200, 100, 40), 0.5);
        assert_eq!(mid, Color::Rgb(100, 50, 20));
    }

    #[test]
    fn gain_bar_colored_total_width_is_n() {
        for val in [0u32, 8, 16, 24, 40] {
            let spans = gain_bar_colored(val, 40, 10,
                Color::Rgb(0, 200, 0), Color::Rgb(200, 200, 0), Color::Rgb(20, 20, 20));
            let total: usize = spans.iter().map(|s| s.content.chars().count()).sum();
            assert_eq!(total, 10, "val={val}");
        }
    }

    #[test]
    fn gain_bar_colored_zero_val_all_empty() {
        let spans = gain_bar_colored(0, 40, 6,
            Color::Rgb(0, 200, 0), Color::Rgb(200, 200, 0), Color::Rgb(20, 20, 20));
        let s: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(s.chars().all(|c| c == ' '), "got {s:?}");
    }

    #[test]
    fn null_meter_total_width_is_width_plus_arrows() {
        let m = null_meter(0.0, 1.0, 16, Color::Rgb(0, 200, 0), Color::Rgb(20, 20, 20));
        let total: usize = m.iter().map(|s| s.content.chars().count()).sum();
        assert_eq!(total, 18, "16 track + 2 arrows");
    }

    #[test]
    fn null_meter_needle_centres_at_zero() {
        let m = null_meter(0.0, 1.0, 17, Color::Rgb(0, 200, 0), Color::Rgb(20, 20, 20));
        let s: String = m.iter().map(|s| s.content.as_ref()).collect();
        let needle = s.chars().position(|c| c == '●').unwrap();
        // 17-wide track, centre = 8, plus the leading ◄ → index 9.
        assert_eq!(needle, 9);
    }

    #[test]
    fn null_meter_needle_deflects_with_sign() {
        let dim = Color::Rgb(20, 20, 20);
        let col = Color::Rgb(0, 200, 0);
        let pos: String = null_meter(0.8, 1.0, 17, col, dim).iter().map(|s| s.content.as_ref()).collect();
        let neg: String = null_meter(-0.8, 1.0, 17, col, dim).iter().map(|s| s.content.as_ref()).collect();
        let center = 9; // includes ◄
        assert!(pos.chars().position(|c| c == '●').unwrap() > center, "positive deflects right");
        assert!(neg.chars().position(|c| c == '●').unwrap() < center, "negative deflects left");
    }

    #[test]
    fn null_meter_clamps_out_of_range() {
        // |value| > full_scale must not panic and stays within the track.
        let m = null_meter(99.0, 1.0, 16, Color::Rgb(0, 200, 0), Color::Rgb(20, 20, 20));
        let total: usize = m.iter().map(|s| s.content.chars().count()).sum();
        assert_eq!(total, 18);
    }

    #[test]
    fn gain_bar_colored_full_val_all_filled() {
        let spans = gain_bar_colored(40, 40, 6,
            Color::Rgb(0, 200, 0), Color::Rgb(200, 200, 0), Color::Rgb(20, 20, 20));
        let s: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(s, "██████");
        // Left end is `lo`, right end is `hi`.
        assert_eq!(spans.first().unwrap().style.fg, Some(Color::Rgb(0, 200, 0)));
        assert_eq!(spans.last().unwrap().style.fg, Some(Color::Rgb(200, 200, 0)));
    }
}
