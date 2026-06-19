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
