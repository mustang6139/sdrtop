# BUG-002 — USB-C Porton Instabil HackRF Streaming

← [Bug Tracker](README.md)

**Phase:** Platform-szintű (nem phase-specifikus)  
**Státusz:** ⚠️ Workaround  
**Felfedezve:** 2026-05-28  
**Javítva:** —  

---

## Tünet

- USB-C porton csatlakoztatott HackRF esetén az RX streaming közben az app összeomlik vagy a kapcsolat megszakad.
- USB-A porton ugyanaz a HackRF stabilan működik.
- Az összeomlás után HackRF nem tud újra csatlakozni — USB replug szükséges.

---

## Gyökérok

A HackRF One USB 2.0 High Speed (480 Mbit/s) izochronus átvitelt használ a streaming során, ami 20 Msps sample rate-nél ~40 MB/s adatáramot jelent. Ez nagy és folyamatos USB sávszélességet igényel.

USB-C portokon ez több okból is problémás lehet:

| Ok | Magyarázat |
|---|---|
| **USB-C hub / adapter** | A legtöbb USB-C hub vagy adapter extra latenciát és timing overhead-et vezet be az izochronus átvitelbe |
| **Alt mode negotiation** | Ha a port Thunderbolt vagy DisplayPort alt mode-on van, az USB controller megosztja a sávszélességet |
| **USB 3.x fallback** | Egyes USB-C kontrollerek nem kezelik jól a USB 2.0 HS izochronus eszközöket USB 3.x módban |
| **Power delivery interference** | USB-C PD tárgyalás megszakíthatja a folyamatos adatátvitelt |
| **Platform-specifikus driver quirk** | Linux usbfs/xhci driver viselkedése eltérhet USB-C kontrollereken |

Az eredmény: `hackrf_transfer.valid_length < hackrf_transfer.buffer_length` — a HackRF csökkentett vagy korrupt buffereket küld, ami végül libhackrf-szintű hibára, vagy az app összeomlására vezet.

---

## Jelenlegi workaround

**Használj USB-A portot.** Ha a laptopon csak USB-C port van:

1. Aktív (powered) USB hub USB-A kimenettel
2. USB-C → USB-A adapter (passzív is működhet, ha a port natívan USB 2.0 fallbacket csinál)

---

## Jövőbeli fejlesztési irányok

### 1. Graceful disconnect detection

A polling task jelenleg `device_bg.is_streaming()` értékét olvassa. Ha a streaming váratlanul leáll (libhackrf error), ez detektálható:

```rust
// polling task-ban:
if hw_rx_active && !device_bg.is_streaming() {
    // streaming leállt külső ok miatt (USB error)
    hw_rx_active = false;
    state_bg.lock()...push_log("WARNING: Streaming stopped unexpectedly (USB error?)");
    state_bg.lock()...rx_enabled = false;
}
```

Ez lehetővé teszi, hogy az app **ne omoljon össze**, hanem gracefully kezelje az USB problémát.

### 2. USB transfer error counter

A `hackrf_transfer.valid_length < hackrf_transfer.buffer_length` feltétel jelenleg drop-ként van számlálva. Érdemes lenne megkülönböztetni:
- **Sample drop**: a HackRF buffer túlcsordult (tipikusan CPU bottleneck)
- **USB transfer error**: a transfer maga rövidebb volt (USB instabilitás jele)

```rust
// Jelenlegi kód — mindkét esetet drop-nak számolja:
if t.valid_length < t.buffer_length {
    m.acc_drops += ((t.buffer_length - t.valid_length) / 2) as u64;
}

// Jövőbeli verzió — megkülönbözteti:
if t.valid_length == 0 {
    m.acc_usb_errors += 1;  // teljes transfer failure
} else if t.valid_length < t.buffer_length {
    m.acc_drops += ((t.buffer_length - t.valid_length) / 2) as u64;
}
```

### 3. Automatic reconnect

Ha a HackRF leválik (USB error vagy fizikai eltávolítás), a jelenlegi app kilép. Egy jövőbeli verzióban:
- Detect disconnect (libhackrf error kód alapján)
- Gracefully stop UI streaming
- Polling loop-ban várj újra megjelenő eszközre
- Automatikusan reconnect és folytatás

### 4. USB-C specifikus warning

Ha a csatlakoztatott USB kontroller USB-C-s (pl. `/sys/bus/usb/devices/*/bcdUSB` alapján), egy egyszeri figyelmeztetés a log-ban:

```
WARNING: HackRF connected via USB-C — if streaming is unstable, try a USB-A port.
```

---

## Kapcsolódó

- [BUG-001](bug-001-iq-histogram-oob.md) — Az OOB crash szintén hackreq replugot igényelt, ezért az USB-C crashel össze lehetett keverni a két hibát.
