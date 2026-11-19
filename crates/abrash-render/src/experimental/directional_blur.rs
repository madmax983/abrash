//! Directional blur post-processing effect.
//!
//! This module provides a fast directional blur (e.g., motion blur) applied in screen space.

use crate::framebuffer::Framebuffer;

/// Applies a directional (motion) blur to the framebuffer.
///
/// Blurs pixels along a given 2D vector `(dx, dy)`.
/// `num_samples` determines the quality and performance of the blur.
#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Directional Blur effect.
#[derive(Clone, Copy, Debug)]
pub struct DirectionalBlurConfig {
    /// Horizontal distance of the blur in pixels.
    pub dx: f32,
    /// Vertical distance of the blur in pixels.
    pub dy: f32,
    /// Number of samples to take along the blur direction.
    pub num_samples: usize,
}

impl Default for DirectionalBlurConfig {
    fn default() -> Self {
        Self {
            dx: 10.0,
            dy: 0.0,
            num_samples: 5,
        }
    }
}

use std::cell::RefCell;

thread_local! {
    static SOURCE_PIXELS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// True if the 16.16 positions `start + k * step` for `k in 0..=last_step` all
/// land inside `0..len` after the `>> 16` (checked at both endpoints; the
/// sequence is linear so the interior follows).
#[inline(always)]
fn span_inside(start: i32, step: i32, last_step: i64, len: usize) -> bool {
    let first = i64::from(start);
    let last = first + last_step * i64::from(step);
    let hi = (len as i64) << 16;
    first >= 0 && first < hi && last >= 0 && last < hi
}

/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
pub fn apply_directional_blur(framebuffer: &mut Framebuffer, config: &DirectionalBlurConfig) {
    if config.num_samples <= 1 {
        return;
    }

    let width = framebuffer.width() as usize;
    let height = framebuffer.height() as usize;
    if width == 0 || height == 0 {
        return;
    }

    // Pre-calculate fixed point step sizes for integer coordinate tracking
    let inv_samples_f32 = 1.0 / (config.num_samples as f32);
    let dx_step_fixed = (config.dx * inv_samples_f32 * 65536.0) as i32;
    let dy_step_fixed = (config.dy * inv_samples_f32 * 65536.0) as i32;

    // Fixed point multiplier for dividing sums
    let inv_samples_fixed = (inv_samples_f32 * 65536.0) as u32;

    let last_step = (config.num_samples - 1) as i64;

    SOURCE_PIXELS.with(|source_pixels_cell| {
        let mut source_pixels = source_pixels_cell.borrow_mut();
        let fb_slice = framebuffer.as_slice();

        if source_pixels.len() != fb_slice.len() {
            source_pixels.resize(fb_slice.len(), 0);
        }
        source_pixels.copy_from_slice(fb_slice);
        // We now safely reference the slice. We can extract it as an immutable reference
        // to pass into the parallel iterator safely since `RefMut` doesn't implement `Sync`.
        let source_slice: &[u32] = &source_pixels;

        let process_row = |(y, row): (usize, &mut [u32])| {
            // Start Y at pixel center + 0.5 (32768) for rounding equivalent
            let start_y = (y << 16) as i32 + 32768;

            // The sample position is linear in the sample index, so every sample's
            // row is in range iff the first and last are (no i32 wrap in i64 math).
            let y_inside = span_inside(start_y, dy_step_fixed, last_step, height);

            for (x, pixel) in row.iter_mut().enumerate().take(width) {
                let mut cur_x = (x << 16) as i32 + 32768;
                let mut cur_y = start_y;

                let mut r_sum = 0;
                let mut g_sum = 0;
                let mut b_sum = 0;

                if y_inside && span_inside(cur_x, dx_step_fixed, last_step, width) {
                    // Interior: no sample can leave the image, so skip the clamps.
                    if config.num_samples <= 256 {
                        // Sum R and B together in 16-bit lanes (<= 256 * 255 fits).
                        let mut rb_sum = 0u32;
                        for _ in 0..config.num_samples {
                            let color = source_slice
                                [(cur_y >> 16) as usize * width + (cur_x >> 16) as usize];
                            rb_sum += color & 0x00FF_00FF;
                            g_sum += (color >> 8) & 0xFF;
                            cur_x += dx_step_fixed;
                            cur_y += dy_step_fixed;
                        }
                        r_sum = rb_sum >> 16;
                        b_sum = rb_sum & 0xFFFF;
                    } else {
                        for _ in 0..config.num_samples {
                            let color = source_slice
                                [(cur_y >> 16) as usize * width + (cur_x >> 16) as usize];
                            r_sum += (color >> 16) & 0xFF;
                            g_sum += (color >> 8) & 0xFF;
                            b_sum += color & 0xFF;
                            cur_x += dx_step_fixed;
                            cur_y += dy_step_fixed;
                        }
                    }
                } else {
                    for _ in 0..config.num_samples {
                        // Nearest neighbor sampling by shifting down the fixed-point coordinate
                        let px = cur_x >> 16;
                        let py = cur_y >> 16;

                        // Clamp to edges
                        let px = px.clamp(0, width as i32 - 1) as usize;
                        let py = py.clamp(0, height as i32 - 1) as usize;

                        let color = source_slice[py * width + px];
                        let r = (color >> 16) & 0xFF;
                        let g = (color >> 8) & 0xFF;
                        let b = color & 0xFF;

                        r_sum += r;
                        g_sum += g;
                        b_sum += b;

                        // ⚡ Bolt: Use `wrapping_add` to prevent overflow panics in debug mode when fuzz testing
                        // injects extreme boundary values that trigger massive deltas.
                        cur_x = cur_x.wrapping_add(dx_step_fixed);
                        cur_y = cur_y.wrapping_add(dy_step_fixed);
                    }
                }

                // Multiply by fixed-point inverse and shift down
                let final_r = (r_sum * inv_samples_fixed) >> 16;
                let final_g = (g_sum * inv_samples_fixed) >> 16;
                let final_b = (b_sum * inv_samples_fixed) >> 16;

                *pixel = 0xFF00_0000 | (final_r << 16) | (final_g << 8) | final_b;
            }
        };

        #[cfg(feature = "parallel")]
        {
            framebuffer
                .as_mut_slice()
                .par_chunks_exact_mut(width)
                .enumerate()
                .for_each(process_row);
        }

        #[cfg(not(feature = "parallel"))]
        {
            framebuffer
                .as_mut_slice()
                .chunks_exact_mut(width)
                .enumerate()
                .for_each(process_row);
        }
    }); // Close SOURCE_PIXELS.with
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_directional_blur_zero_samples() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        fb.set_pixel(0, 0, 0xFF00_0000);

        let config = DirectionalBlurConfig {
            dx: 10.0,
            dy: 0.0,
            num_samples: 0,
        };
        apply_directional_blur(&mut fb, &config);

        assert_eq!(fb.get_pixel(0, 0), Some(0xFF00_0000));
    }

    #[test]
    fn test_directional_blur_horizontal() {
        let mut fb = Framebuffer::new(4, 1).unwrap();
        fb.set_pixel(0, 0, 0xFFFF_FFFF); // White pixel
        fb.set_pixel(1, 0, 0xFF00_0000); // Black pixels
        fb.set_pixel(2, 0, 0xFF00_0000);
        fb.set_pixel(3, 0, 0xFF00_0000);

        // Blur rightwards by 3 pixels, 3 samples
        let config = DirectionalBlurConfig {
            dx: 3.0,
            dy: 0.0,
            num_samples: 3,
        };
        apply_directional_blur(&mut fb, &config);

        // The white pixel should be spread
        let p0 = fb.get_pixel(0, 0).unwrap();
        let p1 = fb.get_pixel(1, 0).unwrap();

        // At x=0, samples at x=0, 1, 2. (White, Black, Black) -> ~1/3 White
        assert!(p0 != 0xFFFF_FFFF);
        assert!(p0 != 0xFF00_0000);

        // At x=1, samples at x=1, 2, 3. (Black, Black, Black) -> Black
        assert_eq!(p1, 0xFF00_0000);
    }

    /// Straight clamp-everywhere reference of the blur.
    fn reference(src: &Framebuffer, c: &DirectionalBlurConfig) -> Vec<u32> {
        let (w, h) = (src.width() as i32, src.height() as i32);
        let inv = 1.0 / (c.num_samples as f32);
        let dxs = (c.dx * inv * 65536.0) as i32;
        let dys = (c.dy * inv * 65536.0) as i32;
        let invf = (inv * 65536.0) as u32;
        let mut out = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let (mut cx, mut cy) = ((x << 16) + 32768, (y << 16) + 32768);
                let (mut r, mut g, mut b) = (0u32, 0u32, 0u32);
                for _ in 0..c.num_samples {
                    let px = (cx >> 16).clamp(0, w - 1);
                    let py = (cy >> 16).clamp(0, h - 1);
                    let p = src.get_pixel(px, py).unwrap();
                    r += (p >> 16) & 0xFF;
                    g += (p >> 8) & 0xFF;
                    b += p & 0xFF;
                    cx = cx.wrapping_add(dxs);
                    cy = cy.wrapping_add(dys);
                }
                out.push(
                    0xFF00_0000
                        | ((r * invf) >> 16) << 16
                        | ((g * invf) >> 16) << 8
                        | ((b * invf) >> 16),
                );
            }
        }
        out
    }

    #[test]
    fn test_directional_blur_matches_clamped_reference() {
        let (w, h) = (37u32, 23u32);
        let mut src = Framebuffer::new(w, h).unwrap();
        for y in 0..h {
            for x in 0..w {
                let c = 0xFF00_0000
                    | ((x * 7 + y) & 0xFF) << 16
                    | ((y * 11) & 0xFF) << 8
                    | ((x * y) & 0xFF);
                src.set_pixel(x as i32, y as i32, c);
            }
        }
        for &(dx, dy, n) in &[
            (10.0, 0.0, 5),
            (-20.0, 13.0, 16),
            (0.0, -40.0, 7),
            (60.0, 60.0, 256),
            (-30.0, 5.0, 300),
            (3.0, -3.0, 2),
        ] {
            let c = DirectionalBlurConfig {
                dx,
                dy,
                num_samples: n,
            };
            let mut fb = Framebuffer::new(w, h).unwrap();
            fb.as_mut_slice().copy_from_slice(src.as_slice());
            apply_directional_blur(&mut fb, &c);
            assert_eq!(
                fb.as_slice(),
                reference(&src, &c).as_slice(),
                "{dx} {dy} {n}"
            );
        }
    }
}
