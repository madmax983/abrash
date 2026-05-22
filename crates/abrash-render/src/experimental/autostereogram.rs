//! Autostereogram (Magic Eye) Generator.
//!
//! Generates a single-image stereogram from a depth map (`ZBuffer`) using
//! a union-find-like approach to link pixels horizontally by depth shift,
//! allowing for perfect row-by-row parallelization.

use crate::framebuffer::Framebuffer;
use crate::zbuffer::ZBuffer;

/// Configuration for the Autostereogram effect.
#[derive(Debug, Clone, Copy)]
pub struct AutostereogramConfig {
    /// The base repeating pattern width in pixels.
    pub pattern_width: u32,
    /// The maximum horizontal shift (in pixels) for the closest objects.
    pub max_shift: u32,
    /// Depth scaling factor.
    pub depth_scale: f32,
}

impl Default for AutostereogramConfig {
    fn default() -> Self {
        Self {
            pattern_width: 100,
            max_shift: 30,
            depth_scale: 1.0,
        }
    }
}

/// Applies the Autostereogram effect to the given framebuffer based on the `ZBuffer`.
///
/// Bolt Performance Optimization:
/// Uses `.par_chunks_exact_mut(width)` and Thread-Local Storage for `links` and `colors`
/// arrays to allow parallel processing of rows while avoiding allocations.
pub fn apply_autostereogram(fb: &mut Framebuffer, zb: &ZBuffer, config: AutostereogramConfig) {
    let width = fb.width() as usize;
    let pixels = fb.as_mut_slice();
    let depths = zb.as_slice();

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;

        pixels
            .par_chunks_exact_mut(width)
            .enumerate()
            .for_each(|(y, row_pixels)| {
                let row_start = y * width;
                let row_depths = &depths[row_start..row_start + width];

                process_row(y as u32, row_pixels, row_depths, width, config);
            });
    }

    #[cfg(not(feature = "parallel"))]
    {
        pixels
            .chunks_exact_mut(width)
            .enumerate()
            .for_each(|(y, row_pixels)| {
                let row_start = y * width;
                let row_depths = &depths[row_start..row_start + width];
                process_row(y as u32, row_pixels, row_depths, width, config);
            });
    }
}

fn process_row(
    row_idx: u32,
    row_pixels: &mut [u32],
    row_depths: &[f32],
    width: usize,
    config: AutostereogramConfig,
) {
    // A simple PRNG seeded with row index to ensure deterministic noise per row
    let mut seed: u32 = 0xDEAD_BEEF ^ (row_idx * 1337);
    let mut next_rand = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed
    };

    // ⚡ Bolt Performance Optimization:
    // Stereogram generation using Union-Find for `links[x] = x - separation` where pixels
    // are processed left-to-right ensures `x - separation` is always fully resolved.
    // By fusing the depth lookup, pattern matching, and color generation into a single forward pass,
    // we completely eliminate O(N) array allocations (links/colors), the O(N) union-find path
    // compressions, and multi-pass loop overhead.
    for x in 0..width {
        let depth = row_depths[x];

        // Map depth to a shift amount.
        let shift = if depth.is_infinite() {
            0
        } else {
            // Normalize depth roughly 0 to 1, invert so closer is larger shift
            let mut d = ((depth + 1.0) * 0.5 * config.depth_scale).clamp(0.0, 1.0);
            d = 1.0 - d; // Closer objects (smaller d) get larger shift

            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            // ⚡ Bolt: Replace f32::round() with fast integer casting
            let s = (((d * config.max_shift as f32) + 16384.5) as i32 as f32 - 16384.0) as u32;
            s.min(config.pattern_width - 1)
        };

        let separation = (config.pattern_width - shift) as usize;

        if x >= separation {
            // Because `left` < `x`, its final color is already fully computed.
            let left = x - separation;
            row_pixels[x] = row_pixels[left];
        } else {
            // Generate random grayscale color for new pattern roots
            let intensity = (next_rand() & 255) as u32;
            row_pixels[x] = 0xFF00_0000 | (intensity << 16) | (intensity << 8) | intensity;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autostereogram_changes_buffer() {
        let width = 200;
        let height = 10;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        // Fill fb with a flat white pattern
        fb.clear(0xFFFF_FFFF);

        // Add a shape to the ZBuffer at center
        for y in 2..8 {
            for x in 80..120 {
                zb.test_and_set(x, y, 0.5); // Closer depth
            }
        }

        let config = AutostereogramConfig {
            pattern_width: 20,
            max_shift: 5,
            depth_scale: 1.0,
        };

        apply_autostereogram(&mut fb, &zb, config);

        // Check that some pixels have changed (since it should generate random noise/pattern)
        let mut changed = false;
        for i in 0..(width * height) as usize {
            if fb.as_slice()[i] != 0xFFFF_FFFF {
                changed = true;
                break;
            }
        }

        assert!(changed, "Autostereogram should alter the framebuffer");
    }
}
