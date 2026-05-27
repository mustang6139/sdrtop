# sdrtop

> A `btop`-inspired terminal monitor for HackRF One — written in Rust.

**Status: early development. The foundation is in place; most features are not yet built.**

---

## What it does (so far)

- Opens a HackRF One via a custom libhackrf FFI layer
- Polls hardware state every 200 ms: frequency, gain settings, streaming status
- Measures live USB throughput from the RX callback
- Displays everything in a live ratatui TUI: telemetry panel, gain gauges, throughput sparkline, log

## What it will do

Full interactive control of every radio parameter, a live FFT spectrum display, waterfall history, config persistence, and multi-device support. See the [roadmap](docs/Roadmap.md) for the full plan.

The current roadmap targets HackRF One and PortaPack H1/H2 (Mayhem firmware) — that's the hardware on hand. The long-term goal is to cover all common SDR devices: RTL-SDR, LimeSDR, Airspy, and others.

---

## Requirements

- Linux
- HackRF One
- `libhackrf` installed:
  - Arch: `sudo pacman -S hackrf pkgconf`
  - Debian/Ubuntu: `sudo apt install libhackrf-dev pkg-config`
- Rust stable toolchain

## Build & run

```sh
cargo build --release
./target/release/sdrtop
```

## Keys

| Key | Action |
|---|---|
| `Space` | Start / stop RX |
| `r` | Reset settings to defaults |
| `q` | Quit |

More keys coming in Phase 5.

---

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).

---

## A note on how this was built

The architecture — what modules to create, how to structure shared state, which problems to solve first and why — was designed through conversation with an AI. The code itself was written by hand. The distinction felt worth noting: AI as a thinking partner, not a code generator.
