# sdrtop — Bug Tracker

← [Home](../Home.md)

Known and fixed bugs, grouped by phase.  
Each bug has its own detailed file; this document is the index.

---

## Conventions

| Status | Meaning |
|---|---|
| ✅ Fixed | Fix is in the codebase; regression test included |
| ⚠️ Workaround | Known workaround exists; no software fix yet |
| 🔲 Open | No resolution; problem is open |

---

## Phase 11 — HackRF Deep Diagnostics

| ID | Title | Status |
|---|---|---|
| [BUG-001](bug-001-iq-histogram-oob.md) | IQ histogram bin index out-of-bounds (`i8::MIN`) | ✅ Fixed |
| [BUG-003](bug-003-iq-histogram-utf8-slice.md) | IQ histogram panel UTF-8 string slice panic (`█` multi-byte) | ✅ Fixed |

---

## Platform / Hardware

| ID | Title | Status |
|---|---|---|
| [BUG-002](bug-002-usbc-streaming-instability.md) | Unstable HackRF streaming on USB-C port | ⚠️ Workaround |

---

## Audit

| ID | Title | Status |
|---|---|---|
| [BUG-004](bug-004-comprehensive-audit.md) | Comprehensive code audit (2026-05-28) — 1 crash-risk fixed, 2 safe concerns documented | ✅ Fixed |

---

## How to add a new entry

1. Create `docs/bugs/bug-NNN-short-description.md` using the template below.
2. Add it to the table above under the appropriate phase section (or create a new section).
3. If the fix involves a code change, reference the relevant file and line.

### Template

```markdown
# BUG-NNN — Title

**Phase:** N  
**Status:** ✅ Fixed / ⚠️ Workaround / 🔲 Open  
**Discovered:** YYYY-MM-DD  
**Fixed:** YYYY-MM-DD  

## Symptom
...

## Root cause
...

## Fix
...

## Regression test
...
```
