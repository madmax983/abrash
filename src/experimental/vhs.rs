//! VHS Glitch Effect Module
//!
//! Simulates a degraded VHS tape playback effect on a `Framebuffer`.
//! Features:
//! - **Scanline Jitter**: Horizontal shifting of rows based on sine waves and noise.
//! - **Chromatic Aberration**: RGB channels are split and offset horizontally.
//! - **Grain/Noise**: Random intensity variations per pixel.

use crate::framebuffer::Framebuffer;

/// A simple Xorshift random number generator for deterministic noise.
///
/// Copied here to keep the module self-contained and dependency-free.
struct XorShift32 {
    state: u32,
}

impl XorShift32 {
    const fn new(seed: u32) -> Self {
        // Ensure non-zero seed
        Self {
            state: if seed == 0 { 0xDEAD_BEEF } else { seed },
        }
    }

    const fn next(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// Returns a float in [0.0, 1.0)
    fn next_f32(&mut self) -> f32 {
        (self.next() as f32) / (u32::MAX as f32)
    }
}

/// Applies a VHS glitch effect to the framebuffer in-place.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify.
/// * `time` - A time parameter (e.g., frame count) used to animate the effect.
///
/// # Examples
///
/// ```
/// use abrash::framebuffer::Framebuffer;
/// use abrash::experimental::vhs::apply_vhs_glitch;
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// apply_vhs_glitch(&mut fb, 12345);
/// ```
pub fn apply_vhs_glitch(fb: &mut Framebuffer, time: u32) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    // Clone the original buffer to read from while writing to `fb`.
    // This is necessary because the effect displaces pixels.
    let source_pixels = fb.as_slice().to_vec();
    let dest_pixels = fb.as_mut_slice();

    let mut rng = XorShift32::new(time.wrapping_mul(123_456_789).wrapping_add(time));

    // Jitter parameters
    let time_f = time as f32;
    let jitter_amplitude = 5.0 + (rng.next_f32() * 5.0); // Random jitter intensity
    let jitter_frequency = 0.05;

    // Chromatic aberration offsets
    // R: -offset, G: 0, B: +offset
    // We vary the offset slightly over time
    let aber_offset = (3.0 + (time_f * 0.1).sin() * 2.0).round() as isize;

    for y in 0..height {
        // Calculate horizontal scanline offset (jitter)
        // Combine a slow wave with fast noise
        let wave_offset = (y as f32 * jitter_frequency + time_f * 0.2).sin() * jitter_amplitude;

        // Occasional "tracking error" glitch lines
        let tracking_error = if rng.next_f32() > 0.98 {
            (rng.next_f32() - 0.5) * 50.0
        } else {
            0.0
        };

        let row_offset = (wave_offset + tracking_error) as isize;

        for x in 0..width {
            let idx = y * width + x;

            // Calculate source coordinates for each channel
            // We clamp or wrap x. Clamping is simpler for now.

            // Red Channel
            let r_x = (x as isize + row_offset - aber_offset).clamp(0, (width - 1) as isize) as usize;
            let r_src_idx = y * width + r_x;
            let r_col = source_pixels[r_src_idx];
            let r = (r_col >> 16) & 0xFF;

            // Green Channel
            let g_x = (x as isize + row_offset).clamp(0, (width - 1) as isize) as usize;
            let g_src_idx = y * width + g_x;
            let g_col = source_pixels[g_src_idx];
            let g = (g_col >> 8) & 0xFF;

            // Blue Channel
            let b_x = (x as isize + row_offset + aber_offset).clamp(0, (width - 1) as isize) as usize;
            let b_src_idx = y * width + b_x;
            let b_col = source_pixels[b_src_idx];
            let b = b_col & 0xFF;

            // Add simple noise
            let noise = ((rng.next_f32() - 0.5) * 20.0) as i32;

            let r_final = (r as i32 + noise).clamp(0, 255) as u32;
            let g_final = (g as i32 + noise).clamp(0, 255) as u32;
            let b_final = (b as i32 + noise).clamp(0, 255) as u32;

            // Preserve original alpha from the unshifted pixel
            let original_alpha = source_pixels[idx] & 0xFF00_0000;

            dest_pixels[idx] = original_alpha | (r_final << 16) | (g_final << 8) | b_final;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vhs_glitch_runs() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        // Fill with a pattern
        for y in 0..100 {
            for x in 0..100 {
                fb.set_pixel(x, y, 0xFFFF_0000); // Red
            }
        }

        apply_vhs_glitch(&mut fb, 0);

        // Check that pixels are modified (noise should change exact values)
        // But red should still be dominant if we started with red
        let p = fb.get_pixel(50, 50).unwrap();
        let r = (p >> 16) & 0xFF;
        // Noise is +/- 10, so R should be close to 255 (clamped)
        assert!(r > 200, "Red channel should remain high, got {r}");
    }

    #[test]
    fn test_rng_determinism() {
        let mut rng1 = XorShift32::new(123);
        let mut rng2 = XorShift32::new(123);

        assert_eq!(rng1.next(), rng2.next());
        assert_eq!(rng1.next(), rng2.next());
    }
}
