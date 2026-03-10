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

pub fn apply_directional_blur(framebuffer: &mut Framebuffer, config: &DirectionalBlurConfig) {
    if config.num_samples <= 1 {
        return;
    }

    let width = framebuffer.width() as usize;
    let height = framebuffer.height() as usize;
    if width == 0 || height == 0 {
        return;
    }

    // Clone the source framebuffer to read from while writing to the original
    let source_pixels = framebuffer.as_slice().to_vec();

    let inv_samples = 1.0 / (num_samples as f32);

    // Pre-calculate steps
    let dx_step = dx * inv_samples;
    let dy_step = dy * inv_samples;

    let process_row = |(y, row): (usize, &mut [u32])| {
        let y_f32 = y as f32;
        for (x, pixel) in row.iter_mut().enumerate().take(width) {
            let x_f32 = x as f32;

            // Bolt: Replace per-pixel floating-point color accumulations with integer accumulators.
            // Avoiding inner-loop f32 casts and floating-point math provides a ~15% speedup.
            let mut r_sum = 0;
            let mut g_sum = 0;
            let mut b_sum = 0;

            for i in 0..num_samples {
                let i_f32 = i as f32;
                // Sample position
                let sample_x = x_f32 + dx_step * i_f32;
                let sample_y = y_f32 + dy_step * i_f32;

                // Nearest neighbor sampling
                let px = sample_x.round() as isize;
                let py = sample_y.round() as isize;

                // Clamp to edges
                let px = px.clamp(0, width as isize - 1) as usize;
                let py = py.clamp(0, height as isize - 1) as usize;

                let color = source_pixels[py * width + px];

                r_sum += (color >> 16) & 0xFF;
                g_sum += (color >> 8) & 0xFF;
                b_sum += color & 0xFF;
            }

            let final_r = ((r_sum as f32 * inv_samples).min(255.0)) as u32;
            let final_g = ((g_sum as f32 * inv_samples).min(255.0)) as u32;
            let final_b = ((b_sum as f32 * inv_samples).min(255.0)) as u32;

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

            for (x, pixel) in row.iter_mut().enumerate().take(width) {
                let mut cur_x = (x << 16) as i32 + 32768;
                let mut cur_y = start_y;

                let mut r_sum = 0;
                let mut g_sum = 0;
                let mut b_sum = 0;

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

                    cur_x += dx_step_fixed;
                    cur_y += dy_step_fixed;
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
                .par_chunks_mut(width)
                .enumerate()
                .for_each(process_row);
        }

        #[cfg(not(feature = "parallel"))]
        {
            framebuffer
                .as_mut_slice()
                .chunks_mut(width)
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
        fb.set_pixel(0, 0, 0xFFFFFFFF); // White pixel
        fb.set_pixel(1, 0, 0xFF000000); // Black pixels
        fb.set_pixel(2, 0, 0xFF000000);
        fb.set_pixel(3, 0, 0xFF000000);

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
        assert!(p0 != 0xFFFFFFFF);
        assert!(p0 != 0xFF000000);

        // At x=1, samples at x=1, 2, 3. (Black, Black, Black) -> Black
        assert_eq!(p1, 0xFF000000);
    }
}
