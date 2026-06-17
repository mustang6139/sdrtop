use std::sync::Arc;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Line as CanvasLine},
        Paragraph,
    },
    Frame,
};

use crate::palette::{magnitude_to_color_themed, ColorDepth};
use crate::state::SdrMetrics;
use crate::ui::band_plan::BAND_PLAN;
use crate::ui::chrome;
use crate::ui::panel::Panel;

/// Dim an `Rgb` color's brightness by `f` (0.0–1.0). Non-Rgb colors pass through.
fn dim(c: Color, f: f32) -> Color {
    match c {
        Color::Rgb(r, g, b) => Color::Rgb(
            (r as f32 * f) as u8, (g as f32 * f) as u8, (b as f32 * f) as u8,
        ),
        other => other,
    }
}

/// Map a frequency to a canvas x-coordinate in `[0, n-1]`, or `None` if out of view.
fn freq_to_canvas_x(freq_hz: f64, left_hz: f64, bw: f64, n: f64) -> Option<f64> {
    if bw <= 0.0 { return None; }
    let frac = (freq_hz - left_hz) / bw;
    if (0.0..=1.0).contains(&frac) { Some(frac * (n - 1.0)) } else { None }
}

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

        // Nameplate: SPECTRUM with the 'E' focus key highlighted, plus live tags.
        let key_style  = Style::default().fg(theme.value_hi).add_modifier(Modifier::BOLD);
        let name_style = Style::default().fg(theme.label).add_modifier(Modifier::BOLD);
        let mut title_spans = chrome::nameplate(vec![
            Span::styled("SP", name_style),
            Span::styled("E", key_style),
            Span::styled("CTRUM", name_style),
        ], border_color);
        if state.spectrum.hold.is_some() {
            title_spans.push(Span::styled(" [HOLD]", Style::default().fg(theme.status_warn)));
        }
        if stale {
            title_spans.push(Span::styled(" [STALE]", Style::default().fg(theme.stale)));
        }
        let title_line = Line::from(title_spans);

        match state.waterfall.last_fft.as_ref() {
            None => {
                f.render_widget(
                    Paragraph::new("Waiting for RX\u{2026}")
                        .block(chrome::deck_block(border_color).title(title_line))
                        .alignment(Alignment::Center)
                        .style(Style::default().fg(theme.label)),
                    area,
                );
            }
            Some(frame) => {
                let outer_block = chrome::deck_block(border_color).title(title_line);
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

                // ── Braille dot-matrix spectrum (ratatui Canvas) ──────────
                // The Canvas widget rasterises every shape onto a 2×4 braille dot
                // grid per cell, giving the classic phosphor dot-matrix instrument
                // look at a stable, terminal-size-independent density.
                let n      = n_bins as f64;
                let y_min  = y_min_f as f64;
                let y_max  = y_max_f as f64;
                let span   = (y_max_f - y_min_f).max(1e-3);
                let depth  = ColorDepth::detect();

                // Height-based color gradient, pre-computed once (the closure can't
                // borrow Theme). Color depends ONLY on vertical position, never on the
                // fluctuating per-bin dB — so the spectrum can never flicker in color
                // frame to frame. One band per braille dot-row for a gap-free fill.
                //
                //   band_dim    → the soft glowing body (dimmed gradient)
                //   band_bright → the crisp full-brightness top edge
                let v_steps = (canvas_area.height as usize * 4).clamp(1, 512);
                let height_color = |s: usize| -> Color {
                    let frac = if v_steps > 1 { s as f32 / (v_steps - 1) as f32 } else { 0.0 };
                    magnitude_to_color_themed(y_min_f + frac * span, y_min_f, y_max_f, depth, theme)
                };
                let band_y:      Vec<f32>   = (0..v_steps)
                    .map(|s| {
                        let frac = if v_steps > 1 { s as f32 / (v_steps - 1) as f32 } else { 0.0 };
                        y_min_f + frac * span
                    })
                    .collect();
                let band_dim:    Vec<Color> = (0..v_steps).map(|s| dim(height_color(s), 0.45)).collect();
                let band_bright: Vec<Color> = (0..v_steps).map(height_color).collect();

                let peak_hold_color   = theme.peak_hold;
                let noise_floor_color = theme.noise_floor;
                // Frozen hold ghost: a soft, dimmed phosphor so it reads as a "past"
                // snapshot of the whole spectrum without competing with the live trace.
                let hold_color  = dim(theme.border_focused, 0.50);
                let cursor_color = theme.value_hi;
                let marker_color    = theme.status_warn;
                let bw_border_color = theme.border_accent;
                // Graticule: a faint reference grid (SA screen) drawn behind the
                // trace, at the same dB ticks and frequency quarters as the axes.
                let grid_color = theme.stale;

                // Cursor canvas x-coordinate (0..n-1).
                let cursor_x_canvas = state.spectrum.cursor_freq.and_then(|cf| {
                    freq_to_canvas_x(cf as f64, left_hz, bw, n)
                });

                // Marker x-coordinates + optional channel-BW boundary pairs.
                struct MarkerCanvas { x: Option<f64>, bw_lo: Option<f64>, bw_hi: Option<f64> }
                let marker_data: Vec<MarkerCanvas> = state.spectrum.markers.iter()
                    .filter_map(|mk| {
                        let x = freq_to_canvas_x(mk.freq_hz as f64, left_hz, bw, n);
                        let (bw_lo, bw_hi) = if let Some(ch_bw) = mk.channel_bw_hz {
                            let half = ch_bw as f64 / 2.0;
                            (freq_to_canvas_x(mk.freq_hz as f64 - half, left_hz, bw, n),
                             freq_to_canvas_x(mk.freq_hz as f64 + half, left_hz, bw, n))
                        } else { (None, None) };
                        if x.is_some() || bw_lo.is_some() || bw_hi.is_some() {
                            Some(MarkerCanvas { x, bw_lo, bw_hi })
                        } else { None }
                    })
                    .collect();

                f.render_widget(
                    Canvas::default()
                        .x_bounds([0.0, (n - 1.0).max(0.0)])
                        .y_bounds([y_min, y_max])
                        .paint(move |ctx| {
                            // 0. Graticule — faint dB + frequency reference grid,
                            //    drawn first so the trace and fill sit on top of it.
                            //    Only the parts above the signal show through, exactly
                            //    like a spectrum-analyser screen.
                            for i in 0..=4 {
                                let yv = y_min + (y_max - y_min) * (i as f64 / 4.0);
                                ctx.draw(&CanvasLine { x1: 0.0, y1: yv, x2: n - 1.0, y2: yv, color: grid_color });
                            }
                            for i in 0..=4 {
                                let xv = (n - 1.0).max(0.0) * (i as f64 / 4.0);
                                ctx.draw(&CanvasLine { x1: xv, y1: y_min, x2: xv, y2: y_max, color: grid_color });
                            }
                            // 1. Hold ghost — the entire frozen spectrum as a soft outline.
                            if let Some(ref held) = held_bins {
                                for i in 1..held.len() {
                                    ctx.draw(&CanvasLine {
                                        x1: (i - 1) as f64, y1: held[i - 1].clamp(y_min_f, y_max_f) as f64,
                                        x2: i as f64,       y2: held[i].clamp(y_min_f, y_max_f) as f64,
                                        color: hold_color,
                                    });
                                }
                            }
                            // 2. Filled body — a dimmed height-gradient glow. Drawn as
                            //    solid horizontal runs per band (every column that reaches
                            //    this height), so the fill is continuous: no isolated dots
                            //    blinking on and off as bins jitter across band edges.
                            for s in 0..v_steps {
                                let yb  = band_y[s];
                                let ybf = yb as f64;
                                let color = band_dim[s];
                                let mut i = 0usize;
                                while i < bins.len() {
                                    if bins[i] >= yb {
                                        let start = i;
                                        while i < bins.len() && bins[i] >= yb { i += 1; }
                                        ctx.draw(&CanvasLine {
                                            x1: start as f64, y1: ybf,
                                            x2: (i - 1) as f64, y2: ybf,
                                            color,
                                        });
                                    } else {
                                        i += 1;
                                    }
                                }
                            }
                            // 3. Live edge — crisp full-brightness trace connecting the bin
                            //    tops. Each segment is colored by its HEIGHT (stable), so
                            //    the edge never flickers in color either.
                            for i in 1..bins.len() {
                                let y0 = bins[i - 1].clamp(y_min_f, y_max_f);
                                let y1 = bins[i].clamp(y_min_f, y_max_f);
                                let frac = (((y0 + y1) * 0.5 - y_min_f) / span).clamp(0.0, 1.0);
                                let idx  = ((frac * (v_steps - 1) as f32) as usize).min(v_steps - 1);
                                ctx.draw(&CanvasLine {
                                    x1: (i - 1) as f64, y1: y0 as f64,
                                    x2: i as f64,       y2: y1 as f64,
                                    color: band_bright[idx],
                                });
                            }
                            // 4. Peak hold — a single connected gold line tracing the decaying
                            //    max envelope. A line (not scattered points) stays calm and
                            //    readable instead of blinking dot by dot.
                            for i in 1..peaks.len() {
                                ctx.draw(&CanvasLine {
                                    x1: (i - 1) as f64, y1: peaks[i - 1].clamp(y_min_f, y_max_f) as f64,
                                    x2: i as f64,       y2: peaks[i].clamp(y_min_f, y_max_f) as f64,
                                    color: peak_hold_color,
                                });
                            }
                            // 5. Noise floor reference line.
                            let nf = noise_floor.clamp(y_min_f, y_max_f) as f64;
                            ctx.draw(&CanvasLine { x1: 0.0, y1: nf, x2: n - 1.0, y2: nf, color: noise_floor_color });
                            // 6. Markers + channel-BW boundaries.
                            for md in &marker_data {
                                if let Some(cx) = md.x {
                                    ctx.draw(&CanvasLine { x1: cx, y1: y_min, x2: cx, y2: y_max, color: marker_color });
                                }
                                if let Some(lo) = md.bw_lo {
                                    ctx.draw(&CanvasLine { x1: lo, y1: y_min, x2: lo, y2: y_max, color: bw_border_color });
                                }
                                if let Some(hi) = md.bw_hi {
                                    ctx.draw(&CanvasLine { x1: hi, y1: y_min, x2: hi, y2: y_max, color: bw_border_color });
                                }
                            }
                            // 7. Tuning cursor — full-height line, always visible.
                            if let Some(cx) = cursor_x_canvas {
                                ctx.draw(&CanvasLine { x1: cx, y1: y_min, x2: cx, y2: y_max, color: cursor_color });
                            }
                        }),
                    canvas_area,
                );

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
                // Ticked axis: a ┬ at each quarter mark, value label trailing it,
                // so the frequency scale reads like a ruled instrument axis.
                let mut freq_spans: Vec<Span> = Vec::with_capacity(10);
                for (i, lab) in freq_labels.iter().enumerate() {
                    freq_spans.push(Span::styled("┬", Style::default().fg(border_color)));
                    let txt = if i < 4 {
                        format!("{:<w$}", lab, w = seg.saturating_sub(1))
                    } else {
                        lab.clone()
                    };
                    freq_spans.push(Span::styled(txt, Style::default().fg(theme.value)));
                }
                f.render_widget(Paragraph::new(Line::from(freq_spans)), freq_area);

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

                // ── dBFS axis (ticked scale, tracks zoom) ─────────────────
                // The right edge is a vertical rule `│`; at each labelled value
                // it becomes a tick `┤`, so the dB scale reads like a ruled
                // instrument axis instead of a plain border.
                let h = db_rows[0].height as usize;
                if h > 0 {
                    let mut row_label: Vec<Option<String>> = vec![None; h];
                    for i in 0..=4 {
                        let frac = i as f32 / 4.0;
                        let db   = y_max_f - (y_max_f - y_min_f) * frac;
                        let row  = (frac * h.saturating_sub(1) as f32).round() as usize;
                        row_label[row.min(h - 1)] = Some(format!("{:>5.0}", db));
                    }
                    let lines: Vec<Line> = (0..h).map(|r| {
                        let (lbl, edge) = match &row_label[r] {
                            Some(s) => (s.clone(), "┤"),
                            None    => (" ".repeat(5), "│"),
                        };
                        Line::from(vec![
                            Span::styled(lbl,  Style::default().fg(theme.value)),
                            Span::styled(edge, Style::default().fg(border_color)),
                        ])
                    }).collect();
                    f.render_widget(Paragraph::new(lines), db_rows[0]);
                }
            }
        }
    }
}
