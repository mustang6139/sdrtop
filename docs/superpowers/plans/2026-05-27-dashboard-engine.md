# Dashboard Engine & Hardware Health Panels — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the fixed TUI layout with a modular panel system (Phase 6), then add hardware health metrics and three new panels (Phase 7).

**Architecture:** Every display panel implements a `Panel` trait and registers in a `PanelRegistry`. A `LayoutEngine` reads the active preset from config and dispatches rendering. New hardware metrics are computed in `rx_callback` and the polling task, stored in `SdrMetrics`, exposed via three new panels.

**Tech Stack:** Rust stable, ratatui 0.26, tokio, serde 1 + toml 0.8 (added in Task 1), libc 0.2 (already present, used for /proc in Task 13).

**Note on commits:** The project owner handles all git commits. Steps do not include commit instructions — validate with `cargo build` and `cargo test` instead.

**Prerequisite:** Phase 5 (Interactive Controls) must be complete. `InputMode` enum and `input_mode: InputMode` field must exist in `src/state.rs` before starting Task 1.

---

## File Map

### Phase 6 — Dashboard Engine

| Action | File | Responsibility |
|---|---|---|
| Create | `src/ui/panel.rs` | `Panel` trait definition |
| Create | `src/ui/registry.rs` | `PanelRegistry` — name → boxed panel |
| Create | `src/ui/engine.rs` | `LayoutEngine` — reads config, builds layout, dispatches |
| Modify | `src/state.rs` | Add `AppState` type alias |
| Modify | `src/config.rs` | Add `LayoutConfig`, `PresetConfig`, `PanelSpec`, `Position` |
| Modify | `Cargo.toml` | Add `serde`, `toml` dependencies |
| Modify | `src/ui/header.rs` | Wrap in `HeaderPanel` struct implementing `Panel` |
| Modify | `src/ui/telemetry.rs` | Wrap in `TelemetryPanel` struct implementing `Panel` |
| Modify | `src/ui/gains.rs` | Wrap in `GainsPanel` struct implementing `Panel` |
| Modify | `src/ui/log.rs` | Wrap in `LogPanel` struct implementing `Panel` |
| Modify | `src/ui/footer.rs` | Wrap in `FooterPanel` struct implementing `Panel` |
| Modify | `src/ui/mod.rs` | Remove `draw()`, export panel structs |
| Modify | `src/app.rs` | Create registry + engine, handle `p`/`1`/`2`/`3` keys |

### Phase 7 — Hardware Health Panels

| Action | File | Responsibility |
|---|---|---|
| Modify | `src/state.rs` | Add new `SdrMetrics` fields for all hardware health metrics |
| Modify | `src/hardware/device.rs` | Drop detection + ADC saturation + jitter in `rx_callback` |
| Modify | `src/app.rs` | System resource polling task + wire new metrics |
| Create | `src/ui/hardware_health.rs` | `HardwareHealthPanel` — drop rate, saturation, jitter |
| Create | `src/ui/iq_diagnostics.rs` | `IqDiagnosticsPanel` — DC offset, IQ imbalance |
| Create | `src/ui/system_resources.rs` | `SystemResourcesPanel` — CPU%, RAM, USB throughput |
| Modify | `src/ui/mod.rs` | Export three new panel structs |
| Modify | `src/app.rs` | Register three new panels, add `monitoring` preset |
| Modify | `docs/Roadmap.md` | Update phase table and add Phase 6/7 sections |

---

## Phase 6 — Dashboard Engine

---

### Task 1: Add serde + toml to Cargo.toml

**Files:**
- Modify: `Cargo.toml`

- [ ] Add `serde` and `toml` to `[dependencies]` in `Cargo.toml`:

```toml
serde = { version = "1", features = ["derive"] }
toml = "0.8"
```

- [ ] Verify it compiles:

```bash
cargo build
```

Expected: `Finished` with no errors.

---

### Task 2: Panel trait

**Files:**
- Create: `src/ui/panel.rs`

- [ ] Create `src/ui/panel.rs` with the following content:

```rust
use ratatui::{Frame, layout::Rect};
use crate::state::SdrMetrics;

pub trait Panel: Send + Sync {
    fn name(&self) -> &'static str;
    fn min_size(&self) -> (u16, u16);
    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics);
}
```

- [ ] Add `pub mod panel;` to `src/ui/mod.rs`.

- [ ] Verify:

```bash
cargo build
```

Expected: `Finished` with no errors.

- [ ] Write a unit test at the bottom of `src/ui/panel.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct DummyPanel;
    impl Panel for DummyPanel {
        fn name(&self) -> &'static str { "dummy" }
        fn min_size(&self) -> (u16, u16) { (10, 3) }
        fn render(&self, _f: &mut Frame, _area: Rect, _state: &SdrMetrics) {}
    }

    #[test]
    fn panel_name_and_min_size() {
        let p = DummyPanel;
        assert_eq!(p.name(), "dummy");
        assert_eq!(p.min_size(), (10, 3));
    }
}
```

- [ ] Run the test:

```bash
cargo test ui::panel
```

Expected: `test ui::panel::tests::panel_name_and_min_size ... ok`

---

### Task 3: PanelRegistry

**Files:**
- Create: `src/ui/registry.rs`

- [ ] Create `src/ui/registry.rs`:

```rust
use std::collections::HashMap;
use super::panel::Panel;

pub struct PanelRegistry {
    panels: HashMap<&'static str, Box<dyn Panel>>,
}

impl PanelRegistry {
    pub fn new() -> Self {
        Self { panels: HashMap::new() }
    }

    pub fn register(&mut self, panel: impl Panel + 'static) {
        self.panels.insert(panel.name(), Box::new(panel));
    }

    pub fn get(&self, name: &str) -> Option<&dyn Panel> {
        self.panels.get(name).map(|p| p.as_ref())
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.panels.keys().copied().collect()
    }
}
```

- [ ] Add `pub mod registry;` to `src/ui/mod.rs`.

- [ ] Write a unit test at the bottom of `src/ui/registry.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::panel::Panel;
    use ratatui::{Frame, layout::Rect};
    use crate::state::SdrMetrics;

    struct NamedPanel(&'static str);
    impl Panel for NamedPanel {
        fn name(&self) -> &'static str { self.0 }
        fn min_size(&self) -> (u16, u16) { (0, 0) }
        fn render(&self, _f: &mut Frame, _area: Rect, _state: &SdrMetrics) {}
    }

    #[test]
    fn register_and_retrieve() {
        let mut reg = PanelRegistry::new();
        reg.register(NamedPanel("alpha"));
        reg.register(NamedPanel("beta"));
        assert!(reg.get("alpha").is_some());
        assert!(reg.get("beta").is_some());
        assert!(reg.get("gamma").is_none());
    }
}
```

- [ ] Run the test:

```bash
cargo test ui::registry
```

Expected: `test ui::registry::tests::register_and_retrieve ... ok`

---

### Task 4: Layout config structs

**Files:**
- Modify: `src/config.rs`
- Modify: `Cargo.toml` (already done in Task 1)

- [ ] Replace the entire contents of `src/config.rs` with:

```rust
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Position {
    Top,
    Bottom,
    Left,
    Right,
    Body,
}

#[derive(Deserialize, Clone, Debug)]
pub struct PanelSpec {
    pub name: String,
    pub position: Position,
    /// Fixed height in terminal rows — used for Top and Bottom panels.
    #[serde(default)]
    pub height: Option<u16>,
    /// Width as a percentage of the body zone — used for Left and Right panels.
    #[serde(default)]
    pub width_pct: Option<u16>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct PresetConfig {
    pub panels: Vec<PanelSpec>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct LayoutConfig {
    pub active_preset: String,
    pub presets: HashMap<String, PresetConfig>,
}

impl LayoutConfig {
    pub fn default_config() -> Self {
        use Position::*;
        let minimal = PresetConfig {
            panels: vec![
                PanelSpec { name: "header".into(),   position: Top,    height: Some(3), width_pct: None },
                PanelSpec { name: "telemetry".into(), position: Body,   height: None,    width_pct: None },
                PanelSpec { name: "gains".into(),     position: Right,  height: None,    width_pct: Some(50) },
                PanelSpec { name: "log".into(),       position: Bottom, height: Some(7), width_pct: None },
                PanelSpec { name: "footer".into(),    position: Bottom, height: Some(3), width_pct: None },
            ],
        };
        let mut presets = HashMap::new();
        presets.insert("minimal".into(), minimal);
        Self {
            active_preset: "minimal".into(),
            presets,
        }
    }

    pub fn active_panels(&self) -> &[PanelSpec] {
        self.presets
            .get(&self.active_preset)
            .map(|p| p.panels.as_slice())
            .unwrap_or(&[])
    }
}
```

- [ ] Write unit tests at the bottom of `src/config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_minimal_preset() {
        let cfg = LayoutConfig::default_config();
        assert_eq!(cfg.active_preset, "minimal");
        assert!(!cfg.active_panels().is_empty());
    }

    #[test]
    fn active_panels_returns_correct_preset() {
        let cfg = LayoutConfig::default_config();
        let panels = cfg.active_panels();
        let names: Vec<&str> = panels.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"header"));
        assert!(names.contains(&"footer"));
    }

    #[test]
    fn deserialize_from_toml() {
        let raw = r#"
            active_preset = "minimal"
            [presets.minimal]
            panels = [
              { name = "header", position = "top", height = 3 },
              { name = "footer", position = "bottom", height = 3 },
            ]
        "#;
        let cfg: LayoutConfig = toml::from_str(raw).unwrap();
        assert_eq!(cfg.active_panels().len(), 2);
    }
}
```

- [ ] Add `use toml;` at the top of `src/config.rs` in the test module (it's already in Cargo.toml from Task 1).

- [ ] Run tests:

```bash
cargo test config::
```

Expected: all three config tests pass.

---

### Task 5: Migrate existing panels to Panel trait

**Files:**
- Modify: `src/ui/header.rs`
- Modify: `src/ui/telemetry.rs`
- Modify: `src/ui/gains.rs`
- Modify: `src/ui/log.rs`
- Modify: `src/ui/footer.rs`

Each existing panel file keeps its render function unchanged and gains a public struct that implements `Panel`. The render function is called from inside the `Panel::render` implementation.

- [ ] At the bottom of `src/ui/header.rs`, add:

```rust
use super::panel::Panel;
use crate::state::SdrMetrics;

pub struct HeaderPanel {
    pub board_name: String,
    pub fw_version: String,
    pub serial: String,
}

impl Panel for HeaderPanel {
    fn name(&self) -> &'static str { "header" }
    fn min_size(&self) -> (u16, u16) { (40, 3) }
    fn render(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect, _state: &SdrMetrics) {
        render(f, area, &self.board_name, &self.fw_version, &self.serial);
    }
}
```

- [ ] At the bottom of `src/ui/telemetry.rs`, add:

```rust
use super::panel::Panel;
use crate::state::SdrMetrics;

pub struct TelemetryPanel {
    pub board_name: String,
    pub serial: String,
}

impl Panel for TelemetryPanel {
    fn name(&self) -> &'static str { "telemetry" }
    fn min_size(&self) -> (u16, u16) { (30, 10) }
    fn render(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect, state: &SdrMetrics) {
        render(f, area, state, &self.board_name, &self.serial);
    }
}
```

- [ ] At the bottom of `src/ui/gains.rs`, add:

```rust
use super::panel::Panel;
use crate::state::SdrMetrics;

pub struct GainsPanel;

impl Panel for GainsPanel {
    fn name(&self) -> &'static str { "gains" }
    fn min_size(&self) -> (u16, u16) { (20, 12) }
    fn render(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect, state: &SdrMetrics) {
        render(f, area, state);
    }
}
```

- [ ] At the bottom of `src/ui/log.rs`, add:

```rust
use super::panel::Panel;
use crate::state::SdrMetrics;

pub struct LogPanel;

impl Panel for LogPanel {
    fn name(&self) -> &'static str { "log" }
    fn min_size(&self) -> (u16, u16) { (20, 7) }
    fn render(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect, state: &SdrMetrics) {
        render(f, area, state);
    }
}
```

- [ ] At the bottom of `src/ui/footer.rs`, add:

```rust
use super::panel::Panel;
use crate::state::SdrMetrics;

pub struct FooterPanel;

impl Panel for FooterPanel {
    fn name(&self) -> &'static str { "footer" }
    fn min_size(&self) -> (u16, u16) { (20, 3) }
    fn render(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect, _state: &SdrMetrics) {
        render(f, area);
    }
}
```

- [ ] Verify:

```bash
cargo build
```

Expected: `Finished` with no errors.

---

### Task 6: LayoutEngine

**Files:**
- Create: `src/ui/engine.rs`

- [ ] Create `src/ui/engine.rs`:

```rust
use ratatui::{Frame, layout::{Layout, Constraint, Direction, Rect}};
use crate::config::{LayoutConfig, Position};
use crate::state::SdrMetrics;
use super::registry::PanelRegistry;

pub struct LayoutEngine {
    config: LayoutConfig,
    registry: PanelRegistry,
}

impl LayoutEngine {
    pub fn new(config: LayoutConfig, registry: PanelRegistry) -> Self {
        Self { config, registry }
    }

    pub fn active_preset(&self) -> &str {
        &self.config.active_preset
    }

    pub fn cycle_preset(&mut self) {
        let presets: Vec<String> = self.config.presets.keys().cloned().collect();
        let mut sorted = presets;
        sorted.sort();
        let current = sorted.iter().position(|p| p == &self.config.active_preset).unwrap_or(0);
        let next = (current + 1) % sorted.len();
        self.config.active_preset = sorted[next].clone();
    }

    pub fn set_preset(&mut self, name: &str) {
        if self.config.presets.contains_key(name) {
            self.config.active_preset = name.to_string();
        }
    }

    pub fn draw(&self, f: &mut Frame, state: &SdrMetrics) {
        let specs = self.config.active_panels();
        let size = f.size();

        // Collect top and bottom panel heights
        let top_specs: Vec<_> = specs.iter().filter(|s| s.position == Position::Top).collect();
        let bottom_specs: Vec<_> = specs.iter().filter(|s| s.position == Position::Bottom).collect();
        let body_specs: Vec<_> = specs.iter().filter(|s| {
            s.position == Position::Left || s.position == Position::Right || s.position == Position::Body
        }).collect();

        // Build outer vertical layout
        let top_height: u16 = top_specs.iter().map(|s| s.height.unwrap_or(3)).sum();
        let bottom_height: u16 = bottom_specs.iter().map(|s| s.height.unwrap_or(3)).sum();
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(top_height),
                Constraint::Min(0),
                Constraint::Length(bottom_height),
            ])
            .split(size);

        // Render top panels
        let mut top_y = outer[0].y;
        for spec in &top_specs {
            let h = spec.height.unwrap_or(3);
            let area = Rect { x: outer[0].x, y: top_y, width: outer[0].width, height: h };
            if let Some(panel) = self.registry.get(&spec.name) {
                panel.render(f, area, state);
            }
            top_y += h;
        }

        // Render bottom panels
        let mut bottom_y = outer[2].y;
        for spec in &bottom_specs {
            let h = spec.height.unwrap_or(3);
            let area = Rect { x: outer[2].x, y: bottom_y, width: outer[2].width, height: h };
            if let Some(panel) = self.registry.get(&spec.name) {
                panel.render(f, area, state);
            }
            bottom_y += h;
        }

        // Render body panels
        if !body_specs.is_empty() {
            let left_specs: Vec<_> = body_specs.iter().filter(|s| s.position == Position::Left).collect();
            let right_specs: Vec<_> = body_specs.iter().filter(|s| s.position == Position::Right).collect();
            let center_specs: Vec<_> = body_specs.iter().filter(|s| s.position == Position::Body).collect();

            let left_pct: u16 = left_specs.iter().map(|s| s.width_pct.unwrap_or(50)).sum();
            let right_pct: u16 = right_specs.iter().map(|s| s.width_pct.unwrap_or(50)).sum();

            let body_area = outer[1];
            let body_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(left_pct),
                    Constraint::Min(0),
                    Constraint::Percentage(right_pct),
                ])
                .split(body_area);

            // Left column — stack vertically
            if !left_specs.is_empty() {
                let constraints: Vec<Constraint> = left_specs.iter()
                    .map(|_| Constraint::Min(0))
                    .collect();
                let left_areas = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(constraints)
                    .split(body_layout[0]);
                for (spec, area) in left_specs.iter().zip(left_areas.iter()) {
                    if let Some(panel) = self.registry.get(&spec.name) {
                        panel.render(f, *area, state);
                    }
                }
            }

            // Center — stack vertically
            if !center_specs.is_empty() {
                let constraints: Vec<Constraint> = center_specs.iter()
                    .map(|_| Constraint::Min(0))
                    .collect();
                let center_areas = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(constraints)
                    .split(body_layout[1]);
                for (spec, area) in center_specs.iter().zip(center_areas.iter()) {
                    if let Some(panel) = self.registry.get(&spec.name) {
                        panel.render(f, *area, state);
                    }
                }
            }

            // Right column — stack vertically
            if !right_specs.is_empty() {
                let constraints: Vec<Constraint> = right_specs.iter()
                    .map(|_| Constraint::Min(0))
                    .collect();
                let right_areas = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(constraints)
                    .split(body_layout[2]);
                for (spec, area) in right_specs.iter().zip(right_areas.iter()) {
                    if let Some(panel) = self.registry.get(&spec.name) {
                        panel.render(f, *area, state);
                    }
                }
            }
        }
    }
}
```

- [ ] Add `pub mod engine;` to `src/ui/mod.rs`.

- [ ] Verify:

```bash
cargo build
```

Expected: `Finished` with no errors.

---

### Task 7: Wire LayoutEngine into App

**Files:**
- Modify: `src/app.rs`
- Modify: `src/ui/mod.rs`

- [ ] In `src/ui/mod.rs`, remove the existing `pub fn draw(...)` function. Export the panel structs instead:

```rust
pub mod engine;
pub mod footer;
pub mod gains;
pub mod header;
pub mod layout;
pub mod log;
pub mod overlay;
pub mod panel;
pub mod registry;
pub mod sparkline;
pub mod spectrum;
pub mod telemetry;
pub mod waterfall;

pub use engine::LayoutEngine;
pub use footer::FooterPanel;
pub use gains::GainsPanel;
pub use header::HeaderPanel;
pub use log::LogPanel;
pub use registry::PanelRegistry;
pub use telemetry::TelemetryPanel;
```

- [ ] In `src/app.rs`, add `engine: ui::LayoutEngine` to the `App` struct, replacing the direct board_name/fw_version/serial fields used for rendering (keep them for reference but use them to build the engine):

```rust
pub struct App {
    state: Arc<Mutex<SdrMetrics>>,
    #[allow(dead_code)]
    device: Arc<hardware::Device>,
    board_name: String,
    fw_version: String,
    serial: String,
    events: EventStream,
    engine: ui::LayoutEngine,
}
```

- [ ] In `App::new()`, build the registry and engine after opening the device:

```rust
let config = crate::config::LayoutConfig::default_config();

let mut registry = ui::PanelRegistry::new();
registry.register(ui::HeaderPanel {
    board_name: board_name.clone(),
    fw_version: fw_version.clone(),
    serial: serial.clone(),
});
registry.register(ui::TelemetryPanel {
    board_name: board_name.clone(),
    serial: serial.clone(),
});
registry.register(ui::GainsPanel);
registry.register(ui::LogPanel);
registry.register(ui::FooterPanel);

let engine = ui::LayoutEngine::new(config, registry);
```

- [ ] In `App::run()`, replace the `terminal.draw(|f| ui::draw(...))` call with:

```rust
terminal.draw(|f| {
    let m = self.state.lock().unwrap();
    self.engine.draw(f, &m);
})?;
```

- [ ] In the event loop inside `App::run()`, add handling for preset keys after the existing key handlers:

```rust
KeyCode::Char('p') => { self.engine.cycle_preset(); }
KeyCode::Char('1') => { self.engine.set_preset("minimal"); }
```

- [ ] Verify the full build:

```bash
cargo build
```

Expected: `Finished` with no errors.

- [ ] Run the app manually and verify:
  - The TUI renders correctly with the `minimal` preset
  - Pressing `p` cycles presets (currently only `minimal` exists, so it stays)
  - All existing keys (`q`, `Space`, `r`) still work

---

**Phase 6 is complete.** The app is functionally identical but uses the panel system. Every future panel is a `Panel` implementation + one `registry.register()` call in `App::new()`.

---

## Phase 7 — Hardware Health Panels

---

### Task 8: New SdrMetrics fields

**Files:**
- Modify: `src/state.rs`

- [ ] Add the following fields to the `SdrMetrics` struct in `src/state.rs`:

```rust
// Sample drop tracking
pub drops_per_sec: u64,
pub total_drops_session: u64,
pub drop_history: VecDeque<u64>,          // 64-point sparkline, drops/sec

// ADC saturation
pub adc_saturation_pct: f32,              // current poll cycle
pub adc_saturation_peak: f32,            // session maximum
pub saturation_history: VecDeque<f32>,   // 64-point sparkline

// IQ diagnostics
pub iq_imbalance_db: f32,                // positive = I stronger, negative = Q stronger
pub dc_offset_i: f32,
pub dc_offset_q: f32,

// Callback jitter
pub callback_jitter_us: u64,             // rolling variance in µs

// System resources
pub process_cpu_pct: f32,
pub process_rss_mb: u64,

// Accumulators for rx_callback → polling task handoff
pub bytes_i_sum: i64,                    // sum of I channel values since last poll
pub bytes_q_sum: i64,                    // sum of Q channel values since last poll
pub i_power_sum: f64,                    // sum of I² since last poll
pub q_power_sum: f64,                    // sum of Q² since last poll
pub saturated_samples: u64,             // saturated sample count since last poll
pub last_callback_time: Option<std::time::Instant>,
pub jitter_sum_us: u64,                  // accumulated jitter for rolling average
pub jitter_count: u64,
```

- [ ] Update `SdrMetrics::new()` (or wherever it is initialized in `App::new()`) to include default values for all new fields:

```rust
drops_per_sec: 0,
total_drops_session: 0,
drop_history: VecDeque::with_capacity(THROUGHPUT_HISTORY_LEN),
adc_saturation_pct: 0.0,
adc_saturation_peak: 0.0,
saturation_history: VecDeque::with_capacity(THROUGHPUT_HISTORY_LEN),
iq_imbalance_db: 0.0,
dc_offset_i: 0.0,
dc_offset_q: 0.0,
callback_jitter_us: 0,
process_cpu_pct: 0.0,
process_rss_mb: 0,
bytes_i_sum: 0,
bytes_q_sum: 0,
i_power_sum: 0.0,
q_power_sum: 0.0,
saturated_samples: 0,
last_callback_time: None,
jitter_sum_us: 0,
jitter_count: 0,
```

- [ ] Verify:

```bash
cargo build
```

Expected: `Finished` with no errors.

---

### Task 9: Drop detection + ADC saturation in rx_callback

**Files:**
- Modify: `src/hardware/device.rs`

The HackRF `hackrf_transfer` struct has `buffer_length` (requested) and `valid_length` (actual). If `valid_length < buffer_length`, samples were dropped. IQ bytes are interleaved: `[I0, Q0, I1, Q1, ...]`. A saturated sample is a byte at `0x00` (i.e. `0u8`) or `0x7F` (127u8) in signed 8-bit terms — these correspond to the rails of the ADC.

- [ ] In `src/hardware/device.rs`, replace the body of `rx_callback` with:

```rust
pub extern "C" fn rx_callback(transfer: *mut hackrf_transfer) -> c_int {
    unsafe {
        let t = &*transfer;
        let metrics_ptr = t.rx_ctx as *const Mutex<SdrMetrics>;
        if metrics_ptr.is_null() { return 0; }
        let Ok(mut m) = (*metrics_ptr).lock() else { return 0; };

        let buf = std::slice::from_raw_parts(
            t.buffer as *const u8,
            t.valid_length as usize,
        );

        // Byte count for throughput
        m.bytes_since_last_poll += t.valid_length as u64;

        // Drop detection: valid < buffer means libhackrf dropped samples
        if t.valid_length < t.buffer_length {
            let dropped = (t.buffer_length - t.valid_length) as u64 / 2; // IQ pairs
            m.total_drops_session += dropped;
        }

        // ADC saturation + IQ accumulation
        let mut saturated: u64 = 0;
        let mut i_sum: i64 = 0;
        let mut q_sum: i64 = 0;
        let mut i_pow: f64 = 0.0;
        let mut q_pow: f64 = 0.0;

        for chunk in buf.chunks_exact(2) {
            let i = chunk[0] as i8 as i64;
            let q = chunk[1] as i8 as i64;
            i_sum += i;
            q_sum += q;
            i_pow += (i * i) as f64;
            q_pow += (q * q) as f64;
            // Saturation: signed 8-bit rails are -128 (0x80) and 127 (0x7F)
            if chunk[0] == 0x80 || chunk[0] == 0x7F { saturated += 1; }
            if chunk[1] == 0x80 || chunk[1] == 0x7F { saturated += 1; }
        }

        m.saturated_samples += saturated;
        m.bytes_i_sum += i_sum;
        m.bytes_q_sum += q_sum;
        m.i_power_sum += i_pow;
        m.q_power_sum += q_pow;

        // Callback jitter
        let now = std::time::Instant::now();
        if let Some(last) = m.last_callback_time {
            let jitter = now.duration_since(last).as_micros() as u64;
            m.jitter_sum_us += jitter;
            m.jitter_count += 1;
        }
        m.last_callback_time = Some(now);
    }
    0
}
```

- [ ] Write a unit test for the saturation detection logic in `src/hardware/device.rs`:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn saturation_detection() {
        // Saturated bytes: 0x80 (i8 min) and 0x7F (i8 max)
        let saturated_i: u8 = 0x7F;
        let saturated_q: u8 = 0x80;
        let normal: u8 = 0x40;

        assert!(saturated_i == 0x7F || saturated_i == 0x80);
        assert!(saturated_q == 0x7F || saturated_q == 0x80);
        assert!(normal != 0x7F && normal != 0x80);
    }
}
```

- [ ] Verify:

```bash
cargo build
cargo test hardware::device::tests
```

---

### Task 10: Compute health metrics in polling task

**Files:**
- Modify: `src/app.rs`

In the background `tokio::spawn` polling task in `App::new()`, after the existing throughput computation, add computation of the new metrics from the accumulated values.

- [ ] In the polling task loop in `src/app.rs`, after the existing throughput computation block, add:

```rust
// Drop rate
if elapsed_ms > 0 {
    m.drops_per_sec = (m.total_drops_session.saturating_sub(last_total_drops)) * 1000 / elapsed_ms;
}
if m.drop_history.len() >= crate::state::THROUGHPUT_HISTORY_LEN {
    m.drop_history.pop_front();
}
m.drop_history.push_back(m.drops_per_sec);

// ADC saturation %
let total_samples = m.bytes_since_last_poll; // already reset below after this block
if total_samples > 0 {
    m.adc_saturation_pct = (m.saturated_samples as f32 / total_samples as f32) * 100.0;
    if m.adc_saturation_pct > m.adc_saturation_peak {
        m.adc_saturation_peak = m.adc_saturation_pct;
    }
}
if m.saturation_history.len() >= crate::state::THROUGHPUT_HISTORY_LEN {
    m.saturation_history.pop_front();
}
m.saturation_history.push_back(m.adc_saturation_pct);

// IQ diagnostics — compute from accumulated sums
let n = (m.bytes_since_last_poll / 2).max(1) as f64;
m.dc_offset_i = m.bytes_i_sum as f32 / n as f32 / 128.0;
m.dc_offset_q = m.bytes_q_sum as f32 / n as f32 / 128.0;
let i_rms = (m.i_power_sum / n).sqrt();
let q_rms = (m.q_power_sum / n).sqrt();
if q_rms > 0.0 {
    m.iq_imbalance_db = 20.0 * (i_rms / q_rms).log10() as f32;
}

// Callback jitter — rolling average
if m.jitter_count > 0 {
    m.callback_jitter_us = m.jitter_sum_us / m.jitter_count;
}

// Reset accumulators
let last_total_drops = m.total_drops_session;
m.saturated_samples = 0;
m.bytes_i_sum = 0;
m.bytes_q_sum = 0;
m.i_power_sum = 0.0;
m.q_power_sum = 0.0;
m.jitter_sum_us = 0;
m.jitter_count = 0;
```

Note: `last_total_drops` must be declared before the lock is taken and updated each cycle to track the delta.

- [ ] Add unit tests for the IQ imbalance formula in `src/app.rs`:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn iq_imbalance_zero_for_balanced() {
        let i_rms = 10.0_f64;
        let q_rms = 10.0_f64;
        let imbalance = 20.0 * (i_rms / q_rms).log10() as f32;
        assert!((imbalance).abs() < 0.001);
    }

    #[test]
    fn iq_imbalance_positive_when_i_stronger() {
        let i_rms = 20.0_f64;
        let q_rms = 10.0_f64;
        let imbalance = 20.0 * (i_rms / q_rms).log10() as f32;
        assert!(imbalance > 0.0);
    }
}
```

- [ ] Verify:

```bash
cargo build
cargo test app::tests
```

---

### Task 11: System resource polling

**Files:**
- Modify: `src/app.rs`

Linux `/proc/self/stat` field 14 (utime) and field 15 (stime) give CPU ticks used. `/proc/self/status` line `VmRSS` gives RSS in kB. We compute CPU% by comparing tick deltas against wall-clock elapsed time.

- [ ] Add a helper function in `src/app.rs` (outside `impl App`):

```rust
fn read_process_stats() -> Option<(u64, u64)> {
    // CPU ticks (utime + stime from /proc/self/stat)
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let fields: Vec<&str> = stat.split_whitespace().collect();
    let utime: u64 = fields.get(13)?.parse().ok()?;
    let stime: u64 = fields.get(14)?.parse().ok()?;
    let total_ticks = utime + stime;

    // RSS in MB from /proc/self/status
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let rss_kb: u64 = status.lines()
        .find(|l| l.starts_with("VmRSS:"))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()?;
    let rss_mb = rss_kb / 1024;

    Some((total_ticks, rss_mb))
}
```

- [ ] In `App::new()`, spawn a second `tokio::spawn` task for system resources that updates `process_cpu_pct` and `process_rss_mb` every second:

```rust
let sys_metrics = Arc::clone(&metrics);
tokio::spawn(async move {
    let ticks_per_sec = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as f64;
    let mut last_ticks: u64 = 0;
    let mut last_time = std::time::Instant::now();
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        if let Some((total_ticks, rss_mb)) = read_process_stats() {
            let elapsed = last_time.elapsed().as_secs_f64();
            let tick_delta = total_ticks.saturating_sub(last_ticks) as f64;
            let cpu_pct = if elapsed > 0.0 {
                (tick_delta / ticks_per_sec / elapsed * 100.0) as f32
            } else {
                0.0
            };
            last_ticks = total_ticks;
            last_time = std::time::Instant::now();
            if let Ok(mut m) = sys_metrics.lock() {
                m.process_cpu_pct = cpu_pct;
                m.process_rss_mb = rss_mb;
            }
        }
    }
});
```

- [ ] Verify:

```bash
cargo build
```

---

### Task 12: HardwareHealthPanel

**Files:**
- Create: `src/ui/hardware_health.rs`

- [ ] Create `src/ui/hardware_health.rs`:

```rust
use ratatui::{
    Frame,
    layout::{Rect, Layout, Direction, Constraint},
    style::{Color, Style},
    widgets::{Block, Borders, Gauge, Paragraph, Sparkline},
    text::Span,
};
use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

pub struct HardwareHealthPanel;

fn health_color(value: f64, warn: f64, crit: f64) -> Color {
    if value >= crit { Color::Red }
    else if value >= warn { Color::Yellow }
    else { Color::Green }
}

impl Panel for HardwareHealthPanel {
    fn name(&self) -> &'static str { "hardware_health" }
    fn min_size(&self) -> (u16, u16) { (30, 12) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics) {
        let block = Block::default()
            .title(" Hardware Health ")
            .borders(Borders::ALL);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // drop rate label
                Constraint::Length(2), // drop sparkline
                Constraint::Length(2), // saturation label
                Constraint::Length(2), // saturation sparkline
                Constraint::Length(1), // jitter
            ])
            .split(inner);

        // Drop rate
        let drop_color = health_color(state.drops_per_sec as f64, 1.0, 10.0);
        let drop_text = format!(
            "Sample drops: {}/s  (total: {})",
            state.drops_per_sec, state.total_drops_session
        );
        f.render_widget(
            Paragraph::new(Span::styled(drop_text, Style::default().fg(drop_color))),
            sections[0],
        );
        let drop_data: Vec<u64> = state.drop_history.iter().cloned().collect();
        f.render_widget(
            Sparkline::default()
                .data(&drop_data)
                .style(Style::default().fg(drop_color)),
            sections[1],
        );

        // ADC saturation
        let sat_color = health_color(state.adc_saturation_pct as f64, 1.0, 5.0);
        let sat_text = format!(
            "ADC saturation: {:.1}%  (peak: {:.1}%)",
            state.adc_saturation_pct, state.adc_saturation_peak
        );
        f.render_widget(
            Paragraph::new(Span::styled(sat_text, Style::default().fg(sat_color))),
            sections[2],
        );
        let sat_data: Vec<u64> = state.saturation_history.iter()
            .map(|v| *v as u64)
            .collect();
        f.render_widget(
            Sparkline::default()
                .data(&sat_data)
                .style(Style::default().fg(sat_color)),
            sections[3],
        );

        // Jitter
        let jitter_color = health_color(state.callback_jitter_us as f64, 500.0, 2000.0);
        let jitter_text = format!("Callback jitter: {} µs", state.callback_jitter_us);
        f.render_widget(
            Paragraph::new(Span::styled(jitter_text, Style::default().fg(jitter_color))),
            sections[4],
        );
    }
}
```

- [ ] Add `pub mod hardware_health;` and `pub use hardware_health::HardwareHealthPanel;` to `src/ui/mod.rs`.

- [ ] Verify:

```bash
cargo build
```

---

### Task 13: IqDiagnosticsPanel

**Files:**
- Create: `src/ui/iq_diagnostics.rs`

- [ ] Create `src/ui/iq_diagnostics.rs`:

```rust
use ratatui::{
    Frame,
    layout::{Rect, Layout, Direction, Constraint},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    text::Span,
};
use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

pub struct IqDiagnosticsPanel;

fn offset_color(val: f32) -> Color {
    let abs = val.abs();
    if abs > 0.02 { Color::Red }
    else if abs > 0.005 { Color::Yellow }
    else { Color::Green }
}

fn imbalance_color(db: f32) -> Color {
    let abs = db.abs();
    if abs > 3.0 { Color::Red }
    else if abs > 1.0 { Color::Yellow }
    else { Color::Green }
}

impl Panel for IqDiagnosticsPanel {
    fn name(&self) -> &'static str { "iq_diagnostics" }
    fn min_size(&self) -> (u16, u16) { (30, 6) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics) {
        let block = Block::default()
            .title(" IQ Diagnostics ")
            .borders(Borders::ALL);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(inner);

        f.render_widget(
            Paragraph::new(Span::styled(
                format!("DC offset  I: {:+.4}  Q: {:+.4}", state.dc_offset_i, state.dc_offset_q),
                Style::default().fg(offset_color(state.dc_offset_i.abs().max(state.dc_offset_q.abs()))),
            )),
            rows[0],
        );

        f.render_widget(
            Paragraph::new(Span::styled(
                format!("IQ imbalance: {:+.2} dB", state.iq_imbalance_db),
                Style::default().fg(imbalance_color(state.iq_imbalance_db)),
            )),
            rows[1],
        );

        let balance_hint = if state.iq_imbalance_db.abs() < 1.0 { "OK" }
            else if state.iq_imbalance_db > 0.0 { "I channel stronger" }
            else { "Q channel stronger" };
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("  → {}", balance_hint),
                Style::default().fg(Color::DarkGray),
            )),
            rows[2],
        );
    }
}
```

- [ ] Add `pub mod iq_diagnostics;` and `pub use iq_diagnostics::IqDiagnosticsPanel;` to `src/ui/mod.rs`.

- [ ] Verify:

```bash
cargo build
```

---

### Task 14: SystemResourcesPanel

**Files:**
- Create: `src/ui/system_resources.rs`

- [ ] Create `src/ui/system_resources.rs`:

```rust
use ratatui::{
    Frame,
    layout::{Rect, Layout, Direction, Constraint},
    style::{Color, Style},
    widgets::{Block, Borders, Gauge, Paragraph, Sparkline},
    text::Span,
};
use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

pub struct SystemResourcesPanel;

impl Panel for SystemResourcesPanel {
    fn name(&self) -> &'static str { "system_resources" }
    fn min_size(&self) -> (u16, u16) { (30, 10) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics) {
        let block = Block::default()
            .title(" System Resources ")
            .borders(Borders::ALL);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // CPU gauge
                Constraint::Length(2), // RAM gauge
                Constraint::Length(1), // USB throughput label
                Constraint::Min(0),    // USB sparkline
            ])
            .split(inner);

        let cpu_pct = state.process_cpu_pct.clamp(0.0, 100.0) as u16;
        let cpu_color = if cpu_pct > 80 { Color::Red }
            else if cpu_pct > 50 { Color::Yellow }
            else { Color::Cyan };
        f.render_widget(
            Gauge::default()
                .label(format!("CPU {cpu_pct}%"))
                .ratio(cpu_pct as f64 / 100.0)
                .style(Style::default().fg(cpu_color)),
            rows[0],
        );

        let rss_mb = state.process_rss_mb;
        let rss_pct = (rss_mb as f64 / 512.0).min(1.0); // 512 MB reference
        f.render_widget(
            Gauge::default()
                .label(format!("RAM {rss_mb} MB"))
                .ratio(rss_pct)
                .style(Style::default().fg(Color::Magenta)),
            rows[1],
        );

        let throughput_mb = state.current_throughput_bps / 1_000_000;
        f.render_widget(
            Paragraph::new(Span::raw(format!("USB  {throughput_mb} MB/s"))),
            rows[2],
        );
        let sparkline_data: Vec<u64> = state.throughput_history.iter()
            .map(|v| v / 1024)
            .collect();
        f.render_widget(
            Sparkline::default()
                .data(&sparkline_data)
                .style(Style::default().fg(Color::Green)),
            rows[3],
        );
    }
}
```

- [ ] Add `pub mod system_resources;` and `pub use system_resources::SystemResourcesPanel;` to `src/ui/mod.rs`.

- [ ] Verify:

```bash
cargo build
```

---

### Task 15: Register new panels and add monitoring preset

**Files:**
- Modify: `src/app.rs`
- Modify: `src/config.rs`

- [ ] In `src/config.rs`, add the `monitoring` and `spectrum_ready` presets to `LayoutConfig::default_config()`:

```rust
let monitoring = PresetConfig {
    panels: vec![
        PanelSpec { name: "header".into(),           position: Top,    height: Some(3), width_pct: None },
        PanelSpec { name: "hardware_health".into(),  position: Left,   height: None,    width_pct: Some(50) },
        PanelSpec { name: "iq_diagnostics".into(),   position: Left,   height: None,    width_pct: Some(50) },
        PanelSpec { name: "telemetry".into(),        position: Right,  height: None,    width_pct: Some(50) },
        PanelSpec { name: "system_resources".into(), position: Right,  height: None,    width_pct: Some(50) },
        PanelSpec { name: "log".into(),              position: Bottom, height: Some(7), width_pct: None },
        PanelSpec { name: "footer".into(),           position: Bottom, height: Some(3), width_pct: None },
    ],
};
presets.insert("monitoring".into(), monitoring);
```

- [ ] Update `active_preset` in `default_config()` to `"monitoring"`:

```rust
Self {
    active_preset: "monitoring".into(),
    presets,
}
```

- [ ] In `App::new()` in `src/app.rs`, register the three new panels after the existing registrations:

```rust
registry.register(ui::HardwareHealthPanel);
registry.register(ui::IqDiagnosticsPanel);
registry.register(ui::SystemResourcesPanel);
```

- [ ] Add `2` key handler in `App::run()` for the monitoring preset:

```rust
KeyCode::Char('2') => { self.engine.set_preset("monitoring"); }
```

- [ ] Verify:

```bash
cargo build
```

- [ ] Run the app manually:
  - Default view is `monitoring` with all new panels visible
  - Press `p` to cycle to `minimal` — new panels disappear
  - Press `2` to jump back to `monitoring`
  - Hardware health panel shows green zeros when idle (no drops, no saturation)
  - Start RX with `Space` and verify panels update with live data

---

### Task 16: Update Roadmap.md

**Files:**
- Modify: `docs/Roadmap.md`

- [ ] In `docs/Roadmap.md`, update the Current Status table to reflect the renumbered phases:

```markdown
| Phase | Status |
|---|---|
| 1 — Device discovery & basic info | ✅ Done |
| 2 — Telemetry polling & USB throughput | ✅ Done |
| 3 — TUI dashboard (gauges, sparkline, log, shortcuts) | ✅ Done |
| 4 — Architecture refactor (modular layout) | ✅ Done |
| 5 — Interactive controls | 🔲 Next |
| 6 — Dashboard engine (panel system, presets, layout config) | 🔲 Planned |
| 7 — Hardware health panels (drop rate, ADC saturation, IQ diagnostics) | 🔲 Planned |
| 8 — FFT spectrum analyzer | 🔲 Planned |
| 9 — Waterfall display | 🔲 Planned |
| 10 — Configuration & persistence | 🔲 Planned |
| 11 — Multi-device support | 🔲 Planned |
| 12 — PortaPack / Mayhem integration | 🔲 Planned |
| 13 — Polish & production readiness | 🔲 Planned |
| 14 — Distribution & community | 🔲 Planned |
```

- [ ] Add `## Phase 6 — Dashboard Engine` and `## Phase 7 — Hardware Health Panels` sections to the Roadmap in the same format as the existing Phase 1–4 sections, with links to the spec:

```markdown
## Phase 6 — Dashboard Engine

**Goal:** Replace fixed TUI layout with a modular panel system. Every display element
is a named `Panel` trait implementation; a `LayoutEngine` reads the active preset and
dispatches rendering. Users control what they see via preset switching and config file.

- Design spec: [2026-05-27-dashboard-engine-design.md](superpowers/specs/2026-05-27-dashboard-engine-design.md)

### Key design decisions

- `Panel` trait: `name()`, `min_size()`, `render(f, area, &SdrMetrics)`
- `PanelRegistry`: `HashMap<&'static str, Box<dyn Panel>>`
- `LayoutEngine`: reads `LayoutConfig` from config, maps position slots to ratatui `Rect`s
- Presets: `minimal`, `monitoring` — switchable with `p` (cycle) or `1`/`2` (direct)

---

## Phase 7 — Hardware Health Panels

**Goal:** Make sample drops, ADC saturation, IQ quality, and system resource usage
visible in real time — the metrics that turn sdrtop from an SDR frontend into a
genuine resource monitor.

- Design spec: [2026-05-27-dashboard-engine-design.md](superpowers/specs/2026-05-27-dashboard-engine-design.md)

### New panels

| Panel | Metrics |
|---|---|
| `hardware_health` | Drop rate (current + sparkline + session total), ADC saturation% (current + sparkline + session peak), callback jitter |
| `iq_diagnostics` | DC offset I/Q, IQ imbalance in dB |
| `system_resources` | Process CPU%, RSS memory, USB throughput sparkline |

### Color thresholds

| Metric | Green | Yellow | Red |
|---|---|---|---|
| Drop rate | 0/s | 1–10/s | >10/s |
| ADC saturation | <1% | 1–5% | >5% |
| IQ imbalance | <1 dB | 1–3 dB | >3 dB |
| Callback jitter | <500 µs | 500–2000 µs | >2000 µs |

---
```

- [ ] Verify the Roadmap renders correctly on GitHub (check that all links are standard markdown, no `[[wiki-link]]` syntax remains).

---

## Final validation

- [ ] `cargo build --release` — zero errors, zero warnings
- [ ] `cargo test` — all tests pass
- [ ] `cargo clippy -- -D warnings` — zero findings
- [ ] Run the app with a real HackRF: all three new panels show live data when RX is active
- [ ] Preset switching works (`p`, `1`, `2`)
- [ ] `minimal` preset matches the pre-Phase-6 layout exactly
