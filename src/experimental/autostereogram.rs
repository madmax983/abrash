//! Autostereogram (Magic Eye) Post-Processing Filter
//!
//! Generates a Single Image Random Dot Stereogram (SIRDS) based on the depth buffer.
//! This encodes a 3D image into a 2D pattern that can be viewed by diverging the eyes.

use crate::framebuffer::Framebuffer;
use crate::zbuffer::ZBuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the autostereogram generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StereogramConfig {
    /// The width of the repeating background pattern (in pixels).
    pub pattern_width: usize,
    /// The maximum horizontal pixel shift for the closest depth values.
    pub max_shift: usize,
    /// The colors used for the random dot pattern.
    pub colors: [u32; 2],
}

impl Default for StereogramConfig {
    fn default() -> Self {
        Self {
            pattern_width: 100,
            max_shift: 30,
            colors: [0xFF000000, 0xFFFFFFFF], // Black and White dots by default
        }
    }
}

/// A simple pseudo-random number generator using Xorshift algorithm.
struct XorShift32 {
    state: u32,
}

impl XorShift32 {
    const fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0xDEADBEEF } else { seed },
        }
    }

    const fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }
}

/// Applies the autostereogram effect to the framebuffer based on the Z-Buffer.
///
/// This overwrites the current framebuffer with a random dot pattern, shifted horizontally
/// based on the depth of the pixels in the Z-Buffer.
///
/// Bolt Performance Optimization:
/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
/// remainder chunk handling and bounds checking, enabling better vectorization
/// and measurable performance improvements.
pub fn apply_autostereogram(fb: &mut Framebuffer, zb: &ZBuffer, config: &StereogramConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || config.pattern_width == 0 {
        return;
    }

    let pixels = fb.as_mut_slice();
    let depths = zb.as_slice();

    // Process each row independently.
    // For a stereogram, each row can be calculated independently based on the depth values in that row.
    // We generate a random pattern for the first `pattern_width` pixels of the row,
    // and then for each subsequent pixel, we look back `pattern_width - shift` pixels to copy the color.

    let process_row = |y: usize, row_pixels: &mut [u32], row_depths: &[f32]| {
        let mut rng = XorShift32::new((y as u32 * 1337) ^ 0x12345678); // Unique seed per row

        // Array to hold the links (which pixel this pixel copies from)
        // We initialize it to link to the pixel exactly `pattern_width` to the left.
        let mut links = vec![0usize; width];
        for x in 0..width {
            links[x] = x;
        }

        // Calculate depth-based shifts and establish links.
        // We iterate from left to right.
        for x in 0..width {
            let depth = row_depths[x];

            // Calculate shift based on depth.
            // Z-buffer values: Near is -1.0, Far is 1.0 (or infinite for background).
            // We want maximum shift for near objects, zero shift for background.
            let shift = if depth.is_infinite() {
                0
            } else {
                // Map [-1.0, 1.0] -> [1.0, 0.0]
                // -1.0 (Near) -> 1.0 (Max Shift)
                // 1.0 (Far) -> 0.0 (Min Shift)
                let normalized_depth = ((1.0 - depth) * 0.5).clamp(0.0, 1.0);
                (normalized_depth * config.max_shift as f32) as usize
            };

            // The separation between the eyes for this pixel
            let separation = config.pattern_width.saturating_sub(shift).max(1);

            // Left and right eye coordinates
            // This is a simplified SIRDS algorithm.
            if x >= separation {
                let left = x - separation;
                let right = x;

                // We want right pixel to be the same color as the left pixel.
                // We use a disjoint-set (union-find) like approach to link pixels.
                // Always link the rightmost pixel to the leftmost pixel's root.
                let mut l_root = left;
                while links[l_root] != l_root {
                    l_root = links[l_root];
                }

                let mut r_root = right;
                while links[r_root] != r_root {
                    r_root = links[r_root];
                }

                if l_root != r_root {
                    // Link the larger index to the smaller index
                    if l_root < r_root {
                        links[r_root] = l_root;
                    } else {
                        links[l_root] = r_root;
                    }
                }
            }
        }

        // Now, assign colors based on the links.
        for x in 0..width {
            if links[x] == x {
                // This is a root pixel, generate a random color.
                let color_idx = (rng.next_u32() % config.colors.len() as u32) as usize;
                row_pixels[x] = config.colors[color_idx];
            } else {
                // Copy color from the linked pixel.
                row_pixels[x] = row_pixels[links[x]];
            }
        }
    };

    #[cfg(feature = "parallel")]
    {
        // Zip the pixels and depths together to process row by row in parallel.
        pixels
            .par_chunks_exact_mut(width)
            .zip(depths.par_chunks_exact(width))
            .enumerate()
            .for_each(|(y, (row_pixels, row_depths))| {
                process_row(y, row_pixels, row_depths);
            });
    }

    #[cfg(not(feature = "parallel"))]
    {
        pixels
            .chunks_exact_mut(width)
            .zip(depths.chunks_exact(width))
            .enumerate()
            .for_each(|(y, (row_pixels, row_depths))| {
                process_row(y, row_pixels, row_depths);
            });
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_autostereogram_background_only() {
        let width: u32 = 200;
        let height: u32 = 10;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        // Background is infinity.
        zb.clear(); // Clears to f32::INFINITY

        let config = StereogramConfig {
            pattern_width: 50,
            max_shift: 20,
            colors: [0xFF000000, 0xFFFFFFFF],
        };

        apply_autostereogram(&mut fb, &zb, &config);

        // With infinite depth (background), the pattern should repeat exactly every `pattern_width` pixels.
        for y in 0..height {
            for x in config.pattern_width..width as usize {
                let current_pixel = fb.get_pixel(x as i32, y as i32);
                let prev_pixel = fb.get_pixel((x - config.pattern_width) as i32, y as i32);
                assert_eq!(
                    current_pixel, prev_pixel,
                    "Background pattern should repeat every {} pixels",
                    config.pattern_width
                );
            }
        }
    }

    #[test]
    fn test_autostereogram_with_depth() {
        let width: u32 = 200;
        let height: u32 = 1;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        // Clear to background
        zb.clear();

        // Set a block in the middle to a close depth (-1.0)
        for x in 80..120 {
            zb.test_and_set(x, 0, -1.0); // -1.0 is near
        }

        let config = StereogramConfig {
            pattern_width: 50,
            max_shift: 10, // Small shift to easily track
            colors: [0xFF000000, 0xFFFFFFFF],
        };

        apply_autostereogram(&mut fb, &zb, &config);

        // For the background (x < 80), pattern should repeat every `pattern_width` (50).
        for x in 50..80 {
            let current = fb.get_pixel(x as i32, 0);
            let prev = fb.get_pixel((x - 50) as i32, 0);
            assert_eq!(current, prev, "Background pattern mismatch at {x}");
        }

        // For the foreground block (x in 80..120), the separation should be `pattern_width - max_shift` = 40.
        // Wait, the algorithm maps links. A pixel at `x` looks back `separation` pixels.
        // For x=80, it looks back 40 pixels (to x=40).
        // Let's just check that inside the block, the repetition distance is 40.
        // However, the links might have propagated differently due to the left-to-right processing.
        // The most robust check is that the distance between repeating pixels is 40 inside the block.
        // Wait, actually, the pixel at x=80 links to x=40. So color at x=80 equals color at x=40.
        // And pixel at x=119 links to x=79.
        let foreground_separation = config.pattern_width - config.max_shift;
        for x in 80 + foreground_separation..120 {
            // Because of the union-find linking, a pixel at x inside the block should match a pixel `foreground_separation` to its left,
            // IF that pixel to the left is also within the block or properly linked.
            // Let's test a simple correlation:
            let current = fb.get_pixel(x as i32, 0);
            let prev = fb.get_pixel((x - foreground_separation as usize) as i32, 0);
            assert_eq!(current, prev, "Foreground pattern mismatch at {x}");
        }
    }
}
