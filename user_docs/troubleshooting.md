# Troubleshooting

← [Back](README.md)

---

## General startup issues

### "Device not found" or permission error

**Problem:** sdrtop can't detect your HackRF One, or you see "Permission denied."

**Solution:**

1. **Check the cable.** Make sure the USB cable is fully connected to the HackRF and your host.
2. **Check device visibility:**
   ```sh
   lsusb | grep HackRF
   ```
   You should see a line with `HackRF One` or similar. If nothing appears, the device isn't detected by the kernel.

3. **Check udev rules.** If you see the device in `lsusb` but get a permission error, you likely need udev rules. On most Linux distributions, installing the `hackrf` package handles this. If not, you can add a rule manually:
   ```sh
   # Create or append to /etc/udev/rules.d/25-hackrf.rules
   SUBSYSTEM=="usb", ATTRS{idVendor}=="04b4", ATTRS{idProduct}=="b200", MODE="0664", GROUP="plugdev"
   SUBSYSTEM=="usb", ATTRS{idVendor}=="04b4", ATTRS{idProduct}=="b201", MODE="0664", GROUP="plugdev"
   ```
   Then reload udev and reconnect the device:
   ```sh
   sudo udevadm control --reload-rules
   sudo udevadm trigger
   ```

4. **Check your user is in the right group:**
   ```sh
   groups $USER | grep -c plugdev || echo "Not in plugdev group"
   ```
   If not, add yourself:
   ```sh
   sudo usermod -aG plugdev $USER
   ```
   Then log out and back in.

### Device selector popup appears every time

**Problem:** You have multiple HackRF devices connected, and sdrtop asks you to choose one every startup.

**Solution:** You can start sdrtop with a specific device by selecting it in the popup, or you can hardcode the device by its serial number in your config file (this feature is planned but not yet implemented). For now, just select your preferred device from the list.

---

## Runtime issues

### Spectrum / waterfall looks wrong or frozen

**Problem:** The spectrum is stuck, or the waterfall isn't scrolling.

**Solution:**

1. **Press `Space` to start RX.** If RX isn't streaming, the display will be frozen. You should see data moving almost immediately.
2. **Check gain settings.** If the spectrum is completely flat (all noise or all empty), your gain is probably too high or too low. Use `↑` / `↓` to adjust LNA gain. A good starting point is LNA 24, VGA 30.
3. **Check the Lab preset.** Press `5` to see the **Lab** view, which shows **Hardware Health** metrics. Look for:
   - **Drops** — if non-zero, USB can't keep up; try lowering sample rate with `s`.
   - **ADC saturation** — if high, turn down gain.
   - **CPU** — if near 100%, the host is maxed out.

### Many samples dropping ("Drops" counter is high)

**Problem:** The **Drops** counter in the Hardware Health panel is climbing, or you see non-zero in the signal strip.

**Solution:**

1. **Lower the sample rate.** Press `s` and type a lower rate (start with 5 MHz if you're at 20 MHz).
2. **Check USB stability.** Use a different USB cable, port, or hub. Long cables or cheap hubs often cause drops.
3. **Check the host CPU.** Press `5` for the Lab view and watch the CPU metric. If it's consistently above 80% on a modern multi-core machine, something else on your system is consuming CPU.
4. **Try a different host.** If you have access to another Linux machine (Pi, another desktop, etc.), test the same setup there. USB or driver issues on your main host will be obvious.

### USB errors or "zero-length transfers"

**Problem:** The **USB errors** metric in the Hardware Health panel shows a non-zero count.

**Solution:**

1. **Inspect your USB cable.** Replace it with a known-good, short cable (under 1 meter).
2. **Try a different USB port.** USB 3.0 ports (usually blue) are sometimes more stable than USB 2.0 (black).
3. **Avoid USB hubs if possible.** Connect the HackRF directly to the host computer.
4. **Check the HackRF firmware.** Update to the latest firmware. This is rare, but occasionally a firmware issue can cause USB glitches.

### Gain won't change (arrows don't work)

**Problem:** You press `↑` or `↓` but the LNA gain doesn't change, or `[` / `]` doesn't change VGA.

**Solution:**

1. **Make sure RX is on.** Press `Space` to start streaming.
2. **Check gain bounds.** LNA is 0–40 dB (step 8), VGA is 0–62 dB (step 2). If you're at the limit, you can't go higher or lower. The display will show your current gain in the RF Chain panel.

### Frequency won't change (pressing `f` does nothing)

**Problem:** Press `f`, type a frequency, but it doesn't tune.

**Solution:**

1. **Check input mode.** After pressing `f`, you should see a prompt at the bottom of the screen asking for a frequency in MHz. Type the frequency and press `Enter`.
2. **Valid range?** HackRF One supports 1 MHz to 6 GHz. If you entered a frequency outside that range, the radio will reject it.
3. **Example:** To tune to 92.8 FM, type `92.8` (not `92800000`). For 433.92 MHz, type `433.92`.

### IQ diagnostics show high DC offset

**Problem:** The **DC I** or **DC Q** metric in the Lab preset is non-zero, or **DC spike** is high (above −40 dBFS).

**Solution:**

1. **This is often hardware-dependent.** Every HackRF has some DC offset due to component tolerances. A DC spike above −40 dBFS is generally acceptable.
2. **Try a different sample rate.** Some rates exhibit lower DC offset than others. Press `s` and experiment.
3. **Check your antenna connection.** Make sure the antenna connector isn't loose or damaged.

### IRR (Image Rejection Ratio) is low

**Problem:** The **IRR** in the Lab preset is below 20 dB.

**Solution:**

1. **Low IRR means quadrature (I/Q) imbalance is high.** This is usually hardware-dependent and not something you can fix in software.
2. **Check sample rate.** Some rates produce better IRR than others. Experiment with different rates.
3. **This is not critical for most use cases.** If you're just observing a strong signal, high IRR doesn't matter. If you need clean spectrum analysis, aim for 30+ dB.

---

## Observer mode

### Another app has my HackRF locked

**Problem:** You see "Observer mode" in the display, and it says another app is using your radio.

**Solution:**

1. **Identify the process.** Observer mode shows the process name that's holding the radio. You can also check with `lsof` or `fuser`:
   ```sh
   sudo fuser -n usb /dev/bus/usb/*/HackRF*
   ```
2. **Close the competing app.** gnuradio, SDR++, or other SDR software needs to release the device. Quit them completely.
3. **sdrtop will pick up the radio automatically** once the other app releases it. You'll see the display transition back to normal control mode. No restart needed.

---

## Configuration and saving

### Settings aren't saved

**Problem:** You quit with `q`, but when you restart, the frequency or gains have reset to defaults.

**Solution:**

1. **Check config file location.** sdrtop saves to `~/.config/sdrtop/config.toml`. Make sure this directory and file exist:
   ```sh
   ls -la ~/.config/sdrtop/config.toml
   ```
   If it doesn't exist, sdrtop will create it on quit.

2. **Check permissions.** Make sure the directory is writable:
   ```sh
   ls -la ~/.config/sdrtop/
   ```
   If you see `dr--r--r--` or similar read-only permissions, change them:
   ```sh
   chmod 755 ~/.config/sdrtop/
   ```

3. **Check disk space.** If your home partition is full, the config save will fail silently. Check:
   ```sh
   df -h ~/
   ```

### Frequency markers not persisting

**Problem:** You place a marker with `m` in spectrum focus mode, but it disappears next time you start sdrtop.

**Solution:**

Markers are saved automatically in the config file. If they're not appearing:

1. **Check the config file manually:**
   ```sh
   cat ~/.config/sdrtop/config.toml | grep -A 5 "spectrum_markers"
   ```
   Your markers should be listed there.

2. **Make sure you quit with `q` (not Ctrl+C).** Only a clean quit saves the config.

3. **Edit the config by hand to add markers:**
   ```toml
   [[display.spectrum_markers]]
   freq_hz = 92800000
   label = "My Station"
   ```

---

## Performance and CPU

### sdrtop is slow or choppy at high sample rates

**Problem:** CPU usage is high, or the display is stuttering at 20 MHz sample rate.

**Solution:**

1. **Reduce frame rate or complexity.** sdrtop runs at ~30 fps. On older or slower machines, this might be too much. There's no config option to reduce it yet, but it's planned.
2. **Close other apps.** Terminal multiplexers, file managers, or background tasks can steal CPU. Close them and try again.
3. **Try a lower sample rate.** If you don't need 20 MHz, use 10 MHz or 5 MHz. This halves the FFT size and CPU load.
4. **Use a faster host.** Raspberry Pi 2 might struggle; Pi 4 or Pi 5 handles high rates well. A modern desktop/laptop should handle 20 MHz easily.

### Memory usage climbs over time

**Problem:** After running sdrtop for hours, it starts using more and more memory.

**Solution:**

This is rare, but if it happens:

1. **Check if there's a log growing.** The in-app log (visible in most presets) accumulates messages. If it's getting huge, that's unusual. Note the message count and report it.
2. **Restart sdrtop.** Memory usage should reset.
3. **Report the issue.** If you can reproduce this, save the config and report the steps.

---

## Getting help

If you've tried these steps and something still isn't working:

1. **Check the [developer changelog](../dev_docs/CHANGELOG.md)** for recent fixes or known issues.
2. **Run with verbose output (if supported).** Future versions will have a debug mode; for now, the app logs go to the in-app log.
3. **Collect info:** Sample rate, HackRF firmware version (`firmware` shown at bottom of screen), host OS, CPU type, and steps to reproduce.
4. **Report on GitHub** with as much detail as possible.

← [Back](README.md)
