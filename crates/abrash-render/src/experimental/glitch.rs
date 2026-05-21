//! Glitch / Datamosh Effect
//!
//! A retro post-processing effect that simulates digital corruption,
//! RGB channel separation, and horizontal row shifting based on time.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::XorShift32;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Applies a glitch/datamosh effect to the framebuffer.
///
/// This effect randomly shifts pixel rows horizontally and offsets
/// the red, green, and blue color channels independently based on
/// a pseudo-random noise function driven by `time`.
///
/// # Arguments
///
/// * `fb` - The framebuffer to apply the effect to.
/// * `intensity` - How strong the glitch effect is (0.0 to 1.0).
/// * `time` - A continuously increasing time value used to seed the random noise.
pub fn apply_glitch(fb: &mut Framebuffer, intensity: f32, time: f32) {
    if intensity <= 0.0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    // Bolt Performance Optimization:
    // Eliminate the massive `Vec` heap allocation per frame by caching the source buffer
    // in a `thread_local`. This read-only buffer prevents mutable aliasing issues when
    // chunks are processed in parallel.
    thread_local! {
        static SOURCE_PIXELS: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    SOURCE_PIXELS.with(|buf| {
        let mut src_pixels = buf.borrow_mut();
        // ⚡ Bolt: Use `resize` and `copy_from_slice` instead of `clear()` followed by `extend_from_slice()`.
        // This avoids iterative bounds and capacity checks in `extend` and utilizes a fast `memcpy`.
        src_pixels.resize(fb.as_slice().len(), 0);
        src_pixels.copy_from_slice(fb.as_slice());

        let src_slice = src_pixels.as_slice();
        let dest_pixels = fb.as_mut_slice();

        // Scale intensity to maximum possible pixel shifts
        let max_shift = (width as f32 * 0.1 * intensity) as i32;
        let channel_shift_max = (width as f32 * 0.05 * intensity) as i32;

        // Seed the PRNG with the current time (scaled and converted to u32)
        // Add an arbitrary prime offset to ensure it's never 0
        let global_seed = (time * 1000.0) as u32 ^ 0x1337_BEEF;

        #[cfg(feature = "parallel")]
        let row_iter = dest_pixels.par_chunks_exact_mut(width).enumerate();
        #[cfg(not(feature = "parallel"))]
        let row_iter = dest_pixels.chunks_exact_mut(width).enumerate();

        row_iter.for_each(|(y, row)| {
            // Initialize a row-specific PRNG using the global seed and row index.
            // This ensures the noise is deterministic per row for Rayon parallelization
            // but changes over time.
            let mut prng_state = global_seed.wrapping_add((y as u32).wrapping_mul(7919));
            if prng_state == 0 {
                prng_state = 1; // Prevent XorShift from getting stuck at 0
            }
            let mut prng = XorShift32::new(prng_state);

            // Block-based glitching (groups of rows glitch together)
            // We simulate this by blending the row index into the PRNG differently
            let block_idx = y / 10;
            let mut block_prng = XorShift32::new(
                global_seed
                    .wrapping_add((block_idx as u32).wrapping_mul(31337))
                    .max(1),
            );
            let is_glitched = (block_prng.next_u32() % 100) as f32 / 100.0 < intensity;

            if !is_glitched {
                // Fast path: Just copy the original row back if it's not affected
                let src_offset = y * width;
                row.copy_from_slice(&src_slice[src_offset..src_offset + width]);
                return;
            }

            // Calculate shifts for this specific glitched row
            // `prng.next_u32() % N` isn't perfectly uniform, but fine for glitch effects.

            // Random horizontal shift for the entire row (-max_shift to +max_shift)
            let row_shift =
                (prng.next_u32() % (max_shift.max(1) as u32 * 2 + 1)) as i32 - max_shift;

            // Random color channel offsets
            let r_shift = (prng.next_u32() % (channel_shift_max.max(1) as u32 * 2 + 1)) as i32
                - channel_shift_max;
            let g_shift = (prng.next_u32() % (channel_shift_max.max(1) as u32 * 2 + 1)) as i32
                - channel_shift_max;
            let b_shift = (prng.next_u32() % (channel_shift_max.max(1) as u32 * 2 + 1)) as i32
                - channel_shift_max;

            for (x, pixel) in row.iter_mut().enumerate() {
                // Base x coordinate after the entire row is shifted
                let base_x = x as i32 - row_shift;

                // Sample each channel independently
                let sample_r =
                    get_channel_safe(src_slice, width, height, base_x - r_shift, y as i32, 16);
                let sample_g =
                    get_channel_safe(src_slice, width, height, base_x - g_shift, y as i32, 8);
                let sample_b =
                    get_channel_safe(src_slice, width, height, base_x - b_shift, y as i32, 0);

                // Reconstruct the ARGB pixel
                *pixel = 0xFF00_0000 | (sample_r << 16) | (sample_g << 8) | sample_b;
            }
        });
    });
}

/// Helper function to safely sample a specific color channel from the source buffer,
/// clamping to the edges of the row if the shift goes out of bounds.
#[inline(always)]
fn get_channel_safe(src: &[u32], width: usize, _height: usize, x: i32, y: i32, shift: u8) -> u32 {
    let clamped_x = x.max(0).min(width as i32 - 1) as usize;
    let idx = y as usize * width + clamped_x;
    (src[idx] >> shift) & 0xFF
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_apply_glitch_changes_buffer() {
        let width = 100;
        let height = 100;
        let mut fb = Framebuffer::new(width, height).unwrap();

        // Draw a vertical white line down the middle on a black background
        fb.clear(0xFF00_0000);
        for y in 0..height {
            fb.set_pixel(50, y as i32, 0xFFFF_FFFF);
        }

        let mut fb_clone = Framebuffer::new(width, height).unwrap();
        fb_clone.as_mut_slice().copy_from_slice(fb.as_slice());

        // Apply a strong glitch effect
        apply_glitch(&mut fb, 1.0, 42.0);

        // Verify the buffer was modified
        let mut different = false;
        for i in 0..(width * height) as usize {
            if fb.as_slice()[i] != fb_clone.as_slice()[i] {
                different = true;
                break;
            }
        }
        assert!(different, "Glitch filter did not modify the framebuffer");
    }

    #[test]
    fn test_apply_glitch_zero_intensity() {
        let width = 10;
        let height = 10;
        let mut fb = Framebuffer::new(width, height).unwrap();
        fb.clear(0xFF12_3456);

        apply_glitch(&mut fb, 0.0, 1.0);

        // Buffer should be untouched
        for &pixel in fb.as_slice() {
            assert_eq!(pixel, 0xFF12_3456);
        }
    }
}
