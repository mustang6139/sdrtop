# IMP-005 — Spectrum focus: frequency tuning + step control

← [Home](../Home.md)

**Added:** 2026-05-30  
**Between phases:** 12 → 13

---

## Why

The spectrum panel in focus mode (`[E]`) had no interactive function beyond visual emphasis. The natural use of a focused spectrum view is to tune the center frequency incrementally — scanning a band manually, hunting a signal, or fine-tuning onto a carrier. Adding keyboard-driven frequency navigation directly in focus mode makes this workflow available without leaving the spectrum view.

---

## What changed

| File | Change |
|---|---|
| `src/state.rs` | Added `spectrum_step_hz: u64` field (default 100 kHz) |
| `src/app.rs` | `SPECTRUM_STEPS` const; `prev/next_spectrum_step()`; `fmt_spectrum_step()`; Left/Right/[/] key handlers gated on spectrum focus |
| `src/ui/spectrum.rs` | Tuning indicator row (focus only); updated `focus_bindings()` |

---

## Key bindings (spectrum focus mode)

| Key | Action |
|---|---|
| `←` | Center frequency − step |
| `→` | Center frequency + step |
| `[` | Step size decrease |
| `]` | Step size increase |
| `Esc` | Exit focus mode |

When spectrum focus is active, `[` and `]` intercept the global VGA gain handlers. Outside focus mode they revert to VGA gain control as before.

---

## Step sizes

Nine presets, cycling with `[`/`]`:

| Step | Use case |
|---|---|
| 1 kHz | Fine carrier alignment |
| 5 kHz | NFM channel grid |
| 10 kHz | AM broadcast channel grid |
| 25 kHz | NFM / PMR channel grid |
| **100 kHz** | **Default — general scanning** |
| 500 kHz | Fast band scan |
| 1 MHz | Broad survey |
| 5 MHz | Cross-band jump |
| 10 MHz | HackRF sample-rate-sized hops |

---

## Tuning indicator

A one-row indicator appears at the bottom of the spectrum canvas when focused, replacing no existing content (the canvas shrinks by one row to accommodate it):

```
────────────────────◀  92.800 MHz  ▶────────────────────  step 100 kHz  [/]
```

| Element | Style | Meaning |
|---|---|---|
| `────` arms | `border_dim` | visual framing, proportional to available width |
| `◀` / `▶` | `border_accent` + bold | press `←`/`→` to tune in that direction |
| `92.800 MHz` | `value_hi` + bold | current center frequency |
| `step 100 kHz` | `label` | current step size |
| `[/]` | `label` | key hint for step adjustment |

The indicator is absent when focus mode is inactive — it does not consume space in normal view.

---

## Frequency bounds

Tuning is clamped to the HackRF's hardware range:

| Bound | Value |
|---|---|
| Minimum | 1 MHz |
| Maximum | 6 000 MHz |

Attempts to tune past either bound are silently ignored (saturating arithmetic).
