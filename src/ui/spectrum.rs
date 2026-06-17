use std::sync::Arc;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::band_plan::BAND_PLAN;
use crate::ui::panel::Panel;
use crate::ui::spectrum_bars::{self, Bar, VLine};

// ── Spectrum step sizes ───────────────────────────────────────────────────────

pub const SPECTRUM_STEPS: &[u64] = &[
    1_000, 5_000, 10_000, 25_000, 100_000, 500_000, 1_000_000, 5_000_000, 10_000_000,
];

pub fn prev_spectrum_step(current: u64) -> u64 {
    match SPECTRUM_STEPS.iter().position(|&s| s == current) {
        Some(idx) => SPECTRUM_STEPS[idx.saturating_sub(1)],
        // Not in list: find the largest step strictly below current
        None => SPECTRUM_STEPS.iter().copied()
            .filter(|&s| s < current)
            .last()
            .unwrap_or(SPECTRUM_STEPS[0]),
    }
}

pub fn next_spectrum_step(current: u64) -> u64 {
    match SPECTRUM_STEPS.iter().position(|&s| s == current) {
        Some(idx) => SPECTRUM_STEPS[(idx + 1).min(SPECTRUM_STEPS.len() - 1)],
        // Not in list: find the smallest step strictly above current
        None => SPECTRUM_STEPS.iter().copied()
            .find(|&s| s > current)
            .unwrap_or(*SPECTRUM_STEPS.last().unwrap()),
    }
}

fn fmt_khz(hz: u64) -> String {
    if hz >= 1_000_000 { format!("{:.1}M", hz as f64 / 1_000_000.0) }
    else               { format!("{}k", hz / 1_000) }
}

pub fn fmt_spectrum_step(hz: u64) -> String {
    if hz >= 1_000_000 { format!("{} MHz", hz / 1_000_000) }
    else { format!("{} kHz", hz / 1_000) }
}

pub struct SpectrumPanel;

impl Panel for SpectrumPanel {
    fn name(&self) -> &'static str { "spectrum" }
    fn min_size(&self) -> (u16, u16) { (40, 10) }
    fn focus_key(&self) -> Option<char> { Some('e') }
    fn focus_bindings(&self) -> &'static [(&'static str, &'static str)] {
        &[
            ("← →", "Tune frequency"),
            ("[ ]", "Step size"),
            ("↑ ↓", "Zoom y-axis"),
            ("J K",  "Cursor"),
            ("M",    "Place/remove marker"),
            ("B",    "Cycle channel BW on nearest marker"),
            ("H",    "Hold/unhold frame"),
        ]
    }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let stale = state.waterfall.last_fft.as_ref()
            .map(|fr| fr.timestamp.elapsed() > std::time::Duration::from_millis(500))
            .unwrap_or(false);
        let no_data = state.waterfall.last_fft.is_none();

        let border_color = if focused          { theme.border_focused }
            else if stale || no_data           { theme.stale }
            else                               { theme.border_accent };

        // Title: 'e' in "Spectrum" highlighted as focus key indicator
        let key_style = Style::default().fg(theme.value_hi).add_modifier(Modifier::BOLD);
        let mut title_spans = vec![
            Span::raw(" Sp"),
            Span::styled("e", key_style),
            Span::raw("ctrum"),
        ];
        if state.spectrum.hold.is_some() {
            title_spans.push(Span::styled(" [HOLD]", Style::default().fg(theme.status_warn)));
        }
        if stale {
            title_spans.push(Span::styled(" [STALE]", Style::default().fg(theme.stale)));
        }
        title_spans.push(Span::raw(" "));
        let title_line = Line::from(title_spans);

        match state.waterfall.last_fft.as_ref() {
            None => {
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
            }
            Some(frame) => {
                let outer_block = Block::default()
                    .title(title_line)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(border_color));
                let inner = outer_block.inner(area);
                f.render_widget(outer_block, area);

                // Layout: dBFS label column (6) | canvas+freq[+indicator]
                let cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Length(6), Constraint::Min(1)])
                    .split(inner);

                let v_constraints: Vec<Constraint> = if focused {
                    vec![Constraint::Min(4), Constraint::Length(1), Constraint::Length(1)]
                } else {
                    vec![Constraint::Min(4), Constraint::Length(1)]
                };
                let rows    = Layout::default().direction(Direction::Vertical)
                    .constraints(v_constraints.clone()).split(cols[1]);
                let db_rows = Layout::default().direction(Direction::Vertical)
                    .constraints(v_constraints).split(cols[0]);

                let canvas_area    = rows[0];
                let freq_area      = rows[1];
                let indicator_area = if focused { rows.get(2).copied() } else { None };

                let n_bins = frame.bins_dbfs.len();
                if n_bins == 0 { return; }
                let bw = frame.sample_rate;
                if bw <= 0.0 { return; }
                let left_hz  = frame.center_freq_hz as f64 - bw / 2.0;
                let right_hz = frame.center_freq_hz as f64 + bw / 2.0;

                // Dynamic y-range from state (user-controlled zoom)
                let y_min_f = state.spectrum.y_min;
                let y_max_f = state.spectrum.y_max;

                // Arc::clone is O(1) — no data copied
                let bins  = Arc::clone(&frame.bins_dbfs);
                let peaks = Arc::clone(&frame.peak_hold);
                let noise_floor = frame.noise_floor;

                // Hold ghost: Arc clone, O(1)
                let held_bins = state.spectrum.hold.clone();

                // Cursor power
                let cursor_power: Option<f32> = state.spectrum.cursor_freq.and_then(|cf| {
                    let frac = (cf as f64 - left_hz) / bw;
                    if (0.0..=1.0).contains(&frac) {
                        let idx = (frac * (n_bins - 1) as f64).round() as usize;
                        Some(bins[idx.min(n_bins - 1)])
                    } else { None }
                });
                let cursor_freq_mhz = state.spectrum.cursor_freq
                    .map(|f| f as f64 / 1_000_000.0);

                // ── Braille spectrum render ───────────────────────────────
                // Downsample to 2× terminal width for braille horizontal resolution.
                let width = canvas_area.width as usize;
                if width > 0 {
                    let width2    = width * 2;
                    let col_db    = spectrum_bars::downsample_max(&bins,  width2);
                    let col_peak  = spectrum_bars::downsample_max(&peaks, width2);
                    let col_hold  = held_bins
                        .as_ref()
                        .map(|h| spectrum_bars::downsample_max(h, width2))
                        .unwrap_or_default();

                    let bars: Vec<Bar> = col_db
                        .iter()
                        .map(|&db| {
                            let t = ((db - y_min_f) / (y_max_f - y_min_f)).clamp(0.0, 1.0);
                            Bar {
                                db,
                                trace_color: spectrum_bars::accent_trace_color(theme.border_accent, t),
                            }
                        })
                        .collect();

                    let freq_to_col = |freq_hz: f64| -> Option<u16> {
                        let frac = (freq_hz - left_hz) / bw;
                        if (0.0..=1.0).contains(&frac) {
                            Some(((frac * (width.saturating_sub(1)) as f64).round() as usize)
                                .min(width - 1) as u16)
                        } else {
                            None
                        }
                    };

                    let mut vlines: Vec<VLine> = Vec::new();
                    for mk in &state.spectrum.markers {
                        if let Some(c) = freq_to_col(mk.freq_hz as f64) {
                            vlines.push(VLine { col: c, color: theme.status_warn, through_bars: false });
                        }
                        if let Some(ch_bw) = mk.channel_bw_hz {
                            let half = ch_bw as f64 / 2.0;
                            if let Some(c) = freq_to_col(mk.freq_hz as f64 - half) {
                                vlines.push(VLine { col: c, color: theme.border_accent, through_bars: false });
                            }
                            if let Some(c) = freq_to_col(mk.freq_hz as f64 + half) {
                                vlines.push(VLine { col: c, color: theme.border_accent, through_bars: false });
                            }
                        }
                    }
                    if let Some(cf) = state.spectrum.cursor_freq {
                        if let Some(c) = freq_to_col(cf as f64) {
                            vlines.push(VLine { col: c, color: theme.value_hi, through_bars: true });
                        }
                    }

                    let buf = f.buffer_mut();
                    // Full-brightness palette for braille ⣿ interior fg.
                    // Interior cells are rendered as braille characters (not bg-colored
                    // blocks), so the full vivid palette colors are appropriate here.
                    let fill_fn = |t: f32| theme.palette_color(t);
                    spectrum_bars::paint_braille(
                        buf, canvas_area, &bars,
                        &col_peak, &col_hold, noise_floor,
                        y_min_f, y_max_f,
                        fill_fn,
                        theme.peak_hold,      // peak hold caps: vivid gold/yellow
                        theme.border_default, // hold ghost caps: subtly visible
                        theme.label,          // noise floor dashes: readable dim
                    );
                    spectrum_bars::paint_vlines(buf, canvas_area, &vlines, &bars, y_min_f, y_max_f);
                }

                // ── Band plan overlay (text on top of canvas, top row) ────
                if canvas_area.height >= 2 && canvas_area.width > 4 {
                    let cw = canvas_area.width as f64;
                    let mut next_free_col: i32 = -1;
                    for &(band_s, band_e, label) in BAND_PLAN {
                        let bs = band_s as f64;
                        let be = band_e as f64;
                        if bs >= right_hz || be <= left_hz { continue; }
                        let vis_s   = bs.max(left_hz);
                        let vis_e   = be.min(right_hz);
                        let center  = (vis_s + vis_e) / 2.0;
                        let frac    = (center - left_hz) / bw;
                        let col     = (frac * cw) as u16;
                        let lw      = label.len() as u16;
                        let col     = col.min(canvas_area.width.saturating_sub(lw));
                        if (col as i32) < next_free_col { continue; }
                        next_free_col = col as i32 + lw as i32 + 1;
                        f.render_widget(
                            Paragraph::new(Span::styled(label, Style::default().fg(theme.label))),
                            Rect { x: canvas_area.x + col, y: canvas_area.y, width: lw, height: 1 },
                        );
                    }
                }

                // ── Marker labels — collision-aware multi-row placement ───
                if canvas_area.height >= 3 {
                    let cw = canvas_area.width as f64;
                    // max rows: up to 1/3 of canvas height, at least 1, at most 4
                    let max_rows = ((canvas_area.height / 3) as usize).max(1).min(4);

                    // Collect visible markers sorted left→right by frequency
                    let mut visible: Vec<(f64, &crate::state::SpectrumMarker)> = state.spectrum.markers.iter()
                        .filter_map(|mk| {
                            let frac = (mk.freq_hz as f64 - left_hz) / bw;
                            if (0.0..=1.0).contains(&frac) { Some((frac, mk)) } else { None }
                        })
                        .collect();
                    visible.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

                    // row_end[i] = first free column on row i (1-indexed in canvas rows)
                    let mut row_end: Vec<u16> = vec![0u16; max_rows];

                    for (frac, mk) in visible {
                        let bw_suffix = match (mk.channel_bw_hz, mk.measured_bw_hz) {
                            (Some(ch), Some(meas)) => {
                                let pct = meas as f64 / ch as f64 * 100.0 - 100.0;
                                format!(" {}/{} {:+.0}%", fmt_khz(ch), fmt_khz(meas), pct)
                            }
                            (Some(ch), None) => format!(" {}?", fmt_khz(ch)),
                            _ => String::new(),
                        };
                        let text = format!("▼{}{}", mk.label, bw_suffix);
                        let lw   = text.chars().count() as u16;
                        let col  = (frac * cw) as u16;
                        let col  = col.min(canvas_area.width.saturating_sub(lw));

                        // Pick the first row where this label fits without overlap
                        let row = row_end.iter().position(|&end| col >= end)
                            .unwrap_or(max_rows - 1);
                        row_end[row] = row_end[row].max(col + lw + 1);

                        f.render_widget(
                            Paragraph::new(Span::styled(
                                text,
                                Style::default().fg(theme.status_warn).add_modifier(Modifier::BOLD),
                            )),
                            Rect {
                                x: canvas_area.x + col,
                                y: canvas_area.y + 1 + row as u16,
                                width: lw,
                                height: 1,
                            },
                        );
                    }
                }

                // ── Frequency axis ────────────────────────────────────────
                let freq_labels: Vec<String> = (0..=4)
                    .map(|i| format!("{:.2}M", (left_hz + bw * i as f64 / 4.0) / 1_000_000.0))
                    .collect();
                let cw  = canvas_area.width as usize;
                let lw  = freq_labels.iter().map(|s| s.len()).max().unwrap_or(7);
                let seg = (cw.saturating_sub(lw)) / 4;
                f.render_widget(
                    Paragraph::new(Span::raw(format!(
                        "{:<w$}{:<w$}{:<w$}{:<w$}{}",
                        freq_labels[0], freq_labels[1],
                        freq_labels[2], freq_labels[3], freq_labels[4],
                        w = seg
                    ))).style(Style::default().fg(theme.value)),
                    freq_area,
                );

                // ── Tuning / cursor indicator (focus only) ────────────────
                if let Some(ind_area) = indicator_area {
                    let step_str  = fmt_spectrum_step(state.spectrum.step_hz);
                    let freq_str  = format!("  {:.3} MHz  ", state.radio.frequency as f64 / 1_000_000.0);

                    let right_info: String = match (cursor_freq_mhz, cursor_power) {
                        (Some(cf), Some(pwr)) => format!("  cur: {:.3} MHz  {:.1} dBFS  step {}  J/K", cf, pwr, step_str),
                        _ => format!("  step {}  [/]", step_str),
                    };

                    let center_len    = 2 + freq_str.chars().count();
                    let right_info_w  = right_info.chars().count();
                    let dashes        = (ind_area.width as usize).saturating_sub(center_len + right_info_w);
                    // Center the ◀ freq ▶ handle in the panel. The trailing
                    // right_info sits to the right of the handle, so the left arm
                    // must balance right_arm + right_info_w — not just half the
                    // dashes — or the handle drifts left of center.
                    let left_arm      = ((ind_area.width as usize).saturating_sub(center_len) / 2).min(dashes);
                    let right_arm     = dashes - left_arm;
                    let line  = Line::from(vec![
                        Span::styled("─".repeat(left_arm), Style::default().fg(theme.border_dim)),
                        Span::styled("◀", Style::default().fg(theme.border_accent).add_modifier(Modifier::BOLD)),
                        Span::styled(freq_str, Style::default().fg(theme.value_hi).add_modifier(Modifier::BOLD)),
                        Span::styled("▶", Style::default().fg(theme.border_accent).add_modifier(Modifier::BOLD)),
                        Span::styled("─".repeat(right_arm), Style::default().fg(theme.border_dim)),
                        Span::styled(right_info, Style::default().fg(theme.label)),
                    ]);
                    f.render_widget(Paragraph::new(line), ind_area);
                }

                // ── dBFS axis labels (dynamic, tracks zoom) ───────────────
                let h = db_rows[0].height as usize;
                if h > 0 {
                    let mut label_lines: Vec<Line> = vec![Line::raw(""); h];
                    for i in 0..=4 {
                        let frac = i as f32 / 4.0;
                        let db   = y_max_f - (y_max_f - y_min_f) * frac;
                        let row  = (frac * h.saturating_sub(1) as f32).round() as usize;
                        label_lines[row.min(h - 1)] = Line::from(
                            Span::styled(format!("{:>4.0}", db), Style::default().fg(theme.value))
                        );
                    }
                    f.render_widget(
                        Paragraph::new(label_lines).block(
                            Block::default()
                                .borders(Borders::RIGHT)
                                .border_style(Style::default().fg(border_color))
                        ),
                        db_rows[0],
                    );
                }
            }
        }
    }
}
