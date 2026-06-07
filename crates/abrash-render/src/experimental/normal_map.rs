//! Screen-Space Normal Generation Filter
//!
//! A post-processing effect that reconstructs surface normals directly from the Z-Buffer,
//! effectively turning a depth map into a normal map (for deferred shading or artistic effects).

use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec3;
use abrash_core::zbuffer::ZBuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Normal Map Generator.
#[derive(Debug, Clone, Copy)]
pub struct NormalMapConfig {
    /// Depth difference scaling factor (higher means steeper normals).
    pub depth_scale: f32,
    /// Whether to encode the normals as RGB colors [0, 255] or keep raw mapping.
    pub rgb_encode: bool,
}

impl Default for NormalMapConfig {
    fn default() -> Self {
        Self {
            depth_scale: 1.0,
            rgb_encode: true,
        }
    }
}

/// Applies a screen-space normal map generation filter to the framebuffer.
///
/// It samples neighboring depths from the Z-buffer to estimate the gradient
/// (surface normal) and writes it into the framebuffer as an RGB color.
pub fn apply_normal_map(fb: &mut Framebuffer, zb: &ZBuffer, config: &NormalMapConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let pixels = fb.as_mut_slice();
    let depths = zb.as_slice();
    let depth_scale = config.depth_scale;
    let rgb_encode = config.rgb_encode;

    #[cfg(feature = "parallel")]
    let row_iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = pixels.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        if y == 0 || y == height - 1 {
            // Edges: clear to flat normal (Z up)
            if rgb_encode {
                row.fill(0xFF_8080FF);
            } else {
                row.fill(0xFF_0000FF);
            }
            return;
        }

        let idx_base = y * width;
        let idx_up = (y - 1) * width;
        let idx_down = (y + 1) * width;

        for x in 1..width - 1 {
            let z_center = depths[idx_base + x];

            if z_center.is_infinite() {
                // Background -> flat normal
                if rgb_encode {
                    row[x] = 0xFF_8080FF;
                } else {
                    row[x] = 0xFF_0000FF;
                }
                continue;
            }

            let z_left = depths[idx_base + x - 1];
            let z_right = depths[idx_base + x + 1];
            let z_up = depths[idx_up + x];
            let z_down = depths[idx_down + x];

            // Handle edges where neighboring pixels are background
            let dx = if z_right.is_infinite() || z_left.is_infinite() {
                0.0
            } else {
                (z_right - z_left) * depth_scale
            };

            let dy = if z_down.is_infinite() || z_up.is_infinite() {
                0.0
            } else {
                (z_down - z_up) * depth_scale
            };

            // Calculate cross product of tangent vectors: (1, 0, dx) x (0, 1, dy)
            // Normal = (-dx, -dy, 1) normalized
            let mut normal = Vec3::new(-dx, -dy, 1.0);
            let _ = normal.normalize();

            if rgb_encode {
                // Encode to RGB [0, 255] mapping [-1, 1] to [0, 255]
                let r = ((normal.x * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u32;
                let g = ((normal.y * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u32;
                let b = ((normal.z * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u32;
                row[x] = 0xFF_000000 | (r << 16) | (g << 8) | b;
            } else {
                // Encode signed mapping [-1, 1] to [-127, 127] then cast to u8 as 2s complement
                let r = (normal.x * 127.0).clamp(-127.0, 127.0) as i8 as u8 as u32;
                let g = (normal.y * 127.0).clamp(-127.0, 127.0) as i8 as u8 as u32;
                let b = (normal.z * 127.0).clamp(-127.0, 127.0) as i8 as u8 as u32;
                row[x] = 0xFF_000000 | (r << 16) | (g << 8) | b;
            }
        }

        // Left and right edges
        if rgb_encode {
            row[0] = 0xFF_8080FF;
            row[width - 1] = 0xFF_8080FF;
        } else {
            row[0] = 0xFF_0000FF;
            row[width - 1] = 0xFF_0000FF;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_map_flat() {
        let mut fb = Framebuffer::new(3, 3).unwrap();
        let mut zb = ZBuffer::new(3, 3).unwrap();

        // Fill with constant depth
        for y in 0..3 {
            for x in 0..3 {
                zb.test_and_set(x, y, 10.0);
            }
        }

        let config = NormalMapConfig::default();
        apply_normal_map(&mut fb, &zb, &config);

        // Center pixel should be flat normal (Z up -> RGB 128, 128, 255)
        let pixel = fb.get_pixel(1, 1).unwrap();
        let r = (pixel >> 16) & 0xFF;
        let g = (pixel >> 8) & 0xFF;
        let b = pixel & 0xFF;

        assert_eq!(r, 127); // 0.5 * 255
        assert_eq!(g, 127);
        assert_eq!(b, 255); // 1.0 * 255
    }

    #[test]
    fn test_normal_map_flat_raw() {
        let mut fb = Framebuffer::new(3, 3).unwrap();
        let mut zb = ZBuffer::new(3, 3).unwrap();

        for y in 0..3 {
            for x in 0..3 {
                zb.test_and_set(x, y, 10.0);
            }
        }

        let config = NormalMapConfig {
            depth_scale: 1.0,
            rgb_encode: false,
        };
        apply_normal_map(&mut fb, &zb, &config);

        let pixel = fb.get_pixel(1, 1).unwrap();
        let r = (pixel >> 16) & 0xFF;
        let g = (pixel >> 8) & 0xFF;
        let b = pixel & 0xFF;

        assert_eq!(r, 0); // 0.0 * 127 = 0
        assert_eq!(g, 0);
        assert_eq!(b, 127); // 1.0 * 127 = 127
    }

    #[test]
    fn test_normal_map_slope() {
        let mut fb = Framebuffer::new(3, 3).unwrap();
        let mut zb = ZBuffer::new(3, 3).unwrap();

        // Create a horizontal slope
        for y in 0..3 {
            zb.test_and_set(0, y, 1.0);
            zb.test_and_set(1, y, 2.0);
            zb.test_and_set(2, y, 3.0);
        }

        let config = NormalMapConfig::default();
        apply_normal_map(&mut fb, &zb, &config);

        // Center pixel should point somewhat left
        let pixel = fb.get_pixel(1, 1).unwrap();
        let r = (pixel >> 16) & 0xFF;

        assert!(r < 127); // Normal points left (negative X), so R < 127
    }
}
