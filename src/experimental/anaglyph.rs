//! Anaglyph 3D Stereoscopic Effect.
//!
//! Creates a retro red-cyan 3D stereoscopic image by offsetting the red color
//! channel based on the depth from the `ZBuffer`.

use crate::framebuffer::Framebuffer;
use crate::zbuffer::ZBuffer;

/// Configuration for the Anaglyph 3D effect.
#[derive(Debug, Clone, Copy)]
pub struct AnaglyphConfig {
    /// The maximum horizontal pixel offset for objects closest to the camera.
    pub max_offset: u32,
    /// The depth value representing the focal plane (where offset is 0).
    pub focal_depth: f32,
}

impl Default for AnaglyphConfig {
    fn default() -> Self {
        Self {
            max_offset: 10,
            focal_depth: 2.0,
        }
    }
}

/// Applies the Anaglyph 3D effect.
///
/// Modifies the framebuffer in place by shifting the red channel horizontally
/// based on the corresponding pixel depth in the `ZBuffer`.
///
/// Bolt Performance Optimization:
/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
/// remainder chunk handling and bounds checking, enabling better vectorization
/// and measurable performance improvements.
pub fn apply_anaglyph(fb: &mut Framebuffer, zb: &ZBuffer, config: AnaglyphConfig) {
    if config.max_offset == 0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    // We need a temporary copy of the original framebuffer to sample from
    // while writing to the destination framebuffer, because the shift depends
    // on depth and we don't want to overwrite pixels we haven't processed yet.
    // Or we can process row by row and only buffer a row. Let's buffer rows for better memory efficiency.

    let mut temp_row_red = vec![0u32; width];
    let mut temp_row_depth = vec![0.0f32; width];

    let pixels = fb.as_mut_slice();
    let depths = zb.as_slice();

    #[cfg(feature = "parallel")]
    {
        // To prevent clippy linting unused imports without `parallel` feature
        #[allow(unused_imports)]
        use rayon::prelude::*;

        // Thread-local buffers for parallel execution
        std::thread_local! {
            static ROW_BUFFERS: std::cell::RefCell<(Vec<u32>, Vec<f32>)> = const { std::cell::RefCell::new((Vec::new(), Vec::new())) };
        }

        pixels
            .par_chunks_exact_mut(width)
            .enumerate()
            .for_each(|(y, row_pixels)| {
                ROW_BUFFERS.with(|buffers| {
                    let mut b = buffers.borrow_mut();
                    if b.0.len() < width {
                        b.0.resize(width, 0);
                        b.1.resize(width, 0.0);
                    }

                    let (ref mut b_0, ref mut b_1) = *b;
                    let t_red = &mut b_0[..width];
                    let t_depth = &mut b_1[..width];

                    // Copy row data
                    t_red.copy_from_slice(row_pixels);
                    let row_start = y * width;
                    t_depth.copy_from_slice(&depths[row_start..row_start + width]);

                    process_row(row_pixels, t_red, t_depth, width, config);
                });
            });
    }

    #[cfg(not(feature = "parallel"))]
    {
        pixels
            .chunks_exact_mut(width)
            .enumerate()
            .for_each(|(y, row_pixels)| {
                // Copy row data
                temp_row_red.copy_from_slice(row_pixels);
                let row_start = y * width;
                temp_row_depth.copy_from_slice(&depths[row_start..row_start + width]);

                process_row(row_pixels, &temp_row_red, &temp_row_depth, width, config);
            });
    }
}

fn process_row(
    row_pixels: &mut [u32],
    orig_red: &[u32],
    depths: &[f32],
    width: usize,
    config: AnaglyphConfig,
) {
    for x in 0..width {
        let current_pixel = orig_red[x];
        // Green and Blue stay at current pixel
        let g = (current_pixel >> 8) & 0xFF;
        let b = current_pixel & 0xFF;
        let a = (current_pixel >> 24) & 0xFF;

        // Find which pixel's red channel should land on 'x'.
        // We do a reverse lookup or a forward write.
        // It's easier to do a forward write by accumulating reds into a buffer,
        // or for each pixel x, look around to see which pixel would shift its red here.
        // A forward pass is much better. Since we already have the original row, we can just clear the row
        // and build it. Wait, the `orig_red` has the original colors.

        // Let's modify the algorithm to a forward write.
        // But we are iterating over `row_pixels` which we want to write to once.
        // Okay, reverse lookup:
        // What pixel x_src has a shift such that x_src + shift = x?
        // Since depth is not monotonic, multiple pixels might shift their red here.
        // The one closest to the camera (lowest depth) should win.

        let mut min_depth = std::f32::INFINITY;
        let mut best_r = (current_pixel >> 16) & 0xFF; // Default to own red if no one shifts here

        // Search window: we only need to look within +/- max_offset
        let max_off = config.max_offset as i32;
        let x_i32 = x as i32;
        let start_x = (x_i32 - max_off).max(0) as usize;
        let end_x = (x_i32 + max_off).min((width - 1) as i32) as usize;

        for src_x in start_x..=end_x {
            let d = depths[src_x];
            if d.is_infinite() {
                continue;
            }

            // Calculate shift for src_x
            // Formula: shift = (focal_depth - depth) * scale
            // Let's map depth to offset.
            // If depth < focal_depth (closer), positive shift (Red moves right).
            // If depth > focal_depth (farther), negative shift (Red moves left).

            // To prevent division by zero or huge shifts, we can use a simple inverse relationship
            // or linear relationship if depths are relatively constrained.
            // A simple approximation: shift = config.max_offset * (1.0 - (depth / config.focal_depth))
            // Clamp shift to [-max_offset, max_offset]

            let mut normalized_shift = 1.0 - (d / config.focal_depth);
            normalized_shift = normalized_shift.clamp(-1.0, 1.0);

            #[allow(clippy::cast_possible_truncation)]
            let shift = (normalized_shift * config.max_offset as f32).round() as i32;

            if (src_x as i32 + shift) == x_i32 {
                // This src_x shifts its red to x
                if d < min_depth {
                    min_depth = d;
                    best_r = (orig_red[src_x] >> 16) & 0xFF;
                }
            }
        }

        // If no pixel shifted its red here, we should probably keep our own red
        // OR we should be cyan (no red). Standard anaglyph behavior: if a background
        // pixel's red was moved away and nothing replaced it, it loses red.
        // But if the background is at infinity, its shift is -max_offset (or whatever it clamps to).
        // Let's refine the infinity handling: infinity depth means background.
        // Background shift should be negative max_offset.

        // Let's re-evaluate:
        // If we didn't find any winning depth, it means no geometry's red shifted here.
        // Let's also check the background (infinity).
        if min_depth == std::f32::INFINITY {
            // Check if infinity background shifted here
            for src_x in start_x..=end_x {
                let d = depths[src_x];
                if d.is_infinite() {
                    let shift = -(config.max_offset as i32); // Far away -> negative shift
                    if (src_x as i32 + shift) == x_i32 {
                        best_r = (orig_red[src_x] >> 16) & 0xFF;
                        break;
                    }
                }
            }
            // If STILL nothing, best_r is 0 (cyan) because the red moved away.
            // But wait, the background might have shifted away, leaving a "hole".
            // We can just leave it 0 or use the original red if we want less artifacting.
            // We'll set it to 0 to be strictly correct physically (red light moved).
            // Actually, best_r defaults to own red. But if own red shifted away, it should be 0.

            // Did our OWN red shift away?
            let my_d = depths[x];
            let my_shift = if my_d.is_infinite() {
                -(config.max_offset as i32)
            } else {
                let mut ns = 1.0 - (my_d / config.focal_depth);
                ns = ns.clamp(-1.0, 1.0);
                (ns * config.max_offset as f32).round() as i32
            };

            if my_shift != 0 {
                // Our red moved away, and nobody replaced it.
                best_r = 0;
            }
        }

        row_pixels[x] = (a << 24) | (best_r << 16) | (g << 8) | b;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_anaglyph() {
        let mut fb = Framebuffer::new(10, 1).unwrap();
        let mut zb = ZBuffer::new(10, 1).unwrap();

        // Fill with white
        fb.clear(0xFF_FFFFFF);
        // Set middle pixel depth to be closer than focal depth
        zb.test_and_set(5, 0, 1.0); // Focus plane is at 2.0, this is closer.

        let config = AnaglyphConfig {
            max_offset: 2,
            focal_depth: 2.0,
        };

        apply_anaglyph(&mut fb, &zb, config);

        // The red channel of pixel 5 should have shifted right (or left, depending on convention)
        // leaving pixel 5 looking cyan, and the destination pixel looking red (if background was dark)
        // Since background is white, moving red away from 5 means 5 loses red -> cyan (0xFF_00FFFF).
        let p5 = fb.get_pixel(5, 0).unwrap();

        // Assert it's no longer white, but cyan
        assert_eq!(
            p5, 0xFF_00FFFF,
            "Pixel at depth < focal should lose its red channel and become cyan"
        );
    }
}
