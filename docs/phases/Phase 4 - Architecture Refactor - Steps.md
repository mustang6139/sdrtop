# Phase 4 — Architecture Refactor: Step-by-Step

← [[Home]] | [[Roadmap]] | [[Phase 4 - Architecture Refactor - Log]]

**Goal:** `main.rs` becomes an entry point only. All logic is split into focused
modules. Every step ends with a passing `cargo build`.

---

## Dependency order

```
state.rs          (no deps — central data types)
    ↓
hardware/ffi.rs   (no deps — raw C bindings)
    ↓
hardware/device.rs  (ffi.rs + state.rs, because rx_callback uses SdrMetrics)
    ↓
event.rs          (standalone — crossterm event wrapper)
    ↓
ui/*.rs           (state.rs — render functions take &SdrMetrics)
    ↓
app.rs            (everything above)
    ↓
main.rs           (App::new()?.run() only)
```

---

## Step 1 — `src/state.rs`

Move out of `main.rs`:

- Constants: `THROUGHPUT_HISTORY_LEN`, `LOG_MAX_ENTRIES`, `DEFAULT_LNA_GAIN`,
  `DEFAULT_VGA_GAIN`, `DEFAULT_FREQUENCY`, `DEFAULT_SAMPLE_RATE`
- `SdrMetrics` struct (with `#[derive(Clone)]`)
- `impl SdrMetrics`: `push_log`, `reset_to_defaults`

`main.rs` changes:
- Remove the moved items
- Add `mod state;` and `use crate::state::*;` (or explicit imports)

`cargo build` must pass before moving on.

---

## Step 2 — `src/hardware/ffi.rs`

Move out of `main.rs` (the entire `mod hackrf_ffi` inner content, **excluding** `Device`):

- `hackrf_transfer` struct
- `HackrfDeviceList` struct
- `ReadPartidSerialno` struct
- `HackrfTransferCallback` type alias
- `extern "C" { ... }` block (all raw function declarations)

`hardware/mod.rs`:
```rust
pub mod ffi;
pub mod device;
```

`main.rs` changes:
- Remove `mod hackrf_ffi { ... }` entirely
- Add `mod hardware;` at the top
- Replace `hackrf_ffi::Device` with `hardware::device::Device` (or re-export)

`cargo build` must pass before moving on.

---

## Step 3 — `src/hardware/device.rs`

Move out of `main.rs`:

- `Device` struct (`pub struct Device(*mut c_void)`)
- `unsafe impl Send` and `unsafe impl Sync` for `Device`
- `#[allow(dead_code)] impl Device { ... }` (all methods: `open`, `version`,
  `is_streaming`, `start_rx`, `stop_rx`, `set_lna_gain`, `set_vga_gain`,
  `set_sample_rate`, `set_frequency`, `set_amp_enable`, `board_id`,
  `board_name`, `serial_number`)
- `impl Drop for Device`
- `rx_callback` extern fn (depends on `state::SdrMetrics`)

Imports needed at the top of `device.rs`:
```rust
use libc::{c_int, c_void};
use std::ffi::CStr;
use std::sync::Mutex;
use crate::state::SdrMetrics;
use super::ffi::*;
```

`cargo build` must pass before moving on.

---

## Step 4 — `src/event.rs`

Create a thin wrapper around crossterm's event polling.

Define:
```rust
pub enum AppEvent {
    Key(crossterm::event::KeyEvent),
    Tick,
}
```

`EventStream` struct:
- Spawns a thread on construction
- Thread calls `crossterm::event::poll(100ms)` in a loop
- On a key event: sends `AppEvent::Key(...)`
- On timeout: sends `AppEvent::Tick`
- Exposes `fn recv(&self) -> AppEvent` (blocking)

`run_app` in `main.rs` switches from `event::poll` / `event::read` to reading
from the `EventStream` receiver.

`cargo build` must pass before moving on.

---

## Step 5 — `src/ui/` panel functions

Create each render function in its own file. Each function signature follows
the pattern `pub fn render(f: &mut Frame, area: Rect, ...)`.

### `ui/header.rs`
```rust
pub fn render(f: &mut Frame, area: Rect, board_name: &str, fw: &str, serial: &str)
```

### `ui/telemetry.rs`
```rust
pub fn render(f: &mut Frame, area: Rect, m: &SdrMetrics, board_name: &str, serial: &str)
```

### `ui/gains.rs`
```rust
pub fn render(f: &mut Frame, area: Rect, m: &SdrMetrics)
```
Contains: LNA gauge, VGA gauge, sample-rate gauge, USB throughput sparkline.

### `ui/log.rs`
```rust
pub fn render(f: &mut Frame, area: Rect, m: &SdrMetrics)
```

### `ui/footer.rs`
```rust
pub fn render(f: &mut Frame, area: Rect)
```

### `ui/layout.rs`
```rust
pub struct Chunks {
    pub header: Rect,
    pub body_left: Rect,
    pub body_right: Rect,
    pub log: Rect,
    pub footer: Rect,
}

pub fn build(size: Rect) -> Chunks
```

### `ui/mod.rs`
```rust
pub fn draw(f: &mut Frame, m: &SdrMetrics, board_name: &str, fw: &str, serial: &str) {
    let chunks = layout::build(f.size());
    header::render(f, chunks.header, board_name, fw, serial);
    telemetry::render(f, chunks.body_left, m, board_name, serial);
    gains::render(f, chunks.body_right, m);
    log::render(f, chunks.log, m);
    footer::render(f, chunks.footer);
}
```

`terminal.draw(|f| { ... })` in `run_app` is replaced by:
```rust
terminal.draw(|f| ui::draw(f, &m, board_name, fw_version, serial))?;
```

`cargo build` must pass before moving on.

---

## Step 6 — `src/app.rs`

Move `run_app` (and `main` body) into an `App` struct.

```rust
pub struct App {
    state: Arc<Mutex<SdrMetrics>>,
    device: Arc<hardware::Device>,   // kept for Drop
    board_name: String,
    fw_version: String,
    serial: String,
    events: EventStream,
}

impl App {
    pub fn new() -> anyhow::Result<Self> { ... }
    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> { ... }
}
```

`main.rs` becomes:
```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut app = App::new()?;
    // terminal setup
    let result = app.run(&mut terminal);
    // terminal teardown
}
```

`cargo build` must pass before moving on.

---

## Step 7 — Final validation

```bash
cargo build --release   # zero errors, zero warnings
cargo clippy -- -D warnings  # zero findings
```

Fix any warnings before marking Phase 4 complete in [[Roadmap]].

See [[Phase 4 - Architecture Refactor - Log]] for what actually happened during execution.
