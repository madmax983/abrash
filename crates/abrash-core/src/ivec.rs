//! Integer vector types for screen-space, grid, and texel coordinates.
//!
//! [`IVec2`] and [`IVec3`] parallel `Vec2`/`Vec3` but store `i32` components.
//! They are the natural coordinate type for:
//!
//! - Pixel/texel positions (`0..width`, `0..height`)
//! - Grid cell coordinates (voxels, tile maps)
//! - Rasterizer scanline endpoints
//! - Image rect corners
//!
//! Conversions to/from `Vec2`/`Vec3` are explicit via `from`/`as_vec` to avoid
//! silent truncation when mixing integral and floating-point coordinates.
//!
//! # Examples
//!
//! ```
//! use abrash_core::ivec::{IVec2, IVec3};
//!
//! let tile = IVec2::new(3, 7);
//! let neighbour = tile + IVec2::new(1, 0);
//! assert_eq!(neighbour, IVec2::new(4, 7));
//!
//! // Linear index into a flat array (row-major)
//! let width = 64;
//! let idx = tile.y * width + tile.x;
//! assert_eq!(idx, 7 * 64 + 3);
//! ```

use crate::math::{Vec2, Vec3};

// ── IVec2 ─────────────────────────────────────────────────────────────────────

/// A 2D integer vector — pixel coordinates, grid offsets, screen-space spans.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct IVec2 {
    /// The X (horizontal) component, representing column or grid width.
    pub x: i32,
    /// The Y (vertical) component, representing row or grid height.
    pub y: i32,
}

impl IVec2 {
    /// Zero vector.
    pub const ZERO: Self = Self::new(0, 0);
    /// One vector.
    pub const ONE: Self = Self::new(1, 1);
    /// Unit X.
    pub const X: Self = Self::new(1, 0);
    /// Unit Y.
    pub const Y: Self = Self::new(0, 1);
    /// Minimum i32 per component (useful as a sentinel for AABB accumulation).
    pub const MIN: Self = Self::new(i32::MIN, i32::MIN);
    /// Maximum i32 per component.
    pub const MAX: Self = Self::new(i32::MAX, i32::MAX);

    /// Construct from `(x, y)`.
    #[must_use]
    #[inline]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Both components set to `v`.
    #[must_use]
    #[inline]
    pub const fn splat(v: i32) -> Self {
        Self::new(v, v)
    }

    // ── Conversion ────────────────────────────────────────────────────────────

    /// Convert to `Vec2` (exact, no precision loss since i32 fits in f32 mantissa up to 2²³).
    #[must_use]
    #[inline]
    pub const fn as_vec2(self) -> Vec2 {
        Vec2::new(self.x as f32, self.y as f32)
    }

    /// Truncate a `Vec2` to integer components (rounds toward zero).
    #[must_use]
    #[inline]
    pub const fn from_vec2_truncate(v: Vec2) -> Self {
        Self::new(v.x as i32, v.y as i32)
    }

    /// Round a `Vec2` to the nearest integer.
    #[must_use]
    #[inline]
    pub const fn from_vec2_round(v: Vec2) -> Self {
        Self::new(v.x.round() as i32, v.y.round() as i32)
    }

    /// Floor a `Vec2` to integer components.
    #[must_use]
    #[inline]
    pub const fn from_vec2_floor(v: Vec2) -> Self {
        Self::new(v.x.floor() as i32, v.y.floor() as i32)
    }

    /// Ceiling a `Vec2` to integer components.
    #[must_use]
    #[inline]
    pub const fn from_vec2_ceil(v: Vec2) -> Self {
        Self::new(v.x.ceil() as i32, v.y.ceil() as i32)
    }

    // ── Arithmetic ────────────────────────────────────────────────────────────

    /// Component-wise absolute value.
    #[must_use]
    #[inline]
    pub const fn abs(self) -> Self {
        Self::new(self.x.abs(), self.y.abs())
    }

    /// Component-wise minimum.
    #[must_use]
    #[inline]
    pub const fn min(self, other: Self) -> Self {
        Self::new(
            if self.x < other.x { self.x } else { other.x },
            if self.y < other.y { self.y } else { other.y },
        )
    }

    /// Component-wise maximum.
    #[must_use]
    #[inline]
    pub const fn max(self, other: Self) -> Self {
        Self::new(
            if self.x > other.x { self.x } else { other.x },
            if self.y > other.y { self.y } else { other.y },
        )
    }

    /// Clamp each component to `[min, max]`.
    #[must_use]
    #[inline]
    pub const fn clamp(self, lo: Self, hi: Self) -> Self {
        self.max(lo).min(hi)
    }

    /// Dot product.
    #[must_use]
    #[inline]
    pub const fn dot(self, other: Self) -> i32 {
        self.x * other.x + self.y * other.y
    }

    /// 2D cross product (returns the z-component of the 3D cross).
    ///
    /// Positive means `other` is counter-clockwise from `self`.
    #[must_use]
    #[inline]
    pub const fn cross(self, other: Self) -> i32 {
        self.x * other.y - self.y * other.x
    }

    /// Squared length (avoids sqrt).
    #[must_use]
    #[inline]
    pub const fn length_sq(self) -> i32 {
        self.x * self.x + self.y * self.y
    }

    /// Euclidean length as `f32`.
    #[must_use]
    #[inline]
    pub fn length(self) -> f32 {
        (self.length_sq() as f32).sqrt()
    }

    /// Manhattan distance to `other`.
    #[must_use]
    #[inline]
    pub const fn manhattan(self, other: Self) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }

    /// Chebyshev distance to `other` (max of absolute component differences).
    #[must_use]
    #[inline]
    pub const fn chebyshev(self, other: Self) -> i32 {
        let dx = (self.x - other.x).abs();
        let dy = (self.y - other.y).abs();
        if dx > dy { dx } else { dy }
    }

    /// Linear array index for a 2D grid with row stride `stride`.
    ///
    /// Equivalent to `self.y * stride + self.x`.
    #[must_use]
    #[inline]
    pub const fn to_index(self, stride: i32) -> i32 {
        self.y * stride + self.x
    }

    /// Construct from a flat linear index and row stride.
    #[must_use]
    #[inline]
    pub const fn from_index(idx: i32, stride: i32) -> Self {
        Self::new(idx % stride, idx / stride)
    }

    /// Returns `true` if all components are in `[lo, hi)` (exclusive upper bound).
    #[must_use]
    #[inline]
    pub const fn in_bounds(self, lo: Self, hi: Self) -> bool {
        self.x >= lo.x && self.y >= lo.y && self.x < hi.x && self.y < hi.y
    }

    /// Swizzle to `(y, x)`.
    #[must_use]
    #[inline]
    pub const fn yx(self) -> Self {
        Self::new(self.y, self.x)
    }

    /// Rotate 90° counter-clockwise: `(x, y)` → `(-y, x)`.
    #[must_use]
    #[inline]
    pub const fn perp(self) -> Self {
        Self::new(-self.y, self.x)
    }
}

impl std::ops::Add for IVec2 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl std::ops::AddAssign for IVec2 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl std::ops::Sub for IVec2 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl std::ops::SubAssign for IVec2 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl std::ops::Mul<i32> for IVec2 {
    type Output = Self;
    #[inline]
    fn mul(self, s: i32) -> Self {
        Self::new(self.x * s, self.y * s)
    }
}

impl std::ops::Mul<IVec2> for i32 {
    type Output = IVec2;
    #[inline]
    fn mul(self, v: IVec2) -> IVec2 {
        v * self
    }
}

impl std::ops::Div<i32> for IVec2 {
    type Output = Self;
    #[inline]
    fn div(self, s: i32) -> Self {
        Self::new(self.x / s, self.y / s)
    }
}

impl std::ops::Neg for IVec2 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y)
    }
}

impl std::fmt::Display for IVec2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

// ── IVec3 ─────────────────────────────────────────────────────────────────────

/// A 3D integer vector — voxel coordinates, 3D grid cells, color components.
///
/// # Examples
///
/// ```
/// use abrash_core::ivec::IVec3;
///
/// let a = IVec3::new(1, 2, 3);
/// let b = IVec3::new(4, 5, 6);
/// assert_eq!(a + b, IVec3::new(5, 7, 9));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct IVec3 {
    /// The X (width/horizontal) Cartesian component in 3D grid space.
    pub x: i32,
    /// The Y (height/vertical) Cartesian component in 3D grid space.
    pub y: i32,
    /// The Z (depth) Cartesian component in 3D grid space.
    pub z: i32,
}

impl IVec3 {
    /// Zero vector.
    pub const ZERO: Self = Self::new(0, 0, 0);
    /// One vector.
    pub const ONE: Self = Self::new(1, 1, 1);
    /// Unit X.
    pub const X: Self = Self::new(1, 0, 0);
    /// Unit Y.
    pub const Y: Self = Self::new(0, 1, 0);
    /// Unit Z.
    pub const Z: Self = Self::new(0, 0, 1);
    /// Minimum i32 per component.
    pub const MIN: Self = Self::new(i32::MIN, i32::MIN, i32::MIN);
    /// Maximum i32 per component.
    pub const MAX: Self = Self::new(i32::MAX, i32::MAX, i32::MAX);

    /// Construct from `(x, y, z)`.
    #[must_use]
    #[inline]
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Both components set to `v`.
    #[must_use]
    #[inline]
    pub const fn splat(v: i32) -> Self {
        Self::new(v, v, v)
    }

    // ── Conversion ────────────────────────────────────────────────────────────

    /// Convert to `Vec3`.
    #[must_use]
    #[inline]
    pub const fn as_vec3(self) -> Vec3 {
        Vec3::new(self.x as f32, self.y as f32, self.z as f32)
    }

    /// Truncate a `Vec3` to integer (rounds toward zero).
    #[must_use]
    #[inline]
    pub const fn from_vec3_truncate(v: Vec3) -> Self {
        Self::new(v.x as i32, v.y as i32, v.z as i32)
    }

    /// Round a `Vec3` to the nearest integer per component.
    #[must_use]
    #[inline]
    pub const fn from_vec3_round(v: Vec3) -> Self {
        Self::new(v.x.round() as i32, v.y.round() as i32, v.z.round() as i32)
    }

    /// Floor a `Vec3`.
    #[must_use]
    #[inline]
    pub const fn from_vec3_floor(v: Vec3) -> Self {
        Self::new(v.x.floor() as i32, v.y.floor() as i32, v.z.floor() as i32)
    }

    /// Drop the z component.
    #[must_use]
    #[inline]
    pub const fn xy(self) -> IVec2 {
        IVec2::new(self.x, self.y)
    }

    // ── Arithmetic ────────────────────────────────────────────────────────────

    /// Component-wise absolute value.
    #[must_use]
    #[inline]
    pub const fn abs(self) -> Self {
        Self::new(self.x.abs(), self.y.abs(), self.z.abs())
    }

    /// Component-wise minimum.
    #[must_use]
    #[inline]
    pub const fn min(self, other: Self) -> Self {
        Self::new(
            if self.x < other.x { self.x } else { other.x },
            if self.y < other.y { self.y } else { other.y },
            if self.z < other.z { self.z } else { other.z },
        )
    }

    /// Component-wise maximum.
    #[must_use]
    #[inline]
    pub const fn max(self, other: Self) -> Self {
        Self::new(
            if self.x > other.x { self.x } else { other.x },
            if self.y > other.y { self.y } else { other.y },
            if self.z > other.z { self.z } else { other.z },
        )
    }

    /// Clamp each component to `[lo, hi]`.
    #[must_use]
    #[inline]
    pub const fn clamp(self, lo: Self, hi: Self) -> Self {
        self.max(lo).min(hi)
    }

    /// Dot product.
    #[must_use]
    #[inline]
    pub const fn dot(self, other: Self) -> i32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// 3D cross product.
    #[must_use]
    #[inline]
    pub const fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    /// Squared length.
    #[must_use]
    #[inline]
    pub const fn length_sq(self) -> i32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    /// Euclidean length as `f32`.
    #[must_use]
    #[inline]
    pub fn length(self) -> f32 {
        (self.length_sq() as f32).sqrt()
    }

    /// Manhattan distance to `other`.
    #[must_use]
    #[inline]
    pub const fn manhattan(self, other: Self) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs() + (self.z - other.z).abs()
    }

    /// Chebyshev distance (max absolute component difference).
    #[must_use]
    #[inline]
    pub const fn chebyshev(self, other: Self) -> i32 {
        let dx = (self.x - other.x).abs();
        let dy = (self.y - other.y).abs();
        let dz = (self.z - other.z).abs();
        let m = if dx > dy { dx } else { dy };
        if m > dz { m } else { dz }
    }

    /// Linear array index for a 3D grid with row and slice strides.
    ///
    /// `idx = z * (height * width) + y * width + x`
    #[must_use]
    #[inline]
    pub const fn to_index_3d(self, width: i32, height: i32) -> i32 {
        self.z * (width * height) + self.y * width + self.x
    }

    /// Returns `true` if all components are in `[lo, hi)`.
    #[must_use]
    #[inline]
    pub const fn in_bounds(self, lo: Self, hi: Self) -> bool {
        self.x >= lo.x
            && self.y >= lo.y
            && self.z >= lo.z
            && self.x < hi.x
            && self.y < hi.y
            && self.z < hi.z
    }
}

impl std::ops::Add for IVec3 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::AddAssign for IVec3 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl std::ops::Sub for IVec3 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::SubAssign for IVec3 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
        self.z -= rhs.z;
    }
}

impl std::ops::Mul<i32> for IVec3 {
    type Output = Self;
    #[inline]
    fn mul(self, s: i32) -> Self {
        Self::new(self.x * s, self.y * s, self.z * s)
    }
}

impl std::ops::Mul<IVec3> for i32 {
    type Output = IVec3;
    #[inline]
    fn mul(self, v: IVec3) -> IVec3 {
        v * self
    }
}

impl std::ops::Div<i32> for IVec3 {
    type Output = Self;
    #[inline]
    fn div(self, s: i32) -> Self {
        Self::new(self.x / s, self.y / s, self.z / s)
    }
}

impl std::ops::Neg for IVec3 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

impl std::fmt::Display for IVec3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

// ── From/Into conversions ─────────────────────────────────────────────────────

impl From<(i32, i32)> for IVec2 {
    fn from((x, y): (i32, i32)) -> Self {
        Self::new(x, y)
    }
}

impl From<IVec2> for (i32, i32) {
    fn from(v: IVec2) -> Self {
        (v.x, v.y)
    }
}

impl From<(i32, i32, i32)> for IVec3 {
    fn from((x, y, z): (i32, i32, i32)) -> Self {
        Self::new(x, y, z)
    }
}

impl From<IVec3> for (i32, i32, i32) {
    fn from(v: IVec3) -> Self {
        (v.x, v.y, v.z)
    }
}

impl From<IVec2> for IVec3 {
    /// Extend with z = 0.
    fn from(v: IVec2) -> Self {
        Self::new(v.x, v.y, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── IVec2 ─────────────────────────────────────────────────────────────────

    #[test]
    fn ivec2_basic_ops() {
        let a = IVec2::new(3, 4);
        let b = IVec2::new(1, -2);
        assert_eq!(a + b, IVec2::new(4, 2));
        assert_eq!(a - b, IVec2::new(2, 6));
        assert_eq!(a * 2, IVec2::new(6, 8));
        assert_eq!(2 * a, IVec2::new(6, 8));
        assert_eq!(-a, IVec2::new(-3, -4));
    }

    #[test]
    fn ivec2_dot_and_cross() {
        let a = IVec2::new(3, 0);
        let b = IVec2::new(0, 4);
        assert_eq!(a.dot(b), 0);
        // (3,0) × (0,4) = 3*4 - 0*0 = 12 (CCW)
        assert_eq!(a.cross(b), 12);
    }

    #[test]
    fn ivec2_length_sq() {
        let v = IVec2::new(3, 4);
        assert_eq!(v.length_sq(), 25);
        assert!((v.length() - 5.0).abs() < 1e-5);
    }

    #[test]
    fn ivec2_min_max_clamp() {
        let a = IVec2::new(5, 2);
        let b = IVec2::new(3, 7);
        assert_eq!(a.min(b), IVec2::new(3, 2));
        assert_eq!(a.max(b), IVec2::new(5, 7));
        let c = IVec2::new(4, 4).clamp(IVec2::new(0, 0), IVec2::new(3, 6));
        assert_eq!(c, IVec2::new(3, 4));
    }

    #[test]
    fn ivec2_manhattan_chebyshev() {
        let a = IVec2::new(0, 0);
        let b = IVec2::new(3, 4);
        assert_eq!(a.manhattan(b), 7);
        assert_eq!(a.chebyshev(b), 4);
    }

    #[test]
    fn ivec2_to_from_index() {
        let pos = IVec2::new(5, 3);
        let stride = 10;
        let idx = pos.to_index(stride);
        assert_eq!(idx, 35); // 3*10 + 5
        let back = IVec2::from_index(idx, stride);
        assert_eq!(back, pos);
    }

    #[test]
    fn ivec2_in_bounds() {
        let lo = IVec2::new(0, 0);
        let hi = IVec2::new(10, 10);
        assert!(IVec2::new(5, 5).in_bounds(lo, hi));
        assert!(!IVec2::new(10, 5).in_bounds(lo, hi)); // exclusive upper
        assert!(!IVec2::new(-1, 5).in_bounds(lo, hi));
    }

    #[test]
    fn ivec2_perp() {
        let v = IVec2::new(1, 0);
        assert_eq!(v.perp(), IVec2::new(0, 1)); // 90° CCW
        assert_eq!(v.perp().perp(), -v);
    }

    #[test]
    fn ivec2_vec2_roundtrip() {
        let iv = IVec2::new(42, -7);
        let fv = iv.as_vec2();
        assert_eq!(IVec2::from_vec2_truncate(fv), iv);
        assert_eq!(IVec2::from_vec2_round(fv), iv);
    }

    #[test]
    fn ivec2_conversions() {
        let v: IVec2 = (3, 4).into();
        assert_eq!(v, IVec2::new(3, 4));
        let t: (i32, i32) = v.into();
        assert_eq!(t, (3, 4));
    }

    // ── IVec3 ─────────────────────────────────────────────────────────────────

    #[test]
    fn ivec3_basic_ops() {
        let a = IVec3::new(1, 2, 3);
        let b = IVec3::new(4, 5, 6);
        assert_eq!(a + b, IVec3::new(5, 7, 9));
        assert_eq!(b - a, IVec3::new(3, 3, 3));
        assert_eq!(a * 3, IVec3::new(3, 6, 9));
        assert_eq!(-a, IVec3::new(-1, -2, -3));
    }

    #[test]
    fn ivec3_cross() {
        let x = IVec3::X;
        let y = IVec3::Y;
        assert_eq!(x.cross(y), IVec3::Z);
    }

    #[test]
    fn ivec3_length_sq() {
        let v = IVec3::new(1, 2, 2);
        assert_eq!(v.length_sq(), 9);
        assert!((v.length() - 3.0).abs() < 1e-5);
    }

    #[test]
    fn ivec3_chebyshev() {
        let a = IVec3::ZERO;
        let b = IVec3::new(3, 1, 2);
        assert_eq!(a.chebyshev(b), 3);
    }

    #[test]
    fn ivec3_in_bounds() {
        let lo = IVec3::ZERO;
        let hi = IVec3::new(8, 8, 8);
        assert!(IVec3::new(4, 4, 4).in_bounds(lo, hi));
        assert!(!IVec3::new(8, 4, 4).in_bounds(lo, hi));
    }

    #[test]
    fn ivec3_to_index_3d() {
        let v = IVec3::new(1, 2, 3);
        // z=3, y=2, x=1 in 4×4×4 grid: 3*16 + 2*4 + 1 = 57
        assert_eq!(v.to_index_3d(4, 4), 57);
    }

    #[test]
    fn ivec3_xy_extracts_correctly() {
        let v = IVec3::new(5, 9, -3);
        assert_eq!(v.xy(), IVec2::new(5, 9));
    }

    #[test]
    fn ivec3_from_ivec2_zero_z() {
        let iv2 = IVec2::new(3, 7);
        let iv3: IVec3 = iv2.into();
        assert_eq!(iv3, IVec3::new(3, 7, 0));
    }

    #[test]
    fn ivec3_min_max() {
        let a = IVec3::new(1, 5, 3);
        let b = IVec3::new(4, 2, 6);
        assert_eq!(a.min(b), IVec3::new(1, 2, 3));
        assert_eq!(a.max(b), IVec3::new(4, 5, 6));
    }

    #[test]
    fn ivec3_vec3_roundtrip() {
        let iv = IVec3::new(-10, 255, 3);
        let fv = iv.as_vec3();
        assert_eq!(IVec3::from_vec3_truncate(fv), iv);
    }

    #[test]
    fn display_format() {
        assert_eq!(IVec2::new(3, -4).to_string(), "(3, -4)");
        assert_eq!(IVec3::new(1, 2, 3).to_string(), "(1, 2, 3)");
    }
}
