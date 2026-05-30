# IMP-008 — Performance overhaul

← [Home](../Home.md)

**Added:** 2026-05-30  
**Between phases:** 12 → 13

---

## Why

Profiling after IMP-006 revealed several compounding allocation and CPU hot-paths that made the app increasingly sluggish as the spectrum panel became richer:

- The `SdrMetrics::clone()` call at the start of every render frame copied **~528 KB** of heap data (512 KB waterfall buffer + FFT bins + peak hold).
- The FFT worker allocated ~88 KB of new `Vec` storage *per FFT frame* — at typical rates this adds up to tens of MB/sec of allocation pressure.
- The spectrum canvas issued ~6 000 individual `CanvasLine` / `Points` draw calls per render, roughly 10× more than the terminal's pixel resolution requires.
- `ColorDepth::detect()` called `std::env::var` on every panel render, including every waterfall row.

---

## What changed

| File | Change |
|---|---|
| `src/state.rs` | `FftFrame.bins_dbfs`, `FftFrame.peak_hold` → `Arc<Vec<f32>>`; `WaterfallBuffer.rows` → `VecDeque<Arc<Vec<f32>>>`; `WaterfallBuffer::push` takes `Arc<Vec<f32>>`; `SdrMetrics.spectrum_hold` → `Option<Arc<Vec<f32>>>` |
| `src/fft.rs` | Pre-allocated scratch buffers reused every frame; cursor-based drain; `select_nth_unstable_by` for noise floor; pre-allocated occupied-BW scratch; Arc wrapping on output |
| `src/palette.rs` | `ColorDepth::detect()` result cached in `OnceLock<ColorDepth>` |
| `src/ui/spectrum.rs` | Bins downsampled to canvas pixel width before canvas closure; `bin_colors` Vec replaced by `col_data` / `col_peaks` / `held_data` arrays sized to `draw_n` (≈ terminal width); closure captures no `Arc<Vec<f32>>` |
| `src/app.rs` | Hold-toggle (`H`) uses `Arc::clone` instead of `Vec::clone` |

---

## Changes in detail

### 1 — `Arc<Vec<f32>>` for shared spectrum data

`FftFrame.bins_dbfs`, `FftFrame.peak_hold`, `WaterfallBuffer.rows`, and `spectrum_hold` all changed from owned `Vec<f32>` to `Arc<Vec<f32>>`.

**Effect on `SdrMetrics::clone()`** (called every render frame):

| Field | Before | After |
|---|---|---|
| `waterfall.rows` (64 rows × 2048 f32) | ~512 KB memcpy | 64 atomic refcount increments |
| `last_fft_frame.bins_dbfs` (2048 f32) | ~8 KB memcpy | 1 atomic refcount increment |
| `last_fft_frame.peak_hold` (2048 f32) | ~8 KB memcpy | 1 atomic refcount increment |
| `spectrum_hold` (2048 f32, when set) | ~8 KB memcpy | 1 atomic refcount increment |
| **Total per render** | **~528 KB** | **~negligible** |

At 30 renders/sec this eliminated ~15 MB/sec of allocator pressure.

The `spectrum.rs` renderer previously did `frame.bins_dbfs.clone()` and `frame.peak_hold.clone()` to move them into the `Canvas::paint` closure — these became `Arc::clone` (O(1)).

### 2 — FFT worker: pre-allocated scratch buffers

Previously `FftWorker::run()` allocated fresh `Vec`s every FFT frame:

| Buffer | Per-frame cost (before) |
|---|---|
| `samples: Vec<Complex<f32>>` | 2048 × 8 B = 16 KB alloc |
| `mags: Vec<f32>` | 2048 × 4 B = 8 KB alloc |
| `shifted: Vec<f32>` | 8 KB alloc + 2× `extend_from_slice` |
| `sorted = smoothed.clone()` (noise floor) | 8 KB clone |
| `indexed: Vec<(f32, usize)>` (occupied BW) | 2048 × 8 B = 16 KB alloc |

All five are now declared once before the receive loop and reused every frame. The `smoothed` and `peak` Vecs are also pre-allocated at `DB_FLOOR` so the `is_empty()` guard on first-frame init is gone.

### 3 — FFT worker: cursor-based drain

Previously:
```rust
while buf.len() >= fft_size * 2 {
    // process buf[..fft_size * 2]
    buf.drain(..fft_size * 2);   // ← O(remaining) shift every frame
}
```

`Vec::drain(..n)` shifts all remaining bytes left. If a USB chunk contains multiple FFT frames this happened O(frames-per-chunk) times.

Now:
```rust
let mut buf_start = 0;
while buf.len() - buf_start >= frame_bytes {
    // process buf[buf_start..buf_start + frame_bytes]
    buf_start += frame_bytes;
}
if buf_start > 0 { buf.drain(..buf_start); }  // single drain per chunk
```

### 4 — Noise floor: O(n) partial sort

```rust
// Before: full O(n log n) sort
let mut sorted = smoothed.clone();
sorted.sort_by(|a, b| a.partial_cmp(b)...);
let noise_floor = sorted[..count].iter().sum::<f32>() / count as f32;

// After: O(n) average via partial selection
noise_scratch.copy_from_slice(&smoothed);
noise_scratch.select_nth_unstable_by(count - 1, |a, b| a.partial_cmp(b)...);
let noise_floor = noise_scratch[..count].iter().sum::<f32>() / count as f32;
```

`select_nth_unstable_by` partitions the slice so the first `count` elements are all ≤ the rest — sufficient for computing the mean without sorting them.

### 5 — Spectrum canvas: per-bin drawing preserved

An initial attempt at downsampling bins to canvas pixel columns was reverted. Max-pooling per display column caused two visible regressions:

- **Noise floor appeared higher and more ragged** — max of N noise bins per column is always elevated vs. the visual mean that the original dense rendering produced.
- **DC spike appeared thicker** — adjacent bins near the HackRF's DC pedestal were pooled into the same column, inflating its apparent width.

The original per-bin canvas loops (2048 fill lines + 2047 outline segments) are retained. The canvas rasteriser naturally handles the many-bins-per-pixel mapping, preserving the correct visual density.

### 6 — Closure captures: `bin_colors` pre-computed outside the closure

`bin_colors: Vec<Color>` (2048 entries, ~8 KB) is pre-computed before the canvas closure, as before. The closure captures it by move (heap pointer transfer, O(1)) together with the two `Arc<Vec<f32>>` handles for bins and peaks.

### 7 — `ColorDepth::detect()` cached

```rust
static CACHED_DEPTH: OnceLock<ColorDepth> = OnceLock::new();

impl ColorDepth {
    pub fn detect() -> Self {
        *CACHED_DEPTH.get_or_init(|| { /* env::var logic */ })
    }
}
```

The terminal's colour capabilities do not change at runtime. `detect()` is called from every panel render (spectrum, waterfall, etc.) and from inside the waterfall row loop. With the cache, all calls after the first are a single atomic load.
