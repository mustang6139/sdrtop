# BUG-004 — Comprehensive Code Audit

← [Bug Tracker](README.md)

**Date:** 2026-05-28  
**Method:** `cargo clippy -W all -W pedantic -W nursery` + manual code review of all `src/` files  

---

## Summary

| Severity | Count | Fixed |
|---|---|---|
| Crash-risk | 1 | ✅ |
| Silent wrong behavior | 2 | 🔲 documented |
| Safe but fragile casts | ~15 | 🔲 documented |
| Style / cosmetic | ~230 | — not worth fixing |

---

## FIXED: `fft.rs:128` — `unwrap()` on `partial_cmp` panics with NaN

**Severity:** Crash-risk (low probability, but possible)

**Location:** `src/fft.rs:128` (occupied BW sort)

**Root cause:**

```rust
// BEFORE (panics if any power value is NaN):
indexed.sort_unstable_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
```

`b.0` is `10f32.powf(bin_dbfs / 10.0)`. If `bin_dbfs` is NaN through EMA propagation or a bad FFT frame, `powf` produces NaN, `partial_cmp` returns `None`, and `unwrap()` panics — killing the FftWorker thread. The noise floor sort at line 96 already used the correct pattern `.unwrap_or(Equal)`. This one was missed.

**Fix:**

```rust
// AFTER:
indexed.sort_unstable_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
```

**When could NaN appear?** The EMA (`alpha * new + (1-alpha) * old`) preserves NaN once it enters: NaN + anything = NaN. A NaN can enter if `z.norm()` produces NaN for the rustfft output (shouldn't happen for finite inputs, but there is no guarantee from rustfft for certain degenerate inputs). The fix costs zero — `Ordering::Equal` for NaN vs non-NaN just leaves their relative order stable.

---

## NOT FIXED: `engine.rs:27` — `cycle_preset()` silently resets to first preset

**Severity:** Silent wrong behavior (very low risk in practice)

**Location:** `src/ui/engine.rs:27`

```rust
let current = names.iter().position(|n| n == &self.config.active_preset).unwrap_or(0);
```

If `config.active_preset` is set to a name that doesn't exist in `presets` (e.g., stale config file referencing a renamed preset), `position()` returns `None`, and `unwrap_or(0)` silently picks the first preset alphabetically. The user sees the preset change without explanation.

**Mitigation in place:** `set_preset()` already guards against this:
```rust
pub fn set_preset(&mut self, name: &str) {
    if self.config.presets.contains_key(name) { ... }  // only sets if valid
}
```

`cycle_preset()` doesn't need the same rigor since it always produces a valid next name from the keys. The only case this triggers is if the active preset was somehow set to an invalid name — which only happens via a bad TOML config (handled gracefully by falling to `unwrap_or(0)`, i.e., first preset).

**Assessment:** Acceptable. Not worth changing — the behavior is correct for all realistic inputs.

---

## NOT FIXED: `engine.rs:47-48` — `u16` overflow in top/bottom height sum

**Severity:** Theoretical (impossible in practice)

```rust
let top_h: u16 = top_specs.iter().map(|s| s.height.unwrap_or(3)).sum();
```

If panels' heights sum to > 65535 rows, wraps around. A terminal is at most ~300 rows. Irrelevant.

---

## SAFE CASTS DOCUMENTED

These are all flagged by clippy pedantic but are safe in their context. Documented here so future changes that alter value ranges are aware of the assumptions.

| Location | Cast | Safe because |
|---|---|---|
| `device.rs:72-73` | `chunk[0] as i8` (u8→i8, intentional wrap) | SDR I/Q samples are signed 8-bit; wrap is the protocol |
| `device.rs:50` | `t.valid_length as usize` (i32→usize) | Already checked `< 0` on line 40 |
| `device.rs:60` | `((buf_len - valid_len) / 2) as u64` | Only reached when `valid_len < buf_len`, so subtraction ≥ 0 |
| `device.rs:99` | `d.as_micros() as u64` (u128→u64) | Microseconds since epoch ≈ 1.7×10¹⁵, fits in u64 until year 584942 |
| `gains.rs:34,50,69` | `(pct * 100.0) as u16` | Values clamped to 0–100 before cast |
| `hardware_health.rs:75` | `*v as u64` (f32→u64) | Saturation % is always ≥ 0; Rust saturates to 0 for negative f32→u64 |
| `palette.rs:45` | `lerp` result `as u8` | Within-segment `t` is always 0–1; result stays within u8 range |
| `palette.rs:78` | `(t * 15.0) as usize` | `t` is clamped 0–1, so result is 0–15, `.min(15)` double-guards |
| `palette.rs:81` | `(t * 3.0) as u8` | `t` is clamped 0–1, result is 0–3, match arm `_ => White` handles 3 |
| `fft.rs:139` | `(f64_value) as u64` | Occupied BW is always ≥ 0 (product of non-negative factors) |
| `app.rs:189` | `as_millis() as u64` | 200ms polling interval; u128→u64 is safe for any duration under ~292 million years |
| `app.rs:451` | `f64 as f32` CPU% | Value is `.min(100.0)` before cast, always representable as f32 |
| `sysfs.rs:72` | `f64 as u64` uptime | `/proc/uptime` is always positive |
| `sysfs.rs:74` | `sysconf() as u64` | `_SC_CLK_TCK` (usually 100) is always positive |

---

## UNWRAP CALLS IN PRODUCTION CODE

All `unwrap()` calls in non-test production code:

| Location | Call | Safe because |
|---|---|---|
| `dsp.rs:39-41` | `max_by(partial_cmp).unwrap()` + `.unwrap()` | **Test code only** |
| `fft.rs:96` | `.unwrap_or(std::cmp::Ordering::Equal)` | ✅ already safe |
| `fft.rs:128` | `.unwrap_or(std::cmp::Ordering::Equal)` | ✅ fixed in this audit |
| `config.rs:246,265,275,276` | Various `.unwrap()` | **Test code only** |
| `engine.rs:27` | `.unwrap_or(0)` | See discussion above |

No unchecked `unwrap()` in the hot path (rx_callback, FftWorker, render loop). ✅

---

## CLIPPY PEDANTIC/NURSERY — NOT ADDRESSING

287 warnings total from `-W all -W pedantic -W nursery`. The majority are:

- **47× `uninlined_format_args`** — `format!("{}", x)` → `format!("{x}")`. Pure cosmetic.
- **46× `redundant_closure`** — `|x| f(x)` → `f`. Negligible.
- **16× `cast_precision_loss`** (`u64 as f64`) — All safe for SDR metric values (frequencies < 20 GHz, throughput < 1 GB/s fit in 52-bit mantissa).
- **10× `significant_drop_tightening`** — Mutex guard held slightly longer than needed. Could cause minor lock contention under extreme load. Not worth restructuring.
- **9× `could_be_const_fn`** — Functions that could be `const`. No runtime impact.
- **8× `cast_possible_truncation`** (`f64 as f32`) — All metric display values, precision loss at the 7th decimal place of a dBFS reading is irrelevant.
- **6× `cloned` instead of `copied`** — Negligible.
- **5× `redundant_clone`** — Minor allocation avoidance.
- **5× `cast_sign_loss`** (`u32 as i32` in gain calculations) — Gains are max 62, never overflow i32.

**Decision:** Leave all of these. Fixing 287 style warnings adds churn with zero functional benefit. The codebase is already clean on default `cargo clippy`.

---

## Files audited

All `src/*.rs` and `src/**/*.rs` — 25 files total.
