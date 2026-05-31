#[derive(Clone)]
pub struct SystemState {
    pub board_name:      String,
    #[allow(dead_code)]
    pub serial:          String,
    pub fw_version:      String,
    pub board_rev:       u8,
    pub usb_api_version: u16,
    pub cpld_ok:         Option<bool>,
    pub process_cpu_pct: f32,
    pub process_rss_mb:  u64,
}
