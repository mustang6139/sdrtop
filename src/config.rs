use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::state::{SpectrumMarker, DEFAULT_FREQUENCY, DEFAULT_LNA_GAIN, DEFAULT_SAMPLE_RATE, DEFAULT_VGA_GAIN};

fn default_frequency_hz() -> u64     { DEFAULT_FREQUENCY }
fn default_sample_rate()  -> f64     { DEFAULT_SAMPLE_RATE }
fn default_lna_gain()     -> u32     { DEFAULT_LNA_GAIN }
fn default_vga_gain()     -> u32     { DEFAULT_VGA_GAIN }
fn default_active_preset() -> String { "spectrum_waterfall".into() }
fn default_waterfall_max_rows() -> usize { 64 }

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct RadioConfig {
    #[serde(default = "default_frequency_hz")]
    pub frequency_hz: u64,
    #[serde(default = "default_sample_rate")]
    pub sample_rate: f64,
    #[serde(default = "default_lna_gain")]
    pub lna_gain: u32,
    #[serde(default = "default_vga_gain")]
    pub vga_gain: u32,
    #[serde(default)]
    pub amp_enabled: bool,
}

impl Default for RadioConfig {
    fn default() -> Self {
        Self {
            frequency_hz: DEFAULT_FREQUENCY,
            sample_rate:  DEFAULT_SAMPLE_RATE,
            lna_gain:     DEFAULT_LNA_GAIN,
            vga_gain:     DEFAULT_VGA_GAIN,
            amp_enabled:  false,
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DisplayConfig {
    #[serde(default = "default_active_preset")]
    pub active_preset: String,
    #[serde(default = "default_waterfall_max_rows")]
    pub waterfall_max_rows: usize,
    #[serde(default)]
    pub spectrum_markers: Vec<SpectrumMarker>,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            active_preset:      "spectrum_waterfall".into(),
            waterfall_max_rows: 64,
            spectrum_markers:   vec![],
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ThemeConfig {
    #[serde(default = "ThemeConfig::default_base")]
    pub base: String,
    // Per-field overrides. "#rrggbb" strings. None = use theme default.
    pub border_accent:  Option<String>,
    pub border_dim:     Option<String>,
    pub border_default: Option<String>,
    pub border_focused: Option<String>,
    pub label:          Option<String>,
    pub value:          Option<String>,
    pub value_hi:       Option<String>,
    pub status_ok:      Option<String>,
    pub status_warn:    Option<String>,
    pub status_crit:    Option<String>,
    pub peak_hold:      Option<String>,
    pub noise_floor:    Option<String>,
    pub stale:          Option<String>,
    pub observer:       Option<String>,
}

impl ThemeConfig {
    fn default_base() -> String { "sdr".into() }
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub radio: RadioConfig,
    #[serde(default)]
    pub display: DisplayConfig,
    #[serde(default)]
    pub theme: ThemeConfig,
}

impl AppConfig {
    pub fn load_or_default(path: &Path) -> Self {
        let content = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => return Self::default(),
        };
        match toml::from_str(&content) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("Warning: failed to parse {}: {e}. Using defaults.", path.display());
                Self::default()
            }
        }
    }

    pub fn build_theme(&self) -> crate::Theme {
        let mut t = crate::Theme::by_name(&self.theme.base);
        let tc = &self.theme;
        macro_rules! apply {
            ($field:ident) => {
                if let Some(ref s) = tc.$field {
                    if let Some(c) = crate::Theme::parse_hex(s) {
                        t.$field = c;
                    }
                }
            };
        }
        apply!(border_accent);
        apply!(border_dim);
        apply!(border_default);
        apply!(border_focused);
        apply!(label);
        apply!(value);
        apply!(value_hi);
        apply!(status_ok);
        apply!(status_warn);
        apply!(status_crit);
        apply!(peak_hold);
        apply!(noise_floor);
        apply!(stale);
        apply!(observer);
        t
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, content)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

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
    /// Height in terminal rows — used for Top and Bottom panels.
    #[serde(default)]
    pub height: Option<u16>,
    /// Width as a percentage of the body zone — used for Left and Right panels.
    /// All panels in the same column carry the same value; the LayoutEngine
    /// reads only the first panel's value to determine column width.
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
        let spectrum = PresetConfig {
            panels: vec![
                PanelSpec { name: "header".into(),   position: Top,    height: Some(5), width_pct: None },
                PanelSpec { name: "spectrum".into(),  position: Body,   height: None,    width_pct: None },
                PanelSpec { name: "log".into(),       position: Bottom, height: Some(5), width_pct: None },
                PanelSpec { name: "footer".into(),    position: Bottom, height: Some(3), width_pct: None },
            ],
        };
        let waterfall = PresetConfig {
            panels: vec![
                PanelSpec { name: "header".into(),   position: Top,    height: Some(5), width_pct: None },
                PanelSpec { name: "waterfall".into(), position: Body,   height: None,    width_pct: None },
                PanelSpec { name: "log".into(),       position: Bottom, height: Some(5), width_pct: None },
                PanelSpec { name: "footer".into(),    position: Bottom, height: Some(3), width_pct: None },
            ],
        };
        let spectrum_waterfall = PresetConfig {
            panels: vec![
                PanelSpec { name: "header".into(),    position: Top,    height: Some(5), width_pct: None },
                PanelSpec { name: "spectrum".into(),  position: Body,   height: None,    width_pct: None },
                PanelSpec { name: "waterfall".into(), position: Body,   height: None,    width_pct: None },
                PanelSpec { name: "log".into(),       position: Bottom, height: Some(5), width_pct: None },
                PanelSpec { name: "footer".into(),    position: Bottom, height: Some(3), width_pct: None },
            ],
        };
        let lab = PresetConfig {
            panels: vec![
                PanelSpec { name: "header".into(),           position: Top,    height: Some(5), width_pct: None     },
                PanelSpec { name: "rf_chain".into(),         position: Left,   height: None,    width_pct: Some(50) },
                PanelSpec { name: "iq_diagnostics".into(),   position: Left,   height: None,    width_pct: Some(50) },
                PanelSpec { name: "signal_metrics".into(),   position: Right,  height: None,    width_pct: Some(50) },
                PanelSpec { name: "iq_histogram".into(),     position: Right,  height: None,    width_pct: Some(50) },
                PanelSpec { name: "hardware_health".into(),  position: Right,  height: None,    width_pct: Some(50) },
                PanelSpec { name: "system_resources".into(), position: Right,  height: None,    width_pct: Some(50) },
                PanelSpec { name: "log".into(),              position: Bottom, height: Some(5), width_pct: None     },
                PanelSpec { name: "footer".into(),           position: Bottom, height: Some(3), width_pct: None     },
            ],
        };
        let observer = PresetConfig {
            panels: vec![
                PanelSpec { name: "header".into(),           position: Top,    height: Some(5), width_pct: None     },
                PanelSpec { name: "observer".into(),         position: Left,   height: None,    width_pct: Some(60) },
                PanelSpec { name: "system_resources".into(), position: Right,  height: None,    width_pct: Some(40) },
                PanelSpec { name: "log".into(),              position: Bottom, height: Some(5), width_pct: None     },
                PanelSpec { name: "footer".into(),           position: Bottom, height: Some(3), width_pct: None     },
            ],
        };
        let main = PresetConfig {
            panels: vec![
                PanelSpec { name: "header".into(),       position: Top,    height: Some(5), width_pct: None },
                PanelSpec { name: "spectrum".into(),      position: Body,   height: None,    width_pct: None },
                PanelSpec { name: "waterfall".into(),     position: Body,   height: None,    width_pct: None },
                PanelSpec { name: "signal_strip".into(),  position: Bottom, height: Some(3), width_pct: None },
                PanelSpec { name: "usb_sr".into(),        position: Bottom, height: Some(5), width_pct: None },
                PanelSpec { name: "footer".into(),        position: Bottom, height: Some(3), width_pct: None },
            ],
        };
        let mut presets = HashMap::new();
        presets.insert("spectrum".into(), spectrum);
        presets.insert("waterfall".into(), waterfall);
        presets.insert("spectrum_waterfall".into(), spectrum_waterfall);
        presets.insert("lab".into(), lab);
        presets.insert("observer".into(), observer);
        presets.insert("main".into(), main);
        Self { active_preset: "spectrum_waterfall".into(), presets }
    }

    pub fn active_panels(&self) -> &[PanelSpec] {
        self.presets
            .get(&self.active_preset)
            .map(|p| p.panels.as_slice())
            .unwrap_or(&[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_minimal_preset() {
        let cfg = LayoutConfig::default_config();
        assert_eq!(cfg.active_preset, "spectrum_waterfall");
        assert!(!cfg.active_panels().is_empty());
    }

    #[test]
    fn active_panels_returns_correct_names() {
        let cfg = LayoutConfig::default_config();
        let names: Vec<&str> = cfg.active_panels().iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"header"));
        assert!(names.contains(&"footer"));
        assert!(names.contains(&"spectrum"));
        assert!(names.contains(&"waterfall"));
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

    #[test]
    fn default_radio_config_frequency() {
        assert_eq!(RadioConfig::default().frequency_hz, 2_400_000_000);
        assert_eq!(RadioConfig::default().lna_gain, 16);
    }

    #[test]
    fn load_or_default_missing_file_returns_default() {
        let cfg = AppConfig::load_or_default(Path::new("/nonexistent/sdrtop/config.toml"));
        assert_eq!(cfg.radio.frequency_hz, RadioConfig::default().frequency_hz);
    }

    #[test]
    fn deserialize_partial_toml_fills_missing_with_defaults() {
        let toml_str = "[radio]\nfrequency_hz = 433_000_000\n";
        let cfg: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.radio.frequency_hz, 433_000_000);
        assert_eq!(cfg.display.active_preset, "spectrum_waterfall");
    }

    #[test]
    fn serialize_deserialize_round_trip() {
        let mut cfg = AppConfig::default();
        cfg.radio.lna_gain = 24;
        cfg.display.active_preset = "spectrum".into();
        let serialized = toml::to_string_pretty(&cfg).unwrap();
        let restored: AppConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(restored.radio.lna_gain, 24);
        assert_eq!(restored.display.active_preset, "spectrum");
    }

    #[test]
    fn build_theme_default_is_sdr() {
        let cfg = AppConfig::load_or_default(Path::new("/nonexistent/sdrtop/config.toml"));
        let t = cfg.build_theme();
        assert_eq!(t.name, "sdr");
    }

    #[test]
    fn build_theme_unknown_base_falls_back_to_sdr() {
        let toml = "[theme]\nbase = \"nonexistent\"\n";
        let cfg: AppConfig = toml::from_str(toml).unwrap();
        assert_eq!(cfg.build_theme().name, "sdr");
    }

    #[test]
    fn build_theme_override_applies_hex_color() {
        let toml = "[theme]\nbase = \"nord\"\nborder_accent = \"#ff0000\"\n";
        let cfg: AppConfig = toml::from_str(toml).unwrap();
        let t = cfg.build_theme();
        assert_eq!(t.border_accent, ratatui::style::Color::Rgb(255, 0, 0));
    }

    #[test]
    fn build_theme_invalid_hex_override_ignored() {
        let toml = "[theme]\nbase = \"nord\"\nborder_accent = \"notahex\"\n";
        let cfg: AppConfig = toml::from_str(toml).unwrap();
        let t = cfg.build_theme();
        assert_eq!(t.name, "nord");
    }
}
