# BUG-002 — Unstable HackRF Streaming on USB-C Port

← [Bug Tracker](README.md)

**Phase:** Platform-level (not phase-specific)  
**Status:** ⚠️ Workaround (hardware-level fix not possible; app-side resilience ✅ implemented)  
**Discovered:** 2026-05-28  
**Fixed:** —  

---

## Symptom

- With HackRF connected via USB-C, the app crashes or the connection drops during RX streaming.
- The same HackRF works stably on USB-A.
- After the crash, the HackRF cannot reconnect — USB replug is required.

---

## Root cause

The HackRF One uses USB 2.0 High Speed (480 Mbit/s) isochronous transfers for streaming, which at 20 Msps produces ~40 MB/s of sustained data flow. This requires large, continuous USB bandwidth.

USB-C ports can make this problematic for several reasons:

| Cause | Explanation |
|---|---|
| **USB-C hub / adapter** | Most USB-C hubs and adapters introduce extra latency and timing overhead into isochronous transfers |
| **Alt mode negotiation** | If the port is in Thunderbolt or DisplayPort alt mode, the USB controller shares bandwidth |
| **USB 3.x fallback** | Some USB-C controllers do not handle USB 2.0 HS isochronous devices well in USB 3.x mode |
| **Power delivery interference** | USB-C PD negotiation can interrupt sustained data transfers |
| **Platform-specific driver quirk** | Linux usbfs/xhci driver behavior may differ on USB-C controllers |

The result: `hackrf_transfer.valid_length < hackrf_transfer.buffer_length` — the HackRF delivers reduced or corrupt buffers, eventually leading to a libhackrf-level error or app crash.

---

## Current workaround

**Use a USB-A port.** If the laptop only has USB-C:

1. An active (powered) USB hub with USB-A output
2. A USB-C → USB-A adapter (passive may work if the port natively falls back to USB 2.0)

---

## Mitigation directions

### 1. Graceful disconnect detection ✅ Implemented

The polling task detects when streaming stops unexpectedly (`hw_rx_active && !device.is_streaming()`):

```rust
if hw_rx_active && !device_bg.is_streaming() {
    let _ = device_bg.stop_rx();
    hw_rx_active = false;
    let mut m = state_bg.lock().unwrap_or_else(|e| e.into_inner());
    m.rx_enabled = false;
    m.push_log("WARNING: Streaming stopped unexpectedly — press [Space] to restart");
}
```

The app **does not crash** — it logs a warning and the user can restart streaming with Space.

### 2. USB transfer error counter ✅ Implemented

`valid_length == 0` (complete transfer failure) is now counted separately from sample drops:

```rust
// in rx_callback_safe:
if t.valid_length == 0 {
    if let Ok(mut m) = ctx.metrics.lock() {
        m.usb_errors_session += 1;
    }
    return 0;
}
```

The `usb_errors_session` field is displayed in the Hardware Health panel (`USB errors: N (session)`), shown in red when > 0.

### 3. Automatic reconnect

If the HackRF disconnects (USB error or physical removal), the current app stops streaming and requires Space to restart. A future version could:
- Detect disconnect via libhackrf error codes
- Gracefully stop UI streaming
- Poll the polling loop for the device to reappear
- Automatically reconnect and resume

### 4. USB-C specific warning

If the connected USB controller is USB-C (e.g. via `/sys/bus/usb/devices/*/bcdUSB`), show a one-time warning in the log:

```
WARNING: HackRF connected via USB-C — if streaming is unstable, try a USB-A port.
```

---

## Related

- [BUG-001](bug-001-iq-histogram-oob.md) — The OOB crash also required a USB replug, making it easy to confuse the two bugs.
