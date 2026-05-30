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
