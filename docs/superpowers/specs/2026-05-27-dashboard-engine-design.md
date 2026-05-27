# sdrtop — Dashboard Engine & Resource Monitoring: Design Spec

**Date:** 2026-05-27  
**Status:** Approved  
**Scope:** Phases 6–7 redesign + roadmap restructure through Phase 12

---

## Problem

The current roadmap adds features (FFT, waterfall, controls) into a fixed layout. There is no panel system — every new widget is hardcoded into the layout. This prevents user customization and makes the app feel like a basic SDR frontend rather than a genuine resource monitor.

Three classes of hardware problems (sample drops, ADC saturation, USB instability) are currently invisible. When they occur simultaneously, there is no way to correlate them or determine which is the root cause.

---

## Goal

Transform sdrtop into a btop-style modular dashboard where:
1. Every display element is a named, self-contained panel
2. The user controls which panels are shown and where, via config file and preset switching
3. Hardware health metrics (drop rate, ADC saturation, IQ quality) are first-class citizens alongside the spectrum and telemetry

---

## Architecture

### Panel Trait

Every display unit implements:

```rust
pub trait Panel: Send {
    fn name(&self) -> &'static str;
    fn min_size(&self) -> (u16, u16);  // minimum (width, height) in terminal cells
    fn render(&self, f: &mut Frame, area: Rect, state: &AppState);
}
```

`AppState` replaces the current `&SdrMetrics` parameter — it holds `SdrMetrics` plus `InputMode` (introduced in Phase 5) so panels can react to UI state.

### PanelRegistry

A `HashMap<&'static str, Box<dyn Panel>>` populated at startup. All panels — existing and new — are registered here. Adding a new panel requires only implementing the trait and one registry call; no other code changes.

### LayoutEngine

Reads the active layout config at startup and on preset switch. Builds a ratatui `Layout` tree from the panel list. Calls each panel's `render` in order. Panels that are not in the active config are not rendered and do not consume CPU.

### Preset Layouts

Named panel configurations defined in `config.toml`. Three built-in presets:

| Preset | Panels included |
|---|---|
| `monitoring` | header, hardware_health, iq_diagnostics, system_resources, telemetry, log, footer |
| `spectrum` | header, spectrum, waterfall, footer |
| `minimal` | header, telemetry, footer |

Runtime switching: `p` cycles through presets in order. `1`/`2`/`3` jump to a specific preset directly. Active preset name shown in footer.

### Config format

```toml
[layout]
active_preset = "monitoring"

[presets.monitoring]
panels = [
  { name = "header",           position = "top",    height = 3 },
  { name = "hardware_health",  position = "left",   width_pct = 50 },
  { name = "system_resources", position = "right",  width_pct = 50 },
  { name = "telemetry",        position = "body" },
  { name = "log",              position = "bottom", height = 7 },
  { name = "footer",           position = "bottom", height = 3 },
]

[presets.spectrum]
panels = [
  { name = "header",    position = "top",    height = 3 },
  { name = "spectrum",  position = "body" },
  { name = "waterfall", position = "body" },
  { name = "footer",    position = "bottom", height = 3 },
]
```

---

## New Hardware Metrics

All computed in `rx_callback` or the polling task. Added as new fields on `SdrMetrics`.

### Sample drop rate

Detected from `hackrf_transfer.valid_length` vs expected buffer size. If `valid_length < buffer_length`, samples were dropped.

```rust
pub drops_per_sec: u64,
pub total_drops_session: u64,
pub drop_history: VecDeque<u64>,  // 64-point sparkline, drops/sec
```

### ADC saturation %

In `rx_callback`, count bytes where `byte == 0x00 || byte == 0x7F` (signed 8-bit rails). Divide by `valid_length`.

```rust
pub adc_saturation_pct: f32,      // current poll cycle
pub adc_saturation_peak: f32,     // session max
pub saturation_history: VecDeque<f32>,  // 64-point sparkline
```

### IQ imbalance

Mean power of I channel vs Q channel. Computed in polling task from accumulated samples. Zero = perfectly balanced.

```rust
pub iq_imbalance_db: f32,   // positive = I stronger, negative = Q stronger
```

### DC offset

Mean value of I and Q streams separately. Zero = AC-coupled signal with no DC bias.

```rust
pub dc_offset_i: f32,
pub dc_offset_q: f32,
```

### Callback jitter

Variance in time between consecutive `rx_callback` invocations. High jitter = USB pipeline instability.

```rust
pub callback_jitter_us: u64,   // microseconds, rolling variance
```

### System resources

Read from `/proc/self/stat` (CPU) and `/proc/self/status` (RSS). Updated every 1 second by a dedicated tokio task (not the hardware polling task).

```rust
pub process_cpu_pct: f32,
pub process_rss_mb: u64,
```

---

## New Panels

| Panel | Displays |
|---|---|
| `hardware_health` | Drop rate: current value + sparkline + session total. ADC saturation: current % + sparkline + session peak. Callback jitter: current µs. |
| `iq_diagnostics` | DC offset I and Q (bar gauges centered at zero). IQ imbalance in dB. Color coding: green = healthy, yellow = mild, red = problematic. |
| `system_resources` | Process CPU% gauge. RSS memory gauge. USB throughput sparkline (moved from gains panel). |

### Existing panels migrated to trait

All current panels are rewritten as `Panel` implementations with identical render logic:
`header`, `telemetry`, `gains`, `log`, `footer`

The gains panel loses the throughput sparkline (moved to `system_resources`).

---

## Thresholds and Color Coding

| Metric | Green | Yellow | Red |
|---|---|---|---|
| Drop rate | 0 drops/sec | 1–10 drops/sec | >10 drops/sec |
| ADC saturation | <1% | 1–5% | >5% |
| IQ imbalance | <1 dB | 1–3 dB | >3 dB |
| DC offset | <0.5% | 0.5–2% | >2% |
| Callback jitter | <500 µs | 500–2000 µs | >2000 µs |

Thresholds are configurable in `config.toml` under `[thresholds]`.

---

## Revised Roadmap

| Phase | Title | Change |
|---|---|---|
| 5 | Interactive Controls | Unchanged |
| 6 | **Dashboard Engine** | New — Panel trait, registry, layout engine, presets, basic config |
| 7 | **Hardware Health Panels** | New — all new metrics + hardware_health, iq_diagnostics, system_resources panels |
| 8 | FFT Spectrum | Was Phase 6 — now implemented as a Panel plugin |
| 9 | Waterfall | Was Phase 7 — now implemented as a Panel plugin |
| 10 | Config & Persistence | Was Phase 8 — expanded with full layout config persistence |
| 11 | Multi-device | Was Phase 9 |
| 12 | PortaPack / Mayhem | Was Phase 10 |
| 13 | Polish & Production Readiness | Was Phase 11 |
| 14 | Distribution & Community | Was Phase 12 |

---

## What This Enables Long-Term

Every future panel (PortaPack battery, GPS fix, per-device stats in multi-device mode) is a self-contained `Panel` implementation. No layout refactoring required. The user controls what they see.
