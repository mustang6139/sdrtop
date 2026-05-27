use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossterm::event::KeyCode;
use ratatui::{backend::Backend, Terminal};

use crate::event::{AppEvent, EventStream};
use crate::hardware;
use crate::state::{
    SdrMetrics, DEFAULT_FREQUENCY, DEFAULT_LNA_GAIN, DEFAULT_SAMPLE_RATE, DEFAULT_VGA_GAIN,
    THROUGHPUT_HISTORY_LEN,
};
use crate::ui;

pub struct App {
    state: Arc<Mutex<SdrMetrics>>,
    // Kept alive to ensure Drop runs (closes device) when App is dropped
    #[allow(dead_code)]
    device: Arc<hardware::Device>,
    board_name: String,
    fw_version: String,
    serial: String,
    events: EventStream,
}

impl App {
    pub fn new() -> anyhow::Result<Self> {
        let device = Arc::new(hardware::Device::open()?);

        let board_id = device.board_id()?;
        let board_name = device.board_name(board_id);
        let fw_version = device.version()?;
        let serial = device.serial_number()?;

        let state = Arc::new(Mutex::new(SdrMetrics {
            frequency: DEFAULT_FREQUENCY,
            config_sample_rate: DEFAULT_SAMPLE_RATE,
            actual_sample_rate: 0,
            lna_gain: DEFAULT_LNA_GAIN,
            vga_gain: DEFAULT_VGA_GAIN,
            amp_enabled: false,
            rx_enabled: false,
            hw_streaming: false,
            bytes_since_last_poll: 0,
            last_poll_time: Instant::now(),
            current_throughput_bps: 0,
            throughput_history: VecDeque::with_capacity(THROUGHPUT_HISTORY_LEN),
            log: VecDeque::new(),
        }));

        {
            let mut m = state.lock().unwrap();
            m.push_log(format!("Connected: {} | Serial: {}", board_name, serial));
            m.push_log(format!("Firmware: {}", fw_version));
        }

        let state_bg = Arc::clone(&state);
        let device_bg = Arc::clone(&device);
        tokio::spawn(async move {
            // Tracks whether we have actually issued start_rx to the hardware
            let mut hw_rx_active = false;

            loop {
                let now = Instant::now();

                // Compute throughput from bytes accumulated by the RX callback
                let (bytes, elapsed_ms) = {
                    let mut m = state_bg.lock().unwrap();
                    let elapsed = now.duration_since(m.last_poll_time).as_millis();
                    let bytes = m.bytes_since_last_poll;
                    m.bytes_since_last_poll = 0;
                    m.last_poll_time = now;
                    (bytes, elapsed)
                };

                {
                    let mut m = state_bg.lock().unwrap();
                    m.hw_streaming = device_bg.is_streaming();
                    if elapsed_ms > 0 {
                        m.current_throughput_bps = (bytes * 1000) / elapsed_ms as u64;
                        // 2 bytes per IQ sample (8-bit I + 8-bit Q)
                        m.actual_sample_rate = (m.current_throughput_bps / 2) as u32;
                        // Record KB/s in history for sparkline
                        let throughput_kb = m.current_throughput_bps / 1024;
                        if m.throughput_history.len() >= THROUGHPUT_HISTORY_LEN {
                            m.throughput_history.pop_front();
                        }
                        m.throughput_history.push_back(throughput_kb);
                    }
                }

                // Manage RX streaming based on user's desired state
                let rx_enabled = state_bg.lock().unwrap().rx_enabled;
                if rx_enabled && !hw_rx_active {
                    let user_param = Arc::as_ptr(&state_bg) as *mut libc::c_void;
                    match device_bg.start_rx(hardware::device::rx_callback, user_param) {
                        Ok(()) => {
                            hw_rx_active = true;
                            state_bg.lock().unwrap().push_log("RX streaming started");
                        }
                        Err(e) => {
                            let msg = format!("Error starting RX: {}", e);
                            let mut m = state_bg.lock().unwrap();
                            m.rx_enabled = false;
                            m.push_log(msg);
                        }
                    }
                } else if !rx_enabled && hw_rx_active {
                    let result = device_bg.stop_rx();
                    hw_rx_active = false;
                    let mut m = state_bg.lock().unwrap();
                    match result {
                        Ok(()) => m.push_log("RX streaming stopped"),
                        Err(e) => m.push_log(format!("Error stopping RX: {}", e)),
                    }
                }

                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        });

        Ok(Self {
            state,
            device,
            board_name,
            fw_version,
            serial,
            events: EventStream::new(Duration::from_millis(100)),
        })
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        loop {
            let m = self.state.lock().unwrap().clone();
            terminal.draw(|f| {
                ui::draw(f, &m, &self.board_name, &self.fw_version, &self.serial)
            })?;

            match self.events.recv() {
                AppEvent::Key(key) => match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char(' ') => {
                        let mut m = self.state.lock().unwrap();
                        m.rx_enabled = !m.rx_enabled;
                    }
                    KeyCode::Char('r') => {
                        self.state.lock().unwrap().reset_to_defaults();
                    }
                    _ => {}
                },
                AppEvent::Tick => {}
            }
        }
    }
}
