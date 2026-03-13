use crate::framebuffer::Framebuffer;
use crate::utils::pixel_luminance;

use std::cell::RefCell;

thread_local! {
    static LUMA_BUFFER: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the Edge Glow post-processing filter.
#[derive(Debug, Clone, Copy)]
pub struct EdgeGlowConfig {
    /// The RGB color to apply to edges (e.g., 0x00FF00FF for Magenta).
    /// The top byte (Alpha) is ignored.
    pub edge_color: u32,
    /// Multiplier for the glow intensity (1.0 is normal, >1.0 is brighter).
    pub intensity: f32,
    /// Minimum magnitude for an edge to be detected (0-255).
    pub edge_threshold: u8,
    /// Multiplier for darkening non-edge areas (0.0 is black, 1.0 is unchanged).
    pub darken_factor: f32,
}

impl Default for EdgeGlowConfig {
    fn default() -> Self {
        Self {
            edge_color: 0x00_00_FF_FF, // Cyan glow
            intensity: 1.5,
            edge_threshold: 30,
            darken_factor: 0.3,
        }
    }
}

/// Applies an "Edge Glow" neon filter to the framebuffer in-place.
#[cfg(feature = "parallel")]
use rayon::prelude::*;

pub fn apply_edge_glow(fb: &mut Framebuffer, config: &EdgeGlowConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let pixels = fb.as_mut_slice();
    let needed_size = width * height;

    LUMA_BUFFER.with(|buf| {
        let mut luma_buffer = buf.borrow_mut();
        if luma_buffer.len() < needed_size {
            luma_buffer.resize(needed_size, 0);
        }
        let luma_slice = &mut luma_buffer[..needed_size];

        // 1. Convert to Luminance
        // The pixel slice and luma_slice are structurally disjoint and can be safely parallelized if feature enabled
        #[cfg(feature = "parallel")]
        {
            luma_slice
                .par_iter_mut()
                .zip(pixels.par_iter())
                .for_each(|(l, p)| {
                    *l = pixel_luminance(*p);
                });
        }
        #[cfg(not(feature = "parallel"))]
        {
            for (i, p) in pixels.iter().enumerate() {
                luma_slice[i] = pixel_luminance(*p);
            }
        }

        // Fixed point configuration for fast inner loops
        let intensity_fixed = (config.intensity * 256.0).max(0.0) as u32;
        let darken_fixed = (config.darken_factor * 256.0).clamp(0.0, 256.0) as u32;

        let edge_r = (config.edge_color >> 16) & 0xFF;
        let edge_g = (config.edge_color >> 8) & 0xFF;
        let edge_b = config.edge_color & 0xFF;

        let threshold = config.edge_threshold as i32;

        // 2. Apply Sobel and Glow
        // We skip 1-pixel border.

        // We use par_chunks_mut over the destination pixels, but we only mutate interior rows.
        // The first and last row are borders.
        let interior_pixels = &mut pixels[width..(height - 1) * width];

        #[cfg(feature = "parallel")]
        let row_iter = interior_pixels.par_chunks_exact_mut(width).enumerate();
        #[cfg(not(feature = "parallel"))]
        let row_iter = interior_pixels.chunks_exact_mut(width).enumerate();

        row_iter.for_each(|(y_idx, row_pixels)| {
            let y = y_idx + 1; // Real y in full buffer
            let row_offset = y * width;
            let prev_row_offset = row_offset - width;
            let next_row_offset = row_offset + width;

            for x in 1..width - 1 {
                let tl = i32::from(luma_slice[prev_row_offset + x - 1]);
                let t = i32::from(luma_slice[prev_row_offset + x]);
                let tr = i32::from(luma_slice[prev_row_offset + x + 1]);
                let l = i32::from(luma_slice[row_offset + x - 1]);
                let r = i32::from(luma_slice[row_offset + x + 1]);
                let bl = i32::from(luma_slice[next_row_offset + x - 1]);
                let b = i32::from(luma_slice[next_row_offset + x]);
                let br = i32::from(luma_slice[next_row_offset + x + 1]);

                let gx = (tr + 2 * r + br) - (tl + 2 * l + bl);
                let gy = (bl + 2 * b + br) - (tl + 2 * t + tr);
                let mag = gx.abs() + gy.abs();

                let original_pixel = row_pixels[x];
                let alpha = original_pixel & 0xFF00_0000;
                let orig_r = (original_pixel >> 16) & 0xFF;
                let orig_g = (original_pixel >> 8) & 0xFF;
                let orig_b = original_pixel & 0xFF;

                let (new_r, new_g, new_b) = if mag > threshold {
                    // Edge detected: Mix edge color with intensity
                    let mag_clamped = mag.min(255) as u32;
                    let glow_r = (edge_r * mag_clamped * intensity_fixed) >> 16;
                    let glow_g = (edge_g * mag_clamped * intensity_fixed) >> 16;
                    let glow_b = (edge_b * mag_clamped * intensity_fixed) >> 16;

                    (
                        (orig_r.saturating_add(glow_r)).min(255),
                        (orig_g.saturating_add(glow_g)).min(255),
                        (orig_b.saturating_add(glow_b)).min(255),
                    )
                } else {
                    // Non-edge: Darken
                    (
                        (orig_r * darken_fixed) >> 8,
                        (orig_g * darken_fixed) >> 8,
                        (orig_b * darken_fixed) >> 8,
                    )
                };

                row_pixels[x] = alpha | (new_r << 16) | (new_g << 8) | new_b;
            }
        });

        // Darken borders correctly too (we skip Sobel there)
        for x in 0..width {
            let idx_top = x;
            let idx_bot = (height - 1) * width + x;
            pixels[idx_top] = darken_pixel(pixels[idx_top], darken_fixed);
            pixels[idx_bot] = darken_pixel(pixels[idx_bot], darken_fixed);
        }
        for y in 1..height - 1 {
            let idx_left = y * width;
            let idx_right = y * width + width - 1;
            pixels[idx_left] = darken_pixel(pixels[idx_left], darken_fixed);
            pixels[idx_right] = darken_pixel(pixels[idx_right], darken_fixed);
        }
    });
}

#[inline(always)]
fn darken_pixel(pixel: u32, darken_fixed: u32) -> u32 {
    let alpha = pixel & 0xFF00_0000;
    let r = (pixel >> 16) & 0xFF;
    let g = (pixel >> 8) & 0xFF;
    let b = pixel & 0xFF;

    let new_r = (r * darken_fixed) >> 8;
    let new_g = (g * darken_fixed) >> 8;
    let new_b = (b * darken_fixed) >> 8;

    alpha | (new_r << 16) | (new_g << 8) | new_b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_glow_config_default() {
        let config = EdgeGlowConfig::default();
        assert_eq!(config.edge_color, 0x00_00_FF_FF);
        assert!((config.intensity - 1.5).abs() < f32::EPSILON);
        assert_eq!(config.edge_threshold, 30);
        assert!((config.darken_factor - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_apply_edge_glow_preserves_alpha() {
        let mut fb = Framebuffer::new(3, 3).unwrap();
        // Set all pixels to transparent white with 0x80 alpha
        fb.clear(0x80_FF_FF_FF);

        let config = EdgeGlowConfig::default();
        apply_edge_glow(&mut fb, &config);

        let p = fb.get_pixel(1, 1).unwrap();
        assert_eq!((p >> 24) & 0xFF, 0x80, "Alpha channel should be preserved");
    }

    #[test]
    fn test_apply_edge_glow_detects_edges() {
        // A 3x3 image with a dark left column and white middle/right column
        let mut fb = Framebuffer::new(3, 3).unwrap();

        // Col 0: Black
        fb.set_pixel(0, 0, 0xFF_00_00_00);
        fb.set_pixel(0, 1, 0xFF_00_00_00);
        fb.set_pixel(0, 2, 0xFF_00_00_00);

        // Col 1,2: White
        fb.set_pixel(1, 0, 0xFF_FF_FF_FF);
        fb.set_pixel(1, 1, 0xFF_FF_FF_FF);
        fb.set_pixel(1, 2, 0xFF_FF_FF_FF);
        fb.set_pixel(2, 0, 0xFF_FF_FF_FF);
        fb.set_pixel(2, 1, 0xFF_FF_FF_FF);
        fb.set_pixel(2, 2, 0xFF_FF_FF_FF);

        let config = EdgeGlowConfig {
            edge_color: 0x00_FF_00_00, // Pure Red glow
            intensity: 1.0,
            edge_threshold: 10,
            darken_factor: 0.0, // Darken everything else to pure black
        };

        apply_edge_glow(&mut fb, &config);

        // The edge should be detected at x=1 (because of the transition 0 -> 255)
        let center_pixel = fb.get_pixel(1, 1).unwrap();

        // It should have some red component because it's an edge
        let r = (center_pixel >> 16) & 0xFF;
        assert!(r > 0, "Edge pixel should have glow color applied");

        // The rightmost column should be completely darkened
        let right_pixel = fb.get_pixel(2, 1).unwrap();
        let right_r = (right_pixel >> 16) & 0xFF;
        let right_g = (right_pixel >> 8) & 0xFF;
        let right_b = right_pixel & 0xFF;
        assert_eq!(right_r, 0, "Non-edge pixel should be darkened");
        assert_eq!(right_g, 0, "Non-edge pixel should be darkened");
        assert_eq!(right_b, 0, "Non-edge pixel should be darkened");
    }
}
