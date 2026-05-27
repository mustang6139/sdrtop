# Phase 4 — Architecture Refactor: Implementation Log

← [Home](../Home.md) | [Roadmap](../Roadmap.md) | [Phase 4 - Architecture Refactor - Steps](Phase%204%20-%20Architecture%20Refactor%20-%20Steps.md)

**Status:** ✅ Complete  
**Outcome:** `main.rs` went from 670 lines to 43 lines. Every module is self-contained
and under 200 lines. `cargo build --release` and `cargo clippy -- -D warnings` both
pass clean.

---

## What I did vs. what was planned

The planned step order in [Phase 4 - Architecture Refactor - Steps](Phase%204%20-%20Architecture%20Refactor%20-%20Steps.md) was
1 → 2 → 3 → 4 → 5 → 6 → 7. I reordered steps 1 and 2: `state.rs` was extracted
first because `device.rs` depends on `SdrMetrics` (via `rx_callback`), so it had
to exist before `device.rs` could be written.

Actual execution order:

```
1. state.rs            (planned step 4 — moved first due to dependency)
2. hardware/ffi.rs     (planned step 2)
3. hardware/device.rs  (planned step 3)
4. event.rs            (planned step 4)
5. ui/*.rs             (planned step 5)
6. app.rs              (planned step 6)
7. Final validation    (planned step 7)
```

---

## Step 1 — `src/state.rs`

**Moved from `main.rs`:**
- `THROUGHPUT_HISTORY_LEN`, `LOG_MAX_ENTRIES`, `DEFAULT_LNA_GAIN`, `DEFAULT_VGA_GAIN`,
  `DEFAULT_FREQUENCY`, `DEFAULT_SAMPLE_RATE` — all made `pub const`
- `SdrMetrics` struct — fields made `pub`, `#[derive(Clone)]` kept
- `impl SdrMetrics` — `push_log`, `reset_to_defaults` — made `pub`

**Deviation from plan:** `AppState` wrapper struct was not added. The plan said
"Add `pub struct AppState` wrapping `Arc<Mutex<SdrMetrics>>`", but this was deferred:
it adds no value until Phase 5 introduces `InputMode`, at which point `AppState`
will hold both `SdrMetrics` and the current input mode together.

**`main.rs` change:** removed ~50 lines, added `mod state; use state::*;`.

---

## Step 2 — `src/hardware/ffi.rs`

**Moved from the inner `mod hackrf_ffi` block in `main.rs`:**
- `hackrf_transfer`, `HackrfDeviceList`, `ReadPartidSerialno` (`#[repr(C)]` structs)
- `HackrfTransferCallback` type alias
- All `extern "C"` function declarations — marked `pub` so `device.rs` can call them

**`hardware/mod.rs`:** `pub mod ffi;`

**`main.rs` change:** `mod hackrf_ffi` shrunk to only the `Device` struct and its
impls. `rx_callback` at the top level now references `hardware::ffi::hackrf_transfer`.

**Key decision:** All extern "C" declarations were made `pub`. This is the correct
Rust pattern — `pub` on an extern fn declaration makes it importable via `use`,
allowing `device.rs` to call the raw functions through `use super::ffi::*;`.

---

## Step 3 — `src/hardware/device.rs`

**Moved from `main.rs`:**
- `Device(*mut c_void)` struct
- `unsafe impl Send` and `unsafe impl Sync`
- `#[allow(dead_code)] impl Device` — all 11 methods
- `impl Drop for Device`
- `rx_callback` extern fn — moved here because it closes over `SdrMetrics`
  and logically belongs with the hardware layer

**`hardware/mod.rs`:** added `pub mod device; pub use device::Device;`

**`main.rs` change:** `mod hackrf_ffi` block removed entirely (~220 lines gone).
References updated: `hackrf_ffi::Device::open()` → `hardware::Device::open()`,
`rx_callback` → `hardware::device::rx_callback`.

**Note on `rx_callback` placement:** The callback is an `extern "C"` fn that is
passed as a function pointer to libhackrf. It accesses `SdrMetrics` via a raw
pointer cast (`t.rx_ctx as *const Mutex<SdrMetrics>`). Placing it in `device.rs`
keeps all hardware-touching unsafe code in one place.

---

## Step 4 — `src/event.rs`

**New code** (no existing code to move, just a new abstraction):

```rust
pub enum AppEvent { Key(KeyEvent), Tick }

pub struct EventStream { rx: Receiver<AppEvent> }

impl EventStream {
    pub fn new(tick_rate: Duration) -> Self  // spawns a thread
    pub fn recv(&self) -> AppEvent           // blocking
}
```

**Implementation choice:** Used `std::sync::mpsc` instead of `crossbeam-channel`.
`crossbeam-channel` is planned for Phase 6 (FFT sample handoff, where lock-free
performance matters). `mpsc` is sufficient for the UI event loop at 100 ms tick rate.

**`main.rs` change:** `event::poll` / `event::read` calls removed. The draw loop
now calls `events.recv()` which blocks until a key or tick arrives.

---

## Step 5 — `src/ui/` panel functions

**Moved from the `terminal.draw(|f| { ... })` closure in `main.rs`:**

| File | Function signature | What it renders |
|---|---|---|
| `layout.rs` | `pub fn build(size: Rect) -> Chunks` | outer + body splits |
| `header.rs` | `pub fn render(f, area, board_name, fw, serial)` | top bar |
| `telemetry.rs` | `pub fn render(f, area, m, board_name, serial)` | left panel |
| `gains.rs` | `pub fn render(f, area, m)` | LNA/VGA/SR gauges + sparkline |
| `log.rs` | `pub fn render(f, area, m)` | log panel |
| `footer.rs` | `pub fn render(f, area)` | key hint bar |
| `mod.rs` | `pub fn draw(f, m, board_name, fw, serial)` | calls all of the above |

**`Chunks` struct** holds the five named `Rect` areas (header, body_left,
body_right, log, footer). The gain panel's internal 4-way split lives inside
`gains.rs`, not in `layout.rs`, keeping each file responsible for its own geometry.

**Deviation from plan:** `footer.rs` does not take an `InputMode` parameter.
The plan said `pub fn render(f, area, mode: InputMode)`. `InputMode` does not
exist yet (Phase 5), so the footer renders a fixed string for now. Adding the
parameter is a one-line change when `InputMode` is introduced.

**Filename fix:** The skeleton had a typo — `sprectrum.rs` was renamed to
`spectrum.rs` before writing `mod.rs`.

**`main.rs` change:** The 150-line `terminal.draw` closure became one line:
```rust
terminal.draw(|f| ui::draw(f, &m, board_name, fw_version, serial))?;
```

---

## Step 6 — `src/app.rs`

**Moved from `main.rs`:**
- Device open + board info queries
- `SdrMetrics` initialization
- Initial log messages
- `tokio::spawn` background polling task
- Event loop (`run_app` function)

**`App` struct:**
```rust
pub struct App {
    state: Arc<Mutex<SdrMetrics>>,
    #[allow(dead_code)]  // kept alive for Drop (closes device on exit)
    device: Arc<hardware::Device>,
    board_name: String,
    fw_version: String,
    serial: String,
    events: EventStream,
}
```

**`App::new()`** is a regular `fn` (not `async`). `tokio::spawn` inside it works
because it is called from within the `#[tokio::main]` runtime context.

**`#[allow(dead_code)]` on `device`:** The field is never read after construction,
but it must be kept in the struct so that `Arc<Device>` is not dropped early.
When `App` drops, `Arc` reference count hits zero, `Drop for Device` runs,
and libhackrf shuts down cleanly.

**`main.rs` final state** — 43 lines, terminal setup/teardown only:
```rust
#[tokio::main]
async fn main() -> Result<()> {
    let mut app = match App::new() { Ok(a) => a, Err(e) => { ... } };
    // terminal setup ...
    let result = app.run(&mut terminal);
    // terminal teardown ...
}
```

---

## Step 7 — Final validation

```
cargo build --release   → Finished, 0 warnings
cargo clippy -- -D warnings → Finished, 0 findings
```

---

## Line count before / after

| File | Before | After |
|---|---|---|
| `src/main.rs` | 670 | 43 |
| `src/state.rs` | — | 48 |
| `src/hardware/ffi.rs` | — | 65 |
| `src/hardware/device.rs` | — | 195 |
| `src/event.rs` | — | 36 |
| `src/ui/layout.rs` | — | 35 |
| `src/ui/header.rs` | — | 14 |
| `src/ui/telemetry.rs` | — | 42 |
| `src/ui/gains.rs` | — | 76 |
| `src/ui/log.rs` | — | 18 |
| `src/ui/footer.rs` | — | 14 |
| `src/ui/mod.rs` | — | 22 |
| `src/app.rs` | — | 125 |
