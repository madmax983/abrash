//! Shared utilities for Abrash.
//!
//! Contains common helpers like Random Number Generation and Color Utilities.

/// A simple Xorshift random number generator for deterministic noise.
#[derive(Debug, Clone, Copy)]
pub struct XorShift32 {
    state: u32,
}

impl XorShift32 {
    /// Create a new RNG with the given seed.
    ///
    /// # Arguments
    ///
    /// * `seed` - The initial seed. If 0 is provided, a default non-zero seed is used.
    #[must_use]
    pub const fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0xDEAD_BEEF } else { seed },
        }
    }

    /// Generate the next random `u32`.
    pub const fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// Generate the next random `f32` in the range `[0.0, 1.0)`.
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32) / (u32::MAX as f32)
    }

    /// Generate the next random `f32` in the range `[-1.0, 1.0)`.
    pub fn next_signed_f32(&mut self) -> f32 {
        self.next_f32() * 2.0 - 1.0
    }
}

/// Calculates the luminance of a pixel using standard weights (Rec. 601).
///
/// Formula: `Y = 0.299*R + 0.587*G + 0.114*B`
/// Approximated as: `Y = (77*R + 150*G + 29*B) >> 8`
///
/// # Arguments
///
/// * `pixel` - A pixel in `0xAARRGGBB` format.
///
/// # Returns
///
/// The luminance value as a `u8` (0-255).
#[must_use]
#[inline]
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
        let mut rng1 = XorShift32::new(12345);
        let mut rng2 = XorShift32::new(12345);

        assert_eq!(rng1.next_u32(), rng2.next_u32());
        assert_eq!(rng1.next_f32(), rng2.next_f32());
    }

    #[test]
    fn test_pixel_luminance() {
        // White
        assert_eq!(pixel_luminance(0xFFFFFFFF), 255);
        // Black
        assert_eq!(pixel_luminance(0xFF000000), 0);
        // Red (pure) -> ~76
        assert_eq!(pixel_luminance(0xFFFF0000), 76);
        // Green (pure) -> ~149
        assert_eq!(pixel_luminance(0xFF00FF00), 149);
    }
}
