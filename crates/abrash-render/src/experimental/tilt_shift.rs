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

        let focus_y = config.focus_dist * height as f32;
        let sharp_range_pixels = config.focus_range * height as f32 * 0.5;

        // 2. Perform separable box blur.
        // Rows inside the sharp band are never blended, so only the rows outside it
        // (plus `radius` rows of vertical context) need blurring. Blurring a row-range
        // sub-image is exact for the rows that are used: clamping at a sub-image edge
        // only affects rows within `radius` of that edge, which are all unused.
        let radius = config.blur_radius;
        let is_sharp = |y: usize| (y as f32 - focus_y).abs() <= sharp_range_pixels;
        let first_sharp = (0..height).find(|&y| is_sharp(y));
        let last_sharp = (0..height).rev().find(|&y| is_sharp(y));

        match (first_sharp, last_sharp) {
            (Some(first), Some(last))
                if radius as usize <= width
                    && (first == 0
                        || last + 1 == height
                        || (last + 1).saturating_sub(radius as usize)
                            >= first + radius as usize) =>
            {
                let r = radius as usize;
                // Top region: rows [0, first) need source rows [0, first + r).
                // Bottom region: rows (last, height) need source rows [last + 1 - r, height).
                // The guard keeps the regions' written rows disjoint (a narrow band falls back
                // to the full blur). Both horizontal passes run before either vertical pass: the vertical pass
                // writes into `blurred_slice`, which the horizontal pass reads, and the two
                // regions' context rows can overlap.
                let top_h = if first > 0 {
                    (first + r).min(height)
                } else {
                    0
                };
                let bot_start = (last + 1).saturating_sub(r);
                let bot_h = if last + 1 < height {
                    height - bot_start
                } else {
                    0
                };
                let (top_n, bot_off) = (top_h * width, bot_start * width);

                if top_h > 0 {
                    box_blur_horizontal(
                        &blurred_slice[..top_n],
                        &mut scratch_slice[..top_n],
                        width,
                        top_h,
                        radius,
                    );
                }
                if bot_h > 0 {
                    box_blur_horizontal(
                        &blurred_slice[bot_off..],
                        &mut scratch_slice[bot_off..],
                        width,
                        bot_h,
                        radius,
                    );
                }
                if top_h > 0 {
                    box_blur_vertical(
                        &scratch_slice[..top_n],
                        &mut blurred_slice[..top_n],
                        acc_slice,
                        width,
                        top_h,
                        radius,
                    );
                }
                if bot_h > 0 {
                    box_blur_vertical(
                        &scratch_slice[bot_off..],
                        &mut blurred_slice[bot_off..],
                        acc_slice,
                        width,
                        bot_h,
                        radius,
                    );
                }
            }
            _ => {
                box_blur_horizontal(blurred_slice, scratch_slice, width, height, radius);
                box_blur_vertical(
                    scratch_slice,
                    blurred_slice,
                    acc_slice,
                    width,
                    height,
                    radius,
                );
            }
        }

        let fb_pixels = fb.as_mut_slice();
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

            // ⚡ Bolt: SWAR (SIMD Within A Register) for per-pixel color blending.
            // Process Red and Blue channels simultaneously to eliminate intermediate shifts.
            for (dst_pixel, blur_color) in dst_row[..width].iter_mut().zip(&blurred_row[..width]) {
                let orig_color = *dst_pixel;
                let blur_color = *blur_color;

                let o_rb = orig_color & 0x00FF_00FF;
                let o_g = orig_color & 0x0000_FF00;

                let b_rb = blur_color & 0x00FF_00FF;
                let b_g = blur_color & 0x0000_FF00;

                let out_rb = ((o_rb * inv_alpha + b_rb * alpha) >> 8) & 0x00FF_00FF;
                let out_g = ((o_g * inv_alpha + b_g * alpha) >> 8) & 0x0000_FF00;

                *dst_pixel = (orig_color & 0xFF00_0000) | out_rb | out_g;
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference: blur the whole image, then blend (the pre-banding behavior).
    fn reference_tilt_shift(fb: &mut Framebuffer, config: &TiltShiftConfig) {
        let (w, h) = (fb.width() as usize, fb.height() as usize);
        let mut blurred = fb.as_slice().to_vec();
        let mut scratch = vec![0u32; w * h];
        let mut acc = vec![0i32; w * 3];
        box_blur_horizontal(&blurred, &mut scratch, w, h, config.blur_radius);
        box_blur_vertical(&scratch, &mut blurred, &mut acc, w, h, config.blur_radius);
        let focus_y = config.focus_dist * h as f32;
        let sharp = config.focus_range * h as f32 * 0.5;
        for y in 0..h {
            let y_dist = (y as f32 - focus_y).abs();
            if y_dist <= sharp {
                continue;
            }
            let amt = ((y_dist - sharp) / (h as f32 * 0.5)).clamp(0.0, 1.0);
            let t = amt * amt * (3.0 - 2.0 * amt);
            let alpha = (t * 256.0) as u32;
            let inv = 256 - alpha;
            for x in 0..w {
                let o = fb.as_slice()[y * w + x];
                let b = blurred[y * w + x];
                let rb = (((o & 0x00FF_00FF) * inv + (b & 0x00FF_00FF) * alpha) >> 8) & 0x00FF_00FF;
                let g = (((o & 0xFF00) * inv + (b & 0xFF00) * alpha) >> 8) & 0xFF00;
                fb.as_mut_slice()[y * w + x] = (o & 0xFF00_0000) | rb | g;
            }
        }
    }

    #[test]
    fn test_tilt_shift_banded_blur_matches_full_blur() {
        let (w, h) = (37usize, 61usize);
        let mut seed = 0x9E37_79B9u32;
        let mut base = Framebuffer::new(w as u32, h as u32).unwrap();
        for p in base.as_mut_slice() {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            *p = 0xFF00_0000 | (seed >> 8);
        }
        for &(focus_dist, focus_range, blur_radius) in &[
            (0.5, 0.2, 5),
            (0.0, 0.2, 3),
            (1.0, 0.3, 4),
            (0.1, 0.05, 7),
            (0.5, 0.0, 2),
            (0.5, 2.0, 5),
            (0.3, 0.1, 40),
            (0.9, 0.1, 1),
        ] {
            let config = TiltShiftConfig {
                focus_dist,
                focus_range,
                blur_radius,
            };
            let mut fast = Framebuffer::new(w as u32, h as u32).unwrap();
            fast.as_mut_slice().copy_from_slice(base.as_slice());
            let mut slow = Framebuffer::new(w as u32, h as u32).unwrap();
            slow.as_mut_slice().copy_from_slice(base.as_slice());
            apply_tilt_shift(&mut fast, &config);
            reference_tilt_shift(&mut slow, &config);
            assert_eq!(fast.as_slice(), slow.as_slice(), "{config:?}");
        }
    }

    #[test]
    fn test_tilt_shift_focus_band() {
        let width = 100;
        let height = 100;
        let mut fb = Framebuffer::new(width as u32, height as u32).unwrap();

        // Fill with white
        fb.clear(0xFFFF_FFFF);

        // Draw a distinct pattern
        for y in 0..height {
            for x in 0..width {
                if (x + y) % 2 == 0 {
                    fb.set_pixel(x as i32, y as i32, 0xFF00_0000);
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
