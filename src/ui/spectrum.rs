use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::Span,
    widgets::{
        canvas::{Canvas, Line as CanvasLine, Points},
        Block, BorderType, Borders, Paragraph,
    },
    Frame,
};

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
        &[("Esc", "Exit focus")]
    }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let stale = state.last_fft_frame.as_ref().map(|fr| {
            fr.timestamp.elapsed() > std::time::Duration::from_millis(500)
        }).unwrap_or(false);

        let title = if stale { " Spectrum [STALE] " } else { " Spectrum " };
        let border_color = if focused { theme.border_focused }
            else if stale { theme.stale }
            else { theme.border_accent };

        match state.last_fft_frame.as_ref() {
            None => {
                f.render_widget(
                    Paragraph::new("Waiting for RX\u{2026}")
                        .block(
                            Block::default()
                                .title(title)
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
                    .title(Span::styled(title, Style::default()))
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

                // Vertical split for right column: canvas above, freq axis below
                let rows = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(4), Constraint::Length(1)])
                    .split(cols[1]);

                // Mirror the same vertical split on the db column so labels align with canvas
                let db_rows = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(4), Constraint::Length(1)])
                    .split(cols[0]);

                let canvas_area = rows[0];
                let freq_area   = rows[1];

                let n = frame.bins_dbfs.len() as f64;

                // Precompute per-bin colors outside the Canvas closure (avoids lifetime issue)
                let depth = ColorDepth::detect();
                let bins = frame.bins_dbfs.clone();
                let peaks = frame.peak_hold.clone();
                let noise_floor = frame.noise_floor;

                let bin_colors: Vec<ratatui::style::Color> = bins.iter()
                    .map(|&db| magnitude_to_color_themed(db, DB_MIN, DB_MAX, depth, theme))
                    .collect();
                let peak_hold_color  = theme.peak_hold;
                let noise_floor_color = theme.noise_floor;

                // Spectrum canvas — outer block handles all borders
                f.render_widget(
                    Canvas::default()
                        .x_bounds([0.0, n])
                        .y_bounds([DB_MIN as f64, DB_MAX as f64])
                        .paint(move |ctx| {
                            // Spectrum outline: polyline connecting adjacent bin tops
                            for i in 1..bins.len() {
                                let y0 = bins[i - 1].clamp(DB_MIN, DB_MAX) as f64;
                                let y1 = bins[i].clamp(DB_MIN, DB_MAX) as f64;
                                ctx.draw(&CanvasLine {
                                    x1: (i - 1) as f64, y1: y0,
                                    x2: i as f64,       y2: y1,
                                    color: bin_colors[i - 1],
                                });
                            }
                            // Peak hold markers
                            for (i, &db) in peaks.iter().enumerate() {
                                let y = db.clamp(DB_MIN, DB_MAX) as f64;
                                ctx.draw(&Points {
                                    coords: &[(i as f64, y)],
                                    color: peak_hold_color,
                                });
                            }
                            // Noise floor
                            let nf = noise_floor.clamp(DB_MIN, DB_MAX) as f64;
                            ctx.draw(&CanvasLine {
                                x1: 0.0, y1: nf,
                                x2: n,   y2: nf,
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
                    .style(Style::default().fg(theme.label)),
                    freq_area,
                );

                // dBFS labels — right-border acts as divider between labels and canvas
                let db_text: String = (0..=4)
                    .map(|i| {
                        let db = DB_MAX - (DB_MAX - DB_MIN) * i as f32 / 4.0;
                        format!("{:+4.0}\n", db)
                    })
                    .collect();
                f.render_widget(
                    Paragraph::new(db_text)
                        .block(
                            Block::default()
                                .borders(Borders::RIGHT)
                                .border_style(Style::default().fg(border_color)),
                        )
                        .style(Style::default().fg(theme.label)),
                    db_rows[0],
                );
            }
        }
    }
}
