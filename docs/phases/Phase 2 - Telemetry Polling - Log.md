# Phase 2 — Telemetry Polling & USB Throughput: Implementation Log

← [Home](../Home.md) | [Roadmap](../Roadmap.md) | [Phase 2 - Telemetry Polling - Steps](Phase%202%20-%20Telemetry%20Polling%20-%20Steps.md)

**Status:** ✅ Complete

---

## What was built

A `tokio::spawn` background task that polls hardware state every 200 ms, computes
USB throughput from bytes accumulated by `rx_callback`, and keeps `SdrMetrics`
up to date behind an `Arc<Mutex<SdrMetrics>>`.

---

## Critical bug: dual-purpose `is_streaming` field

This was the most significant logic bug of the project so far.

**The problem:** A single `is_streaming: bool` field in `SdrMetrics` was used for
two different purposes:
1. The user's *desired* state (toggled by the Space key)
2. The *actual* hardware streaming state (read from `hackrf_is_streaming()`)

The polling task wrote `m.is_streaming = board.is_streaming()` every 200 ms,
overwriting whatever the Space key had set. As a result, `rx_enabled` could
never be set to `true` for long enough to trigger `start_rx` — the next poll
immediately reset it back to `false`.

**The fix:** Split into two separate fields:

```rust
rx_enabled: bool,   // desired state — only touched by UI (Space key)
hw_streaming: bool, // actual HW state — only written by polling task
```

The polling task reads `rx_enabled` to decide whether to call `start_rx` or
`stop_rx`, and writes `hw_streaming` to reflect what the hardware is actually doing.
The UI reads `hw_streaming` to color the status indicator.

A local variable `hw_rx_active: bool` in the polling task tracks whether
`start_rx` was actually issued to the hardware, without touching `rx_enabled`.

---

## Bug: `hw_streaming` inside `elapsed_ms > 0` guard

**The problem:** The initial implementation updated `hw_streaming` only when
`elapsed_ms > 0`, i.e., only when throughput was being computed. This meant
the streaming status could be stale for an entire poll cycle.

**The fix:** Move `m.hw_streaming = board.is_streaming()` outside the guard
so it is updated unconditionally on every poll cycle regardless of elapsed time.

```rust
// Wrong — hw_streaming only updated when elapsed_ms > 0:
if elapsed_ms > 0 {
    m.current_throughput_bps = ...;
    m.hw_streaming = board.is_streaming();  // too late
}

// Correct — always updated:
m.hw_streaming = board.is_streaming();
if elapsed_ms > 0 {
    m.current_throughput_bps = ...;
}
```

---

## `rx_callback` design

```rust
extern "C" fn rx_callback(transfer: *mut hackrf_transfer) -> c_int {
    unsafe {
        let t = &*transfer;
        let metrics_ptr = t.rx_ctx as *const Mutex<SdrMetrics>;
        if !metrics_ptr.is_null() {
            if let Ok(mut m) = (*metrics_ptr).lock() {
                m.bytes_since_last_poll += t.valid_length as u64;
            }
        }
    }
    0
}
```

`rx_ctx` is set to `Arc::as_ptr(&metrics)` cast to `*mut c_void`, which points
to the inner `Mutex<SdrMetrics>`. The callback does the minimal work possible —
just accumulating bytes. All computation happens in the polling task.

The `Arc` itself keeps the `Mutex<SdrMetrics>` alive for the lifetime of the app,
so the raw pointer in `rx_ctx` is valid as long as streaming is active.

---

## Throughput calculation

```
throughput_bps = (bytes_since_last_poll * 1000) / elapsed_ms
actual_sample_rate = throughput_bps / 2   // 2 bytes per IQ sample (8-bit I + 8-bit Q)
```

The `* 1000 / elapsed_ms` pattern avoids floating point: multiply first to
preserve precision in integer arithmetic.

---

## Unused setter methods

All hardware setter methods (`set_lna_gain`, `set_vga_gain`, `set_frequency`, etc.)
were added to `Device` with `#[allow(dead_code)]` — they are not called yet
(interactive controls are Phase 5) but must exist for the architecture to be complete.

---
