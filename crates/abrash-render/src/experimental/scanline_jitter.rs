//! Scanline Jitter Filter
//!
//! A post-processing effect that simulates analog video degradation where
//! individual scanlines shift horizontally by small random amounts.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::XorShift32;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Scanline Jitter effect.
#[derive(Debug, Clone, Copy)]
pub struct ScanlineJitterConfig {
    /// Maximum number of pixels a scanline can shift horizontally.
    pub max_shift: u32,
    /// The probability (0.0 to 1.0) that a given scanline will jitter.
    pub probability: f32,
    /// Random seed for jitter variation per frame.
    pub seed: u32,
}

impl Default for ScanlineJitterConfig {
    fn default() -> Self {
        Self {
            max_shift: 5,
            probability: 0.1,
            seed: 42,
        }
    }
}

/// Applies a Scanline Jitter effect to the framebuffer.
///
/// Shifts horizontal scanlines (rows) left or right randomly based on the
/// configuration to simulate a glitchy, unstable analog video signal.
pub fn apply_scanline_jitter(fb: &mut Framebuffer, config: &ScanlineJitterConfig) {
    if config.max_shift == 0 || config.probability <= 0.0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let pixels = fb.as_mut_slice();
    let max_shift = config.max_shift.min(width as u32 / 2) as i32;

    // Use chunk mapping so we can apply thread-safe random noise
    #[cfg(feature = "parallel")]
    let row_iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = pixels.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        // Fast PRNG per row, seeded by the frame seed and row index
        let mut rng = XorShift32::new(config.seed.wrapping_add(y as u32));

        let should_jitter = rng.next_f32() < config.probability;

        if should_jitter {
            // Jitter shift from -max_shift to +max_shift
            let shift_range = (max_shift * 2 + 1) as u32;
            let shift = (rng.next_u32() % shift_range) as i32 - max_shift;

            if shift > 0 {
                // Shift right
                let s = shift as usize;
                row.copy_within(0..width - s, s);
                // Fill newly exposed pixels with black or clamp color (we choose clamp)
                let clamp_color = row[s];
                row[0..s].fill(clamp_color);
            } else if shift < 0 {
                // Shift left
                let s = (-shift) as usize;
                row.copy_within(s..width, 0);
                let clamp_color = row[width - s - 1];
                row[width - s..width].fill(clamp_color);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanline_jitter() {
        let mut fb = Framebuffer::new(10, 10).unwrap();

        // Draw vertical white line
        for y in 0..10 {
            fb.set_pixel(5, y, 0xFF_FFFFFF);
        }

        let config = ScanlineJitterConfig {
            max_shift: 3,
            probability: 1.0, // Force every line to jitter
            seed: 12345,
        };

        apply_scanline_jitter(&mut fb, &config);

        // Verify the line has jittered (is no longer perfectly at x=5)
        let mut all_at_5 = true;
        for y in 0..10 {
            if fb.get_pixel(5, y).unwrap() != 0xFF_FFFFFF {
                all_at_5 = false;
                break;
            }
        }
        assert!(!all_at_5, "Jitter should have moved the vertical line");
    }
}
