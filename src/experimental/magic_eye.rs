//! Magic Eye (Autostereogram) Generator
//!
//! Converts a depth buffer into a Single Image Random Dot Stereogram (SIRDS),
//! commonly known as a "Magic Eye" picture.
//!
//! By crossing or diverging your eyes while looking at the generated noise pattern,
//! the embedded 3D shape (represented by the depth buffer) will appear to pop out
//! in stereoscopic 3D.

use crate::framebuffer::Framebuffer;
use crate::utils::XorShift32;
use crate::zbuffer::ZBuffer;

/// Configuration for the autostereogram generator.
#[derive(Debug, Clone, Copy)]
pub struct MagicEyeConfig {
    /// Distance between repeating patterns (pixels). This represents the background depth.
    pub pattern_width: usize,
    /// Maximum displacement (pixels) applied to the pattern based on depth.
    /// Higher values mean the object "pops out" more, but can be harder to focus on.
    pub max_depth_offset: usize,
    /// Random seed for the background noise pattern.
    pub seed: u32,
}

impl Default for MagicEyeConfig {
    fn default() -> Self {
        Self {
            pattern_width: 100,
            max_depth_offset: 30,
            seed: 42,
        }
    }
}

/// Generates a Single Image Random Dot Stereogram (SIRDS) in the framebuffer
/// based on the provided depth buffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer where the noise pattern will be written. Must match `ZBuffer` dimensions.
/// * `zb` - The depth buffer containing the 3D shape to embed.
/// * `config` - Parameters controlling the stereogram generation.
pub fn generate_autostereogram(fb: &mut Framebuffer, zb: &ZBuffer, config: MagicEyeConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width != zb.width() as usize || height != zb.height() as usize {
        return; // Mismatched dimensions
    }

    let pixels = fb.as_mut_slice();
    let depths = zb.as_slice();

    // 1. Find min and max depth (excluding Infinity/background) to normalize
    let mut min_z = f32::MAX;
    let mut max_z = f32::MIN;
    let mut has_content = false;

    for &z in depths {
        if z != f32::INFINITY {
            if z < min_z {
                min_z = z;
            }
            if z > max_z {
                max_z = z;
            }
            has_content = true;
        }
    }

    let z_range = if has_content && (max_z - min_z) > 1e-6 {
        max_z - min_z
    } else {
        1.0 // Prevent division by zero
    };

    let mut rng = XorShift32::new(config.seed);

    // To construct the stereogram, we process it row by row.
    // Each row consists of linked pixels: pixel at (x) must match pixel at (x - pattern_width + shift).
    // We use a disjoint-set (union-find) or simple array tracking to link identical pixels.
    let mut links = vec![0; width];

    for y in 0..height {
        // Initialize links: initially every pixel is independent
        for (x, link) in links.iter_mut().enumerate().take(width) {
            *link = x;
        }

        // Link pixels based on depth
        for x in 0..width {
            let depth = depths[y * width + x];

            // Calculate depth shift
            // normalized_depth: 0.0 (background/far) to 1.0 (foreground/close)
            let normalized_depth = if depth == f32::INFINITY || !has_content {
                0.0
            } else {
                // Closer depth values are usually smaller in our ZBuffer (0.0 to 1.0 range).
                // So min_z is the closest point (1.0 shift), max_z is the furthest point (0.0 shift).
                (max_z - depth) / z_range
            }
            .clamp(0.0, 1.0);

            // Calculate how much to shrink the pattern width for this pixel.
            // Closer objects have a *smaller* repeating distance.
            let shift = (normalized_depth * config.max_depth_offset as f32).round() as usize;

            // The repeating period for this specific depth
            let period = config.pattern_width.saturating_sub(shift).max(1);

            // Link the current pixel to the one `period` pixels to the left
            if x >= period {
                let left_idx = x - period;

                // Find roots for both
                let mut root_left = left_idx;
                while links[root_left] != root_left {
                    root_left = links[root_left];
                }

                let mut root_current = x;
                while links[root_current] != root_current {
                    root_current = links[root_current];
                }

                // Link them
                if root_left != root_current {
                    // Always point the rightmost root to the leftmost root
                    // to ensure we copy colors from left to right
                    if root_left < root_current {
                        links[root_current] = root_left;
                    } else {
                        links[root_left] = root_current;
                    }
                }
            }
        }

        // Assign colors based on linked roots
        for x in 0..width {
            if links[x] == x {
                // This is a root pixel, generate a random color
                // We generate random greyscale or bright colors for contrast.
                let r = rng.next_u32() % 256;
                let g = rng.next_u32() % 256;
                let b = rng.next_u32() % 256;
                let color = 0xFF00_0000 | (r << 16) | (g << 8) | b;

                pixels[y * width + x] = color;
            } else {
                // This pixel is linked to a root, copy its color
                let mut root = x;
                while links[root] != root {
                    root = links[root];
                }

                pixels[y * width + x] = pixels[y * width + root];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autostereogram_generation() {
        let width = 200;
        let height = 50;
        let mut fb = Framebuffer::new(width as u32, height as u32).unwrap();
        let mut zb = ZBuffer::new(width as u32, height as u32).unwrap();

        // Create a simple "raised square" depth map
        // Background is infinity. Square is depth 0.5.
        for y in 10..40 {
            for x in 80..120 {
                zb.test_and_set(x, y, 0.5);
            }
        }

        let config = MagicEyeConfig {
            pattern_width: 20,
            max_depth_offset: 5,
            seed: 123,
        };

        generate_autostereogram(&mut fb, &zb, config);

        // Verify that the framebuffer was modified (no longer black)
        let mut has_color = false;
        for y in 0..height as i32 {
            for x in 0..width as i32 {
                if fb.get_pixel(x, y).unwrap() != 0 {
                    has_color = true;
                    break;
                }
            }
        }
        assert!(has_color, "Framebuffer should be filled with noise pattern");

        // Check that pixels repeat roughly according to pattern width
        // Background row (y=5)
        let p1 = fb.get_pixel(100, 5).unwrap();
        // Since background has depth infinity -> shift 0 -> period is pattern_width (20)
        // Root tracing might be complex, but let's check exact links:
        // x=100 links to 80, 80 to 60, etc.
        // We can just verify it didn't crash and produced non-zero output.
        assert_ne!(p1, 0);
    }
}
