# Keyboard Shortcuts

← [Back](README.md)

---

## General

| Key | What it does |
|-----|-------------|
| `Space` | Start or stop receiving |
| `f` | Type a new center frequency (in MHz) |
| `s` | Type a new sample rate (2–20 MHz) |
| `r` | Reset all settings to defaults |
| `a` | Toggle the RF amplifier on/off |
| `w` | Pause or resume the waterfall |
| `h` | Freeze the spectrum (hold the current frame) |
| `e` | Enter spectrum focus mode |
| `l` | Enter waterfall focus mode |
| `1`–`5` | Switch layout preset |
| `p` | Cycle through presets |
| `Tab` | Show or hide the footer bar |
| `?` | Show the help overlay |
| `q` | Quit and save settings |

---

## Gain

| Key | What it does |
|-----|-------------|
| `↑` / `↓` | LNA gain up or down by 8 dB |
| `[` / `]` | VGA gain up or down by 2 dB |

LNA (Low Noise Amplifier) is the first gain stage — controls how much you amplify before the signal reaches the chip. VGA (Variable Gain Amplifier) is the second stage, fine-tuning the level further in.

A good starting point: LNA 24, VGA 30. If the spectrum is maxed out (everything near 0 dBFS), turn it down. If it's all noise at the bottom, try turning it up.

---

## Spectrum focus mode

Press `e` to enter focus mode on the spectrum panel. The border changes color to show you're in focus mode.

| Key | What it does |
|-----|-------------|
| `←` / `→` | Tune the center frequency by one step |
| `[` / `]` | Change the tuning step size (1 kHz up to 10 MHz) |
| `↑` / `↓` | Zoom the dBFS axis (expand or compress the signal range shown) |
| `j` / `k` | Move the cursor left or right across the spectrum |
| `m` | Place a named marker at the cursor position |
| `Esc` | Exit focus mode |

---

## Waterfall focus mode

Press `l` to enter focus mode on the waterfall panel.

| Key | What it does |
|-----|-------------|
| `↑` / `↓` | Adjust the color scale (show faint or strong signals in more detail) |
| `[` / `]` | Frame averaging — combine multiple frames per row for a longer time window |
| `+` / `-` | Frequency zoom — magnify the centre of the band (`=` also zooms in) |
| `m` | Place or remove a frequency cursor |
| `←` / `→` | Move the cursor frequency when one is placed |
| `j` / `k` | Scroll back and forth through waterfall history |
| `Esc` | Exit focus mode |

---

## Tips

- If you're not sure what a reading means, the `?` overlay shows a quick summary while you use the app.
- Gain settings and frequency are saved when you quit with `q`. You can also edit them directly in the config file.
