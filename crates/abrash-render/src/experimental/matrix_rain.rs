//! Matrix Digital Rain Filter
//!
//! A retro post-processing effect simulating falling digital code, where
//! underlying scene luminance drives the brightness of the code streams.

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

thread_local! {
    static MATRIX_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the Matrix Rain filter.
#[derive(Debug, Clone)]
pub struct MatrixRainConfig {
    /// The current time of the simulation.
    pub time: f32,
    /// Speed multiplier for the falling rain.
    pub speed: f32,
    /// Density of the rain streams (0.0 to 1.0).
    pub density: f32,
    /// Random offsets for each column.
    offsets: Vec<f32>,
    /// Fall speeds for each column.
    speeds: Vec<f32>,
    /// Density map for columns.
    columns: Vec<bool>,
    /// Whether state is initialized for the current width.
    initialized_width: usize,
}

impl Default for MatrixRainConfig {
    fn default() -> Self {
        Self {
            time: 0.0,
            speed: 15.0,
            density: 0.2,
            offsets: Vec::new(),
            speeds: Vec::new(),
            columns: Vec::new(),
            initialized_width: 0,
        }
    }
}

/// Applies the Matrix Rain effect.
pub fn apply_matrix_rain(fb: &mut Framebuffer, config: &mut MatrixRainConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    if config.initialized_width != width {
        config.offsets.resize(width, 0.0);
        config.speeds.resize(width, 0.0);
        config.columns.resize(width, false);

        for x in 0..width {
            let x_f = x as f32;
            let rand_val1 = (crate::math::fast_sin(x_f * 12.9898) * 43758.5453)
                .fract()
                .abs();
            let rand_val2 = (crate::math::fast_sin(x_f * 78.233) * 43758.5453)
                .fract()
                .abs();

            config.offsets[x] = rand_val1 * height as f32;
            config.speeds[x] = 0.5 + rand_val2 * 1.5;
            config.columns[x] = rand_val1 < config.density;
        }
        config.initialized_width = width;
    }

    let time = config.time;
    let speed = config.speed;

    MATRIX_BUFFER.with(|buf| {
        let mut src_fb_vec = buf.borrow_mut();
        let size = width * height;
        if src_fb_vec.len() < size {
            src_fb_vec.resize(size, 0);
        }

        let src_fb = &mut src_fb_vec[..size];
        src_fb.copy_from_slice(fb.as_slice());

        let dest_pixels = fb.as_mut_slice();

        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            dest_pixels
                .par_chunks_exact_mut(width)
                .enumerate()
                .for_each(|(y, row)| {
                    for (x, pixel) in row.iter_mut().enumerate() {
                        if !config.columns[x] {
                            continue;
                        }

                        let y_pos = ((time * speed * config.speeds[x] + config.offsets[x])
                            % height as f32) as i32;
                        let dist = (y_pos - y as i32).rem_euclid(height as i32);

                        if dist < 40 {
                            let src_pixel = src_fb[y * width + x];
                            let r = ((src_pixel >> 16) & 0xFF) as f32;
                            let g = ((src_pixel >> 8) & 0xFF) as f32;
                            let b = (src_pixel & 0xFF) as f32;
                            let lum = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;

                            let intensity = (1.0 - (dist as f32 / 40.0)) * lum * 255.0;
                            let green_val = intensity as u32;
                            *pixel = 0xFF00_0000 | (green_val << 8);
                        } else {
                            *pixel = 0xFF00_0000;
                        }
                    }
                });
        }
        #[cfg(not(feature = "parallel"))]
        {
            for y in 0..height {
                for x in 0..width {
                    if !config.columns[x] {
                        continue;
                    }
                    let y_pos = ((time * speed * config.speeds[x] + config.offsets[x])
                        % height as f32) as i32;
                    let dist = (y_pos - y as i32).rem_euclid(height as i32);

                    let idx = y * width + x;
                    if dist < 40 {
                        let src_pixel = src_fb[idx];
                        let r = ((src_pixel >> 16) & 0xFF) as f32;
                        let g = ((src_pixel >> 8) & 0xFF) as f32;
                        let b = (src_pixel & 0xFF) as f32;
                        let lum = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;

                        let intensity = (1.0 - (dist as f32 / 40.0)) * lum * 255.0;
                        let green_val = intensity as u32;
                        dest_pixels[idx] = 0xFF00_0000 | (green_val << 8);
                    } else {
                        dest_pixels[idx] = 0xFF00_0000;
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_matrix_rain() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        let mut config = MatrixRainConfig::default();
        config.density = 1.0;
        apply_matrix_rain(&mut fb, &mut config);
        assert_eq!(config.initialized_width, 10);
    }
}
