//! Kintsugi Post-Processing Filter
//!
//! Simulates the Japanese art of repairing broken pottery with gold.
//! It applies a Sobel edge detection to find strong features and fills them
//! with a bright gold color, while desaturating the background to simulate
//! restored art pieces.

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

thread_local! {
    static KINTSUGI_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the Kintsugi effect.
#[derive(Debug, Clone, Copy)]
pub struct KintsugiConfig {
    /// The color of the "gold" veins used to repair cracks.
    pub gold_color: u32,
    /// The threshold for edge detection (0.0 to 1.0).
    pub edge_threshold: f32,
    /// How much to desaturate the non-edge background (0.0 = original, 1.0 = grayscale).
    pub desaturation: f32,
}

impl Default for KintsugiConfig {
    fn default() -> Self {
        Self {
            gold_color: 0xFF_D4_AF_37, // Metallic Gold
            edge_threshold: 0.15,
            desaturation: 0.8,
        }
    }
}

/// Applies a Kintsugi effect to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - Configuration for the effect.
pub fn apply_kintsugi(fb: &mut Framebuffer, config: &KintsugiConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    KINTSUGI_BUFFER.with(|buf| {
        let mut src_fb_vec = buf.borrow_mut();
        let size = width * height;
        if src_fb_vec.len() < size {
            src_fb_vec.resize(size, 0);
        }

        let src_pixels = &mut src_fb_vec[..size];
        src_pixels.copy_from_slice(fb.as_slice());

        let dest_pixels = fb.as_mut_slice();

        let threshold_sq = (config.edge_threshold * 255.0 * 8.0).powi(2);

        #[cfg(feature = "parallel")]
        let iter = dest_pixels.par_chunks_exact_mut(width).enumerate();
        #[cfg(not(feature = "parallel"))]
        let iter = dest_pixels.chunks_exact_mut(width).enumerate();

        iter.for_each(|(y, row)| {
            for (x, pixel) in row.iter_mut().enumerate() {
                if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
                    let original = src_pixels[y * width + x];
                    let luma = get_luminance(original);

                    let r = ((original >> 16) & 0xFF) as f32;
                    let g = ((original >> 8) & 0xFF) as f32;
                    let b = (original & 0xFF) as f32;

                    let blended_r = r * (1.0 - config.desaturation) + luma * config.desaturation;
                    let blended_g = g * (1.0 - config.desaturation) + luma * config.desaturation;
                    let blended_b = b * (1.0 - config.desaturation) + luma * config.desaturation;

                    *pixel = 0xFF00_0000
                        | ((blended_r as u32) << 16)
                        | ((blended_g as u32) << 8)
                        | (blended_b as u32);
                    continue;
                }

                // Sobel kernels for X and Y
                let p00 = get_luminance(src_pixels[(y - 1) * width + (x - 1)]);
                let p10 = get_luminance(src_pixels[(y - 1) * width + x]);
                let p20 = get_luminance(src_pixels[(y - 1) * width + (x + 1)]);

                let p01 = get_luminance(src_pixels[y * width + (x - 1)]);
                let p21 = get_luminance(src_pixels[y * width + (x + 1)]);

                let p02 = get_luminance(src_pixels[(y + 1) * width + (x - 1)]);
                let p12 = get_luminance(src_pixels[(y + 1) * width + x]);
                let p22 = get_luminance(src_pixels[(y + 1) * width + (x + 1)]);

                let gx = (p20 + 2.0 * p21 + p22) - (p00 + 2.0 * p01 + p02);
                let gy = (p02 + 2.0 * p12 + p22) - (p00 + 2.0 * p10 + p20);

                let mag_sq = gx * gx + gy * gy;

                if mag_sq > threshold_sq {
                    *pixel = config.gold_color;
                } else {
                    let original = src_pixels[y * width + x];
                    let luma = get_luminance(original);

                    let r = ((original >> 16) & 0xFF) as f32;
                    let g = ((original >> 8) & 0xFF) as f32;
                    let b = (original & 0xFF) as f32;

                    let blended_r = r * (1.0 - config.desaturation) + luma * config.desaturation;
                    let blended_g = g * (1.0 - config.desaturation) + luma * config.desaturation;
                    let blended_b = b * (1.0 - config.desaturation) + luma * config.desaturation;

                    *pixel = 0xFF00_0000
                        | ((blended_r as u32) << 16)
                        | ((blended_g as u32) << 8)
                        | (blended_b as u32);
                }
            }
        });
    });
}

#[inline(always)]
fn get_luminance(color: u32) -> f32 {
    let r = ((color >> 16) & 0xFF) as f32;
    let g = ((color >> 8) & 0xFF) as f32;
    let b = (color & 0xFF) as f32;
    0.299 * r + 0.587 * g + 0.114 * b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kintsugi_edge_detection() {
        let mut fb = Framebuffer::new(4, 4).unwrap();
        fb.clear(0xFF_00_00_00); // Black background

        // Draw a 2x2 white square in the center to create edges
        fb.set_pixel(1, 1, 0xFF_FF_FF_FF);
        fb.set_pixel(2, 1, 0xFF_FF_FF_FF);
        fb.set_pixel(1, 2, 0xFF_FF_FF_FF);
        fb.set_pixel(2, 2, 0xFF_FF_FF_FF);

        let config = KintsugiConfig {
            gold_color: 0xFF_D4_AF_37,
            edge_threshold: 0.1,
            desaturation: 1.0,
        };

        apply_kintsugi(&mut fb, &config);

        // The corner of the white square (1, 1) should be gold due to the strong edge
        assert_eq!(fb.get_pixel(1, 1).unwrap(), 0xFF_D4_AF_37);
    }

    #[test]
    fn test_kintsugi_desaturation() {
        let mut fb = Framebuffer::new(3, 3).unwrap();
        // Fill with solid red, no edges will be detected
        fb.clear(0xFF_FF_00_00);

        let config = KintsugiConfig {
            gold_color: 0xFF_D4_AF_37,
            edge_threshold: 0.9,
            desaturation: 1.0,
        };

        apply_kintsugi(&mut fb, &config);

        // Since it's pure red and desaturated, it should be a shade of gray based on luminance
        let center = fb.get_pixel(1, 1).unwrap();
        let r = (center >> 16) & 0xFF;
        let g = (center >> 8) & 0xFF;
        let b = center & 0xFF;

        assert_eq!(r, g);
        assert_eq!(g, b);
        assert!(r > 0 && r < 255); // Red's luminance is approx 76
    }

    #[test]
    fn test_kintsugi_zero_size() {
        let mut fb = Framebuffer::new(0, 0).unwrap();
        let config = KintsugiConfig::default();
        apply_kintsugi(&mut fb, &config);
        assert_eq!(fb.width(), 0);
    }
}
