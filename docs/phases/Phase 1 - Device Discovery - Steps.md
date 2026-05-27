# Phase 1 — Device Discovery & Basic Info: Steps

← [Home](../Home.md) | [Roadmap](../Roadmap.md) | [Phase 1 - Device Discovery - Log](Phase%201%20-%20Device%20Discovery%20-%20Log.md)

**Goal:** Open a HackRF device, read its identity (board name, firmware version,
serial number), and print that information. No TUI yet — just a working FFI layer
and a clean Rust wrapper around `libhackrf`.

---

## Constraints

- Do **not** use the `hackrf` crate from crates.io (version 0.0.1 is broken and unmaintained).
- Link against the system `libhackrf` via `pkg-config`.
- All unsafe FFI code must be isolated behind a safe Rust API.

---

## Step 1 — Build system (`build.rs` + `Cargo.toml`)

- [ ] Add `pkg-config = "0.3"` to `[build-dependencies]`
- [ ] Add `libc = "0.2"` and `anyhow = "1.0"` to `[dependencies]`
- [ ] Write `build.rs`:
  ```rust
  fn main() {
      println!("cargo:rustc-link-lib=hackrf");
      // or use pkg_config::probe_library("libhackrf", true)
  }
  ```
- [ ] `cargo build` — must link successfully

---

## Step 2 — FFI type definitions

- [ ] Define `#[repr(C)]` structs matching `hackrf.h` exactly:
  - `hackrf_transfer` — device, buffer, buffer_length, valid_length, rx_ctx, tx_ctx
  - `HackrfDeviceList` — serial_numbers, usb_board_ids, usb_device_count,
    usb_devices, usb_device_index, devicecount
  - `ReadPartidSerialno` — part_id: [u32; 2], serial_no: [u32; 4]
- [ ] Define `HackrfTransferCallback` type alias
- [ ] Declare `extern "C"` block with all needed functions:
  `hackrf_init`, `hackrf_exit`, `hackrf_close`, `hackrf_device_list`,
  `hackrf_device_list_free`, `hackrf_device_list_open`,
  `hackrf_version_string_read`, `hackrf_board_id_read`, `hackrf_board_id_name`,
  `hackrf_board_partid_serialno_read`, `hackrf_error_name`

---

## Step 3 — Safe `Device` wrapper

- [ ] `pub struct Device(*mut c_void)` — newtype around raw pointer
- [ ] `unsafe impl Send` and `unsafe impl Sync`
- [ ] `impl Drop for Device` — call `hackrf_stop_rx` if streaming, then `hackrf_close` + `hackrf_exit`
- [ ] `Device::open() -> anyhow::Result<Self>`:
  - `hackrf_init()`
  - `hackrf_device_list()` — handle null, handle zero devices
  - If multiple devices: list serials, prompt user to pick one
  - `hackrf_device_list_open(list, index, &mut ptr)`
  - `hackrf_device_list_free(list)`
- [ ] `Device::version() -> anyhow::Result<String>`
- [ ] `Device::board_id() -> anyhow::Result<u8>`
- [ ] `Device::board_name(id: u8) -> String`
- [ ] `Device::serial_number() -> anyhow::Result<String>` — via `hackrf_board_partid_serialno_read`

---

## Step 4 — `main()` smoke test

- [ ] Call `Device::open()`
- [ ] Print board name, firmware version, serial number
- [ ] Verify on real hardware

---
