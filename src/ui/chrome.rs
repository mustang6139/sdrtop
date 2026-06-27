//! Shared "schematic deck" chrome — one frame language for every panel.
//!
//! Square (Plain) borders with a tick-tab nameplate on the top rule:
//! `┌╴LABEL╶─────┐`. This reads as precision field-instrument hardware rather
//! than a soft rounded window — without touching the colour palette.
//!
//! Panels build their own title spans (focus-key highlight, live state tags)
//! and wrap the name with [`nameplate`]; static panels use [`title`] directly.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

/// A panel frame in the schematic deck language: square corners, single rule.
pub fn deck_block<'a>(border_color: Color) -> Block<'a> {
    deck_block_borders(border_color, Borders::ALL)
}

/// Like [`deck_block`] but with an explicit border set — used when two panels
/// bond into one instrument and the facing edge is dropped (e.g. the spectrum
/// renders `TOP | LEFT | RIGHT`, letting the waterfall's top border below it act
/// as the shared frequency ruler).
pub fn deck_block_borders<'a>(border_color: Color, borders: Borders) -> Block<'a> {
    Block::default()
        .borders(borders)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(border_color))
}

/// Overlay only the top reinforced corners (`┏┓`) — for a panel bonded below a
/// neighbour, which has no bottom border to anchor `┗┛`.
pub fn corner_accents_top(f: &mut Frame, area: Rect, color: Color) {
    if area.width < 2 || area.height < 1 { return; }
    let style = Style::default().fg(color);
    let r = area.x + area.width - 1;
    f.render_widget(Paragraph::new(Span::styled("\u{250F}", style)),
                    Rect { x: area.x, y: area.y, width: 1, height: 1 }); // ┏
    f.render_widget(Paragraph::new(Span::styled("\u{2513}", style)),
                    Rect { x: r, y: area.y, width: 1, height: 1 });      // ┓
}

/// Overlay `├` / `┤` T-junctions on the top corners of `area`, in `color`. Used
/// at a bonded boundary so the shared-ruler row ties into the continuous side
/// borders of the panel above instead of reading as a separate box's `┌`/`┐`.
pub fn junction_caps(f: &mut Frame, area: Rect, color: Color) {
    if area.width < 2 { return; }
    let style = Style::default().fg(color);
    let r = area.x + area.width - 1;
    f.render_widget(Paragraph::new(Span::styled("\u{251C}", style)),
                    Rect { x: area.x, y: area.y, width: 1, height: 1 }); // ├
    f.render_widget(Paragraph::new(Span::styled("\u{2524}", style)),
                    Rect { x: r, y: area.y, width: 1, height: 1 });      // ┤
}

/// Overlay reinforced "bracket" corners on an already-rendered panel frame, in
/// the panel's own border colour. The heavier corner glyphs (`┏┓┗┛`) against the
/// light edges read as fastened instrument-panel corners — a schematic-deck
/// detail that adds structure without touching the colour palette. Call right
/// after rendering the block. No-op for frames too small to have real corners.
pub fn corner_accents(f: &mut Frame, area: Rect, color: Color) {
    if area.width < 2 || area.height < 2 { return; }
    let style = Style::default().fg(color);
    let (l, t) = (area.x, area.y);
    let (r, b) = (area.x + area.width - 1, area.y + area.height - 1);
    for (x, y, ch) in [
        (l, t, "\u{250F}"), // ┏
        (r, t, "\u{2513}"), // ┓
        (l, b, "\u{2517}"), // ┗
        (r, b, "\u{251B}"), // ┛
    ] {
        f.render_widget(Paragraph::new(Span::styled(ch, style)),
                        Rect { x, y, width: 1, height: 1 });
    }
}

/// Wrap nameplate label spans with tick end-caps: `╴…╶`. The caller may append
/// live state tags after the returned spans before building the title `Line`.
pub fn nameplate<'a>(label_spans: Vec<Span<'a>>, tick_color: Color) -> Vec<Span<'a>> {
    let mut spans = Vec::with_capacity(label_spans.len() + 2);
    spans.push(Span::styled("╴", Style::default().fg(tick_color)));
    spans.extend(label_spans);
    spans.push(Span::styled("╶", Style::default().fg(tick_color)));
    spans
}

/// A single uppercase nameplate label span (no focus key) in `color`.
pub fn label<'a>(text: &str, color: Color) -> Span<'a> {
    Span::styled(
        text.to_uppercase(),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
}

/// A complete nameplate title `Line` for a static label: `╴LABEL╶`.
pub fn title<'a>(text: &str, label_color: Color, tick_color: Color) -> Line<'a> {
    Line::from(nameplate(vec![label(text, label_color)], tick_color))
}

/// Which blank-spacer indices to drop so an airy stack of `total` lines fits
/// `avail` rows. `blank_idx` lists the indices (into the full line list) of the
/// droppable spacer rows, in order.
///
/// When the overflow meets or exceeds the whole spacer budget every spacer goes
/// (true dense). Otherwise only as many as needed are removed, picked evenly
/// across the spacer list so the surviving breathing room stays balanced —
/// instead of the all-or-nothing cliff that, at in-between heights, collapsed a
/// panel to fully dense and stranded a block of blank rows at its foot.
pub fn spacers_to_drop(total: usize, blank_idx: &[usize], avail: usize) -> Vec<usize> {
    if total <= avail { return Vec::new(); }
    let excess = total - avail;
    if excess >= blank_idx.len() { return blank_idx.to_vec(); }
    (0..excess).map(|k| blank_idx[k * blank_idx.len() / excess]).collect()
}

/// Fit an airy `lines` stack into `avail` rows in place: drop only as many blank
/// spacer rows as needed — evenly across the stack — so a panel keeps as much
/// breathing room as fits rather than snapping to fully dense and stranding empty
/// rows. A blank row is one whose spans are all whitespace. No-op when it already
/// fits. The shared self-adjusting-density routine for every airy-stack panel.
pub fn collapse_spacers(lines: &mut Vec<Line<'_>>, avail: usize) {
    if lines.len() <= avail { return; }
    let blank_idx: Vec<usize> = lines.iter().enumerate()
        .filter(|(_, l)| l.spans.iter().all(|s| s.content.trim().is_empty()))
        .map(|(i, _)| i)
        .collect();
    let drop: std::collections::HashSet<usize> =
        spacers_to_drop(lines.len(), &blank_idx, avail).into_iter().collect();
    if drop.is_empty() { return; }
    let mut i = 0usize;
    lines.retain(|_| { let keep = !drop.contains(&i); i += 1; keep });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spacers_to_drop_keeps_all_when_it_fits() {
        let blanks = vec![3, 5, 7, 9];
        assert!(spacers_to_drop(20, &blanks, 20).is_empty(), "exact fit drops nothing");
        assert!(spacers_to_drop(18, &blanks, 20).is_empty(), "room to spare drops nothing");
    }

    #[test]
    fn spacers_to_drop_drops_all_when_overflow_exceeds_budget() {
        let blanks = vec![3, 5, 7, 9];
        // Overflow of 6 but only 4 spacers — every spacer must go (true dense).
        assert_eq!(spacers_to_drop(30, &blanks, 24), blanks);
        // Overflow exactly equal to the spacer count also clears them all.
        assert_eq!(spacers_to_drop(28, &blanks, 24), blanks);
    }

    #[test]
    fn spacers_to_drop_removes_only_excess_spread_evenly() {
        let blanks = vec![3, 5, 7, 9, 11, 13]; // 6 spacers
        // Overflow of 2 → drop 2 spacers, spread across the list (not the first two).
        let drop = spacers_to_drop(20, &blanks, 18);
        assert_eq!(drop, vec![3, 9], "evenly spaced: 1st and 4th spacer");
    }

    #[test]
    fn spacers_to_drop_indices_distinct_and_sized() {
        let blanks: Vec<usize> = (0..13).collect();
        for excess in 1..13 {
            let drop = spacers_to_drop(40, &blanks, 40 - excess);
            let unique: std::collections::HashSet<_> = drop.iter().collect();
            assert_eq!(drop.len(), unique.len(), "excess={excess}: no repeated index");
            assert_eq!(drop.len(), excess, "excess={excess}: drops exactly `excess`");
        }
    }

    #[test]
    fn collapse_spacers_drops_only_excess_in_place() {
        // 3 content rows interleaved with 3 spacers (6 lines); fit into 5 → drop 1.
        let mk = |s: &str| Line::from(Span::raw(s.to_string()));
        let blank = || Line::from(Span::raw("   ".to_string()));
        let mut lines = vec![mk("a"), blank(), mk("b"), blank(), mk("c"), blank()];
        collapse_spacers(&mut lines, 5);
        assert_eq!(lines.len(), 5, "exactly one spacer removed");
        // All three content rows survive.
        let content: Vec<String> = lines.iter()
            .filter(|l| !l.spans.iter().all(|s| s.content.trim().is_empty()))
            .map(|l| l.spans[0].content.to_string())
            .collect();
        assert_eq!(content, vec!["a", "b", "c"]);
    }

    #[test]
    fn collapse_spacers_noop_when_it_fits() {
        let mk = |s: &str| Line::from(Span::raw(s.to_string()));
        let mut lines = vec![mk("a"), mk("b")];
        collapse_spacers(&mut lines, 10);
        assert_eq!(lines.len(), 2);
    }
}
