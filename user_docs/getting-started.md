# Getting Started

← [Back](README.md)

---

## What you need

- A Linux machine
- A HackRF One connected via USB
- The `libhackrf` library installed

```sh
# Arch Linux / Manjaro
sudo pacman -S hackrf pkgconf

# Debian / Ubuntu / Linux Mint / Pop!_OS
sudo apt install libhackrf-dev pkg-config

# Fedora
sudo dnf install hackrf-devel pkgconf-pkg-config

# openSUSE Tumbleweed / Leap
sudo zypper install libhackrf-devel pkg-config

# Void Linux
sudo xbps-install hackrf-devel pkg-config

# Gentoo
sudo emerge net-wireless/hackrf

# NixOS — add to your configuration.nix or use a dev shell:
nix-shell -p hackrf pkg-config
```

You also need Rust installed. If you don't have it yet:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

---

## Build and run

```sh
git clone https://github.com/mustang6139/sdrtop
cd sdrtop
cargo build --release
./target/release/sdrtop
```

That's it. sdrtop will find your HackRF automatically.

---

## Common startup options

```sh
# Start tuned to a specific frequency (in Hz)
sdrtop --frequency 92800000

# Start with specific gain settings
sdrtop --lna 24 --vga 30

# Use a different color theme
sdrtop --theme nord

# Load a custom config file
sdrtop --config ~/my-config.toml
```

---

## First run

When sdrtop starts, you may see a **device selector** if you have multiple HackRF devices connected. Use `↑` / `↓` (or `j` / `k`) to select your preferred device, then press `Enter`.

Once the app starts, press `Space` to begin receiving. The spectrum and waterfall will come to life. Use `↑` / `↓` to adjust LNA gain if the signal looks too weak or too strong.

Press `?` at any time to see the full key reference on screen.

Press `q` to quit. Your settings are saved automatically.
