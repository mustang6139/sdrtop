# sdrtop — Vault Home

> btop-inspired TUI for HackRF One & PortaPack SDR devices, written in Rust.

---

## Navigation

| Document | Purpose |
|---|---|
| [[Roadmap]] | Full project roadmap — phases, goals, risks |
| [[Phase 1 - Device Discovery - Steps]] | Phase 1 — intended approach |
| [[Phase 1 - Device Discovery - Log]] | Phase 1 — FFI design, HackrfDeviceList struct bug |
| [[Phase 2 - Telemetry Polling - Steps]] | Phase 2 — intended approach |
| [[Phase 2 - Telemetry Polling - Log]] | Phase 2 — dual is_streaming bug, rx_callback design |
| [[Phase 3 - TUI Dashboard - Steps]] | Phase 3 — intended approach |
| [[Phase 3 - TUI Dashboard - Log]] | Phase 3 — borrow checker fix, misleading footer removal |
| [[Phase 4 - Architecture Refactor - Steps]] | Phase 4 — intended approach |
| [[Phase 4 - Architecture Refactor - Log]] | Phase 4 — 670 → 43 lines, deviations from plan |

---

## Phase status

| Phase | Title | Status |
|---|---|---|
| 1 | Device discovery & basic info | ✅ Done |
| 2 | Telemetry polling & USB throughput | ✅ Done |
| 3 | TUI dashboard (gauges, sparkline, log, shortcuts) | ✅ Done |
| 4 | Architecture refactor (modular layout) | ✅ Done |
| 5 | Interactive controls | 🔲 Next |
| 6 | FFT spectrum analyzer | 🔲 Planned |
| 7 | Waterfall display | 🔲 Planned |
| 8 | Configuration & persistence | 🔲 Planned |
| 9 | Multi-device support | 🔲 Planned |
| 10 | PortaPack / Mayhem integration | 🔲 Planned |
| 11 | Polish & production readiness | 🔲 Planned |
| 12 | Distribution & community | 🔲 Planned |

---

## Docs convention

- **Steps file** — written *before* implementation; describes intended approach and sub-steps.
- **Log file** — written *after* implementation; records what actually happened, deviations, and decisions.
- Both files exist for every completed phase.
