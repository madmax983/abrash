//! Neon Outline Filter Module
//!
//! A retro cyberpunk/synthwave post-processing filter that extracts edges using
//! a Sobel operator and applies a vibrant, glowing neon tint based on the edge direction.
//! Horizontal and vertical edges are mapped to distinct neon colors (e.g., cyan and pink).

use crate::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::cell::RefCell;

thread_local! {
    static NEON_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the Neon Outline effect.
#[derive(Debug, Clone, Copy)]
pub struct NeonOutlineConfig {
    /// The threshold for edge detection. Higher values show fewer edges.
    pub threshold: u32,
    /// The color for predominantly horizontal edges.
    pub color_horizontal: u32,
    /// The color for predominantly vertical edges.
    pub color_vertical: u32,
    /// How much of the original background to blend in (0.0 = completely dark background).
    pub background_blend: f32,
}

impl Default for NeonOutlineConfig {
    fn default() -> Self {
        Self {
            threshold: 50,
            color_horizontal: 0xFF_00FFFF, // Cyan
            color_vertical: 0xFF_FF00FF,   // Pink
            background_blend: 0.1,
        }
    }
}

/// Applies a Neon Outline filter to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - The configuration for the effect.
pub fn apply_neon_outline(fb: &mut Framebuffer, config: &NeonOutlineConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    // ⚡ Bolt Optimization: Pre-calculating the squared threshold allows us to elide
    // the expensive `sqrt()` operation for the vast majority of non-edge pixels.
    let threshold_sq = u64::from(config.threshold) * u64::from(config.threshold);

    NEON_BUFFER.with(|buf| {
        let mut src_fb_vec = buf.borrow_mut();
        let size = width * height;
        if src_fb_vec.len() < size {
            src_fb_vec.resize(size, 0);
        }

        let src_pixels = &mut src_fb_vec[..size];
        src_pixels.copy_from_slice(fb.as_slice());

        let dest_pixels = fb.as_mut_slice();

        #[cfg(feature = "parallel")]
        let row_iter = dest_pixels.par_chunks_exact_mut(width.max(1)).enumerate();
        #[cfg(not(feature = "parallel"))]
        let row_iter = dest_pixels.chunks_exact_mut(width.max(1)).enumerate();

        row_iter.for_each(|(y, row)| {
            for (x, pixel_out) in row.iter_mut().enumerate() {
                // Keep the border pixels as dark background or original
                if x == 0 || x == width - 1 || y == 0 || y == height - 1 {
                    let original = src_pixels[y * width + x];
                    *pixel_out = blend_color(0xFF_000000, original, config.background_blend);
                    continue;
                }

                // Sobel Operator for luminance
                //
                // Gx:
                // -1  0  1
                // -2  0  2
                // -1  0  1
                //
                // Gy:
                // -1 -2 -1
                //  0  0  0
                //  1  2  1

                let tl = i32::from(get_luminance(src_pixels, width, x - 1, y - 1));
                let tc = i32::from(get_luminance(src_pixels, width, x, y - 1));
                let tr = i32::from(get_luminance(src_pixels, width, x + 1, y - 1));

                let cl = i32::from(get_luminance(src_pixels, width, x - 1, y));
                // let cc = get_luminance(src_pixels, width, x, y) as i32;
                let cr = i32::from(get_luminance(src_pixels, width, x + 1, y));

                let bl = i32::from(get_luminance(src_pixels, width, x - 1, y + 1));
                let bc = i32::from(get_luminance(src_pixels, width, x, y + 1));
                let br = i32::from(get_luminance(src_pixels, width, x + 1, y + 1));

                let gx = (tr + 2 * cr + br) - (tl + 2 * cl + bl);
                let gy = (bl + 2 * bc + br) - (tl + 2 * tc + tr);

                let magnitude_sq = (gx * gx + gy * gy) as u64;

                if magnitude_sq > threshold_sq {
                    let magnitude = (magnitude_sq as f32).sqrt() as u32;
                    // Normalize gx and gy to find the edge direction
                    let abs_gx = gx.abs() as f32;
                    let abs_gy = gy.abs() as f32;

                    let total_g = abs_gx + abs_gy + 0.0001; // Avoid divide by zero

                    // We color the edge based on its direction
                    // If Gx is dominant, it's a vertical edge
                    let vertical_weight = abs_gx / total_g;
                    let horizontal_weight = abs_gy / total_g;

                    // The intensity of the glow depends on how far past the threshold we are
                    let intensity = ((magnitude - config.threshold) as f32 / 100.0).clamp(0.0, 1.0);

                    let color = blend_neon(
                        config.color_horizontal,
                        config.color_vertical,
                        horizontal_weight,
                        vertical_weight,
                        intensity,
                    );
                    *pixel_out = color;
                } else {
                    let original = src_pixels[y * width + x];
                    *pixel_out = blend_color(0xFF_000000, original, config.background_blend);
                }
            }
        });
    });
}

#[inline(always)]
fn get_luminance(src: &[u32], width: usize, x: usize, y: usize) -> u8 {
    pixel_luminance(src[y * width + x])
}

#[inline(always)]
fn blend_color(c1: u32, c2: u32, t: f32) -> u32 {
    let r1 = ((c1 >> 16) & 0xFF) as f32;
    let g1 = ((c1 >> 8) & 0xFF) as f32;
    let b1 = (c1 & 0xFF) as f32;

    let r2 = ((c2 >> 16) & 0xFF) as f32;
    let g2 = ((c2 >> 8) & 0xFF) as f32;
    let b2 = (c2 & 0xFF) as f32;

    let r = (r1 + (r2 - r1) * t) as u32;
    let g = (g1 + (g2 - g1) * t) as u32;
    let b = (b1 + (b2 - b1) * t) as u32;

    0xFF00_0000 | (r << 16) | (g << 8) | b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neon_outline_basic() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFF00_0000);

        // Draw a solid white box
        for y in 3..7 {
            for x in 3..7 {
                fb.set_pixel(x, y, 0xFFFF_FFFF);
            }
        }

        let config = NeonOutlineConfig {
            threshold: 10,
            color_horizontal: 0xFF_00FFFF,
            color_vertical: 0xFF_FF00FF,
            background_blend: 0.0,
        };

        apply_neon_outline(&mut fb, &config);

        // Edge pixels should have some neon color, inner/outer should be dark
        let center = fb.get_pixel(5, 5).unwrap_or(0);
        assert_eq!(
            center, 0xFF00_0000,
            "Center of solid box should have no edge"
        );

        let outside = fb.get_pixel(1, 1).unwrap_or(0);
        assert_eq!(outside, 0xFF00_0000, "Outside of box should be dark");
    }
}

#[inline(always)]
fn blend_neon(ch: u32, cv: u32, wh: f32, wv: f32, intensity: f32) -> u32 {
    let rh = ((ch >> 16) & 0xFF) as f32;
    let gh = ((ch >> 8) & 0xFF) as f32;
    let bh = (ch & 0xFF) as f32;

    let rv = ((cv >> 16) & 0xFF) as f32;
    let gv = ((cv >> 8) & 0xFF) as f32;
    let bv = (cv & 0xFF) as f32;

    // Additive blend based on weights and intensity
    let r = ((rh * wh + rv * wv) * intensity).clamp(0.0, 255.0) as u32;
    let g = ((gh * wh + gv * wv) * intensity).clamp(0.0, 255.0) as u32;
    let b = ((bh * wh + bv * wv) * intensity).clamp(0.0, 255.0) as u32;

    0xFF00_0000 | (r << 16) | (g << 8) | b
}
