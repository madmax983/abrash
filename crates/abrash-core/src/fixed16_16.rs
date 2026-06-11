//! Fixed-point 16.16 arithmetic for world-coordinate math.
//!
//! This is a 16.16 fixed-point type where bits [31..16] represent the integer
//! part and bits [15..0] represent the fractional part. One unit = `1 << 16 = 65536`.
//!
//! Ported from doom-rs with additions for f32 conversion. Coexists with the
//! existing 24.8 fixed-point format used for the z-buffer (different precision,
//! different domain).

use core::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// Fixed-point 16.16 number: bits [31..16] = integer, bits [15..0] = fraction.
///
/// One unit = `1 << 16 = 65536`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fixed16_16(pub i32);

/// The fractional shift (FRACBITS in the original Doom C source).
pub const FRAC_BITS: u32 = 16;

/// One unit in fixed-point: equivalent to the float `1.0`.
pub const FIXED_ONE: Fixed16_16 = Fixed16_16(1 << FRAC_BITS);

impl Fixed16_16 {
    /// Zero.
    pub const ZERO: Self = Self(0);

    /// Create from a raw `i32` (no scaling -- you supply the already-shifted bits).
    #[inline]
    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        Self(raw)
    }

    /// Convert an integer to fixed-point by shifting left 16 bits.
    #[inline]
    #[must_use]
    pub const fn from_int(n: i32) -> Self {
        Self(n << FRAC_BITS)
    }

    /// Extract the integer part (truncates toward negative infinity).
    #[inline]
    #[must_use]
    pub const fn to_int(self) -> i32 {
        self.0 >> FRAC_BITS
    }

    /// Return the raw `i32` bit pattern.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> i32 {
        self.0
    }

    /// Convert to `f32`.
    #[inline]
    #[must_use]
    pub fn to_f32(self) -> f32 {
        self.0 as f32 / (1 << FRAC_BITS) as f32
    }

    /// Convert from `f32`.
    #[inline]
    #[must_use]
    pub fn from_f32(v: f32) -> Self {
        Self((v * (1 << FRAC_BITS) as f32) as i32)
    }

    /// Compute `self * rhs` using a 64-bit intermediate to avoid overflow.
    ///
    /// Equivalent to the C macro `FixedMul(a, b)`.
    #[inline]
    #[must_use]
    pub const fn fixed_mul(self, rhs: Self) -> Self {
        let product = (self.0 as i64) * (rhs.0 as i64);
        Self((product >> FRAC_BITS) as i32)
    }

    /// Compute `self / rhs` using a 64-bit intermediate.
    ///
    /// Clamps to `i32::MIN`/`i32::MAX` on overflow.
    ///
    /// # Panics
    /// Panics (debug) if `rhs == 0`.
    #[inline]
    #[must_use]
    pub fn fixed_div(self, rhs: Self) -> Self {
        debug_assert!(rhs.0 != 0, "fixed_div: division by zero");
        let numerator = i64::from(self.0) << FRAC_BITS;
        let mut result = numerator / i64::from(rhs.0);
        if result > i64::from(i32::MAX) {
            result = i64::from(i32::MAX);
        } else if result < i64::from(i32::MIN) {
            result = i64::from(i32::MIN);
        }
        Self(result as i32)
    }

    /// Absolute value.
    #[inline]
    #[must_use]
    pub const fn abs(self) -> Self {
        Self(self.0.wrapping_abs())
    }

    /// Linear interpolation: `self + t * (other - self)` where `t` is in `[0, FIXED_ONE]`.
    #[inline]
    #[must_use]
    pub fn lerp(self, other: Self, t: Self) -> Self {
        self + (other - self).fixed_mul(t)
    }
}

impl Add for Fixed16_16 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self(self.0.wrapping_add(rhs.0))
    }
}

impl AddAssign for Fixed16_16 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.wrapping_add(rhs.0);
    }
}

impl Sub for Fixed16_16 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self(self.0.wrapping_sub(rhs.0))
    }
}

impl SubAssign for Fixed16_16 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0.wrapping_sub(rhs.0);
    }
}

impl Neg for Fixed16_16 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self(self.0.wrapping_neg())
    }
}

/// `*` calls `fixed_mul` -- not the same as integer multiplication.
impl Mul for Fixed16_16 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        self.fixed_mul(rhs)
    }
}

/// `/` calls `fixed_div`.
impl Div for Fixed16_16 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Self) -> Self {
        self.fixed_div(rhs)
    }
}

impl From<i32> for Fixed16_16 {
    #[inline]
    fn from(n: i32) -> Self {
        Self::from_int(n)
    }
}

impl From<Fixed16_16> for i32 {
    #[inline]
    fn from(f: Fixed16_16) -> Self {
        f.to_int()
    }
}

impl core::fmt::Display for Fixed16_16 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.0 < 0 {
            write!(f, "-")?;
            return Self(self.0.wrapping_neg()).fmt(f);
        }
        let int_part = self.to_int();
        let frac = (self.0 & 0xFFFF) as u32;
        let frac_dec = (frac * 100_000) >> FRAC_BITS;
        write!(f, "{int_part}.{frac_dec:05}")
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_int_roundtrip() {
        for n in -32768_i32..=32767 {
            assert_eq!(Fixed16_16::from_int(n).to_int(), n);
        }
    }

    #[test]
    fn fixed_one_is_unit() {
        assert_eq!(FIXED_ONE.to_int(), 1);
        assert_eq!(FIXED_ONE.fixed_mul(FIXED_ONE), FIXED_ONE);
    }

    #[test]
    fn mul_basic() {
        let a = Fixed16_16::from_int(3);
        let b = Fixed16_16::from_int(4);
        assert_eq!(a.fixed_mul(b), Fixed16_16::from_int(12));
    }

    #[test]
    fn mul_commutative() {
        let a = Fixed16_16::from_int(3);
        let b = Fixed16_16::from_int(7);
        assert_eq!(a.fixed_mul(b), b.fixed_mul(a));
    }

    #[test]
    fn mul_by_zero() {
        let a = Fixed16_16::from_int(12345);
        assert_eq!(a.fixed_mul(Fixed16_16::ZERO), Fixed16_16::ZERO);
    }

    #[test]
    fn div_basic() {
        let a = Fixed16_16::from_int(10);
        let b = Fixed16_16::from_int(2);
        assert_eq!(a.fixed_div(b), Fixed16_16::from_int(5));
    }

    #[test]
    fn to_f32_roundtrip() {
        // Integer roundtrip
        let a = Fixed16_16::from_int(42);
        assert!((a.to_f32() - 42.0).abs() < f32::EPSILON);

        // Fractional roundtrip
        let b = Fixed16_16::from_f32(3.5);
        assert!((b.to_f32() - 3.5).abs() < 0.001);

        // Negative roundtrip
        let c = Fixed16_16::from_f32(-7.25);
        assert!((c.to_f32() - (-7.25)).abs() < 0.001);
    }

    #[test]
    fn operators_consistent() {
        let a = Fixed16_16::from_int(10);
        let b = Fixed16_16::from_int(3);
        assert_eq!(a * b, a.fixed_mul(b));
        assert_eq!(a / b, a.fixed_div(b));
    }

    #[test]
    fn operator_add_sub() {
        let a = Fixed16_16::from_int(10);
        let b = Fixed16_16::from_int(3);
        assert_eq!(a + b, Fixed16_16::from_int(13));
        assert_eq!(a - b, Fixed16_16::from_int(7));
    }

    #[test]
    fn operator_neg() {
        let a = Fixed16_16::from_int(5);
        assert_eq!(-a, Fixed16_16::from_int(-5));
    }

    #[test]
    fn operator_add_assign_sub_assign() {
        let mut a = Fixed16_16::from_int(10);
        a += Fixed16_16::from_int(3);
        assert_eq!(a, Fixed16_16::from_int(13));
        a -= Fixed16_16::from_int(6);
        assert_eq!(a, Fixed16_16::from_int(7));
    }

    #[test]
    fn abs_positive_unchanged() {
        let a = Fixed16_16::from_int(5);
        assert_eq!(a.abs(), a);
    }

    #[test]
    fn abs_negative_negated() {
        let a = Fixed16_16::from_int(-5);
        assert_eq!(a.abs(), Fixed16_16::from_int(5));
    }

    #[test]
    fn lerp_endpoints() {
        let a = Fixed16_16::from_int(10);
        let b = Fixed16_16::from_int(20);
        assert_eq!(a.lerp(b, Fixed16_16::ZERO), a);
        assert_eq!(a.lerp(b, FIXED_ONE), b);
    }

    #[test]
    fn lerp_midpoint() {
        let a = Fixed16_16::from_int(10);
        let b = Fixed16_16::from_int(20);
        let t_half = Fixed16_16(1 << 15); // 0.5 in 16.16
        assert_eq!(a.lerp(b, t_half), Fixed16_16::from_int(15));
    }

    #[test]
    #[should_panic]
    fn div_by_zero_panics() {
        let a = Fixed16_16::from_int(10);
        let _ = a.fixed_div(Fixed16_16::ZERO);
    }

    #[test]
    fn display_formatting() {
        let a = Fixed16_16::from_int(3);
        let s = format!("{a}");
        assert_eq!(s, "3.00000");

        let b = Fixed16_16(3 * (1 << 16) + (1 << 15)); // 3.5
        let s2 = format!("{b}");
        assert_eq!(s2, "3.50000");

        let c = Fixed16_16::from_f32(-3.5);
        let s3 = format!("{c}");
        assert_eq!(s3, "-3.50000");
    }

    #[test]
    fn from_i32_conversion() {
        let a: Fixed16_16 = 42.into();
        assert_eq!(a, Fixed16_16::from_int(42));
    }

    #[test]
    fn into_i32_conversion() {
        let a = Fixed16_16::from_int(42);
        let n: i32 = a.into();
        assert_eq!(n, 42);
    }

    #[test]
    fn from_raw_preserves_bits() {
        let raw = 0x0003_8000; // 3.5 in 16.16
        let a = Fixed16_16::from_raw(raw);
        assert_eq!(a.raw(), raw);
    }

    #[test]
    fn div_overflow_clamps() {
        // Large numerator / tiny denominator should clamp, not overflow
        let big = Fixed16_16::from_int(30000);
        let tiny = Fixed16_16::from_raw(1); // smallest positive fixed-point
        let result = big.fixed_div(tiny);
        assert_eq!(result.raw(), i32::MAX);
    }
}

// ---------------------------------------------------------------------------
// Proptest property tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate fixed-point values from integers in the safe game-world range
    /// [-32767, 32767] (avoids intermediate i64 overflow in `fixed_mul`).
    fn fixed_strategy() -> impl Strategy<Value = Fixed16_16> {
        (-32767_i32..=32767).prop_map(Fixed16_16::from_int)
    }

    proptest! {
        #[test]
        fn mul_commutative(a in fixed_strategy(), b in fixed_strategy()) {
            prop_assert_eq!(a.fixed_mul(b), b.fixed_mul(a));
        }

        #[test]
        fn add_sub_inverse(a in fixed_strategy(), b in fixed_strategy()) {
            prop_assert_eq!((a + b) - b, a);
        }

        #[test]
        fn mul_by_one_identity(a in fixed_strategy()) {
            prop_assert_eq!(a.fixed_mul(FIXED_ONE), a);
        }

        #[test]
        fn mul_by_zero_is_zero(a in fixed_strategy()) {
            prop_assert_eq!(a.fixed_mul(Fixed16_16::ZERO), Fixed16_16::ZERO);
        }
    }
}
