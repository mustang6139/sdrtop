# IMP-009 — Waterfall focus panel

← [Home](../Home.md)

**Added:** 2026-05-30  
**Between phases:** 12 → 13

---

## Why

The waterfall panel was a passive, read-only display — it showed signal history but offered no way to interact with it. For SDR work the waterfall is often the primary tool for spotting signals (WSPR transmissions, slow drifters, brief bursts), so the inability to investigate what you just saw was a real gap. This improvement adds a full focus mode to the waterfall panel, matching the interaction model already established by the spectrum panel (IMP-005, IMP-006).

Additionally, the `spawn_sys_resource_task` CPU measurement had been reporting inflated values: `last_ticks` started at `0`, so the first measurement included all CPU ticks accumulated since process start rather than just the last second. Separately, the FFT worker was executing the full analysis pipeline (noise floor sort, occupied-BW sort, state lock) on every FFT frame — at 10 MHz sample rate that is ~4 882 times per second when only ~30 state updates per second are needed for display.

---

## What changed

| File | Change |
|---|---|
| `src/ui/band_plan.rs` | New shared module — `BAND_PLAN` constant (14 allocations); moved out of `spectrum.rs` |
| `src/ui/mod.rs` | `pub mod band_plan` registered |
| `src/ui/spectrum.rs` | Imports `BAND_PLAN` from `band_plan` instead of local const |
| `src/ui/waterfall.rs` | Full rewrite: `focus_key() → 'l'`; `focus_bindings()`; dynamic `db_min`; scroll offset; stride averaging; frequency cursor; indicator row; band plan overlay |
| `src/state.rs` | `WaterfallBuffer.rows` now `VecDeque<(Instant, Arc<Vec<f32>>)>`; added `row_stride`, private `acc_bins`/`acc_count`, `set_row_stride()`; `SdrMetrics`: `waterfall_db_min`, `waterfall_scroll_offset`, `waterfall_cursor_freq` |
| `src/app.rs` | `WF_STRIDES`, `prev/next_wf_stride()`; key handlers for waterfall focus; Esc clears scroll + cursor; `last_ticks` seeded from current value (CPU measurement fix) |
| `src/fft.rs` | State updates throttled to ~30 fps via `UPDATE_INTERVAL = 33 ms`; EMA smoothing still runs on every frame |

---

## Features

### Focus key: `l`

Press `l` to enter waterfall focus mode. The second `l` in `Waterfa`**`l`**`l` is rendered in `value_hi + BOLD` as the activation hint, matching the `e` in `Sp`**`e`**`ctrum`. The border switches to `border_focused`. Press `Esc` to exit; scroll offset and cursor are reset on exit.

---

### 1 — Colour scale zoom (`↑` / `↓`)

Adjusts `waterfall_db_min` while keeping `waterfall_db_max` fixed at 0 dBFS.

| Key | Effect |
|---|---|
| `↑` | Raise `db_min` by 10 dB — fewer weak signals visible, strong signals fill the range |
| `↓` | Lower `db_min` by 10 dB — more dynamic range, down to −120 dBFS |

Minimum visible range: 20 dBFS. The dBFS colour scale legend on the left updates dynamically to always reflect the current range. The setting persists after exiting focus.

---

### 2 — Scroll history (`J` / `K`)

| Key | Effect |
|---|---|
| `J` | Scroll toward older data (offset + 1) |
| `K` | Scroll toward newer data (offset − 1, minimum 0) |

The offset is clamped to `buf.rows.len() − rows_visible` so scrolling never goes past the available history. When any scroll offset is active the title shows `[↑N]` in `value_hi`:

```
 Waterfall [↑ 8] 
```

`Esc` resets the offset to 0.

---

### 3 — Temporal aggregation stride (`[` / `]`)

Controls how many FFT frames are averaged into each waterfall row.

| Key | Effect |
|---|---|
| `[` | Decrease stride — faster scroll, more time resolution |
| `]` | Increase stride — slower scroll, longer history on screen |

**Step presets:** 1 × 2 × 4 × 8 × 16 × 32 × 64 frames per row.

Averaging is performed in dBFS (sufficient for visual display). When stride > 1 the title shows `[×N]` in `label` colour. Changing stride resets the in-progress accumulator so there are no blending artifacts.

**Effect on displayed history depth** (example: 10 MHz, 2048 bins):

| Stride | Rows/sec pushed | 64-row buffer covers |
|---|---|---|
| ×1 (default) | ~150 | ~0.4 s |
| ×8 | ~19 | ~3.4 s |
| ×32 | ~5 | ~13 s |
| ×64 | ~2 | ~26 s |

The waterfall rows now store a push timestamp `(Instant, Arc<Vec<f32>>)` instead of a bare `Arc<Vec<f32>>`, enabling accurate "N seconds ago" readout in the cursor indicator.

---

### 4 — Frequency cursor (`M` / `←` / `→`)

A vertical `│` line at a chosen frequency.

| Key | Effect |
|---|---|
| `M` | Toggle cursor. First press places it at `state.frequency`; second press removes it |
| `←` | Move cursor left by `spectrum_step_hz` |
| `→` | Move cursor right by `spectrum_step_hz` |

The cursor column is mapped from `waterfall_cursor_freq` to display column in the renderer; it stays visually aligned when the panel is resized. The cell character is `│` with the signal's background colour preserved, so signal intensity remains readable through the cursor line.

When the cursor is active, the **indicator row** at the bottom of the panel shows:

```
──────────────  cur: 92.800 MHz  -42.3 dBFS  14s ago  ← →  M
```

- **MHz**: frequency of the cursor
- **dBFS**: signal level in the topmost visible row at the cursor column (reflects scroll position)
- **s ago**: wall-clock seconds since that row was pushed (from the stored `Instant`)

When the cursor is inactive the indicator shows:

```
──────────  ×1  frames/row  [ ]  M cursor  step 100 kHz  ↑↓ zoom  J/K scroll
```

---

### 5 — Band plan overlay

Identical logic to the spectrum panel's band plan (IMP-006). 14 built-in allocations are displayed as dim labels on the top row of the waterfall canvas when the visible frequency range overlaps:

| Label | Range |
|---|---|
| FM | 87.5 – 108 MHz |
| VOR/ILS | 108 – 118 MHz |
| AIR | 118 – 137 MHz |
| 2m | 144 – 146 MHz |
| Marine | 156 – 174 MHz |
| WX | 162.4 – 163.3 MHz |
| 70cm | 430 – 440 MHz |
| ISM433 | 433.05 – 434.79 MHz |
| PMR | 446.0 – 446.2 MHz |
| ISM868 | 868 – 869 MHz |
| GPS-L2 | 1227.6 MHz |
| GPS-L1 | 1575.42 MHz |
| CELL | 1710 – 2170 MHz |
| 2.4G | 2400 – 2483.5 MHz |

The constant was moved from `spectrum.rs` to the new shared `src/ui/band_plan.rs` module so both panels draw from the same source of truth.

---

## Key summary (waterfall focus)

| Key | Action |
|---|---|
| `↑` | Zoom in colour scale (+10 dB) |
| `↓` | Zoom out colour scale (−10 dB) |
| `J` | Scroll toward older data |
| `K` | Scroll toward newer data |
| `[` | Decrease row stride (faster) |
| `]` | Increase row stride (slower) |
| `M` | Place / remove frequency cursor |
| `←` | Move cursor left (by step) |
| `→` | Move cursor right (by step) |
| `W` | Pause / resume (global) |
| `Esc` | Exit focus, reset scroll + cursor |

---

## CPU measurement and FFT throttling fixes

Two related performance issues were fixed alongside this improvement.

### `last_ticks` initialisation bug

`spawn_sys_resource_task` started `last_ticks` at `0`. The first measurement subtracted zero from the total accumulated CPU ticks since process start, making the initial reading always 100% (clamped). Fix: seed `last_ticks` with `read_process_stats()` before the first sleep.

### FFT state update rate

At 10 MHz sample rate the FFT worker processed ~4 882 frames per second, running the full analysis pipeline (noise floor partial sort, occupied-BW sort, state mutex lock, `Arc::new` allocation) on every frame. Only ~30 state updates per second are needed for display. Fix: after EMA smoothing, skip the expensive analysis unless `UPDATE_INTERVAL = 33 ms` has elapsed since the last state write. EMA and peak-hold still run on every frame to preserve smoothing accuracy.
