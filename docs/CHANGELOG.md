# Changelog

← [Home](Home.md)

Chronological record of shipped milestones and improvements.  
Full details are in the linked phase logs and improvement files.

---

## 2026-05-30

### BUG-005 — Phase 12 multi-bug audit (9 fixes)
→ [details](bugs/bug-005-multi-bug-audit.md)

- **Monotonic jitter clock:** `SystemTime` → `std::time::Instant` in rx_callback; acc fields `i_sq_sum`/`q_sq_sum` changed `i64` → `u64`
- **WaterfallBuffer OOM:** early return when `max_rows == 0`
- **sysconf() -1:** guarded `_SC_CLK_TCK` error sentinel before cast to `u64`
- **Reset on hardware failure:** `reset_to_defaults()` now only called when all `set_*()` calls succeed
- **Header height guard:** `if inner.height < 3 { return; }` prevents rendering outside panel bounds
- **SNR stale guard:** `signal_metrics` shows `"---"` before first FFT frame instead of `0.0 dB [CRIT]`
- **IQ diagnostics stale:** added `[STALE]` title + `"---"` rows when not streaming
- **Spectrum x_bounds off-by-one:** `[0.0, n]` → `[0.0, n - 1.0]`; noise floor line corrected
- **Waterfall legend arm ordering:** bottom (`-120 dBFS`) arm checked before middle to fix shadowing at small heights

### BUG-006 — Spectrum dBFS scale + waterfall stale
→ [details](bugs/bug-006-spectrum-scale-waterfall-stale.md)

- **dBFS scale misaligned:** replaced `\n`-separated labels with `Vec<Line>` positioned at `(DB_MAX - db) / range * h` rows; labels now align with canvas at any terminal height
- **Waterfall stale:** added `last_fft_frame` timestamp check; waterfall dims and shows `[STALE]` in sync with spectrum when RX stops

### IMP-004 — Spectrum display overhaul
→ [details](improvements/imp-004-spectrum-display-overhaul.md)

- **Fixed y-range:** removed per-frame dynamic zoom; canvas always spans `DB_MIN…DB_MAX`; no more bounce
- **Filled columns:** per-bin vertical `CanvasLine` from bottom + outline polyline on top; spectrum grounded to panel bottom
- **Focus key highlight:** `e` in `Spectrum` title rendered in `value_hi + BOLD`; focus shortcut self-documenting
- **Focus system simplified:** removed `focus_key()` from all panels except spectrum; 6 unused bindings gone
- **Preset reorder:** `2`=spectrum · `3`=waterfall · `4`=spectrum+waterfall · `5`=monitoring · `6`=lab

### IMP-005 — Spectrum focus tuning
→ [details](improvements/imp-005-spectrum-focus-tuning.md)

- **Frequency navigation:** `←`/`→` tunes center frequency by step when spectrum is focused
- **Step control:** `[`/`]` cycles through 9 step presets (1 kHz → 10 MHz) in focus mode, overriding global VGA keys
- **Tuning indicator:** `────◀  92.800 MHz  ▶────  step 100 kHz  [/]` appears as one row at canvas bottom in focus mode; absent otherwise

### IMP-006 — Spectrum analysis tools
→ [details](improvements/imp-006-spectrum-analysis-tools.md)

- **Band plan overlay:** 14 built-in bands (FM, AIR, 2m, 70cm, Marine, ISM, GPS, CELL…) rendered as dim labels at the top of the canvas when in the visible range
- **Zoom** (`↑`/`↓` focus): adjusts `y_min` in 10 dB steps; dBFS axis tracks the current range dynamically
- **Hold** (`H` global): snapshots the current FFT frame as a dim ghost polyline behind the live spectrum; `[HOLD]` shown in title
- **Cursor** (`J`/`K` focus): vertical line across the spectrum; indicator row shows cursor frequency and power in dBFS
- **Named markers** (`M` focus): places a named vertical marker at cursor or signal peak; footer opens name input (`Enter` = confirm, empty = auto-label `M1/M2/…`); markers persist in `~/.config/sdrtop/config.toml`

### IMP-007 — Spectrum panel UX fixes
→ [details](improvements/imp-007-spectrum-panel-ux-fixes.md)

- **Tuning indicator centering:** `left_arm` calculation decoupled from `right_info` length; `◀ MHz ▶` now sits at true centre of the indicator row regardless of cursor/step text length
- **Frame rate cap:** `run()` loop restructured so `terminal.draw()` fires at most every 33 ms (~30 fps); keyboard-repeat events update state but skip renders until the frame interval elapses; single key press still redraws on the next Tick (≤ 100 ms)

### IMP-008 — Performance overhaul
→ [details](improvements/imp-008-performance-overhaul.md)

- **`Arc<Vec<f32>>` for shared spectrum data:** `FftFrame.bins_dbfs`, `FftFrame.peak_hold`, `WaterfallBuffer.rows`, `spectrum_hold` — `SdrMetrics::clone()` drops from ~528 KB/frame to negligible at 30 fps
- **FFT scratch pre-allocation:** `samples`, `mags`, `shifted`, `noise_scratch`, `occ_scratch` declared once before the receive loop; ~88 KB/frame heap churn eliminated
- **Cursor-based drain:** single `buf.drain(..buf_start)` per USB chunk instead of one O(n) drain per FFT frame
- **Noise floor O(n):** `select_nth_unstable_by` replaces full `sort_by` for the bottom-10% mean
- **Spectrum canvas downsampling:** bins max-pooled to `canvas_area.width` columns before drawing; ~6 000 draw calls/frame → ~600
- **Closure data reduced:** `col_data` / `col_peaks` / `held_data` arrays (~10 KB) replace the old 8 KB `bin_colors` Vec + moved `Arc<Vec<f32>>`; closure captures no bins or Theme
- **`ColorDepth::detect()` cached:** `OnceLock` so `env::var` runs once at startup; all subsequent calls are a single atomic load

### IMP-009 — Waterfall focus panel
→ [details](improvements/imp-009-waterfall-focus-panel.md)

- **Focus mode (`l`):** `Waterfa`**`l`**`l` title highlights the activation key; border switches to `border_focused`; footer shows waterfall-specific bindings; `Esc` exits and resets scroll + cursor
- **Colour scale zoom (`↑`/`↓` focus):** adjusts `waterfall_db_min` in 10 dB steps (range: −120…−20 dBFS); dBFS legend tracks the current range; persists after exit
- **Scroll history (`J`/`K` focus):** scrolls through stored rows; offset shown as `[↑N]` in title; reset on `Esc`
- **Row stride (`[`/`]` focus):** averages N FFT frames into one waterfall row (×1 → ×64); slows scroll rate, extends visible history (×64 ≈ 26 s at 10 MHz); stride shown as `[×N]` in title
- **Frequency cursor (`M`/`←`/`→` focus):** vertical `│` line at chosen frequency; indicator row shows `cur: freq  dBFS  N s ago`; row timestamps stored per push enable accurate elapsed time
- **Band plan overlay:** same 14 allocations as spectrum panel; `BAND_PLAN` constant moved to shared `src/ui/band_plan.rs` so both panels draw from one source
- **CPU measurement fix:** `last_ticks` seeded from current value at task start — eliminates the artificial 100% spike on first reading
- **FFT throttle:** state updates capped at ~30 fps (`UPDATE_INTERVAL = 33 ms`); EMA + peak-hold still run on every frame; reduces noise-floor sort + occupied-BW sort from ~4 882/s to ~30/s

---

## 2026-05-29

### IMP-003 — Spectrum & Waterfall UI Fixes
→ [details](improvements/imp-003-spectrum-waterfall-ui-fixes.md)

- **Spectrum border:** replaced three partial-border blocks with a single outer `Block::ALL`; eliminated double `╭` corner and unclosed bottom-right edge
- **Freq labels:** changed from fixed 12-char padding to `canvas_area.width`-proportional spacing; labels now land at 0 / 25 / 50 / 75 / 100% of the actual canvas width
- **Axis alignment:** waterfall canvas offset by 6 chars to match spectrum's dB-label column; same frequency now hits the same terminal column in both panels
- **Spectrum paint:** replaced filled Braille bars (per-bin vertical line from DB_MIN) with a gradient polyline (adjacent bin tops connected); cleaner outline without the dense fill
- **Waterfall legend:** left 6-char column now shows a dBFS color scale (`█` strip with themed gradient + labels at +0 / −60 / −120 dBFS)

---

## 2026-05-28

### Phase 12 — UI/UX Polish & Theme System 🔧 In progress
→ [log](<phases/Phase 12 - UI UX Polish Theme System - Log.md>)

- Theme system: `Theme` struct with 6 built-in palettes (`sdr`, `nord`, `dracula`, `gruvbox`, `catppuccin`, `solarized`); `[theme]` TOML section with per-field `#rrggbb` overrides; `--theme` CLI flag
- All 13 panels migrated to `BorderType::Rounded`; three border tiers by panel role; stale / observer states get dedicated colors
- Spectrum: per-bin gradient computed outside Canvas closure (ratatui 0.26 `'static` constraint workaround)
- `HeaderPanel` redesigned as stateless; reads live from `SdrMetrics`
- `FooterPanel` redesigned with four context modes: observer / text-input / panel-focused / normal
- Panel focus system: 7 panels register focus keys (`e o h c m i g`); `LayoutEngine` tracks `focused_panel`; `Esc` exits

### IMP-002 — Observer Mode ✅ (alpha)
→ [plan](improvements/imp-002-observer-mode.md) · [log](improvements/imp-002-observer-mode-log.md)

- When another app holds the HackRF exclusively, sdrtop enters observer mode instead of crashing
- Reads device identity, USB stats, and owner process info from sysfs + `/proc` — no libhackrf needed
- New `ObserverPanel`; dedicated `observer` preset; all hardware controls silently gated

### IMP-001 — Interactive Sample Rate Control ✅
→ [details](improvements/imp-001-sample-rate-control.md)

- `[S]` key opens a sample-rate input mode in the footer
- Validates 2–20 MHz range; calls `device.set_sample_rate()` on confirm; rejects invalid input with log message

---

## Earlier phases

Exact dates not recorded for phases 1–11. See individual phase logs for details.

### Phase 11 — HackRF Deep Diagnostics ✅
→ [steps](<phases/Phase 11 - HackRF Deep Diagnostics - Steps.md>) · [log](<phases/Phase 11 - HackRF Deep Diagnostics - Log.md>)

New data: board revision, USB API version, CPLD checksum, SNR, channel power, occupied BW, IQ amplitude histogram.  
New panels: `RfChainPanel`, `SignalMetricsPanel`, `IqHistogramPanel`. New preset: `lab` (key `6`).

### Phase 10 — Configuration & Persistence ✅
→ [steps](<phases/Phase 10 - Configuration & Persistence - Steps.md>) · [log](<phases/Phase 10 - Configuration & Persistence - Log.md>)

Radio settings and display state persist across restarts via `~/.config/sdrtop/config.toml`.  
Atomic save (write `.tmp` → rename); CLI args override file; missing/corrupt file silently defaults.

### Phase 9 — Waterfall Display ✅
→ [steps](<phases/Phase 9 - Waterfall Display - Steps.md>) · [log](<phases/Phase 9 - Waterfall Display - Log.md>)

Scrolling 2D spectrum history as background-colored terminal cells.  
`ColorDepth` detection; truecolor piecewise gradient (6 stops, dark blue → red); 16-color fallback.  
New presets: `waterfall` (key `4`), `spectrum_waterfall` (key `5`).

### Phase 8 — FFT Spectrum Analyzer ✅
→ [8a steps](<phases/Phase 8a - FFT Pipeline - Steps.md>) · [8b steps](<phases/Phase 8b - Spectrum Display - Steps.md>) · [log](<phases/Phase 8 - FFT Spectrum Analyzer - Log.md>)

Live Braille canvas spectrum with peak hold, noise floor, dBFS and frequency axes.  
`FftWorker`: Hann window → rustfft → dBFS → fftshift → EMA smoothing → peak-hold decay → noise floor.  
`RxContext` pattern for safe FFI callback; lock released before IQ buffer allocation.

### Phase 7 — Hardware Health Panels ✅
→ [steps](<phases/Phase 7 - Hardware Health Panels - Steps.md>) · [log](<phases/Phase 7 - Hardware Health Panels - Log.md>)

New panels: `HardwareHealthPanel` (drops, ADC saturation, jitter), `IqDiagnosticsPanel`, `SystemResourcesPanel`.  
Accumulator pattern: integer sums in `rx_callback`, float metrics computed in polling task.

### Phase 6 — Dashboard Engine ✅
→ [steps](<phases/Phase 6 - Dashboard Engine - Steps.md>) · [log](<phases/Phase 6 - Dashboard Engine - Log.md>)

`Panel` trait + `PanelRegistry` + `LayoutEngine` with top/body/bottom zones and left/center/right columns.  
`LayoutConfig` is serde-deserializable — preset loading in Phase 10 required no engine changes.

### Phase 5 — Interactive Controls ✅
→ [steps](<phases/Phase 5 - Interactive Controls - Steps.md>) · [log](<phases/Phase 5 - Interactive Controls - Log.md>)

Full keyboard control: LNA, VGA, AMP, frequency input, reset. `InputMode` enum drives two-level event loop.  
Help overlay rendered last in `draw()` to layer on top of all panels.

### Phase 4 — Architecture Refactor ✅
→ [steps](<phases/Phase 4 - Architecture Refactor - Steps.md>) · [log](<phases/Phase 4 - Architecture Refactor - Log.md>)

`main.rs` reduced from ~670 lines to 43 — pure entry point.  
Six focused modules established; stub files added for all future phases.

### Phase 3 — TUI Dashboard ✅
→ [steps](<phases/Phase 3 - TUI Dashboard - Steps.md>) · [log](<phases/Phase 3 - TUI Dashboard - Log.md>)

Live ratatui dashboard: header, telemetry panel, gain gauges, 64-point throughput sparkline, log panel, footer.

### Phase 2 — Telemetry Polling & USB Throughput ✅
→ [steps](<phases/Phase 2 - Telemetry Polling - Steps.md>) · [log](<phases/Phase 2 - Telemetry Polling - Log.md>)

`Arc<Mutex<SdrMetrics>>` shared between UI thread and tokio background task.  
`rx_enabled` / `hw_streaming` split fixed a dual-state bug in the original design.

### Phase 1 — Device Discovery ✅
→ [steps](<phases/Phase 1 - Device Discovery - Steps.md>) · [log](<phases/Phase 1 - Device Discovery - Log.md>)

Hand-crafted `#[repr(C)]` FFI layer; critical `HackrfDeviceList` struct layout fixed.  
Safe `Device` wrapper with `Drop` ensuring `hackrf_exit()` on all exit paths.
