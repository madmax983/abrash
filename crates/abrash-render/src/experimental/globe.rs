//! Globe / Sphere Mapping Post-Processing Filter
//!
//! A retro demoscene effect that maps the 2D framebuffer onto a rotating
//! 3D sphere, complete with shading for a 3D marble look.

use crate::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Globe effect.
#[derive(Debug, Clone, Copy)]
pub struct GlobeConfig {
    /// The normalized radius of the globe relative to the shortest dimension (0.0 to 1.0).
    pub radius: f32,
    /// Rotation around the Y axis (longitude) in radians.
    pub rotation_y: f32,
    /// Rotation around the X axis (latitude) in radians.
    pub rotation_x: f32,
    /// Background color to draw outside the globe.
    pub background_color: u32,
    /// Amount of ambient lighting applied (0.0 to 1.0).
    pub ambient_light: f32,
    /// Direction of the light source (x, y, z).
    pub light_dir: (f32, f32, f32),
}

impl Default for GlobeConfig {
    fn default() -> Self {
        Self {
            radius: 0.8,
            rotation_y: 0.0,
            rotation_x: 0.0,
            background_color: 0xFF_000000,
            ambient_light: 0.2,
            light_dir: (0.577, 0.577, 0.577), // Normalized (1, 1, 1)
        }
    }
}

/// Applies a Globe mapping effect to the framebuffer.
///
/// This effect simulates inverse spherical projection, mapping each pixel within
/// the globe's radius back to a latitude/longitude coordinate on the original image.
///
/// * `fb`: The Framebuffer to modify.
/// * `config`: The configuration for the globe effect.
pub fn apply_globe(fb: &mut Framebuffer, config: &GlobeConfig) {
    if config.radius <= 0.0 {
        fb.clear(config.background_color);
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;
    if width == 0 || height == 0 {
        return;
    }

    let half_w = width as f32 / 2.0;
    let half_h = height as f32 / 2.0;

    // Radius in pixels based on the smallest dimension
    let min_dim = half_w.min(half_h);
    let r_pixels = min_dim * config.radius;
    let r_sq = r_pixels * r_pixels;

    // Precalculate lighting normalization
    let len = (config.light_dir.0 * config.light_dir.0
        + config.light_dir.1 * config.light_dir.1
        + config.light_dir.2 * config.light_dir.2)
        .sqrt();
    let l_dir = if len > 0.0 {
        (
            config.light_dir.0 / len,
            config.light_dir.1 / len,
            config.light_dir.2 / len,
        )
    } else {
        (0.0, 0.0, 1.0)
    };

    // Bolt Performance Optimization:
    // Caching the source buffer in a thread_local prevents per-frame Vec heap allocation.
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
            let dy_sq = dy * dy;

            for (x, pixel) in row.iter_mut().enumerate().take(width) {
                let dx = (x as f32) - half_w;
                let dx_sq = dx * dx;

                let dist_sq = dx_sq + dy_sq;

                if dist_sq <= r_sq {
                    // Inside the sphere. Calculate z based on the sphere equation: x^2 + y^2 + z^2 = r^2
                    let z = (r_sq - dist_sq).sqrt();

                    // Calculate the normal vector at this point on the sphere surface
                    let nx = dx / r_pixels;
                    let ny = dy / r_pixels;
                    let nz = z / r_pixels;

                    // Calculate lighting (Lambertian reflectance)
                    let dot = (nx * l_dir.0 + ny * l_dir.1 + nz * l_dir.2).max(0.0);
                    let light_intensity = config.ambient_light + (1.0 - config.ambient_light) * dot;

                    // Inverse spherical projection to find original texture coordinates
                    // Normalization logic mapping spherical normal to UV coordinates
                    let u = 0.5 + (nz.atan2(nx) + config.rotation_y) / (2.0 * std::f32::consts::PI);
                    let v = 0.5 - (ny.asin() + config.rotation_x) / std::f32::consts::PI;

                    // Wrap UVs
                    let u = u.fract();
                    let v = v.fract();

                    let u = if u < 0.0 { u + 1.0 } else { u };
                    let v = if v < 0.0 { v + 1.0 } else { v };

                    let src_x = ((u * (width as f32)) as usize).min(width - 1);
                    let src_y = ((v * (height as f32)) as usize).min(height - 1);

                    let src_idx = src_y * width + src_x;
                    let src_color = src_pixels_slice[src_idx];

                    // Apply lighting
                    let a = (src_color >> 24) & 0xFF;
                    let mut r = (src_color >> 16) & 0xFF;
                    let mut g = (src_color >> 8) & 0xFF;
                    let mut b = src_color & 0xFF;

                    r = ((r as f32) * light_intensity) as u32;
                    g = ((g as f32) * light_intensity) as u32;
                    b = ((b as f32) * light_intensity) as u32;

                    *pixel = (a << 24) | (r << 16) | (g << 8) | b;
                } else {
                    // Outside the sphere
                    *pixel = config.background_color;
                }
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_globe_zero_radius() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFFFF_FFFF);

        let mut config = GlobeConfig::default();
        config.radius = 0.0;
        config.background_color = 0xFF_000000;

        apply_globe(&mut fb, &config);

        for &pixel in fb.as_slice() {
            assert_eq!(pixel, 0xFF_000000);
        }
    }

    #[test]
    fn test_globe_mapping() {
        let mut fb = Framebuffer::new(20, 20).unwrap();
        fb.clear(0xFFFF_FFFF);

        // Draw a black dot in the center
        fb.set_pixel(10, 10, 0xFF_000000);

        let config = GlobeConfig::default();
        apply_globe(&mut fb, &config);

        // Outside should be black
        assert_eq!(fb.get_pixel(0, 0), Some(0xFF_000000));

        // Inside should have mapped some pixels (not crashing)
        let has_mapped = fb.as_slice().iter().any(|&p| p != 0xFF_000000);
        assert!(has_mapped);
    }
}
