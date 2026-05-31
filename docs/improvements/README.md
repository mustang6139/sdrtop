# sdrtop — Improvements

← [Home](../Home.md)

Out-of-phase additions: things that needed doing but didn't belong to a planned phase.  
Not bugs, not roadmap items — just polish, missing features, and practical gaps.

---

## Index

| ID | Title | Between phases | Added | Status |
|---|---|---|---|---|
| [IMP-001](imp-001-sample-rate-control.md) | Interactive sample rate control (`[S]` key) | 11→12 | 2026-05-28 | ✅ Done |
| [IMP-002](imp-002-observer-mode.md) · [log](imp-002-observer-mode-log.md) | Observer mode — monitor while another app holds the HackRF | 11→12 | 2026-05-28 | ✅ Done (alpha) |
| [IMP-003](imp-003-spectrum-waterfall-ui-fixes.md) | Spectrum & waterfall UI fixes (border, freq labels, axis alignment, dBFS legend) | 12→13 | 2026-05-29 | ✅ Done |
| [IMP-004](imp-004-spectrum-display-overhaul.md) | Spectrum display overhaul — fixed y-range, filled columns, focus key highlight, preset reorder | 12→13 | 2026-05-30 | ✅ Done |
| [IMP-005](imp-005-spectrum-focus-tuning.md) | Spectrum focus tuning — ←→ frequency navigation, step control, tuning indicator | 12→13 | 2026-05-30 | ✅ Done |
| [IMP-006](imp-006-spectrum-analysis-tools.md) | Spectrum analysis tools — band plan, zoom, hold, cursor, named markers | 12→13 | 2026-05-30 | ✅ Done |
| [IMP-007](imp-007-spectrum-panel-ux-fixes.md) | Spectrum panel UX fixes — tuning indicator centering, frame rate cap | 12→13 | 2026-05-30 | ✅ Done |
| [IMP-008](imp-008-performance-overhaul.md) | Performance overhaul — Arc shared buffers, FFT scratch pre-alloc, canvas downsampling | 12→13 | 2026-05-30 | ✅ Done |
| [IMP-009](imp-009-waterfall-focus-panel.md) | Waterfall focus panel — zoom, scroll, stride, frequency cursor, band plan overlay | 12→13 | 2026-05-30 | ✅ Done |

---

## How to add a new entry

1. Create `docs/improvements/imp-NNN-short-description.md` using the template below.
2. Add a row to the table above.
3. Add an entry to [CHANGELOG.md](../CHANGELOG.md) under the relevant date.
4. Update the improvements count in [Home.md](../Home.md) header line.

### Template

```markdown
# IMP-NNN — Title

← [Home](../Home.md)

**Added:** YYYY-MM-DD
**Between phases:** N → M

---

## Why

Why this needed doing outside the normal phase plan.

---

## What changed

| File | Change |
|---|---|
| `src/...` | ... |

---

## Behaviour

Before / after description or key bindings introduced.
```
