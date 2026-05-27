# Phase 3 — TUI Dashboard: Implementation Log

← [Home](../Home.md) | [Roadmap](../Roadmap.md) | [Phase 3 - TUI Dashboard - Steps](Phase%203%20-%20TUI%20Dashboard%20-%20Steps.md)

**Status:** ✅ Complete

---

## What was built

A live ratatui TUI with four sections: header (device info), body (telemetry left +
gauges/sparkline right), log panel, and footer. Reacts to `q`, `Space`, and `r`.

---

## Features added in this phase

Three features were identified as missing from the initial TUI and added:

| Feature | Where |
|---|---|
| Serial number in header | `Paragraph` in header row |
| Sample rate gauge | Third `Gauge` in the right panel |
| USB throughput sparkline | `Sparkline` with 64-point `VecDeque<u64>` history |
| Log panel | 7-row `Paragraph` backed by `VecDeque<String>` (max 100 entries) |
| `r` reset key | Calls `reset_to_defaults()` on `SdrMetrics` |

---

## Bug: borrow checker error with sparkline history

When pushing to `throughput_history`, the compiler rejected:

```rust
m.throughput_history.push_back(m.current_throughput_bps / 1024);
```

**Error:**
```
error[E0502]: cannot borrow `m` as immutable because it is also borrowed as mutable
```

The mutable borrow from `push_back` conflicted with reading `current_throughput_bps`
in the same expression. Fixed by extracting to a local variable first:

```rust
let throughput_kb = m.current_throughput_bps / 1024;
m.throughput_history.push_back(throughput_kb);
```

---

## Bug: misleading footer keybindings

The initial footer advertised keys that had no corresponding match arms in the
event handler:

```
[F] Freq | [S] Sample Rate | [L] LNA | [V] VGA | [A] AMP
```

None of these were implemented (Phase 5 work). Showing them created a false
impression that the app was broken. The footer was cleaned up to only show
actually-implemented keys:

```
[Q] Quit | [SPACE] Start/Stop RX | [R] Reset to defaults
```

---

## Compile errors encountered

**Error 1 — duplicate code block:**  
Lines 551–567 in the original `main.rs` contained a second footer/event-loop
fragment that appeared after `run_app()`'s closing brace — dead code outside
any function. Removed entirely.

**Error 2 — `m.6` tuple index on named struct:**  
`SdrMetrics` is a named struct, not a tuple struct. A reference to `m.6` (from
an earlier version) was invalid Rust. Fixed to use the correct field name.

---

## Telemetry panel color coding

The telemetry panel border changes color based on hardware state:

```rust
let status_color = if m.hw_streaming { Color::Green } else { Color::Yellow };
Block::default()
    .border_style(Style::default().fg(status_color))
```

Green = actively streaming IQ data. Yellow = idle (device open but not streaming).

---

## Gauge ranges

| Gauge | Min | Max | Step | Color |
|---|---|---|---|---|
| LNA | 0 dB | 40 dB | 8 dB | Cyan |
| VGA | 0 dB | 62 dB | 2 dB | Magenta |
| Sample Rate | 0 Msps | 20 Msps | — | Yellow |

The sample rate gauge shows `actual_sample_rate` (derived from throughput),
not the configured value. At 0 throughput (RX not active) it reads 0.

---

## `reset_to_defaults()`

```rust
fn reset_to_defaults(&mut self) {
    self.lna_gain = DEFAULT_LNA_GAIN;        // 16
    self.vga_gain = DEFAULT_VGA_GAIN;        // 20
    self.amp_enabled = false;
    self.frequency = DEFAULT_FREQUENCY;      // 2400 MHz
    self.config_sample_rate = DEFAULT_SAMPLE_RATE;  // 10 Msps
    self.push_log("Settings reset to defaults");
}
```

Note: `reset_to_defaults` updates the metrics struct only. It does **not** call
the hardware setter methods (`set_lna_gain`, etc.) — those are called by the
interactive controls in Phase 5. For now, reset affects the display only.

---

## `SdrMetrics` additions

The struct gained these fields in this phase (vs. Phase 2):

```rust
throughput_history: VecDeque<u64>,  // KB/s history for sparkline, max 64 entries
log: VecDeque<String>,              // in-app log messages, max 100 entries
```

And these methods:

```rust
fn push_log(&mut self, msg: impl Into<String>)
fn reset_to_defaults(&mut self)
```

---
