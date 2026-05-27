# Phase 1 — Device Discovery & Basic Info: Implementation Log

← [Home](../Home.md) | [Roadmap](../Roadmap.md) | [Phase 1 - Device Discovery - Steps](Phase%201%20-%20Device%20Discovery%20-%20Steps.md)

**Status:** ✅ Complete

---

## What was built

A working FFI layer against `libhackrf` with a safe Rust `Device` wrapper.
`main()` opens the device and prints board name, firmware version, and serial number.

---

## FFI approach

The `hackrf` crate on crates.io (0.0.1) was evaluated and rejected — it is
unmaintained and its struct definitions do not match the current `hackrf.h`.
Instead, we wrote a custom `mod hackrf_ffi` with hand-crafted `#[repr(C)]` structs
and an `extern "C"` block, linked via `pkg-config` in `build.rs`.

---

## Critical bug: `HackrfDeviceList` struct layout

The struct was initially defined incorrectly, causing wrong memory offset reads
during device enumeration.

**Wrong definition:**
```rust
#[repr(C)]
pub struct HackrfDeviceList {
    pub serial_numbers: *mut *mut c_char,
    pub usb_device_count: c_int,          // wrong: should be *mut c_int
    pub usb_devices: *mut *mut c_void,
    pub usb_device_index: *mut c_int,
    // missing: usb_board_ids and devicecount
}
```

**Correct definition** (matching `hackrf_device_list_t` in `hackrf.h`):
```rust
#[repr(C)]
pub struct HackrfDeviceList {
    pub serial_numbers: *mut *mut c_char,
    pub usb_board_ids: *mut c_int,        // was missing entirely
    pub usb_device_count: *mut c_int,     // was wrong type (c_int not *mut c_int)
    pub usb_devices: *mut *mut c_void,
    pub usb_device_index: *mut c_int,
    pub devicecount: c_int,               // was missing entirely
}
```

**Impact:** Without this fix, `list.devicecount` read garbage memory, causing
either a false "no device found" error or an attempt to open a non-existent device.

**Fix:** Field-by-field comparison against the `hackrf.h` source.
`Device::open()` uses `list.devicecount` (not `list.usb_device_count`) to check
how many devices are attached.

---

## `Device` wrapper design

```rust
pub struct Device(*mut c_void);

unsafe impl Send for Device {}  // libhackrf is thread-safe for polling and control
unsafe impl Sync for Device {}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            if hackrf_is_streaming(self.0) == 1 {
                let _ = hackrf_stop_rx(self.0);
            }
            hackrf_close(self.0);
            hackrf_exit();
        }
    }
}
```

The `Drop` impl ensures the device is always cleanly closed, even if the app
panics or returns early. `hackrf_exit()` is called unconditionally on drop.

---

## Serial number format

`hackrf_board_partid_serialno_read` returns a `ReadPartidSerialno` struct with
`serial_no: [u32; 4]`. The serial is formatted as a 32-character lowercase hex string:

```rust
format!("{:08x}{:08x}{:08x}{:08x}", s[0], s[1], s[2], s[3])
```

This matches the format shown by `hackrf_info` on the command line.

---

## Multi-device support

`Device::open()` handles multiple connected devices: it lists serials and prompts
the user to choose an index on stdin before the TUI starts. Single-device case
skips the prompt.

---
