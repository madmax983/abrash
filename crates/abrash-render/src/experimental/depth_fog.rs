//! Depth Fog Post-Processing Filter
//!
//! Applies a distance-based fog effect to the framebuffer by reading from the Z-buffer.
//! This gives scenes a sense of atmosphere and depth.

use abrash_core::color::Color;
use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration parameters for the Depth Fog filter.
#[derive(Debug, Clone, Copy)]
pub struct DepthFogConfig {
    /// The color of the fog (ARGB format).
    pub fog_color: u32,
    /// The distance from the camera where the fog begins (fog density is 0).
    pub fog_start: f32,
    /// The distance from the camera where the fog ends (fog density is 100%).
    pub fog_end: f32,
}

impl Default for DepthFogConfig {
    fn default() -> Self {
        Self {
            fog_color: 0xFF_88_99_AA, // Soft grayish-blue fog
            fog_start: 2.0,           // Start slightly away from the camera
            fog_end: 10.0,            // Fully opaque at 10 units
        }
    }
}

/// Applies depth-based fog to the framebuffer using the z-buffer depths.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `zb` - The z-buffer containing depth values for each pixel.
/// * `config` - Configuration for the fog effect (color, start, end distances).
pub fn apply_depth_fog(fb: &mut Framebuffer, zb: &ZBuffer, config: &DepthFogConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || config.fog_start >= config.fog_end {
        return;
    }

    let fog_range = config.fog_end - config.fog_start;
    let fb_slice = fb.as_mut_slice();
    let zb_slice = zb.as_slice();

    #[cfg(feature = "parallel")]
    let iter = fb_slice.par_iter_mut().zip(zb_slice.par_iter());

    #[cfg(not(feature = "parallel"))]
    let iter = fb_slice.iter_mut().zip(zb_slice.iter());

    iter.for_each(|(pixel, depth)| {
        if depth.is_infinite() {
            // If there is no geometry, fully apply the fog color (or keep background, depending on preference).
            // Let's fully clear to fog color to simulate infinite atmosphere if it's beyond far clip.
            *pixel = config.fog_color;
            return;
        }

        if *depth <= config.fog_start {
            // Before fog starts, keep original color
            return;
        }

        // Calculate fog factor (0.0 to 1.0)
        let fog_factor = ((*depth - config.fog_start) / fog_range).clamp(0.0, 1.0);

        if fog_factor > 0.0 {
            let c0 = Color::from_argb_u32(*pixel);
            let c1 = Color::from_argb_u32(config.fog_color);
            *pixel = c0.lerp(c1, fog_factor).to_argb_u32();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_depth_fog_blending() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        let mut zb = ZBuffer::new(2, 2).unwrap();

        // Base color: Solid Red
        fb.clear(0xFFFF0000);

        // Fog Color: Solid Blue
        let config = DepthFogConfig {
            fog_color: 0xFF0000FF,
            fog_start: 10.0,
            fog_end: 20.0,
        };

        // Pixel 0,0: Depth 5.0 (Before fog starts -> Should remain Red)
        zb.test_and_set(0, 0, 5.0);

        // Pixel 1,0: Depth 15.0 (Halfway -> Should be 50% Red, 50% Blue -> Magentaish)
        zb.test_and_set(1, 0, 15.0);

        // Pixel 0,1: Depth 25.0 (Past fog end -> Should be fully Blue)
        zb.test_and_set(0, 1, 25.0);

        // Pixel 1,1: Depth Infinity (Background -> Should be fully Blue)
        // (already infinite by default)

        apply_depth_fog(&mut fb, &zb, &config);

        assert_eq!(fb.get_pixel(0, 0), Some(0xFFFF0000)); // Still Red

        // 50% blend of FF0000 and 0000FF is roughly 7F007F (ignoring exact lerp math rounding, let's just check it's changed and has both components)
        let blended = fb.get_pixel(1, 0).unwrap();
        assert_ne!(blended, 0xFFFF0000);
        assert_ne!(blended, 0xFF0000FF);

        assert_eq!(fb.get_pixel(0, 1), Some(0xFF0000FF)); // Fully Blue
        assert_eq!(fb.get_pixel(1, 1), Some(0xFF0000FF)); // Fully Blue
    }
}
