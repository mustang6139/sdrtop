//! `IqConstellationPanel` — 2-D braille dot-cloud of recent I/Q samples.
//!
//! Each frame shows up to [`CONSTELLATION_CAP`] normalised (I, Q) pairs from
//! the RX hot-path, decimated 1 : 1024. The cloud's position reveals the DC
//! offset; its shape reveals amplitude/phase imbalance (circular = perfect,
//! elliptical = amplitude imbalance, tilted = phase imbalance). A unit circle
//! and faint I/Q axes give a fixed reference frame.

use std::f64::consts::PI;

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Line as CanvasLine, Points},
        Block, BorderType, Borders, Paragraph,
    },
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

pub struct IqConstellationPanel;

/// Canvas coordinate half-extent — slightly wider than the unit circle so the
/// circle border and labels are not clipped.
const BOUND: f64 = 1.3;

/// Number of line segments used to approximate the unit circle.
const CIRCLE_SEGS: usize = 48;

/// Density-grid resolution (cells per axis) for the persistence colouring.
const DENSITY_GRID: usize = 28;

/// Number of heat buckets the cloud is split into, coolest → hottest.
const HEAT_LEVELS: usize = 5;

/// Cool→hot persistence palette: sparse points are a cool blue, dense cores glow
/// orange — the classic phosphor-scope look.
const HEAT: [Color; HEAT_LEVELS] = [
    Color::Rgb(35, 65, 115),   // sparse — cool blue
    Color::Rgb(30, 140, 150),  // teal
    Color::Rgb(70, 180, 90),   // green
    Color::Rgb(215, 200, 55),  // yellow
    Color::Rgb(245, 130, 35),  // hot orange (dense core)
];

/// Split the cloud into [`HEAT_LEVELS`] layers by local point density, so each can
/// be drawn in its own heat colour. Bins points on a [`DENSITY_GRID`]² grid over
/// the canvas extent; a point's bucket is `sqrt(cell_count / max_count)` (the sqrt
/// spreads the low end so sparse structure stays visible).
fn density_layers(coords: &[(f64, f64)]) -> Vec<Vec<(f64, f64)>> {
    let cell = |v: f64| -> usize {
        (((v + BOUND) / (2.0 * BOUND) * DENSITY_GRID as f64) as isize)
            .clamp(0, DENSITY_GRID as isize - 1) as usize
    };
    let mut counts = vec![0u32; DENSITY_GRID * DENSITY_GRID];
    for &(x, y) in coords {
        counts[cell(y) * DENSITY_GRID + cell(x)] += 1;
    }
    let max_c = counts.iter().copied().max().unwrap_or(1).max(1) as f64;

    let mut layers = vec![Vec::new(); HEAT_LEVELS];
    for &(x, y) in coords {
        let c = counts[cell(y) * DENSITY_GRID + cell(x)] as f64;
        let t = (c / max_c).sqrt();
        let k = ((t * HEAT_LEVELS as f64) as usize).min(HEAT_LEVELS - 1);
        layers[k].push((x, y));
    }
    layers
}

/// Fit a covariance ("RMS") ellipse to the cloud. Returns `(cx, cy, a, b, theta)`:
/// centre, semi-axes and tilt. The semi-axes use `sqrt(2·λ)` so a balanced ring is
/// traced exactly; amplitude imbalance then shows as `a≠b`, phase imbalance as a
/// non-zero tilt. `None` for too few points or a degenerate spread.
fn fit_ellipse(coords: &[(f64, f64)]) -> Option<(f64, f64, f64, f64, f64)> {
    let n = coords.len();
    if n < 16 { return None; }
    let nf = n as f64;
    let (mut sx, mut sy) = (0.0, 0.0);
    for &(x, y) in coords { sx += x; sy += y; }
    let (mx, my) = (sx / nf, sy / nf);

    let (mut cxx, mut cyy, mut cxy) = (0.0, 0.0, 0.0);
    for &(x, y) in coords {
        let (dx, dy) = (x - mx, y - my);
        cxx += dx * dx; cyy += dy * dy; cxy += dx * dy;
    }
    cxx /= nf; cyy /= nf; cxy /= nf;

    let tr = cxx + cyy;
    let det = cxx * cyy - cxy * cxy;
    let disc = (tr * tr / 4.0 - det).max(0.0).sqrt();
    let l1 = tr / 2.0 + disc;
    let l2 = (tr / 2.0 - disc).max(0.0);
    if l1 <= 1e-9 { return None; }

    let a = (2.0 * l1).sqrt();
    let b = (2.0 * l2).sqrt();
    let theta = 0.5 * (2.0 * cxy).atan2(cxx - cyy);
    Some((mx, my, a, b, theta))
}

impl Panel for IqConstellationPanel {
    fn name(&self) -> &'static str { "iq_constellation" }
    fn min_size(&self) -> (u16, u16) { (18, 10) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let stale = !state.radio.hw_streaming;
        let border_color = if focused { theme.border_focused }
            else if stale { theme.stale }
            else { theme.border_default };

        let title_line = Line::from(Span::styled(
            " IQ Constellation ",
            Style::default().fg(theme.label).add_modifier(Modifier::BOLD),
        ));
        let block = Block::default()
            .title(title_line)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        f.render_widget(block, area);

        if stale {
            f.render_widget(
                Paragraph::new(Span::styled("Waiting for RX\u{2026}", Style::default().fg(theme.label))),
                inner,
            );
            return;
        }

        if state.iq.constellation.is_empty() {
            f.render_widget(
                Paragraph::new(Span::styled("No samples yet\u{2026}", Style::default().fg(theme.label))),
                inner,
            );
            return;
        }

        // Pre-collect coords into an owned Vec so the closure can borrow them.
        let coords: Vec<(f64, f64)> = state.iq.constellation.iter()
            .map(|&(i, q)| (i as f64, q as f64))
            .collect();

        let dc_i = state.iq.dc_offset_i as f64;
        let dc_q = state.iq.dc_offset_q as f64;

        // Density-coloured layers (cool→hot) and the fitted imbalance ellipse,
        // computed once outside the paint closure and moved in.
        let layers = density_layers(&coords);
        let ellipse = fit_ellipse(&coords);

        let axis_color    = theme.border_dim;
        let circle_color  = theme.border_dim;
        let ref_color     = theme.border_dim;
        let ellipse_color = theme.border_focused;
        let dc_color      = theme.status_warn;
        let label_color   = theme.label;

        f.render_widget(
            Canvas::default()
                .x_bounds([-BOUND, BOUND])
                .y_bounds([-BOUND, BOUND])
                .paint(move |ctx| {
                    // I-axis (horizontal) + Q-axis (vertical)
                    ctx.draw(&CanvasLine { x1: -BOUND, y1: 0.0, x2: BOUND, y2: 0.0, color: axis_color });
                    ctx.draw(&CanvasLine { x1: 0.0, y1: -BOUND, x2: 0.0, y2: BOUND, color: axis_color });

                    // Faint ±0.5 reference ring (inner scale).
                    for k in 0..CIRCLE_SEGS {
                        let a0 = 2.0 * PI * k as f64 / CIRCLE_SEGS as f64;
                        let a1 = 2.0 * PI * (k + 1) as f64 / CIRCLE_SEGS as f64;
                        ctx.draw(&CanvasLine {
                            x1: 0.5 * a0.cos(), y1: 0.5 * a0.sin(),
                            x2: 0.5 * a1.cos(), y2: 0.5 * a1.sin(),
                            color: ref_color,
                        });
                    }
                    // Unit (1.0) reference circle.
                    for k in 0..CIRCLE_SEGS {
                        let a0 = 2.0 * PI * k as f64 / CIRCLE_SEGS as f64;
                        let a1 = 2.0 * PI * (k + 1) as f64 / CIRCLE_SEGS as f64;
                        ctx.draw(&CanvasLine {
                            x1: a0.cos(), y1: a0.sin(),
                            x2: a1.cos(), y2: a1.sin(),
                            color: circle_color,
                        });
                    }

                    // Constellation cloud — sparse (cool) layers first, dense (hot)
                    // core on top, for a phosphor-persistence look.
                    for (k, layer) in layers.iter().enumerate() {
                        ctx.draw(&Points { coords: layer, color: HEAT[k] });
                    }

                    // Fitted imbalance ellipse: axis ratio = amplitude imbalance,
                    // tilt = phase imbalance. Bright outline over the cloud.
                    if let Some((cx, cy, a, b, th)) = ellipse {
                        let (ct, st) = (th.cos(), th.sin());
                        let mut prev: Option<(f64, f64)> = None;
                        for k in 0..=CIRCLE_SEGS {
                            let t = 2.0 * PI * k as f64 / CIRCLE_SEGS as f64;
                            let (ex, ey) = (a * t.cos(), b * t.sin());
                            let x = cx + ex * ct - ey * st;
                            let y = cy + ex * st + ey * ct;
                            if let Some((px, py)) = prev {
                                ctx.draw(&CanvasLine { x1: px, y1: py, x2: x, y2: y, color: ellipse_color });
                            }
                            prev = Some((x, y));
                        }
                    }

                    // DC offset crosshair (short arms centred on the measured offset).
                    let arm = 0.07;
                    ctx.draw(&CanvasLine { x1: dc_i - arm, y1: dc_q,       x2: dc_i + arm, y2: dc_q,       color: dc_color });
                    ctx.draw(&CanvasLine { x1: dc_i,       y1: dc_q - arm, x2: dc_i,       y2: dc_q + arm, color: dc_color });

                    // Reference labels (no live numbers — just orientation).
                    let tick = |s: &str| Line::from(Span::styled(s.to_string(), Style::default().fg(label_color)));
                    ctx.print(BOUND - 0.16, 0.10, tick("I"));
                    ctx.print(0.07, BOUND - 0.08, tick("Q"));
                    ctx.print(1.02, -0.14, tick("1.0"));
                }),
            inner,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_name_and_min_size() {
        let p = IqConstellationPanel;
        assert_eq!(p.name(), "iq_constellation");
        let (w, h) = p.min_size();
        assert!(w > 0 && h > 0);
    }

    #[test]
    fn circle_segs_constant_is_positive_even() {
        assert!(CIRCLE_SEGS > 0);
        assert_eq!(CIRCLE_SEGS % 2, 0, "even number of segments gives symmetric circle");
    }

    /// A unit circle of points: balanced → the fitted ellipse is ~circular (a≈b≈1).
    fn ring(n: usize, scale_i: f64, scale_q: f64) -> Vec<(f64, f64)> {
        (0..n).map(|k| {
            let a = 2.0 * PI * k as f64 / n as f64;
            (scale_i * a.cos(), scale_q * a.sin())
        }).collect()
    }

    #[test]
    fn fit_ellipse_balanced_ring_is_circular() {
        let (_, _, a, b, _) = fit_ellipse(&ring(256, 1.0, 1.0)).unwrap();
        assert!((a - 1.0).abs() < 0.05, "a≈1, got {a:.3}");
        assert!((b - 1.0).abs() < 0.05, "b≈1, got {b:.3}");
        assert!((a - b).abs() < 0.05, "balanced → a≈b");
    }

    #[test]
    fn fit_ellipse_amplitude_imbalance_stretches_axes() {
        // I stretched 2×, Q unchanged → major axis ~2× the minor, tilt ~0.
        let (_, _, a, b, th) = fit_ellipse(&ring(256, 2.0, 1.0)).unwrap();
        assert!(a > b, "major > minor");
        assert!((a / b - 2.0).abs() < 0.1, "axis ratio ≈ 2, got {:.2}", a / b);
        assert!(th.abs() < 0.05 || (th.abs() - PI).abs() < 0.05, "tilt ≈ 0 along I");
    }

    #[test]
    fn fit_ellipse_too_few_points_is_none() {
        assert!(fit_ellipse(&ring(8, 1.0, 1.0)).is_none());
        assert!(fit_ellipse(&[]).is_none());
    }

    #[test]
    fn density_layers_partition_all_points() {
        let coords = ring(200, 1.0, 1.0);
        let layers = density_layers(&coords);
        assert_eq!(layers.len(), HEAT_LEVELS);
        let total: usize = layers.iter().map(|l| l.len()).sum();
        assert_eq!(total, coords.len(), "every point lands in exactly one layer");
    }

    #[test]
    fn density_layers_hot_core_for_concentrated_cloud() {
        // Most points piled on one spot + a few scattered → the hottest layer is used.
        let mut coords = vec![(0.1, 0.1); 500];
        coords.extend(ring(20, 1.0, 1.0));
        let layers = density_layers(&coords);
        assert!(!layers[HEAT_LEVELS - 1].is_empty(), "dense core should reach the hot layer");
    }
}
