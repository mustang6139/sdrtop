use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::palette::{magnitude_to_color_themed, ColorDepth};
use crate::state::SdrMetrics;
use crate::ui::band_plan::BAND_PLAN;
use crate::ui::panel::Panel;
use crate::ui::spectrum::fmt_spectrum_step;

// ── Waterfall row-stride steps ────────────────────────────────────────────────

pub const WF_STRIDES: &[usize] = &[1, 2, 4, 8, 16, 32, 64];

pub fn prev_wf_stride(current: usize) -> usize {
    WF_STRIDES.iter().rev().find(|&&s| s < current).copied().unwrap_or(1)
}

pub fn next_wf_stride(current: usize) -> usize {
    WF_STRIDES.iter().find(|&&s| s > current).copied().unwrap_or(64)
}

const DB_MAX: f32 = 0.0;

pub struct WaterfallPanel;

impl WaterfallPanel {
    pub fn new() -> Self { Self }
}

impl Panel for WaterfallPanel {
    fn name(&self) -> &'static str { "waterfall" }
    fn min_size(&self) -> (u16, u16) { (40, 5) }
    fn focus_key(&self) -> Option<char> { Some('l') }
    fn focus_bindings(&self) -> &'static [(&'static str, &'static str)] {
        &[
            ("↑ ↓", "Zoom colour scale"),
            ("J K",  "Scroll history"),
            ("[ ]",  "Row stride (speed)"),
            ("M",    "Place/remove cursor"),
            ("← →",  "Move cursor"),
            ("Esc",  "Exit focus"),
        ]
    }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let buf = &state.waterfall.buffer;
        let db_min  = state.waterfall.db_min;
        let scroll  = state.waterfall.scroll_offset;
        let stride  = buf.row_stride;

        let stale = state.waterfall.last_fft.as_ref()
            .map(|fr| fr.timestamp.elapsed() > std::time::Duration::from_millis(500))
            .unwrap_or(false);
        let border_color = if focused { theme.border_focused }
            else if buf.paused || stale { theme.stale }
            else { theme.border_accent };

        // Title: second 'l' in "Waterfall" highlighted as focus key indicator
        let key_style = Style::default().fg(theme.value_hi).add_modifier(Modifier::BOLD);
        let mut title_spans = vec![
            Span::raw(" Waterfa"),
            Span::styled("l", key_style),
            Span::raw("l"),
        ];
        if buf.paused {
            title_spans.push(Span::styled(" [PAUSED]", Style::default().fg(theme.status_warn)));
        } else if stale {
            title_spans.push(Span::raw(" [STALE]"));
        }
        if stride > 1 {
            title_spans.push(Span::styled(
                format!(" [×{}]", stride),
                Style::default().fg(theme.label),
            ));
        }
        if scroll > 0 {
            title_spans.push(Span::styled(
                format!(" [↑{}]", scroll),
                Style::default().fg(theme.value_hi),
            ));
        }
        title_spans.push(Span::raw(" "));
        let title_line = Line::from(title_spans);

        if buf.rows.is_empty() {
            f.render_widget(
                Paragraph::new("Waiting for RX\u{2026}")
                    .block(
                        Block::default()
                            .title(title_line)
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(border_color)),
                    )
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(theme.label)),
                area,
            );
            return;
        }

        let block = Block::default()
            .title(title_line)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        f.render_widget(block, area);

        // When focused, reserve one row for the indicator line (like spectrum panel)
        let (content_area, indicator_area) = if focused && inner.height > 2 {
            let split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);
            (split[0], Some(split[1]))
        } else {
            (inner, None)
        };

        // Offset matches spectrum panel's dB-label column so both panels share the same x-axis
        const DB_COL: u16 = 6;
        let wf_area = Rect {
            x: content_area.x + DB_COL,
            y: content_area.y,
            width: content_area.width.saturating_sub(DB_COL),
            height: content_area.height,
        };
        let cols = wf_area.width as usize;
        if cols == 0 { return; }

        // Frequency bounds from last FFT frame (for cursor mapping)
        let (left_hz, bw) = state.waterfall.last_fft.as_ref()
            .map(|fr| (fr.center_freq_hz as f64 - fr.sample_rate / 2.0, fr.sample_rate))
            .unwrap_or((0.0, 1.0));

        // Cursor column in display space
        let cursor_col: Option<usize> = state.waterfall.cursor_freq.and_then(|cf| {
            let frac = (cf as f64 - left_hz) / bw;
            if (0.0..=1.0).contains(&frac) {
                Some(((frac * cols as f64) as usize).min(cols - 1))
            } else {
                None
            }
        });

        let rows_to_show = wf_area.height as usize;
        let max_scroll = buf.rows.len().saturating_sub(rows_to_show);
        let skip = scroll.min(max_scroll);

        let depth = ColorDepth::detect();
        let cursor_style = Style::default().fg(theme.value_hi);
        let mut lines: Vec<Line> = Vec::with_capacity(rows_to_show);

        for (_ts, row_data) in buf.rows.iter().skip(skip).take(rows_to_show) {
            let n = row_data.len();
            let mut spans: Vec<Span> = Vec::with_capacity(cols);
            for col in 0..cols {
                let bin_start = col * n / cols;
                let bin_end = (((col + 1) * n) / cols).max(bin_start + 1).min(n);
                let db = row_data[bin_start..bin_end]
                    .iter()
                    .cloned()
                    .fold(f32::NEG_INFINITY, f32::max);
                let bg = magnitude_to_color_themed(db, db_min, DB_MAX, depth, theme);
                if Some(col) == cursor_col {
                    spans.push(Span::styled("│", cursor_style.bg(bg)));
                } else {
                    spans.push(Span::styled(" ", Style::default().bg(bg)));
                }
            }
            lines.push(Line::from(spans));
        }

        f.render_widget(Paragraph::new(lines), wf_area);

        // ── Band plan overlay (dim labels on the top row of the waterfall) ─
        if wf_area.height >= 2 && wf_area.width > 4 {
            let cw = wf_area.width as f64;
            let right_hz = left_hz + bw;
            let mut next_free_col: i32 = -1;
            for &(band_s, band_e, label) in BAND_PLAN {
                let bs = band_s as f64;
                let be = band_e as f64;
                if bs >= right_hz || be <= left_hz { continue; }
                let vis_s  = bs.max(left_hz);
                let vis_e  = be.min(right_hz);
                let center = (vis_s + vis_e) / 2.0;
                let frac   = (center - left_hz) / bw;
                let col    = (frac * cw) as u16;
                let lw     = label.len() as u16;
                let col    = col.min(wf_area.width.saturating_sub(lw));
                if col as i32 <= next_free_col { continue; }
                next_free_col = col as i32 + lw as i32 + 1;
                f.render_widget(
                    Paragraph::new(Span::styled(label, Style::default().fg(theme.label))),
                    Rect { x: wf_area.x + col, y: wf_area.y, width: lw, height: 1 },
                );
            }
        }

        // dBFS colour scale legend — tracks dynamic db_min
        let legend_area = Rect {
            x: content_area.x,
            y: content_area.y,
            width: DB_COL,
            height: content_area.height,
        };
        let h = legend_area.height as usize;
        if h > 0 {
            let mut legend: Vec<Line> = Vec::with_capacity(h);
            for row in 0..h {
                let t = row as f32 / (h.saturating_sub(1)).max(1) as f32;
                let db = DB_MAX + (db_min - DB_MAX) * t;
                let bar_color = magnitude_to_color_themed(db, db_min, DB_MAX, depth, theme);
                let label = match row {
                    0 => format!("{:>+4} ", DB_MAX as i32),
                    r if r == h.saturating_sub(1) => format!("{:>4} ", db_min as i32),
                    r if r == h / 2 => format!("{:>4} ", ((DB_MAX + db_min) / 2.0) as i32),
                    _ => "     ".to_string(),
                };
                legend.push(Line::from(vec![
                    Span::styled("█", Style::default().fg(bar_color)),
                    Span::styled(label, Style::default().fg(theme.value)),
                ]));
            }
            f.render_widget(Paragraph::new(legend), legend_area);
        }

        // ── Indicator row (focus only) ─────────────────────────────────
        if let Some(ind_area) = indicator_area {
            let right_info: String = if let Some(cf) = state.waterfall.cursor_freq {
                // Cursor active: show freq, dBFS at top visible row, time ago
                let freq_mhz = cf as f64 / 1_000_000.0;
                let (db_at_cursor, secs_ago) = buf.rows.iter().skip(skip).next()
                    .and_then(|(ts, row)| {
                        let frac = (cf as f64 - left_hz) / bw;
                        if !(0.0..=1.0).contains(&frac) { return None; }
                        let col = ((frac * cols as f64) as usize).min(cols - 1);
                        let n = row.len();
                        let lo = col * n / cols;
                        let hi = ((col + 1) * n / cols).max(lo + 1).min(n);
                        let db = row[lo..hi].iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                        Some((db, ts.elapsed().as_secs()))
                    })
                    .unwrap_or((f32::NEG_INFINITY, 0));
                if db_at_cursor.is_finite() {
                    format!("  cur: {:.3} MHz  {:.1} dBFS  {}s ago  ← →  M", freq_mhz, db_at_cursor, secs_ago)
                } else {
                    format!("  cur: {:.3} MHz  ← →  M", freq_mhz)
                }
            } else {
                let step_str = fmt_spectrum_step(state.spectrum.step_hz);
                format!("  ×{}  frames/row  [ ]  M cursor  step {}  ↑↓ zoom  J/K scroll", stride, step_str)
            };

            let dashes = (ind_area.width as usize).saturating_sub(right_info.len());
            let line = Line::from(vec![
                Span::styled("─".repeat(dashes), Style::default().fg(theme.border_dim)),
                Span::styled(right_info, Style::default().fg(theme.label)),
            ]);
            f.render_widget(Paragraph::new(line), ind_area);
        }
    }
}
