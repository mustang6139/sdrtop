use std::sync::Arc;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Line as CanvasLine, Points},
        Block, BorderType, Borders, Paragraph,
    },
    Frame,
};

use crate::palette::{magnitude_to_color_themed, ColorDepth};
use crate::state::SdrMetrics;
use crate::ui::band_plan::BAND_PLAN;
use crate::ui::panel::Panel;

// ── Spectrum step sizes ───────────────────────────────────────────────────────

pub const SPECTRUM_STEPS: &[u64] = &[
    1_000, 5_000, 10_000, 25_000, 100_000, 500_000, 1_000_000, 5_000_000, 10_000_000,
];

pub fn prev_spectrum_step(current: u64) -> u64 {
    let idx = SPECTRUM_STEPS.iter().position(|&s| s == current).unwrap_or(4);
    SPECTRUM_STEPS[idx.saturating_sub(1)]
}

pub fn next_spectrum_step(current: u64) -> u64 {
    let idx = SPECTRUM_STEPS.iter().position(|&s| s == current).unwrap_or(4);
    SPECTRUM_STEPS[(idx + 1).min(SPECTRUM_STEPS.len() - 1)]
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
            ("H",    "Hold/unhold frame"),
            ("Esc",  "Exit focus"),
        ]
    }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let stale = state.waterfall.last_fft.as_ref()
            .map(|fr| fr.timestamp.elapsed() > std::time::Duration::from_millis(500))
            .unwrap_or(false);

        let border_color = if focused { theme.border_focused }
            else if stale { theme.stale }
            else { theme.border_accent };

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
            title_spans.push(Span::raw(" [STALE]"));
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

                let n_bins     = frame.bins_dbfs.len();
                let n          = n_bins as f64;
                let bw         = frame.sample_rate;
                let left_hz    = frame.center_freq_hz as f64 - bw / 2.0;
                let right_hz   = frame.center_freq_hz as f64 + bw / 2.0;

                // Dynamic y-range from state (user-controlled zoom)
                let y_min_f = state.spectrum.y_min;
                let y_max_f = state.spectrum.y_max;
                let y_min   = y_min_f as f64;
                let y_max   = y_max_f as f64;

                let depth = ColorDepth::detect();
                // Arc::clone is O(1) — no data copied
                let bins  = Arc::clone(&frame.bins_dbfs);
                let peaks = Arc::clone(&frame.peak_hold);
                let noise_floor = frame.noise_floor;

                // Per-bin colors pre-computed outside the closure (closure can't borrow Theme)
                let bin_colors: Vec<ratatui::style::Color> = bins.iter()
                    .map(|&db| magnitude_to_color_themed(db, y_min_f, y_max_f, depth, theme))
                    .collect();
                let peak_hold_color   = theme.peak_hold;
                let noise_floor_color = theme.noise_floor;

                // Hold ghost: Arc clone, O(1)
                let held_bins = state.spectrum.hold.clone();
                let hold_color = theme.border_dim;

                // Cursor: canvas x-coordinate (0..n-1)
                let cursor_x_canvas = state.spectrum.cursor_freq.and_then(|cf| {
                    let frac = (cf as f64 - left_hz) / bw;
                    if (0.0..=1.0).contains(&frac) { Some(frac * (n - 1.0)) } else { None }
                });
                let cursor_color = theme.value_hi;

                // Marker canvas x-coordinates
                let marker_xs: Vec<f64> = state.spectrum.markers.iter()
                    .filter_map(|mk| {
                        let frac = (mk.freq_hz as f64 - left_hz) / bw;
                        if (0.0..=1.0).contains(&frac) { Some(frac * (n - 1.0)) } else { None }
                    })
                    .collect();
                let marker_color = theme.status_warn;

                // Cursor power (read before closure)
                let cursor_power: Option<f32> = state.spectrum.cursor_freq.and_then(|cf| {
                    let frac = (cf as f64 - left_hz) / bw;
                    if (0.0..=1.0).contains(&frac) {
                        let idx = (frac * (n_bins - 1) as f64).round() as usize;
                        Some(bins[idx.min(n_bins - 1)])
                    } else { None }
                });
                let cursor_freq_mhz = state.spectrum.cursor_freq
                    .map(|f| f as f64 / 1_000_000.0);

                // ── Canvas ────────────────────────────────────────────────
                f.render_widget(
                    Canvas::default()
                        .x_bounds([0.0, n - 1.0])
                        .y_bounds([y_min, y_max])
                        .paint(move |ctx| {
                            // 1. Hold ghost (behind live signal)
                            if let Some(ref held) = held_bins {
                                for i in 1..held.len() {
                                    let y0 = held[i - 1].clamp(y_min_f, y_max_f) as f64;
                                    let y1 = held[i].clamp(y_min_f, y_max_f) as f64;
                                    ctx.draw(&CanvasLine {
                                        x1: (i - 1) as f64, y1: y0,
                                        x2: i as f64,       y2: y1,
                                        color: hold_color,
                                    });
                                }
                            }
                            // 2. Filled columns
                            for i in 0..bins.len() {
                                let y_top = bins[i].clamp(y_min_f, y_max_f) as f64;
                                ctx.draw(&CanvasLine {
                                    x1: i as f64, y1: y_min,
                                    x2: i as f64, y2: y_top,
                                    color: bin_colors[i],
                                });
                            }
                            // 3. Outline
                            for i in 1..bins.len() {
                                let y0 = bins[i - 1].clamp(y_min_f, y_max_f) as f64;
                                let y1 = bins[i].clamp(y_min_f, y_max_f) as f64;
                                ctx.draw(&CanvasLine {
                                    x1: (i - 1) as f64, y1: y0,
                                    x2: i as f64,       y2: y1,
                                    color: bin_colors[i - 1],
                                });
                            }
                            // 4. Peak hold
                            for (i, &db) in peaks.iter().enumerate() {
                                let y = db.clamp(y_min_f, y_max_f) as f64;
                                ctx.draw(&Points { coords: &[(i as f64, y)], color: peak_hold_color });
                            }
                            // 5. Noise floor
                            let nf = noise_floor.clamp(y_min_f, y_max_f) as f64;
                            ctx.draw(&CanvasLine { x1: 0.0, y1: nf, x2: n - 1.0, y2: nf, color: noise_floor_color });
                            // 6. Marker lines
                            for &mx in &marker_xs {
                                ctx.draw(&CanvasLine { x1: mx, y1: y_min, x2: mx, y2: y_max, color: marker_color });
                            }
                            // 7. Cursor line
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
                        if col as i32 <= next_free_col { continue; }
                        next_free_col = col as i32 + lw as i32 + 1;
                        f.render_widget(
                            Paragraph::new(Span::styled(label, Style::default().fg(theme.label))),
                            Rect { x: canvas_area.x + col, y: canvas_area.y, width: lw, height: 1 },
                        );
                    }
                }

                // ── Marker labels (second row of canvas) ──────────────────
                if canvas_area.height >= 3 {
                    let cw = canvas_area.width as f64;
                    for mk in &state.spectrum.markers {
                        let frac = (mk.freq_hz as f64 - left_hz) / bw;
                        if !(0.0..=1.0).contains(&frac) { continue; }
                        let col = (frac * cw) as u16;
                        let lw  = mk.label.len() as u16;
                        let col = col.min(canvas_area.width.saturating_sub(lw));
                        f.render_widget(
                            Paragraph::new(Span::styled(
                                format!("▼{}", mk.label),
                                Style::default().fg(theme.status_warn).add_modifier(Modifier::BOLD),
                            )),
                            Rect { x: canvas_area.x + col, y: canvas_area.y + 1, width: lw + 1, height: 1 },
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

                    let center_len = 2 + freq_str.len();
                    let left_arm   = (ind_area.width as usize).saturating_sub(center_len) / 2;
                    let right_arm  = (ind_area.width as usize)
                        .saturating_sub(left_arm + center_len + right_info.len());
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
