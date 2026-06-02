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

## Signal strip

A single bar at the bottom of the main view with eight live readings:

- **SNR** — signal-to-noise ratio. Higher is cleaner.
- **PWR** — channel power in dBFS.
- **NF** — estimated noise floor in dBFS.
- **SAT** — ADC saturation percentage. Non-zero means the input is clipping; turn gain down.
- **DROP** — sample drops per second. If this is non-zero, USB can't keep up.
- **BUF** — receive buffer fill percentage. A leading indicator — if this climbs toward 100%, drops are coming.
- **IQ** — IQ amplitude imbalance in dB. Small values (under ±1 dB) are normal.
- **RBW** — resolution bandwidth. Tells you the frequency resolution of the current FFT.

---

## Hardware health

Shows whether your HackRF is running smoothly, with trend sparklines for each metric:

- **Drops** — sample drops per second + session total + trend graph.
- **ADC saturation** — how often samples hit the ADC ceiling + peak + trend.
- **CPU / RAM** — sdrtop's own processor and memory use + trend. CPU is a system-wide percentage (100% = all cores maxed).
- **USB errors** — zero-length USB transfers, usually caused by cable or hub issues + trend.
- **SR** — configured vs. actually-measured sample rate. A large gap means USB can't sustain the requested rate.
- **BUF fill** — receive-buffer fill percentage + trend. A leading indicator — if it climbs toward 100%, drops are coming.

---

## RF chain

Diagnostic view of the signal path. Shows the current frequency and its wavelength, sample rate, baseband filter bandwidth, and a visual gain chain (AMP → LNA → VGA = total dB). Two derived figures stand out:

- **Est. NF** — estimated noise figure (how much noise the receiver adds), via the Friis formula.
- **MDS** — minimum detectable signal in dBm (the weakest signal you can hear in this configuration).

At the bottom:

- **ADC utilisation gauge** — what fraction of incoming samples land in the optimal amplitude range (not too weak, not clipping).
- **Gain advisor** — reads the ADC utilisation and tells you whether to increase or reduce gain, and by how much.

See the [Lab preset guide](lab.md) for what each number means and how to use them.

---

## IQ diagnostics

Measures the quality of the I/Q signal from the ADC:

- **DC offset** — how far the I and Q channels are shifted from zero. A non-zero offset causes the DC spike at the center frequency. Shown separately for I and Q, plus a combined magnitude gauge.
- **DC spike** — how tall that centre-frequency spike is, in dBFS.
- **Amplitude imbalance** — whether I and Q have the same power level. Causes mirror images in the spectrum.
- **Phase imbalance** — whether I and Q are exactly 90° apart. Also causes mirroring.
- **IRR** — image rejection ratio in dB: how far below each real signal its mirror image appears. Higher is better (30 dB+ is clean).

A contextual hint at the bottom summarises whether anything needs attention.

---

## IQ histogram

A bar chart of incoming signal amplitudes across 32 bins. The color zones show:

- **Dim (left)** — low amplitude: signal is weak, ADC is under-utilised.
- **Green (center)** — healthy range: good dynamic range usage.
- **Red (right)** — high amplitude: approaching or hitting clipping.

Below the chart: a **Low / Mid / Clip** percentage breakdown for setting gain precisely, and **PAPR** (peak-to-average power ratio) which fingerprints the signal type — under 3 dB is CW/FM, higher values mean AM, wideband, or bursty signals.

A status line tells you what it means: "Dynamic range OK", "weak signal — ADC under-utilised", or "clipping risk".

---

## Observer mode

If another app (like GNU Radio or SDR++) already has your HackRF open, sdrtop can't control it — but it doesn't crash. Instead it switches to observer mode: it reads what it can from the operating system (device info, which app is using the radio, USB stats) and displays that instead.

When the other app lets go, sdrtop picks the radio back up automatically.

---

## Layouts

Switch between preset layouts with number keys. Each preset rearranges which panels are visible and how large they are.

| Key | Layout |
|-----|--------|
| `1` | Main — spectrum + waterfall + signal strip + log |
| `2` | Spectrum only |
| `3` | Waterfall only |
| `4` | Spectrum + waterfall |
| `5` | Lab — RF chain · IQ histogram · IQ diagnostics · hardware health ([full guide](lab.md)) |
| `p` | Cycle through presets |

The **Lab** preset has its own detailed walkthrough: **[The Lab Preset](lab.md)**.
