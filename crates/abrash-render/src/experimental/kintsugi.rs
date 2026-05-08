//! Kintsugi Filter
//!
//! A post-processing effect that simulates the Japanese art of Kintsugi
//! (repairing broken pottery with gold). It works by using edge detection
//! to find "cracks" and rendering them in shimmering gold, while optionally
//! desaturating the rest of the image.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Kintsugi effect.
#[derive(Debug, Clone, Copy)]
pub struct KintsugiConfig {
    /// Threshold for edge detection (0-255 * 8 roughly).
    pub edge_threshold: u32,
    /// Base color of the gold repair.
    pub gold_color: u32,
    /// Whether to desaturate the non-edge areas.
    pub desaturate: bool,
}

impl Default for KintsugiConfig {
    fn default() -> Self {
        Self {
            edge_threshold: 120,
            gold_color: 0xFF_D4_AF_37, // Metallic gold
            desaturate: true,
        }
    }
}

/// Applies the Kintsugi effect to the framebuffer.
pub fn apply_kintsugi(fb: &mut Framebuffer, config: &KintsugiConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    // Use a thread-local static buffer to avoid per-frame allocation
    thread_local! {
        static SOURCE_PIXELS: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    let mut src_pixels = SOURCE_PIXELS.with(std::cell::RefCell::take);
    src_pixels.clear();
    src_pixels.extend_from_slice(fb.as_slice());
    let source_buffer = src_pixels.as_slice();

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let iter = pixels.chunks_exact_mut(width).enumerate();

    let gold_r = (config.gold_color >> 16) & 0xFF;
    let gold_g = (config.gold_color >> 8) & 0xFF;
    let gold_b = config.gold_color & 0xFF;

    iter.for_each(|(y, row)| {
        for (x, pixel) in row.iter_mut().enumerate() {
            if x == 0 || y == 0 || x >= width - 1 || y >= height - 1 {
                if config.desaturate {
                    let l = u32::from(pixel_luminance(*pixel));
                    *pixel = 0xFF00_0000 | (l << 16) | (l << 8) | l;
                }
                continue;
            }

            let idx = y * width + x;

            let tl = u32::from(pixel_luminance(unsafe { *source_buffer.get_unchecked(idx - width - 1) }));
            let tc = u32::from(pixel_luminance(unsafe { *source_buffer.get_unchecked(idx - width) }));
            let tr = u32::from(pixel_luminance(unsafe { *source_buffer.get_unchecked(idx - width + 1) }));
            let cl = u32::from(pixel_luminance(unsafe { *source_buffer.get_unchecked(idx - 1) }));
            let cr = u32::from(pixel_luminance(unsafe { *source_buffer.get_unchecked(idx + 1) }));
            let bl = u32::from(pixel_luminance(unsafe { *source_buffer.get_unchecked(idx + width - 1) }));
            let bc = u32::from(pixel_luminance(unsafe { *source_buffer.get_unchecked(idx + width) }));
            let br = u32::from(pixel_luminance(unsafe { *source_buffer.get_unchecked(idx + width + 1) }));

            // Sobel X
            let gx = (tl as i32 + 2 * cl as i32 + bl as i32)
                - (tr as i32 + 2 * cr as i32 + br as i32);

            // Sobel Y
            let gy = (tl as i32 + 2 * tc as i32 + tr as i32)
                - (bl as i32 + 2 * bc as i32 + br as i32);

            // Approximate gradient magnitude
            let magnitude = (gx.abs() + gy.abs()) as u32;

            if magnitude > config.edge_threshold {
                // Procedural shimmer based on position
                let shimmer = ((x * 17 + y * 31) % 32) as u32;

                // Add some brightness variation to the gold
                let mut r = gold_r.saturating_add(shimmer);
                let mut g = gold_g.saturating_add(shimmer);
                let mut b = gold_b.saturating_add(shimmer);

                // Prevent overflow
                r = r.min(255);
                g = g.min(255);
                b = b.min(255);

                *pixel = 0xFF00_0000 | (r << 16) | (g << 8) | b;
            } else if config.desaturate {
                let l = u32::from(pixel_luminance(unsafe { *source_buffer.get_unchecked(idx) }));
                // Mix in a bit of the original color so it's not purely grayscale
                let orig = unsafe { *source_buffer.get_unchecked(idx) };
                let orig_r = (orig >> 16) & 0xFF;
                let orig_g = (orig >> 8) & 0xFF;
                let orig_b = orig & 0xFF;

                let r = (l * 3 + orig_r) / 4;
                let g = (l * 3 + orig_g) / 4;
                let b = (l * 3 + orig_b) / 4;

                *pixel = 0xFF00_0000 | (r << 16) | (g << 8) | b;
            }
        }
    });

    SOURCE_PIXELS.with(|buf| {
        buf.replace(src_pixels);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kintsugi_config_default() {
        let config = KintsugiConfig::default();
        assert_eq!(config.edge_threshold, 120);
        assert_eq!(config.gold_color, 0xFF_D4_AF_37);
        assert!(config.desaturate);
    }

    #[test]
    fn test_apply_kintsugi_edge() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        // White background
        fb.clear(0xFF_FF_FF_FF);

        // Draw a black square to create strong edges
        for y in 3..7 {
            for x in 3..7 {
                fb.set_pixel(x as i32, y as i32, 0xFF_00_00_00);
            }
        }

        let config = KintsugiConfig {
            edge_threshold: 50,
            gold_color: 0xFF_D4_AF_37,
            desaturate: false,
        };

        apply_kintsugi(&mut fb, &config);

        // Edge pixel should be some gold variant. Since there's shimmer, it won't be exactly gold_color,
        // but it will be a goldish color (not black or white).
        let edge_pixel = fb.get_pixel(3, 3).unwrap();

        // The shimmer adds up to 31, let's just check it's not black or white.
        assert_ne!(edge_pixel, 0xFF_00_00_00);
        assert_ne!(edge_pixel, 0xFF_FF_FF_FF);
    }

    #[test]
    fn test_apply_kintsugi_desaturate() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFF_FF_00_00); // Red

        let config = KintsugiConfig {
            edge_threshold: 200,
            gold_color: 0xFF_D4_AF_37,
            desaturate: true,
        };

        apply_kintsugi(&mut fb, &config);

        // Non-edge pixel should be desaturated. Red (255, 0, 0) -> Luminance ~76
        // Mix 76 (3 parts) and orig (1 part). R: (76*3+255)/4 = 120. G, B: (76*3+0)/4 = 57.
        // Let's just check that R, G, B are closer to each other than pure red.
        let pixel = fb.get_pixel(5, 5).unwrap();
        let r = (pixel >> 16) & 0xFF;
        let g = (pixel >> 8) & 0xFF;
        let b = pixel & 0xFF;

        assert!(r < 255);
        assert!(g > 0);
        assert!(b > 0);
    }
}
