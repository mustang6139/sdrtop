# Tips and Tricks

← [Back](README.md)

Practical advice for getting the most out of sdrtop.

---

## Gain tuning

### The 80/20 rule

A good signal occupies the middle 20% of the ADC range — enough headroom above to catch peaks, enough margin below to stay out of the noise.

The **IQ Amplitude Distribution** in the Lab preset is your best friend. Adjust LNA and VGA until:

- **Low**: under 5% (ADC not wasting bits on empty space)
- **Mid**: 60–80% (the healthy zone)
- **Clip**: 0–5% (room for peaks, but not clipping)

If you're clipping regularly, turn down VGA first (finer control), then LNA.

### Gain settings by scenario

| Scenario | LNA | VGA | Notes |
|----------|-----|-----|-------|
| Weak signal (−100+ dBm) | 40 | 60 | Max gain, very sensitive. Watch for noise. |
| Moderate signal (−80 to −100 dBm) | 24 | 20 | A safe default. Good balance. |
| Strong signal (−60 dBm+) | 8–16 | 10 | Lots of headroom to avoid clipping. |
| In an urban area with many strong signals | 0 | 0 | Start here; turn up only as needed. |
| Noisy environment | 40 | 40 | Maximize gain; you need all the sensitivity. |

Use these as **starting points**, not rules. Every antenna, frequency, and environment is different.

---

## Frequency input tricks

### Shorthand notation

When you press `f` to enter a frequency, you can type numbers in a few formats:

- **MHz (most common):** `92.8` → 92.8 MHz
- **Direct Hz (for precision):** You can also think of the input as millions of Hz, so `92.8` is always interpreted as MHz.
- **Decimal places:** `433.920` for 433.920 MHz, `2.4065` for 2.4065 GHz

### Quick scans

To quickly check a few frequencies:

1. Press `f`, type a frequency, press `Enter`.
2. Look at the spectrum for 1–2 seconds.
3. Repeat for the next frequency.

The **frequency markers** help you remember which peaks you've seen. Use `m` in spectrum focus mode (`e`) to mark them.

### Using markers for band planning

Place markers at the band edges you care about:

1. Tune to the edge of a band (e.g., 88 MHz for FM broadcast start).
2. Press `e` to enter spectrum focus mode.
3. Press `m` to place a marker.
4. Name it (e.g., "FM start").

Now when you tune across the band, you'll always see where the edges are.

---

## Spectrum analysis in focus mode

### The hold feature

Press `h` to freeze the current spectrum while live data continues below it. This is useful for:

- **Comparing signals.** Freeze a strong signal, then tune to another frequency to compare peak heights.
- **Measuring peak-to-peak.** Freeze the current spectrum, watch the live signal rise and fall relative to it, and estimate the dynamic range.
- **Spotting weak signals.** Hold a moment, then look for where the new signal is lower.

Press `h` again to disable the hold.

### Using the zoom feature

Press `↑` in spectrum focus mode to expand the dBFS axis (zoom in on weak signals). Press `↓` to compress it (see a wider dynamic range).

**Use case:** You're looking for a very weak signal, but the spectrum is dominated by one strong transmitter. Zoom in on the weak part to see fine detail.

### Cursor and markers

1. Press `e` to enter spectrum focus mode.
2. Use `j` / `k` to move a cursor across the spectrum.
3. The bottom of the panel shows the exact frequency and signal level at the cursor.
4. Press `m` to place a named marker at that spot.

**Use case:** You see a spike at an unknown frequency. Use the cursor to hover over it, read the frequency, then mark it if it's interesting.

### Channel bandwidth cycling

Once you've placed a marker, you can assign a **channel bandwidth** to it. This is useful if you're tracking a known signal:

1. Place a marker with `m` (in spectrum focus mode).
2. Name it (e.g., "FM station").
3. Move the cursor near the marker (within a few spectrum steps).
4. Press `B` (capital B) to cycle through channel bandwidth options.

Available bandwidths: 6.25 kHz, 12.5 kHz, 25 kHz, 50 kHz, 100 kHz, 200 kHz, 500 kHz. Each press advances to the next one. When you reach the end, it cycles back to "no bandwidth assigned."

**Why?** This is a placeholder for future features (bandwidth measurement, channel power readouts). For now, it's a way to tag a signal with its expected bandwidth.

---

## Waterfall analysis

### Reading the waterfall history

The waterfall scrolls downward, with each row representing a moment in time. The oldest rows are at the top. To look at older data:

1. Press `l` to enter waterfall focus mode.
2. Use `j` / `k` to scroll backward (upward) and forward (downward) through history.

This is great for:

- **Spotting intermittent signals.** Scroll back and see when they appeared and disappeared.
- **Measuring signal duration.** Count the rows between when a signal started and ended.
- **Checking for interference patterns.** Some interferers are periodic. Scrolling the history often reveals the pattern.

### Slow-motion waterfall

Press `[` to slow down the waterfall by averaging multiple frames into each row. Press `]` to speed it back up.

**Use case:** A signal appears and disappears very quickly, and you're missing it. Slow the waterfall down to see it more clearly.

### Zooming the waterfall

Press `+` (or `=`) in waterfall focus mode to zoom in on the center frequency (narrow the displayed bandwidth). Press `-` to zoom out.

This is the **inverse** of the spectrum zoom: the spectrum zoom changes the dBFS range, the waterfall zoom changes the frequency span shown.

---

## Lab preset workflow

The Lab preset (`5`) is designed for capture setup. Here's a typical flow:

### Pre-capture checklist

1. **Tune to your target frequency** with `f`.
2. **Switch to the Lab preset** with `5`.
3. **Start RX** with `Space`.
4. **Adjust gain:**
   - Watch the **IQ Amplitude Distribution** histogram.
   - Use `↑` / `↓` and `[` / `]` until **Low** < 5%, **Mid** > 60%, **Clip** < 5%.
5. **Check RF Chain:**
   - Is **Est. NF** < 5 dB? (Good noise figure.)
   - Is **MDS** low enough to hear your target?
6. **Check IQ Diagnostics:**
   - Is **IRR** > 20 dB? (20+ dB is acceptable; 30+ is clean.)
   - Is **DC spike** < −40 dBFS? (Minimal center tone.)
7. **Check Hardware Health:**
   - Are **Drops** zero?
   - Is **CPU** < 80%?
   - Is **BUF** stable (not climbing)?

If everything checks out, you're ready to capture.

### During a long capture

Keep an eye on the **Hardware Health** panel:

- **Drops** climbing → USB is struggling; lower sample rate or try a different cable.
- **CPU** trending up → something else on your system is consuming CPU; close other apps.
- **BUF** trending toward 100% → a warning sign that drops are imminent.

---

## Configuration and presets

### Custom presets for different scenarios

Define presets for different use cases in your config:

```toml
[presets.airband_chase]
panels = [
  { name = "header",       position = "top",    height = 2  },
  { name = "spectrum",     position = "body"                 },
  { name = "signal_strip", position = "bottom", height = 2  },
  { name = "footer",       position = "bottom", height = 1  },
]

[presets.lab_detailed]
panels = [
  { name = "header",           position = "top",    height = 2  },
  { name = "rf_chain",         position = "left",   width_pct = 25 },
  { name = "iq_histogram",     position = "body"                 },
  { name = "iq_diagnostics",   position = "right",  width_pct = 25 },
  { name = "hardware_health",  position = "bottom", height = 3  },
  { name = "footer",           position = "bottom", height = 1  },
]
```

Then assign one to a number key (`8` is free):

```toml
[presets.lab_timing]
# your custom preset definition
```

Press `8` to switch to it on the fly.

### Persistent frequency markers

Add markers to your config so they're always loaded:

```toml
[[display.spectrum_markers]]
freq_hz = 88000000
label = "FM start"

[[display.spectrum_markers]]
freq_hz = 108000000
label = "FM end"

[[display.spectrum_markers]]
freq_hz = 2400000000
label = "ISM 2.4 GHz"
```

---

## SSH and remote monitoring

### Using sdrtop over SSH

If you have a HackRF on a Raspberry Pi or embedded system, you can run sdrtop over SSH:

```sh
ssh pi@raspberrypi.local sdrtop --theme nord --frequency 433920000
```

You might want to use a smaller screen layout. Press `0` for **micro mode** (a compact, single-panel view that adapts to window width).

### tmux pane

sdrtop fits nicely in a tmux pane. Create a split and run:

```sh
tmux split-window -v -c ~/SDR -p 30 'sdrtop --lna 24'
```

Now you have sdrtop running in a 30% pane, with other commands above. Quit with `q` — the pane closes cleanly.

---

## Noise figure and minimum detectable signal

### Understanding Est. NF (Noise Figure)

The **estimated noise figure** in the Lab preset tells you how much noise your receiver adds. Lower is better.

- **2 dB** (with AMP on) — excellent; this is about as good as HackRF gets.
- **3–4 dB** (AMP off, high LNA) — very good.
- **6+ dB** (low LNA gain) — acceptable, but the receiver is adding noticeable noise.

The calculation uses the Friis formula and known HackRF characteristics. It's an estimate, not a lab measurement, but it's reliable enough for field tuning.

### Understanding MDS (Minimum Detectable Signal)

The **MDS** in dBm tells you the weakest signal you can pull out of the noise:

```
MDS = −174 dBm/Hz + 10·log₁₀(bandwidth) + NF
```

A typical HackRF at 10 MHz bandwidth with a 3.5 dB noise figure:

```
MDS = −174 + 40 + 3.5 = −130.5 dBm
```

**In practice:** If your target signal is stronger than the MDS, you can hear it. If it's much weaker, you probably can't.

**To improve MDS:**
- Lower the noise figure (use AMP, increase LNA).
- Narrow the baseband filter (lower sample rate → narrower filter, but less bandwidth).
- Both work, but they trade off flexibility.

---

## IQ quality diagnosis

### IRR (Image Rejection Ratio) is the key quadrature metric

A signal appears on both sides of center — the real signal on one side, a mirror image on the other. **IRR** tells you how far below the real signal the image appears.

- **30+ dB** — clean; the image is faint.
- **20–30 dB** — acceptable for most work.
- **< 20 dB** — poor; mirror images become visible in the spectrum.

IRR depends on **amplitude and phase imbalance** between I and Q. Some is hardware-dependent, but sample rate can affect it. Experiment.

### PAPR fingerprints signal type

The **PAPR** (peak-to-average power ratio) in the IQ Amplitude Distribution is a quick way to identify what kind of signal you're looking at:

| PAPR | Signal type |
|------|-------------|
| < 3 dB | CW, FM, constant-envelope |
| 3–8 dB | AM, mixed modulation |
| 8–15 dB | Wideband, spread-spectrum |
| > 15 dB | Bursty, impulsive, radar |

This is useful for blind signal identification.

---

## Antenna tips

### Quarter-wavelength antennas

The **RF Chain** panel shows **λ/4** (quarter-wavelength) at your tuned frequency. Cut a wire or monopole to this length for a resonant antenna with no tuning.

Examples:
- **433 MHz:** λ/4 ≈ 17.3 cm (good for 433 MHz LoRaWAN / ISM)
- **2.4 GHz:** λ/4 ≈ 3.1 cm (tiny!)
- **144 MHz (2m band):** λ/4 ≈ 52 cm (longer, but very effective)

### Practical antenna tuning

If you're not getting a strong signal:

1. **Try a different antenna orientation.** Some patterns are directional.
2. **Move the antenna around.** Even 30 cm can make a difference.
3. **Use **λ/4** as a starting point, then adjust length slightly for better signal.
4. **Watch the spectrum.** If the noise floor rises when you add an antenna, it's working (catching more signal and noise together is a good sign).

---

## Recommended default config for first-time use

Save this to `~/.config/sdrtop/config.toml`:

```toml
[radio]
frequency_hz = 92800000   # FM broadcast band
sample_rate = 5000000.0   # 5 MHz is a good default
lna_gain = 24             # Safe, mid-range gain
vga_gain = 20             # Complements LNA
amp_enabled = false       # Off by default; turn on if signal is too weak

[display]
active_preset = "main"    # Spectrum + waterfall + signal strip
waterfall_max_rows = 64

[theme]
base = "nord"             # Easy on the eyes

[[display.spectrum_markers]]
freq_hz = 88000000
label = "FM start"

[[display.spectrum_markers]]
freq_hz = 108000000
label = "FM end"
```

This gives you a good starting point for learning the app without overwhelming configuration.

← [Back](README.md)
