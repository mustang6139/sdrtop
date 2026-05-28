# sdrtop

**A terminal monitor for HackRF One — built in Rust, inspired by btop.**

If you've ever wanted to know what your radio is actually doing — not just "it's receiving" but *how much* signal is dropping, whether your ADC is saturating, what your IQ imbalance looks like, what's happening in the spectrum right now — sdrtop is for that.

It runs entirely in the terminal. No GUI, no Electron, no browser. A real-time dashboard you can `ssh` into, run on a Raspberry Pi taped to a cyberdeck, or keep open in a tmux pane next to your SDR pipeline.

---

## Who it's for

**SDR tinkerers and RF engineers** who want more than `hackrf_info` but less than a full GUI app. You're capturing IQ, scanning frequencies, doing signal analysis — and you want to see what the hardware is doing while you do it.

**Cyberdeck and embedded Linux users** who live in the terminal. sdrtop is the kind of tool that feels at home next to `htop`, `iotop`, and `bmon` in a tiled terminal layout. No X required.

**People who run SDR++ or another app** alongside their HackRF — sdrtop has an observer mode that shows you device info, USB stats, and which process is holding the radio, even when you can't open it yourself.

---

## What it actually shows

```
┌─ RF Chain ─────────┐  ┌─ Spectrum ─────────────────────────────────────┐
│ Frequency  433.920 M│  │                  ░░░                           │
│ Sample rate  10.0 M │  │             ░░░░████░░░                         │
│ BB filter  10.0 MHz │  │        ░░░░░████████████░░░░                    │
│ LNA gain     16 dB  │  └────────────────────────────────────────────────┘
│ VGA gain     20 dB  │  ┌─ Hardware Health ──────────────────────────────┐
│ AMP          OFF    │  │ Drops: 0/s  (session: 0)                       │
│ Total gain   36 dB  │  │ ADC sat: 0.0%  (peak: 0.0%)                   │
│ Board    HackRF r9  │  │ Jitter: 42 µs                                  │
│ USB API   0x0102    │  │ USB errors: 0                                   │
└────────────────────┘  └────────────────────────────────────────────────┘
```

- **Spectrum analyzer** — FFT with EMA smoothing and peak hold
- **Waterfall** — scrolling spectrogram history (truecolor, 256-color, or 16-color)
- **Hardware health** — sample drops, ADC saturation, IQ imbalance, DC offset, USB errors
- **Signal metrics** — SNR, channel power (dBFS), 99% occupied bandwidth
- **RF chain** — baseband filter BW, board revision, total gain chain
- **IQ histogram** — amplitude distribution with saturation and weak-signal warnings
- **Observer mode** — shows device info and owner process when HackRF is in use by another app
- **Six layout presets** — switch on the fly with number keys

---

## Requirements

- Linux
- HackRF One
- `libhackrf` + `pkg-config`:
  - Arch: `sudo pacman -S hackrf pkgconf`
  - Debian/Ubuntu: `sudo apt install libhackrf-dev pkg-config`
- Rust stable

---

## Build & run

```sh
cargo build --release
./target/release/sdrtop
```

```sh
# Useful flags
sdrtop --frequency 433920000   # center frequency in Hz
sdrtop --lna 24 --vga 30       # starting gain
sdrtop --config ~/my.toml      # custom config path
```

Config saves automatically to `~/.config/sdrtop/config.toml` on quit.

---

## Keys

| Key | Action |
|---|---|
| `Space` | Start / stop RX |
| `↑` / `↓` | LNA gain ±8 dB |
| `[` / `]` | VGA gain ±2 dB |
| `a` | Toggle RF amp |
| `f` | Enter frequency (MHz) |
| `s` | Enter sample rate (2–20 MHz) |
| `r` | Reset to defaults |
| `w` | Pause / resume waterfall |
| `1`–`6` | Switch preset |
| `p` | Cycle presets |
| `?` | Help overlay |
| `q` | Quit |

---

## Status

Phases 1–11 done. Working on real hardware. PortaPack / Mayhem integration is next.

→ [Roadmap](docs/Roadmap.md) · [Bug tracker](docs/bugs/README.md) · [Docs home](docs/Home.md)

---

## Built with Claude

Written by MusiThang and [Claude](https://claude.ai).
