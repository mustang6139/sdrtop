# Phase 3 — TUI Dashboard: Steps

← [Home](../Home.md) | [Roadmap](../Roadmap.md) | [Phase 3 - TUI Dashboard - Log](Phase%203%20-%20TUI%20Dashboard%20-%20Log.md)

**Goal:** Replace stdout output with a live ratatui TUI showing all telemetry.
The dashboard must update in real time and respond to keyboard input.

---

## Step 1 — Terminal setup / teardown

- [ ] Add `ratatui`, `crossterm` to `Cargo.toml`
- [ ] In `main()`, wrap the TUI in setup / teardown:
  ```rust
  enable_raw_mode()?;
  execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
  // ... run app ...
  disable_raw_mode()?;
  execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;
  ```
- [ ] Ensure teardown runs even if `run_app` returns an error

---

## Step 2 — Layout

```
┌─────────────── Header (3 rows) ───────────────┐
│  HackRF One | FW: 2024.02.1 | S/N: abc123    │
├───────────────────────┬───────────────────────┤
│                       │  LNA Gain: 16 dB      │  ← 3 rows
│    Telemetry          │  VGA Gain: 20 dB      │  ← 3 rows
│    (left 50%)         │  Sample Rate: 0.0 Msps│  ← 3 rows
│                       │  USB Throughput ▁▂▃▄  │  ← remaining
├───────────────────────┴───────────────────────┤
│ Log (7 rows)                                  │
├───────────────────────────────────────────────┤
│ Footer (3 rows) — key hints                   │
└───────────────────────────────────────────────┘
```

- [ ] Outer vertical split: `[Length(3), Min(0), Length(7), Length(3)]`
- [ ] Body horizontal split: `[Percentage(50), Percentage(50)]`
- [ ] Right side vertical split: `[Length(3), Length(3), Length(3), Min(0)]`

---

## Step 3 — Header widget

- [ ] `Paragraph` with board name, firmware version, serial number
- [ ] Centered, full border

---

## Step 4 — Telemetry panel

- [ ] `Paragraph` listing: model, serial, status, frequency, sample rate (cfg),
      throughput (MB/s + actual Msps), AMP state
- [ ] Border color: green when `hw_streaming`, yellow when idle

---

## Step 5 — Gain gauges

- [ ] LNA `Gauge`: 0–40 dB range (8 dB steps), cyan
- [ ] VGA `Gauge`: 0–62 dB range (2 dB steps), magenta
- [ ] Sample rate `Gauge`: 0–20 Msps range, yellow

---

## Step 6 — USB throughput sparkline

- [ ] `VecDeque<u64>` of length 64, storing KB/s values
- [ ] `Sparkline` widget, peak shown in title, green

---

## Step 7 — Log panel

- [ ] `VecDeque<String>` of max 100 entries, newest at bottom
- [ ] `Paragraph` with `join("\n")`, 7 rows tall

---

## Step 8 — Footer

- [ ] `Paragraph` with keybind hints: `[Q] Quit | [SPACE] Start/Stop RX | [R] Reset`
- [ ] Only show keys that are actually implemented

---

## Step 9 — Event loop

- [ ] `event::poll(100ms)` + `event::read()`
- [ ] `q` → quit
- [ ] `Space` → toggle `rx_enabled`
- [ ] `r` → reset to defaults

---
