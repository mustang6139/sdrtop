# sdrtop — Roadmap to btop-level Quality

← [[Home]]

## Vision

`sdrtop` is a terminal-based SDR monitor in the spirit of `btop`: visually rich,
fully interactive, and genuinely useful as a daily driver. The end state is an app
that an RF engineer opens instead of `hackrf_info` + `gqrx` + a scratchpad — one
tool that shows everything, lets you tune everything, and gets out of the way.

**Current Focus:** HackRF One and PortaPack H1/H2 with Mayhem firmware (primary).
While these are the immediate priority based on available hardware, the architecture 
aims for future extensibility to other SDR platforms (e.g., RTL-SDR, LimeSDR, Airspy).

---

## Current Status

| Phase | Status |
|---|---|
| 1 — Device discovery & basic info | ✅ Done |
| 2 — Telemetry polling & USB throughput | ✅ Done |
| 3 — TUI dashboard (gauges, sparkline, log, shortcuts) | ✅ Done |
| 4 — Architecture refactor (modular layout) | ✅ Done |

---

## Technology Stack

| Concern | Choice | Notes |
|---|---|---|
| Language | Rust stable | |
| TUI | `ratatui 0.26+` | layout, widgets, Braille canvas |
| Hardware FFI | `libhackrf` via `pkg-config` | custom FFI (bypasses broken 0.0.1 crate) |
| Async runtime | `tokio` | background polling & FFT task |
| FFT | `rustfft 6` | pure-Rust, no C dependency |
| Config | `toml 0.8` + `serde 1` | `~/.config/sdrtop/config.toml` |
| CLI args | `clap 4` (derive feature) | |
| Channels | `crossbeam-channel 0.5` | lock-free sample handoff |

---

## Phase 1 — Device Discovery & Basic Info ✅ Done

**Goal:** Open a HackRF device via a hand-crafted libhackrf FFI layer and read
its identity: board name, firmware version, and serial number.

- Step-by-step execution guide: [[Phase 1 - Device Discovery - Steps]]
- Implementation log (what was done, decisions made): [[Phase 1 - Device Discovery - Log]]

### Key outcomes

- Custom `#[repr(C)]` FFI layer bypassing the broken `hackrf` 0.0.1 crate
- Critical `HackrfDeviceList` struct layout fixed (missing fields, wrong types)
- Safe `Device` wrapper with `Drop` ensuring clean `hackrf_exit()` on all exit paths

---

## Phase 2 — Telemetry Polling & USB Throughput ✅ Done

**Goal:** Start RX streaming and measure live USB throughput via a tokio background
task. Shared state updated every 200 ms behind `Arc<Mutex<SdrMetrics>>`.

- Step-by-step execution guide: [[Phase 2 - Telemetry Polling - Steps]]
- Implementation log (what was done, decisions made): [[Phase 2 - Telemetry Polling - Log]]

### Key outcomes

- `Arc<Mutex<SdrMetrics>>` shared between UI thread and background polling task
- Critical bug fixed: single `is_streaming` split into `rx_enabled` (desired, UI only) and `hw_streaming` (actual, polling only)
- `rx_callback` accumulates bytes; polling task computes throughput every 200 ms using integer arithmetic

---

## Phase 3 — TUI Dashboard ✅ Done

**Goal:** Live ratatui dashboard with telemetry panel, gain gauges, USB throughput
sparkline, log panel, and keyboard shortcuts.

- Step-by-step execution guide: [[Phase 3 - TUI Dashboard - Steps]]
- Implementation log (what was done, decisions made): [[Phase 3 - TUI Dashboard - Log]]

### Key outcomes

- Layout: header / body (telemetry left + gauges right) / log / footer
- Added: serial number in header, sample rate gauge, 64-point throughput sparkline, 7-row log panel, `r` reset key
- Footer shows only implemented keys — misleading Phase 5 shortcuts (`F`, `S`, `L`, `V`, `A`) removed

---

## Phase 4 — Architecture Refactor ✅ Done

**Goal:** `main.rs` becomes an entry point only. Split into focused modules before
adding more features. Every future phase has a clean home; no file exceeds ~200 lines.

- Step-by-step execution guide: [[Phase 4 - Architecture Refactor - Steps]]
- Implementation log (what was done, decisions made): [[Phase 4 - Architecture Refactor - Log]]

### Key outcomes

- `main.rs` reduced from ~670 lines to 43 — pure entry point, no logic
- Six focused modules: `state`, `event`, `app`, `hardware/{ffi,device}`, `ui/{layout,header,telemetry,gains,log,footer}`
- `rx_callback` and `Device` wrapper isolated in `hardware/device.rs`; UI modules have no FFI dependencies
- Stub files added for all future phases so every new feature has a clear home from day one

### Final module layout

```
src/
  main.rs               43 lines — terminal setup/teardown + App::new()?.run()
  app.rs                App struct + new() + run()
  event.rs              AppEvent enum, EventStream (mpsc + thread)
  state.rs              SdrMetrics, constants
  config.rs             stub — Phase 8
  hardware/
    mod.rs              pub use device::Device
    ffi.rs              #[repr(C)] structs + pub extern "C" declarations
    device.rs           Device wrapper + rx_callback
    buffer.rs           stub — Phase 6
  ui/
    mod.rs              pub fn draw(frame, state, ...)
    layout.rs           Chunks struct + build(size)
    header.rs           render(f, area, board_name, fw, serial)
    telemetry.rs        render(f, area, m, board_name, serial)
    gains.rs            render(f, area, m) — gauges + sparkline
    log.rs              render(f, area, m)
    footer.rs           render(f, area)
    overlay.rs          stub — Phase 5
    sparkline.rs        stub — Phase 5+
    spectrum.rs         stub — Phase 6
    waterfall.rs        stub — Phase 7
```

---

## Phase 5 — Interactive Controls

**Goal:** Every parameter visible in the UI can be changed live from the keyboard.
Hardware is called immediately; the display reflects the new value within one render frame.

### Full keybinding table

| Key | Action | HW call |
|---|---|---|
| `Space` | Toggle RX on/off | `start_rx` / `stop_rx` |
| `f` | Enter frequency input mode | — |
| `↑` / `↓` | LNA gain +8 / −8 dB | `set_lna_gain` |
| `[` / `]` | VGA gain −2 / +2 dB | `set_vga_gain` |
| `a` | Toggle AMP | `set_amp_enable` |
| `r` | Reset all to defaults | all setters |
| `?` | Toggle help overlay | — |
| `q` | Quit | — |

### Steps

**5.1 — Add `InputMode` to state**
- [ ] In `state.rs`, define:
  ```rust
  #[derive(Clone, PartialEq)]
  pub enum InputMode { Normal, FrequencyInput }
  ```
- [ ] Add `input_mode: InputMode` and `input_buf: String` to `SdrMetrics`

**5.2 — LNA gain keys (`↑` / `↓`)**
- [ ] In `app.rs`, match `KeyCode::Up` / `KeyCode::Down` in `Normal` mode
- [ ] Clamp to 0–40 in steps of 8; call `device.set_lna_gain(new_gain)?`
- [ ] On `Ok`: update `state.lna_gain`; push log entry
- [ ] On `Err`: push error to log; do not update `state.lna_gain`

**5.3 — VGA gain keys (`[` / `]`)**
- [ ] Same pattern as LNA; clamp to 0–62 in steps of 2; call `set_vga_gain`

**5.4 — AMP toggle (`a`)**
- [ ] Toggle `state.amp_enabled`; call `device.set_amp_enable(state.amp_enabled)`

**5.5 — Frequency input mode (`f`)**
- [ ] `f` in `Normal` mode → set `input_mode = FrequencyInput`, clear `input_buf`
- [ ] While in `FrequencyInput`:
  - digits and `.` append to `input_buf`
  - `Backspace` pops last char
  - `Esc` → back to `Normal`, discard buffer
  - `Enter` → parse `input_buf` as MHz, multiply × 1 000 000, call `set_frequency`
    - on `Ok`: update `state.frequency`, back to `Normal`
    - on parse error or `Err`: push log, stay in `FrequencyInput`

**5.6 — Update footer widget for input mode**
- [ ] In `ui/footer.rs`, check `input_mode`:
  - `Normal`: render keybind hints as before
  - `FrequencyInput`: render `" Frequency (MHz): [<buf>_] | Enter = confirm | Esc = cancel "`
    with cursor represented as `_`

**5.7 — Help overlay**
- [ ] Create `src/ui/overlay.rs`:
  ```rust
  pub fn render_help(f: &mut Frame, all_keys: &[(key, description)])
  ```
  - centered `Clear` + bordered `Paragraph` listing every keybinding
- [ ] In `App`, add `show_help: bool` field
- [ ] Match `KeyCode::Char('?')` to toggle `show_help`
- [ ] In `ui/mod.rs` `draw()`: if `show_help`, call `render_help` last (on top)
- [ ] Manual test: press `?`, overlay appears; press again, disappears

**5.8 — End-to-end validation**
- [ ] Every key in the table above exercised manually
- [ ] All hardware calls return `Ok`; any `Err` appears in the log panel, never crashes
- [ ] `cargo clippy -- -D warnings` — zero findings

---

## Phase 6 — FFT Spectrum Analyzer

**Goal:** A live, full-width spectrum display on a Braille canvas — the feature that
makes `sdrtop` genuinely useful for RF work instead of just pretty.

### Data pipeline

```
RX callback (libhackrf thread)
   │  raw IQ bytes pushed into crossbeam channel (bounded, drops oldest on full)
   ▼
FftWorker (tokio task)
   │  reads N samples, applies window function, runs rustfft
   │  converts magnitude to dBFS, runs EMA, computes peak-hold
   │  sends FftFrame { bins: Vec<f32> } on a second bounded channel
   ▼
UI render loop
   │  receives latest FftFrame (non-blocking, uses previous if none ready)
   ▼
SpectrumWidget → Canvas → Braille dots
```

The UI never waits for FFT. If the FFT worker is behind, the UI re-renders
the last good frame and shows a stale-frame indicator.

### FftFrame spec

```rust
pub struct FftFrame {
    pub bins_dbfs: Vec<f32>,   // length = fft_size, ordered low→high freq
    pub peak_hold: Vec<f32>,   // same length, decaying peak
    pub noise_floor: f32,      // running average of bottom 10% of bins
    pub center_freq_hz: u64,
    pub sample_rate: f64,
    pub stale: bool,           // true if this frame is older than 500 ms
}
```

### Steps

**6.1 — Add dependencies**
- [ ] Add to `Cargo.toml`:
  ```toml
  rustfft = "6"
  crossbeam-channel = "0.5"
  num-complex = "0.4"
  ```
- [ ] `cargo build` — must pass

**6.2 — Sample ring buffer (`src/hardware/buffer.rs`)**
- [ ] Define `SampleBuffer`:
  - wraps a `crossbeam_channel::Sender<Vec<u8>>`
  - channel bounded at 4 messages (≈ 4 × callback buffer, ~1 M samples)
- [ ] `SampleBuffer::push(&self, data: &[u8])` — sends a clone; on full channel
      pops the oldest by doing a non-blocking `recv` first, then `send`
- [ ] `SampleBuffer::receiver() -> Receiver<Vec<u8>>` — returns the other half
- [ ] Update `rx_callback` to call `SampleBuffer::push` instead of accumulating
      in `SdrMetrics.bytes_since_last_poll` — throughput counting moves to
      the FFT worker (it already has the byte count from the received Vec)

**6.3 — FFT worker (`src/fft.rs`)**
- [ ] Define `FftWorker` struct:
  ```rust
  pub struct FftWorker {
      samples_rx: Receiver<Vec<u8>>,
      frame_tx: Sender<FftFrame>,
      fft_size: usize,
      window: WindowFn,
      ema_alpha: f32,
  }
  ```
- [ ] Implement window functions in `src/dsp.rs`:
  - `hann(size: usize) -> Vec<f32>`
  - `hamming(size: usize) -> Vec<f32>`
  - `blackman(size: usize) -> Vec<f32>`
  - `pub enum WindowFn { Hann, Hamming, Blackman }`
- [ ] Implement `FftWorker::run(self)` as an async loop:
  1. accumulate raw bytes into a local `Vec<u8>` until `len >= fft_size * 2`
  2. convert bytes to `Vec<Complex<f32>>`: `i = byte as f32 / 128.0 - 1.0`
  3. apply window function element-wise
  4. run `rustfft` in-place
  5. compute magnitude: `20 * log10(|z| / fft_size)` → dBFS
  6. shift output so DC is at index 0 → center of display (fftshift)
  7. apply EMA: `bin = alpha * new + (1-alpha) * prev`
  8. update peak-hold: `peak[i] = max(peak[i] - decay, bin[i])`
  9. compute noise floor: mean of bottom 10% of bin values
  10. send `FftFrame` on `frame_tx`; if channel full, drop frame (non-blocking `try_send`)

**6.4 — Wire FftWorker into App**
- [ ] In `App::new()`, create `SampleBuffer`, give `Sender` to `rx_callback` context,
      give `Receiver` to `FftWorker`
- [ ] Spawn `FftWorker::run()` as a `tokio::task`
- [ ] Add `fft_rx: Receiver<FftFrame>` to `App`; store latest received frame in
      `App.last_fft_frame: Option<FftFrame>`
- [ ] In the render loop, do a non-blocking `fft_rx.try_recv()` before `draw()`;
      update `last_fft_frame` if a new frame arrived

**6.5 — Spectrum widget (`src/ui/spectrum.rs`)**
- [ ] Implement `pub fn render(f, area, frame: Option<&FftFrame>, center_hz, sr)`
- [ ] Use `ratatui::widgets::canvas::Canvas`:
  - x range: 0.0 ..= 1.0 (normalized bin index)
  - y range: `db_min ..= db_max` (configurable, default −120..0)
  - draw a filled bar for each bin using Braille dots
- [ ] Draw peak-hold as a separate line in a dimmer color
- [ ] Draw noise floor as a dashed horizontal line
- [ ] Render frequency axis: 5 equally-spaced labels in MHz below the canvas
- [ ] Render dBFS axis: 5 labels on the left side
- [ ] If `frame.stale`, tint the entire widget grey and add `[STALE]` to title
- [ ] If `frame` is `None`, render an empty canvas with "Waiting for RX…" centered

**6.6 — Integrate spectrum into layout**
- [ ] Update `ui/layout.rs` to add a `spectrum` area above the existing body
      (default height 14 rows, configurable)
- [ ] Update `ui/mod.rs` `draw()` to call `spectrum::render`
- [ ] Add `n` key to cycle FFT size: `[1024, 2048, 4096]`
- [ ] Add `w` key to cycle window function: Hann → Hamming → Blackman → Hann

**6.7 — Benchmark**
- [ ] Run with a real HackRF at 20 Msps; verify FFT frame rate ≥ 10 fps
- [ ] On Raspberry Pi 4 (if available): target ≥ 5 fps at 2048-point FFT
- [ ] `cargo build --release` — profile build must pass clean

---

## Phase 7 — Waterfall Display

**Goal:** A scrolling 2D spectrum history below the spectrum plot.

### Color palette

| Terminal capability | Palette used |
|---|---|
| Truecolor (`COLORTERM=truecolor`) | 24-bit RGB gradient: `#000080` → `#00ff00` → `#ffff00` → `#ff0000` |
| 256-color | pre-computed 32-entry lookup into xterm-256 palette |
| 16-color fallback | 4 levels: black, dark blue, cyan, white |

### Steps

**7.1 — WaterfallBuffer**
- [ ] In `state.rs`, add `WaterfallBuffer` struct with `push`, `paused`, `max_rows`
- [ ] Add `waterfall: WaterfallBuffer` to `SdrMetrics`
- [ ] Update FFT frame consumer in `app.rs`

**7.2 — Color palette (`src/palette.rs`)**
- [ ] `ColorDepth` enum + `detect()` (reads `COLORTERM` env var)
- [ ] `magnitude_to_color(db, db_min, db_max, depth) -> Color`

**7.3 — WaterfallWidget (`src/ui/waterfall.rs`)**
- [ ] `pub fn render(f, area, buf, db_min, db_max, depth)`
- [ ] Canvas with solid colored blocks (1 col × 1 row per cell)

**7.4 — Layout integration**
- [ ] `show_waterfall: bool` and `waterfall_height: u16` in `SdrMetrics`
- [ ] Conditional spectrum/waterfall split in `ui/layout.rs`

**7.5 — Keyboard controls**
- [ ] `w` cycles display mode: Spectrum → Both → Waterfall only
- [ ] `p` toggles `waterfall.paused`

**7.6 — Validation**
- [ ] At 80×24, 2048-point FFT, `Both` mode: render stays ≥ 10 fps
- [ ] Palette degrades correctly with `COLORTERM` unset

---

## Phase 8 — Configuration & Persistence

**Goal:** Settings survive restarts.

### Config schema (`~/.config/sdrtop/config.toml`)

```toml
[device]
serial = ""

[radio]
frequency_hz = 2400000000
sample_rate  = 10000000.0
lna_gain     = 16
vga_gain     = 20
amp_enabled  = false
fft_size     = 2048
fft_window   = "hann"

[display]
spectrum_height  = 14
waterfall_rows   = 20
spectrum_db_min  = -120
spectrum_db_max  = 0
theme            = "default"
show_waterfall   = true
```

### Steps

**8.1** — Add `serde`, `toml`, `clap` to `Cargo.toml`  
**8.2** — Define `Config` struct (`src/config.rs`) with nested sections  
**8.3** — `Config::load_or_default(path)` — missing file returns default, parse error logs warning  
**8.4** — `Config::save(&self, path)` — atomic write via `.tmp` + rename  
**8.5** — CLI args via `clap` (`--config`, `--frequency`, `--lna`, `--vga`, `--serial`)  
**8.6** — Apply config to initial `SdrMetrics` in `App::new()`  
**8.7** — Save on `q` exit; best-effort save via `std::panic::set_hook`

---

## Phase 9 — Multi-Device Support

**Goal:** Multiple HackRF devices monitored simultaneously; `Tab` switches focus.

### Steps

**9.1** — Introduce `DeviceHandle` struct; refactor `App` to hold `Vec<DeviceHandle>`  
**9.2** — Open all connected devices at startup; spawn one polling task + FFT worker per device  
**9.3** — Device list panel (`src/ui/device_list.rs`); `d` key toggles; `Tab` changes focus  
**9.4** — Disconnect detection; mark device offline, stop FFT worker  
**9.5** — Reconnect detection via 2-second watcher task

---

## Phase 10 — PortaPack / Mayhem Integration

**Goal:** Show Mayhem-specific telemetry when a PortaPack is connected.

### Known telemetry (USB vendor control transfers)

| Data | bRequest |
|---|---|
| Battery voltage (mV) | 0x10 |
| Active application | 0x11 |
| GPS fix + coordinates | 0x12 |

### Steps

**10.1** — USB product string detection (`"PortaPack"` → `device.is_portapack = true`)  
**10.2** — `Device::vendor_read(request, buf)` helper  
**10.3** — PortaPack telemetry polling in the background task  
**10.4** — PortaPack panel (`src/ui/portapack.rs`), hidden if `!is_portapack`

---

## Phase 11 — Polish & Production Readiness

**Steps**

**11.1** — Startup UX: loading message, clean "no device" error  
**11.2** — Terminal resize: forward `Event::Resize` as `AppEvent::Resize`  
**11.3** — Mouse support: scroll over gauges, click device list  
**11.4** — Themes: `default`, `gruvbox`, `nord`, `light`; `t` key cycles  
**11.5** — Panic hook: restore terminal unconditionally before printing panic  
**11.6** — Audit `unwrap()` calls; replace with `?` or `expect("reason")`  
**11.7** — `--no-color` flag + `NO_COLOR` env var  
**11.8** — Performance: flamegraph, ≥25 fps render, <30% CPU, <50 MB RSS  
**11.9** — Integration test harness with `libhackrf_mock.so`

---

## Phase 12 — Distribution & Community

**Steps**

**12.1** — AUR packages (`sdrtop-git` and `sdrtop`)  
**12.2** — GitHub Actions CI (lint + test) and release matrix (4 targets)  
**12.3** — Nix flake  
**12.4** — Homebrew formula  
**12.5** — `README.md`, `CONTRIBUTING.md`, man page via `clap`

---

## Key Risks & Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| `libhackrf` FFI struct differs across versions | crash / wrong data | check `hackrf_library_version()` at startup |
| FFT worker can't keep up at 20 Msps | stale spectrum | bounded drop channel; `FftFrame.stale` flag |
| Terminal lacks Braille / truecolor | broken display | `ColorDepth::detect()` at startup; ASCII fallback |
| USB disconnect mid-session | crash or hang | polling task catches error, recovers on reconnect |
| `main.rs` grows again | development friction | no file over 200 lines; clippy as CI gate |
| Mutex poisoning under panic | terminal in raw mode | `std::panic::set_hook` restores terminal (Phase 11.5) |
