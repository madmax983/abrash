//! Screen Melt post-processing effect.
//!
//! Creates a classic "Doom-style" screen melt transition effect where columns
//! of pixels drip down the screen at varying speeds.

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

thread_local! {
    static MELT_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration and state for the Screen Melt post-processing filter.
#[derive(Debug, Clone)]
pub struct MeltConfig {
    /// The current time or progress of the melt effect.
    pub time: f32,
    /// Speed multiplier for how fast columns drop.
    pub speed: f32,
    /// Color to fill in the empty space left behind by the falling columns.
    pub background_color: u32,
    /// Array of random column speeds/offsets.
    /// This is generated once on the first run and re-used to keep the melt consistent.
    offsets: Vec<f32>,
    /// Whether the offsets have been initialized for the current resolution.
    initialized_width: usize,
}

impl Default for MeltConfig {
    fn default() -> Self {
        Self {
            time: 0.0,
            speed: 50.0,
            background_color: 0xFF_00_00_00, // Black
            offsets: Vec::new(),
            initialized_width: 0,
        }
    }
}

impl MeltConfig {
    /// Creates a new `MeltConfig` with the specified speed and background color.
    #[must_use]
    pub const fn new(speed: f32, background_color: u32) -> Self {
        Self {
            time: 0.0,
            speed,
            background_color,
            offsets: Vec::new(),
            initialized_width: 0,
        }
    }

    /// ⚡ Bolt: Creates a new `MeltConfig` with pre-allocated column capacities to avoid dynamic heap reallocations when resizing the offsets vector per frame.
    #[must_use]
    pub fn with_capacity(speed: f32, background_color: u32, capacity: usize) -> Self {
        Self {
            time: 0.0,
            speed,
            background_color,
            offsets: Vec::with_capacity(capacity),
            initialized_width: 0,
        }
    }

    /// Resets the melt state, forcing it to generate new column offsets on the next frame.
    pub fn reset(&mut self) {
        self.time = 0.0;
        self.initialized_width = 0;
        self.offsets.clear();
    }
}

/// Applies a Screen Melt filter to the framebuffer in-place.
///
/// Displaces columns of pixels downwards based on their individual random speeds
/// and the total elapsed time.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify.
/// * `config` - The configuration and state parameters for the melt effect.
pub fn apply_melt(fb: &mut Framebuffer, config: &mut MeltConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    // Initialize or resize offsets if resolution changes
    if config.initialized_width != width {
        config.offsets.resize(width, 0.0);

        // Generate pseudo-random melt offsets
        // We use a simple hash/sine combination to avoid depending on the `rand` crate directly here
        for x in 0..width {
            let x_f = x as f32;
            // Generate a chaotic but deterministic value between 0.0 and 1.0
            let r = ((x_f * 12.9898 + x_f * 78.233).sin() * 43758.5453)
                .fract()
                .abs();
            // Start columns at slightly different negative offsets so they don't all fall immediately
            config.offsets[x] = -r * 50.0;
        }
        config.initialized_width = width;
    }

    let time = config.time;
    let speed = config.speed;
    let bg_color = config.background_color;
    let offsets = &config.offsets;

    MELT_BUFFER.with(|buf| {
        let mut src_fb_vec = buf.borrow_mut();
        let size = width * height;
        if src_fb_vec.len() < size {
            src_fb_vec.resize(size, 0);
        }

        // Copy current frame to read from
        let src_fb = &mut src_fb_vec[..size];
        src_fb.copy_from_slice(fb.as_slice());

        let dest_pixels = fb.as_mut_slice();

        // Process column by column
        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;

            // We iterate over the destination pixels row by row, but the shift logic
            // means we look "up" in the source buffer.
            dest_pixels
                .par_chunks_exact_mut(width)
                .enumerate()
                .for_each(|(y, row)| {
                    for (x, pixel) in row.iter_mut().enumerate() {
                        // How far down has this column moved?
                        // Offset starts negative, so it waits a bit before dropping.
                        let column_drop = (offsets[x] + time * speed).max(0.0) as i32;

                        // Where did this pixel come from in the original image?
                        let source_y = y as i32 - column_drop;

                        if source_y >= 0 {
                            // Pixel comes from higher up
                            *pixel = src_fb[(source_y as usize) * width + x];
                        } else {
                            // Empty space left behind by the melt
                            *pixel = bg_color;
                        }
                    }
                });
        }

        #[cfg(not(feature = "parallel"))]
        {
            for y in 0..height {
                let dest_row_start = y * width;
                let y_i32 = y as i32;

                for x in 0..width {
                    let column_drop = (offsets[x] + time * speed).max(0.0) as i32;
                    let source_y = y_i32 - column_drop;

                    if source_y >= 0 {
                        dest_pixels[dest_row_start + x] = src_fb[(source_y as usize) * width + x];
                    } else {
                        dest_pixels[dest_row_start + x] = bg_color;
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_melt_initialization() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        let mut config = MeltConfig::default();

        assert_eq!(config.initialized_width, 0);

        apply_melt(&mut fb, &mut config);

        assert_eq!(config.initialized_width, 10);
        assert_eq!(config.offsets.len(), 10);
    }

    #[test]
    fn test_apply_melt_shift() {
        let mut fb = Framebuffer::new(5, 5).unwrap();
        // Fill top row with RED, everything else BLACK
        fb.clear(0xFF_00_00_00);
        for x in 0..5 {
            fb.set_pixel(x, 0, 0xFF_FF_00_00);
        }

        let mut config = MeltConfig::with_capacity(100.0, 0xFF_00_FF_00, 5);
        config.time = 100.0; // Force a large time to ensure all columns drop

        apply_melt(&mut fb, &mut config);

        // Since time is huge, the red top row should be pushed off the bottom of the screen,
        // and the entire screen should be filled with the background color.
        for pixel in fb.as_slice() {
            assert_eq!(
                *pixel, 0xFF_00_FF_00,
                "Pixel should be replaced by background color after a long melt"
            );
        }
    }
}
