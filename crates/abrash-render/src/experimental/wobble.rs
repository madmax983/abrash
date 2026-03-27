//! Wobble/Sine Wave Distortion Filter.
//!
//! A retro post-processing effect that distorts the image by displacing pixels horizontally
//! based on a sine wave of their Y-coordinate. This simulates classic SNES-style
//! underwater, heat haze, or dream sequence effects.

use crate::framebuffer::Framebuffer;

/// Configuration for the Wobble effect.
#[derive(Debug, Clone, Copy)]
pub struct WobbleConfig {
    /// Maximum horizontal displacement in pixels.
    pub amplitude: f32,
    /// Number of waves that fit across the height of the screen.
    pub frequency: f32,
    /// The current time or phase of the animation.
    pub time: f32,
}

impl Default for WobbleConfig {
    fn default() -> Self {
        Self {
            amplitude: 10.0,
            frequency: 5.0,
            time: 0.0,
        }
    }
}

use std::cell::RefCell;

thread_local! {
    static ROW_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Applies a horizontal sine-wave wobble distortion to the framebuffer.
///
/// Displaces pixels horizontally by `amplitude * sin(y * frequency + time)`.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - The configuration for the effect.
pub fn apply_wobble(fb: &mut Framebuffer, config: &WobbleConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || config.amplitude <= 0.0 {
        return;
    }

    // We pre-calculate frequency scaling so that `frequency = 1.0` means exactly
    // one full sine wave (2 * PI) fits in the height of the screen.
    let freq_scale = std::f32::consts::TAU * config.frequency / height as f32;

    let dest = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let row_iter = dest.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = dest.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        let y_f32 = y as f32;
        // Calculate the horizontal shift for this entire row.
        // Fast float-to-int cast (as i32) is preferred over .round() in hot loops.
        // Adding 0.5 for rounding before truncation since `fast_sin` approximation might return 0.9999999
        let shift_f = config.amplitude * crate::math::fast_sin(y_f32 * freq_scale + config.time);
        let shift = if shift_f >= 0.0 {
            (shift_f + 0.5) as i32
        } else {
            (shift_f - 0.5) as i32
        };

        if shift == 0 {
            return;
        }

        let mut row_buffer = ROW_BUFFER.with(RefCell::take);
        if row_buffer.len() != width {
            row_buffer.resize(width, 0);
        }
        row_buffer.copy_from_slice(row);

        if shift > 0 {
            // Shift right. We read from left.
            let shift_u = shift as usize;
            let safe_read_end = width.saturating_sub(shift_u);

            if safe_read_end > 0 {
                row[shift_u..width].copy_from_slice(&row_buffer[0..safe_read_end]);
            }

            // Left edge clamping
            let clamp_val = row_buffer[0];
            row[..shift_u.min(width)].fill(clamp_val);

        } else {
            // Shift left. We read from right.
            let abs_shift = (-shift) as usize;
            let safe_write_end = width.saturating_sub(abs_shift);

            if safe_write_end > 0 {
                row[0..safe_write_end].copy_from_slice(&row_buffer[abs_shift..width]);
            }

            // Right edge clamping
            let clamp_val = row_buffer[width - 1];
            row[safe_write_end..width].fill(clamp_val);
        }

        ROW_BUFFER.with(|cell| cell.replace(row_buffer));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_apply_wobble_shifts_pixels() {
        let width = 10;
        let height = 10;
        let mut fb = Framebuffer::new(width, height).unwrap();

        // Draw a vertical white line at x = 5
        for y in 0..height {
            fb.set_pixel(5, y as i32, 0xFFFFFFFF);
        }

        let config = WobbleConfig {
            amplitude: 2.0,
            frequency: 1.0,
            // time = PI / 2.0 means sin(y=0) = 1.0 -> shift = 2.0
            time: std::f32::consts::PI / 2.0,
        };

        apply_wobble(&mut fb, &config);

        // At y = 0: y_f32 * freq_scale = 0.
        // sin(0 + PI/2) = 1.0. Shift = 2.0 * 1.0 = 2.
        // The pixel at x = 5 shifted right by 2 -> should now be at x = 7.
        // Meaning at x = 7, the pixel is read from src_x = 7 - 2 = 5 (which is White).
        let pixel_at_7 = fb.get_pixel(7, 0).unwrap();
        assert_eq!(pixel_at_7, 0xFFFFFFFF, "Pixel should be shifted to x=7");

        let pixel_at_5 = fb.get_pixel(5, 0).unwrap();
        assert_eq!(
            pixel_at_5, 0xFF000000,
            "Original pixel position should be empty/black"
        );
    }
}
