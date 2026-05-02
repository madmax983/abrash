//! Kinect Depth Camera Filter
//!
//! A post-processing effect that simulates the raw output of a depth-sensing
//! camera like the Microsoft Kinect. It maps z-buffer depths to a heat-map
//! color gradient (red for near, blue for far) and optionally adds synthetic
//! noise or "point cloud" missing data dots.

use abrash_core::color::Color;
use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::XorShift32;
use abrash_core::zbuffer::ZBuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration parameters for the Kinect Depth filter.
#[derive(Debug, Clone, Copy)]
pub struct KinectDepthConfig {
    /// Color for nearest objects.
    pub near_color: u32,
    /// Color for far objects.
    pub far_color: u32,
    /// Color for background or missing data.
    pub bg_color: u32,
    /// The maximum distance to map to the `far_color`.
    pub max_distance: f32,
    /// The minimum distance to map to the `near_color`.
    pub min_distance: f32,
    /// Amount of synthetic noise/missing data points (0.0 to 1.0).
    pub noise_amount: f32,
    /// Seed for the synthetic noise generator.
    pub seed: u32,
}

impl Default for KinectDepthConfig {
    fn default() -> Self {
        Self {
            near_color: 0xFF_FF_00_00, // Red
            far_color: 0xFF_00_00_FF,  // Blue
            bg_color: 0xFF_00_00_00,   // Black
            max_distance: 100.0,
            min_distance: 0.1,
            noise_amount: 0.05,
            seed: 12345,
        }
    }
}

/// Applies a Kinect-style depth camera effect based on z-buffer depths.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `zb` - The z-buffer containing depth values for each pixel.
/// * `config` - Configuration for the Kinect depth effect.
pub fn apply_kinect_depth(fb: &mut Framebuffer, zb: &ZBuffer, config: &KinectDepthConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let fb_slice = fb.as_mut_slice();
    let zb_slice = zb.as_slice();

    let c_near = Color::from_argb_u32(config.near_color);
    let c_far = Color::from_argb_u32(config.far_color);

    let dist_range = config.max_distance - config.min_distance;
    let inv_range = if dist_range > 0.0 {
        1.0 / dist_range
    } else {
        0.0
    };

    #[cfg(feature = "parallel")]
    let row_iter = fb_slice
        .par_chunks_exact_mut(width)
        .zip(zb_slice.par_chunks_exact(width))
        .enumerate();

    #[cfg(not(feature = "parallel"))]
    let row_iter = fb_slice
        .chunks_exact_mut(width)
        .zip(zb_slice.chunks_exact(width))
        .enumerate();

    row_iter.for_each(|(y, (fb_row, zb_row))| {
        // Use a fast local random generator for noise
        let mut rng = XorShift32::new(config.seed.wrapping_add(y as u32 * 1337));

        for (_, (pixel, depth)) in fb_row.iter_mut().zip(zb_row.iter()).enumerate() {
            // Apply synthetic noise (missing data points)
            if config.noise_amount > 0.0 {
                let r = rng.next_f32();
                if r < config.noise_amount {
                    *pixel = config.bg_color;
                    continue;
                }
            }

            if depth.is_infinite() || *depth > config.max_distance {
                *pixel = config.bg_color;
                continue;
            }

            if *depth < config.min_distance {
                *pixel = config.near_color;
                continue;
            }

            // Calculate normalized depth (0.0 = near, 1.0 = far)
            let normalized_depth = ((*depth - config.min_distance) * inv_range).clamp(0.0, 1.0);

            // Interpolate colors based on depth
            // We use an intermediate color (Yellow/Green) to make it look more like a heat map
            let final_color = if normalized_depth < 0.5 {
                let t = normalized_depth * 2.0;
                let c_mid = Color::new(1.0, 1.0, 1.0, 0.0); // Yellow
                c_near.lerp(c_mid, t)
            } else {
                let t = (normalized_depth - 0.5) * 2.0;
                let c_mid = Color::new(1.0, 1.0, 1.0, 0.0); // Yellow
                c_mid.lerp(c_far, t)
            };

            *pixel = final_color.to_argb_u32();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kinect_depth_infinite() {
        let mut fb = Framebuffer::new(1, 1).unwrap();
        let mut zb = ZBuffer::new(1, 1).unwrap();

        fb.clear(0xFF_FF_FF_FF); // White

        let config = KinectDepthConfig {
            noise_amount: 0.0,
            ..Default::default()
        };
        apply_kinect_depth(&mut fb, &zb, &config);

        assert_eq!(fb.get_pixel(0, 0), Some(config.bg_color));
    }

    #[test]
    fn test_kinect_depth_near() {
        let mut fb = Framebuffer::new(1, 1).unwrap();
        let mut zb = ZBuffer::new(1, 1).unwrap();

        let config = KinectDepthConfig {
            noise_amount: 0.0,
            ..Default::default()
        };

        zb.test_and_set(0, 0, 0.05); // closer than min_distance (0.1)

        apply_kinect_depth(&mut fb, &zb, &config);

        let color = fb.get_pixel(0, 0).unwrap();
        assert_eq!(color, config.near_color);
    }
}
