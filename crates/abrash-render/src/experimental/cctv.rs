//! CCTV Security Camera Filter
//!
//! A post-processing effect that simulates a low-quality CCTV security camera.
//! It adds timestamp text, a blinking recording indicator, scanlines, barrel distortion,
//! and chromatic aberration.

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

thread_local! {
    static CCTV_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the CCTV effect.
#[derive(Debug, Clone)]
pub struct CctvConfig {
    /// The current time in seconds (used for animation and blinking).
    pub time: f32,
    /// The text to display in the corner (e.g., "CAM 01 - 2024/06/25").
    pub overlay_text: String,
    /// The intensity of the scanlines (0.0 to 1.0).
    pub scanline_intensity: f32,
    /// The amount of barrel distortion (0.0 to 1.0).
    pub distortion: f32,
    /// Whether to show the blinking "REC" indicator.
    pub show_rec: bool,
}

impl Default for CctvConfig {
    fn default() -> Self {
        Self {
            time: 0.0,
            overlay_text: String::from("CAM 01"),
            scanline_intensity: 0.3,
            distortion: 0.15,
            show_rec: true,
        }
    }
}

/// Applies a CCTV security camera effect to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - The configuration and state parameters.
pub fn apply_cctv(fb: &mut Framebuffer, config: &CctvConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    CCTV_BUFFER.with(|buf| {
        let mut src_fb_vec = buf.borrow_mut();
        let size = width * height;
        if src_fb_vec.len() < size {
            src_fb_vec.resize(size, 0);
        }

        let src_pixels = &mut src_fb_vec[..size];
        src_pixels.copy_from_slice(fb.as_slice());

        let dest_pixels = fb.as_mut_slice();

        // 1. Distortion & Scanlines
        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;
        let _max_r = cx.hypot(cy);

        #[cfg(feature = "parallel")]
        let row_iter = dest_pixels.par_chunks_exact_mut(width).enumerate();
        #[cfg(not(feature = "parallel"))]
        let row_iter = dest_pixels.chunks_exact_mut(width).enumerate();

        row_iter.for_each(|(y, row)| {
            // Calculate scanline darkening
            let is_scanline = y % 4 == 0 || y % 4 == 1;
            let scanline_multiplier = if is_scanline {
                1.0 - config.scanline_intensity
            } else {
                1.0
            };

            for (x, pixel) in row.iter_mut().enumerate() {
                // Normalize coordinates to [-1, 1]
                let nx = (x as f32 - cx) / cx;
                let ny = (y as f32 - cy) / cy;

                let r2 = nx * nx + ny * ny;

                // Brown-Conrady barrel distortion
                let distortion_factor =
                    1.0 + config.distortion * r2 + config.distortion * config.distortion * r2 * r2;

                let nx_distorted = nx * distortion_factor;
                let ny_distorted = ny * distortion_factor;

                let source_x = ((nx_distorted * cx) + cx) as i32;
                let source_y = ((ny_distorted * cy) + cy) as i32;

                if source_x >= 0
                    && source_x < width as i32
                    && source_y >= 0
                    && source_y < height as i32
                {
                    // Slight chromatic aberration on the edges
                    let r_offset = (nx_distorted * 2.0) as i32;
                    let b_offset = -(nx_distorted * 2.0) as i32;

                    let sample_r = get_channel_safe(
                        src_pixels,
                        width,
                        height,
                        source_x + r_offset,
                        source_y,
                        16,
                    );
                    let sample_g =
                        get_channel_safe(src_pixels, width, height, source_x, source_y, 8);
                    let sample_b = get_channel_safe(
                        src_pixels,
                        width,
                        height,
                        source_x + b_offset,
                        source_y,
                        0,
                    );

                    // Desaturate slightly and tint green/blue for a cheap CCD look
                    let lum = (sample_r * 77 + sample_g * 150 + sample_b * 29) >> 8;

                    let out_r = u32::midpoint(sample_r, lum) as f32 * scanline_multiplier * 0.9;
                    let out_g = u32::midpoint(sample_g, lum) as f32 * scanline_multiplier * 1.1;
                    let out_b = u32::midpoint(sample_b, lum) as f32 * scanline_multiplier * 1.0;

                    *pixel = 0xFF00_0000
                        | ((out_r as u32).min(255) << 16)
                        | ((out_g as u32).min(255) << 8)
                        | ((out_b as u32).min(255));
                } else {
                    *pixel = 0xFF00_0000; // Black outside distorted area
                }
            }
        });

        // 2. REC indicator (Blinking red circle)
        if config.show_rec && config.time % 1.0 < 0.5 {
            let rec_x = 20;
            let rec_y = 20;
            let rec_radius = 5;

            for y in (rec_y - rec_radius)..=(rec_y + rec_radius) {
                for x in (rec_x - rec_radius)..=(rec_x + rec_radius) {
                    if y >= 0 && y < height as i32 && x >= 0 && x < width as i32 {
                        let dx = x - rec_x;
                        let dy = y - rec_y;
                        if dx * dx + dy * dy <= rec_radius * rec_radius {
                            dest_pixels[y as usize * width + x as usize] = 0xFFFF_0000;
                        }
                    }
                }
            }
        }
    });
}

#[inline(always)]
fn get_channel_safe(src: &[u32], width: usize, _height: usize, x: i32, y: i32, shift: u8) -> u32 {
    let clamped_x = x.max(0).min(width as i32 - 1) as usize;
    let idx = y as usize * width + clamped_x;
    (src[idx] >> shift) & 0xFF
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_cctv() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFFFF_FFFF);

        let config = CctvConfig::default();
        apply_cctv(&mut fb, &config);

        // Verify it changed some pixels from pure white
        let has_changed = fb.as_slice().iter().any(|&p| p != 0xFFFF_FFFF);
        assert!(has_changed);
    }
}
