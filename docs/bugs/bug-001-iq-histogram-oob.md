# BUG-001 — IQ Histogram Bin Index Out-of-Bounds (`i8::MIN`)

← [Bug Tracker](README.md)

**Phase:** 11 — HackRF Deep Diagnostics  
**Status:** ✅ Fixed  
**Discovered:** 2026-05-28  
**Fixed:** 2026-05-28  

---

## Symptom

- The app crashes while RX is active and the IQ amplitude histogram is receiving data.
- After the crash, the HackRF cannot reconnect — USB unplug/replug is required.
- Deterministically reproducible: apply a strong signal or set high gain (near ADC saturation).

---

## Root cause

In `rx_callback`, the IQ amplitude calculation:

```rust
let amp = i_byte.unsigned_abs().max(q_byte.unsigned_abs());
m.acc_iq_hist[(amp / 4) as usize] += 1;  // ← BUG
```

`i8::unsigned_abs()` returns the absolute value as `u8`.  
The absolute value of `i8::MIN` (`-128`) is **128** — which fits in `u8` but not in `i8`.

```
i8::MIN = -128
(-128i8).unsigned_abs() = 128u8
128u8 / 4 = 32
acc_iq_hist[32]  →  index out of bounds!  (array len = 32, valid range: 0..31)
```

### Why was it catastrophic?

The panic occurred inside `rx_callback`, which is called by the C libhackrf library. Rust panics cannot safely unwind through C stack frames — the process terminates via `abort()`, **without running Drop**.

Consequences:
- `hackrf_stop_rx()` and `hackrf_close()` are never called
- The HackRF firmware remains in streaming mode
- The next connection attempt fails (`HACKRF_ERROR_STREAMING_THREAD_ERR`)
- USB replug is required to reset the firmware

---

## Fix

**File:** `src/hardware/device.rs`

```rust
// Before:
m.acc_iq_hist[(amp / 4) as usize] += 1;

// After:
m.acc_iq_hist[((amp / 4) as usize).min(31)] += 1;
```

`amp = 128` (the only value `i8` can produce at this boundary) is placed in **bin 31**, the highest amplitude range — semantically correct, not just a workaround.

---

## Regression test

**File:** `src/hardware/device::tests::histogram_bin_i8_min_does_not_overflow`

```rust
#[test]
fn histogram_bin_i8_min_does_not_overflow() {
    let i_byte: i8 = i8::MIN;
    let q_byte: i8 = 0;
    let amp = i_byte.unsigned_abs().max(q_byte.unsigned_abs());
    assert_eq!(amp, 128);                          // unsigned_abs() == 128, not 127
    let bin = ((amp / 4) as usize).min(31);
    assert_eq!(bin, 31);                           // clamped, not 32
}
```

---

## Lesson learned

**Never use `i8::unsigned_abs()` as a direct array index without explicitly handling the `i8::MIN` case.**  
The absolute value of `i8` does not fit in `i8` (the range is asymmetric: -128..=127), but it does fit in `u8` — where the value 128 causes an off-by-one error on any 32-element array.
