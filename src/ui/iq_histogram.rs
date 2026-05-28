use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

pub struct IqHistogramPanel;

impl Panel for IqHistogramPanel {
    fn name(&self) -> &'static str { "iq_histogram" }
    fn min_size(&self) -> (u16, u16) { (36, 6) }

    fn render(&self, f: &mut Frame, area: ratatui::layout::Rect, state: &SdrMetrics) {
        let block = Block::default()
            .title(" IQ Amplitude Distribution ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        let inner = block.inner(area);
        f.render_widget(block, area);

        if inner.height < 3 || inner.width < 4 { return; }

        // Bottom row = status label, rest = bar chart
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);
        let chart_area = layout[0];
        let label_area = layout[1];

        let hist = &state.iq_amplitude_hist;
        let total: u64 = hist.iter().sum();
        let max_count = hist.iter().copied().max().unwrap_or(1).max(1);
        let bar_height = chart_area.height as usize;
        let n_bins = 32usize.min(chart_area.width as usize);

        // Build rows of the bar chart (top = high count, bottom = low count)
        // Use log scale: fill = log2(count+1) / log2(max+1) * bar_height
        let log_max = ((max_count + 1) as f64).log2();

        let mut rows: Vec<String> = (0..bar_height).map(|_| String::new()).collect();
        for &count in hist.iter().take(n_bins) {
            let fill_frac = if log_max > 0.0 {
                ((count + 1) as f64).log2() / log_max
            } else {
                0.0
            };
            let fill = (fill_frac * bar_height as f64).round() as usize;
            let fill = fill.min(bar_height);

            for (row, row_str) in rows.iter_mut().enumerate().take(bar_height) {
                let row_from_bottom = bar_height - 1 - row;
                row_str.push(if row_from_bottom < fill { '█' } else { ' ' });
            }
        }

        // Render chart rows — color by the dominant bin range in each column
        // For simplicity render whole chart with a mixed color approach:
        // split chart area into 3 horizontal segments: low/mid/high amplitude
        let low_cols  = (n_bins * 8 / 32).min(chart_area.width as usize);
        let high_cols = (n_bins * 4 / 32).min(chart_area.width as usize);
        let mid_cols  = n_bins.saturating_sub(low_cols + high_cols);

        // Build colored column groups — split by character index, not byte index,
        // because '█' is 3 bytes in UTF-8 and byte-slicing would panic mid-char.
        let low_rows:  Vec<String> = rows.iter()
            .map(|r| r.chars().take(low_cols).collect())
            .collect();
        let mid_rows:  Vec<String> = rows.iter()
            .map(|r| r.chars().skip(low_cols).take(mid_cols).collect())
            .collect();
        let high_rows: Vec<String> = rows.iter()
            .map(|r| r.chars().skip(low_cols + mid_cols).collect())
            .collect();

        // Render as 3 vertical strips with different fg colors
        let h_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(low_cols as u16),
                Constraint::Length(mid_cols as u16),
                Constraint::Min(0),
            ])
            .split(chart_area);

        f.render_widget(
            Paragraph::new(low_rows.join("\n")).style(Style::default().fg(Color::DarkGray)),
            h_layout[0],
        );
        f.render_widget(
            Paragraph::new(mid_rows.join("\n")).style(Style::default().fg(Color::Green)),
            h_layout[1],
        );
        f.render_widget(
            Paragraph::new(high_rows.join("\n")).style(Style::default().fg(Color::Red)),
            h_layout[2],
        );

        // Status label
        let high_count: u64 = hist[28..32].iter().sum();
        let low_count:  u64 = hist[..8].iter().sum();
        let label = if total == 0 {
            Span::styled("No samples yet", Style::default().fg(Color::DarkGray))
        } else if high_count > total / 10 {
            Span::styled("▲ clipping risk", Style::default().fg(Color::Red))
        } else if total > 0 && low_count > total * 9 / 10 {
            Span::styled("▼ weak signal — ADC under-utilised", Style::default().fg(Color::Yellow))
        } else {
            Span::styled("Dynamic range OK", Style::default().fg(Color::Green))
        };
        f.render_widget(Paragraph::new(label), label_area);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn histogram_row_split_does_not_panic_on_block_chars() {
        // '█' is 3 bytes in UTF-8; byte-slicing at column index 8 would panic
        // mid-character. chars().take/skip must be used instead.
        let row: String = (0..32).map(|_| '█').collect();
        let low_cols = 8usize;
        let mid_cols = 16usize;

        let low:  String = row.chars().take(low_cols).collect();
        let mid:  String = row.chars().skip(low_cols).take(mid_cols).collect();
        let high: String = row.chars().skip(low_cols + mid_cols).collect();

        assert_eq!(low.chars().count(),  8);
        assert_eq!(mid.chars().count(), 16);
        assert_eq!(high.chars().count(), 8);
    }
}
