//! `command_rail` — the Command Rail layout's left instrument strip (`[1]`).
//!
//! A single vertical column that gathers what a poweruser reads at a glance:
//! the frequency hero (big segmented VFO + band tag), a value-first SIGNAL zone
//! (SNR with its short-term trend arrow, PWR, NF, SAT), the GAIN chain, the
//! STREAM health, and a one-line log foot. The header thins to status + dial
//! (see `SlimHeaderPanel`) and the frequency lives here instead.
//!
//! It carries the big frequency hero, value-first metrics with sparklines, and a
//! HUNT·MONITOR·BENCH mode strip whose lead card adapts to what you're doing
//! (tuning → Hunt peak-finder, gain → Bench gain-health, idle → Monitor watch).
//! The mode auto-follows actions and `Tab` (in rail-focus, key `c`) pins it.
//! Recall slots and the log overlay are still later steps. Rendering is two
//! non-overlapping `Paragraph`s (the stack and the bottom-anchored log foot), so
//! it never flickers.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use std::collections::VecDeque;

use crate::state::{active_recall_slot, RailMode, SdrMetrics, RECALL_SLOTS};
use super::charts::sparkline;
use super::header::{active_digit_idx, gain_bar, vfo_spans, vfo_string};
use super::micro_common::{fft_stale, fmt_rbw, snr_color};
use super::panel::Panel;
use super::spectrum::detect_peaks;
use super::{bigdigits, chrome, log};
use crate::ui::band_plan::band_at;

pub struct CommandRailPanel;

/// Combined front-end gain for the TOTAL readout: primary + secondary stage when
/// the device has two (HackRF LNA+VGA), else just the primary (RTL-SDR tuner).
fn total_gain(lna: u32, vga: u32, has_second_stage: bool) -> u32 {
    if has_second_stage { lna + vga } else { lna }
}

/// Throughput as a compact `5.2 MB/s` string; `—` when not streaming.
fn fmt_mb(bps: u64) -> String {
    if bps == 0 { "—".to_string() } else { format!("{:.1} MB/s", bps as f64 / 1_000_000.0) }
}

/// Width of the gain bar given the rail's inner width — leaves room for the
/// `LNA ` label, a space, and a 2-col value. Clamped so it neither vanishes on a
/// narrow rail nor sprawls on a wide one.
fn gain_bar_width(inner_w: usize) -> usize {
    inner_w.saturating_sub(10).clamp(4, 12)
}

/// Short-term trend of a metric history: mean of the recent half minus the older
/// half (same shape as `SignalState::snr_delta`). `None` until ≥4 samples.
fn series_delta(h: &VecDeque<f32>) -> Option<f32> {
    let n = h.len();
    if n < 4 { return None; }
    let half = n / 2;
    let older:  f32 = h.iter().take(half).sum::<f32>() / half as f32;
    let recent: f32 = h.iter().skip(n - half).sum::<f32>() / half as f32;
    Some(recent - older)
}

/// A trend arrow for a metric delta. `good_when_rising` colours the direction by
/// meaning: `Some(true)` → rising is good (SNR), `Some(false)` → rising is bad
/// (NF, SAT), `None` → neutral (PWR). Below `eps` it's a dim steady `→`.
fn trend_arrow(delta: Option<f32>, eps: f32, good_when_rising: Option<bool>,
               theme: &crate::Theme) -> Option<Span<'static>> {
    let d = delta?;
    let dir: i8 = if d > eps { 1 } else if d < -eps { -1 } else { 0 };
    let glyph = match dir { 1 => "↑", -1 => "↓", _ => "→" };
    let color = match good_when_rising {
        _ if dir == 0 => theme.stale,
        None          => theme.stale,
        Some(gw)      => if (dir == 1) == gw { theme.status_ok } else { theme.status_warn },
    };
    Some(Span::styled(glyph, Style::default().fg(color)))
}

/// One metric as the rail's two-row block: `LABEL … UNIT` over `VALUE … spark ↑`.
/// `value == None` renders a stale dash and drops the sparkline/arrow. Both lines
/// are padded to `iw` so the unit and the trend cluster sit flush right.
fn metric_block(label: &str, unit: &str, value: Option<String>, value_color: Color,
                spark: &str, arrow: Option<Span<'static>>, iw: usize,
                theme: &crate::Theme) -> [Line<'static>; 2] {
    let pad = |n: usize| Span::raw(" ".repeat(n.max(1)));

    // Row 1: label (left) + unit (right).
    let l1_used = 1 + label.chars().count() + unit.chars().count();
    let head = Line::from(vec![
        Span::raw(" "),
        Span::styled(label.to_string(), Style::default().fg(theme.label)),
        pad(iw.saturating_sub(l1_used)),
        Span::styled(unit.to_string(), Style::default().fg(theme.border_dim)),
    ]);

    // Row 2: big-ish bold value (left) + sparkline + arrow (right).
    let Some(val) = value else {
        let stale = Line::from(vec![
            Span::raw(" "),
            Span::styled("—".to_string(), Style::default().fg(theme.stale)),
        ]);
        return [head, stale];
    };
    let arrow_w = arrow.as_ref().map_or(0, |_| 2); // " " + glyph
    let right_w = spark.chars().count() + arrow_w;
    let used = 1 + val.chars().count() + right_w;
    let mut spans = vec![
        Span::raw(" "),
        Span::styled(val, Style::default().fg(value_color).add_modifier(Modifier::BOLD)),
        pad(iw.saturating_sub(used)),
        Span::styled(spark.to_string(), Style::default().fg(value_color)),
    ];
    if let Some(a) = arrow {
        spans.push(Span::raw(" "));
        spans.push(a);
    }
    [head, Line::from(spans)]
}

/// Colour for the ADC-saturation value: calm below 10 %, warn to 50 %, crit above.
fn sat_color(pct: f32, theme: &crate::Theme) -> Color {
    if pct >= 50.0 { theme.status_crit }
    else if pct >= 10.0 { theme.status_warn }
    else { theme.value }
}

/// How long a clip is remembered, and the window in which it's still "fresh"
/// (loud red) before it fades to a dim memory line.
const CLIP_FRESH_SECS:  u64 = 6;
const CLIP_MEMORY_SECS: u64 = 30;

/// Compact relative age for the alert-memory: `"4s"`, `"2m"`, `"1h"`. Pure.
fn fmt_since(secs: u64) -> String {
    if secs < 60        { format!("{secs}s") }
    else if secs < 3600 { format!("{}m", secs / 60) }
    else                { format!("{}h", secs / 3600) }
}

/// The SAT clip alert-memory state: `Some((age_secs, fresh))` while a clip is
/// still remembered, `None` once it's older than [`CLIP_MEMORY_SECS`]. A fresh
/// clip (≤ [`CLIP_FRESH_SECS`]) renders loud; afterwards it fades. Pure over the
/// clock so it's testable, and it only ever fades — it never flickers.
fn clip_alert(last_clip_at: Option<u64>, now: u64) -> Option<(u64, bool)> {
    let since = now.saturating_sub(last_clip_at?);
    (since <= CLIP_MEMORY_SECS).then_some((since, since <= CLIP_FRESH_SECS))
}

/// Columns the full `HUNT·MONITOR·BENCH` strip needs: a leading space, then each
/// mode as ` LABEL ` (label+2) plus a one-column gap. Pure, for the width check.
fn mode_tabs_full_w() -> usize {
    1 + RailMode::ALL.iter().map(|m| m.label().len() + 3).sum::<usize>()
}

/// The `HUNT·MONITOR·BENCH` mode strip — the active mode lit as a chip
/// (`value_hi` bg), the others dim. Falls back to 3-letter codes when the rail is
/// too narrow for the full labels, so the strip never clips mid-word.
fn mode_tabs_line(active: RailMode, iw: usize, theme: &crate::Theme) -> Line<'static> {
    let compact = mode_tabs_full_w() > iw;
    let mut spans = vec![Span::raw(" ")];
    for m in RailMode::ALL {
        let label = if compact { &m.label()[..3] } else { m.label() };
        let style = if m == active {
            Style::default().fg(Color::Rgb(4, 6, 15)).bg(theme.value_hi).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.label)
        };
        spans.push(Span::styled(format!(" {label} "), style));
        spans.push(Span::raw(" "));
    }
    Line::from(spans)
}

/// The rail's signal list: the strongest distinct spectral peaks mapped to
/// `(freq_hz, dbfs)`, strongest-first. A thin wrapper over the spectrum panel's
/// [`detect_peaks`] (shared prominence ≥ NF+10 dB + min-separation logic) plus
/// the bin→Hz map, so HUNT and the MONITOR activity count agree with the markers.
fn rail_peaks(bins: &[f32], noise_floor: f32, center_hz: u64, sample_rate: f64, n: usize)
    -> Vec<(u64, f32)> {
    if sample_rate <= 0.0 || bins.is_empty() { return Vec::new(); }
    let len = bins.len();
    let sep = (len / 48).max(2);
    let left_hz = center_hz as f64 - sample_rate / 2.0;
    detect_peaks(bins, noise_floor, n, sep).into_iter().map(|i| {
        let hz = (left_hz + i as f64 / len as f64 * sample_rate).max(0.0) as u64;
        (hz, bins[i])
    }).collect()
}

/// BENCH gain-health verdict from ADC saturation and clip headroom. Returns the
/// word plus whether it's an alarm/warn/ok, so the caller picks the colour.
/// Pure for testability. `headroom_db` is `-channel_power_dbfs` (how far the
/// in-channel level sits below full scale).
fn chain_verdict(sat_pct: f32, headroom_db: f32) -> (&'static str, i8) {
    if sat_pct >= 10.0      { ("hot",     2) }   // clipping → back off gain
    else if headroom_db > 45.0 { ("low",  1) }   // lots of room → add gain
    else                    { ("optimal", 0) }
}

/// The mode-adaptive lead card that sits between the mode strip and the SIGNAL
/// zone. Only this block changes with the mode; everything below is fixed.
fn mode_card_lines(mode: RailMode, state: &SdrMetrics, stale: bool,
                   theme: &crate::Theme) -> Vec<Line<'static>> {
    let dim = |s: String| Line::from(vec![
        Span::raw(" "), Span::styled(s, Style::default().fg(theme.stale))]);

    match mode {
        // HUNT — the three strongest signals on screen, with band tags.
        RailMode::Hunt => {
            let fft = state.waterfall.last_fft.as_ref().filter(|_| !stale);
            let Some(fr) = fft else { return vec![dim("scanning…".into())]; };
            let peaks = rail_peaks(&fr.bins_dbfs, fr.noise_floor, state.radio.frequency, fr.sample_rate, 3);
            if peaks.is_empty() { return vec![dim("no peaks".into())]; }
            peaks.into_iter().enumerate().map(|(i, (hz, db))| {
                let mark = if i == 0 { "▸" } else { " " };
                let mut spans = vec![
                    Span::styled(mark.to_string(), Style::default().fg(theme.value_hi)),
                    Span::styled(format!("{:7.2}", hz as f64 / 1e6),
                                 Style::default().fg(if i == 0 { theme.value_hi } else { theme.value })),
                    Span::styled(format!(" {db:4.0}"), Style::default().fg(theme.label)),
                ];
                if let Some(b) = band_at(hz) {
                    spans.push(Span::styled(format!("  {b}"), Style::default().fg(theme.border_accent)));
                }
                Line::from(spans)
            }).collect()
        }

        // MONITOR — a calm watch headline: signal quality + how many signals are up.
        RailMode::Monitor => {
            let snr = state.signal.peak_to_nf_db;
            let (word, col) = if stale { ("—", theme.stale) }
                else if snr >= 20.0 { ("strong", theme.status_ok) }
                else if snr >= 10.0 { ("fair",   theme.value) }
                else                { ("quiet",  theme.label) };
            // detect_peaks already gates on NF+10 dB, so its count is the activity.
            let n_active = state.waterfall.last_fft.as_ref().filter(|_| !stale).map_or(0, |fr| {
                rail_peaks(&fr.bins_dbfs, fr.noise_floor, state.radio.frequency, fr.sample_rate, 8).len()
            });
            vec![
                Line::from(vec![
                    Span::raw(" "),
                    Span::styled("WATCH ", Style::default().fg(theme.label)),
                    Span::styled(word.to_string(), Style::default().fg(col).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::raw(" "),
                    Span::styled(format!("{n_active}"), Style::default().fg(theme.value).add_modifier(Modifier::BOLD)),
                    Span::styled(" active", Style::default().fg(theme.label)),
                ]),
            ]
        }

        // BENCH — gain-chain health: clip headroom + a one-word verdict.
        RailMode::Bench => {
            let power = state.signal.channel_power_dbfs;
            let headroom = if power.is_finite() { (-power).max(0.0) } else { f32::NAN };
            let sat = state.signal.adc_saturation_pct;
            let (verdict, sev) = chain_verdict(sat, if headroom.is_finite() { headroom } else { 0.0 });
            let vcol = match sev { 2 => theme.status_crit, 1 => theme.status_warn, _ => theme.status_ok };
            let hstr = if headroom.is_finite() { format!("{headroom:.0} dB") } else { "—".into() };
            vec![
                Line::from(vec![
                    Span::raw(" "),
                    Span::styled("HEADROOM ", Style::default().fg(theme.label)),
                    Span::styled(hstr, Style::default().fg(theme.value).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::raw(" "),
                    Span::styled("CHAIN ", Style::default().fg(theme.label)),
                    Span::styled(verdict.to_string(), Style::default().fg(vcol).add_modifier(Modifier::BOLD)),
                ]),
            ]
        }
    }
}

/// The RECALL list: the three saved-frequency slots, the one the radio is parked
/// on lit with `▸`. Empty slots show a dim dash. `M` saves the current tuning,
/// `1·2·3` jump (both in rail-focus). Band tags come from `band_at`.
fn recall_lines(state: &SdrMetrics, theme: &crate::Theme) -> Vec<Line<'static>> {
    let active = active_recall_slot(&state.ui.recall, state.radio.frequency);
    (0..RECALL_SLOTS).map(|i| {
        let n = i + 1;
        match state.ui.recall[i] {
            Some(hz) => {
                let on = active == Some(i);
                let mark = if on { "▸" } else { " " };
                let modi = if on { Modifier::BOLD } else { Modifier::empty() };
                let mut spans = vec![
                    Span::styled(mark.to_string(), Style::default().fg(theme.value_hi)),
                    Span::styled(format!("{n} "), Style::default().fg(if on { theme.value_hi } else { theme.label })),
                    Span::styled(vfo_string(hz),
                        Style::default().fg(if on { theme.value_hi } else { theme.value }).add_modifier(modi)),
                ];
                if let Some(b) = band_at(hz) {
                    spans.push(Span::styled(format!("  {b}"), Style::default().fg(theme.border_accent)));
                }
                Line::from(spans)
            }
            None => Line::from(vec![
                Span::raw(" "),
                Span::styled(format!("{n} "), Style::default().fg(theme.border_dim)),
                Span::styled("—", Style::default().fg(theme.stale)),
            ]),
        }
    }).collect()
}

/// The frequency hero: the big 3-row block readout, or a single bold line when
/// the rail is too narrow for the block font. The actively-tuned digit is lit in
/// `value_hi` (the same digit the small VFO underlines), the rest in `value`, the
/// decimal point dim — all dim in observer mode.
fn freq_hero_lines(freq: u64, step: u64, observer: bool, inner_w: usize,
                   theme: &crate::Theme) -> Vec<Line<'static>> {
    let s = vfo_string(freq);

    // Narrow fallback: the existing single-line segmented VFO (+" MHz"). The
    // budget covers the leading space (1) + the gap (1) + the "MHz" suffix (3),
    // so the big readout shows whenever its widest (middle) row actually fits.
    if bigdigits::big_width(&s) + 5 > inner_w {
        let col = if observer { theme.label } else { theme.value_hi };
        let mut spans = vec![Span::raw(" ")];
        spans.extend(vfo_spans(freq, step, col, theme.label, theme.value_hi));
        spans.push(Span::raw(" "));
        spans.push(Span::styled("MHz", Style::default().fg(theme.label)));
        return vec![Line::from(spans)];
    }

    let active = active_digit_idx(freq, step);
    let chars: Vec<char> = s.chars().collect();
    let mut rows: [Vec<Span<'static>>; 3] =
        [vec![Span::raw(" ")], vec![Span::raw(" ")], vec![Span::raw(" ")]];
    for (i, &c) in chars.iter().enumerate() {
        let color = if observer { theme.label }
            else if Some(i) == active { theme.value_hi }
            else if c == '.' { theme.label }
            else { theme.value };
        let g = bigdigits::glyph(c);
        for (r, row) in rows.iter_mut().enumerate() {
            if i > 0 { row.push(Span::raw(" ")); }
            row.push(Span::styled(g[r].to_string(), Style::default().fg(color)));
        }
    }
    // "MHz" rides the middle row, just past the digits.
    rows[1].push(Span::raw(" "));
    rows[1].push(Span::styled("MHz", Style::default().fg(theme.label)));
    let [r0, r1, r2] = rows;
    vec![Line::from(r0), Line::from(r1), Line::from(r2)]
}

/// `[FM]  SR 2.0M · RBW 1.5 kHz` — the band chip plus sample-rate / resolution
/// context, sitting just under the frequency hero.
fn band_sr_line(state: &SdrMetrics, iw: usize, theme: &crate::Theme) -> Line<'static> {
    let mut spans = vec![Span::raw(" ")];
    let mut used = 1;
    if let Some(b) = band_at(state.radio.frequency) {
        let chip = format!(" {b} ");
        used += chip.chars().count() + 2;
        spans.push(Span::styled(chip, Style::default()
            .fg(Color::Rgb(4, 6, 15)).bg(theme.value_hi).add_modifier(Modifier::BOLD)));
        spans.push(Span::raw("  "));
    }
    let sr = format!("SR {:.1}M", state.radio.config_sample_rate / 1_000_000.0);
    used += sr.chars().count();
    spans.push(Span::styled(sr, Style::default().fg(theme.label)));
    // RBW is the first thing to go on a narrow rail — drop it (and its separator)
    // rather than let it clip mid-word at the panel border.
    let rbw = match state.waterfall.last_fft.as_ref().filter(|fr| fr.enbw_hz > 0.0) {
        Some(fr) => fmt_rbw(fr.enbw_hz),
        None     => "—".to_string(),
    };
    let rbw_str = format!(" · RBW {rbw}");
    if used + rbw_str.chars().count() <= iw {
        spans.push(Span::styled(" · ", Style::default().fg(theme.border_dim)));
        spans.push(Span::styled(format!("RBW {rbw}"), Style::default().fg(theme.label)));
    }
    Line::from(spans)
}

impl Panel for CommandRailPanel {
    fn name(&self) -> &'static str { "command_rail" }
    fn min_size(&self) -> (u16, u16) { (22, 12) }

    // `c` for Command: focus the rail to drive it directly. In focus, `←/→` tune
    // (which auto-switches the mode to Hunt) and `Tab` cycles the mode manually.
    fn focus_key(&self) -> Option<char> { Some('c') }
    fn focus_bindings(&self) -> &'static [(&'static str, &'static str)] {
        &[("←→", "Tune"), ("1·2·3", "Recall"), ("M", "Save"), ("L", "Log")]
    }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let border = if focused { theme.border_focused } else { theme.border_dim };
        // Nameplate: COMMAND with the 'C' focus key highlighted (matches the
        // SPECTRUM/WATERFALL convention — the lit letter is the key that focuses it).
        let key_style  = Style::default().fg(theme.value_hi).add_modifier(Modifier::BOLD);
        let name_style = Style::default().fg(theme.label).add_modifier(Modifier::BOLD);
        let title_line = Line::from(chrome::nameplate(vec![
            Span::styled("C", key_style),
            Span::styled("OMMAND", name_style),
        ], border));
        let block = chrome::deck_block(border).title(title_line);
        let inner = block.inner(area);
        f.render_widget(block, area);
        chrome::corner_accents(f, area, border);
        if inner.width == 0 || inner.height == 0 { return; }

        let iw = inner.width as usize;
        let stale = fft_stale(state);
        let observer = state.observer.active;
        let active = state.radio.hw_streaming && !observer;

        // Width 6 so the 5-char "TOTAL" still keeps a gap before its value.
        let lbl   = |s: &str| Span::styled(format!("{s:<6}"), Style::default().fg(theme.label));
        // Dim `╴SECTION╶` divider, matching the deck nameplate language.
        let section = |name: &str| Line::from(chrome::nameplate(
            vec![chrome::label(name, theme.label)], theme.border_dim));

        let mut lines: Vec<Line> = Vec::new();

        // --- FREQ HERO ---------------------------------------------------------
        lines.extend(freq_hero_lines(state.radio.frequency, state.spectrum.step_hz,
                                     observer, iw, theme));
        lines.push(band_sr_line(state, iw, theme));
        lines.push(Line::raw(""));

        // --- MODE STRIP + lead card -------------------------------------------
        // The mode auto-follows actions (tune→Hunt, gain→Bench) and decays to
        // Monitor; the lead card below adapts to it. Everything under it is fixed.
        let mode = state.ui.effective_rail_mode();
        lines.push(mode_tabs_line(mode, iw, theme));
        lines.extend(mode_card_lines(mode, state, stale, theme));
        lines.push(Line::raw(""));

        // --- RECALL ------------------------------------------------------------
        lines.push(section("Recall"));
        lines.extend(recall_lines(state, theme));
        lines.push(Line::raw(""));

        // --- SIGNAL ------------------------------------------------------------
        // Each metric: value-first, with an inline sparkline of its recent trend
        // and a meaning-coloured arrow. Sparkline width scales with the rail.
        lines.push(section("Signal"));
        let sw = (iw / 4).clamp(5, 9);
        let spk = |h: &VecDeque<f32>| {
            let v: Vec<f32> = h.iter().copied().collect();
            sparkline(&v, sw)
        };

        let snr = state.signal.peak_to_nf_db;
        lines.extend(metric_block(
            "SNR", "dB",
            (!stale).then(|| format!("{snr:.1}")),
            snr_color(snr, theme),
            &spk(&state.signal.snr_history),
            trend_arrow(series_delta(&state.signal.snr_history), 0.3, Some(true), theme),
            iw, theme));

        let pwr = state.signal.channel_power_dbfs;
        lines.extend(metric_block(
            "PWR", "dBFS",
            (!stale && pwr.is_finite()).then(|| format!("{pwr:.1}")),
            theme.value,
            &spk(&state.signal.pwr_history),
            trend_arrow(series_delta(&state.signal.pwr_history), 0.5, None, theme),
            iw, theme));

        let nf = state.waterfall.last_fft.as_ref().filter(|_| !stale).map(|fr| fr.noise_floor);
        lines.extend(metric_block(
            "NF", "dBFS",
            nf.map(|v| format!("{v:.1}")),
            theme.value,
            &spk(&state.signal.nf_history),
            trend_arrow(series_delta(&state.signal.nf_history), 0.3, Some(false), theme),
            iw, theme));

        let sat = state.signal.adc_saturation_pct;
        lines.extend(metric_block(
            "SAT", "%",
            active.then(|| format!("{sat:.1}")),
            sat_color(sat, theme),
            &spk(&state.signal.saturation_history),
            trend_arrow(series_delta(&state.signal.saturation_history), 0.5, Some(false), theme),
            iw, theme));
        // Alert-memory: a recent clip leaves a fading "⚠ last clip Xs" line under
        // SAT — loud while fresh, then dim, then gone. It only ever fades.
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs()).unwrap_or(0);
        if let Some((since, fresh)) = clip_alert(state.signal.last_clip_at, now) {
            let col = if fresh { theme.status_crit } else { theme.stale };
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled(format!("⚠ last clip {}", fmt_since(since)), Style::default().fg(col)),
            ]));
        }
        lines.push(Line::raw(""));

        // --- GAIN --------------------------------------------------------------
        lines.push(section("Gain"));
        let gm = &state.caps.gain;
        let bw = gain_bar_width(iw);
        let val_col = if active { theme.value } else { theme.label };
        // Front-end boost (AMP / AGC).
        let (boost_val, boost_col) = if observer { ("—".to_string(), theme.label) }
            else if state.radio.amp_enabled { ("ON".to_string(), theme.value_hi) }
            else { ("OFF".to_string(), theme.label) };
        lines.push(Line::from(vec![
            Span::raw(" "), lbl(gm.boost_label()),
            Span::styled(boost_val, Style::default().fg(boost_col)),
        ]));
        // Primary stage (LNA / Tuner).
        let (p_f, p_e) = gain_bar(state.radio.lna_gain, gm.primary_max_db(), bw);
        lines.push(Line::from(vec![
            Span::raw(" "), lbl(gm.primary_label()),
            Span::styled(p_f, Style::default().fg(if active { theme.status_ok } else { theme.label })),
            Span::styled(p_e, Style::default().fg(theme.border_dim)),
            Span::raw(" "),
            Span::styled(format!("{:2}", state.radio.lna_gain), Style::default().fg(val_col)),
        ]));
        // Secondary stage (HackRF VGA only).
        if gm.has_second_stage() {
            let (v_f, v_e) = gain_bar(state.radio.vga_gain, 62, bw);
            lines.push(Line::from(vec![
                Span::raw(" "), lbl("VGA"),
                Span::styled(v_f, Style::default().fg(if active { theme.status_warn } else { theme.label })),
                Span::styled(v_e, Style::default().fg(theme.border_dim)),
                Span::raw(" "),
                Span::styled(format!("{:2}", state.radio.vga_gain), Style::default().fg(val_col)),
            ]));
        }
        let total = total_gain(state.radio.lna_gain, state.radio.vga_gain, gm.has_second_stage());
        lines.push(Line::from(vec![
            Span::raw(" "), lbl("TOTAL"),
            Span::styled(format!("{total} dB"), Style::default().fg(theme.value)),
        ]));
        lines.push(Line::raw(""));

        // --- STREAM ------------------------------------------------------------
        lines.push(section("Stream"));
        let stream_val = |s: String| Span::styled(s, Style::default().fg(if active { theme.value } else { theme.label }));
        lines.push(Line::from(vec![Span::raw(" "), lbl("DROP"),
            stream_val(format!("{} /s", state.signal.drops_per_sec))]));
        lines.push(Line::from(vec![Span::raw(" "), lbl("BUF"),
            stream_val(format!("{:.0} %", state.iq.buf_fill_pct))]));
        lines.push(Line::from(vec![Span::raw(" "), lbl("USB"),
            stream_val(fmt_mb(if active { state.radio.current_throughput_bps } else { 0 }))]));

        // Split off the bottom inner row for the log foot so the stack and the
        // foot never overlap (no flicker), and the foot stays anchored.
        let (stack_area, foot_area) = if inner.height >= 4 {
            (Rect { height: inner.height - 1, ..inner },
             Some(Rect { x: inner.x, y: inner.y + inner.height - 1, width: inner.width, height: 1 }))
        } else {
            (inner, None)
        };
        // Self-adjusting density: on a short rail where the airy layout would
        // overflow (and clip a whole section), drop the blank section spacers so
        // every section still shows. Tall rails keep the breathing room. The
        // `╴SECTION╶` nameplates carry the separation either way.
        if lines.len() > stack_area.height as usize {
            lines.retain(|l| l.spans.iter().any(|s| !s.content.trim().is_empty()));
        }
        f.render_widget(Paragraph::new(lines), stack_area);

        if let Some(foot) = foot_area {
            if let Some(e) = state.ui.log.back() {
                let foot_line = Line::from(vec![
                    Span::raw(" "),
                    log::lamp(e.level, theme),
                    Span::raw(" "),
                    Span::styled(log::fmt_clock(e.at_epoch_secs), Style::default().fg(theme.border_dim)),
                    Span::raw(" "),
                    Span::styled(e.text.as_ref(), Style::default().fg(theme.value)),
                ]);
                f.render_widget(Paragraph::new(foot_line), foot);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    #[test]
    fn total_gain_sums_only_with_second_stage() {
        assert_eq!(total_gain(32, 30, true), 62);  // HackRF LNA+VGA
        assert_eq!(total_gain(40, 99, false), 40); // RTL-SDR tuner only
    }

    #[test]
    fn fmt_mb_blanks_when_idle() {
        assert_eq!(fmt_mb(0), "—");
        assert_eq!(fmt_mb(5_200_000), "5.2 MB/s");
    }

    #[test]
    fn gain_bar_width_clamps() {
        assert_eq!(gain_bar_width(10), 4);   // tiny rail → floor
        assert_eq!(gain_bar_width(0), 4);
        assert_eq!(gain_bar_width(22), 12);  // wide rail → ceiling
        assert_eq!(gain_bar_width(18), 8);   // mid → 18-10
    }

    #[test]
    fn trend_arrow_colours_by_meaning() {
        let t = Theme::sdr();
        assert!(trend_arrow(None, 0.3, Some(true), &t).is_none());
        // rising-is-good (SNR): up → ok, down → warn
        assert_eq!(trend_arrow(Some(1.0), 0.3, Some(true), &t).unwrap().style.fg, Some(t.status_ok));
        assert_eq!(trend_arrow(Some(-1.0), 0.3, Some(true), &t).unwrap().style.fg, Some(t.status_warn));
        // rising-is-bad (NF/SAT): up → warn
        assert_eq!(trend_arrow(Some(1.0), 0.3, Some(false), &t).unwrap().style.fg, Some(t.status_warn));
        // neutral (PWR) and within-eps → dim steady
        assert_eq!(trend_arrow(Some(1.0), 0.3, None, &t).unwrap().style.fg, Some(t.stale));
        assert_eq!(trend_arrow(Some(0.0), 0.3, Some(true), &t).unwrap().style.fg, Some(t.stale));
    }

    #[test]
    fn series_delta_needs_four_samples() {
        let mut h: VecDeque<f32> = VecDeque::new();
        h.extend([10.0, 10.0, 20.0]);
        assert_eq!(series_delta(&h), None);
        h.push_back(20.0); // older half [10,10]=10, recent half [20,20]=20 → +10
        assert!((series_delta(&h).unwrap() - 10.0).abs() < 1e-6);
    }

    #[test]
    fn sat_color_escalates() {
        let t = Theme::sdr();
        assert_eq!(sat_color(0.0, &t), t.value);
        assert_eq!(sat_color(20.0, &t), t.status_warn);
        assert_eq!(sat_color(80.0, &t), t.status_crit);
    }

    #[test]
    fn rail_peaks_maps_bins_to_frequency_strongest_first() {
        // Two lobes above the −80 dB noise floor: a tall one left of centre
        // (bin 2), a shorter one right (bin 6). 10 Msps @ 100 MHz → 95..105 MHz.
        let bins = [-90.0, -40.0, -10.0, -40.0, -80.0, -50.0, -25.0, -50.0, -90.0];
        let peaks = rail_peaks(&bins, -80.0, 100_000_000, 10_000_000.0, 3);
        assert_eq!(peaks.len(), 2, "two distinct lobes above NF+10");
        assert!(peaks[0].1 > peaks[1].1, "strongest first");
        assert!((peaks[0].1 - (-10.0)).abs() < 1e-3);
        // Bin 2 of 9 maps below centre; bin 6 above it.
        assert!(peaks[0].0 < 100_000_000 && peaks[1].0 > 100_000_000);
    }

    #[test]
    fn rail_peaks_empty_without_signal_or_rate() {
        // All near the floor → nothing clears NF+10 dB.
        assert!(rail_peaks(&[-90.0, -88.0, -90.0], -90.0, 100_000_000, 6_000_000.0, 3).is_empty());
        // No sample rate → no usable frequency map.
        assert!(rail_peaks(&[-90.0, -10.0, -90.0], -90.0, 100_000_000, 0.0, 3).is_empty());
    }

    #[test]
    fn fmt_since_scales_units() {
        assert_eq!(fmt_since(4), "4s");
        assert_eq!(fmt_since(59), "59s");
        assert_eq!(fmt_since(120), "2m");
        assert_eq!(fmt_since(7200), "2h");
    }

    #[test]
    fn clip_alert_is_fresh_then_fades_then_expires() {
        assert_eq!(clip_alert(None, 100), None);                 // never clipped
        assert_eq!(clip_alert(Some(100), 103), Some((3, true))); // fresh & loud
        assert_eq!(clip_alert(Some(100), 115), Some((15, false))); // remembered, dim
        assert_eq!(clip_alert(Some(100), 140), None);            // older than memory
        // Clock skew (clip "in the future") must not panic or misread.
        assert_eq!(clip_alert(Some(100), 90), Some((0, true)));
    }

    #[test]
    fn mode_tabs_full_width_is_the_label_budget() {
        // " HUNT " + gap + " MONITOR " + gap + " BENCH " + gap, plus leading space.
        // (4+3) + (7+3) + (5+3) + 1 = 26.
        assert_eq!(mode_tabs_full_w(), 26);
        // Compact kicks in below that — the strip then uses 3-letter codes.
        assert!(mode_tabs_full_w() > 20, "narrow rail must compact");
        assert!(mode_tabs_full_w() <= 28, "wide rail shows full labels");
    }

    #[test]
    fn chain_verdict_reads_saturation_and_headroom() {
        assert_eq!(chain_verdict(20.0, 10.0).0, "hot");      // clipping wins
        assert_eq!(chain_verdict(0.0, 60.0),  ("low", 1));   // lots of headroom
        assert_eq!(chain_verdict(0.0, 20.0),  ("optimal", 0));
    }
}
