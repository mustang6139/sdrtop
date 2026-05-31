use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone)]
#[allow(dead_code)]
pub struct FftFrame {
    pub bins_dbfs:        Arc<Vec<f32>>,
    pub peak_hold:        Arc<Vec<f32>>,
    pub noise_floor:      f32,
    pub center_freq_hz:   u64,
    pub sample_rate:      f64,
    pub timestamp:        Instant,
    pub snr_db:           f32,
    pub channel_power_dbfs: f32,
    pub occupied_bw_hz:   u64,
}

#[derive(Clone)]
pub struct WaterfallBuffer {
    /// Each row: (push timestamp, averaged bins). Newest row first.
    pub rows:       VecDeque<(Instant, Arc<Vec<f32>>)>,
    pub max_rows:   usize,
    pub paused:     bool,
    pub row_stride: usize,
    acc_bins:  Vec<f32>,
    acc_count: usize,
}

impl WaterfallBuffer {
    pub fn new(max_rows: usize) -> Self {
        Self {
            rows: VecDeque::new(),
            max_rows,
            paused: false,
            row_stride: 1,
            acc_bins: Vec::new(),
            acc_count: 0,
        }
    }

    pub fn push(&mut self, bins: Arc<Vec<f32>>) {
        if self.paused || self.max_rows == 0 { return; }

        if self.acc_count == 0 || self.acc_bins.len() != bins.len() {
            self.acc_bins = (*bins).clone();
        } else {
            for (a, &b) in self.acc_bins.iter_mut().zip(bins.iter()) {
                *a += b;
            }
        }
        self.acc_count += 1;

        if self.acc_count >= self.row_stride {
            let inv = 1.0 / self.acc_count as f32;
            for a in self.acc_bins.iter_mut() { *a *= inv; }
            let averaged = Arc::new(std::mem::take(&mut self.acc_bins));
            if self.rows.len() >= self.max_rows { self.rows.pop_back(); }
            self.rows.push_front((Instant::now(), averaged));
            self.acc_count = 0;
        }
    }

    pub fn set_row_stride(&mut self, stride: usize) {
        self.row_stride = stride.max(1);
        self.acc_bins.clear();
        self.acc_count = 0;
    }
}

#[derive(Clone)]
pub struct WaterfallState {
    pub db_min:        f32,
    pub scroll_offset: usize,
    pub cursor_freq:   Option<u64>,
    pub buffer:        WaterfallBuffer,
    pub last_fft:      Option<FftFrame>,
}

impl WaterfallState {
    pub fn new(max_rows: usize) -> Self {
        Self {
            db_min:        -120.0,
            scroll_offset: 0,
            cursor_freq:   None,
            buffer:        WaterfallBuffer::new(max_rows),
            last_fft:      None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_adds_newest_row_first() {
        let mut buf = WaterfallBuffer::new(4);
        buf.push(Arc::new(vec![1.0, 2.0]));
        buf.push(Arc::new(vec![3.0, 4.0]));
        assert_eq!(*buf.rows[0].1, vec![3.0, 4.0], "newest row should be at index 0");
        assert_eq!(*buf.rows[1].1, vec![1.0, 2.0]);
    }

    #[test]
    fn push_respects_max_rows() {
        let mut buf = WaterfallBuffer::new(3);
        for i in 0..5u32 {
            buf.push(Arc::new(vec![i as f32]));
        }
        assert_eq!(buf.rows.len(), 3, "should not exceed max_rows");
    }

    #[test]
    fn paused_ignores_push() {
        let mut buf = WaterfallBuffer::new(4);
        buf.paused = true;
        buf.push(Arc::new(vec![1.0, 2.0]));
        assert!(buf.rows.is_empty(), "paused buffer should not accept new rows");
    }

    #[test]
    fn stride_averages_frames() {
        let mut buf = WaterfallBuffer::new(4);
        buf.set_row_stride(2);
        buf.push(Arc::new(vec![10.0, 20.0]));
        assert!(buf.rows.is_empty(), "first frame should not push yet");
        buf.push(Arc::new(vec![20.0, 40.0]));
        assert_eq!(buf.rows.len(), 1, "second frame should push averaged row");
        assert_eq!(*buf.rows[0].1, vec![15.0, 30.0]);
    }

    #[test]
    fn stride_reset_clears_accumulator() {
        let mut buf = WaterfallBuffer::new(4);
        buf.set_row_stride(3);
        buf.push(Arc::new(vec![10.0]));
        buf.set_row_stride(1);
        buf.push(Arc::new(vec![5.0]));
        assert_eq!(buf.rows.len(), 1);
        assert_eq!(*buf.rows[0].1, vec![5.0]);
    }
}
