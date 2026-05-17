//! Tiny Planet (Stereographic / Polar Mapping) Filter
//!
//! A post-processing effect that maps a standard Cartesian framebuffer
//! (typically containing a panoramic scene or landscape) into a "Tiny Planet"
//! by converting screen coordinates to polar coordinates and sampling the original image.

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

thread_local! {
    static SOURCE_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the Tiny Planet filter.
#[derive(Debug, Clone, Copy)]
pub struct TinyPlanetConfig {
    /// Zoom factor. Higher values make the planet smaller.
    pub zoom: f32,
    /// Rotation of the planet in radians.
    pub rotation: f32,
    /// Offset for the horizon mapping (typically 0.0).
    pub horizon_offset: f32,
    /// Background color for areas outside the planet's mapping.
    pub sky_color: u32,
}

impl Default for TinyPlanetConfig {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            rotation: 0.0,
            horizon_offset: 0.0,
            sky_color: 0xFF_000000,
        }
    }
}

/// Applies a Tiny Planet (stereographic) effect to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - Configuration parameters.
pub fn apply_tiny_planet(fb: &mut Framebuffer, config: &TinyPlanetConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    // Clone the source framebuffer to avoid read-after-write aliasing
    SOURCE_BUFFER.with(|buffer_ref| {
        let mut buffer = buffer_ref.borrow_mut();
        buffer.resize(width * height, 0);
        buffer.copy_from_slice(fb.as_slice());
        let src_pixels = buffer.as_slice();

        let center_x = width as f32 / 2.0;
        let center_y = height as f32 / 2.0;

        // Let the maximum radius be half the height (or width, usually height makes a nice planet).
        let max_radius = (height as f32 / 2.0) / config.zoom;
        let pi2 = 2.0 * std::f32::consts::PI;

        let pixels = fb.as_mut_slice();

        #[cfg(feature = "parallel")]
        {
            pixels
                .par_chunks_exact_mut(width)
                .enumerate()
                .for_each(|(y, row)| {
                    for (x, pixel) in row.iter_mut().enumerate() {
                        let dx = x as f32 - center_x;
                        let dy = y as f32 - center_y;

                        let r = dx.hypot(dy);
                        let mut theta = dy.atan2(dx) + config.rotation;

                        // Normalize theta to [0, 2PI)
                        if theta < 0.0 {
                            theta += pi2;
                        }
                        if theta >= pi2 {
                            theta -= pi2;
                        }

                        let u = theta / pi2;
                        let v = 1.0 - (r / max_radius) + config.horizon_offset;

                        if v < 0.0 || v >= 1.0 {
                            *pixel = config.sky_color;
                        } else {
                            let src_x = (u * width as f32) as usize;
                            let src_y = (v * height as f32) as usize;
                            let src_x = src_x.clamp(0, width - 1);
                            let src_y = src_y.clamp(0, height - 1);
                            *pixel = src_pixels[src_y * width + src_x];
                        }
                    }
                });
        }

        #[cfg(not(feature = "parallel"))]
        {
            for y in 0..height {
                let row_start = y * width;
                for x in 0..width {
                    let dx = x as f32 - center_x;
                    let dy = y as f32 - center_y;

                    let r = dx.hypot(dy);
                    let mut theta = (dy.atan2(dx) + config.rotation) % pi2;

                    if theta < 0.0 {
                        theta += pi2;
                    }

                    let u = theta / pi2;
                    let v = 1.0 - (r / max_radius) + config.horizon_offset;

                    if v < 0.0 || v >= 1.0 {
                        pixels[row_start + x] = config.sky_color;
                    } else {
                        let src_x = (u * width as f32) as usize;
                        let src_y = (v * height as f32) as usize;
                        let src_x = src_x.clamp(0, width - 1);
                        let src_y = src_y.clamp(0, height - 1);
                        pixels[row_start + x] = src_pixels[src_y * width + src_x];
                    }
                }
            }
        }
    });
}
