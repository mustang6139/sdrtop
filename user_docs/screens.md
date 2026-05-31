# What You See on Screen

← [Back](README.md)

sdrtop is divided into panels. Each panel shows a different aspect of what your radio is doing. You can switch between layout presets with the number keys `1`–`6`.

---

## Spectrum

The main view — a live graph of signal strength across the frequency range you're tuned to. The horizontal axis is frequency, the vertical axis is signal strength (dBFS, where 0 is maximum). Stronger signals appear higher up.

- The bright line is the live signal.
- The dimmer line behind it shows the peak levels seen so far (peak hold).
- The dashed line near the bottom is the noise floor — what "silence" looks like for your radio in current conditions.

Band labels (FM, Aviation, Marine, etc.) appear at the top of the graph when relevant frequencies are in view.

---

## Waterfall

A scrolling history of the spectrum. Each new row represents one moment in time, scrolling downward. Colors go from dark (weak signal) to bright (strong signal). This lets you see patterns over time — a signal that appears and disappears, interference that comes and goes.

---

## Signal metrics

Three numbers about the signal you're currently tuned to:

- **SNR** — how much stronger the signal is compared to the noise around it. Higher is cleaner.
- **Channel power** — overall signal strength in the current band, in dBFS.
- **Occupied bandwidth** — how wide the signal actually is (99% of its energy).

---

## Hardware health

Information about whether your HackRF is running smoothly:

- **Sample drops** — samples the USB connection couldn't deliver in time. Some is normal; a lot means the USB is struggling.
- **ADC saturation** — the radio's input is overloaded. Turn the gain down.
- **IQ imbalance** — a hardware characteristic. Small values are normal.
- **DC offset** — a spike at the exact center frequency. Normal for most SDR hardware.

---

## RF chain

Your radio's current settings at a glance: frequency, sample rate, LNA gain, VGA gain, and whether the RF amplifier is on. Also shows the HackRF board revision and baseband filter width.

---

## IQ histogram

A bar chart showing the distribution of incoming signal amplitudes. Ideally it looks like a hill centered in the middle. If it's pushed to the edges, the gain is too high (saturation). If it's a thin spike in the center, the signal is very weak.

---

## Observer mode

If another app (like GNU Radio or SDR++) already has your HackRF open, sdrtop can't control it — but it doesn't crash. Instead it switches to observer mode: it reads what it can from the operating system (device info, which app is using the radio, USB stats) and displays that instead.

When the other app lets go, sdrtop picks the radio back up automatically.

---

## Layouts

Switch between preset layouts with number keys. Each preset rearranges which panels are visible and how large they are.

| Key | Layout |
|-----|--------|
| `1` | Main — spectrum + all diagnostic panels |
| `2` | Spectrum only |
| `3` | Waterfall only |
| `4` | Spectrum + waterfall |
| `5` | Monitoring — metrics + health focused |
| `6` | Lab — everything visible at once |
| `p` | Cycle through presets |
