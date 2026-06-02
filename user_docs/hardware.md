# Supported Hardware

← [Back](README.md)

---

## What works today

| Device | Status |
|--------|--------|
| HackRF One | Fully supported — spectrum, waterfall, all diagnostics |
| PortaPack H4M (Mayhem) | In development — telemetry panel via USB serial |

sdrtop is built and tested on real hardware. Support is only added after physical testing — no guessing from documentation alone.

---

## Host platforms

| Platform | Status |
|----------|--------|
| x86-64 Linux | Fully supported |
| Raspberry Pi (Pi 2 and newer, 64-bit Raspberry Pi OS Bookworm) | Supported — lower sample rates on older Pis |
| ARM / Android (Termux) | Builds and runs; needs a root-capable USB stack to reach the device |

sdrtop needs **libhackrf 2023.01.1 or newer** (the version in Raspberry Pi OS Bookworm and Ubuntu 24.04). Older distributions need libhackrf built from source.

---

## What's coming

| Device | Status | Notes |
|--------|--------|-------|
| RTL-SDR (R820T, R828D) | Planned | Most common SDR dongle — first on the list |
| Airspy Mini | Planned | Needs hardware to test |
| Airspy HF+ Discovery | Planned | Needs hardware to test |
| LimeSDR / bladeRF / SDRplay / PlutoSDR | Planned | Wide range of devices, needs hardware |

---

## Supporting hardware development

New device support requires physically owning and testing the hardware. Development currently runs on a HackRF One and a PortaPack H4M.

The next target is an **RTL-SDR** — by far the most common SDR dongle, with the most potential impact on who can use sdrtop. After that, Airspy and then the wider SoapySDR ecosystem.

If you'd like to support this, contributions go directly toward hardware purchases:

[![Ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/mustang6139)
