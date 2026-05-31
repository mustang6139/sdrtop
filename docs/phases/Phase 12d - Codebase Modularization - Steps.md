# Phase 12d — Codebase Modularization: Steps

← [Home](../Home.md) | [Roadmap](../Roadmap.md)

**Goal:** Eliminate the current spaghetti structure. Every module lives in exactly
one logical home, no file exceeds ~300 lines, and adding a new feature requires
touching only the files that are actually about that feature.

**Prerequisite:** Phase 12c complete. This phase is pure refactoring — no new
features, no behaviour changes, all tests must stay green.

**Why now:** Phase 13 (HAL) will refactor `hardware/` into a trait-based system.
That refactor is much safer when `app.rs`, `state.rs`, and `tasks.rs` are already
clean. Doing both at once is how you get a 2000-line god-object.

---

## Diagnosis: what is broken today

### `app.rs` — 1006 lines, three jobs in one file

1. **Construction** — `new_normal()` and `new_observer()` each contain an
   identical 50-field `SdrMetrics { … }` literal. Two copies, 200+ duplicated
   lines. When a field is added, both copies need updating.

2. **UI wiring** — `build_ui()` registers every panel and builds the focus-key
   map. Nothing about this belongs in the same file as the event loop.

3. **Event loop** — `run()` is a single 550-line `match` that handles:
   - Global keys (`q`, `r`, `space`, `p`, `1`–`6`, `w`, `h`, `?`, `a`)
   - Spectrum-focus keys (`←`, `→`, `[`, `]`, `↑`, `↓`, `j`, `k`, `m`) — only
     active when `focused_panel == "spectrum"`
   - Waterfall-focus keys (`↑`, `↓`, `[`, `]`, `←`, `→`, `j`, `k`, `m`) —
     only active when `focused_panel == "waterfall"`
   - Text input modes: `FrequencyInput`, `SampleRateInput`, `MarkerNameInput`

   Adding a new panel with focus keys means editing this one massive `match`.

### `state.rs` — `SdrMetrics` is a 50-field blob

Fields from at least **eight** conceptually distinct areas are merged into one
flat struct. Every panel that renders reads `m.frequency`, `m.lna_gain`,
`m.observer_owner`, `m.spectrum_cursor_freq`, etc. — all at the same level:

| Concern | Fields |
|---|---|
| Hardware identity | `board_name`, `serial`, `fw_version`, `board_rev`, `usb_api_version`, `cpld_ok` |
| Radio config | `frequency`, `config_sample_rate`, `lna_gain`, `vga_gain`, `amp_enabled` |
| Runtime streaming | `rx_enabled`, `hw_streaming`, `bytes_since_last_poll`, `last_poll_time`, `actual_sample_rate`, `current_throughput_bps`, `throughput_history`, `sample_rate_history` |
| Signal quality | `drops_per_sec`, `total_drops_session`, `drop_history`, `adc_saturation_pct`, `adc_saturation_peak`, `saturation_history`, `snr_db`, `channel_power_dbfs`, `occupied_bw_hz`, `usb_errors_session` |
| IQ diagnostics | `iq_imbalance_db`, `dc_offset_i`, `dc_offset_q`, `callback_jitter_us`, `iq_amplitude_hist` |
| Observer mode | `observer_mode`, `observer_device`, `observer_serial`, `observer_usb`, `observer_connected`, `observer_owner`, `observer_cmdline`, `observer_owner_cpu_pct`, `observer_owner_ram_mb`, `observer_owner_uptime` |
| Spectrum UI state | `spectrum_step_hz`, `spectrum_y_min`, `spectrum_y_max`, `spectrum_hold`, `spectrum_cursor_freq`, `spectrum_markers`, `pending_marker_freq` |
| Waterfall UI state | `waterfall_db_min`, `waterfall_scroll_offset`, `waterfall_cursor_freq`, `waterfall` (buffer) |
| UI control | `input_mode`, `input_buf`, `focused_panel`, `focused_panel_bindings`, `log` |
| Raw accumulators | `acc_drops`, `acc_saturated`, `acc_i_sum`, `acc_q_sum`, `acc_i_sq_sum`, `acc_q_sq_sum`, `acc_sample_count`, `acc_jitter_sum_us`, `acc_jitter_count`, `acc_last_callback_us`, `acc_iq_hist` |

The accumulator fields (`acc_*`) are especially wrong: they are written only by
the hardware callback and read/reset only by the polling task. They should never
be visible to the UI layer, yet they are `pub` on the shared state struct.

### Top-level `fft.rs` and `dsp.rs`

Signal processing code lives at the crate root alongside `config.rs`, `theme.rs`,
and `main.rs`. It belongs under `signal/`.

### `tasks.rs` — one flat file for three unrelated tasks

`spawn_rx_task`, `spawn_observer_task`, and `spawn_sys_resource_task` have nothing
to do with each other. They share a file only because they were written at the
same time.

---

## Target structure

```
src/
├── main.rs                    (~100 lines — unchanged, already clean)
├── config.rs                  (unchanged)
├── event.rs                   (unchanged)
├── theme.rs                   (unchanged)
├── palette.rs                 (unchanged)
│
├── state/
│   ├── mod.rs                 re-export SdrMetrics; impl push_log / reset_to_defaults
│   ├── radio.rs               RadioState
│   ├── signal.rs              SignalState  (throughput, drops, saturation, snr …)
│   ├── iq.rs                  IqState  (dc_offset, imbalance, hist, jitter)
│   ├── observer.rs            ObserverState
│   ├── spectrum.rs            SpectrumState + SpectrumMarker
│   ├── waterfall.rs           WaterfallState + WaterfallBuffer + FftFrame
│   ├── system.rs              SystemState  (hw identity + process resources)
│   ├── ui.rs                  UiState  (input_mode, input_buf, focused, log)
│   └── acc.rs                 Accumulators  (pub(crate) — invisible outside crate)
│
├── signal/
│   ├── mod.rs                 re-exports
│   ├── fft.rs                 FftWorker  (moved from src/fft.rs)
│   └── dsp.rs                 DSP helpers  (moved from src/dsp.rs)
│
├── hardware/                  (unchanged internally — Phase 13 will refactor this)
│   ├── mod.rs
│   ├── device.rs
│   ├── ffi.rs
│   ├── sysfs.rs
│   └── buffer.rs
│
├── tasks/
│   ├── mod.rs                 re-exports + fmt_duration
│   ├── rx.rs                  spawn_rx_task
│   ├── observer.rs            spawn_observer_task
│   └── system.rs              spawn_sys_resource_task + read_self_stats
│
├── app/
│   ├── mod.rs                 App struct + run() — thin event loop (~120 lines)
│   ├── builder.rs             App::new / new_normal / new_observer / build_ui
│   └── input.rs               handle_key() — dispatches to mode-specific handlers
│
└── ui/                        (all panel files unchanged)
    ├── mod.rs
    ├── engine.rs
    ├── panel.rs
    ├── registry.rs
    ├── overlay.rs
    └── … (all existing panel .rs files)
```

---

## Dependency order

```
Step 1   src/signal/        fft.rs + dsp.rs  — mechanical move, update use paths
    ↓
Step 2   src/tasks/         rx.rs + observer.rs + system.rs  — split tasks.rs
    ↓
Step 3   src/state/         split SdrMetrics into 9 sub-structs
                            update all panel files + tasks/ to new paths
    ↓
Step 4   src/app/           split app.rs into mod.rs + builder.rs + input.rs
    ↓
Step 5   verify             cargo test, cargo clippy --  must be clean
```

Each step compiles independently. Never leave the codebase in a broken state
between steps.

---

## Step 1 — Move signal processing to `src/signal/`

**Files touched:** `src/fft.rs` → `src/signal/fft.rs`,
`src/dsp.rs` → `src/signal/dsp.rs`, `src/main.rs`, `src/app.rs`,
`src/hardware/buffer.rs`, `src/tasks/rx.rs`

- [ ] Create `src/signal/mod.rs` — re-exports `FftWorker`, `FftFrame`
- [ ] Move `src/fft.rs` → `src/signal/fft.rs` verbatim
- [ ] Move `src/dsp.rs` → `src/signal/dsp.rs` verbatim
- [ ] Remove `mod fft;` and `mod dsp;` from `src/main.rs`; add `mod signal;`
- [ ] Update every `use crate::fft::` and `use crate::dsp::` to
  `use crate::signal::{FftWorker, …}`
- [ ] `cargo build` — must compile with zero errors

**Why:** Signal processing is neither hardware nor UI. Grouping it under `signal/`
makes the crate root mean something: config, app entry point, theme — not a pile
of unrelated modules.

---

## Step 2 — Split `tasks.rs` into `src/tasks/`

**Files touched:** `src/tasks.rs` → deleted;
new `src/tasks/mod.rs`, `src/tasks/rx.rs`,
`src/tasks/observer.rs`, `src/tasks/system.rs`

- [ ] Create `src/tasks/rx.rs` — move `spawn_rx_task` verbatim
- [ ] Create `src/tasks/observer.rs` — move `spawn_observer_task` verbatim
- [ ] Create `src/tasks/system.rs` — move `spawn_sys_resource_task` +
  `read_self_stats` verbatim
- [ ] Create `src/tasks/mod.rs`:

```rust
mod rx;
mod observer;
mod system;

pub use rx::spawn_rx_task;
pub use observer::spawn_observer_task;
pub use system::{spawn_sys_resource_task, read_self_stats};

pub fn fmt_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 { format!("{}h {}m {}s", h, m, s) }
    else if m > 0 { format!("{}m {}s", m, s) }
    else { format!("{}s", s) }
}
```

- [ ] Delete `src/tasks.rs`
- [ ] `cargo build` — must compile with zero errors

**Why:** Three background tasks that share nothing belong in separate files.
`fmt_duration` is shared utility; it lives in `mod.rs` so callers import it as
`tasks::fmt_duration`.

---

## Step 3 — Split `SdrMetrics` into sub-structs

This is the largest step. `SdrMetrics` becomes a **container of named sub-structs**
rather than a flat bag of 50 fields.

**New shape:**

```rust
// src/state/mod.rs
pub struct SdrMetrics {
    pub radio:    RadioState,
    pub signal:   SignalState,
    pub iq:       IqState,
    pub observer: ObserverState,
    pub spectrum: SpectrumState,
    pub waterfall: WaterfallState,
    pub system:   SystemState,
    pub ui:       UiState,
    pub(crate) acc: Accumulators,  // invisible to UI panels
}
```

### Step 3a — Define sub-structs (no breakage yet)

- [ ] Create `src/state/radio.rs`:
  ```rust
  #[derive(Clone)]
  pub struct RadioState {
      pub frequency:          u64,
      pub config_sample_rate: f64,
      pub actual_sample_rate: u32,
      pub lna_gain:           u32,
      pub vga_gain:           u32,
      pub amp_enabled:        bool,
      pub rx_enabled:         bool,
      pub hw_streaming:       bool,
  }
  ```

- [ ] Create `src/state/signal.rs`:
  ```rust
  #[derive(Clone)]
  pub struct SignalState {
      pub bytes_since_last_poll:   u64,
      pub last_poll_time:          std::time::Instant,
      pub current_throughput_bps:  u64,
      pub throughput_history:      VecDeque<u64>,
      pub sample_rate_history:     VecDeque<u64>,
      pub drops_per_sec:           u64,
      pub total_drops_session:     u64,
      pub drop_history:            VecDeque<u64>,
      pub adc_saturation_pct:      f32,
      pub adc_saturation_peak:     f32,
      pub saturation_history:      VecDeque<f32>,
      pub snr_db:                  f32,
      pub channel_power_dbfs:      f32,
      pub occupied_bw_hz:          u64,
      pub usb_errors_session:      u64,
  }
  ```

- [ ] Create `src/state/iq.rs`:
  ```rust
  #[derive(Clone)]
  pub struct IqState {
      pub iq_imbalance_db:   f32,
      pub dc_offset_i:       f32,
      pub dc_offset_q:       f32,
      pub callback_jitter_us: u64,
      pub iq_amplitude_hist: [u64; 32],
  }
  ```

- [ ] Create `src/state/observer.rs`:
  ```rust
  #[derive(Clone, Default)]
  pub struct ObserverState {
      pub active:          bool,
      pub device:          Option<String>,
      pub serial:          Option<String>,
      pub usb:             Option<String>,
      pub connected:       Option<String>,
      pub owner:           Option<String>,
      pub cmdline:         Option<String>,
      pub owner_cpu_pct:   f32,
      pub owner_ram_mb:    u64,
      pub owner_uptime:    Option<String>,
  }
  ```
  Note: rename `observer_mode` → `active` since the struct is already named
  `ObserverState`. Old code: `m.observer_mode` → `m.observer.active`.

- [ ] Create `src/state/spectrum.rs`:
  ```rust
  #[derive(Clone)]
  pub struct SpectrumState {
      pub step_hz:         u64,
      pub y_min:           f32,
      pub y_max:           f32,
      pub hold:            Option<Arc<Vec<f32>>>,
      pub cursor_freq:     Option<u64>,
      pub markers:         Vec<SpectrumMarker>,
      pub pending_marker:  Option<u64>,
  }

  #[derive(Clone, Debug, Serialize, Deserialize)]
  pub struct SpectrumMarker {
      pub freq_hz: u64,
      pub label:   String,
  }
  ```

- [ ] Create `src/state/waterfall.rs` — move `WaterfallBuffer` + `FftFrame` here;
  add:
  ```rust
  #[derive(Clone)]
  pub struct WaterfallState {
      pub db_min:        f32,
      pub scroll_offset: usize,
      pub cursor_freq:   Option<u64>,
      pub buffer:        WaterfallBuffer,
      pub last_fft:      Option<FftFrame>,
  }
  ```
  The current `m.waterfall` (buffer) and `m.last_fft_frame` merge into
  `m.waterfall.buffer` and `m.waterfall.last_fft`.

- [ ] Create `src/state/system.rs`:
  ```rust
  #[derive(Clone)]
  pub struct SystemState {
      pub board_name:      String,
      pub serial:          String,
      pub fw_version:      String,
      pub board_rev:       u8,
      pub usb_api_version: u16,
      pub cpld_ok:         Option<bool>,
      pub process_cpu_pct: f32,
      pub process_rss_mb:  u64,
  }
  ```

- [ ] Create `src/state/ui.rs`:
  ```rust
  #[derive(Clone)]
  pub struct UiState {
      pub input_mode:             InputMode,
      pub input_buf:              String,
      pub focused_panel:          Option<String>,
      pub focused_panel_bindings: &'static [(&'static str, &'static str)],
      pub log:                    VecDeque<String>,
  }
  ```
  Move `InputMode` enum here. Move `push_log` and `LOG_MAX_ENTRIES` here.

- [ ] Create `src/state/acc.rs`:
  ```rust
  #[derive(Clone, Default)]
  pub(crate) struct Accumulators {
      pub drops:           u64,
      pub saturated:       u64,
      pub i_sum:           i64,
      pub q_sum:           i64,
      pub i_sq_sum:        u64,
      pub q_sq_sum:        u64,
      pub sample_count:    u64,
      pub jitter_sum_us:   u64,
      pub jitter_count:    u64,
      pub last_callback:   Option<std::time::Instant>,
      pub iq_hist:         [u64; 32],
  }
  ```
  `pub(crate)` on the struct and all fields — UI panels cannot see it.

### Step 3b — Rewrite `src/state/mod.rs`

Replace the current flat `SdrMetrics` struct with the container:

```rust
mod acc;
mod iq;
mod observer;
mod radio;
mod signal;
mod spectrum;
mod system;
mod ui;
mod waterfall;

pub use acc::Accumulators;
pub use iq::IqState;
pub use observer::ObserverState;
pub use radio::RadioState;
pub use signal::SignalState;
pub use spectrum::{SpectrumMarker, SpectrumState};
pub use system::SystemState;
pub use ui::{InputMode, UiState, LOG_MAX_ENTRIES};
pub use waterfall::{FftFrame, WaterfallBuffer, WaterfallState};

pub const THROUGHPUT_HISTORY_LEN: usize = 64;
pub const DEFAULT_LNA_GAIN: u32 = 16;
pub const DEFAULT_VGA_GAIN: u32 = 20;
pub const DEFAULT_FREQUENCY: u64 = 2_400_000_000;
pub const DEFAULT_SAMPLE_RATE: f64 = 10_000_000.0;

#[derive(Clone)]
pub struct SdrMetrics {
    pub radio:    RadioState,
    pub signal:   SignalState,
    pub iq:       IqState,
    pub observer: ObserverState,
    pub spectrum: SpectrumState,
    pub waterfall: WaterfallState,
    pub system:   SystemState,
    pub ui:       UiState,
    pub(crate) acc: Accumulators,
}

impl SdrMetrics {
    pub fn push_log(&mut self, msg: impl Into<String>) {
        self.ui.push_log(msg);
    }

    pub fn reset_to_defaults(&mut self) {
        self.radio.lna_gain           = DEFAULT_LNA_GAIN;
        self.radio.vga_gain           = DEFAULT_VGA_GAIN;
        self.radio.amp_enabled        = false;
        self.radio.frequency          = DEFAULT_FREQUENCY;
        self.radio.config_sample_rate = DEFAULT_SAMPLE_RATE;
        self.push_log("Settings reset to defaults");
    }
}
```

### Step 3c — Update all callers

Every file that reads from `m.<field>` must be updated to `m.<substruct>.<field>`.

**Systematic rename map** (old → new):

| Old | New |
|---|---|
| `m.frequency` | `m.radio.frequency` |
| `m.lna_gain` | `m.radio.lna_gain` |
| `m.vga_gain` | `m.radio.vga_gain` |
| `m.amp_enabled` | `m.radio.amp_enabled` |
| `m.config_sample_rate` | `m.radio.config_sample_rate` |
| `m.actual_sample_rate` | `m.radio.actual_sample_rate` |
| `m.rx_enabled` | `m.radio.rx_enabled` |
| `m.hw_streaming` | `m.radio.hw_streaming` |
| `m.current_throughput_bps` | `m.signal.current_throughput_bps` |
| `m.throughput_history` | `m.signal.throughput_history` |
| `m.sample_rate_history` | `m.signal.sample_rate_history` |
| `m.drops_per_sec` | `m.signal.drops_per_sec` |
| `m.total_drops_session` | `m.signal.total_drops_session` |
| `m.drop_history` | `m.signal.drop_history` |
| `m.adc_saturation_pct` | `m.signal.adc_saturation_pct` |
| `m.adc_saturation_peak` | `m.signal.adc_saturation_peak` |
| `m.saturation_history` | `m.signal.saturation_history` |
| `m.snr_db` | `m.signal.snr_db` |
| `m.channel_power_dbfs` | `m.signal.channel_power_dbfs` |
| `m.occupied_bw_hz` | `m.signal.occupied_bw_hz` |
| `m.usb_errors_session` | `m.signal.usb_errors_session` |
| `m.iq_imbalance_db` | `m.iq.iq_imbalance_db` |
| `m.dc_offset_i` | `m.iq.dc_offset_i` |
| `m.dc_offset_q` | `m.iq.dc_offset_q` |
| `m.callback_jitter_us` | `m.iq.callback_jitter_us` |
| `m.iq_amplitude_hist` | `m.iq.iq_amplitude_hist` |
| `m.observer_mode` | `m.observer.active` |
| `m.observer_device` | `m.observer.device` |
| `m.observer_serial` | `m.observer.serial` |
| `m.observer_usb` | `m.observer.usb` |
| `m.observer_connected` | `m.observer.connected` |
| `m.observer_owner` | `m.observer.owner` |
| `m.observer_cmdline` | `m.observer.cmdline` |
| `m.observer_owner_cpu_pct` | `m.observer.owner_cpu_pct` |
| `m.observer_owner_ram_mb` | `m.observer.owner_ram_mb` |
| `m.observer_owner_uptime` | `m.observer.owner_uptime` |
| `m.spectrum_step_hz` | `m.spectrum.step_hz` |
| `m.spectrum_y_min` | `m.spectrum.y_min` |
| `m.spectrum_y_max` | `m.spectrum.y_max` |
| `m.spectrum_hold` | `m.spectrum.hold` |
| `m.spectrum_cursor_freq` | `m.spectrum.cursor_freq` |
| `m.spectrum_markers` | `m.spectrum.markers` |
| `m.pending_marker_freq` | `m.spectrum.pending_marker` |
| `m.waterfall_db_min` | `m.waterfall.db_min` |
| `m.waterfall_scroll_offset` | `m.waterfall.scroll_offset` |
| `m.waterfall_cursor_freq` | `m.waterfall.cursor_freq` |
| `m.waterfall` (the buffer) | `m.waterfall.buffer` |
| `m.last_fft_frame` | `m.waterfall.last_fft` |
| `m.board_name` | `m.system.board_name` |
| `m.serial` | `m.system.serial` |
| `m.fw_version` | `m.system.fw_version` |
| `m.board_rev` | `m.system.board_rev` |
| `m.usb_api_version` | `m.system.usb_api_version` |
| `m.cpld_ok` | `m.system.cpld_ok` |
| `m.process_cpu_pct` | `m.system.process_cpu_pct` |
| `m.process_rss_mb` | `m.system.process_rss_mb` |
| `m.input_mode` | `m.ui.input_mode` |
| `m.input_buf` | `m.ui.input_buf` |
| `m.focused_panel` | `m.ui.focused_panel` |
| `m.focused_panel_bindings` | `m.ui.focused_panel_bindings` |
| `m.log` | `m.ui.log` |
| `m.acc_drops` | `m.acc.drops` |
| `m.acc_saturated` | `m.acc.saturated` |
| `m.acc_i_sum` | `m.acc.i_sum` |
| `m.acc_q_sum` | `m.acc.q_sum` |
| `m.acc_i_sq_sum` | `m.acc.i_sq_sum` |
| `m.acc_q_sq_sum` | `m.acc.q_sq_sum` |
| `m.acc_sample_count` | `m.acc.sample_count` |
| `m.acc_jitter_sum_us` | `m.acc.jitter_sum_us` |
| `m.acc_jitter_count` | `m.acc.jitter_count` |
| `m.acc_last_callback_us` | `m.acc.last_callback` |
| `m.acc_iq_hist` | `m.acc.iq_hist` |

**Files that need updating (check each):**
- `src/app.rs`
- `src/tasks/rx.rs`
- `src/tasks/observer.rs`
- `src/tasks/system.rs`
- `src/hardware/buffer.rs`
- `src/signal/fft.rs`
- `src/ui/telemetry.rs`
- `src/ui/gains.rs`
- `src/ui/throughput.rs`
- `src/ui/sample_rate.rs`
- `src/ui/signal_strip.rs`
- `src/ui/usb_sr.rs`
- `src/ui/hardware_health.rs`
- `src/ui/iq_diagnostics.rs`
- `src/ui/iq_histogram.rs`
- `src/ui/system_resources.rs`
- `src/ui/spectrum.rs`
- `src/ui/waterfall.rs`
- `src/ui/rf_chain.rs`
- `src/ui/signal_metrics.rs`
- `src/ui/observer.rs`
- `src/ui/log.rs`
- `src/ui/footer.rs`
- `src/ui/header.rs`

- [ ] Complete all renames; `cargo build` must be clean before continuing

**Correctness note on accumulators:** `hardware/buffer.rs` writes to `m.acc.*`
inside the rx callback. Since `Accumulators` is `pub(crate)`, `buffer.rs` (same
crate) can still access it directly. The UI panels simply cannot import it.

---

## Step 4 — Split `app.rs` into `src/app/`

**Files touched:** `src/app.rs` → deleted;
new `src/app/mod.rs`, `src/app/builder.rs`, `src/app/input.rs`

### Step 4a — `src/app/builder.rs`

Move `App::new`, `App::new_normal`, `App::new_observer`, and `App::build_ui`
here. The duplicate `SdrMetrics { … }` literals get replaced by a single
`SdrMetrics::default_for(config, system_state, observer_state)` constructor,
eliminating the 200-line duplication.

```rust
// builder.rs — key idea: one constructor, two callers
impl SdrMetrics {
    pub fn for_startup(
        cfg:      &AppConfig,
        system:   SystemState,
        observer: ObserverState,
    ) -> Self {
        Self {
            radio: RadioState {
                frequency:          cfg.radio.frequency_hz,
                config_sample_rate: cfg.radio.sample_rate,
                lna_gain:           cfg.radio.lna_gain,
                vga_gain:           cfg.radio.vga_gain,
                amp_enabled:        cfg.radio.amp_enabled,
                ..Default::default()
            },
            signal:   SignalState::default(),
            iq:       IqState::default(),
            observer,
            spectrum: SpectrumState {
                step_hz: 100_000,
                y_min:   -120.0,
                y_max:   0.0,
                markers: cfg.display.spectrum_markers.clone(),
                ..Default::default()
            },
            waterfall: WaterfallState::new(cfg.display.waterfall_max_rows),
            system,
            ui:  UiState::default(),
            acc: Accumulators::default(),
        }
    }
}
```

`new_normal` and `new_observer` each call `SdrMetrics::for_startup` with their
respective `SystemState` and `ObserverState`. The duplication is gone.

### Step 4b — `src/app/input.rs`

Extract the entire key-handling block from `run()` into a free function:

```rust
pub fn handle_key(
    key:    KeyEvent,
    state:  &Arc<Mutex<SdrMetrics>>,
    device: Option<&Arc<Device>>,
    engine: &mut LayoutEngine,
    show_help: &mut bool,
    focus_keys: &HashMap<char, &'static str>,
    theme:  &Theme,
) { … }
```

The function dispatches internally:

```rust
match input_mode {
    InputMode::Normal        => handle_normal(key, …),
    InputMode::FrequencyInput  => handle_freq_input(key, …),
    InputMode::SampleRateInput => handle_sr_input(key, …),
    InputMode::MarkerNameInput => handle_marker_input(key, …),
}
```

`handle_normal` further dispatches by focused panel:

```rust
fn handle_normal(key, …) {
    match focused_panel {
        Some("spectrum")  => handle_spectrum_focus(key, …),
        Some("waterfall") => handle_waterfall_focus(key, …),
        _                 => handle_global(key, …),
    }
}
```

Each handler (`handle_global`, `handle_spectrum_focus`, `handle_waterfall_focus`,
`handle_freq_input`, `handle_sr_input`, `handle_marker_input`) lives as a private
`fn` in `input.rs`. Each is ≤ 80 lines.

**Result:** Adding a new panel's focus keys means adding one `handle_<panel>_focus`
function and one arm to the `match focused_panel` — nothing else changes.

### Step 4c — `src/app/mod.rs`

The `App` struct and `run()` remain here, now thin:

```rust
pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
    const FRAME_DURATION: Duration = Duration::from_millis(33);
    let mut last_draw = Instant::now();

    // initial draw
    self.draw(terminal)?;

    loop {
        match self.events.recv() {
            AppEvent::Key(key) => {
                input::handle_key(
                    key, &self.state, self.device.as_ref(),
                    &mut self.engine, &mut self.show_help,
                    &self.focus_keys, &self.theme,
                );
                if last_draw.elapsed() >= FRAME_DURATION {
                    self.draw(terminal)?;
                    last_draw = Instant::now();
                }
            }
            AppEvent::Tick => {
                self.draw(terminal)?;
                last_draw = Instant::now();
            }
        }
    }
}
```

Target size: `mod.rs` ≤ 80 lines, `builder.rs` ≤ 200 lines, `input.rs` ≤ 250 lines.

- [ ] Move `App` struct and `run()` to `src/app/mod.rs`
- [ ] Move construction to `src/app/builder.rs`; unify the two init paths
- [ ] Move key handling to `src/app/input.rs`; split by mode and focus
- [ ] Delete `src/app.rs`
- [ ] `cargo build` must be clean

---

## Step 5 — Verify

- [ ] `cargo test` — all tests pass (the existing unit tests in `state`, `tasks`,
  `app` are preserved in their new locations)
- [ ] `cargo clippy -- -D warnings` — clean
- [ ] Run the app against a real HackRF: normal mode + observer mode + all presets
- [ ] No behaviour changes visible to the user

---

## Size targets after this phase

| File | Before | Target |
|---|---|---|
| `src/app.rs` | 1006 lines | deleted |
| `src/app/mod.rs` | — | ≤ 80 lines |
| `src/app/builder.rs` | — | ≤ 200 lines |
| `src/app/input.rs` | — | ≤ 250 lines |
| `src/state.rs` | 278 lines | deleted |
| `src/state/mod.rs` | — | ≤ 80 lines |
| `src/state/*.rs` | — | ≤ 60 lines each |
| `src/tasks.rs` | 285 lines | deleted |
| `src/tasks/*.rs` | — | ≤ 120 lines each |
| `src/fft.rs` | 267 lines | moved to `src/signal/fft.rs` |
| `src/dsp.rs` | 57 lines | moved to `src/signal/dsp.rs` |

Total codebase line count stays approximately the same — the goal is
**distribution**, not deletion.

---

## What this does NOT change

- All panel `.rs` files under `src/ui/` — field access paths change (Step 3c)
  but the rendering logic is untouched.
- `src/hardware/` — untouched internally. Phase 13 (HAL) will refactor this.
- `src/config.rs`, `src/event.rs`, `src/theme.rs`, `src/palette.rs` — untouched.
- `src/main.rs` — only the `mod` declarations change.
- No new features. No behaviour changes. No new dependencies.

---

## What Phase 13 (HAL) becomes easier after this phase

Phase 13 introduces `SdrDevice` trait + `BackendRegistry`. With clean modules:
- `hardware/device.rs` can be moved into `hardware/hackrf/device.rs` without
  touching `app/` or `state/`
- `tasks/rx.rs` will depend on `Box<dyn SdrDevice>` instead of `Arc<Device>` —
  a one-file change
- `MockDevice` for tests slots into `hardware/mock.rs` — no other file changes
