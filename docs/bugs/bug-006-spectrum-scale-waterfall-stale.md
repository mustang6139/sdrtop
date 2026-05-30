# BUG-006 — Spectrum dBFS scale misalignment + waterfall stale not shown

← [Bug Tracker](README.md)

**Date:** 2026-05-30  
**Status:** ✅ Fixed

---

## BUG-006a — dBFS axis labels misaligned on resize

**Severity:** Rendering defect (consistent, reproducible on any non-default terminal height)

**Location:** `src/ui/spectrum.rs` — dBFS label rendering

**Symptom:** The dBFS scale on the left side of the spectrum panel showed correct values but they were bunched near the top instead of spanning the full canvas height. Resizing the terminal made the misalignment worse.

**Root cause:**

```rust
// BEFORE — 5 labels separated by \n, placed at character rows 0, 1, 2, 3, 4
let db_text: String = (0..=4)
    .map(|i| {
        let db = y_max_f - (y_max_f - y_min_f) * i as f32 / 4.0;
        format!("{:+4.0}\n", db)
    })
    .collect();
f.render_widget(Paragraph::new(db_text), db_rows[0]);
```

With a canvas of 20 character rows, the five labels landed at rows 0–4 (the top 20% of the canvas). At 8 rows they were still at rows 0–4 (50%). Only at exactly 4 rows were they correctly distributed. The `\n`-based approach always places consecutive labels one row apart, regardless of canvas height.

**Fix:** Build a `Vec<Line>` of length `h` (canvas height). For each label, compute the exact character row from its dB value:

```rust
let frac = (DB_MAX - db) / (DB_MAX - DB_MIN);
let row = (frac * h.saturating_sub(1) as f32).round() as usize;
label_lines[row.min(h - 1)] = Line::from(Span::styled(text, ...));
```

Labels now sit at rows proportional to `(DB_MAX - db) / (DB_MAX - DB_MIN) * h`, staying aligned with the canvas at any terminal height.

---

## BUG-006b — Waterfall border stays active (not dimmed) after RX stops

**Severity:** Misleading UI — stale data shown as live

**Location:** `src/ui/waterfall.rs` — stale detection

**Symptom:** When RX was stopped with `[Space]`, the spectrum panel correctly dimmed its border and showed `[STALE]` in the title. The waterfall panel kept its full-brightness `border_accent` color and normal title, making it look like it was still receiving data.

**Root cause:** The waterfall only checked `buf.paused` (set by the `[W]` key) for its stale state. There was no check against `last_fft_frame` timestamp, which is the same mechanism the spectrum and signal_metrics panels use to detect that new FFT data has stopped arriving.

```rust
// BEFORE — only manual pause was detected
let border_color = if focused { theme.border_focused }
    else if buf.paused { theme.stale }
    else { theme.border_accent };
```

**Fix:** Added the same `last_fft_frame` timestamp check as the spectrum panel:

```rust
let stale = state.last_fft_frame.as_ref()
    .map(|fr| fr.timestamp.elapsed() > std::time::Duration::from_millis(500))
    .unwrap_or(false);
let title = if buf.paused    { " Waterfall [PAUSED] " }
    else if stale            { " Waterfall [STALE] "  }
    else                     { " Waterfall "          };
let border_color = if focused       { theme.border_focused }
    else if buf.paused || stale     { theme.stale          }
    else                            { theme.border_accent  };
```

Now both panels dim together when RX stops, and recover together when RX restarts.
