//! Voronoi Post-Processing Filter
//!
//! Applies a Voronoi diagram effect to the framebuffer based on a set of seed points.
//! This creates a stained-glass or cellular aesthetic.

use crate::framebuffer::Framebuffer;
use crate::math::Vec2;
use crate::utils::XorShift32;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration parameters for the Voronoi filter.
pub struct VoronoiConfig {
    /// Number of random seed points to generate.
    pub num_seeds: u32,
    /// Whether to color cells by the original image color at the seed point.
    /// If false, cells will be colored based on random palette or distance.
    pub use_image_color: bool,
    /// Distance metric exponent (2.0 = Euclidean, 1.0 = Manhattan).
    pub metric: f32,
    /// Seed for random number generator.
    pub seed: u32,
    /// Thickness of the cell borders (0.0 for no borders).
    pub border_thickness: f32,
    /// Color of the cell borders.
    pub border_color: u32,
}

impl Default for VoronoiConfig {
    fn default() -> Self {
        Self {
            num_seeds: 100,
            use_image_color: true,
            metric: 2.0,
            seed: 0,
            border_thickness: 1.0,
            border_color: 0xFF000000,
        }
    }
}

/// Applies a Voronoi filter to the framebuffer.
///
/// This effect calculates the nearest seed point for each pixel and assigns
/// the color of that seed point to the pixel, creating a cellular look.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify.
/// * `config` - Configuration for the Voronoi effect.
pub fn apply_voronoi(fb: &mut Framebuffer, config: &VoronoiConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || config.num_seeds == 0 {
        return;
    }

    // 1. Generate Seed Points
    let mut rng = XorShift32::new(config.seed);
    let mut seeds = Vec::with_capacity(config.num_seeds as usize);
    let mut seed_colors = Vec::with_capacity(config.num_seeds as usize);

    for _ in 0..config.num_seeds {
        let x = (rng.next_u32() as usize) % width;
        let y = (rng.next_u32() as usize) % height;
        seeds.push(Vec2::new(x as f32, y as f32));

        if config.use_image_color {
            // Sample color from the original image at the seed location
            if let Some(color) = fb.get_pixel(x as i32, y as i32) {
                seed_colors.push(color);
            } else {
                seed_colors.push(0xFF000000);
            }
        } else {
            // Generate a random color
            let r = (rng.next_u32() & 0xFF) as u32;
            let g = (rng.next_u32() & 0xFF) as u32;
            let b = (rng.next_u32() & 0xFF) as u32;
            seed_colors.push(0xFF000000 | (r << 16) | (g << 8) | b);
        }
    }

    let dest = fb.as_mut_slice();

    // 2. Process Pixels
    // We iterate over the entire framebuffer. For each pixel, find the nearest seed.
    let metric = config.metric;

    #[cfg(feature = "parallel")]
    let chunk_iter = dest.par_chunks_exact_mut(width);
    #[cfg(not(feature = "parallel"))]
    let chunk_iter = dest.chunks_exact_mut(width);

    chunk_iter.enumerate().for_each(|(y, row)| {
        let fy = y as f32;
        for (x, pixel) in row.iter_mut().enumerate() {
            let fx = x as f32;

            let mut min_dist = f32::MAX;
            let mut second_min_dist = f32::MAX;
            let mut closest_idx = 0;

            for (i, seed) in seeds.iter().enumerate() {
                let dx = (fx - seed.x).abs();
                let dy = (fy - seed.y).abs();

                // Calculate Minkowski distance.
                // Fast paths for Euclidean (metric == 2) and Manhattan (metric == 1).
                let dist = if (metric - 2.0).abs() < f32::EPSILON {
                    #[allow(clippy::imprecise_flops)]
                    let res = (dx * dx + dy * dy).sqrt();
                    res
                } else if (metric - 1.0).abs() < f32::EPSILON {
                    dx + dy
                } else {
                    // General case
                    (dx.powf(metric) + dy.powf(metric)).powf(1.0 / metric)
                };

                if dist < min_dist {
                    second_min_dist = min_dist;
                    min_dist = dist;
                    closest_idx = i;
                } else if dist < second_min_dist {
                    second_min_dist = dist;
                }
            }

            // Check if we are on a border (if border thickness is configured)
            if config.border_thickness > 0.0 {
                // Determine border thickness based on distance difference.
                let diff = (second_min_dist - min_dist).abs();
                if diff <= config.border_thickness {
                    *pixel = config.border_color;
                    continue;
                }
            }

            // Assign color of closest seed
            *pixel = seed_colors[closest_idx];
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voronoi_basic() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFFFFFFFF);

        let config = VoronoiConfig {
            num_seeds: 2,
            use_image_color: false,
            seed: 12345,
            border_thickness: 0.0,
            ..Default::default()
        };

        apply_voronoi(&mut fb, &config);

        // Check that at least two different colors exist in the output
        let mut first_color = None;
        let mut has_multiple_colors = false;

        for &pixel in fb.as_slice() {
            if let Some(c) = first_color {
                if c != pixel {
                    has_multiple_colors = true;
                    break;
                }
            } else {
                first_color = Some(pixel);
            }
        }

        assert!(
            has_multiple_colors,
            "Voronoi with 2 seeds should produce multiple colors"
        );
    }

    #[test]
    fn test_voronoi_image_color() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        // Set left half to red, right half to blue
        for y in 0..10 {
            for x in 0..5 {
                fb.set_pixel(x, y, 0xFFFF0000);
            }
            for x in 5..10 {
                fb.set_pixel(x, y, 0xFF0000FF);
            }
        }

        let config = VoronoiConfig {
            num_seeds: 10,
            use_image_color: true,
            seed: 42,
            border_thickness: 0.0,
            ..Default::default()
        };

        apply_voronoi(&mut fb, &config);

        // All resulting pixels should be either red or blue, nothing else
        for &pixel in fb.as_slice() {
            assert!(
                pixel == 0xFFFF0000 || pixel == 0xFF0000FF,
                "Pixel color should be from the original image (red or blue)"
            );
        }
    }

    #[test]
    fn test_voronoi_borders() {
        let mut fb = Framebuffer::new(20, 20).unwrap();
        fb.clear(0xFFFFFFFF);

        let border_color = 0xFF123456;
        let config = VoronoiConfig {
            num_seeds: 4,
            use_image_color: false,
            seed: 99,
            border_thickness: 2.0, // Large border to ensure it triggers
            border_color,
            ..Default::default()
        };

        apply_voronoi(&mut fb, &config);

        // Check that at least some pixels are border colored
        let has_borders = fb.as_slice().iter().any(|&p| p == border_color);
        assert!(has_borders, "Voronoi borders should be drawn");
    }
}
