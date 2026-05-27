# Phase 2 — Telemetry Polling & USB Throughput: Steps

← [Home](../Home.md) | [Roadmap](../Roadmap.md) | [Phase 2 - Telemetry Polling - Log](Phase%202%20-%20Telemetry%20Polling%20-%20Log.md)

**Goal:** Start RX streaming and measure live USB throughput. A background tokio
task polls the hardware state every 200 ms and updates a shared metrics struct.
The main thread reads metrics and displays them.

---

## Step 1 — `SdrMetrics` struct

- [ ] Define `SdrMetrics` in `main.rs`:
  ```rust
  #[derive(Clone)]
  struct SdrMetrics {
      frequency: u64,
      config_sample_rate: f64,
      actual_sample_rate: u32,
      lna_gain: u32,
      vga_gain: u32,
      amp_enabled: bool,
      rx_enabled: bool,        // desired RX state (Space toggle)
      hw_streaming: bool,      // actual hardware streaming state
      bytes_since_last_poll: u64,
      last_poll_time: Instant,
      current_throughput_bps: u64,
  }
  ```
- [ ] Wrap in `Arc<Mutex<SdrMetrics>>` for shared access between UI and polling task

---

## Step 2 — `rx_callback`

- [ ] Define `extern "C" fn rx_callback(transfer: *mut hackrf_transfer) -> c_int`
- [ ] Cast `transfer.rx_ctx` back to `*const Mutex<SdrMetrics>`
- [ ] Lock and increment `bytes_since_last_poll += transfer.valid_length`
- [ ] Return `0`

---

## Step 3 — Background polling task

- [ ] `tokio::spawn(async move { loop { ... } })`
- [ ] Every 200 ms:
  1. Snapshot `bytes_since_last_poll` and reset it (under lock)
  2. Compute `elapsed_ms` since last poll
  3. Compute `current_throughput_bps = (bytes * 1000) / elapsed_ms`
  4. Compute `actual_sample_rate = throughput_bps / 2` (2 bytes per IQ sample)
  5. Poll `board.is_streaming()` and write to `hw_streaming`
  6. If `rx_enabled && !hw_rx_active`: call `board.start_rx(rx_callback, user_param)`
  7. If `!rx_enabled && hw_rx_active`: call `board.stop_rx()`

---

## Step 4 — Add setter methods to `Device`

- [ ] `start_rx(callback, user_param) -> Result<()>`
- [ ] `stop_rx() -> Result<()>`
- [ ] `set_lna_gain(gain: u32) -> Result<()>`
- [ ] `set_vga_gain(gain: u32) -> Result<()>`
- [ ] `set_sample_rate(hz: f64) -> Result<()>`
- [ ] `set_frequency(hz: u64) -> Result<()>`
- [ ] `set_amp_enable(enable: bool) -> Result<()>`
- [ ] `is_streaming() -> bool`

---

## Step 5 — Apply initial hardware settings

- [ ] After `Device::open()`, call `set_sample_rate`, `set_frequency`,
      `set_lna_gain`, `set_vga_gain` with defaults before spawning the task
- [ ] Defaults: 2400 MHz, 10 Msps, LNA 16 dB, VGA 20 dB, AMP off

---
