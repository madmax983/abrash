#![allow(dead_code)]
#![allow(clippy::suboptimal_flops)]
#![allow(clippy::should_panic_without_expect)]
#![allow(clippy::ignore_without_reason)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::float_cmp)]
//! Fast, high-quality pseudo-random number generation.
//!
//! [`Pcg32`] implements the **PCG-XSH-RR** algorithm — a 64-bit state LCG
//! combined with an xor-shift and rotate permutation output function.
//! It passes `BigCrush`, produces 32 bits per call, and costs ~1-2 cycles.
//!
//! [`Rng`] is a thin wrapper that adds ergonomic sampling methods on top.
//!
//! # Examples
//!
//! ```
//! use abrash_core::random::Rng;
//!
//! let mut rng = Rng::seeded(42);
//!
//! let v = rng.f32();           // [0, 1)
//! let n = rng.i32_range(-5, 5); // [-5, 4]
//! let coin: bool = rng.bool();
//! ```
//!
//! # Reproducibility
//!
//! Given the same seed, output is deterministic across platforms and Rust versions
//! (the algorithm is fixed, not stdlib-dependent).
//!
//! # Reference
//!
//! O'Neill, M. E. (2014). *PCG: A Family of Simple Fast Space-Efficient
//! Statistically Good Algorithms for Random Number Generation.*

/// PCG-XSH-RR: 64-bit state, 32-bit output.
///
/// Multiple independent streams are supported via the `inc` (stream selector)
/// parameter — two generators with the same seed but different `inc` values
/// produce independent sequences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pcg32 {
    state: u64,
    inc: u64, // must be odd
}

impl Pcg32 {
    /// Multiplier from the PCG specification.
    const MULT: u64 = 6_364_136_223_846_793_005;

    /// Construct with explicit `seed` and `stream` selector.
    ///
    /// `stream` selects among 2⁶³ independent sequences; the default stream
    /// is `1` (see [`Pcg32::seeded`]).
    #[must_use]
    pub const fn new(seed: u64, stream: u64) -> Self {
        let inc = stream.wrapping_shl(1) | 1; // ensure inc is odd
        let mut rng = Self { state: 0, inc };
        rng.state = rng.state.wrapping_add(inc);
        rng.state = rng.state.wrapping_mul(Self::MULT).wrapping_add(inc);
        // Discard one output to separate seed influence from first sample
        let _ = rng.next_u32();
        rng.state = seed.wrapping_add(inc);
        rng.state = rng.state.wrapping_mul(Self::MULT).wrapping_add(inc);
        let _ = rng.next_u32();
        rng
    }

    /// Construct from a single seed value (stream 1).
    #[must_use]
    pub const fn seeded(seed: u64) -> Self {
        Self::new(seed, 1)
    }

    /// Advance state and return the next 32-bit output.
    ///
    /// The PCG-XSH-RR permutation: xor-shift the high bits down, rotate by
    /// the top 5 bits to eliminate the fixed-point artifact of pure xorshift.
    #[inline]
    pub const fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(Self::MULT).wrapping_add(self.inc);
        // XSH-RR output function
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Advance state and return the next 64-bit output (two 32-bit calls).
    #[inline]
    pub const fn next_u64(&mut self) -> u64 {
        #[allow(clippy::cast_lossless)]
        let lo = self.next_u32() as u64;
        #[allow(clippy::cast_lossless)]
        let hi = self.next_u32() as u64;
        (hi << 32) | lo
    }

    /// Skip `delta` outputs forward (or backward with wrapping) in O(log n).
    ///
    /// Useful for parallel work: seed once, advance each lane to its starting
    /// position instead of generating sequentially.
    pub const fn advance(&mut self, delta: u64) {
        let mut cur_mult = Self::MULT;
        let mut cur_plus = self.inc;
        let mut acc_mult = 1u64;
        let mut acc_plus = 0u64;
        let mut d = delta;
        while d > 0 {
            if d & 1 != 0 {
                acc_mult = acc_mult.wrapping_mul(cur_mult);
                acc_plus = acc_plus.wrapping_mul(cur_mult).wrapping_add(cur_plus);
            }
            cur_plus = cur_mult.wrapping_add(1).wrapping_mul(cur_plus);
            cur_mult = cur_mult.wrapping_mul(cur_mult);
            d >>= 1;
        }
        self.state = acc_mult.wrapping_mul(self.state).wrapping_add(acc_plus);
    }
}

// ── Ergonomic wrapper ─────────────────────────────────────────────────────────

/// Ergonomic sampling on top of [`Pcg32`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rng(Pcg32);

impl Rng {
    /// Construct with seed and stream `1`.
    #[must_use]
    pub const fn seeded(seed: u64) -> Self {
        Self(Pcg32::seeded(seed))
    }

    /// Construct with explicit seed and stream.
    #[must_use]
    pub const fn with_stream(seed: u64, stream: u64) -> Self {
        Self(Pcg32::new(seed, stream))
    }

    /// Raw 32-bit output.
    #[inline]
    pub const fn u32(&mut self) -> u32 {
        self.0.next_u32()
    }

    /// Raw 64-bit output.
    #[inline]
    pub const fn u64(&mut self) -> u64 {
        self.0.next_u64()
    }

    /// Uniform `f32` in `[0, 1)`.
    #[inline]
    pub fn f32(&mut self) -> f32 {
        // Use top 24 bits for mantissa precision (f32 has 23-bit mantissa).
        let bits = self.u32() >> 8;
        bits as f32 * (1.0 / (1u32 << 24) as f32)
    }

    /// Uniform `f32` in `[lo, hi)`.
    ///
    /// # Panics
    ///
    /// Panics (debug-only) if `lo >= hi`.
    #[inline]
    pub fn f32_range(&mut self, lo: f32, hi: f32) -> f32 {
        debug_assert!(lo < hi, "f32_range: lo must be < hi");
        lo + self.f32() * (hi - lo)
    }

    /// Uniform `f64` in `[0, 1)`.
    #[inline]
    pub fn f64(&mut self) -> f64 {
        let bits = self.u64() >> 11;
        bits as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Uniform `u32` in `[0, bound)` using Daniel Lemire's fast bounded method.
    ///
    /// Bias-free alternative to `% bound`.
    ///
    /// # Panics
    ///
    /// Panics if `bound == 0`.
    #[inline]
    pub fn u32_below(&mut self, bound: u32) -> u32 {
        assert!(bound > 0, "u32_below: bound must be > 0");
        // Lemire's nearly-divisionless algorithm
        let mut x = u64::from(self.u32());
        let mut m = x * u64::from(bound);
        let mut lo = m as u32;
        if lo < bound {
            let threshold = bound.wrapping_neg() % bound;
            while lo < threshold {
                x = u64::from(self.u32());
                m = x * u64::from(bound);
                lo = m as u32;
            }
        }
        (m >> 32) as u32
    }

    /// Uniform `i32` in `[lo, hi]` (inclusive both ends).
    ///
    /// # Panics
    ///
    /// Panics if `lo > hi`.
    #[inline]
    pub fn i32_range(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi, "i32_range: lo must be <= hi");
        let range = (i64::from(hi) - i64::from(lo) + 1) as u32;
        lo + self.u32_below(range) as i32
    }

    /// Returns `true` with probability `0.5`.
    #[inline]
    pub const fn bool(&mut self) -> bool {
        self.u32() & 1 != 0
    }

    /// Returns `true` with probability `p ∈ [0, 1]`.
    #[inline]
    pub fn bool_prob(&mut self, p: f32) -> bool {
        self.f32() < p
    }

    /// Shuffle a slice in place using Fisher-Yates.
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        let n = slice.len();
        for i in (1..n).rev() {
            let j = self.u32_below(i as u32 + 1) as usize;
            slice.swap(i, j);
        }
    }

    /// Pick a random element from a non-empty slice.
    ///
    /// # Panics
    ///
    /// Panics if `slice` is empty.
    #[inline]
    pub fn choose<'a, T>(&mut self, slice: &'a [T]) -> &'a T {
        assert!(!slice.is_empty(), "choose: slice must not be empty");
        &slice[self.u32_below(slice.len() as u32) as usize]
    }

    /// Sample from a unit disk (uniform distribution).
    ///
    /// Returns `(x, y)` with `x² + y² < 1`.
    pub fn in_unit_disk(&mut self) -> (f32, f32) {
        loop {
            let x = self.f32_range(-1.0, 1.0);
            let y = self.f32_range(-1.0, 1.0);
            if x * x + y * y < 1.0 {
                return (x, y);
            }
        }
    }

    /// Sample from a unit sphere surface (uniform distribution).
    ///
    /// Uses the Marsaglia method — rejection sample in a cube, then normalize.
    /// Returns `(x, y, z)` on the unit sphere.
    pub fn on_unit_sphere(&mut self) -> (f32, f32, f32) {
        loop {
            let x = self.f32_range(-1.0, 1.0);
            let y = self.f32_range(-1.0, 1.0);
            let z = self.f32_range(-1.0, 1.0);
            let d2 = x * x + y * y + z * z;
            if d2 > 0.0 && d2 < 1.0 {
                let inv = 1.0 / d2.sqrt();
                return (x * inv, y * inv, z * inv);
            }
        }
    }

    /// Access the underlying [`Pcg32`] for bulk generation or stream skipping.
    #[must_use]
    pub const fn inner(&mut self) -> &mut Pcg32 {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_with_same_seed() {
        let mut a = Rng::seeded(12345);
        let mut b = Rng::seeded(12345);
        for _ in 0..100 {
            assert_eq!(a.u32(), b.u32());
        }
    }

    #[test]
    fn different_seeds_differ() {
        let mut a = Rng::seeded(1);
        let mut b = Rng::seeded(2);
        let va: Vec<u32> = (0..10).map(|_| a.u32()).collect();
        let vb: Vec<u32> = (0..10).map(|_| b.u32()).collect();
        assert_ne!(va, vb);
    }

    #[test]
    fn f32_in_range() {
        let mut rng = Rng::seeded(42);
        for _ in 0..10_000 {
            let v = rng.f32();
            assert!(v >= 0.0 && v < 1.0, "f32 out of [0,1): {v}");
        }
    }

    #[test]
    fn f32_range_respects_bounds() {
        let mut rng = Rng::seeded(99);
        for _ in 0..10_000 {
            let v = rng.f32_range(-5.0, 5.0);
            assert!(v >= -5.0 && v < 5.0, "out of range: {v}");
        }
    }

    #[test]
    fn u32_below_bias_free() {
        let mut rng = Rng::seeded(7);
        let bound = 6u32;
        let mut counts = [0u32; 6];
        for _ in 0..60_000 {
            counts[rng.u32_below(bound) as usize] += 1;
        }
        // Each bucket should be ~10000; allow 15% deviation
        for (i, &c) in counts.iter().enumerate() {
            assert!(
                c > 8_500 && c < 11_500,
                "bucket {i} count {c} deviates too much"
            );
        }
    }

    #[test]
    fn i32_range_inclusive() {
        let mut rng = Rng::seeded(0);
        let mut saw_min = false;
        let mut saw_max = false;
        for _ in 0..10_000 {
            let v = rng.i32_range(-2, 2);
            assert!(v >= -2 && v <= 2);
            if v == -2 {
                saw_min = true;
            }
            if v == 2 {
                saw_max = true;
            }
        }
        assert!(saw_min, "never sampled min");
        assert!(saw_max, "never sampled max");
    }

    #[test]
    fn bool_roughly_half() {
        let mut rng = Rng::seeded(1);
        let trues = (0..10_000).filter(|_| rng.bool()).count();
        assert!(
            trues > 4_500 && trues < 5_500,
            "bool skewed: {trues}/10000 true"
        );
    }

    #[test]
    fn shuffle_is_permutation() {
        let mut rng = Rng::seeded(42);
        let mut v: Vec<i32> = (0..20).collect();
        let orig = v.clone();
        rng.shuffle(&mut v);
        // Same elements, different order
        let mut sorted = v.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, orig);
        assert_ne!(
            v, orig,
            "shuffle didn't change order (astronomically unlikely)"
        );
    }

    #[test]
    fn choose_covers_all_elements() {
        let mut rng = Rng::seeded(3);
        let arr = [1, 2, 3, 4, 5];
        let mut seen = [false; 5];
        for _ in 0..10_000 {
            let v = *rng.choose(&arr);
            seen[(v - 1) as usize] = true;
        }
        assert!(seen.iter().all(|&s| s));
    }

    #[test]
    fn advance_matches_sequential() {
        let mut a = Rng::seeded(55);
        // Consume 100 values
        for _ in 0..100 {
            a.u32();
        }
        // Start fresh and jump 100 forward
        let mut b = Rng::seeded(55);
        b.inner().advance(100);
        assert_eq!(a.u32(), b.u32());
    }

    #[test]
    fn in_unit_disk_inside() {
        let mut rng = Rng::seeded(10);
        for _ in 0..1000 {
            let (x, y) = rng.in_unit_disk();
            assert!(x * x + y * y < 1.0);
        }
    }

    #[test]
    fn on_unit_sphere_unit_length() {
        let mut rng = Rng::seeded(20);
        for _ in 0..1000 {
            let (x, y, z) = rng.on_unit_sphere();
            let len = (x * x + y * y + z * z).sqrt();
            assert!((len - 1.0).abs() < 1e-5, "len={len}");
        }
    }

    #[test]
    fn streams_independent() {
        let mut a = Rng::with_stream(42, 1);
        let mut b = Rng::with_stream(42, 2);
        let va: Vec<u32> = (0..10).map(|_| a.u32()).collect();
        let vb: Vec<u32> = (0..10).map(|_| b.u32()).collect();
        assert_ne!(va, vb);
    }
}
