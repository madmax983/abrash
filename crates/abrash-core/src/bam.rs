//! Binary Angle Measure (BAM) -- full-circle `u32` angle type.
//!
//! The full circle is `u32::MAX + 1` (2^32). All arithmetic is naturally
//! modular via `u32` wrapping, so there is no "angle clamping" needed.
//!
//! # Key difference from doom-rs
//!
//! The sine table is `const` (compile-time generated via Taylor series),
//! not `static mut` with runtime init. Zero unsafe, zero startup cost.
//!
//! # Verus invariant
//! `Bam` is always valid -- every `u32` bit pattern is a legal angle.
//! Additive inverse holds: `a.wrapping_add(b).wrapping_sub(b) == a`.

use crate::fixed16_16::Fixed16_16;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Number of entries in the fine-angle sine table (2048 * 4 quadrants).
pub const FINE_TABLE_SIZE: usize = 8192;

/// Shift to convert a `Bam` to a fine-angle index (0..8191).
/// `bam >> 19` gives an index covering 8192 steps in the full circle.
pub const BAM_TO_FINE_SHIFT: u32 = 32 - 13; // 19

// ---------------------------------------------------------------------------
// Const Taylor-series sine (f64, compile-time)
// ---------------------------------------------------------------------------

/// Normalize an angle in radians to the range `[-pi, pi]` using const-safe
/// arithmetic (no `f64::rem_euclid` or `f64::round` in const context).
const fn const_normalize_to_neg_pi_pi(x: f64) -> f64 {
    const TWO_PI: f64 = 6.283_185_307_179_586;
    const PI: f64 = 3.141_592_653_589_793;

    // Reduce to [-2pi, 2pi] via repeated subtraction/addition.
    // For our use case (0..2pi input range), one pass suffices, but we
    // handle the general case with a loop for robustness.
    let mut r = x;

    // Coarse reduction: subtract multiples of 2*pi.
    // floor(x / 2pi) -- const-safe integer truncation toward negative infinity.
    let n = (r / TWO_PI) as i64; // truncates toward zero
    r -= (n as f64) * TWO_PI;

    // Fine reduction: bring into [-pi, pi].
    if r > PI {
        r -= TWO_PI;
    } else if r < -PI {
        r += TWO_PI;
    }

    r
}

/// Compute `sin(x)` at compile time using a 13th-order Taylor series.
///
/// Accuracy: < 0.0001 absolute error for `|x| <= pi`.
const fn const_sin_f64(x: f64) -> f64 {
    let x = const_normalize_to_neg_pi_pi(x);

    // Taylor series: sin(x) = x - x^3/3! + x^5/5! - x^7/7! + x^9/9! - x^11/11! + x^13/13!
    let x2 = x * x;
    let x3 = x2 * x;
    let x5 = x3 * x2;
    let x7 = x5 * x2;
    let x9 = x7 * x2;
    let x11 = x9 * x2;
    let x13 = x11 * x2;

    x - x3 / 6.0 + x5 / 120.0 - x7 / 5040.0 + x9 / 362_880.0 - x11 / 39_916_800.0
        + x13 / 6_227_020_800.0
}

// ---------------------------------------------------------------------------
// Const sine table (compile-time generated)
// ---------------------------------------------------------------------------

/// 8192-entry sine table computed at compile time via Taylor series.
///
/// Index `i` maps to `sin(i * 2*pi / 8192)` stored as `Fixed16_16`.
const SINE_TABLE: [Fixed16_16; FINE_TABLE_SIZE] = {
    let mut table = [Fixed16_16(0); FINE_TABLE_SIZE];
    let mut i = 0;
    while i < FINE_TABLE_SIZE {
        let angle = (i as f64) * 6.283_185_307_179_586 / (FINE_TABLE_SIZE as f64);
        let sin_val = const_sin_f64(angle);
        table[i] = Fixed16_16((sin_val * 65536.0) as i32);
        i += 1;
    }
    table
};

// ---------------------------------------------------------------------------
// Bam struct
// ---------------------------------------------------------------------------

/// Binary Angle Measure: 2^32 = full circle.
///
/// Cardinal directions:
/// - `0x0000_0000` -- East (0 degrees)
/// - `0x4000_0000` -- North (90 degrees)
/// - `0x8000_0000` -- West (180 degrees)
/// - `0xC000_0000` -- South (270 degrees)
/// Binary Angle Measure (BAM) -- full-circle `u32` angle type.
///
/// The full circle is `u32::MAX + 1` (2^32). All arithmetic is naturally
/// modular via `u32` wrapping, so there is no "angle clamping" needed.
///
/// # Examples
///
/// ```
/// use abrash_core::bam::{Bam, ANG90};
///
/// let angle = Bam::ZERO;
/// let turned = angle + ANG90;
/// assert_eq!(turned, ANG90);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bam(pub u32);

/// 45 degrees in BAM units.
pub const ANG45: Bam = Bam(0x2000_0000);
/// 90 degrees.
pub const ANG90: Bam = Bam(0x4000_0000);
/// 180 degrees.
pub const ANG180: Bam = Bam(0x8000_0000);
/// 270 degrees.
pub const ANG270: Bam = Bam(0xC000_0000);

impl Bam {
    /// Zero angle (East).
    pub const ZERO: Self = Self(0);

    /// Create from a raw `u32`.
    #[inline]
    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Raw `u32` bit pattern.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Wrapping addition (always correct for angles).
    #[inline]
    #[must_use]
    pub const fn wrapping_add(self, rhs: Self) -> Self {
        Self(self.0.wrapping_add(rhs.0))
    }

    /// Wrapping subtraction.
    #[inline]
    #[must_use]
    pub const fn wrapping_sub(self, rhs: Self) -> Self {
        Self(self.0.wrapping_sub(rhs.0))
    }

    /// Negate (180-degree flip = additive inverse in modular arithmetic).
    #[inline]
    #[must_use]
    pub const fn negate(self) -> Self {
        Self(self.0.wrapping_neg())
    }

    /// Fine-angle index (0..8191) used for sin/cos table lookup.
    #[inline]
    #[must_use]
    pub const fn fine_angle(self) -> usize {
        (self.0 >> BAM_TO_FINE_SHIFT) as usize
    }

    /// Fast sin/cos via the compile-time `Fixed16_16` table.
    ///
    /// This is the hot path for raycasting: two table lookups, zero
    /// floating-point work.
    #[inline]
    #[must_use]
    pub const fn sin_cos_fixed(self) -> (Fixed16_16, Fixed16_16) {
        let sin = SINE_TABLE[self.fine_angle()];
        // cos(x) = sin(x + 90 degrees)
        let cos_angle = Self(self.0.wrapping_add(ANG90.0));
        let cos = SINE_TABLE[cos_angle.fine_angle()];
        (sin, cos)
    }

    ///
    /// Slightly less precise than the table but avoids cache pressure.
    /// Good for effects, particles, and non-critical paths.
    #[inline]
    #[must_use]
    pub fn sin_cos_f32(self) -> (f32, f32) {
        self.to_radians().sin_cos()
    }

    /// Convert to radians (`f32`).
    #[inline]
    #[must_use]
    pub fn to_radians(self) -> f32 {
        (f64::from(self.0) * (core::f64::consts::TAU / 4_294_967_296.0)) as f32
    }

    /// Create a `Bam` from radians.
    #[inline]
    #[must_use]
    pub fn from_radians(rad: f32) -> Self {
        let bam = (f64::from(rad) / core::f64::consts::TAU * 4_294_967_296.0) as i64 as u32;
        Self(bam)
    }
}

// ---------------------------------------------------------------------------
// Operator impls
// ---------------------------------------------------------------------------

impl core::ops::Add for Bam {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        self.wrapping_add(rhs)
    }
}

impl core::ops::Sub for Bam {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        self.wrapping_sub(rhs)
    }
}

impl core::ops::Neg for Bam {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        self.negate()
    }
}

impl core::fmt::Display for Bam {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let degrees = f64::from(self.0) * 360.0 / (f64::from(u32::MAX) + 1.0);
        write!(f, "{degrees:.2}\u{00B0}")
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ang90_plus_ang90_is_ang180() {
        assert_eq!(ANG90 + ANG90, ANG180);
    }

    #[test]
    fn full_circle_wraps_to_zero() {
        assert_eq!(ANG180 + ANG180, Bam::ZERO);
    }

    #[test]
    fn additive_inverse_holds() {
        let a = Bam(0x1234_5678);
        let b = Bam(0xABCD_EF01);
        assert_eq!((a + b) - b, a);
    }

    #[test]
    fn negate_is_additive_inverse() {
        let a = Bam(0x4000_0000);
        assert_eq!(a + (-a), Bam::ZERO);
    }

    #[test]
    fn fine_angle_in_bounds() {
        for raw in [0u32, 0x1000_0000, 0x4000_0000, 0x8000_0000, 0xFFFF_FFFF] {
            let idx = Bam(raw).fine_angle();
            assert!(idx < FINE_TABLE_SIZE, "fine_angle {idx} out of bounds");
        }
    }

    #[test]
    fn sin_cos_at_cardinal_angles() {
        // sin(0)=0, cos(0)=1
        let (s, c) = Bam::ZERO.sin_cos_fixed();
        assert_eq!(s.raw(), 0, "sin(0) should be 0, got {}", s.raw());
        assert!(
            (c.to_f32() - 1.0).abs() < 0.001,
            "cos(0) should be ~1.0, got {}",
            c.to_f32()
        );

        // sin(90)=1, cos(90)=0
        let (s, c) = ANG90.sin_cos_fixed();
        assert!(
            (s.to_f32() - 1.0).abs() < 0.001,
            "sin(90) should be ~1.0, got {}",
            s.to_f32()
        );
        assert!(
            c.to_f32().abs() < 0.001,
            "cos(90) should be ~0.0, got {}",
            c.to_f32()
        );

        // sin(180)=0, cos(180)=-1
        let (s, c) = ANG180.sin_cos_fixed();
        assert!(
            s.to_f32().abs() < 0.001,
            "sin(180) should be ~0.0, got {}",
            s.to_f32()
        );
        assert!(
            (c.to_f32() + 1.0).abs() < 0.001,
            "cos(180) should be ~-1.0, got {}",
            c.to_f32()
        );

        // sin(270)=-1, cos(270)=0
        let (s, c) = ANG270.sin_cos_fixed();
        assert!(
            (s.to_f32() + 1.0).abs() < 0.001,
            "sin(270) should be ~-1.0, got {}",
            s.to_f32()
        );
        assert!(
            c.to_f32().abs() < 0.001,
            "cos(270) should be ~0.0, got {}",
            c.to_f32()
        );
    }

    #[test]
    fn sin_cos_f32_delegates_to_polynomial() {
        // 45 degrees -> sin ~= cos ~= 0.7071
        let (s, c) = ANG45.sin_cos_f32();
        assert!(
            (s - 0.7071).abs() < 0.01,
            "sin(45) should be ~0.7071, got {s}"
        );
        assert!(
            (c - 0.7071).abs() < 0.01,
            "cos(45) should be ~0.7071, got {c}"
        );
    }

    #[test]
    fn from_radians_roundtrip() {
        let angles = [0.0_f32, 0.5, 1.0, 2.0, 3.0, 5.0, 6.28];
        for rad in angles {
            let bam = Bam::from_radians(rad);
            let back = bam.to_radians();
            assert!(
                (back - rad).abs() < 0.002,
                "roundtrip failed: {rad} -> {} -> {back}",
                bam.0
            );
        }

        // Negative radians should wrap correctly (e.g., -π/2 ≈ 270°)
        let neg = Bam::from_radians(-std::f32::consts::FRAC_PI_2);
        let diff = neg.0.wrapping_sub(ANG270.0);
        assert!(
            diff < 0x0010_0000 || diff > 0xFFF0_0000,
            "from_radians(-π/2) should be near ANG270, got {neg}"
        );
    }

    #[test]
    fn const_table_matches_stdlib() {
        // Sample every 64th entry and compare against f64::sin.
        let max_error = 0.001_f64;
        let mut worst_error = 0.0_f64;

        let mut i = 0;
        while i < FINE_TABLE_SIZE {
            let angle = (i as f64) * core::f64::consts::TAU / (FINE_TABLE_SIZE as f64);
            let expected = angle.sin();
            let actual = f64::from(SINE_TABLE[i].to_f32());
            let error = (actual - expected).abs();
            if error > worst_error {
                worst_error = error;
            }
            assert!(
                error < max_error,
                "Table entry [{i}]: expected {expected:.6}, got {actual:.6}, error {error:.6}"
            );
            i += 64;
        }

        // Informational: print worst error (only visible with --nocapture).
        eprintln!("const_table_matches_stdlib: worst error = {worst_error:.8}");
    }

    #[test]
    fn display_shows_degrees() {
        let s = format!("{ANG90}");
        assert_eq!(s, "90.00\u{00B0}");

        let s2 = format!("{ANG180}");
        assert_eq!(s2, "180.00\u{00B0}");

        let s3 = format!("{}", Bam::ZERO);
        assert_eq!(s3, "0.00\u{00B0}");
    }

    #[test]
    fn from_raw_roundtrip() {
        let raw = 0x1234_5678_u32;
        let bam = Bam::from_raw(raw);
        assert_eq!(bam.raw(), raw);
    }
}

// ---------------------------------------------------------------------------
// Proptest property tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn wrapping_add_sub_inverse(a: u32, b: u32) {
            let ba = Bam(a);
            let bb = Bam(b);
            prop_assert_eq!((ba + bb) - bb, ba);
        }

        #[test]
        fn sin_bounded(a: u32) {
            let (s, c) = Bam(a).sin_cos_fixed();
            let sf = s.to_f32();
            let cf = c.to_f32();
            prop_assert!(sf.abs() <= 1.01, "|sin| = {} > 1.01", sf.abs());
            prop_assert!(cf.abs() <= 1.01, "|cos| = {} > 1.01", cf.abs());
        }

        #[test]
        fn fine_angle_always_in_bounds(a: u32) {
            let idx = Bam(a).fine_angle();
            prop_assert!(idx < FINE_TABLE_SIZE);
        }
    }
}
