# The Lab Presets

← [Back](README.md)

sdrtop's **lab presets** are the bench-engineer views: instead of just a live spectrum, they surface the measurements sdrtop can derive about your receiver's *signal quality* and *hardware health*. They're built for setting up a clean capture and watching for trouble during a long run.

The measurements are split across four focused presets, each on its own number key:

| Key | Preset | Focus |
|-----|--------|-------|
| `5` | **Lab IQ** | IQ diagnostics + amplitude histogram + spectrum |
| `6` | **Lab RF** | RF chain (NF / MDS) + spectrum + hardware vitals |
| `7` | **Lab Timing** | stream-timing diagnostics + hardware vitals |
| `8` | **Lab Signal** | spectrum + signal metrics + waterfall |

This guide explains each measurement below; the heading notes which preset to open for it. Every panel turns its border and title **[STALE]** when RX is not streaming, so you always know whether you're looking at live data or a frozen snapshot.

> The lab panels also have a focus mode for extra actions — see [Keyboard Shortcuts](keys.md#lab-panel-focus-modes). The focus key is the highlighted letter in each panel's title.

---

## RF Chain  ·  *Lab RF (`6`)*

The receiver's capability in the current configuration — what the hardware *can* do, before any signal arrives.

**Top block — what you're tuned to:**

- **Freq** — current centre frequency.
- **λ / λ/4** — the wavelength and quarter-wavelength at that frequency. Handy in the field for cutting an antenna: at 433 MHz, λ/4 ≈ 17.3 cm; at 2.4 GHz, ≈ 3.1 cm.
- **Sample rate** — the configured rate (how wide a slice of spectrum you're capturing).
- **BB filter** — the analog baseband filter bandwidth the HackRF picked for that rate.

**Gain chain:**

```
AMP[14] → LNA[24] → VGA[20] = 58 dB
```

A visual of the three amplifier stages in order, with each stage's gain and the total. The AMP stage only appears when the front-end amplifier is enabled (`a`).

**Est. NF (Friis)** — estimated cascade **Noise Figure** in dB. This is the single number that describes how much noise your receiver adds to the signal. Computed from the HackRF's known stage characteristics using the Friis formula. Lower is better:

- With AMP on at high LNA gain: ~2 dB (excellent)
- AMP off, LNA at max: ~3.5 dB (good)
- Low LNA gain: 6 dB and up (the receiver is adding significant noise)

Green below 4 dB, amber to 8 dB, red above.

**MDS** — **Minimum Detectable Signal** in dBm. The weakest signal your receiver can pull out of the noise in the current configuration:

```
MDS = −174 dBm/Hz + 10·log₁₀(bandwidth) + NF
```

A typical value at 10 MHz bandwidth with a 3.5 dB noise figure is about −100 dBm. Narrowing the BB filter or lowering the noise figure improves (lowers) the MDS. This is the number to watch when you're trying to hear something faint.

**Board / USB API** — board revision and firmware USB API version, dimmed because they're reference info, not something you monitor.

**Gain advisor + ADC utilisation gauge** (bottom) — reads the live amplitude distribution and tells you whether to raise or lower gain, with the fraction of samples landing in the ADC's sweet spot.

---

## IQ Amplitude Distribution  ·  *Lab IQ (`5`)*

A histogram of incoming sample amplitudes across 32 bins, log-scaled vertically so both rare strong peaks and the bulk of weak samples are visible at once. Colour zones:

- **Dim (left)** — low amplitude. The ADC is under-utilised.
- **Green (centre)** — the healthy range.
- **Red (right)** — high amplitude, approaching clipping.

**Numeric breakdown** — the exact percentages so you can set gain precisely:

```
Low  12%   Mid  71%   Clip  17%
```

**PAPR** — **Peak-to-Average Power Ratio** (crest factor) in dB, estimated from the distribution. This is a quick fingerprint of *what kind* of signal you're looking at:

| PAPR | Likely signal |
|------|---------------|
| under 3 dB | CW / FM (constant envelope) |
| 3–8 dB | AM / mixed |
| 8–15 dB | wideband / spread-spectrum |
| over 15 dB | bursty / impulsive |

A status line at the bottom summarises the picture: "Dynamic range OK", "weak signal — ADC under-utilised", or "clipping risk".

---

## IQ Diagnostics  ·  *Lab IQ (`5`)*

The quality of the I/Q signal coming off the ADC. Problems here show up as artefacts in the spectrum.

- **DC I / DC Q** — how far each channel is offset from zero, with a combined **DC magnitude** gauge.
- **DC spike** — how tall the resulting spike at the centre frequency is, in dBFS. A high DC offset puts a fixed tone right in the middle of your spectrum; this tells you how loud it is. Green below −40 dBFS.
- **Amp imbalance** — whether I and Q carry the same power. A mismatch creates mirror images of signals on the opposite side of centre.
- **Phase imbalance** — whether I and Q are exactly 90° apart. Also causes mirroring.
- **IRR** — **Image Rejection Ratio** in dB, computed from the amplitude and phase imbalance. This is the key quadrature-quality figure: it tells you how far *below* every real signal its mirror image appears. 30 dB or more is good (images are faint); below 20 dB and the images become a problem.

A contextual hint at the bottom summarises whether anything needs attention, colour-matched to severity.

---

## Hardware Vitals  ·  *Lab RF (`6`) / Lab Timing (`7`)*

Whether the capture chain is keeping up, with a trend sparkline under each metric.

- **Drops** — samples lost per second, plus the session total. Non-zero means USB or CPU can't keep up.
- **ADC saturation** — how often samples hit the ADC ceiling, with the session peak.
- **CPU / RAM** — sdrtop's own processor and memory use. CPU is a system-wide percentage (100% means every core is maxed), so on a multi-core machine a healthy figure is well under 100%. If CPU climbs toward the warn/crit thresholds at high sample rates, that's often the cause of drops.
- **USB errors** — zero-length USB transfers, usually a cable or hub problem. Coloured by recent rate, not session total, so a single old glitch doesn't pin it red forever.
- **SR** — configured versus actually-measured sample rate, e.g. `20.000 → 19.847 MHz (−0.8%)`. A large gap means USB can't sustain the requested rate. Shows `→ ---` when not streaming.
- **BUF fill** — receive-buffer fill percentage with history. A leading indicator: if this trends upward toward 100%, drops are about to start.

---

## Using the lab presets in practice

A typical setup flow, switching presets as you go:

1. Tune to your target and start RX (`Space`).
2. In **Lab IQ (`5`)**, watch the **IQ Amplitude Distribution**. Adjust LNA/VGA (`↑`/`↓`, `[`/`]`) until Mid is high and Clip stays at 0%; glance at **IQ Diagnostics** — IRR above 30 dB and DC spike below −40 dBFS mean clean quadrature.
3. In **Lab RF (`6`)**, check the **gain advisor**, **Est. NF** and **MDS** — confirm the receiver is sensitive enough for what you're chasing.
4. In **Lab Timing (`7`)**, confirm the timing verdict is Good/Excellent before committing to a long run.
5. During a long capture, keep an eye on **Hardware Vitals** (in the `6`/`7` labs) — CPU, BUF fill, and Drops together tell you whether the run is sustainable.

---

← [Back to all screens](screens.md)
