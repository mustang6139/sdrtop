# sdrtop — Development Hub

> btop-inspired universal SDR terminal monitor, written in Rust.

**Stack:** Rust · ratatui · libhackrf FFI · tokio · rustfft · crossbeam  
**Progress:** Phase 11 complete · Phase 12 in progress · 6 bugs tracked · 9 improvements logged

---

## Navigation

| Document                               | Purpose                                |
| -------------------------------------- | -------------------------------------- |
| [Roadmap](Roadmap.md)                  | Vision, upcoming phases (13–17), risks |
| [Changelog](CHANGELOG.md)              | Chronological milestone history        |
| [Bug tracker](bugs/README.md)          | Known, active, and resolved bugs       |
| [Improvements](improvements/README.md) | Out-of-phase additions                 |

---

## Phase progress

| # | Title | Status | Docs |
|---|---|---|---|
| 1 | Device discovery & basic info | ✅ Done | [steps](<phases/Phase 1 - Device Discovery - Steps.md>) · [log](<phases/Phase 1 - Device Discovery - Log.md>) |
| 2 | Telemetry polling & USB throughput | ✅ Done | [steps](<phases/Phase 2 - Telemetry Polling - Steps.md>) · [log](<phases/Phase 2 - Telemetry Polling - Log.md>) |
| 3 | TUI dashboard | ✅ Done | [steps](<phases/Phase 3 - TUI Dashboard - Steps.md>) · [log](<phases/Phase 3 - TUI Dashboard - Log.md>) |
| 4 | Architecture refactor | ✅ Done | [steps](<phases/Phase 4 - Architecture Refactor - Steps.md>) · [log](<phases/Phase 4 - Architecture Refactor - Log.md>) |
| 5 | Interactive controls | ✅ Done | [steps](<phases/Phase 5 - Interactive Controls - Steps.md>) · [log](<phases/Phase 5 - Interactive Controls - Log.md>) |
| 6 | Dashboard engine (panel system, presets) | ✅ Done | [steps](<phases/Phase 6 - Dashboard Engine - Steps.md>) · [log](<phases/Phase 6 - Dashboard Engine - Log.md>) |
| 7 | Hardware health panels | ✅ Done | [steps](<phases/Phase 7 - Hardware Health Panels - Steps.md>) · [log](<phases/Phase 7 - Hardware Health Panels - Log.md>) |
| 8 | FFT spectrum analyzer | ✅ Done | [8a](<phases/Phase 8a - FFT Pipeline - Steps.md>) · [8b](<phases/Phase 8b - Spectrum Display - Steps.md>) · [log](<phases/Phase 8 - FFT Spectrum Analyzer - Log.md>) |
| 9 | Waterfall display | ✅ Done | [steps](<phases/Phase 9 - Waterfall Display - Steps.md>) · [log](<phases/Phase 9 - Waterfall Display - Log.md>) |
| 10 | Configuration & persistence | ✅ Done | [steps](<phases/Phase 10 - Configuration & Persistence - Steps.md>) · [log](<phases/Phase 10 - Configuration & Persistence - Log.md>) |
| 11 | HackRF deep diagnostics | ✅ Done | [steps](<phases/Phase 11 - HackRF Deep Diagnostics - Steps.md>) · [log](<phases/Phase 11 - HackRF Deep Diagnostics - Log.md>) |
| 12 | UI/UX polish & theme system | 🔧 In progress | [12a](<phases/Phase 12a - Theme Foundation - Steps.md>) · [12b](<phases/Phase 12b - Panel Visual Updates - Steps.md>) · [12c](<phases/Phase 12c - Header Footer Focus - Steps.md>) · [log](<phases/Phase 12 - UI UX Polish Theme System - Log.md>) |
| 13 | Hardware abstraction layer (HAL) | 🔧 **Next** | — |
| 14 | PortaPack / Mayhem integration | 🔲 Planned | [steps](<phases/Phase 13 - PortaPack Mayhem Integration - Steps.md>) |
| 15 | HackRF feature pass (band plans, bookmarks, recording, scan) | 🔲 Planned | — |
| 16 | Polish & production readiness | 🔲 Planned | — |
| 17 | Distribution & community | 🔲 Planned | — |
| 18 | RTL-SDR support | ⏳ Needs hardware | — |
| 19 | Airspy / Airspy HF+ support | ⏳ Needs hardware | — |
| 20 | SoapySDR backend | ⏳ Needs hardware | — |
| 21 | Multi-device monitor | ⏳ Needs hardware | — |
| 22 | Advanced UI | 🔲 Planned | — |
| 23 | Ecosystem | 🔲 Planned | — |

---

## Out-of-phase improvements

Additions made between planned phases — not bugs, not roadmap items.

| ID | Title | Between | Status |
|---|---|---|---|
| [IMP-001](improvements/imp-001-sample-rate-control.md) | Interactive sample rate control (`[S]` key) | 11→12 | ✅ Done |
| [IMP-002](improvements/imp-002-observer-mode.md) · [log](improvements/imp-002-observer-mode-log.md) | Observer mode — monitor while another app holds the HackRF | 11→12 | ✅ Done |
| [IMP-003](improvements/imp-003-spectrum-waterfall-ui-fixes.md) | Spectrum & waterfall UI fixes (border, freq labels, axis alignment, dBFS legend) | 12→13 | ✅ Done |
| [IMP-004](improvements/imp-004-spectrum-display-overhaul.md) | Spectrum display overhaul — fixed y-range, filled columns, focus key, preset reorder | 12→13 | ✅ Done |
| [IMP-005](improvements/imp-005-spectrum-focus-tuning.md) | Spectrum focus tuning — ←→ frequency navigation, step control, tuning indicator | 12→13 | ✅ Done |
| [IMP-006](improvements/imp-006-spectrum-analysis-tools.md) | Spectrum analysis tools — band plan, zoom, hold, cursor, named markers | 12→13 | ✅ Done |
| [IMP-007](improvements/imp-007-spectrum-panel-ux-fixes.md) | Spectrum panel UX fixes — tuning indicator centering, frame rate cap | 12→13 | ✅ Done |
| [IMP-008](improvements/imp-008-performance-overhaul.md) | Performance overhaul — Arc shared buffers, FFT scratch pre-alloc, canvas downsampling | 12→13 | ✅ Done |
| [IMP-009](improvements/imp-009-waterfall-focus-panel.md) | Waterfall focus panel — zoom, scroll, stride, frequency cursor, band plan overlay | 12→13 | ✅ Done |

---

## Docs conventions

| File type | When written | Purpose |
|---|---|---|
| `Phase N - … - Steps.md` | Before implementation | Intended approach, sub-steps, expected tests |
| `Phase N - … - Log.md` | After implementation | What actually happened, deviations, key decisions |
| `bug-NNN-….md` | At discovery | Symptom, root cause, fix, regression test |
| `imp-NNN-….md` | After completion | Why, what changed, before/after behaviour |

Both Steps and Log exist for every completed phase. For planned phases, only Steps exist.
