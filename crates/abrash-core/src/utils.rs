#![allow(dead_code)]
#![allow(clippy::suboptimal_flops)]
#![allow(clippy::should_panic_without_expect)]
#![allow(clippy::ignore_without_reason)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::float_cmp)]
//! Shared utilities for random number generation and pixel manipulation.
//!
//! This module reduces duplication across the codebase.

/// A simple Xorshift random number generator for deterministic noise.
///
/// Features:
/// - Fast, non-cryptographic RNG.
/// - Deterministic (same seed produces same sequence).
/// - 32-bit state.
///
/// # Examples
///
/// ```
/// use abrash_core::utils::XorShift32;
///
/// let mut rng = XorShift32::new(12345);
/// let val = rng.next_f32();
/// assert!(val >= 0.0 && val < 1.0);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct XorShift32 {
    state: u32,
}

impl XorShift32 {
    /// Creates a new RNG with the given seed.
    ///
    /// If seed is 0, it defaults to `0xDEAD_BEEF` to ensure a non-zero state.
    #[must_use]
    pub const fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0xDEAD_BEEF } else { seed },
        }
    }

    /// Generates the next random `u32`.
    pub const fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// Generates a random float in the range `[0.0, 1.0)`.
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32) / (u32::MAX as f32)
    }

    /// Generates a random float in the range `[-1.0, 1.0)`.
    pub fn next_f32_signed(&mut self) -> f32 {
        self.next_f32() * 2.0 - 1.0
    }
}

/// Calculates the luminance of a 32-bit ARGB pixel.
///
/// Uses the Rec. 601 luma coefficients:
/// `Y = 0.299*R + 0.587*G + 0.114*B`
///
/// Approximated as: `Y = (77*R + 150*G + 29*B) >> 8`
///
/// # Examples
///
/// ```
/// use abrash_core::utils::pixel_luminance;
///
/// let white_luma = pixel_luminance(0xFFFFFFFF);
/// assert_eq!(white_luma, 255);
/// ```
#[inline(always)]
#[must_use]
pub const fn pixel_luminance(pixel: u32) -> u8 {
    let r = (pixel >> 16) & 0xFF;
    let g = (pixel >> 8) & 0xFF;
    let b = pixel & 0xFF;

    // Fixed-point calculation: (77*R + 150*G + 29*B) >> 8
    ((77 * r + 150 * g + 29 * b) >> 8) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xorshift_determinism() {
        let seed = 12345;
        let mut rng1 = XorShift32::new(seed);
        let mut rng2 = XorShift32::new(seed);

        for _ in 0..100 {
            assert_eq!(rng1.next_u32(), rng2.next_u32());
        }
    }

    #[test]
    fn test_xorshift_zero_seed() {
        let mut rng = XorShift32::new(0);
        // Should not be 0, otherwise it stays 0
        assert_ne!(rng.next_u32(), 0);
    }

    #[test]
    fn test_pixel_luminance() {
        // Black
        assert_eq!(pixel_luminance(0xFF000000), 0);
        // White
        assert_eq!(pixel_luminance(0xFFFFFFFF), 255); // (77*255 + 150*255 + 29*255) >> 8 = (256*255) >> 8 = 255

        // Red
        let r = pixel_luminance(0xFFFF0000);
        // 77*255 >> 8 = 19635 >> 8 = 76.69 -> 76
        assert_eq!(r, 76);

        // Green
        let g = pixel_luminance(0xFF00FF00);
        // 150*255 >> 8 = 38250 >> 8 = 149.4 -> 149
        assert_eq!(g, 149);

        // Blue
        let b = pixel_luminance(0xFF0000FF);
        // 29*255 >> 8 = 7395 >> 8 = 28.88 -> 28
        assert_eq!(b, 28);

        // Sum roughly 253 (76+149+28 = 253), slight precision loss due to integer math is expected vs 255
        // But white test passed because (256*255) >> 8 is exactly 255.
    }
}
