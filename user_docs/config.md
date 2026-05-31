# Configuration

← [Back](README.md)

---

## Where the config lives

sdrtop saves your settings automatically when you quit (`q`). The file is at:

```
~/.config/sdrtop/config.toml
```

You can open and edit it by hand — it's plain text. Changes take effect next time you start sdrtop.

---

## What's in the config

```toml
[radio]
frequency_hz = 92800000   # center frequency in Hz
sample_rate  = 2000000.0  # samples per second (2–20 million)
lna_gain     = 24         # LNA gain (0–40 dB, step 8)
vga_gain     = 30         # VGA gain (0–62 dB, step 2)
amp_enabled  = false      # RF amplifier on or off

[display]
active_preset      = "main"   # which layout to use at startup
waterfall_max_rows = 64       # how many rows of history the waterfall keeps

[theme]
base = "nord"   # which color theme to use
```

---

## Frequency markers

You can save named frequency markers. They appear as vertical lines on the spectrum with a label.

```toml
[[display.spectrum_markers]]
freq_hz = 92800000
label   = "FM Radio"

[[display.spectrum_markers]]
freq_hz = 156800000
label   = "Marine ch16"
```

You can add as many as you like. You can also place them from within sdrtop using the `m` key in spectrum focus mode.

---

## CLI flags

If you want to start sdrtop with specific settings without changing the config file, you can pass them on the command line. These override the saved config for that session only.

```sh
sdrtop --frequency 145500000 --lna 16 --vga 20 --theme dracula
```

Run `sdrtop --help` to see all available options.
