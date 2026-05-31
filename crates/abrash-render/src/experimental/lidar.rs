//! `LiDAR` / Point Cloud Scanner Filter
//!
//! A post-processing effect that simulates a `LiDAR` (Light Detection and Ranging) scanner,
//! converting depth information into discrete point clouds that accumulate over time.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_core::utils::XorShift32;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the `LiDAR` Scanner effect.
#[derive(Debug, Clone, Copy)]
pub struct LidarConfig {
    /// Scanner sweeping line position (0.0 to 1.0) vertically.
    pub scan_y: f32,
    /// Thickness of the scanning beam.
    pub beam_thickness: f32,
    /// Color of the active scanning beam.
    pub beam_color: u32,
    /// Color of the scanned point cloud.
    pub point_color: u32,
    /// Density of points (0.0 to 1.0).
    pub point_density: f32,
    /// Background color.
    pub background_color: u32,
    /// Maximum range of the scanner.
    pub max_depth: f32,
    /// Random seed for noise.
    pub seed: u32,
    /// Amount by which points fade out over time (0 to 255).
    pub fade_rate: u32,
}

impl Default for LidarConfig {
    fn default() -> Self {
        Self {
            scan_y: 0.5,
            beam_thickness: 0.05,
            beam_color: 0xFF_00FF00, // Bright Green
            point_color: 0xFF_00AA00, // Dim Green
            point_density: 0.2,
            background_color: 0xFF_000000,
            max_depth: 50.0,
            seed: 42,
            fade_rate: 2,
        }
    }
}

/// A stateful scanner to hold persistent pixel data over frames without using globals.
pub struct LidarScanner {
    state: Vec<u32>,
}

impl Default for LidarScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl LidarScanner {
    pub fn new() -> Self {
        Self {
            state: Vec::new(),
        }
    }

    /// Applies a `LiDAR` scanner effect to the framebuffer based on depth.
    pub fn apply(&mut self, fb: &mut Framebuffer, zb: &ZBuffer, config: &LidarConfig) {
        let width = fb.width() as usize;
        let height = fb.height() as usize;

        if width == 0 || height == 0 {
            return;
        }

        let size = width * height;
        if self.state.len() != size {
            self.state.resize(size, config.background_color);
            self.state.fill(config.background_color);
        }

        let scan_center = (config.scan_y * height as f32) as i32;
        let scan_half = (config.beam_thickness * height as f32 * 0.5) as i32;

        let scan_top = scan_center - scan_half;
        let scan_bottom = scan_center + scan_half;

        // Convert to slice refs
        let dest_pixels = fb.as_mut_slice();
        let z_pixels = zb.as_slice();
        let state_pixels = &mut self.state;

        #[cfg(feature = "parallel")]
        let iter = dest_pixels
            .par_chunks_exact_mut(width)
            .zip(z_pixels.par_chunks_exact(width))
            .zip(state_pixels.par_chunks_exact_mut(width))
            .enumerate();

        #[cfg(not(feature = "parallel"))]
        let iter = dest_pixels
            .chunks_exact_mut(width)
            .zip(z_pixels.chunks_exact(width))
            .zip(state_pixels.chunks_exact_mut(width))
            .enumerate();

        let global_seed = config.seed;

        // Get target background color channels to interpolate towards
        let bg_r = (config.background_color >> 16) & 0xFF;
        let bg_g = (config.background_color >> 8) & 0xFF;
        let bg_b = config.background_color & 0xFF;

        iter.for_each(|(y, ((out_row, z_row), state_row))| {
            let mut prng_state = global_seed.wrapping_add((y as u32).wrapping_mul(7919));
            if prng_state == 0 { prng_state = 1; }
            let mut prng = XorShift32::new(prng_state);

            let in_beam = y as i32 >= scan_top && y as i32 <= scan_bottom;

            for x in 0..width {
                let depth = z_row[x];
                let mut current_state = state_row[x];

                // Fade existing state slightly towards the background color
                if current_state != config.background_color {
                    let mut r = (current_state >> 16) & 0xFF;
                    let mut g = (current_state >> 8) & 0xFF;
                    let mut b = current_state & 0xFF;

                    if r > bg_r {
                        r = r.saturating_sub(config.fade_rate).max(bg_r);
                    } else if r < bg_r {
                        r = r.saturating_add(config.fade_rate).min(bg_r);
                    }

                    if g > bg_g {
                        g = g.saturating_sub(config.fade_rate).max(bg_g);
                    } else if g < bg_g {
                        g = g.saturating_add(config.fade_rate).min(bg_g);
                    }

                    if b > bg_b {
                        b = b.saturating_sub(config.fade_rate).max(bg_b);
                    } else if b < bg_b {
                        b = b.saturating_add(config.fade_rate).min(bg_b);
                    }

                    if r == bg_r && g == bg_g && b == bg_b {
                        current_state = config.background_color;
                    } else {
                        current_state = 0xFF_000000 | (r << 16) | (g << 8) | b;
                    }
                }

                // If inside scan beam, randomly add new points based on depth map
                if in_beam && depth < config.max_depth && !depth.is_infinite() {
                    let noise = (prng.next_u32() % 100) as f32 / 100.0;

                    // The further away, the less likely to capture a point
                    let distance_factor = 1.0 - (depth / config.max_depth);
                    let chance = config.point_density * distance_factor;

                    if noise < chance {
                        current_state = config.beam_color;
                    } else if current_state == config.beam_color {
                        // Immediately transition beam to point color
                        current_state = config.point_color;
                    }
                } else if current_state == config.beam_color {
                    // Turn beam pixels into point pixels when beam passes
                    current_state = config.point_color;
                }

                state_row[x] = current_state;
                out_row[x] = current_state;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lidar_background_color() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        let zb = ZBuffer::new(10, 10).unwrap();

        let config = LidarConfig {
            scan_y: 2.0, // Beam way off screen
            ..Default::default()
        };

        let mut scanner = LidarScanner::new();
        scanner.apply(&mut fb, &zb, &config);

        for &pixel in fb.as_slice() {
            assert_eq!(pixel, config.background_color);
        }
    }
}
