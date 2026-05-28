# sdrtop — Bug Tracker

← [Home](../Home.md)

Ismert és javított hibák, phase-enként csoportosítva.  
Minden bug saját részletes fájlt kap; ez a dokumentum az index.

---

## Konvenciók

| Státusz | Jelentés |
|---|---|
| ✅ Fixed | A fix benne van a kódbázisban, regressziós teszt is van hozzá |
| ⚠️ Workaround | Ismert kerülő megoldás létezik, szoftveres fix még nem |
| 🔲 Open | Nincs megoldás, nyitott probléma |

---

## Phase 11 — HackRF Deep Diagnostics

| ID | Cím | Státusz |
|---|---|---|
| [BUG-001](bug-001-iq-histogram-oob.md) | IQ histogram bin index out-of-bounds (`i8::MIN`) | ✅ Fixed |

---

## Platform / Hardware

| ID | Cím | Státusz |
|---|---|---|
| [BUG-002](bug-002-usbc-streaming-instability.md) | USB-C porton instabil HackRF streaming | ⚠️ Workaround |

---

## Hogyan adj hozzá új bejegyzést

1. Hozz létre `docs/bugs/bug-NNN-rovid-leiras.md` fájlt az alábbi template alapján.
2. Add hozzá a fenti táblázathoz a megfelelő phase-szekcióba (vagy hozz létre új szekciót).
3. Ha a fix kódváltozással jár: hivatkozz a releváns fájlra és sorra.

### Template

```markdown
# BUG-NNN — Cím

**Phase:** N  
**Státusz:** ✅ Fixed / ⚠️ Workaround / 🔲 Open  
**Felfedezve:** YYYY-MM-DD  
**Javítva:** YYYY-MM-DD  

## Tünet
...

## Gyökérok
...

## Fix
...

## Regressziós teszt
...
```
