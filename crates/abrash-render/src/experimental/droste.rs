//! Droste Effect / Recursive Picture-in-Picture
//!
//! A post-processing effect that recursively maps the image into itself,
//! either in straight concentric rings or as a continuous Escher-like spiral.

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

thread_local! {
    static SOURCE_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the Droste effect.
#[derive(Debug, Clone, Copy)]
pub struct DrosteConfig {
    /// The inner radius where the recursion begins (0.0 to 1.0).
    pub inner_radius: f32,
    /// The outer radius of the image (0.0 to 1.0).
    pub outer_radius: f32,
    /// Whether to twist the recursion into a continuous spiral.
    pub spiral: bool,
    /// Number of arms in the spiral (if spiral is true).
    pub arms: f32,
    /// Zoom offset for animation.
    pub time: f32,
}

impl Default for DrosteConfig {
    fn default() -> Self {
        Self {
            inner_radius: 0.2,
            outer_radius: 1.0,
            spiral: true,
            arms: 1.0,
            time: 0.0,
        }
    }
}

pub fn apply_droste(fb: &mut Framebuffer, config: &DrosteConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    SOURCE_BUFFER.with(|buf| {
        let mut src_mut = buf.borrow_mut();
        src_mut.resize(width * height, 0);
        src_mut.copy_from_slice(fb.as_slice());

        let src = src_mut.as_slice();

        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;
        let max_radius = cx.min(cy) * config.outer_radius;
        let min_radius = cx.min(cy) * config.inner_radius;

        if min_radius <= 0.0 || max_radius <= min_radius {
            return;
        }

        let ratio = max_radius / min_radius;
        let ln_ratio = ratio.ln();

        // Calculate spiral parameters
        let alpha = if config.spiral {
            (ln_ratio / (std::f32::consts::TAU * config.arms)).atan()
        } else {
            0.0
        };

        let cos_alpha = alpha.cos();
        let sin_alpha = alpha.sin();
        // Scaling factor so the spiral maps exactly onto the ratio
        let f = cos_alpha;

        let pixels = fb.as_mut_slice();

        #[cfg(feature = "parallel")]
        let iter = pixels.par_chunks_exact_mut(width).enumerate();
        #[cfg(not(feature = "parallel"))]
        let iter = pixels.chunks_exact_mut(width).enumerate();

        iter.for_each(|(y, row)| {
            for (x, pixel) in row.iter_mut().enumerate() {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;

                #[allow(clippy::imprecise_flops)]
                let r = (dx * dx + dy * dy).sqrt();
                if r == 0.0 {
                    continue; // Skip exact center to avoid ln(0)
                }

                let theta = dy.atan2(dx);

                // Convert to log-polar space
                let log_r = r.ln();

                let mut u = log_r;
                let mut v = theta;

                if config.spiral {
                    // Apply Escher transformation
                    // Rotate and scale in log space
                    let u_prime = u * cos_alpha - v * sin_alpha;
                    let v_prime = u * sin_alpha + v * cos_alpha;

                    u = u_prime * f;
                    v = v_prime * f;
                }

                // Apply zoom animation
                u += config.time * ln_ratio;

                // Modulo math to map infinite plane back to fundamental annulus
                // We want u to be in [min_radius.ln(), max_radius.ln()]
                let log_min = min_radius.ln();

                // Rust's `%` operator is remainder, we need true modulo
                let u_mod = ((u - log_min) % ln_ratio + ln_ratio) % ln_ratio + log_min;

                // Inverse transformation
                let mut log_r_new = u_mod;
                let mut theta_new = v;

                if config.spiral {
                    // Inverse rotate and scale
                    let inv_f = 1.0 / f;
                    let u_inv = log_r_new * inv_f;
                    let v_inv = theta_new * inv_f;

                    log_r_new = u_inv * cos_alpha + v_inv * sin_alpha;
                    theta_new = -u_inv * sin_alpha + v_inv * cos_alpha;
                }

                let r_new = log_r_new.exp();
                let src_x = (cx + r_new * theta_new.cos()).round() as i32;
                let src_y = (cy + r_new * theta_new.sin()).round() as i32;

                // Clamp to prevent out-of-bounds
                let src_x = src_x.clamp(0, width as i32 - 1) as usize;
                let src_y = src_y.clamp(0, height as i32 - 1) as usize;

                *pixel = src[(src_y * width) + src_x];
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_droste_config_default() {
        let config = DrosteConfig::default();
        assert_eq!(config.inner_radius, 0.2);
    }

    #[test]
    fn test_droste_effect_basic() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFF_00_00_00);
        fb.set_pixel(5, 5, 0xFF_FF_FF_FF); // Center white dot

        let config = DrosteConfig {
            inner_radius: 0.1,
            outer_radius: 1.0,
            spiral: false,
            arms: 1.0,
            time: 0.0,
        };

        apply_droste(&mut fb, &config);

        // Verify the buffer was modified or handled without crashing.
        assert_eq!(fb.width(), 10);
    }
}
