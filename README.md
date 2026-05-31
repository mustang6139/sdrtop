# sdrtop

[![Rust](https://img.shields.io/badge/rust-stable-orange?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-GPL--3.0-blue)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-linux-lightgrey?logo=linux&logoColor=white)]()
[![HackRF One](https://img.shields.io/badge/hardware-HackRF%20One-brightgreen)](https://greatscottgadgets.com/hackrf/one/)
[![PortaPack](https://img.shields.io/badge/hardware-PortaPack%20H4M-blueviolet)](https://github.com/portapack-mayhem/mayhem-firmware)
[![Ko-fi](https://img.shields.io/badge/Ko--fi-support%20the%20project-FF5E5B?logo=ko-fi&logoColor=white)](https://ko-fi.com/mustang6139)

**A terminal monitor for SDR hardware. Built to squeeze every piece of diagnostic data out of your radio and put it on screen, live.**

No fluff, and definitely not a lazy wrapper around `hackrf_info`. It delivers raw, real-time data: spectrum, waterfall, ADC health, gain chain, and signal metrics - entirely in the terminal, over SSH, in a tmux pane.

---

## Video

![](/docs/pics/sdrtop.gif)

---

## What it shows

- **Spectrum analyzer** - FFT with EMA smoothing, peak hold, noise floor, dBFS axis, zoom, band plan overlay, frequency markers
- **Waterfall** - scrolling spectrogram with truecolor / 256-color / 16-color support
- **Signal metrics** - SNR, channel power (dBFS), 99% occupied bandwidth
- **Hardware health** - sample drop rate, ADC saturation, IQ imbalance, DC offset
- **RF chain** - board revision, baseband filter BW, full gain chain
- **IQ histogram** - amplitude distribution; flags saturation and dynamic range issues
- **Observer mode** - device identity and owner process when another app holds the radio
- **Six themes** - `sdr` · `nord` · `dracula` · `gruvbox` · `catppuccin` · `solarized`
- **Six layout presets** - switch on the fly with number keys

---

## Requirements

- Linux
- HackRF One
- `libhackrf` + `pkg-config`
- Rust stable

```sh
# Arch
sudo pacman -S hackrf pkgconf

# Debian / Ubuntu
sudo apt install libhackrf-dev pkg-config
```

## Build & run

```sh
cargo build --release
./target/release/sdrtop
```

```sh
# Common options
sdrtop --frequency 92800000     # center frequency in Hz
sdrtop --lna 24 --vga 30        # initial gain settings
sdrtop --theme nord              # built-in theme
sdrtop --config ~/my.toml       # custom config path
```

---

## Keys

| Key | Action |
|---|---|
| `Space` | Start / stop RX |
| `↑` / `↓` | LNA gain ±8 dB |
| `[` / `]` | VGA gain ±2 dB |
| `a` | Toggle RF amplifier |
| `f` | Enter frequency (MHz) |
| `s` | Enter sample rate (2–20 MHz) |
| `r` | Reset all settings to defaults |
| `w` | Pause / resume waterfall |
| `h` | Hold / unhold spectrum frame |
| `e` | Focus spectrum panel |
| `1`–`6` | Switch layout preset |
| `p` | Cycle presets |
| `?` | Help overlay |
| `q` | Quit and save config |

### Spectrum focus mode (`e`)

| Key | Action |
|---|---|
| `←` / `→` | Tune center frequency by step |
| `[` / `]` | Step size (1 kHz → 10 MHz) |
| `↑` / `↓` | Zoom y-axis (±10 dB per step) |
| `j` / `k` | Move cursor left / right |
| `m` | Place or remove a named marker |
| `Esc` | Exit focus |

---

## Config

Saved automatically to `~/.config/sdrtop/config.toml` on quit. Hand-editing is safe.

```toml
[radio]
frequency_hz = 92800000
sample_rate  = 2000000.0
lna_gain     = 24
vga_gain     = 30
amp_enabled  = false

[display]
active_preset      = "main"
waterfall_max_rows = 64

# Spectrum markers persist here
[[display.spectrum_markers]]
freq_hz = 92800000
label   = "FM Radio"

[theme]
base = "nord"
# optional per-field overrides
# border_accent = "#88c0d0"
# value_hi      = "#ebcb8b"
```

Available themes: `sdr` (default) · `nord` · `dracula` · `gruvbox` · `catppuccin` · `solarized`

---

## Supported hardware

| Device                                 | Status            | Notes                                     |
| -------------------------------------- | ----------------- | ----------------------------------------- |
| HackRF One                             | ✅ Full support    | All diagnostics, gain stages, ADC metrics |
| PortaPack H4M (Mayhem)                 | 🔧 In development | Telemetry panel via CDC/ACM serial        |
| RTL-SDR (R820T, E4000, R828D)          | 🔲 Planned        | Most common SDR dongle                    |
| Airspy Mini / Airspy HF+               | 🔲 Planned        | Needs hardware                            |
| HackRF Pro                             | 🔲 Planned        | Needs hardware                            |
| LimeSDR / bladeRF / SDRplay / PlutoSDR | 🔲 Planned        | Needs hardware                            |

> Hardware support is added only after physical testing on real devices — no guessing from datasheets.  
> See [Supporting the project](#supporting-the-project) if you want to help expand this list.

---

## Roadmap

### What's possible with current hardware

| Phase | Milestone | Status |
|---|---|---|
| 1–11 | Core pipeline: FFT · waterfall · HackRF diagnostics · theme engine foundation | ✅ Done |
| 12 | UI/UX polish — full theme system, rounded panels, header/footer redesign, panel focus | 🔧 In progress |
| 13 | Hardware abstraction layer — `SdrDevice` trait, HackRF refactored as first backend | 🔲 Next |
| 14 | PortaPack / Mayhem — telemetry panel via Mayhem's serial interface | 🔲 Planned |
| 15 | HackRF feature pass — frequency bookmarks, IQ recording (SigMF), scan mode | 🔲 Planned |
| 16 | Polish — mouse support, terminal resize, panic recovery, performance profiling | 🔲 Planned |
| 17 | Distribution — AUR, CI, Nix flake, man page, `CONTRIBUTING.md` | 🔲 Planned |

### When hardware arrives

| Phase | Milestone | Hardware needed |
|---|---|---|
| 18 | RTL-SDR support — tuner detection, AGC, PPM correction | RTL-SDR dongle (~€25) |
| 19 | Airspy / Airspy HF+ — sensitivity presets, bias-T, HF diagnostics | Airspy Mini / HF+ (~€80–150) |
| 20 | SoapySDR backend — LimeSDR, bladeRF, SDRplay, PlutoSDR, USRP | Various |
| 21 | Multi-device — simultaneous monitoring, side-by-side spectrum | ≥2 different devices |
| 22 | Advanced UI — constellation display, demodulation preview, custom layouts | — |
| 23 | Community & ecosystem | — |

Full technical detail: [docs/Roadmap.md](docs/Roadmap.md)

---

## Supporting the project

`sdrtop` is built to support every SDR device out there, but that requires actually owning them. Right now development runs on a HackRF One and a PortaPack H4M. The next device on the list is an **RTL-SDR dongle**, which I'm buying myself - it's the most common SDR hardware in the world and the most impactful single addition this project can make.

The more expensive hardware (Airspy, LimeSDR, SDRplay) I'm saving toward as well, but that takes longer. If you use `sdrtop` and want to see support for your device sooner, contributions go directly toward hardware purchases. Every device that arrives gets a proper backend: tested on real hardware, documented, shipped.

| Device               | Why it matters                                                    | Price |
| -------------------- | ----------------------------------------------------------------- | ----- |
| RTL-SDR Blog V4      | Most common SDR dongle - immediate impact on user base            | ~€25  |
| Airspy Mini          | Clean 24–1700 MHz, popular with hams and scanner hobbyists        | ~€80  |
| Airspy HF+ Discovery | Best budget HF receiver, dedicated listener community             | ~€150 |
| LimeSDR Mini 2.0     | Full-duplex, wide range - opens up SoapySDR for dozens of devices | ~€160 |

No pressure, but if this scratches an itch for you, this is where it goes.

[![Ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/mustang6139)

---

## Status

Phase 11 complete · Phase 12 in progress. Running on real hardware. Next: finish UI polish, then hardware abstraction layer.

→ [Roadmap](docs/Roadmap.md) · [Changelog](docs/CHANGELOG.md) · [Docs](docs/Home.md) · [Credits](CREDITS.md)
