//! Depth Fog Post-Processing Effect
//!
//! A post-processing filter that blends pixels with a specified fog color
//! based on their depth in the Z-buffer. This simulates distance fog or
//! atmospheric perspective.

use crate::framebuffer::Framebuffer;
use crate::zbuffer::ZBuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Depth Fog effect.
#[derive(Debug, Clone, Copy)]
pub struct FogConfig {
    /// The RGB color of the fog (e.g., 0x00AABBCC). The alpha channel is ignored.
    pub color: u32,
    /// The depth at which fog starts to appear (0.0 = no fog).
    pub near: f32,
    /// The depth at which fog becomes fully opaque (1.0 = solid color).
    pub far: f32,
}

impl Default for FogConfig {
    fn default() -> Self {
        Self {
            color: 0x00_80_80_80, // Gray fog
            near: 10.0,
            far: 50.0,
        }
    }
}

/// Applies a depth-based fog effect to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `zb` - The Z-buffer containing the depth values for each pixel.
/// * `config` - The configuration defining fog color and distance range.
pub fn apply_fog(fb: &mut Framebuffer, zb: &ZBuffer, config: &FogConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width != zb.width() as usize || height != zb.height() as usize {
        return; // Dimensions mismatch
    }

    if config.near >= config.far {
        return; // Invalid fog range
    }

    let pixels = fb.as_mut_slice();
    let depths = zb.as_slice();

    let fog_r = (config.color >> 16) & 0xFF;
    let fog_g = (config.color >> 8) & 0xFF;
    let fog_b = config.color & 0xFF;

    let range = config.far - config.near;
    let inv_range = 1.0 / range;

    #[cfg(feature = "parallel")]
    {
        // We use an iterator that pairs rows of pixels with rows of depths.
        pixels
            .par_chunks_exact_mut(width)
            .zip(depths.par_chunks_exact(width))
            .for_each(|(pixel_row, depth_row)| {
                for (pixel, &depth) in pixel_row.iter_mut().zip(depth_row.iter()) {
                    if depth <= config.near {
                        continue; // No fog
                    }

                    if depth >= config.far || depth == f32::INFINITY {
                        // Full fog (or background)
                        *pixel = (*pixel & 0xFF00_0000) | (fog_r << 16) | (fog_g << 8) | fog_b;
                        continue;
                    }

                    // Linear interpolation factor (0.0 to 1.0)
                    let factor = (depth - config.near) * inv_range;
                    let factor_fixed = (factor * 256.0) as u32;
                    let inv_factor = 256 - factor_fixed;

                    let p = *pixel;
                    let p_r = (p >> 16) & 0xFF;
                    let p_g = (p >> 8) & 0xFF;
                    let p_b = p & 0xFF;

                    let new_r = (p_r * inv_factor + fog_r * factor_fixed) >> 8;
                    let new_g = (p_g * inv_factor + fog_g * factor_fixed) >> 8;
                    let new_b = (p_b * inv_factor + fog_b * factor_fixed) >> 8;

                    *pixel = (p & 0xFF00_0000) | (new_r << 16) | (new_g << 8) | new_b;
                }
            });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for (pixel, &depth) in pixels.iter_mut().zip(depths.iter()) {
            if depth <= config.near {
                continue; // No fog
            }

            if depth >= config.far || depth == f32::INFINITY {
                // Full fog (or background)
                *pixel = (*pixel & 0xFF00_0000) | (fog_r << 16) | (fog_g << 8) | fog_b;
                continue;
            }

            // Linear interpolation factor (0.0 to 1.0)
            let factor = (depth - config.near) * inv_range;
            let factor_fixed = (factor * 256.0) as u32;
            let inv_factor = 256 - factor_fixed;

            let p = *pixel;
            let p_r = (p >> 16) & 0xFF;
            let p_g = (p >> 8) & 0xFF;
            let p_b = p & 0xFF;

            let new_r = (p_r * inv_factor + fog_r * factor_fixed) >> 8;
            let new_g = (p_g * inv_factor + fog_g * factor_fixed) >> 8;
            let new_b = (p_b * inv_factor + fog_b * factor_fixed) >> 8;

            *pixel = (p & 0xFF00_0000) | (new_r << 16) | (new_g << 8) | new_b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fog_config_default() {
        let config = FogConfig::default();
        assert_eq!(config.color, 0x00_80_80_80);
        assert!((config.near - 10.0).abs() < f32::EPSILON);
        assert!((config.far - 50.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_apply_fog() {
        let mut fb = Framebuffer::new(3, 1).unwrap();
        let mut zb = ZBuffer::new(3, 1).unwrap();

        // 1. Near (0.0) -> No fog
        // 2. Mid (5.0) -> 50% fog
        // 3. Far (10.0) -> 100% fog
        zb.test_and_set(0, 0, 0.0);
        zb.test_and_set(1, 0, 5.0);
        zb.test_and_set(2, 0, 10.0);

        fb.set_pixel(0, 0, 0xFF_FF_FF_FF); // White
        fb.set_pixel(1, 0, 0xFF_FF_FF_FF);
        fb.set_pixel(2, 0, 0xFF_FF_FF_FF);

        let config = FogConfig {
            color: 0x00_00_00_00, // Black fog
            near: 0.0,
            far: 10.0,
        };

        apply_fog(&mut fb, &zb, &config);

        // Near: White
        assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF_FF_FF_FF);

        // Mid: 50% Black (approx 127)
        let mid = fb.get_pixel(1, 0).unwrap();
        let r = (mid >> 16) & 0xFF;
        assert!((r as i32 - 127).abs() <= 1, "Expected ~127, got {r}");

        // Far: Black
        assert_eq!(fb.get_pixel(2, 0).unwrap(), 0xFF_00_00_00);
    }

    #[test]
    fn test_fog_background() {
        let mut fb = Framebuffer::new(1, 1).unwrap();
        let zb = ZBuffer::new(1, 1).unwrap(); // Initialized to INFINITY

        fb.set_pixel(0, 0, 0xFF_FF_00_00); // Red background

        let config = FogConfig {
            color: 0x00_00_FF_00, // Green fog
            near: 10.0,
            far: 50.0,
        };

        apply_fog(&mut fb, &zb, &config);

        // Should be completely replaced by fog color
        assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF_00_FF_00);
    }
}
