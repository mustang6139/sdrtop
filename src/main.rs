mod app;
mod config;
mod theme;
pub use theme::Theme;
mod event;
mod hardware;
mod palette;
mod signal;
mod state;
mod tasks;
mod ui;

use anyhow::Result;
use app::App;
use clap::Parser;
use config::AppConfig;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "sdrtop", about = "HackRF One / PortaPack terminal monitor")]
struct Cli {
    /// Path to config file (default: ~/.config/sdrtop/config.toml)
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Center frequency in Hz, e.g. 433920000 (overrides config)
    #[arg(long, value_name = "HZ")]
    frequency: Option<u64>,

    /// LNA gain in dB, 0–40 step 8 (overrides config)
    #[arg(long)]
    lna: Option<u32>,

    /// VGA gain in dB, 0–62 step 2 (overrides config)
    #[arg(long)]
    vga: Option<u32>,

    /// Color theme (sdr, nord, dracula, gruvbox, catppuccin, solarized)
    #[arg(long, value_name = "THEME")]
    theme: Option<String>,
}

fn default_config_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join(".config/sdrtop/config.toml"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let config_path = cli.config.or_else(default_config_path);
    let mut app_cfg = config_path
        .as_deref()
        .map(AppConfig::load_or_default)
        .unwrap_or_default();

    if let Some(f) = cli.frequency { app_cfg.radio.frequency_hz = f; }
    if let Some(l) = cli.lna       { app_cfg.radio.lna_gain = l.min(40); }
    if let Some(v) = cli.vga       { app_cfg.radio.vga_gain = v.min(62); }
    if let Some(t) = cli.theme     { app_cfg.theme.base = t; }

    let theme = app_cfg.build_theme();

    let device_serials = match hardware::Device::list_serials() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let device_index = if device_serials.len() > 1 {
        match ui::device_selector::run(device_serials, &theme, &mut terminal) {
            Ok(Some(idx)) => idx,
            Ok(None) => {
                disable_raw_mode()?;
                execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
                terminal.show_cursor()?;
                return Ok(());
            }
            Err(e) => {
                disable_raw_mode()?;
                execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
                terminal.show_cursor()?;
                eprintln!("Device selection error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        device_serials[0].0
    };

    let mut app = match App::new(app_cfg, config_path, device_index) {
        Ok(a) => a,
        Err(e) => {
            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
            terminal.show_cursor()?;
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let result = app.run(&mut terminal);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Application error: {:?}", err);
    }

    Ok(())
}
