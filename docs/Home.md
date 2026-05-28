# sdrtop — Vault Home

> btop-inspired TUI for HackRF One & PortaPack SDR devices, written in Rust.

---

## Navigation

| Document                                                                                                                   | Purpose                                                                       |
| -------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| [Roadmap](Roadmap.md)                                                                                                      | Full project roadmap — phases, goals, risks                                   |
| [Bug Tracker](bugs/README.md)                                                                                              | Ismert és javított hibák, phase-enként csoportosítva                          |
| [Phase 1 - Device Discovery - Steps](phases/Phase%201%20-%20Device%20Discovery%20-%20Steps.md)                             | Phase 1 — intended approach                                                   |
| [Phase 1 - Device Discovery - Log](phases/Phase%201%20-%20Device%20Discovery%20-%20Log.md)                                 | Phase 1 — FFI design, HackrfDeviceList struct bug                             |
| [Phase 2 - Telemetry Polling - Steps](phases/Phase%202%20-%20Telemetry%20Polling%20-%20Steps.md)                           | Phase 2 — intended approach                                                   |
| [Phase 2 - Telemetry Polling - Log](phases/Phase%202%20-%20Telemetry%20Polling%20-%20Log.md)                               | Phase 2 — dual is_streaming bug, rx_callback design                           |
| [Phase 3 - TUI Dashboard - Steps](phases/Phase%203%20-%20TUI%20Dashboard%20-%20Steps.md)                                   | Phase 3 — intended approach                                                   |
| [Phase 3 - TUI Dashboard - Log](phases/Phase%203%20-%20TUI%20Dashboard%20-%20Log.md)                                       | Phase 3 — borrow checker fix, misleading footer removal                       |
| [Phase 4 - Architecture Refactor - Steps](phases/Phase%204%20-%20Architecture%20Refactor%20-%20Steps.md)                   | Phase 4 — intended approach                                                   |
| [Phase 4 - Architecture Refactor - Log](phases/Phase%204%20-%20Architecture%20Refactor%20-%20Log.md)                       | Phase 4 — 670 → 43 lines, deviations from plan                                |
| [Phase 5 - Interactive Controls - Steps](phases/Phase%205%20-%20Interactive%20Controls%20-%20Steps.md)                     | Phase 5 — intended approach                                                   |
| [Phase 5 - Interactive Controls - Log](phases/Phase%205%20-%20Interactive%20Controls%20-%20Log.md)                         | Phase 5 — InputMode design, event loop restructure, gain clamping             |
| [Phase 6 - Dashboard Engine - Steps](phases/Phase%206%20-%20Dashboard%20Engine%20-%20Steps.md)                             | Phase 6 — intended approach                                                   |
| [Phase 6 - Dashboard Engine - Log](phases/Phase%206%20-%20Dashboard%20Engine%20-%20Log.md)                                 | Phase 6 — Panel trait, LayoutEngine, left_pct bug fix                         |
| [Phase 7 - Hardware Health Panels - Steps](phases/Phase%207%20-%20Hardware%20Health%20Panels%20-%20Steps.md)               | Phase 7 — intended approach                                                   |
| [Phase 7 - Hardware Health Panels - Log](phases/Phase%207%20-%20Hardware%20Health%20Panels%20-%20Log.md)                   | Phase 7 — accumulator pattern, clippy checked_div fix                         |
| [Phase 8a - FFT Pipeline - Steps](phases/Phase%208a%20-%20FFT%20Pipeline%20-%20Steps.md)                                   | Phase 8a — RxContext, crossbeam channel, DSP, FftWorker                       |
| [Phase 8b - Spectrum Display - Steps](phases/Phase%208b%20-%20Spectrum%20Display%20-%20Steps.md)                           | Phase 8b — SpectrumPanel, Canvas rendering, spectrum preset                   |
| [Phase 8 - FFT Spectrum Analyzer - Log](phases/Phase%208%20-%20FFT%20Spectrum%20Analyzer%20-%20Log.md)                     | Phase 8 — DSP pipeline, lock discipline, Canvas layout, deviations            |
| [Phase 9 - Waterfall Display - Steps](phases/Phase%209%20-%20Waterfall%20Display%20-%20Steps.md)                           | Phase 9 — WaterfallBuffer, palette, span rendering, spectrum_waterfall preset |
| [Phase 9 - Waterfall Display - Log](phases/Phase%209%20-%20Waterfall%20Display%20-%20Log.md)                               | Phase 9 — ring buffer design, palette tiers, span rendering, overlay fix      |
| [Phase 10 - Configuration & Persistence - Steps](phases/Phase%2010%20-%20Configuration%20%26%20Persistence%20-%20Steps.md) | Phase 10 — AppConfig, load/save, clap CLI, App::new() wiring                  |
| [Phase 10 - Configuration & Persistence - Log](phases/Phase%2010%20-%20Configuration%20%26%20Persistence%20-%20Log.md)     | Phase 10 — per-field serde defaults, lock discipline, scope decisions         |
| [Phase 11 - HackRF Deep Diagnostics - Steps](phases/Phase%2011%20-%20HackRF%20Deep%20Diagnostics%20-%20Steps.md)           | Phase 11 — board rev, CPLD, SNR, channel power, OBW, IQ histogram             |
| [Phase 11 - HackRF Deep Diagnostics - Log](phases/Phase%2011%20-%20HackRF%20Deep%20Diagnostics%20-%20Log.md)               | Phase 11 — CPLD not in libhackrf, computed BB BW, histogram accumulator       |
| [Phase 12 - PortaPack Mayhem Integration - Steps](phases/Phase%2012%20-%20PortaPack%20Mayhem%20Integration%20-%20Steps.md) | Phase 12 — USB CDC/ACM protocol, PortaPackWorker, panel, preset               |

---

## Phase status

| Phase | Title | Status |
|---|---|---|
| 1 | Device discovery & basic info | ✅ Done |
| 2 | Telemetry polling & USB throughput | ✅ Done |
| 3 | TUI dashboard (gauges, sparkline, log, shortcuts) | ✅ Done |
| 4 | Architecture refactor (modular layout) | ✅ Done |
| 5 | Interactive controls | ✅ Done |
| 6 | Dashboard engine (panel system, presets, layout config) | ✅ Done |
| 7 | Hardware health panels (drop rate, ADC saturation, IQ diagnostics) | ✅ Done |
| 8 | FFT spectrum analyzer | ✅ Done |
| 9 | Waterfall display | ✅ Done |
| 10 | Configuration & persistence | ✅ Done |
| 11 | HackRF deep diagnostics | ✅ Done |
| 12 | PortaPack / Mayhem integration | 🔲 Planned |
| 13 | Multi-device support | 🔲 Planned |
| 14 | Polish & production readiness | 🔲 Planned |
| 15 | Distribution & community | 🔲 Planned |

---

## Docs convention

- **Steps file** — written *before* implementation; describes intended approach and sub-steps.
- **Log file** — written *after* implementation; records what actually happened, deviations, and decisions.
- Both files exist for every completed phase.
