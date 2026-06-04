//! Filled block-bar spectrum renderer (replaces the braille Canvas).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

/// Reduce `bins` (dBFS) to exactly `cols` columns, taking the MAX dBFS of each
/// column's covering bin range. Max preserves narrow peaks; mean would smear
/// them. Empty/zero cases yield an empty Vec.
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

/// Eighth-block glyphs indexed 0..=8. Index 0 is a space (empty), index 8 is a
/// full block. A partial top cell uses indices 1..=7.
pub const EIGHTHS: [&str; 9] = [" ", "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

/// Map a dBFS value to a vertical fill over `rows` cells.
/// Returns `(full_cells, top_eighth)` where `full_cells` cells are full blocks
/// (`EIGHTHS[8]`) and, if `top_eighth > 0`, one more cell above them shows
/// `EIGHTHS[top_eighth]`.
pub fn column_fill(db: f32, y_min: f32, y_max: f32, rows: u16) -> (u16, u8) {
    if rows == 0 || y_max <= y_min {
        return (0, 0);
    }
    let frac = ((db - y_min) / (y_max - y_min)).clamp(0.0, 1.0);
    let total_eighths = (frac * rows as f32 * 8.0).round() as u32;
    let full = (total_eighths / 8) as u16;
    let rem = (total_eighths % 8) as u8;
    (full, rem)
}

/// Per-column bar input: downsampled dBFS and its palette color.
pub struct Bar {
    pub db: f32,
    pub color: Color,
}

/// Paint filled bars into `area` of `buf`. `bars.len()` should equal
/// `area.width`; extra/short slices are clamped. Bars grow up from the bottom
/// row. The top partial cell uses the matching eighth glyph.
pub fn paint_bars(buf: &mut Buffer, area: Rect, bars: &[Bar], y_min: f32, y_max: f32) {
    let rows = area.height;
    if rows == 0 || area.width == 0 {
        return;
    }
    let bottom = area.y + rows - 1;
    let cols = (area.width as usize).min(bars.len());
    for (cx, bar) in bars.iter().enumerate().take(cols) {
        let (full, rem) = column_fill(bar.db, y_min, y_max, rows);
        let x = area.x + cx as u16;
        for r in 0..full.min(rows) {
            let y = bottom - r;
            buf.get_mut(x, y).set_symbol(EIGHTHS[8]).set_fg(bar.color);
        }
        if rem > 0 && full < rows {
            let y = bottom - full;
            buf.get_mut(x, y).set_symbol(EIGHTHS[rem as usize]).set_fg(bar.color);
        }
    }
}

/// First empty row index (0 = bottom) above the bar for a column of value `db`.
fn empty_floor(db: f32, y_min: f32, y_max: f32, rows: u16) -> u16 {
    let (full, rem) = column_fill(db, y_min, y_max, rows);
    (full + u16::from(rem > 0)).min(rows)
}

/// A vertical overlay line (marker / channel-BW boundary / cursor) at a column.
pub struct VLine {
    pub col: u16,
    pub color: Color,
}

/// Inputs for the overlay pass. Per-column slices share the bars length;
/// `peak_db`/`hold_db` may be empty to skip them.
pub struct Overlays<'a> {
    pub bar_db: &'a [f32],
    pub peak_db: &'a [f32],
    pub hold_db: &'a [f32],
    pub vlines: &'a [VLine],
    pub noise_floor: f32,
    pub peak_color: Color,
    pub hold_color: Color,
    pub noise_color: Color,
}

/// Paint overlays into `area`, only into empty cells above each bar.
pub fn paint_overlays(buf: &mut Buffer, area: Rect, ov: &Overlays, y_min: f32, y_max: f32) {
    let rows = area.height;
    if rows == 0 || area.width == 0 {
        return;
    }
    let bottom = area.y + rows - 1;
    let cols = area.width as usize;

    let row_of = |db: f32| -> u16 {
        let frac = ((db - y_min) / (y_max - y_min)).clamp(0.0, 1.0);
        (frac * (rows.saturating_sub(1)) as f32).round() as u16
    };

    for (slice, glyph, color) in [
        (ov.hold_db, "▔", ov.hold_color),
        (ov.peak_db, "▔", ov.peak_color),
    ] {
        for (cx, &db_val) in slice.iter().enumerate().take(cols.min(slice.len())) {
            let floor = empty_floor(ov.bar_db.get(cx).copied().unwrap_or(y_min), y_min, y_max, rows);
            let r = row_of(db_val);
            if r >= floor && r < rows {
                let x = area.x + cx as u16;
                let y = bottom - r;
                buf.get_mut(x, y).set_symbol(glyph).set_fg(color);
            }
        }
    }

    if ov.noise_floor > y_min {
        let r = row_of(ov.noise_floor);
        if r < rows {
            let y = bottom - r;
            for cx in 0..cols.min(ov.bar_db.len()) {
                let floor = empty_floor(ov.bar_db[cx], y_min, y_max, rows);
                if r >= floor {
                    let x = area.x + cx as u16;
                    buf.get_mut(x, y).set_symbol("─").set_fg(ov.noise_color);
                }
            }
        }
    }

    for vl in ov.vlines {
        if (vl.col as usize) >= cols {
            continue;
        }
        let floor = empty_floor(
            ov.bar_db.get(vl.col as usize).copied().unwrap_or(y_min),
            y_min, y_max, rows,
        );
        let x = area.x + vl.col;
        for r in floor..rows {
            let y = bottom - r;
            buf.get_mut(x, y).set_symbol("│").set_fg(vl.color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Color;

    fn sym(buf: &Buffer, x: u16, y: u16) -> String {
        buf.get(x, y).symbol().to_string()
    }

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

    #[test]
    fn fill_below_min_is_empty() {
        assert_eq!(column_fill(-100.0, -90.0, -20.0, 5), (0, 0));
    }

    #[test]
    fn fill_at_or_above_max_is_full() {
        assert_eq!(column_fill(-20.0, -90.0, -20.0, 5), (5, 0));
        assert_eq!(column_fill(0.0, -90.0, -20.0, 5), (5, 0));
    }

    #[test]
    fn fill_half_height() {
        assert_eq!(column_fill(-55.0, -90.0, -20.0, 4), (2, 0));
    }

    #[test]
    fn fill_partial_top_cell() {
        assert_eq!(column_fill(-55.0, -90.0, -20.0, 1), (0, 4));
    }

    #[test]
    fn fill_zero_rows() {
        assert_eq!(column_fill(-20.0, -90.0, -20.0, 0), (0, 0));
    }

    #[test]
    fn paint_bars_fills_from_bottom() {
        let area = Rect::new(0, 0, 2, 4);
        let mut buf = Buffer::empty(area);
        let bars = vec![
            Bar { db: -20.0, color: Color::Red },
            Bar { db: -90.0, color: Color::Red },
        ];
        paint_bars(&mut buf, area, &bars, -90.0, -20.0);
        for y in 0..4 {
            assert_eq!(sym(&buf, 0, y), "█", "col0 row {y}");
        }
        assert_eq!(buf.get(0, 3).fg, Color::Red);
        assert_eq!(sym(&buf, 1, 3), " ");
    }

    #[test]
    fn peak_cap_sits_above_bar() {
        let area = Rect::new(0, 0, 1, 4);
        let mut buf = Buffer::empty(area);
        let bars = vec![Bar { db: -55.0, color: Color::Red }];
        paint_bars(&mut buf, area, &bars, -90.0, -20.0);
        let ov = Overlays {
            bar_db: &[-55.0],
            peak_db: &[-20.0],
            hold_db: &[],
            vlines: &[],
            noise_floor: -200.0,
            peak_color: Color::White,
            hold_color: Color::DarkGray,
            noise_color: Color::Blue,
        };
        paint_overlays(&mut buf, area, &ov, -90.0, -20.0);
        assert_eq!(sym(&buf, 0, 0), "▔");
        assert_eq!(buf.get(0, 0).fg, Color::White);
        assert_eq!(sym(&buf, 0, 3), "█");
    }

    #[test]
    fn vline_only_fills_empty_region() {
        let area = Rect::new(0, 0, 1, 4);
        let mut buf = Buffer::empty(area);
        let bars = vec![Bar { db: -55.0, color: Color::Red }];
        paint_bars(&mut buf, area, &bars, -90.0, -20.0);
        let ov = Overlays {
            bar_db: &[-55.0],
            peak_db: &[],
            hold_db: &[],
            vlines: &[VLine { col: 0, color: Color::Yellow }],
            noise_floor: -200.0,
            peak_color: Color::White,
            hold_color: Color::DarkGray,
            noise_color: Color::Blue,
        };
        paint_overlays(&mut buf, area, &ov, -90.0, -20.0);
        assert_eq!(sym(&buf, 0, 0), "│");
        assert_eq!(buf.get(0, 0).fg, Color::Yellow);
        assert_eq!(sym(&buf, 0, 3), "█");
    }
}
