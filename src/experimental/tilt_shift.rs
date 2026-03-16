//! Tilt-Shift Post-Processing Effect.
//!
//! Simulates a miniature faking effect by keeping a central band in focus
//! and blurring the top and bottom of the image.

use crate::framebuffer::Framebuffer;
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::cell::RefCell;

use crate::post_process::blur::{box_blur_horizontal, box_blur_vertical};

thread_local! {
    static TILT_SHIFT_CONTEXT: RefCell<TiltShiftContext> = RefCell::new(TiltShiftContext::default());
}

#[derive(Default)]
struct TiltShiftContext {
    blurred_buffer: Vec<u32>,
    scratch_buffer: Vec<u32>,
    acc_buffer: Vec<i32>,
}

/// Configuration for the Tilt-Shift effect.
#[derive(Clone, Copy, Debug)]
pub struct TiltShiftConfig {
    /// The vertical position of the focal plane (0.0 to 1.0, where 0.5 is the center).
    pub focus_dist: f32,
    /// The range around the focal plane that remains perfectly sharp (0.0 to 1.0).
    pub focus_range: f32,
    /// The maximum radius of the blur for out-of-focus areas.
    pub blur_radius: u32,
}

impl Default for TiltShiftConfig {
    fn default() -> Self {
        Self {
            focus_dist: 0.5,
            focus_range: 0.2,
            blur_radius: 5,
        }
    }
}

/// Applies tilt-shift effect.
///
/// This simulates a miniature effect by blurring the top and bottom of the image
/// while keeping the center band (determined by `focus_dist` and `focus_range`) in focus.
///
/// # Arguments
/// * `fb` - The framebuffer (modified in-place).
/// * `config` - Configuration for the Tilt-Shift effect.
///
/// # Panics
/// Panics if the framebuffer size is extremely large causing allocation failure.
pub fn apply_tilt_shift(fb: &mut Framebuffer, config: &TiltShiftConfig) {
    if config.blur_radius == 0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let needed_size = width * height;
    let acc_needed_size = width * 3;

    if needed_size == 0 {
        return;
    }

    TILT_SHIFT_CONTEXT.with(|ctx| {
        let mut ctx = ctx.borrow_mut();

        // 1. Ensure buffers are large enough
        if ctx.blurred_buffer.len() < needed_size {
            ctx.blurred_buffer.resize(needed_size, 0);
        }
        if ctx.scratch_buffer.len() < needed_size {
            ctx.scratch_buffer.resize(needed_size, 0);
        }
        if ctx.acc_buffer.len() < acc_needed_size {
            ctx.acc_buffer.resize(acc_needed_size, 0);
        }

        let TiltShiftContext {
            blurred_buffer,
            scratch_buffer,
            acc_buffer,
        } = &mut *ctx;

        let blurred_slice = &mut blurred_buffer[..needed_size];
        let scratch_slice = &mut scratch_buffer[..needed_size];
        let acc_slice = &mut acc_buffer[..acc_needed_size];

        let original_pixels = fb.as_slice();

        // Copy original to blurred_buffer to prepare for blurring
        blurred_slice.copy_from_slice(original_pixels);

        // 2. Perform separable box blur
        box_blur_horizontal(
            blurred_slice,
            scratch_slice,
            width,
            height,
            config.blur_radius,
        );
        box_blur_vertical(
            scratch_slice,
            blurred_slice,
            acc_slice,
            width,
            height,
            config.blur_radius,
        );

        let fb_pixels = fb.as_mut_slice();
        let focus_y = (config.focus_dist * height as f32) as f32;
        let sharp_range_pixels = (config.focus_range * height as f32 * 0.5) as f32;

        let max_blur_radius_f = config.blur_radius as f32;

        // 3. Blend original and blurred pixels based on Y-coordinate distance from focus plane
        #[cfg(feature = "parallel")]
        let iter = fb_pixels
            .par_chunks_exact_mut(width)
            .zip(blurred_slice.par_chunks_exact(width))
            .enumerate();

        #[cfg(not(feature = "parallel"))]
        let iter = fb_pixels
            .chunks_exact_mut(width)
            .zip(blurred_slice.chunks_exact(width))
            .enumerate();

        iter.for_each(|(y, (dst_row, blurred_row))| {
            // Distance from current row to focus center
            let y_dist = (y as f32 - focus_y).abs();

            // If we are fully inside the sharp range, we don't need to do anything
            // (dst_row already has the original pixels)
            if y_dist <= sharp_range_pixels {
                return;
            }

            // Calculate blur amount (0.0 to 1.0)
            // Normalize distance outside the sharp range
            let blur_amount =
                ((y_dist - sharp_range_pixels) / (height as f32 * 0.5)).clamp(0.0, 1.0);

            // Apply a smoothstep for softer transition
            let t = blur_amount * blur_amount * (3.0 - 2.0 * blur_amount);

            // Map t to 0-256 for integer blending
            let alpha = (t * 256.0) as u32;
            let inv_alpha = 256 - alpha;

            for x in 0..width {
                let orig_color = dst_row[x];
                let blur_color = blurred_row[x];

                let o_r = (orig_color >> 16) & 0xFF;
                let o_g = (orig_color >> 8) & 0xFF;
                let o_b = orig_color & 0xFF;

                let b_r = (blur_color >> 16) & 0xFF;
                let b_g = (blur_color >> 8) & 0xFF;
                let b_b = blur_color & 0xFF;

                let r = ((o_r * inv_alpha + b_r * alpha) >> 8) as u8;
                let g = ((o_g * inv_alpha + b_g * alpha) >> 8) as u8;
                let b = ((o_b * inv_alpha + b_b * alpha) >> 8) as u8;

                dst_row[x] = 0xFF00_0000 | (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b);
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tilt_shift_focus_band() {
        let width = 100;
        let height = 100;
        let mut fb = Framebuffer::new(width as u32, height as u32).unwrap();

        // Fill with white
        fb.clear(0xFFFFFFFF);

        // Draw a distinct pattern
        for y in 0..height {
            for x in 0..width {
                if (x + y) % 2 == 0 {
                    fb.set_pixel(x as i32, y as i32, 0xFF000000);
                }
            }
        }

        // Clone to compare later
        let mut fb_clone = Framebuffer::new(width as u32, height as u32).unwrap();
        fb_clone.as_mut_slice().copy_from_slice(fb.as_slice());

        let config = TiltShiftConfig {
            focus_dist: 0.5,
            focus_range: 0.2, // 40 to 60 should be perfectly sharp
            blur_radius: 5,
        };

        apply_tilt_shift(&mut fb, &config);

        // Center should be untouched
        let center_y = height / 2;
        for x in 0..width {
            let orig = fb_clone.get_pixel(x as i32, center_y as i32).unwrap();
            let new = fb.get_pixel(x as i32, center_y as i32).unwrap();
            assert_eq!(orig, new, "Center pixel should remain in focus");
        }

        // Edges should be blurred (different from original checkerboard)
        let edge_y = 0;
        let mut different = false;
        for x in 0..width {
            let orig = fb_clone.get_pixel(x as i32, edge_y as i32).unwrap();
            let new = fb.get_pixel(x as i32, edge_y as i32).unwrap();
            if orig != new {
                different = true;
                break;
            }
        }
        assert!(different, "Edge pixels should be blurred");
    }
}
