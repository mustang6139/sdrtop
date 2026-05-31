# What's New

← [Back](README.md)

A plain-language summary of recent changes. Full technical details are in the [developer changelog](../dev_docs/CHANGELOG.md).

---

## May 2026

### Waterfall focus mode
You can now press `l` (the letter in "Waterfall") to enter focus mode on the waterfall panel. While focused:
- `↑` / `↓` adjusts the color scale so faint or strong signals show more detail
- `j` / `k` scrolls back through waterfall history
- `[` / `]` slows the waterfall down by averaging multiple frames into one row — useful for seeing a longer time window
- You can place a frequency cursor and see exactly what signal level was at that point and when

### Spectrum analysis tools
Several new tools in spectrum focus mode (`e`):
- **Band plan overlay** — frequency band labels (FM, Aviation, Marine, ISM, GPS, etc.) appear on the spectrum when those bands are in view
- **Zoom** — `↑` / `↓` in focus mode adjusts the dBFS range so you can zoom in on weak signals
- **Hold** — press `h` to freeze the current spectrum as a ghost behind the live signal, useful for comparing
- **Cursor** — `j` / `k` move a crosshair across the spectrum; frequency and signal level at that point are shown
- **Markers** — press `m` to place a named vertical marker at the cursor; markers persist between sessions

### Frequency navigation in spectrum focus
While in spectrum focus mode, `←` / `→` now tune the center frequency. The step size is shown on screen and can be changed with `[` / `]` (1 kHz up to 10 MHz).

### Observer mode
If another app has the HackRF open, sdrtop now switches to observer mode instead of crashing. It shows what it can read without holding the radio (device info, which app is using it, USB stats). When the other app releases the radio, sdrtop picks it back up automatically.

### Sample rate control
Press `s` to type in a new sample rate (2–20 MHz) while the app is running.

### Performance improvements
The app is significantly smoother at high sample rates. CPU and memory usage at 30 fps are substantially lower than before.

### Six themes, six layouts
The full theme system and six layout presets are all live. Switch themes with `--theme <name>` at startup; switch layouts with number keys `1`–`6` while running.
