//! Shared utilities for random number generation and pixel manipulation.
//!
//! This module reduces duplication across the codebase.

/// A simple Xorshift random number generator for deterministic noise.
///
/// Features:
/// - Fast, non-cryptographic RNG.
/// - Deterministic (same seed produces same sequence).
/// - 32-bit state.
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
    pub fn next_u32(&mut self) -> u32 {
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
#[inline(always)]
#[must_use]
pub const fn pixel_luminance(pixel: u32) -> u8 {
    let r = (pixel >> 16) & 0xFF;
    let g = (pixel >> 8) & 0xFF;
    let b = pixel & 0xFF;

    // Fixed-point calculation: (77*R + 150*G + 29*B) >> 8
    ((77 * r + 150 * g + 29 * b) >> 8) as u8
}
