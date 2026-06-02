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

## Runtime input: frequency and sample rate

While sdrtop is running, you can change settings without restarting:

### Frequency (press `f`)

You'll see a prompt asking for the frequency in **MHz**. Examples:

```
Enter frequency (MHz): 92.8    → 92.8 MHz (FM broadcast)
Enter frequency (MHz): 433.92  → 433.92 MHz (ISM band)
Enter frequency (MHz): 2.4065  → 2.4065 GHz (WiFi)
```

Valid range: **1 MHz to 6 GHz** (HackRF One limits).

### Sample rate (press `s`)

You'll see a prompt asking for the sample rate in **MHz**. Examples:

```
Enter sample rate (MHz): 2     → 2 MHz (narrow capture)
Enter sample rate (MHz): 10    → 10 MHz (balanced)
Enter sample rate (MHz): 20    → 20 MHz (maximum, uses full USB 2.0 bandwidth)
```

Valid range: **2 MHz to 20 MHz**.

The actual achieved rate may be slightly lower than requested, especially on slower systems or with poor USB cables. Check the **Lab preset** (`5`) to see the configured vs. measured rate.

---

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

## Custom layout presets

A *preset* is a named arrangement of panels. sdrtop ships with built-in presets you switch between with the number keys, but you can also define your own in the config file. Your presets are merged with the built-in ones at startup, and they survive a save — sdrtop never erases hand-written presets.

A preset is a list of panels. Each panel has a `name`, a `position`, and optionally a size:

```toml
[presets.my_view]
panels = [
  { name = "header",   position = "top",    height = 5     },
  { name = "spectrum", position = "body"                    },
  { name = "log",      position = "right",  width_pct = 30  },
  { name = "footer",   position = "bottom"                  },
]
```

**Positions:**

| Position | Where it goes | Size field |
|----------|---------------|------------|
| `top`    | Full-width strip at the top    | `height` (rows) |
| `bottom` | Full-width strip at the bottom | `height` (rows) |
| `left`   | Left column of the body        | `width_pct` (% of body) |
| `right`  | Right column of the body       | `width_pct` (% of body) |
| `body`   | Centre column (fills remaining space) | — |

**Available panel names:** `header`, `spectrum`, `waterfall`, `log`, `footer`, `signal_strip`, `rf_chain`, `iq_diagnostics`, `iq_histogram`, `hardware_health`, `signal_metrics`, `system_resources`.

**Reaching your preset:**

- It automatically joins the `p` cycle (presets are cycled in alphabetical order).
- If you name it after a reserved number-key slot, that key switches to it directly:

  | Key | Reserved name | Status |
  |-----|---------------|--------|
  | `6` | `lab_iq`      | Built-in |
  | `7` | `lab_rf`      | Built-in |
  | `8` | `lab_timing`  | Free — define it yourself |
  | `9` | `lab_signal`  | Built-in |
  | `0` | `micro_main`  | Built-in (micro field mode) |

  Keys `6`, `7`, `9`, and `0` already map to built-in presets. The free slot (`8`) does nothing until you define a preset with the matching name — for example, adding `[presets.lab_timing]` makes the `8` key switch to it. (Pressing a number key whose preset isn't defined just logs a note and does nothing.) If you override any existing name — built-in or reserved — your version replaces it.

  > **Micro field mode (`0`):** Pressing `0` enters a compact, single-panel layout designed for small screens and SSH sessions. Pressing `0` again cycles through the micro views (signal, gain, health) as they become available.

---

## CLI flags

If you want to start sdrtop with specific settings without changing the config file, you can pass them on the command line. These override the saved config for that session only.

```sh
sdrtop --frequency 145500000 --lna 16 --vga 20 --theme dracula
```

Run `sdrtop --help` to see all available options.
