# sdrtop

> A `btop`-inspired terminal monitor for HackRF One — written in Rust.

sdrtop gives you a live view of your HackRF One's RF activity, hardware health, and signal diagnostics — all in the terminal. Think `htop`, but for your radio.

---

## What it does

- **Live RF monitoring** — frequency, gain settings, sample rate, USB throughput, streaming status
- **FFT spectrum analyzer** — real-time spectrum with EMA smoothing and peak hold
- **Waterfall display** — scrolling spectrogram history
- **Hardware health diagnostics** — sample drop rate, ADC saturation, IQ imbalance, DC offset, callback jitter, USB transfer errors
- **IQ amplitude histogram** — log-scale distribution of raw sample amplitudes with saturation warning
- **Signal metrics** — SNR, channel power (dBFS), occupied bandwidth (99% OBW)
- **RF chain panel** — computed baseband filter bandwidth, board revision, USB API version
- **Interactive controls** — live frequency input, LNA/VGA/AMP gain adjustment, config persistence
- **Multiple layout presets** — switch between views on the fly

---

## Requirements

- Linux
- HackRF One
- `libhackrf` installed:
  - Arch: `sudo pacman -S hackrf pkgconf`
  - Debian/Ubuntu: `sudo apt install libhackrf-dev pkg-config`
- Rust stable toolchain

---

## Build & run

```sh
cargo build --release
./target/release/sdrtop
```

Command-line options:

```sh
sdrtop --frequency 433920000   # center frequency in Hz
sdrtop --lna 24                # LNA gain 0–40 dB (step 8)
sdrtop --vga 30                # VGA gain 0–62 dB (step 2)
sdrtop --config ~/my.toml      # custom config file path
```

Settings are saved automatically to `~/.config/sdrtop/config.toml` on quit.

---

## Keys

| Key | Action |
|---|---|
| `Space` | Start / stop RX streaming |
| `↑` / `↓` | LNA gain +8 / −8 dB |
| `]` / `[` | VGA gain +2 / −2 dB |
| `a` | Toggle RF amplifier |
| `f` | Enter frequency in MHz |
| `r` | Reset all settings to defaults |
| `w` | Pause / resume waterfall |
| `p` | Cycle through presets |
| `1` | Preset: minimal |
| `2` | Preset: monitoring |
| `3` | Preset: spectrum |
| `4` | Preset: waterfall |
| `5` | Preset: spectrum + waterfall |
| `6` | Preset: lab (all panels) |
| `?` | Toggle help overlay |
| `q` | Quit |

---

## Roadmap

Phase 11 of 15 complete. Core functionality is stable and usable on real hardware.

| Phase | Title | Status |
|---|---|---|
| 1–11 | Device discovery, telemetry, TUI, controls, FFT, waterfall, config, diagnostics | ✅ Done |
| 12 | PortaPack / Mayhem integration | 🔲 Next |
| 13 | Multi-device support | 🔲 Planned |
| 14–15 | Polish & distribution | 🔲 Planned |

Full roadmap: [docs/Roadmap.md](docs/Roadmap.md)

---

## Known bugs

| ID | Description | Status |
|---|---|---|
| [BUG-001](docs/bugs/bug-001-iq-histogram-oob.md) | IQ histogram bin index out-of-bounds (`i8::MIN`) | ✅ Fixed |
| [BUG-002](docs/bugs/bug-002-usbc-streaming-instability.md) | Unstable HackRF streaming on USB-C port | ⚠️ Workaround |
| [BUG-003](docs/bugs/bug-003-iq-histogram-utf8-slice.md) | IQ histogram UTF-8 string slice panic (`█` multi-byte) | ✅ Fixed |

Full bug tracker: [docs/bugs/README.md](docs/bugs/README.md)

---

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).

---

## Built with Claude

Written by MusiThang and [Claude](https://claude.ai) (Anthropic).
