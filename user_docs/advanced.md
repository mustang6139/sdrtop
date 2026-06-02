# Advanced Features

← [Back](README.md)

Deep dives into less-obvious features and workflows.

---

## Multiple HackRF devices

### Device selection on startup

If you have multiple HackRF devices connected, sdrtop shows a menu on startup:

```
Select HackRF device:
  ▸ HackRF One (Serial: 000000000000953c64dc2a1d89c3)
    HackRF One (Serial: 00000000000055cc97c3da0a8bf1)

[J]up [K]down [Enter]confirm [Q]uit
```

Use `j` / `k` to select, then press `Enter`. You can also use `↑` / `↓`.

**Note:** There's currently no way to permanently prefer one device over another, so if you always use the same device, you'll need to select it each time (or use a script to automate the selection).

### Multiple devices over SSH

If you're managing multiple HackRFs on different hosts, you can run separate sdrtop instances:

```sh
ssh pi1@pi1.local 'sdrtop --frequency 433920000' &
ssh pi2@pi2.local 'sdrtop --frequency 156800000' &
```

Each runs independently, monitoring its own HackRF.

---

## Focus mode binding reference

Each panel in focus mode has its own set of keybindings. This is a complete reference.

### Spectrum focus mode (press `e`)

| Key | Action |
|-----|--------|
| `←` / `→` | Tune center frequency by one step |
| `[` / `]` | Change tuning step size (1 kHz – 10 MHz) |
| `↑` / `↓` | Zoom dBFS axis (expand/compress signal range) |
| `j` / `k` | Move cursor left/right |
| `m` | Place named marker at cursor |
| `b` | Cycle channel BW on nearest marker |
| `h` | Hold/unhold spectrum frame |
| `Esc` | Exit focus mode |

**Channel BW values:** 6.25 kHz, 12.5 kHz, 25 kHz, 50 kHz, 100 kHz, 200 kHz, 500 kHz, none.

### Waterfall focus mode (press `l`)

| Key | Action |
|-----|--------|
| `↑` / `↓` | Adjust color scale (show faint or strong signals more clearly) |
| `[` / `]` | Frame averaging (combine multiple frames per row for longer time window) |
| `+` / `=` | Frequency zoom in (magnify center) |
| `-` | Frequency zoom out |
| `m` | Place or remove frequency cursor |
| `←` / `→` | Move cursor frequency (when placed) |
| `j` / `k` | Scroll back/forward through waterfall history |
| `Esc` | Exit focus mode |

---

## Custom input modes: frequency and sample rate

### Frequency input (press `f`)

After pressing `f`, you'll see a prompt:

```
Enter frequency (MHz): _
```

Type a frequency in MHz and press `Enter`. Examples:

- `92.8` → 92.8 MHz (FM station)
- `433.92` → 433.92 MHz (ISM band)
- `2.4065` → 2.4065 GHz (WiFi channel 1)
- `10` → 10 MHz (decimal is allowed)

**Valid range:** 1 MHz to 6 GHz.

If you enter an invalid frequency, you'll see an error message briefly, and the frequency won't change. Try again.

### Sample rate input (press `s`)

After pressing `s`, you'll see:

```
Enter sample rate (MHz): _
```

Type a sample rate in MHz and press `Enter`. Examples:

- `2` → 2 MHz (narrow bandwidth, low data rate)
- `5` → 5 MHz (balanced)
- `10` → 10 MHz (wider spectrum, higher CPU)
- `20` → 20 MHz (maximum for HackRF One over USB 2.0)

**Valid range:** 2 MHz to 20 MHz.

The actual sample rate **may be slightly lower** than requested due to USB and clock constraints. The Lab preset shows **SR** (configured vs. measured), so you can verify the actual rate achieved.

---

## Marker system and naming

### Placing markers

1. Enter spectrum focus mode with `e`.
2. Use `j` / `k` to position the cursor.
3. Press `m` to place a marker.
4. Type a name (e.g., "FM 92.8") and press `Enter`.

The marker appears as a vertical line on the spectrum with a label.

### Auto-labeled markers

If you press `m` without typing a name (just press `Enter`), the marker gets a numeric label like `[1]`, `[2]`, etc.

### Viewing and editing markers

Markers are stored in your config file:

```toml
[[display.spectrum_markers]]
freq_hz = 92800000
label = "FM 92.8"

[[display.spectrum_markers]]
freq_hz = 433920000
label = "LoRa"
```

Edit the config directly to change labels or add markers before starting sdrtop. They'll be loaded automatically.

### Removing markers

To remove a marker, hover the cursor over it (in spectrum focus mode) and press `m` again. The marker disappears.

Alternatively, edit the config file and remove the `[[display.spectrum_markers]]` block, then restart.

---

## Observer mode: when another app owns the radio

### What is observer mode?

If you start sdrtop while another app (GNU Radio, SDR++, etc.) is using the HackRF, sdrtop can't take control. Instead, it enters **observer mode**.

In observer mode, sdrtop can still read some information from the system:

- Device serial number and board revision
- Which process is using the radio
- USB statistics (errors, data transferred)
- CPU/RAM usage

But it can't:

- Tune the radio
- Change gains
- Stream data for spectrum/waterfall

The display shows **[Observer Mode]** at the top to remind you.

### Recovering control

When the other app releases the HackRF (usually by quitting), sdrtop automatically picks it back up. You don't need to restart sdrtop; just keep it running and wait for the other app to close.

Once you have control back, the display switches to normal mode, and you can start streaming.

### Identifying the process

Observer mode shows the process name that's holding the radio. You can also find it with `lsof`:

```sh
sudo lsof -i -P -n | grep usb
```

Or use `fuser`:

```sh
sudo fuser -n usb /dev/bus/usb/*/*HackRF*
```

If you know the process name, you can kill it:

```sh
pkill -f gnuradio
pkill -f sdrpp
```

Then sdrtop will regain control.

---

## Preset system and layout configuration

### Built-in presets

| Key | Preset name | Description |
|-----|-------------|-------------|
| `1` | `main` | Spectrum + waterfall + signal strip + log |
| `2` | `spectrum_only` | Spectrum only |
| `3` | `waterfall_only` | Waterfall only |
| `4` | `spectrum_waterfall` | Spectrum + waterfall (no signal strip) |
| `5` | `lab` | RF Chain + IQ histogram + IQ diagnostics + hardware health |
| `6` | `lab_iq` | IQ-focused lab view (histogram + diagnostics) |
| `7` | `lab_rf` | RF-focused lab view (RF chain + hardware health) |
| `9` | `lab_signal` | Signal metrics (IQ diagnostics + hardware health) |
| `0` | `micro_main` | Compact field-mode view (adapts to small screens) |

### Micro mode

Press `0` to enter **micro field mode**. This is a single-panel, compact layout designed for small terminals and SSH sessions.

Press `0` again to cycle through available micro views:

- **Signal view** — simplified spectrum or IQ metrics
- **Gain view** — LNA/VGA, gains, sample rate
- **Health view** — drops, USB errors, CPU, buffer fill

The layout adapts to your terminal width, keeping everything readable even in narrow panes.

### Cycling presets

Press `p` to cycle forward through all available presets (built-in + custom). Presets cycle in alphabetical order.

### Defining custom presets

In your config file, add a `[presets.name]` section:

```toml
[presets.my_spectrum_lab]
panels = [
  { name = "header",    position = "top",    height = 2  },
  { name = "spectrum",  position = "body"                 },
  { name = "log",       position = "right",  width_pct = 20 },
  { name = "footer",    position = "bottom", height = 1  },
]
```

**Panel names (available for layout):**

- `header` — title and device info
- `spectrum` — frequency domain view
- `waterfall` — time-frequency spectrogram
- `signal_strip` — SNR, PWR, NF, SAT, DROP, BUF, IQ, RBW (horizontal bar)
- `signal_metrics` — more detailed signal metrics (subset of lab)
- `rf_chain` — RF path diagnostics (noise figure, MDS, gain chain, etc.)
- `iq_diagnostics` — DC offset, phase/amplitude imbalance, IRR
- `iq_histogram` — amplitude distribution with PAPR and clipping warning
- `hardware_health` — drops, ADC saturation, CPU, RAM, USB errors, buffer fill
- `log` — scrollable message log
- `footer` — help and status

**Positions:**

- `top` — full-width strip at the top; needs `height` (rows)
- `bottom` — full-width strip at the bottom; needs `height` (rows)
- `left` — left column of the body; optional `width_pct` (default 30%)
- `right` — right column of the body; optional `width_pct` (default 30%)
- `body` — center, fills remaining space; no size parameter

**Constraints:**

- Only one panel per position.
- `body` is optional; if omitted, `left` and `right` split the available space.
- `header` and `footer` cannot both be absent (the UI needs at least one).

### Overriding built-in presets

If you define a custom preset with the same name as a built-in, yours replaces it. For example:

```toml
[presets.main]
panels = [
  { name = "spectrum",      position = "body"                 },
  { name = "hardware_health", position = "bottom", height = 4 },
]
```

Now pressing `1` uses your custom layout instead of the default.

---

## Baseband filter and bandwidth

### What is the baseband filter?

The HackRF's analog front end includes a tunable baseband filter. Its bandwidth is automatically chosen based on your sample rate, but it limits the **usable** spectrum you can capture.

**Example:**

- At 10 MHz sample rate, the filter is ~10 MHz wide.
- At 2 MHz sample rate, the filter is ~2 MHz wide.
- At 20 MHz sample rate (USB 2.0 limit), the filter is ~20 MHz wide.

The **BB filter** field in the RF Chain panel shows the actual filter bandwidth in use.

### Practical implications

- **Narrower filter** → less noise (cleaner spectrum) but smaller bandwidth captured.
- **Wider filter** → more bandwidth but more noise.

There's no direct control; it's automatic based on sample rate. If you want a narrower filter, lower the sample rate with `s`.

---

## Frequency tuning steps

In spectrum focus mode, you can change the **tuning step** size with `[` / `]`. This controls how far the frequency moves per arrow press.

**Available steps:** 1 kHz, 5 kHz, 10 kHz, 25 kHz, 100 kHz, 500 kHz, 1 MHz, 5 MHz, 10 MHz.

The step size is shown at the top of the spectrum panel (e.g., `Step: 1.0 MHz`).

**Use case:** At 1 MHz steps, you can quickly scan across a wide band. At 1 kHz steps, you can fine-tune to an exact frequency.

---

## Color depth: 16, 256, or true color

sdrtop detects your terminal's color depth automatically:

- **16-color** — basic terminal colors (older terminals, some SSH sessions)
- **256-color** — extended palette (most modern terminals)
- **True color** — 24-bit RGB (modern Linux terminals, iTerm, etc.)

The spectrum and waterfall colors adapt accordingly. True color looks best, but 16-color still works.

No configuration needed; sdrtop handles it automatically.

---

## Config file location and backup

### Default location

`~/.config/sdrtop/config.toml`

### Using a custom config file

Start sdrtop with a different config path:

```sh
sdrtop --config ~/my-custom-config.toml
```

This is useful for having multiple profiles (e.g., one for weak-signal hunting, another for interference measurement).

### Backing up and sharing configs

Your config is a plain text TOML file. You can:

1. **Backup:** Copy to a safe location.
   ```sh
   cp ~/.config/sdrtop/config.toml ~/backups/sdrtop-config-$(date +%Y%m%d).toml
   ```

2. **Share:** Email or version-control the file (no secrets inside).

3. **Restore:** Copy back and restart sdrtop.
   ```sh
   cp ~/backups/sdrtop-config-20260602.toml ~/.config/sdrtop/config.toml
   ```

---

## Known limitations

- **One HackRF at a time.** sdrtop can only use one HackRF per instance. For multiple devices, run multiple instances on different hosts or use different serial selection logic (planned feature).
- **Sample rate limits.** USB 2.0 limits the HackRF to ~20 MHz sample rate reliably. Older USB hubs or cables reduce this further.
- **No recording to file.** sdrtop is a terminal UI; it doesn't save raw IQ data. Use `hackrf_transfer` or another tool for that.
- **Limited PortaPack support.** PortaPack Mayhem telemetry is in development; full control like the HackRF is not yet available.

---

← [Back](README.md)
