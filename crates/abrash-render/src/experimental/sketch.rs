//! Sketch Filter
//!
//! A post-processing effect that converts the framebuffer into a pencil sketch look,
//! extracting edges and applying a noise/crosshatch pass on an inverted background.

use crate::framebuffer::Framebuffer;
use crate::post_process::filters::apply_sobel;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Sketch effect.
#[derive(Debug, Clone, Copy)]
pub struct SketchConfig {
    /// Intensity of the sketch effect.
    pub intensity: f32,
    /// Whether to apply crosshatching noise.
    pub noise: bool,
    /// Seed for the crosshatch pattern.
    pub seed: u32,
}

impl Default for SketchConfig {
    fn default() -> Self {
        Self {
            intensity: 1.0,
            noise: true,
            seed: 0x1337_CAFE,
        }
    }
}

/// Applies a pencil sketch effect to the framebuffer.
pub fn apply_sketch(fb: &mut Framebuffer, config: &SketchConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let len = width * height;

    if width == 0 || height == 0 {
        return;
    }

    // 1. Copy source to our thread_local buffer to prevent read/write tearing and reallocations
    // We must use a separate Framebuffer instead of just pixels because `apply_sobel` takes a Framebuffer.
    // To avoid allocations, we will implement this carefully:
    // Framebuffer doesn't expose a `from_vec` or `from_slice`, so we must use `Framebuffer::new`.
    // However, we can cache the entire Framebuffer in thread_local!

    thread_local! {
        static TEMP_FB: RefCell<Option<Framebuffer>> = const { RefCell::new(None) };
    }

    TEMP_FB.with(|buf| {
        let mut temp_fb_opt = buf.borrow_mut();

        // Resize or create the cached framebuffer
        let needs_new = match &*temp_fb_opt {
            Some(tfb) => tfb.width() != width as u32 || tfb.height() != height as u32,
            None => true,
        };

        if needs_new {
            *temp_fb_opt = Some(Framebuffer::new(width as u32, height as u32).unwrap());
        }

        let temp_fb = temp_fb_opt.as_mut().unwrap();

        // Copy the pixels
        temp_fb.as_mut_slice().copy_from_slice(fb.as_slice());

        // Apply sobel to get edges
        apply_sobel(temp_fb);

        let sobel_pixels = temp_fb.as_slice();
        let dest_pixels = fb.as_mut_slice();
        let intensity_factor = (config.intensity.clamp(0.0, 5.0) * 256.0) as u32;

        let seed = config.seed;
        let noise_enabled = config.noise;

        #[cfg(feature = "parallel")]
        let iter = dest_pixels.par_chunks_exact_mut(width).enumerate();
        #[cfg(not(feature = "parallel"))]
        let iter = dest_pixels.chunks_exact_mut(width).enumerate();

        iter.for_each(|(y, row)| {
            let row_offset = y * width;
            let sobel_row = &sobel_pixels[row_offset..row_offset + width];

            // Re-seed per row for better parallel determinism and speed
            let mut lcg = seed.wrapping_add((y as u32).wrapping_mul(1337));

            for (dest_pixel, &sobel_pixel) in row.iter_mut().zip(sobel_row) {
                // Invert sobel. Sobel is white on edges, black elsewhere. We want black on edges, white elsewhere.
                let sobel_r = (sobel_pixel >> 16) & 0xFF;

                // For a pencil sketch, let's invert it:
                let edge_val = 255u32.saturating_sub(sobel_r);

                // Apply intensity using shift
                let mut out_val = 255u32
                    .saturating_sub(((255u32.saturating_sub(edge_val)) * intensity_factor) >> 8);

                if noise_enabled {
                    lcg ^= lcg << 13;
                    lcg ^= lcg >> 17;
                    lcg ^= lcg << 5;

                    let noise = lcg & 0xFF;

                    // Only apply noise on non-edge areas, or gently on edges
                    if out_val > 200 {
                        let noise_blend = (noise * 30) >> 8;
                        out_val = out_val.saturating_sub(noise_blend);
                    }
                }

                *dest_pixel = 0xFF00_0000 | (out_val << 16) | (out_val << 8) | out_val;
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_apply_sketch() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF00_0000);

        // Draw a white rectangle
        for y in 40..60 {
            for x in 40..60 {
                fb.set_pixel(x as i32, y as i32, 0xFFFF_FFFF);
            }
        }

        let config = SketchConfig::default();
        apply_sketch(&mut fb, &config);

        // Edge pixels should be dark, flat regions should be white/light (plus noise)
        let center_pixel = fb.get_pixel(50, 50).unwrap();
        let edge_pixel = fb.get_pixel(40, 40).unwrap();

        // White areas with no edges will be mostly white minus a bit of noise
        let center_r = (center_pixel >> 16) & 0xFF;
        assert!(center_r > 200, "Flat areas should remain light (was {})", center_r);

        // Edges should be darker
        let edge_r = (edge_pixel >> 16) & 0xFF;
        assert!(edge_r < 200, "Edge areas should be darkened (was {})", edge_r);
    }
}
