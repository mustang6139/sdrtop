//! Braille-based spectrum renderer.
//!
//! Each terminal cell covers a 2×4 braille dot grid (Unicode 8-dot braille,
//! U+2800–U+28FF), giving 2× horizontal and 4× vertical resolution compared to
//! plain block characters.
//!
//! Rendering model:
//!   - **Fill cells** (signal fully clears the cell): solid background block.
//!   - **Trace cells** (signal edge inside the cell): braille character whose
//!     dots mark exactly where the signal is, colored by signal strength.
//!   - Everything above: empty.
//!
//! The braille dot layout within one terminal cell (left col / right col):
//!
//!   Dot 1 (bit 0)  Dot 4 (bit 3)   ← top of cell
//!   Dot 2 (bit 1)  Dot 5 (bit 4)
//!   Dot 3 (bit 2)  Dot 6 (bit 5)
//!   Dot 7 (bit 6)  Dot 8 (bit 7)   ← bottom of cell
//!
//! Indexed by `dot_row_from_bottom` (0 = cell bottom, 3 = cell top):

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

// ── Downsampler ───────────────────────────────────────────────────────────────

/// Reduce `bins` to exactly `cols` values, taking the MAX in each bucket.
/// MAX preserves narrow peaks that MEAN would smear. Empty / zero cases → empty Vec.
pub fn downsample_max(bins: &[f32], cols: usize) -> Vec<f32> {
    if bins.is_empty() || cols == 0 {
        return Vec::new();
    }
    let n = bins.len();
    (0..cols)
        .map(|c| {
            let lo = c * n / cols;
            let hi = ((c + 1) * n / cols).max(lo + 1).min(n);
            bins[lo..hi].iter().copied().fold(f32::NEG_INFINITY, f32::max)
        })
        .collect()
}

// ── Public types ──────────────────────────────────────────────────────────────

/// One braille column's worth of signal data.
pub struct Bar {
    pub db:          f32,
    /// Color of the braille trace edge (foreground on black).  Should contrast
    /// well against the fill background — use a vivid / theme-accent-derived color.
    pub trace_color: Color,
}

/// Compute a trace color from the theme's accent, scaled by signal strength.
/// Weak signals get ~35 % brightness; strong signals get 100 %.  This keeps
/// the trace visible at the noise floor while still encoding signal level.
pub fn accent_trace_color(accent: Color, signal_fraction: f32) -> Color {
    match accent {
        Color::Rgb(r, g, b) => {
            let f = (0.35 + 0.65 * signal_fraction.clamp(0.0, 1.0)) as f32;
            Color::Rgb((r as f32 * f) as u8, (g as f32 * f) as u8, (b as f32 * f) as u8)
        }
        other => other,
    }
}

/// A vertical overlay line drawn at a **terminal** column index.
pub struct VLine {
    pub col:          u16,
    pub color:        Color,
    /// `true` → spans full height (cursor, always visible).
    /// `false` → only in the empty region above the signal (markers, BW edges).
    pub through_bars: bool,
}

// ── Braille bit constants ─────────────────────────────────────────────────────

/// Left-column bit mask indexed by dot_row_from_bottom (0 = bottom of cell).
const LEFT_BITS:  [u8; 4] = [0x40, 0x04, 0x02, 0x01];
/// Right-column bit mask indexed by dot_row_from_bottom.
const RIGHT_BITS: [u8; 4] = [0x80, 0x20, 0x10, 0x08];

// ── Private helpers ───────────────────────────────────────────────────────────

/// Map a dBFS value to effective dot rows from the bottom of the canvas.
/// Returns 0 at or below `y_min`, `total` at or above `y_max`.
fn to_dots(db: f32, y_min: f32, y_max: f32, total: u32) -> u32 {
    let f = ((db - y_min) / (y_max - y_min)).clamp(0.0, 1.0);
    (f * total as f32).round() as u32
}

/// Braille fill mask for a terminal cell whose bottom effective row is `base`.
/// A dot at effective row `base + dr` is lit if it lies below `dots_l` (left
/// braille column) or `dots_r` (right braille column).
fn cell_mask(dots_l: u32, dots_r: u32, base: u32) -> u8 {
    let mut m = 0u8;
    for dr in 0u32..4 {
        let eff = base + dr;
        if eff < dots_l { m |= LEFT_BITS[dr as usize]; }
        if eff < dots_r { m |= RIGHT_BITS[dr as usize]; }
    }
    m
}

/// Terminal rows from the bottom occupied by a signal at `db` (ceil of dot height / 4).
fn signal_floor_rows(db: f32, y_min: f32, y_max: f32, rows: u32) -> u32 {
    let dots = to_dots(db, y_min, y_max, rows * 4);
    (dots + 3) / 4
}

// ── Public renderers ──────────────────────────────────────────────────────────

/// Paint the spectrum as a braille canvas.
///
/// `bars` must have length `area.width as usize * 2` — two braille columns per
/// terminal column for 2× horizontal resolution.  `peak_db` and `hold_db` may
/// be empty or shorter than `bars`; missing slots are treated as `y_min`.
///
/// `fill_fn` receives a signal-level fraction (0.0 = bottom / y_min,
/// 1.0 = top / y_max) and returns the background color for that height.
/// Called at `0.0` for the canvas pre-fill and at the cell's level for fill
/// cells.  Use a dimmed palette gradient for best results.
pub fn paint_braille<F>(
    buf:         &mut Buffer,
    area:        Rect,
    bars:        &[Bar],
    peak_db:     &[f32],
    hold_db:     &[f32],
    noise_floor: f32,
    y_min:       f32,
    y_max:       f32,
    fill_fn:     F,
    peak_color:  Color,
    hold_color:  Color,
    noise_color: Color,
) where F: Fn(f32) -> Color {
    let rows = area.height as u32;
    let cols = area.width as u32;
    if rows == 0 || cols == 0 || y_max <= y_min { return; }
    let total_dots = rows * 4;

    // ── Canvas background ────────────────────────────────────────────────
    // Pre-fill with a very dark version of the palette cold end so the panel
    // looks like a dark SA display but the colored braille dots (set_fg) always
    // have visible contrast against the background.
    let canvas_bg = match fill_fn(0.0) {
        Color::Rgb(r, g, b) => Color::Rgb(r / 5, g / 5, b / 5),
        c => c,
    };
    for cy in 0..rows {
        for cx in 0..cols {
            buf.get_mut(area.x + cx as u16, area.y + cy as u16).set_bg(canvas_bg);
        }
    }

    // ── Live signal: braille interior + braille trace ─────────────────────
    // ALL signal cells use braille characters with fg=palette gradient.
    // Interior (fully-cleared) cells: ⣿ (all 8 dots) — no bg override, so the
    // braille dot texture stays visible rather than looking like a solid block.
    // Trace (edge) cells: partial braille with the accent trace color.
    for cy in 0..rows {
        let base = (rows - 1 - cy) * 4;   // effective dot row at BOTTOM of this cell
        let ty   = area.y + cy as u16;

        for cx in 0..cols {
            let ec_l = (cx * 2) as usize;
            let ec_r = (cx * 2 + 1) as usize;
            let tx   = area.x + cx as u16;

            let d_l = bars.get(ec_l).map(|b| to_dots(b.db, y_min, y_max, total_dots)).unwrap_or(0);
            let d_r = bars.get(ec_r).map(|b| to_dots(b.db, y_min, y_max, total_dots)).unwrap_or(0);

            let mask = cell_mask(d_l, d_r, base);
            if mask == 0 { continue; }

            let cell = buf.get_mut(tx, ty);

            if d_l > base + 3 && d_r > base + 3 {
                // Interior cell: signal completely fills this cell.
                // Render as ⣿ (all-8-dot braille) with palette gradient fg so the
                // braille dot texture remains visible — not a solid background block.
                let level_frac = (base as f32 + 2.0) / total_dots as f32;
                cell.set_symbol("⣿").set_fg(fill_fn(level_frac));
            } else {
                // Trace cell: signal edge is inside this cell.
                let color = if d_l > base && d_l <= base + 4 {
                    bars.get(ec_l).map(|b| b.trace_color).unwrap_or(canvas_bg)
                } else {
                    bars.get(ec_r).map(|b| b.trace_color).unwrap_or(canvas_bg)
                };
                let ch = char::from_u32(0x2800 | mask as u32).unwrap();
                cell.set_symbol(&ch.to_string()).set_fg(color);
            }
        }
    }

    // ── Peak hold (▔ cap at terminal-row resolution above live signal) ────
    for cx in 0..cols {
        let ec_l = (cx * 2) as usize;
        let ec_r = (cx * 2 + 1) as usize;
        let tx   = area.x + cx as u16;

        let p = peak_db.get(ec_l).copied().unwrap_or(y_min)
            .max(peak_db.get(ec_r).copied().unwrap_or(y_min));
        let s = bars.get(ec_l).map(|b| b.db).unwrap_or(y_min)
            .max(bars.get(ec_r).map(|b| b.db).unwrap_or(y_min));

        let peak_floor = signal_floor_rows(p, y_min, y_max, rows);
        let live_floor = signal_floor_rows(s, y_min, y_max, rows);

        if peak_floor > live_floor && peak_floor > 0 && peak_floor <= rows {
            buf.get_mut(tx, area.y + (rows - peak_floor) as u16)
                .set_symbol("▔").set_fg(peak_color);
        }
    }

    // ── Hold ghost (▔ cap where held frame is above live signal) ─────────
    for cx in 0..cols {
        let ec_l = (cx * 2) as usize;
        let ec_r = (cx * 2 + 1) as usize;
        let tx   = area.x + cx as u16;

        let h = hold_db.get(ec_l).copied().unwrap_or(y_min)
            .max(hold_db.get(ec_r).copied().unwrap_or(y_min));
        let s = bars.get(ec_l).map(|b| b.db).unwrap_or(y_min)
            .max(bars.get(ec_r).map(|b| b.db).unwrap_or(y_min));

        let hold_floor = signal_floor_rows(h, y_min, y_max, rows);
        let live_floor = signal_floor_rows(s, y_min, y_max, rows);

        if hold_floor > live_floor && hold_floor > 0 && hold_floor <= rows {
            buf.get_mut(tx, area.y + (rows - hold_floor) as u16)
                .set_symbol("▔").set_fg(hold_color);
        }
    }

    // ── Noise floor (─ dash reference line at the NF level) ─────────────
    // Drawn only in columns where the live signal is at or below the NF,
    // i.e. where the floor is actually the dominant "signal".  A dash looks
    // like a clean reference baseline rather than random noise scatter.
    if noise_floor > y_min {
        let nf_floor = signal_floor_rows(noise_floor, y_min, y_max, rows);
        if nf_floor > 0 && nf_floor <= rows {
            let ty = area.y + (rows - nf_floor) as u16;
            for cx in 0..cols {
                let ec_l = (cx * 2) as usize;
                let ec_r = (cx * 2 + 1) as usize;
                let db = bars.get(ec_l).map(|b| b.db).unwrap_or(y_min)
                    .max(bars.get(ec_r).map(|b| b.db).unwrap_or(y_min));
                if nf_floor >= signal_floor_rows(db, y_min, y_max, rows) {
                    buf.get_mut(area.x + cx as u16, ty)
                        .set_symbol("─").set_fg(noise_color);
                }
            }
        }
    }
}

/// Paint vertical overlay lines (markers, cursor) at **terminal**-column resolution.
///
/// `bars` is the same braille-resolution slice passed to `paint_braille`
/// (`len == area.width * 2`).  `vlines[i].col` is a terminal column index.
pub fn paint_vlines(
    buf:    &mut Buffer,
    area:   Rect,
    vlines: &[VLine],
    bars:   &[Bar],
    y_min:  f32,
    y_max:  f32,
) {
    let rows = area.height as u32;
    let cols = area.width as u32;
    if rows == 0 || cols == 0 { return; }

    for vl in vlines {
        let cx = vl.col as u32;
        if cx >= cols { continue; }

        let ec_l = (cx * 2) as usize;
        let ec_r = (cx * 2 + 1) as usize;
        let db = bars.get(ec_l).map(|b| b.db).unwrap_or(y_min)
            .max(bars.get(ec_r).map(|b| b.db).unwrap_or(y_min));

        let floor = signal_floor_rows(db, y_min, y_max, rows);
        let start = if vl.through_bars { 0 } else { floor };
        let tx = area.x + vl.col;

        for r in start..rows {
            buf.get_mut(tx, area.y + (rows - 1 - r) as u16)
                .set_symbol("│").set_fg(vl.color);
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── downsample_max ────────────────────────────────────────────────────

    #[test]
    fn downsample_identity_when_cols_equals_len() {
        let bins = [-10.0, -20.0, -30.0, -40.0];
        assert_eq!(downsample_max(&bins, 4), vec![-10.0, -20.0, -30.0, -40.0]);
    }

    #[test]
    fn downsample_takes_max_not_mean() {
        let bins = [-10.0, -90.0, -50.0, -40.0];
        assert_eq!(downsample_max(&bins, 2), vec![-10.0, -40.0]);
    }

    #[test]
    fn downsample_upsamples_when_cols_gt_len() {
        let bins = [-10.0, -50.0];
        assert_eq!(downsample_max(&bins, 4), vec![-10.0, -10.0, -50.0, -50.0]);
    }

    #[test]
    fn downsample_empty_inputs() {
        assert_eq!(downsample_max(&[], 8), Vec::<f32>::new());
        assert_eq!(downsample_max(&[-10.0, -20.0], 0), Vec::<f32>::new());
    }

    // ── to_dots ───────────────────────────────────────────────────────────

    #[test]
    fn to_dots_at_min_is_zero() {
        assert_eq!(to_dots(-90.0, -90.0, -20.0, 40), 0);
    }

    #[test]
    fn to_dots_at_max_is_total() {
        assert_eq!(to_dots(-20.0, -90.0, -20.0, 40), 40);
    }

    #[test]
    fn to_dots_clamps_out_of_range() {
        assert_eq!(to_dots(-200.0, -90.0, -20.0, 40), 0);
        assert_eq!(to_dots(0.0,    -90.0, -20.0, 40), 40);
    }

    // ── cell_mask ─────────────────────────────────────────────────────────

    #[test]
    fn cell_mask_empty_when_signal_below_base() {
        // Signal occupies 0 dots — nothing lit
        assert_eq!(cell_mask(0, 0, 0), 0);
        // Signal at effective row 3 but base = 4 → all dots of this cell are above signal
        assert_eq!(cell_mask(3, 3, 4), 0);
    }

    #[test]
    fn cell_mask_full_when_signal_clears_cell() {
        // Both columns' signal (12 dots) exceeds base (4) by more than 3 → all 8 bits set
        assert_eq!(cell_mask(12, 12, 4), 0xFF);
    }

    #[test]
    fn cell_mask_left_only_partial() {
        // Left column signal = 2 dots, right = 0; base = 0.
        // dot_row=0: eff=0 < 2 → LEFT_BITS[0]=0x40; dot_row=1: eff=1 < 2 → 0x04; dot_row=2,3: no
        assert_eq!(cell_mask(2, 0, 0), LEFT_BITS[0] | LEFT_BITS[1]);
    }

    #[test]
    fn cell_mask_right_only_single_dot() {
        // Right column signal = 1; base = 0.  Only dot_row=0 lit on right.
        assert_eq!(cell_mask(0, 1, 0), RIGHT_BITS[0]);
    }

    // ── signal_floor_rows ─────────────────────────────────────────────────

    #[test]
    fn signal_floor_rows_at_min_is_zero() {
        assert_eq!(signal_floor_rows(-90.0, -90.0, -20.0, 10), 0);
    }

    #[test]
    fn signal_floor_rows_at_max_is_rows() {
        assert_eq!(signal_floor_rows(-20.0, -90.0, -20.0, 10), 10);
    }

    #[test]
    fn signal_floor_rows_half_signal() {
        // frac=0.5 → dots=20 → floor=ceil(20/4)=5
        assert_eq!(signal_floor_rows(-55.0, -90.0, -20.0, 10), 5);
    }

    // ── paint_braille smoke tests ─────────────────────────────────────────

    #[test]
    fn paint_braille_empty_area_no_panic() {
        let mut buf = ratatui::buffer::Buffer::empty(Rect::new(0, 0, 0, 0));
        paint_braille(&mut buf, Rect::new(0,0,0,0), &[], &[], &[], -80.0, -90.0, -20.0,
            |_| Color::Black, Color::White, Color::Gray, Color::Blue);
    }

    #[test]
    fn paint_braille_max_signal_fills_canvas() {
        // Signal at max → every cell should be an interior braille cell (⣿ with fg color).
        let area = Rect::new(0, 0, 2, 2);
        let mut buf = ratatui::buffer::Buffer::empty(area);
        let bars: Vec<Bar> = vec![
            Bar { db: -20.0, trace_color: Color::Red },
            Bar { db: -20.0, trace_color: Color::Red },
            Bar { db: -20.0, trace_color: Color::Red },
            Bar { db: -20.0, trace_color: Color::Red },
        ];
        paint_braille(&mut buf, area, &bars, &[], &[], -200.0, -90.0, -20.0,
            |_| Color::DarkGray, Color::White, Color::Gray, Color::Blue);
        // All cells should be interior braille: ⣿ symbol with fg = fill_fn color
        for y in 0..2 {
            for x in 0..2 {
                assert_eq!(buf.get(x, y).symbol(), "⣿", "cell ({x},{y}) should be all-8-dot braille");
                assert_eq!(buf.get(x, y).fg, Color::DarkGray, "cell ({x},{y}) fg should be fill_fn color");
            }
        }
    }

    #[test]
    fn paint_braille_min_signal_leaves_canvas_empty() {
        // Signal at min → only the canvas pre-fill is drawn (dark bg, no braille symbols).
        let area = Rect::new(0, 0, 2, 2);
        let mut buf = ratatui::buffer::Buffer::empty(area);
        let bars: Vec<Bar> = vec![
            Bar { db: -90.0, trace_color: Color::Red },
            Bar { db: -90.0, trace_color: Color::Red },
            Bar { db: -90.0, trace_color: Color::Red },
            Bar { db: -90.0, trace_color: Color::Red },
        ];
        // fill_fn(0.0) = Rgb(50,50,50) → canvas_bg = Rgb(10,10,10)
        paint_braille(&mut buf, area, &bars, &[], &[], -200.0, -90.0, -20.0,
            |_| Color::Rgb(50, 50, 50), Color::White, Color::Gray, Color::Blue);
        let expected_bg = Color::Rgb(10, 10, 10);  // 50/5 = 10
        for y in 0..2 {
            for x in 0..2 {
                assert_eq!(buf.get(x, y).symbol(), " ");
                assert_eq!(buf.get(x, y).bg, expected_bg, "canvas pre-fill expected");
            }
        }
    }

    #[test]
    fn paint_braille_mid_signal_has_visible_cells() {
        // A signal at mid-range should produce visible fill or trace cells.
        // At -55 dBFS with -90..=-20 range: frac=0.5 → 8 dots → 2 fill rows.
        let area = Rect::new(0, 0, 1, 4);
        let mut buf = ratatui::buffer::Buffer::empty(area);
        let bars = vec![
            Bar { db: -55.0, trace_color: Color::Cyan },
            Bar { db: -55.0, trace_color: Color::Cyan },
        ];
        // Use a gradient fill that returns non-black for t > 0 so we can
        // distinguish fill cells from the canvas pre-fill (t=0 → black).
        paint_braille(&mut buf, area, &bars, &[], &[], -200.0, -90.0, -20.0,
            |t| Color::Rgb((t * 200.0) as u8, 0, 0), Color::White, Color::Gray, Color::Blue);

        let canvas_bg = Color::Rgb(0, 0, 0);  // fill_fn(0.0)
        // At least one fill cell should be brighter than the canvas background,
        // OR a braille trace character should be present.
        let signal_visible = (0..4u16).any(|y| {
            let c = buf.get(0, y);
            c.symbol() != " " || c.bg != canvas_bg
        });
        assert!(signal_visible, "mid signal should produce fill or trace cells distinct from canvas bg");
        // Top row should not have signal content
        assert_eq!(buf.get(0, 0).symbol(), " ");
        assert_eq!(buf.get(0, 0).bg, canvas_bg);
    }

    #[test]
    fn paint_vlines_through_bars_spans_full_height() {
        let area = Rect::new(0, 0, 1, 4);
        let mut buf = ratatui::buffer::Buffer::empty(area);
        let bars = vec![
            Bar { db: -90.0, trace_color: Color::Red },  // signal at min → floor=0
            Bar { db: -90.0, trace_color: Color::Red },
        ];
        let vlines = vec![VLine { col: 0, color: Color::Yellow, through_bars: true }];
        paint_vlines(&mut buf, area, &vlines, &bars, -90.0, -20.0);
        for y in 0..4 {
            assert_eq!(buf.get(0, y).symbol(), "│", "row {y}");
            assert_eq!(buf.get(0, y).fg, Color::Yellow);
        }
    }

    #[test]
    fn paint_vlines_non_through_only_above_signal() {
        // Signal at max → floor=rows → non-through vline has nowhere to draw
        let area = Rect::new(0, 0, 1, 4);
        let mut buf = ratatui::buffer::Buffer::empty(area);
        let bars = vec![
            Bar { db: -20.0, trace_color: Color::Red },  // at max → floor=4
            Bar { db: -20.0, trace_color: Color::Red },
        ];
        let vlines = vec![VLine { col: 0, color: Color::Yellow, through_bars: false }];
        paint_vlines(&mut buf, area, &vlines, &bars, -90.0, -20.0);
        for y in 0..4 {
            assert_eq!(buf.get(0, y).symbol(), " ", "non-through vline should not draw over full signal at row {y}");
        }
    }
}
