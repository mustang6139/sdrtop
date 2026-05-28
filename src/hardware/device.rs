use libc::{c_int, c_void};
use std::ffi::CStr;
use std::sync::{Arc, Mutex};

use crate::state::SdrMetrics;
use super::ffi::*;

pub struct RxContext {
    pub metrics: Arc<Mutex<SdrMetrics>>,
    pub sample_tx: crossbeam_channel::Sender<Vec<u8>>,
}

pub struct Device(*mut c_void);

// Safety: libhackrf is thread-safe for status polling and streaming control
unsafe impl Send for Device {}
unsafe impl Sync for Device {}

pub extern "C" fn rx_callback(transfer: *mut hackrf_transfer) -> c_int {
    unsafe {
        let t = &*transfer;
        let ctx_ptr = t.rx_ctx as *const RxContext;
        if ctx_ptr.is_null() { return 0; }
        let ctx = &*ctx_ptr;

        let buf = std::slice::from_raw_parts(
            t.buffer as *const u8,
            t.valid_length as usize,
        );

        // Health accumulation — lock held briefly, no allocation inside
        {
            let Ok(mut m) = ctx.metrics.lock() else { return 0; };

            m.bytes_since_last_poll += t.valid_length as u64;

            if t.valid_length < t.buffer_length {
                let dropped_pairs = ((t.buffer_length - t.valid_length) / 2) as u64;
                m.acc_drops += dropped_pairs;
                m.total_drops_session += dropped_pairs;
            }

            let mut saturated: u64 = 0;
            let mut i_sum: i64 = 0;
            let mut q_sum: i64 = 0;
            let mut i_sq: i64 = 0;
            let mut q_sq: i64 = 0;

            for chunk in buf.chunks_exact(2) {
                let i_byte = chunk[0] as i8;
                let q_byte = chunk[1] as i8;
                let i = i_byte as i64;
                let q = q_byte as i64;
                i_sum += i;
                q_sum += q;
                i_sq  += i * i;
                q_sq  += q * q;
                if chunk[0] == 0x80 || chunk[0] == 0x7F { saturated += 1; }
                if chunk[1] == 0x80 || chunk[1] == 0x7F { saturated += 1; }
                // IQ amplitude histogram: Chebyshev distance, 32 bins of width 4.
                // Clamp to bin 31: i8::MIN.unsigned_abs() == 128, which would overflow
                // a 32-element array without the min(31).
                let amp = i_byte.unsigned_abs().max(q_byte.unsigned_abs());
                m.acc_iq_hist[((amp / 4) as usize).min(31)] += 1;
            }

            let pairs = (buf.len() / 2) as u64;
            m.acc_saturated    += saturated;
            m.acc_i_sum        += i_sum;
            m.acc_q_sum        += q_sum;
            m.acc_i_sq_sum     += i_sq;
            m.acc_q_sq_sum     += q_sq;
            m.acc_sample_count += pairs;

            let now_us = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_micros() as u64)
                .unwrap_or(0);
            if let Some(last_us) = m.acc_last_callback_us {
                let gap = now_us.saturating_sub(last_us);
                m.acc_jitter_sum_us += gap;
                m.acc_jitter_count  += 1;
            }
            m.acc_last_callback_us = Some(now_us);
        }
        // Lock released — allocate outside the critical section
        ctx.sample_tx.try_send(buf.to_vec()).ok();
    }
    0
}

#[cfg(test)]
mod tests {
    #[test]
    fn saturation_byte_detection() {
        let at_max: u8 = 0x7F;
        let at_min: u8 = 0x80;
        let normal: u8 = 0x40;
        assert!(at_max == 0x7F || at_max == 0x80);
        assert!(at_min == 0x7F || at_min == 0x80);
        assert!(normal != 0x7F && normal != 0x80);
    }

    #[test]
    fn drop_detection_arithmetic() {
        let buffer_length: i32 = 262144;
        let valid_length: i32  = 262144 - 128;
        let dropped_pairs = ((buffer_length - valid_length) / 2) as u64;
        assert_eq!(dropped_pairs, 64);
    }

    #[test]
    fn board_rev_name_known_revisions() {
        assert_eq!(super::Device::board_rev_name(9),    "HackRF One r9");
        assert_eq!(super::Device::board_rev_name(0xFF), "Unrecognized");
        assert_eq!(super::Device::board_rev_name(0xFE), "Undetected");
        assert_eq!(super::Device::board_rev_name(0),    "HackRF One (old)");
    }

    #[test]
    fn bb_filter_bw_exact_match() {
        assert_eq!(super::compute_bb_filter_bw(10_000_000.0), 10_000_000);
        assert_eq!(super::compute_bb_filter_bw(20_000_000.0), 20_000_000);
        assert_eq!(super::compute_bb_filter_bw(28_000_000.0), 28_000_000);
    }

    #[test]
    fn bb_filter_bw_rounds_to_nearest() {
        // 11.5 MHz → nearest is 12 MHz (distance 0.5 M) vs 10 MHz (distance 1.5 M)
        assert_eq!(super::compute_bb_filter_bw(11_500_000.0), 12_000_000);
        // 4 MHz → nearest is 3.5 MHz (distance 0.5 M) vs 5 MHz (distance 1 M)
        assert_eq!(super::compute_bb_filter_bw(4_000_000.0), 3_500_000);
    }

    #[test]
    fn bb_filter_bw_clamps_to_valid_range() {
        assert_eq!(super::compute_bb_filter_bw(500_000.0),    1_750_000);
        assert_eq!(super::compute_bb_filter_bw(30_000_000.0), 28_000_000);
    }

    #[test]
    fn histogram_bin_for_max_amplitude() {
        let amp: u8 = 127;
        let bin = ((amp / 4) as usize).min(31);
        assert_eq!(bin, 31);
    }

    #[test]
    fn histogram_bin_for_zero_amplitude() {
        let amp: u8 = 0;
        let bin = ((amp / 4) as usize).min(31);
        assert_eq!(bin, 0);
    }

    #[test]
    fn histogram_bin_i8_min_does_not_overflow() {
        // i8::MIN.unsigned_abs() == 128, which without clamping would index [32] on
        // a [u64; 32] array and panic inside the C rx_callback (no Drop = HackRF stuck).
        let i_byte: i8 = i8::MIN;
        let q_byte: i8 = 0;
        let amp = i_byte.unsigned_abs().max(q_byte.unsigned_abs());
        assert_eq!(amp, 128);
        let bin = ((amp / 4) as usize).min(31);
        assert_eq!(bin, 31);  // clamped to last bin, not 32
    }
}

#[allow(dead_code)]
impl Device {
    pub fn open() -> anyhow::Result<Self> {
        unsafe {
            let init_res = hackrf_init();
            if init_res != 0 {
                let err = CStr::from_ptr(hackrf_error_name(init_res)).to_string_lossy();
                anyhow::bail!("Failed to initialize libhackrf: {}", err);
            }

            let list_ptr = hackrf_device_list();
            if list_ptr.is_null() {
                hackrf_exit();
                anyhow::bail!("Failed to retrieve HackRF device list.");
            }

            let list = &*list_ptr;
            let count = list.devicecount as usize;

            if count == 0 {
                hackrf_device_list_free(list_ptr);
                hackrf_exit();
                anyhow::bail!(
                    "No HackRF device found. Please connect your device and try again."
                );
            }

            let selected_index = if count == 1 {
                0
            } else {
                println!("Multiple HackRF devices found:");
                let mut valid_count = 0;
                if !list.serial_numbers.is_null() {
                    for i in 0..count {
                        let serial_ptr = *list.serial_numbers.add(i);
                        if !serial_ptr.is_null() {
                            let serial = CStr::from_ptr(serial_ptr).to_string_lossy();
                            println!("[{}] Serial: {}", i, serial);
                            valid_count += 1;
                        }
                    }
                }

                if valid_count == 0 {
                    hackrf_device_list_free(list_ptr);
                    hackrf_exit();
                    anyhow::bail!("No valid serial numbers found for connected devices.");
                }
                print!("Select device index [0-{}]: ", count - 1);
                use std::io::{self, Write};
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let idx = input.trim().parse::<usize>().unwrap_or(usize::MAX);
                if idx >= count {
                    hackrf_device_list_free(list_ptr);
                    hackrf_exit();
                    anyhow::bail!("Invalid device index selected.");
                }
                idx
            };

            let mut ptr = std::ptr::null_mut();
            let res = hackrf_device_list_open(list_ptr, selected_index as c_int, &mut ptr);
            hackrf_device_list_free(list_ptr);

            if res != 0 {
                let err = CStr::from_ptr(hackrf_error_name(res)).to_string_lossy();
                hackrf_exit();
                anyhow::bail!("Failed to open HackRF device: {} (code {})", err, res);
            }

            Ok(Device(ptr))
        }
    }

    pub fn version(&self) -> anyhow::Result<String> {
        let mut buf = [0i8; 64];
        unsafe {
            if hackrf_version_string_read(self.0, buf.as_mut_ptr(), 63) != 0 {
                anyhow::bail!("Failed to read firmware version");
            }
            Ok(CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned())
        }
    }

    pub fn is_streaming(&self) -> bool {
        unsafe { hackrf_is_streaming(self.0) == 1 }
    }

    pub fn start_rx(
        &self,
        callback: HackrfTransferCallback,
        user_param: *mut c_void,
    ) -> anyhow::Result<()> {
        unsafe {
            if hackrf_start_rx(self.0, callback, user_param) != 0 {
                anyhow::bail!("Failed to start RX streaming");
            }
        }
        Ok(())
    }

    pub fn stop_rx(&self) -> anyhow::Result<()> {
        unsafe {
            if hackrf_stop_rx(self.0) != 0 {
                anyhow::bail!("Failed to stop RX streaming");
            }
        }
        Ok(())
    }

    pub fn set_lna_gain(&self, gain: u32) -> anyhow::Result<()> {
        unsafe {
            if hackrf_set_lna_gain(self.0, gain) != 0 {
                anyhow::bail!("Failed to set LNA gain");
            }
        }
        Ok(())
    }

    pub fn set_vga_gain(&self, gain: u32) -> anyhow::Result<()> {
        unsafe {
            if hackrf_set_vga_gain(self.0, gain) != 0 {
                anyhow::bail!("Failed to set VGA gain");
            }
        }
        Ok(())
    }

    pub fn set_sample_rate(&self, sample_rate: f64) -> anyhow::Result<()> {
        unsafe {
            if hackrf_set_sample_rate(self.0, sample_rate) != 0 {
                anyhow::bail!("Failed to set sample rate");
            }
        }
        Ok(())
    }

    pub fn set_frequency(&self, freq_hz: u64) -> anyhow::Result<()> {
        unsafe {
            if hackrf_set_freq(self.0, freq_hz) != 0 {
                anyhow::bail!("Failed to set frequency");
            }
        }
        Ok(())
    }

    pub fn set_amp_enable(&self, enable: bool) -> anyhow::Result<()> {
        unsafe {
            if hackrf_set_amp_enable(self.0, enable as u8) != 0 {
                anyhow::bail!("Failed to set AMP enable");
            }
        }
        Ok(())
    }

    pub fn board_id(&self) -> anyhow::Result<u8> {
        let mut id = 0u8;
        unsafe {
            if hackrf_board_id_read(self.0, &mut id) != 0 {
                anyhow::bail!("Failed to read board ID");
            }
            Ok(id)
        }
    }

    pub fn board_name(&self, id: u8) -> String {
        unsafe {
            let ptr = hackrf_board_id_name(id);
            if ptr.is_null() {
                return "Unknown".to_string();
            }
            CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    }

    pub fn serial_number(&self) -> anyhow::Result<String> {
        let mut data = ReadPartidSerialno {
            part_id: [0; 2],
            serial_no: [0; 4],
        };
        unsafe {
            if hackrf_board_partid_serialno_read(self.0, &mut data) != 0 {
                anyhow::bail!("Failed to read serial number");
            }
            let s = data.serial_no;
            Ok(format!(
                "{:08x}{:08x}{:08x}{:08x}",
                s[0], s[1], s[2], s[3]
            ))
        }
    }
    pub fn board_rev(&self) -> anyhow::Result<u8> {
        let mut rev = 0u8;
        unsafe {
            if hackrf_board_rev_read(self.0, &mut rev) != 0 {
                anyhow::bail!("Failed to read board revision");
            }
        }
        Ok(rev)
    }

    pub fn board_rev_name(rev: u8) -> &'static str {
        match rev {
            0    => "HackRF One (old)",
            6    => "HackRF One r6",
            7    => "HackRF One r7",
            8    => "HackRF One r8",
            9    => "HackRF One r9",
            10   => "HackRF One r10",
            0xFE => "Undetected",
            0xFF => "Unrecognized",
            _    => "Unknown",
        }
    }

    pub fn usb_api_version(&self) -> anyhow::Result<u16> {
        let mut ver = 0u16;
        unsafe {
            if hackrf_usb_api_version_read(self.0, &mut ver) != 0 {
                anyhow::bail!("Failed to read USB API version");
            }
        }
        Ok(ver)
    }
}

pub fn compute_bb_filter_bw(sample_rate_hz: f64) -> u32 {
    const STEPS: &[u32] = &[
        1_750_000, 2_500_000, 3_500_000, 5_000_000, 5_500_000, 6_000_000,
        7_000_000, 8_000_000, 9_000_000, 10_000_000, 12_000_000, 14_000_000,
        15_000_000, 20_000_000, 24_000_000, 28_000_000,
    ];
    let target = sample_rate_hz as u32;
    STEPS.iter()
        .copied()
        .min_by_key(|&bw| (bw as i64 - target as i64).unsigned_abs())
        .unwrap_or(10_000_000)
}

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
