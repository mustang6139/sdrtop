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

use crate::app::fmt_spectrum_step;
use crate::palette::{magnitude_to_color_themed, ColorDepth};
use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

const DB_MIN: f32 = -120.0;
const DB_MAX: f32 = 0.0;

pub struct SpectrumPanel;

impl Panel for SpectrumPanel {
    fn name(&self) -> &'static str { "spectrum" }
    fn min_size(&self) -> (u16, u16) { (40, 10) }
    fn focus_key(&self) -> Option<char> { Some('e') }
    fn focus_bindings(&self) -> &'static [(&'static str, &'static str)] {
        &[
            ("← →", "Tune frequency"),
            ("[ ]", "Step size"),
            ("Esc",  "Exit focus"),
        ]
    }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let stale = state.last_fft_frame.as_ref().map(|fr| {
            fr.timestamp.elapsed() > std::time::Duration::from_millis(500)
        }).unwrap_or(false);

        let border_color = if focused { theme.border_focused }
            else if stale { theme.stale }
            else { theme.border_accent };

        // Title: 'e' in "Spectrum" is highlighted as the focus key indicator
        let key_style = Style::default().fg(theme.value_hi).add_modifier(Modifier::BOLD);
        let suffix = if stale { "ctrum [STALE] " } else { "ctrum " };
        let title_line = Line::from(vec![
            Span::raw(" Sp"),
            Span::styled("e", key_style),
            Span::raw(suffix),
        ]);

        match state.last_fft_frame.as_ref() {
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
                // Single outer block provides all borders + title
                let outer_block = Block::default()
                    .title(title_line)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(border_color));
                let inner = outer_block.inner(area);
                f.render_widget(outer_block, area);

                // Horizontal split: dBFS label column (6) + canvas+freq
                let cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Length(6), Constraint::Min(1)])
                    .split(inner);

                // Vertical split: canvas + freq axis + optional tuning indicator (focus only)
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

                let n = frame.bins_dbfs.len() as f64;

                // Fixed y range — anchors the spectrum to a stable absolute dBFS scale.
                // Dynamic zoom caused per-frame y-bound changes that made the line bounce.
                let y_min_f = DB_MIN;
                let y_max_f = DB_MAX;
                let y_min = DB_MIN as f64;
                let y_max = DB_MAX as f64;

                // Precompute per-bin colors outside the Canvas closure (avoids lifetime issue).
                // Colors map to the visible range so the spectrum line uses the full palette.
                let depth = ColorDepth::detect();
                let bins = frame.bins_dbfs.clone();
                let peaks = frame.peak_hold.clone();
                let noise_floor = frame.noise_floor;

                let bin_colors: Vec<ratatui::style::Color> = bins.iter()
                    .map(|&db| magnitude_to_color_themed(db, y_min_f, y_max_f, depth, theme))
                    .collect();
                let peak_hold_color  = theme.peak_hold;
                let noise_floor_color = theme.noise_floor;

                // Spectrum canvas — outer block handles all borders
                f.render_widget(
                    Canvas::default()
                        .x_bounds([0.0, n - 1.0])
                        .y_bounds([y_min, y_max])
                        .paint(move |ctx| {
                            // Filled columns: vertical line per bin from DB_MIN up to bin level.
                            // This anchors the spectrum to the panel bottom on a fixed dBFS scale.
                            for i in 0..bins.len() {
                                let y_top = bins[i].clamp(y_min_f, y_max_f) as f64;
                                ctx.draw(&CanvasLine {
                                    x1: i as f64, y1: y_min,
                                    x2: i as f64, y2: y_top,
                                    color: bin_colors[i],
                                });
                            }
                            // Outline on top of the fill for a clean signal edge
                            for i in 1..bins.len() {
                                let y0 = bins[i - 1].clamp(y_min_f, y_max_f) as f64;
                                let y1 = bins[i].clamp(y_min_f, y_max_f) as f64;
                                ctx.draw(&CanvasLine {
                                    x1: (i - 1) as f64, y1: y0,
                                    x2: i as f64,       y2: y1,
                                    color: bin_colors[i - 1],
                                });
                            }
                            // Peak hold markers
                            for (i, &db) in peaks.iter().enumerate() {
                                let y = db.clamp(y_min_f, y_max_f) as f64;
                                ctx.draw(&Points {
                                    coords: &[(i as f64, y)],
                                    color: peak_hold_color,
                                });
                            }
                            // Noise floor
                            let nf = noise_floor.clamp(y_min_f, y_max_f) as f64;
                            ctx.draw(&CanvasLine {
                                x1: 0.0,       y1: nf,
                                x2: n - 1.0,   y2: nf,
                                color: noise_floor_color,
                            });
                        }),
                    canvas_area,
                );

                // Frequency axis labels — proportionally distributed across canvas width
                let bw = frame.sample_rate;
                let left_hz = frame.center_freq_hz as f64 - bw / 2.0;
                let freq_labels: Vec<String> = (0..=4)
                    .map(|i| format!("{:.2}M", (left_hz + bw * i as f64 / 4.0) / 1_000_000.0))
                    .collect();
                let cw = canvas_area.width as usize;
                let lw = freq_labels.iter().map(|s| s.len()).max().unwrap_or(7);
                let seg = (cw.saturating_sub(lw)) / 4;
                f.render_widget(
                    Paragraph::new(Span::raw(format!(
                        "{:<w$}{:<w$}{:<w$}{:<w$}{}",
                        freq_labels[0], freq_labels[1],
                        freq_labels[2], freq_labels[3], freq_labels[4],
                        w = seg
                    )))
                    .style(Style::default().fg(theme.value)),
                    freq_area,
                );

                // Tuning indicator — shown only in spectrum focus mode
                if let Some(ind_area) = indicator_area {
                    let step_str = fmt_spectrum_step(state.spectrum_step_hz);
                    let freq_str = format!("  {:.3} MHz  ", state.frequency as f64 / 1_000_000.0);
                    let right_info = format!("  step {}  [/]", step_str);
                    let fixed = 1 + 1 + freq_str.len() + 1 + 1 + right_info.len(); // ◀ + freq + ▶ + info
                    let arm = (ind_area.width as usize).saturating_sub(fixed) / 2;
                    let line = Line::from(vec![
                        Span::styled("─".repeat(arm), Style::default().fg(theme.border_dim)),
                        Span::styled("◀", Style::default().fg(theme.border_accent).add_modifier(Modifier::BOLD)),
                        Span::styled(freq_str, Style::default().fg(theme.value_hi).add_modifier(Modifier::BOLD)),
                        Span::styled("▶", Style::default().fg(theme.border_accent).add_modifier(Modifier::BOLD)),
                        Span::styled("─".repeat(arm), Style::default().fg(theme.border_dim)),
                        Span::styled(right_info, Style::default().fg(theme.label)),
                    ]);
                    f.render_widget(Paragraph::new(line), ind_area);
                }

                // dBFS axis labels — each label placed at the character row that corresponds
                // to its dB value in the canvas, so they stay aligned on any terminal height.
                // Label column is 6 chars wide; Borders::RIGHT takes 1 → 5 chars for text.
                let h = db_rows[0].height as usize;
                if h > 0 {
                    const DB_MARKERS: &[(f32, &str)] = &[
                        (   0.0, "   0"),
                        ( -30.0, " -30"),
                        ( -60.0, " -60"),
                        ( -90.0, " -90"),
                        (-120.0, "-120"),
                    ];
                    let mut label_lines: Vec<Line> = vec![Line::raw(""); h];
                    for &(db, text) in DB_MARKERS {
                        let frac = (DB_MAX - db) / (DB_MAX - DB_MIN);
                        let row = (frac * h.saturating_sub(1) as f32).round() as usize;
                        label_lines[row.min(h - 1)] = Line::from(
                            Span::styled(text, Style::default().fg(theme.value))
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
