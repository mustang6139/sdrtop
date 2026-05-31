# sdrtop

[![Rust](https://img.shields.io/badge/rust-stable-orange?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-GPL--3.0-blue)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-linux-lightgrey?logo=linux&logoColor=white)]()
[![HackRF One](https://img.shields.io/badge/hardware-HackRF%20One-brightgreen)](https://greatscottgadgets.com/hackrf/one/)
[![PortaPack](https://img.shields.io/badge/hardware-PortaPack%20H4M-blueviolet)](https://github.com/portapack-mayhem/mayhem-firmware)

**Hey there! This is my take on a terminal monitor for SDR hardware.** I wanted something that could hunt down every bit of diagnostic data from your radio and stream it straight to your terminal.

I didn't want to cut corners, so this definitely isn't a lazy `hackrf_info` clone. It delivers raw, real-time metrics (spectrum, waterfall, ADC health, gain chain) right inside the terminal. It's lightweight, distraction-free, and fits perfectly into a tmux pane, an SSH session, or the custom screen of your cyberdeck.

**[Full user guide](user_docs/README.md)**

---

## Video

![](/dev_docs/pics/)

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
- **Layout presets** - switch on the fly with number keys

---

## Quick start

**Requirements:** Linux · HackRF One · `libhackrf` + `pkg-config` · Rust stable

```sh
# Arch
sudo pacman -S hackrf pkgconf

# Debian / Ubuntu
sudo apt install libhackrf-dev pkg-config
```

```sh
cargo build --release
./target/release/sdrtop
```

Press `Space` to start receiving. Press `?` for the key reference. Press `q` to quit and save.

---

## Keys

| Key       | Action                         |
| --------- | ------------------------------ |
| `Space`   | Start / stop RX                |
| `↑` / `↓` | LNA gain ±8 dB                 |
| `[` / `]` | VGA gain ±2 dB                 |
| `a`       | Toggle RF amplifier            |
| `f`       | Enter frequency (MHz)          |
| `s`       | Enter sample rate (2–20 MHz)   |
| `r`       | Reset all settings to defaults |
| `w`       | Pause / resume waterfall       |
| `h`       | Hold / unhold spectrum frame   |
| `e`       | Focus spectrum panel           |
| `l`       | Focus waterfall panel          |
| `1`–`6`   | Switch layout preset           |
| `p`       | Cycle presets                  |
| `?`       | Help overlay                   |
| `q`       | Quit and save config           |

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
