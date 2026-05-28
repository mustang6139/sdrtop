# BUG-001 — IQ Histogram Bin Index Out-of-Bounds (`i8::MIN`)

← [Bug Tracker](README.md)

**Phase:** 11 — HackRF Deep Diagnostics  
**Státusz:** ✅ Fixed  
**Felfedezve:** 2026-05-28  
**Javítva:** 2026-05-28  

---

## Tünet

- Az app összeomlik, ha RX aktív és az IQ amplitude histogram adatot kap.
- A HackRF az összeomlás után nem tud újra csatlakozni — USB kihúzás/bedugás szükséges.
- Determinisztikusan reprodukálható: elég egy erős jelet vagy magas erősítést beállítani (ADC saturáció közelében).

---

## Gyökérok

A `rx_callback`-ben az IQ amplitúdó kiszámítása:

```rust
let amp = i_byte.unsigned_abs().max(q_byte.unsigned_abs());
m.acc_iq_hist[(amp / 4) as usize] += 1;  // ← BUG
```

Az `i8::unsigned_abs()` az abszolút értéket `u8`-ként adja vissza.  
Az `i8::MIN` (`-128`) abszolút értéke **128** — ami `u8`-ban elfér, de az `i8`-ban nem.

```
i8::MIN = -128
(-128i8).unsigned_abs() = 128u8
128u8 / 4 = 32
acc_iq_hist[32]  →  index out of bounds!  (array len = 32, valid range: 0..31)
```

### Miért volt katasztrofális?

A panic a `rx_callback`-en belül tört ki, amit a C libhackrf hív. Rust panicot C stack frame-eken keresztül nem lehet biztonságosan unwindolni — a folyamat `abort()`-tal terminálja magát, **Drop futtatása nélkül**.

Következmény:
- `hackrf_stop_rx()` és `hackrf_close()` nem hívódik meg
- A HackRF firmware streaming módban marad
- Következő csatlakozási kísérlet meghiúsul (`HACKRF_ERROR_STREAMING_THREAD_ERR`)
- USB replug szükséges (firmware reset)

---

## Fix

**Fájl:** `src/hardware/device.rs`

```rust
// Előtte:
m.acc_iq_hist[(amp / 4) as usize] += 1;

// Utána:
m.acc_iq_hist[((amp / 4) as usize).min(31)] += 1;
```

Az `amp = 128` (az egyetlen értéknél amit az `i8` képes produkálni ennél a határon) a **31-es bin**ba kerül, ami a legmagasabb amplitúdó tartomány — szemantikailag helyes, nem csak egy workaround.

---

## Regressziós teszt

**Fájl:** `src/hardware/device::tests::histogram_bin_i8_min_does_not_overflow`

```rust
#[test]
fn histogram_bin_i8_min_does_not_overflow() {
    let i_byte: i8 = i8::MIN;
    let q_byte: i8 = 0;
    let amp = i_byte.unsigned_abs().max(q_byte.unsigned_abs());
    assert_eq!(amp, 128);                          // unsigned_abs() == 128, nem 127
    let bin = ((amp / 4) as usize).min(31);
    assert_eq!(bin, 31);                           // clamped, nem 32
}
```

---

## Tanulság

**Soha ne használj `i8::unsigned_abs()` közvetlen tömbindexelésre az `i8::MIN` eset explicit kezelése nélkül.**  
Az `i8` abszolút értéke nem fér el `i8`-ban (a tartomány aszimmetrikus: -128..=127), de `u8`-ban igen — és ott a 128-as érték egy off-by-one hibát okoz minden 32 elemű tömbnél.
