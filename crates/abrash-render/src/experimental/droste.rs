//! Droste Effect Filter
//!
//! A post-processing effect that creates a recursive "picture-in-picture" look
//! by continuously mapping the edges of the image into a smaller central frame.

use abrash_core::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration parameters for the Droste filter.
#[derive(Debug, Clone, Copy)]
pub struct DrosteConfig {
    /// The scale factor of the inner picture (e.g. 0.5 means the inner picture is half the size).
    pub scale_factor: f32,
    /// The number of recursive iterations.
    pub iterations: u32,
}

impl Default for DrosteConfig {
    fn default() -> Self {
        Self {
            scale_factor: 0.5,
            iterations: 3,
        }
    }
}

/// Applies a Droste effect to the framebuffer.
///
/// Warps the image recursively towards the center.
pub fn apply_droste(fb: &mut Framebuffer, config: &DrosteConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || config.iterations == 0 || config.scale_factor <= 0.0 || config.scale_factor >= 1.0 {
        return;
    }

    let half_w = width as f32 / 2.0;
    let half_h = height as f32 / 2.0;

    // Bolt Performance Optimization:
    // Clone source pixels into a thread_local to prevent per-frame allocations
    // since destination pixels map to arbitrary source pixels.
    thread_local! {
        static SOURCE_PIXELS: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    SOURCE_PIXELS.with(|buf| {
        let mut src_pixels = buf.borrow_mut();
        src_pixels.clear();
        src_pixels.extend_from_slice(fb.as_slice());

        let src_pixels_slice = src_pixels.as_slice();
        let dst_pixels = fb.as_mut_slice();

        #[cfg(feature = "parallel")]
        let row_iter = dst_pixels.par_chunks_exact_mut(width).enumerate();
        #[cfg(not(feature = "parallel"))]
        let row_iter = dst_pixels.chunks_exact_mut(width).enumerate();

        row_iter.for_each(|(y, row)| {
            let dy = (y as f32) - half_h;
            // Map ny to [-1, 1] range
            let ny = dy / half_h;

            for (x, pixel) in row.iter_mut().enumerate().take(width) {
                let dx = (x as f32) - half_w;
                // Map nx to [-1, 1] range
                let nx = dx / half_w;

                // Find the maximum absolute normalized coordinate to determine which "ring" we are in
                let max_abs = nx.abs().max(ny.abs());

                // Determine the recursion depth based on max_abs and scale_factor
                let mut current_scale = 1.0;
                let mut depth = 0;

                while max_abs <= current_scale * config.scale_factor && depth < config.iterations {
                    current_scale *= config.scale_factor;
                    depth += 1;
                }

                // Map the coordinate back to the original image space by unscaling
                let nx_src = nx / current_scale;
                let ny_src = ny / current_scale;

                // Convert back to pixel coordinates
                let src_x = (nx_src * half_w + half_w) as i32;
                let src_y = (ny_src * half_h + half_h) as i32;

                // Check bounds to ensure we sample valid pixels
                if src_x >= 0 && src_x < width as i32 && src_y >= 0 && src_y < height as i32 {
                    let src_idx = (src_y as usize) * width + (src_x as usize);
                    *pixel = src_pixels_slice[src_idx];
                } else {
                    // Out of bounds
                    *pixel = 0xFF00_0000;
                }
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_droste_default_config() {
        let config = DrosteConfig::default();
        assert_eq!(config.scale_factor, 0.5);
        assert_eq!(config.iterations, 3);
    }

    #[test]
    fn test_apply_droste_no_iterations() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFFFFFFFF);

        let config = DrosteConfig { iterations: 0, scale_factor: 0.5 };
        apply_droste(&mut fb, &config);

        for &pixel in fb.as_slice() {
            assert_eq!(pixel, 0xFFFFFFFF);
        }
    }

    #[test]
    fn test_apply_droste_effect() {
        let mut fb = Framebuffer::new(4, 4).unwrap();
        fb.clear(0xFFFFFFFF);
        // Draw a black border
        for x in 0..4 {
            fb.set_pixel(x, 0, 0xFF000000);
            fb.set_pixel(x, 3, 0xFF000000);
            fb.set_pixel(0, x, 0xFF000000);
            fb.set_pixel(3, x, 0xFF000000);
        }

        let config = DrosteConfig { iterations: 1, scale_factor: 0.5 };
        apply_droste(&mut fb, &config);

        // Without panic
        assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF000000);
    }
}
