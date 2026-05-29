# IMP-003 — Spectrum & Waterfall UI Fixes

← [Home](../Home.md)

**Added:** 2026-05-29  
**Between phases:** 12 → 12 (post-phase polish)

---

## Why

Three visual problems were found after the main dashboard overhaul:

1. **Spectrum border misalignment** — the panel was built from three separate blocks (dB label column, canvas, freq row) each with their own partial borders. This produced two `╭` corners at the top-left, an unclosed bottom-right edge, and freq labels that floated without a bottom border.

2. **Freq label positions wrong** — the five frequency tick labels used a hard-coded `{:<12}` width, which placed them at chars 0, 12, 24, 36, 48 regardless of the actual canvas width. On a typical 130+ char terminal the labels were bunched on the left while the spectrum extended to the right edge.

3. **Spectrum and waterfall x-axes misaligned** — the spectrum canvas started 6 chars to the right of the panel's left border (dB label column). The waterfall spanned the full inner width. The same frequency mapped to a different horizontal pixel in each panel.

---

## What changed

| File | Change |
|---|---|
| `src/ui/spectrum.rs` | Single outer block with `Borders::ALL`; inner area split for dB column and canvas+freq. dB column uses a `Borders::RIGHT` block as a divider. Freq labels computed proportionally from `canvas_area.width`. Spectrum paint changed from filled vertical bars to a polyline (adjacent bin tops connected). |
| `src/ui/waterfall.rs` | Canvas offset by `DB_COL = 6` chars to align with spectrum. Left 6-char column filled with a dBFS color scale legend (gradient `█` strip + labels at +0 / −60 / −120 dBFS). |

---

## Spectrum border fix

**Before** — three independent blocks assembled side-by-side:
```
╭─────╭────────────────────────────╮   ← two ╭ corners
│ +0  │                            │
│-120 │                            ╯   ← right side unclosed
      freq  freq  freq  freq           ← floats without bottom border
```

**After** — one outer `Block::ALL` + `block.inner()` for sub-layout:
```
╭──────────────────────────────────╮
│  +0 │                            │
│ -60 │         spectrum           │
│-120 │                            │
│      freq   freq   freq   freq   │
╰──────────────────────────────────╯
```

The `│` divider between the dB column and the canvas comes from a `Borders::RIGHT` block rendered on the dB sub-area (height = canvas height, not full inner height, so it stops above the freq row).

---

## Freq label alignment fix

Old format string:
```rust
format!("{:<12}{:<12}{:<12}{:<12}{}", ...)
```
Labels were fixed at char positions 0, 12, 24, 36, 48 — correct only for a 48-char canvas.

New approach:
```rust
let seg = (canvas_area.width as usize - max_label_len) / 4;
format!("{:<w$}{:<w$}{:<w$}{:<w$}{}", ..., w = seg)
```
Each segment width scales with the actual canvas, so the five labels land at 0%, 25%, 50%, 75%, ~100% of the canvas width.

---

## Spectrum outline fix

**Before** — each bin rendered as a vertical `CanvasLine` from `DB_MIN` to its current value (filled braille bars):
```rust
ctx.draw(&CanvasLine { x1: i, y1: DB_MIN, x2: i, y2: db, color });
```

**After** — adjacent bin tops connected as a polyline:
```rust
for i in 1..bins.len() {
    ctx.draw(&CanvasLine { x1: i-1, y1: bins[i-1], x2: i, y2: bins[i], color: bin_colors[i-1] });
}
```

Result: a clean gradient outline instead of a dense filled waterfall. Peak hold markers and noise floor line are unchanged.

---

## Waterfall x-axis alignment

The spectrum canvas and the waterfall must start at the same terminal column so the same frequency lands at the same horizontal position in both panels.

```rust
const DB_COL: u16 = 6;  // must match spectrum.rs dB column width
let wf_area = Rect {
    x: inner.x + DB_COL,
    width: inner.width.saturating_sub(DB_COL),
    ..inner
};
```

The freed left column is used for the dBFS legend (see below).

---

## Waterfall dBFS legend

The 6-char left column of the waterfall now shows the color-to-dBFS mapping:

```
█  +0
█
█  -60
█
█-120
```

Each row's `█` is rendered in `magnitude_to_color_themed(db, ...)` where `db` maps linearly from `DB_MAX` (top) to `DB_MIN` (bottom). Labels appear at rows 0, `h/2`, and `h-1`. This makes the waterfall self-documenting without adding a separate legend panel.

---

## Files touched

| File | Change |
|---|---|
| `src/ui/spectrum.rs` | Border redesign, proportional freq labels, polyline paint |
| `src/ui/waterfall.rs` | x-axis offset, dBFS legend in left column |
