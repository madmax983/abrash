//! Kinect Depth Camera Filter
//!
//! Maps the depth buffer (Z-buffer) to a false-color heat map, simulating the
//! aesthetic of a Kinect depth camera or structured light scanner.
//! Uses a classic blue->green->yellow->red linear mapping.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Kinect depth filter.
#[derive(Debug, Clone, Copy)]
pub struct KinectDepthConfig {
    /// The minimum depth distance (mapped to hot/red).
    pub min_depth: f32,
    /// The maximum depth distance (mapped to cold/blue).
    pub max_depth: f32,
}

impl Default for KinectDepthConfig {
    fn default() -> Self {
        Self {
            min_depth: 1.0,
            max_depth: 20.0,
        }
    }
}

/// Applies a Kinect-style depth mapping to the framebuffer based on the depth buffer.
///
/// *   **Close objects** (near `min_depth`) are rendered as Red/Orange.
/// *   **Mid objects** are rendered as Yellow/Green.
/// *   **Far objects** (near `max_depth`) are rendered as Blue/Purple.
/// *   **Background** (Infinity or beyond `max_depth`) is rendered as black.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `zb` - The z-buffer containing depth values for each pixel.
/// * `config` - Configuration for depth ranges.
pub fn apply_kinect_depth(fb: &mut Framebuffer, zb: &ZBuffer, config: &KinectDepthConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || config.min_depth >= config.max_depth {
        return;
    }

    let pixels = fb.as_mut_slice();
    let depths = zb.as_slice();

    let range = config.max_depth - config.min_depth;
    let inv_range = 1.0 / range;

    #[cfg(feature = "parallel")]
    let iter = pixels.par_iter_mut().zip(depths.par_iter());

    #[cfg(not(feature = "parallel"))]
    let iter = pixels.iter_mut().zip(depths.iter());

    iter.for_each(|(pixel, &depth)| {
        if depth == f32::INFINITY || depth > config.max_depth {
            *pixel = 0xFF00_0000; // Black background
            return;
        }

        // Clamp depth to the minimum bound
        let clamped_depth = depth.max(config.min_depth);

        // Normalize Z to [0.0, 1.0]
        // 0.0 = Closest (Red)
        // 1.0 = Furthest (Blue/Black)
        let normalized = ((clamped_depth - config.min_depth) * inv_range).clamp(0.0, 1.0);

        // Kinect-style false color palette
        // 0.00 (Hot)  -> White (255, 255, 255)
        // 0.25        -> Red (255, 0, 0)
        // 0.50        -> Yellow (255, 255, 0)
        // 0.75        -> Green (0, 255, 0)
        // 1.00 (Cold) -> Blue (0, 0, 255) -> Black (0, 0, 0)

        let (r, g, b) = if normalized < 0.25 {
            // White -> Red
            let t = normalized * 4.0;
            (255, ((1.0 - t) * 255.0) as u32, ((1.0 - t) * 255.0) as u32)
        } else if normalized < 0.5 {
            // Red -> Yellow
            let t = (normalized - 0.25) * 4.0;
            (255, (t * 255.0) as u32, 0)
        } else if normalized < 0.75 {
            // Yellow -> Green
            let t = (normalized - 0.5) * 4.0;
            (((1.0 - t) * 255.0) as u32, 255, 0)
        } else if normalized < 0.95 {
            // Green -> Blue
            // Compress the Green -> Blue transition slightly
            let t = (normalized - 0.75) * 5.0;
            (0, ((1.0 - t) * 255.0) as u32, (t * 255.0) as u32)
        } else {
            // Blue -> Black fade out at the very end
            let t = (normalized - 0.95) * 20.0;
            (0, 0, ((1.0 - t) * 255.0) as u32)
        };

        *pixel = 0xFF00_0000 | (r << 16) | (g << 8) | b;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_kinect_depth() {
        let mut fb = Framebuffer::new(5, 1).unwrap();
        let mut zb = ZBuffer::new(5, 1).unwrap();

        // 0: Near -> Red
        // 1: Mid-Near -> Yellow
        // 2: Mid -> Green
        // 3: Mid-Far -> Blue
        // 4: Far -> Black

        let config = KinectDepthConfig {
            min_depth: 1.0,
            max_depth: 5.0,
        };

        // 0.0, 0.25, 0.5, 0.75, 1.0
        zb.test_and_set(0, 0, 1.0); // 0.0 -> White/Red
        zb.test_and_set(1, 0, 2.0); // 0.25 -> Red
        zb.test_and_set(2, 0, 3.0); // 0.5 -> Yellow
        zb.test_and_set(3, 0, 4.0); // 0.75 -> Green
        zb.test_and_set(4, 0, 5.0); // 1.0 -> Black

        apply_kinect_depth(&mut fb, &zb, &config);

        // Near / 0.0 (White -> Red, normalized=0.0 -> Red=255)
        let p0 = fb.get_pixel(0, 0).unwrap();
        assert_eq!((p0 >> 16) & 0xFF, 255); // Red is high

        // Mid / 0.5 (Yellow, normalized=0.5 -> Red=255, Green=255, Blue=0)
        let p2 = fb.get_pixel(2, 0).unwrap();
        assert_eq!((p2 >> 16) & 0xFF, 255);
        assert_eq!((p2 >> 8) & 0xFF, 255);
        assert_eq!(p2 & 0xFF, 0);

        // Far / 1.0 (Black)
        let p4 = fb.get_pixel(4, 0).unwrap();
        assert_eq!(p4 & 0x00FF_FFFF, 0x0000_0000);
    }
}
