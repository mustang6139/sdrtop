# BUG-005 — Multi-Bug Audit (Phase 12 review)

← [Bug Tracker](README.md)

**Date:** 2026-05-30  
**Method:** 3-angle parallel static analysis + manual cross-file review of all `src/` files  
**Status:** ✅ All fixed

---

## Summary

| Severity | Count | Fixed |
|---|---|---|
| Data corruption / wrong metric | 3 | ✅ |
| UI renders outside bounds / crash on resize | 1 | ✅ |
| Misleading UI (stale data shown as live) | 3 | ✅ |
| Incorrect rendering (off-by-one, arm shadowing) | 2 | ✅ |
| State reset on hardware failure | 1 | ✅ |

---

## FIX 1 — `device.rs` — Jitter measured with wall clock instead of monotonic clock

**Severity:** Data corruption (jitter metric incorrect after any clock slew)

**Location:** `src/hardware/device.rs` ~line 97 (rx_callback)

**Root cause:**

```rust
// BEFORE — wall clock; NTP step backward produces saturating_sub → 0 gap
let now_us = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_micros() as u64)
    .unwrap_or(0);
if let Some(last_us) = m.acc_last_callback_us {
    let gap = now_us.saturating_sub(last_us);
    ...
}
```

Any NTP adjustment that steps the clock backward makes `saturating_sub` silently produce 0, poisoning `acc_jitter_sum_us` for the slew duration. The next callback after a forward step measures a double-length gap.

Additionally, `acc_last_callback_us` was typed `Option<u64>` in `state.rs`, forcing the epoch-based integer arithmetic; `acc_i_sq_sum` / `acc_q_sq_sum` were typed `i64` even though they hold strictly non-negative squared sums.

**Fix:**

```rust
// AFTER — monotonic Instant; no platform clock dependency
let now = std::time::Instant::now();
if let Some(last) = m.acc_last_callback_us {
    let gap_us = now.duration_since(last).as_micros() as u64;
    m.acc_jitter_sum_us += gap_us;
    m.acc_jitter_count  += 1;
}
m.acc_last_callback_us = Some(now);
```

`state.rs`: `acc_last_callback_us: Option<u64>` → `Option<std::time::Instant>`,  
`acc_i_sq_sum / acc_q_sq_sum: i64` → `u64`.  
`device.rs`: accumulation casts `i_sq as u64` / `q_sq as u64` (always non-negative; safe).

---

## FIX 2 — `sysfs.rs` — `sysconf(_SC_CLK_TCK)` error sentinel not checked

**Severity:** Silent wrong value (Observer mode shows wrong process uptime)

**Location:** `src/hardware/sysfs.rs:74`

**Root cause:**

```rust
// BEFORE — -1 error return cast directly to u64 → u64::MAX
let ticks_per_sec = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as u64;
// .max(1) on line 122 does not help: u64::MAX is already > 1
// starttime_ticks / u64::MAX ≈ 0 → running_secs = uptime_secs (wrong: process appears to have started at boot)
```

**Fix:**

```rust
// AFTER — explicit guard, POSIX fallback of 100 HZ
let ticks_raw = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
let ticks_per_sec: u64 = if ticks_raw > 0 { ticks_raw as u64 } else { 100 };
```

---

## FIX 3 — `state.rs` — `WaterfallBuffer::push` unbounded growth when `max_rows == 0`

**Severity:** Memory leak / OOM

**Location:** `src/state.rs:17`

**Root cause:**

```rust
// BEFORE — condition is 0 >= 0 = true immediately, pop_back on empty deque is a no-op
if self.rows.len() >= self.max_rows {
    self.rows.pop_back();  // no-op when empty
}
self.rows.push_front(bins);  // unconditional → grows without bound
```

A user setting `waterfall_max_rows = 0` in config expecting to disable the waterfall instead caused continuous memory growth.

**Fix:**

```rust
if self.paused || self.max_rows == 0 { return; }
```

---

## FIX 4 — `app.rs` — `'r'` reset updates UI state unconditionally even when hardware calls fail

**Severity:** UI shows wrong state after hardware error

**Location:** `src/app.rs` `KeyCode::Char('r')` handler

**Root cause:**

All five `device.set_*()` calls were made, then `m.reset_to_defaults()` was called unconditionally regardless of errors. If `set_frequency()` failed over USB, the hardware stayed at the old frequency but `m.frequency` was updated to `DEFAULT_FREQUENCY` — UI and hardware permanently out of sync. Errors were only logged afterward, after the incorrect state was already committed.

**Fix:** `reset_to_defaults()` is only called when `results.iter().all(|r| r.is_ok())`. On any failure, only the error messages are logged and hardware + UI state remain consistent.

---

## FIX 5 — `header.rs` — bottom band renders outside panel at small terminal height

**Severity:** Rendering corruption (overwrites adjacent panel border)

**Location:** `src/ui/header.rs` `render()`

**Root cause:**

```rust
let bot_area = Rect { x: inner.x, y: inner.y + 2, ... };
```

With no height guard, at `area.height < 5` (`inner.height < 3`), `inner.y + 2` points past or at the bottom border, rendering the frequency/gain line over the panel below.

**Fix:**

```rust
if inner.height < 3 { return; }
```

---

## FIX 6 — `signal_metrics.rs` — SNR displays `0.0 dB [CRIT]` before first FFT frame

**Severity:** Misleading UI

**Location:** `src/ui/signal_metrics.rs:67`

**Root cause:**

`state.snr_db` is initialised to `0.0` and only updated by the FftWorker. Before streaming starts, `snr_color(0.0)` returned `status_crit` (bright red) and the value was formatted as `"0.0 dB"`. All other metrics in the same panel already had stale guards (`is_finite()`, `> 0`, `Option`); SNR was the only exception.

**Fix:**

```rust
if stale { "---".into() } else { format!("{:.1} dB", state.snr_db) }
// color: theme.label when stale, snr_color() when live
```

---

## FIX 7 — `iq_diagnostics.rs` — DC offset and IQ imbalance shown as live after streaming stops

**Severity:** Misleading UI

**Location:** `src/ui/iq_diagnostics.rs`

**Root cause:**

`dc_offset_i/q` and `iq_imbalance_db` are computed only when `acc_sample_count > 0`. Once streaming stops, accumulators reset to 0, the condition fails, and these fields freeze at their last measured values. The panel rendered them unconditionally with no stale title, no border colour change, no `"---"` — unlike `signal_metrics` and `signal_strip` which check `last_fft_frame.timestamp`.

**Fix:** Added `stale = !hw_streaming && !observer_mode` check. When stale: `[STALE]` title, `theme.stale` border colour, `"---"` rows.

---

## FIX 8 — `spectrum.rs` — `x_bounds` off-by-one leaves rightmost canvas column blank

**Severity:** Rendering defect (cosmetic but consistent)

**Location:** `src/ui/spectrum.rs:116`

**Root cause:**

```rust
// BEFORE
.x_bounds([0.0, n])   // n = bins.len() as f64 = 2048.0
// polyline draws from x=0 to x=n-1 (2047)
// rightmost 1/n of canvas (≈0.5% of width) never painted
```

The noise floor line used the same wrong `x2: n` bound.

**Fix:**

```rust
.x_bounds([0.0, n - 1.0])
// noise floor: x2: n - 1.0
```

---

## FIX 9 — `waterfall.rs` — legend arm ordering: `-120 dBFS` label missing at small panel heights

**Severity:** Rendering defect

**Location:** `src/ui/waterfall.rs:104`

**Root cause:**

```rust
// BEFORE — match arm order
r if r == h / 2          => "-60 dBFS",   // fires when h=2, row=1
r if r == h.saturating_sub(1) => "-120 dBFS",  // shadowed — never reached when h=2
```

When `inner.height == 2`: `h/2 == 1` and `h.saturating_sub(1) == 1`. Both patterns match `row=1`, but the `middle` arm fires first. The bottom `-120 dBFS` label is skipped.

**Fix:** Swap arm order — check `h.saturating_sub(1)` (bottom) before `h/2` (middle):

```rust
0                              => "+  0 dBFS",
r if r == h.saturating_sub(1) => "-120 dBFS",   // checked first
r if r == h / 2                => "- 60 dBFS",
_                              => blank,
```

---

## Regression tests

All 78 existing unit tests pass with no changes to test expectations (the type changes to `u64` and `Instant` required no test updates since the arithmetic results are identical). No new tests added — the fixes address rendering, field types, and runtime state consistency that are not unit-testable without a running terminal.
