# sdrtop — Vault Home

> btop-inspired TUI for HackRF One & PortaPack SDR devices, written in Rust.

---

## Navigation

| Document | Purpose |
|---|---|
| [Roadmap](Roadmap.md) | Full project roadmap — phases, goals, risks |
| [Phase 1 - Device Discovery - Steps](phases/Phase%201%20-%20Device%20Discovery%20-%20Steps.md) | Phase 1 — intended approach |
| [Phase 1 - Device Discovery - Log](phases/Phase%201%20-%20Device%20Discovery%20-%20Log.md) | Phase 1 — FFI design, HackrfDeviceList struct bug |
| [Phase 2 - Telemetry Polling - Steps](phases/Phase%202%20-%20Telemetry%20Polling%20-%20Steps.md) | Phase 2 — intended approach |
| [Phase 2 - Telemetry Polling - Log](phases/Phase%202%20-%20Telemetry%20Polling%20-%20Log.md) | Phase 2 — dual is_streaming bug, rx_callback design |
| [Phase 3 - TUI Dashboard - Steps](phases/Phase%203%20-%20TUI%20Dashboard%20-%20Steps.md) | Phase 3 — intended approach |
| [Phase 3 - TUI Dashboard - Log](phases/Phase%203%20-%20TUI%20Dashboard%20-%20Log.md) | Phase 3 — borrow checker fix, misleading footer removal |
| [Phase 4 - Architecture Refactor - Steps](phases/Phase%204%20-%20Architecture%20Refactor%20-%20Steps.md) | Phase 4 — intended approach |
| [Phase 4 - Architecture Refactor - Log](phases/Phase%204%20-%20Architecture%20Refactor%20-%20Log.md) | Phase 4 — 670 → 43 lines, deviations from plan |

---

## Phase status

| Phase | Title | Status |
|---|---|---|
| 1 | Device discovery & basic info | ✅ Done |
| 2 | Telemetry polling & USB throughput | ✅ Done |
| 3 | TUI dashboard (gauges, sparkline, log, shortcuts) | ✅ Done |
| 4 | Architecture refactor (modular layout) | ✅ Done |
| 5 | Interactive controls | 🔲 Next |
| 6 | Dashboard engine (panel system, presets, layout config) | 🔲 Planned |
| 7 | Hardware health panels (drop rate, ADC saturation, IQ diagnostics) | 🔲 Planned |
| 8 | FFT spectrum analyzer | 🔲 Planned |
| 9 | Waterfall display | 🔲 Planned |
| 10 | Configuration & persistence | 🔲 Planned |
| 11 | Multi-device support | 🔲 Planned |
| 12 | PortaPack / Mayhem integration | 🔲 Planned |
| 13 | Polish & production readiness | 🔲 Planned |
| 14 | Distribution & community | 🔲 Planned |

---

## Docs convention

- **Steps file** — written *before* implementation; describes intended approach and sub-steps.
- **Log file** — written *after* implementation; records what actually happened, deviations, and decisions.
- Both files exist for every completed phase.
