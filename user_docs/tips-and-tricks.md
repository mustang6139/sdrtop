# Tips and Tricks

← [Back](README.md)

Practical advice for getting the most out of sdrtop.

---

## Gain tuning

### The 80/20 rule

A good signal occupies the middle 20% of the ADC range — enough headroom above to catch peaks, enough margin below to stay out of the noise.

The **IQ Amplitude Distribution** in the Lab IQ preset (`5`) is your best friend. Adjust LNA and VGA until:

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

## Frequency tuning

### Quick frequency scanning

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

## Lab presets workflow

The lab presets (`5`–`8`) are built for capture setup — each one focuses on a
different aspect, so you switch between them as you dial things in. A typical flow:

### Pre-capture checklist

1. **Tune to your target frequency** with `f`.
2. **Start RX** with `Space`.
3. **Set gain — Lab IQ (`5`):**
   - Watch the **IQ Amplitude Distribution** histogram.
   - Use `↑` / `↓` and `[` / `]` until **Low** < 5%, **Mid** > 60%, **Clip** < 5%.
   - While here, check **IQ Diagnostics**: **IRR** > 20 dB? **DC spike** < −40 dBFS?
4. **Check the front end — Lab RF (`6`):**
   - Is **Est. NF** < 5 dB? (Good noise figure.)
   - Is **MDS** low enough to hear your target?
5. **Check stability — Lab Timing (`7`):**
   - Are **Drops** zero and the **timing verdict** Good/Excellent?
   - Is **CPU** < 80% and **BUF** stable (not climbing)?

If everything checks out, you're ready to capture.

### During a long capture

Keep an eye on the **Hardware Vitals** panel (in the `6`/`7` labs — or press `v`
to focus it):

- **Drops** climbing → USB is struggling; lower sample rate or try a different cable.
- **CPU** trending up → something else on your system is consuming CPU; close other apps.
- **BUF** trending toward 100% → a warning sign that drops are imminent.

---

## Custom presets for your workflow

Define your own presets in the config for different use cases. See [Advanced Features](advanced.md#defining-custom-presets) for the full syntax, but here's a quick example:

```toml
[presets.airband_chase]
panels = [
  { name = "header",       position = "top",    height = 2  },
  { name = "spectrum",     position = "body"                 },
  { name = "signal_strip", position = "bottom", height = 2  },
]
```

Assign it to a key by naming it after a built-in (e.g. `lab_timing` uses key `7`), which overrides that built-in, or access it via `p` (preset cycle).

---

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

For small screens, press `0` to enter **micro mode** — a compact, single-panel view.

### tmux pane

sdrtop fits nicely in a tmux pane. Create a split and run:

```sh
tmux split-window -v -c ~/SDR -p 30 'sdrtop --lna 24'
```

---

← [Back](README.md)
