use crate::framebuffer::Framebuffer;
#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// A Slit-Scan post-processing filter.
///
/// This effect simulates an analog slit-scan camera by capturing the image
/// line-by-line over time. It maintains a circular history buffer of recent frames
/// and constructs a composite output frame where different rows are sampled from
/// different points in time.
pub struct SlitScanFilter {
    history: Vec<Vec<u32>>,
    current_index: usize,
    width: usize,
    height: usize,
}

impl SlitScanFilter {
    /// Creates a new `SlitScanFilter`.
    ///
    /// # Panics
    ///
    /// Panics if `history_len` is 0.
    ///
    /// # Arguments
    /// * `width` - The width of the framebuffer.
    /// * `height` - The height of the framebuffer.
    /// * `history_len` - The number of frames to keep in history. A larger number
    ///                   creates a more pronounced time-stretching effect.
    #[must_use]
    pub fn new(width: usize, height: usize, history_len: usize) -> Self {
        assert!(history_len > 0, "history_len must be greater than 0");
        let mut history = Vec::with_capacity(history_len);
        for _ in 0..history_len {
            history.push(vec![0xFF00_0000; width * height]);
        }

        Self {
            history,
            current_index: 0,
            width,
            height,
        }
    }

    /// Applies the slit-scan effect to the given framebuffer.
    ///
    /// # Panics
    ///
    /// Panics if the dimensions of the framebuffer do not match the dimensions
    /// used to initialize the `SlitScanFilter`.
    pub fn apply(&mut self, fb: &mut Framebuffer) {
        assert_eq!(fb.width() as usize, self.width);
        assert_eq!(fb.height() as usize, self.height);

        let history_len = self.history.len();

        // Save current frame to the history buffer
        self.history[self.current_index].copy_from_slice(fb.as_slice());

        let current_index = self.current_index;
        let height = self.height;
        let width = self.width;
        let history = &self.history; // borrow for closure

        // We calculate the time delay (t) for each row.
        // y = 0 -> t = 0 (current frame)
        // y = height - 1 -> t = history_len - 1 (oldest frame)

        #[cfg(feature = "parallel")]
        {
            fb.as_mut_slice()
                .par_chunks_exact_mut(width)
                .enumerate()
                .for_each(|(y, row_out)| {
                    let t = (y * history_len) / height;
                    // Clamp t just in case
                    let t = t.min(history_len - 1);

                    let sample_index = (current_index + history_len - t) % history_len;
                    let row_start = y * width;
                    let row_end = row_start + width;

                    row_out.copy_from_slice(&history[sample_index][row_start..row_end]);
                });
        }

        #[cfg(not(feature = "parallel"))]
        {
            fb.as_mut_slice()
                .chunks_exact_mut(width)
                .enumerate()
                .for_each(|(y, row_out)| {
                    let t = (y * history_len) / height;
                    let t = t.min(history_len - 1);

                    let sample_index = (current_index + history_len - t) % history_len;
                    let row_start = y * width;
                    let row_end = row_start + width;

                    row_out.copy_from_slice(&history[sample_index][row_start..row_end]);
                });
        }

        // Advance the circular buffer index
        self.current_index = (self.current_index + 1) % history_len;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slitscan_history() {
        let width = 2;
        let height = 2;
        let history_len = 2;
        let mut filter = SlitScanFilter::new(width, height, history_len);

        let mut fb = Framebuffer::new(width as u32, height as u32).unwrap();

        // Frame 0: Fill with 0x00000001
        fb.clear(0x0000_0001);
        filter.apply(&mut fb);

        // Frame 1: Fill with 0x00000002
        fb.clear(0x0000_0002);
        filter.apply(&mut fb);

        // After frame 1:
        // y = 0 -> t = 0 -> current index (which is now 1 because apply() advances the index at the end, but wait!)
        // In apply(), the buffer state is processed before current_index is advanced.
        // During Frame 1's apply:
        // current_index was 1.
        // y = 0 -> t = (0 * 2)/2 = 0. sample_index = (1 + 2 - 0) % 2 = 1. row 0 uses history[1] (Frame 1 -> 2)
        // y = 1 -> t = (1 * 2)/2 = 1. sample_index = (1 + 2 - 1) % 2 = 0. row 1 uses history[0] (Frame 0 -> 1)

        let row0 = &fb.as_slice()[0..2];
        let row1 = &fb.as_slice()[2..4];

        assert_eq!(row0, &[0x0000_0002, 0x0000_0002]);
        assert_eq!(row1, &[0x0000_0001, 0x0000_0001]);
    }
}
