# sdrtop

[![Rust](https://img.shields.io/badge/rust-stable-orange?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-GPL--3.0-blue)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-linux-lightgrey?logo=linux&logoColor=white)]()
[![HackRF One](https://img.shields.io/badge/hardware-HackRF%20One-brightgreen)](https://greatscottgadgets.com/hackrf/one/)
[![PortaPack](https://img.shields.io/badge/hardware-PortaPack%20H4M-blueviolet)](https://github.com/portapack-mayhem/mayhem-firmware)
[![Development Stage](https://img.shields.io/badge/stage-early%20development-red)]()

**Hey there! This is my take on a terminal monitor for SDR hardware.** I wanted something that could hunt down every bit of diagnostic data from your radio and stream it straight to your terminal.

I didn't want to cut corners, so this definitely isn't a lazy `hackrf_info` clone. It delivers raw, real-time metrics (spectrum, waterfall, ADC health, gain chain) right inside the terminal. It's lightweight, distraction-free, and fits perfectly into a tmux pane, an SSH session, or the custom screen of your cyberdeck.

> [!IMPORTANT]
> **Project Status:** `sdrtop` is currently in an **early development stage**. 
> * At the moment, **it only supports the HackRF One**. Support for other devices is planned.
> * **Known Issues:** Plenty 😄... You might run into some performance issues.

**[Full user guide](user_docs/README.md)**

---

## Video

![](/user_docs/pics/sdrtop.gif)

---

## What it shows

- **Spectrum analyzer** — FFT with EMA smoothing, peak hold, noise floor, dBFS axis, zoom, band plan overlay, frequency markers
- **Waterfall** — scrolling spectrogram with truecolor / 256-color / 16-color support
- **Signal strip** — live bar: P/NF · channel power · noise floor · ADC saturation · drops · buffer fill · IQ imbalance · RBW
- **RF chain** — frequency + wavelength (λ, λ/4), visual gain chain, estimated noise figure (Friis), minimum detectable signal (MDS), ADC utilisation gauge, gain advisor
- **IQ diagnostics** — DC offset (I/Q + magnitude gauge), DC spike level, amplitude/phase imbalance, image rejection ratio (IRR), contextual hint
- **Hardware health** — drops, ADC saturation, CPU/RAM, USB errors, sample-rate accuracy, buffer fill (all with trend sparklines)
- **IQ histogram** — ADC amplitude distribution with Low/Mid/Clip breakdown and PAPR (signal-type fingerprint); flags clipping and dynamic range issues
- **Observer mode** — device identity and owner process when another app holds the radio
- **Six themes** — `sdr` · `nord` · `dracula` · `gruvbox` · `catppuccin` · `solarized`
- **Layout presets** — general + specialised lab layouts, switch on the fly with number keys or cycle with `p`; define your own in the config

---

## Quick start

**Requirements:** Linux · HackRF One · `libhackrf` + `pkg-config` · Rust stable

### Arch

```sh
# Arch
sudo pacman -S hackrf pkgconf rust
```
### Debian / Ubuntu

```sh
sudo apt install libhackrf-dev pkg-config
```

You also need Rust installed. If you don't have it yet:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Then build:

```sh
cargo build --release
./target/release/sdrtop
```

Press `Space` to start receiving. Press `?` for the key reference. Press `q` to quit and save.

---

## Keys

| Key        | Action                         |
| ---------- | ------------------------------ |
| `Space`    | Start / stop RX                |
| `↑` / `↓` | LNA gain ±8 dB                 |
| `[` / `]`  | VGA gain ±2 dB                 |
| `a`        | Toggle RF amplifier            |
| `f`        | Enter frequency (MHz)          |
| `s`        | Enter sample rate (2–20 MHz)   |
| `r`        | Reset all settings to defaults |
| `w`        | Pause / resume waterfall       |
| `h`        | Hold / unhold spectrum frame   |
| `e`        | Focus spectrum panel           |
| `l`        | Focus waterfall panel          |
| `i` / `v` / `t` | Focus lab panel: IQ / hardware vitals / timing |
| `1`–`4`    | Switch built-in layout preset  |
| `5` / `6` / `7` / `8` | Lab presets: IQ / RF / timing / signal |
| `0`        | Micro field-mode view (compact, adapts to width) |
| `p`        | Cycle presets                  |
| `Tab`      | Toggle footer bar              |
| `?`        | Help overlay                   |
| `q`        | Quit and save config           |

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

You can also define your own `[presets.*]` layouts in the config — they merge with the built-ins and survive a save. See [user_docs/config.md](user_docs/config.md#custom-layout-presets).

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

> Hardware support is added only after physical testing on real devices - no guessing from datasheets.

---

## Roadmap

### Near term
- [ ] RTL-SDR support — R820T / R828D / E4000 (most common dongle, highest impact)

### Hardware pipeline
- [ ] Airspy Mini / Airspy HF+ Discovery
- [ ] HackRF Pro
- [ ] LimeSDR / bladeRF / SDRplay / PlutoSDR via SoapySDR

### App
- [ ] Frequency scanner mode
- [ ] Signal recording to file
- [ ] In-app config editing (no hand-editing TOML)

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

**[Credits](CREDITS.md)**