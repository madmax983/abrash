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
    static SOURCE_PIXELS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
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

    let mut source_pixels = SOURCE_PIXELS.with(RefCell::take);

    let fb_slice = fb.as_slice();
    if source_pixels.len() != fb_slice.len() {
        source_pixels.resize(fb_slice.len(), 0);
    }
    source_pixels.copy_from_slice(fb_slice);

    let dest = fb.as_mut_slice();
    let src: &[u32] = &source_pixels;

    // We pre-calculate frequency scaling so that `frequency = 1.0` means exactly
    // one full sine wave (2 * PI) fits in the height of the screen.
    let freq_scale = std::f32::consts::TAU * config.frequency / height as f32;

    #[cfg(feature = "parallel")]
    let row_iter = dest.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = dest.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        let y_f32 = y as f32;
        // Calculate the horizontal shift for this entire row.
        // Fast float-to-int cast (as i32) is preferred over .round() in hot loops.
        // Adding 0.5 for rounding before truncation since `fast_sin` approximation might return 0.9999999
        let shift_f = config.amplitude * (y_f32 * freq_scale + config.time).sin();
        let shift = if shift_f >= 0.0 {
            (shift_f + 0.5) as i32
        } else {
            (shift_f - 0.5) as i32
        };

        let src_row_start = y * width;
        let src_row = &src[src_row_start..src_row_start + width];

        // Every pixel reads `src_row[(x - shift).clamp(0, width - 1)]`, so the row is a
        // clamped fill on one edge and a straight copy of the rest.
        let edge = (shift.unsigned_abs() as usize).min(width);
        if shift >= 0 {
            row[..edge].fill(src_row[0]);
            row[edge..].copy_from_slice(&src_row[..width - edge]);
        } else {
            row[..width - edge].copy_from_slice(&src_row[edge..]);
            row[width - edge..].fill(src_row[width - 1]);
        }
    });

    SOURCE_PIXELS.with(|source_pixels_cell| source_pixels_cell.replace(source_pixels));
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
            fb.set_pixel(5, y as i32, 0xFFFF_FFFF);
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
        assert_eq!(pixel_at_7, 0xFFFF_FFFF, "Pixel should be shifted to x=7");

        let pixel_at_5 = fb.get_pixel(5, 0).unwrap();
        assert_eq!(
            pixel_at_5, 0xFF00_0000,
            "Original pixel position should be empty/black"
        );
    }

    #[test]
    fn test_apply_wobble_matches_per_pixel_clamp() {
        let (width, height) = (7u32, 64u32);
        for amplitude in [0.5_f32, 3.0, 6.0, 7.0, 20.0, 1.0e12] {
            let mut fb = Framebuffer::new(width, height).unwrap();
            for (i, p) in fb.as_mut_slice().iter_mut().enumerate() {
                *p = 0xFF00_0000 | i as u32;
            }
            let original = fb.as_slice().to_vec();
            let config = WobbleConfig {
                amplitude,
                frequency: 3.0,
                time: 0.7,
            };
            apply_wobble(&mut fb, &config);

            let freq_scale = std::f32::consts::TAU * config.frequency / height as f32;
            for y in 0..height as usize {
                let shift_f = amplitude * (y as f32 * freq_scale + config.time).sin();
                let shift = if shift_f >= 0.0 {
                    (shift_f + 0.5) as i64
                } else {
                    (shift_f - 0.5) as i64
                };
                for x in 0..width as usize {
                    let sx = (x as i64 - shift).clamp(0, i64::from(width) - 1) as usize;
                    assert_eq!(
                        fb.as_slice()[y * width as usize + x],
                        original[y * width as usize + sx]
                    );
                }
            }
        }
    }
}
