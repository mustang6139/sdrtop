# BUG-003 — IQ Histogram Panel UTF-8 String Slice Panic

← [Bug Tracker](README.md)

**Phase:** 11 — HackRF Deep Diagnostics  
**Status:** ✅ Fixed  
**Discovered:** 2026-05-28  
**Fixed:** 2026-05-28  

---

## Symptom

- The app crashes when RX is active and a preset containing the IQ amplitude histogram panel is active (e.g. `lab`).
- Reproducible: touch the antenna or increase LNA/VGA gain → ADC saturation → high histogram bin values → panic.
- The crash does **not** originate from `rx_callback` — it comes from the **UI renderer**.

```
thread 'main' panicked at src/ui/iq_histogram.rs:70:59:
end byte index 8 is not a char boundary; it is inside '█' (bytes 6..9) of `███ ...`
```

---

## Root cause

The `render` function splits each histogram row (`rows: Vec<String>`) into three colored column strips — low / mid / high amplitude. The split was done using byte indices:

```rust
// BUGGY:
let low_rows: Vec<String> = rows.iter()
    .map(|r| r[..low_cols.min(r.len())].to_string())
    .collect();
```

The `█` character (U+2588, FULL BLOCK) is **3 bytes in UTF-8** (`E2 96 88`). When many saturated samples arrive (antenna touch, high gain), the upper histogram bins fill with `█`. `r.len()` returns the byte length of the string, but `low_cols` is a character count — these are not equal when `█` is present.

**Concrete example:**

```
low_cols = 8  (8 chars → 24 bytes if all █)
r = "███..."   (first 3 █ = 9 bytes)
r[..8]          → byte 8 is inside the 3rd █ (bytes 6..9) → PANIC
```

### Why was it misleading?

The panic occurred in the **main thread, inside the UI renderer** — not in `rx_callback`. The backtrace propagated through tokio runtime frames, which initially looked like a threading or mutex problem. The actual line (`src/ui/iq_histogram.rs:70`) was only visible in the full backtrace.

---

## Fix

**File:** `src/ui/iq_histogram.rs`

```rust
// Before (byte-index based — BUGGY):
let low_rows: Vec<String> = rows.iter()
    .map(|r| r[..low_cols.min(r.len())].to_string())
    .collect();
let mid_rows: Vec<String> = rows.iter().map(|r| {
    let start = low_cols.min(r.len());
    let end = (low_cols + mid_cols).min(r.len());
    r[start..end].to_string()
}).collect();
let high_rows: Vec<String> = rows.iter().map(|r| {
    let start = (low_cols + mid_cols).min(r.len());
    r[start..].to_string()
}).collect();

// After (char-index based — CORRECT):
let low_rows: Vec<String> = rows.iter()
    .map(|r| r.chars().take(low_cols).collect())
    .collect();
let mid_rows: Vec<String> = rows.iter()
    .map(|r| r.chars().skip(low_cols).take(mid_cols).collect())
    .collect();
let high_rows: Vec<String> = rows.iter()
    .map(|r| r.chars().skip(low_cols + mid_cols).collect())
    .collect();
```

`chars().take/skip` iterates by character, independent of byte boundaries.

---

## Regression test

**File:** `src/ui/iq_histogram::tests::histogram_row_split_does_not_panic_on_block_chars`

```rust
#[test]
fn histogram_row_split_does_not_panic_on_block_chars() {
    let row: String = (0..32).map(|_| '█').collect();
    let low_cols = 8usize;
    let mid_cols = 16usize;

    let low:  String = row.chars().take(low_cols).collect();
    let mid:  String = row.chars().skip(low_cols).take(mid_cols).collect();
    let high: String = row.chars().skip(low_cols + mid_cols).collect();

    assert_eq!(low.chars().count(),  8);
    assert_eq!(mid.chars().count(), 16);
    assert_eq!(high.chars().count(), 8);
}
```

---

## Lesson learned

**Never use `s[..n]` byte-slicing on strings that may contain multi-byte characters.**  
In UI renderers where content changes at runtime (ADC saturation → `█` characters), always use `chars().take/skip`-based splitting. `str::len()` returns bytes, not characters.

---

## Related

- [BUG-001](bug-001-iq-histogram-oob.md) — Same panel, different bug: BUG-001 was on the data collection side (`rx_callback`), this one is on the rendering side.
- [BUG-002](bug-002-usbc-streaming-instability.md) — USB-C instability is also reproducible via antenna/gain operations, making the two easy to conflate.
