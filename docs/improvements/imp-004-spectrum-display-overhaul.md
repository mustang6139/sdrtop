# IMP-004 — Spectrum display overhaul

← [Home](../Home.md)

**Added:** 2026-05-30  
**Between phases:** 12 → 13

---

## Why

Three visual problems remained in the spectrum panel after IMP-003:

1. **Spectrum bounced** — the auto-zoom recalculated y-bounds every frame from the current signal peak. Small peak variations caused the entire spectrum line to jump up/down with every render cycle.

2. **Spectrum floated** — with a fixed `-120…0 dBFS` scale, the polyline-only rendering (outline of bin tops) left the area below the signal empty. A signal at −60 dBFS appeared as a line floating in the middle of the panel with nothing below it, disconnected from the panel bottom.

3. **Focus key not discoverable** — there was no visual hint that `[E]` focused the spectrum panel. The key only appeared in the help overlay.

4. **Preset key order was unintuitive** — `2` launched `monitoring` (a diagnostic preset), while the main visual panels `spectrum` and `waterfall` were on `3` and `4`.

---

## What changed

| File | Change |
|---|---|
| `src/ui/spectrum.rs` | Fixed y-bounds; filled columns; highlighted title key |
| `src/app.rs` | Preset key reorder |
| `src/ui/overlay.rs` | Updated help text for new preset order |

---

## Fixed y-range

**Before** — y-bounds recalculated per frame from `sig_peak`:
```rust
let y_max_f = (sig_peak + 15.0).min(DB_MAX);
let y_min_f = (sig_peak - 50.0).max(DB_MIN);
```
Every frame the canvas rescaled, causing visible bounce.

**After** — fixed absolute bounds:
```rust
let y_min_f = DB_MIN;  // -120.0
let y_max_f = DB_MAX;  //   0.0
```
The spectrum is always anchored to the same absolute dBFS scale. Signal strength is read directly from position: a signal at −40 dBFS sits at 1/3 from the top regardless of other signals in the band.

---

## Filled columns

**Before** — polyline only (outline of bin tops):
```
        ∧
       / \
──────/   \──────────────         ← signal at -60 dBFS, middle of panel
                                  ← lower half of panel empty
```

**After** — vertical fill per bin + outline on top:
```
        ∧
       /█\
──────/███\──────────────
     ████████████████████         ← fills down to panel bottom
```

Each bin draws a vertical `CanvasLine` from `DB_MIN` (bottom) to its value, then the outline polyline is drawn on top for a clean signal edge. The spectrum is visually grounded.

---

## Focus key highlight in title

The `e` in `Spectrum` is rendered in `theme.value_hi + BOLD`, making the focus shortcut self-documenting without cluttering the UI:

```
 Sp[e]ctrum           ← e in bright accent color, rest in normal title style
 Sp[e]ctrum [STALE]   ← stale state appended, e still highlighted
```

---

## Preset key reorder

| Key | Before | After |
|---|---|---|
| `1` | main | main (unchanged) |
| `2` | monitoring | **spectrum** |
| `3` | spectrum | **waterfall** |
| `4` | waterfall | **spectrum+waterfall** |
| `5` | spectrum+waterfall | monitoring |
| `6` | lab | lab (unchanged) |

The three most-used visual presets (spectrum, waterfall, spectrum+waterfall) are now on keys 2–4, matching their natural order of complexity.

---

## Focus system simplification

All panels except spectrum had their `focus_key()` and `focus_bindings()` overrides removed. The `Panel` trait default (`None`) applies to them. Only the spectrum panel participates in the focus system. This removes six unused key bindings (`o h c m i g`) and the corresponding footer hints.
