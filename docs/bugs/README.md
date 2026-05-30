# sdrtop — Bug Tracker

← [Home](../Home.md)

Known and fixed bugs, grouped by the phase in which they were discovered.  
Each bug has its own detailed file; this document is the index.

---

## Status legend

| Status | Meaning |
|---|---|
| ✅ Fixed | Fix is in the codebase; regression test included where applicable |
| ⚠️ Workaround | Known workaround exists; no software fix possible or planned yet |
| 🔲 Open | No resolution; problem is open |

---

## Phase 11 — HackRF Deep Diagnostics

| ID | Title | Discovered | Status |
|---|---|---|---|
| [BUG-001](bug-001-iq-histogram-oob.md) | IQ histogram bin index out-of-bounds (`i8::MIN` → `usize` overflow) | Phase 11 | ✅ Fixed |
| [BUG-003](bug-003-iq-histogram-utf8-slice.md) | IQ histogram panel UTF-8 string slice panic (`█` multi-byte boundary) | Phase 11 | ✅ Fixed |

---

## Platform / Hardware

| ID | Title | Discovered | Status |
|---|---|---|---|
| [BUG-002](bug-002-usbc-streaming-instability.md) | Unstable HackRF streaming on USB-C port | Phase 2 | ⚠️ Workaround |

---

## Code Audits

| ID | Title | Date | Status |
|---|---|---|---|
| [BUG-004](bug-004-comprehensive-audit.md) | Comprehensive audit — 1 crash-risk fixed, 2 safe concerns documented | 2026-05-28 | ✅ Fixed |
| [BUG-005](bug-005-multi-bug-audit.md) | Phase 12 multi-bug audit — 9 bugs fixed (jitter clock, OOM, stale UI, off-by-one, reset safety) | 2026-05-30 | ✅ Fixed |
| [BUG-006](bug-006-spectrum-scale-waterfall-stale.md) | Spectrum dBFS scale misaligned on resize; waterfall not dimming on RX stop | 2026-05-30 | ✅ Fixed |

---

## How to add a new entry

1. Create `docs/bugs/bug-NNN-short-description.md` using the template below.
2. Add a row to the appropriate section above (or create a new section).
3. If the fix involves a code change, reference the relevant file and line number.

### Template

```markdown
# BUG-NNN — Title

← [Home](../Home.md)

**Phase:** N
**Status:** ✅ Fixed / ⚠️ Workaround / 🔲 Open
**Discovered:** YYYY-MM-DD
**Fixed:** YYYY-MM-DD

---

## Symptom

What the user or developer observed.

## Root cause

Why it happened.

## Fix

What was changed and where.

## Regression test

How to verify the fix holds.
```
