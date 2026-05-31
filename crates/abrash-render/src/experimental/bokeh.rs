//! Bokeh Blur Filter.
//!
//! A post-processing effect that simulates out-of-focus areas of an image,
//! mimicking the depth-of-field effect in photography with a circular aperture.

use abrash_core::framebuffer::Framebuffer;
#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Bokeh effect.
#[derive(Debug, Clone, Copy)]
pub struct BokehConfig {
    /// Radius of the blur.
    pub radius: f32,
    /// Threshold to trigger bokeh highlights (0.0 to 1.0).
    pub threshold: f32,
    /// Multiplier for the highlight intensity.
    pub intensity: f32,
}

impl Default for BokehConfig {
    fn default() -> Self {
        Self {
            radius: 5.0,
            threshold: 0.8,
            intensity: 1.5,
        }
    }
}

/// Applies a Bokeh blur effect to the framebuffer in-place.
pub fn apply_bokeh(fb: &mut Framebuffer, config: &BokehConfig) {
    let width = fb.width() as i32;
    let height = fb.height() as i32;
    let radius_sq = config.radius * config.radius;
    let radius_i = config.radius.ceil() as i32;
    let threshold_rgb = (config.threshold * 255.0) as u32;

    let original_fb = fb.as_slice().to_vec();

    let process_row = |y: i32, row: &mut [u32]| {
        for (x_usize, pixel_out) in row.iter_mut().enumerate() {
            let x = x_usize as i32;
            let mut r_sum = 0.0;
            let mut g_sum = 0.0;
            let mut b_sum = 0.0;
            let mut weight_sum = 0.0;

            for dy in -radius_i..=radius_i {
                for dx in -radius_i..=radius_i {
                    let dist_sq = (dx * dx + dy * dy) as f32;
                    if dist_sq <= radius_sq {
                        let nx = x + dx;
                        let ny = y + dy;

                        if nx >= 0 && nx < width && ny >= 0 && ny < height {
                            let idx = (ny * width + nx) as usize;
                            let pixel = original_fb[idx];
                            let r = (pixel >> 16) & 0xFF;
                            let g = (pixel >> 8) & 0xFF;
                            let b = pixel & 0xFF;

                            // Calculate luminance to determine highlight
                            let luminance = (r * 77 + g * 150 + b * 29) >> 8;
                            let mut weight = 1.0;

                            if luminance > threshold_rgb {
                                weight += config.intensity * ((luminance - threshold_rgb) as f32 / 255.0);
                            }

                            r_sum += r as f32 * weight;
                            g_sum += g as f32 * weight;
                            b_sum += b as f32 * weight;
                            weight_sum += weight;
                        }
                    }
                }
            }

            if weight_sum > 0.0 {
                let idx = (y * width + x) as usize;
                let current_pixel = original_fb[idx];
                let a = (current_pixel >> 24) & 0xFF;
                let r_out = (r_sum / weight_sum).clamp(0.0, 255.0) as u32;
                let g_out = (g_sum / weight_sum).clamp(0.0, 255.0) as u32;
                let b_out = (b_sum / weight_sum).clamp(0.0, 255.0) as u32;

                *pixel_out = (a << 24) | (r_out << 16) | (g_out << 8) | b_out;
            }
        }
    };

    #[cfg(feature = "parallel")]
    {
        let chunks = fb.as_mut_slice().chunks_exact_mut(width as usize);
        chunks.enumerate().par_bridge().for_each(|(y, row)| {
            process_row(y as i32, row);
        });
    }

    #[cfg(not(feature = "parallel"))]
    {
        let chunks = fb.as_mut_slice().chunks_exact_mut(width as usize);
        chunks.enumerate().for_each(|(y, row)| {
            process_row(y as i32, row);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_bokeh_changes_buffer() {
        let mut fb = Framebuffer::new(32, 32).unwrap();
        // Draw a bright dot
        fb.set_pixel(16, 16, 0xFF_FF_FF_FF);

        let fb_clone_slice = fb.as_slice().to_vec();

        let config = BokehConfig {
            radius: 5.0,
            threshold: 0.5,
            intensity: 2.0,
        };

        apply_bokeh(&mut fb, &config);

        // Verify the buffer changed
        assert_ne!(fb.as_slice(), fb_clone_slice.as_slice(), "Buffer should be modified by the bokeh effect");

        // Ensure the original pixel isn't just bright, it spread
        let mut found_blur = false;
        for y in 14..=18 {
            for x in 14..=18 {
                if (x != 16 || y != 16) && fb.get_pixel(x, y) != Some(0xFF_00_00_00) {
                    found_blur = true;
                }
            }
        }
        assert!(found_blur, "Bokeh effect did not spread the bright pixel");
    }
}
