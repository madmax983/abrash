//! 2D and 3D math types for graphics programming.
//!
//! # Coordinate System
//!
//! This library uses a **Right-Handed** coordinate system.
//! *   **X**: Right
//! *   **Y**: Up
//! *   **Z**: Backward (Camera looks down -Z)
//!
//! # Matrix Convention
//!
//! Matrices are stored in **Row-Major** order.
//!
//! Transformations follow the **Row-Vector** convention ($v \cdot M$), meaning vectors are treated as rows and multiplied on the left.
//!
//! $$ v' = v \cdot M $$
//!
//! This implies that the order of multiplication matches the order of transformations:
//!
//! ```
//! # use abrash_core::math::{Mat4, Vec3};
//! // Scale, then Rotate, then Translate
//! let scale = Mat4::scale(2.0, 2.0, 2.0);
//! let rotate = Mat4::rotation_y(1.57); // 90 degrees
//! let translate = Mat4::translation(10.0, 0.0, 0.0);
//!
//! // Combine transformations
//! let transform = scale * rotate * translate;
//!
//! // Apply to a vector
//! let v = Vec3::new(1.0, 0.0, 0.0);
//! let (v_prime, _) = transform.transform_point(v);
//! ```
//!
//! All operations use `f32` for compatibility with graphics APIs.

use std::mem::MaybeUninit;
use std::ops::{Add, Mul, Sub};

/// Approximates the reciprocal square root ($1 / \sqrt{x}$).
///
/// This uses the hardware-accelerated AVX/SSE intrinsic if available, which offers
/// excellent performance (around 4 cycles) at the cost of a small precision error.
/// If AVX/SSE is not available, it falls back to a standard `sqrt().recip()`, which
/// is typically faster on modern generic `x86_64` CPUs than the legacy "Quake III bit-hack".
///
/// # Examples
///
/// ```
/// use abrash_core::math::fast_inv_sqrt;
///
/// let x = 4.0;
/// let inv_sqrt = fast_inv_sqrt(x); // 1.0 / sqrt(4.0) = 0.5
///
/// // Assert with a small tolerance due to approximation
/// assert!((inv_sqrt - 0.5).abs() < 0.01);
/// ```
///
/// Fast approximation of sine and cosine.
///
/// Computes an approximation of `sin(x)` and `cos(x)` (where `x` is in radians)
/// using a minimax polynomial approximation.
/// This trades a small amount of precision for performance.
///
/// # Examples
///
/// ```
/// use abrash_core::math::fast_sin_cos;
/// use std::f32::consts::PI;
///
/// let (s, c) = fast_sin_cos(PI / 4.0);
/// assert!((s - 0.7071).abs() < 0.01);
/// assert!((c - 0.7071).abs() < 0.01);
/// ```
#[inline]
#[must_use]
pub fn fast_sin_cos(mut x: f32) -> (f32, f32) {
    use std::f32::consts::PI;

    // Wrap x to [-PI, PI]
    let inv_twopi = 0.159_154_94; // 1.0 / (2.0 * PI)
    let y = (x * inv_twopi).round();
    x -= y * (2.0 * PI);

    // Compute sine using parabolic approximation
    // sin(x) ≈ 4/PI * x - 4/PI^2 * x * |x|
    let x_abs = x.abs();
    let mut sin_x = 1.273_239_54 * x - 0.405_284_73 * x * x_abs;

    // Additional precision step (optional but good for graphics)
    // sin(x) ≈ 0.225 * (sin_x * |sin_x| - sin_x) + sin_x
    let sin_x_abs = sin_x.abs();
    sin_x = 0.225 * (sin_x * sin_x_abs - sin_x) + sin_x;

    // Compute cosine by phase shifting: cos(x) = sin(x + PI/2)
    let mut x_cos = x + std::f32::consts::FRAC_PI_2;
    if x_cos > PI {
        x_cos -= 2.0 * PI;
    }

    let x_cos_abs = x_cos.abs();
    let mut cos_x = 1.273_239_54 * x_cos - 0.405_284_73 * x_cos * x_cos_abs;
    let cos_x_abs = cos_x.abs();
    cos_x = 0.225 * (cos_x * cos_x_abs - cos_x) + cos_x;

    (sin_x, cos_x)
}

/// Fast approximation of the sine function.
///
/// Computes an approximation of `sin(x)` using the same minimax polynomial
/// approximation as `fast_sin_cos`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::fast_sin;
/// use std::f32::consts::PI;
///
/// let s = fast_sin(PI / 4.0);
/// assert!((s - 0.7071).abs() < 0.01);
/// ```
#[inline]
#[must_use]
pub fn fast_sin(x: f32) -> f32 {
    fast_sin_cos(x).0
}

/// Fast approximation of the cosine function.
///
/// Computes an approximation of `cos(x)` using the same minimax polynomial
/// approximation as `fast_sin_cos`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::fast_cos;
/// use std::f32::consts::PI;
///
/// let c = fast_cos(PI / 4.0);
/// assert!((c - 0.7071).abs() < 0.01);
/// ```
#[inline]
#[must_use]
pub fn fast_cos(x: f32) -> f32 {
    fast_sin_cos(x).1
}

/// Fast approximation of the inverse square root.
///
/// Computes an approximation of `1.0 / sqrt(x)`. This uses the hardware-accelerated
/// AVX/SSE intrinsic if available, which offers excellent performance (around 4 cycles)
/// at the cost of a small precision error. If AVX/SSE is not available, it falls back
/// to a standard `sqrt().recip()`, which is typically faster on modern generic `x86_64`
/// CPUs than the legacy "Quake III bit-hack".
///
/// # Examples
///
/// ```
/// use abrash_core::math::fast_inv_sqrt;
///
/// let x = 4.0;
/// let inv_sqrt = fast_inv_sqrt(x); // 1.0 / sqrt(4.0) = 0.5
///
/// // Assert with a small tolerance due to approximation
/// assert!((inv_sqrt - 0.5).abs() < 0.01);
/// ```
#[must_use]
pub fn fast_inv_sqrt(n: f32) -> f32 {
    // Use AVX/SSE approximate reciprocal square root if available.
    // This is faster (~4 cycles latency vs ~23 for sqrt+div) but less precise.
    // We accept the approximation (error < 1.5*2^-12) for the sake of speed in lighting/normalization.
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    unsafe {
        // _mm_rsqrt_ss computes approximate 1/sqrt(a) for the lower float.
        let n_vec = std::arch::x86_64::_mm_set_ss(n);
        let r = std::arch::x86_64::_mm_rsqrt_ss(n_vec);
        std::arch::x86_64::_mm_cvtss_f32(r)
    }

    #[cfg(not(all(target_arch = "x86_64", feature = "simd")))]
    {
        // Modern hardware sqrt (e.g. sqrtss) is extremely fast.
        // Combined with reciprocal, this is faster (~2.3ns) than the legacy Quake III
        // bit-hack (~3.4ns) on modern x86_64, and safer than manual intrinsics.
        n.sqrt().recip()
    }
}

/// Linearly interpolates between two scalar values.
///
/// `t = 0.0` returns `a`, `t = 1.0` returns `b`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::lerp;
///
/// assert_eq!(lerp(0.0, 10.0, 0.5), 5.0);
/// ```
#[must_use]
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Clamps `x` to the range \[0.0, 1.0\].
#[must_use]
#[inline]
pub const fn saturate(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

/// Hermite interpolation: smooth ramp from 0 to 1 over \[edge0, edge1\].
///
/// Equivalent to GLSL `smoothstep`. Returns 0 for `x <= edge0`, 1 for `x >= edge1`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::smoothstep;
///
/// assert!((smoothstep(0.0, 1.0, 0.5) - 0.5).abs() < 1e-6);
/// assert_eq!(smoothstep(0.0, 1.0, 0.0), 0.0);
/// assert_eq!(smoothstep(0.0, 1.0, 1.0), 1.0);
/// ```
#[must_use]
#[inline]
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Smoother step: 6th-degree Hermite — zero first AND second derivatives at edges.
///
/// Less ringing than `smoothstep` for animation curves.
#[must_use]
#[inline]
pub fn smootherstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Remaps `x` from the range \[`from_min`, `from_max`\] to \[`to_min`, `to_max`\].
///
/// Does not clamp; extrapolates outside the input range.
///
/// # Examples
///
/// ```
/// use abrash_core::math::remap;
///
/// assert_eq!(remap(5.0, 0.0, 10.0, 0.0, 1.0), 0.5);
/// ```
#[must_use]
#[inline]
pub fn remap(x: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
    let t = (x - from_min) / (from_max - from_min);
    to_min + t * (to_max - to_min)
}

/// Shortest-path lerp between two angles (in radians).
///
/// Interpolates by the shortest arc, wrapping through the ±π boundary
/// so that `lerp_angle(3.1, -3.1, 0.5)` goes through π rather than
/// all the way around.
///
/// # Examples
///
/// ```
/// use abrash_core::math::lerp_angle;
/// use std::f32::consts::PI;
///
/// // Halfway between 0 and PI/2 → PI/4
/// let a = lerp_angle(0.0, PI / 2.0, 0.5);
/// assert!((a - PI / 4.0).abs() < 1e-5);
///
/// // Wraps correctly: from 3.0 to -3.0 goes through ±PI
/// let b = lerp_angle(3.0, -3.0, 0.5);
/// assert!(b.abs() > 3.0 || b.abs() < 0.2, "should be near ±PI, got {b}");
/// ```
#[must_use]
#[inline]
pub fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    use std::f32::consts::TAU;
    let diff =
        ((b - a).rem_euclid(TAU) + std::f32::consts::PI).rem_euclid(TAU) - std::f32::consts::PI;
    a + diff * t
}

/// Signed shortest angular difference from `a` to `b` (both in radians).
///
/// Returns a value in `(-π, π]`: positive means counter-clockwise.
///
/// # Examples
///
/// ```
/// use abrash_core::math::angle_diff;
/// use std::f32::consts::PI;
///
/// assert!((angle_diff(0.0, PI / 2.0) - PI / 2.0).abs() < 1e-5);
/// // Going the short way: from 3.0 rad to -3.0 rad is ~0.28 rad
/// let d = angle_diff(3.0, -3.0);
/// assert!(d.abs() < 0.3, "short arc, got {d}");
/// ```
#[must_use]
#[inline]
pub fn angle_diff(a: f32, b: f32) -> f32 {
    use std::f32::consts::TAU;
    ((b - a).rem_euclid(TAU) + std::f32::consts::PI).rem_euclid(TAU) - std::f32::consts::PI
}

/// Ping-pong (triangle wave): bounces `t` back and forth between 0 and `length`.
///
/// Equivalent to `abs(fract(t / (2·length)) * 2·length - length)`.
/// Returns a value always in `[0, length]` with no discontinuities.
///
/// # Examples
///
/// ```
/// use abrash_core::math::ping_pong;
///
/// assert!((ping_pong(0.0,  1.0) - 0.0).abs() < 1e-5);
/// assert!((ping_pong(0.5,  1.0) - 0.5).abs() < 1e-5);
/// assert!((ping_pong(1.0,  1.0) - 1.0).abs() < 1e-5);
/// assert!((ping_pong(1.5,  1.0) - 0.5).abs() < 1e-5); // bouncing back
/// assert!((ping_pong(2.0,  1.0) - 0.0).abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn ping_pong(t: f32, length: f32) -> f32 {
    let period = 2.0 * length;
    let t = t - (t / period).floor() * period;
    if t < length { t } else { period - t }
}

/// Map a value from one range to another.
///
/// Equivalent to `lerp(to_min, to_max, (x - from_min) / (from_max - from_min))`.
/// Does NOT clamp the output — use [`remap`] for a clamped version.
///
/// # Examples
///
/// ```
/// use abrash_core::math::map_range;
///
/// assert!((map_range(5.0, 0.0, 10.0, 0.0, 1.0) - 0.5).abs() < 1e-5);
/// assert!((map_range(0.0, 0.0, 10.0, -1.0, 1.0) - (-1.0)).abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn map_range(x: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
    let t = (x - from_min) / (from_max - from_min);
    to_min + t * (to_max - to_min)
}

/// Wrap `x` into `[min, max)` by repeating.
///
/// Similar to GLSL `mod(x - min, max - min) + min`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::wrap;
///
/// assert!((wrap(1.5, 0.0, 1.0) - 0.5).abs() < 1e-5);
/// assert!((wrap(-0.5, 0.0, 1.0) - 0.5).abs() < 1e-5);
/// assert!((wrap(3.0, 1.0, 4.0) - 3.0).abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn wrap(x: f32, min: f32, max: f32) -> f32 {
    let range = max - min;
    min + (x - min).rem_euclid(range)
}

/// Evaluate a quadratic (degree-2) Bézier curve at `t ∈ [0, 1]`.
///
/// Uses de Casteljau's algorithm: two linear lerps then one more.
/// `p0`, `p1`, `p2` are the control points; `p0` and `p2` are endpoints.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{quadratic_bezier_eval, Vec3};
///
/// let p0 = Vec3::ZERO;
/// let p1 = Vec3::new(0.5, 1.0, 0.0);
/// let p2 = Vec3::new(1.0, 0.0, 0.0);
///
/// // At t=0 we should be at p0
/// let at0 = quadratic_bezier_eval(p0, p1, p2, 0.0);
/// assert!((at0 - p0).length() < 1e-5);
///
/// // At t=1 we should be at p2
/// let at1 = quadratic_bezier_eval(p0, p1, p2, 1.0);
/// assert!((at1 - p2).length() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn quadratic_bezier_eval(p0: Vec3, p1: Vec3, p2: Vec3, t: f32) -> Vec3 {
    let q0 = p0.lerp(p1, t);
    let q1 = p1.lerp(p2, t);
    q0.lerp(q1, t)
}

/// Evaluate a cubic (degree-3) Bézier curve at `t ∈ [0, 1]`.
///
/// Uses de Casteljau's algorithm with three control points plus two endpoints.
/// `p0`..`p3` are the four control points; `p0` and `p3` are endpoints.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{cubic_bezier_eval, Vec3};
///
/// let p0 = Vec3::ZERO;
/// let p1 = Vec3::new(0.0, 1.0, 0.0);
/// let p2 = Vec3::new(1.0, 1.0, 0.0);
/// let p3 = Vec3::new(1.0, 0.0, 0.0);
///
/// let at0 = cubic_bezier_eval(p0, p1, p2, p3, 0.0);
/// assert!((at0 - p0).length() < 1e-5);
///
/// let at1 = cubic_bezier_eval(p0, p1, p2, p3, 1.0);
/// assert!((at1 - p3).length() < 1e-5);
///
/// // Midpoint should be between the endpoints
/// let mid = cubic_bezier_eval(p0, p1, p2, p3, 0.5);
/// assert!(mid.x > 0.0 && mid.x < 1.0);
/// assert!(mid.y > 0.0);
/// ```
#[must_use]
#[inline]
pub fn cubic_bezier_eval(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let q0 = p0.lerp(p1, t);
    let q1 = p1.lerp(p2, t);
    let q2 = p2.lerp(p3, t);
    let r0 = q0.lerp(q1, t);
    let r1 = q1.lerp(q2, t);
    r0.lerp(r1, t)
}

/// Convert spherical coordinates to Cartesian (physics convention).
///
/// - `r`: radial distance from origin
/// - `theta`: polar angle from the +Y axis, in `[0, π]`
/// - `phi`: azimuthal angle from the +X axis toward +Z, in `[0, 2π]`
///
/// Returns `(x, y, z) = (r sin θ cos φ, r cos θ, r sin θ sin φ)`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::spherical_to_cartesian;
/// use std::f32::consts::FRAC_PI_2;
///
/// // θ=0 → +Y pole
/// let p = spherical_to_cartesian(1.0, 0.0, 0.0);
/// assert!((p.y - 1.0).abs() < 1e-5);
///
/// // θ=π/2, φ=0 → +X equator
/// let q = spherical_to_cartesian(1.0, FRAC_PI_2, 0.0);
/// assert!((q.x - 1.0).abs() < 1e-5);
/// assert!(q.y.abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn spherical_to_cartesian(r: f32, theta: f32, phi: f32) -> Vec3 {
    let sin_theta = theta.sin();
    Vec3::new(
        r * sin_theta * phi.cos(),
        r * theta.cos(),
        r * sin_theta * phi.sin(),
    )
}

/// Convert Cartesian coordinates to spherical (physics convention).
///
/// Returns `(r, theta, phi)`:
/// - `r` ≥ 0: radial distance
/// - `theta` ∈ `[0, π]`: polar angle from +Y
/// - `phi` ∈ `[0, 2π]`: azimuthal angle from +X toward +Z
///
/// # Examples
///
/// ```
/// use abrash_core::math::{cartesian_to_spherical, Vec3};
///
/// let v = Vec3::new(0.0, 1.0, 0.0);
/// let (r, theta, phi) = cartesian_to_spherical(v);
/// assert!((r - 1.0).abs() < 1e-5);
/// assert!(theta.abs() < 1e-5); // pointing up = θ=0
/// ```
#[must_use]
#[inline]
pub fn cartesian_to_spherical(v: Vec3) -> (f32, f32, f32) {
    let r = v.length();
    if r < 1e-10 {
        return (0.0, 0.0, 0.0);
    }
    let theta = (v.y / r).clamp(-1.0, 1.0).acos();
    let phi = v.z.atan2(v.x).rem_euclid(std::f32::consts::TAU);
    (r, theta, phi)
}

/// Convert cylindrical coordinates to Cartesian.
///
/// - `r`: radial distance in XZ plane
/// - `theta`: azimuthal angle from +X toward +Z
/// - `y`: height along Y axis
///
/// # Examples
///
/// ```
/// use abrash_core::math::cylindrical_to_cartesian;
///
/// let p = cylindrical_to_cartesian(1.0, 0.0, 2.0);
/// assert!((p.x - 1.0).abs() < 1e-5);
/// assert!((p.y - 2.0).abs() < 1e-5);
/// assert!(p.z.abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn cylindrical_to_cartesian(r: f32, theta: f32, y: f32) -> Vec3 {
    Vec3::new(r * theta.cos(), y, r * theta.sin())
}

/// Convert Cartesian coordinates to cylindrical.
///
/// Returns `(r, theta, y)` where `theta ∈ [0, 2π]`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{cartesian_to_cylindrical, Vec3};
///
/// let v = Vec3::new(1.0, 3.0, 0.0);
/// let (r, theta, y) = cartesian_to_cylindrical(v);
/// assert!((r - 1.0).abs() < 1e-5);
/// assert!(theta.abs() < 1e-5);
/// assert!((y - 3.0).abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn cartesian_to_cylindrical(v: Vec3) -> (f32, f32, f32) {
    let r = (v.x * v.x + v.z * v.z).sqrt();
    let theta = v.z.atan2(v.x).rem_euclid(std::f32::consts::TAU);
    (r, theta, v.y)
}

/// Fast polynomial approximation of `atan2(y, x)`.
///
/// Maximum error is approximately 0.005 radians (~0.3°).
/// Returns angle in \[-PI, PI\].
///
/// About 3× faster than `f32::atan2` on modern hardware.
///
/// # Examples
///
/// ```
/// use abrash_core::math::fast_atan2;
/// use std::f32::consts::FRAC_PI_2;
///
/// let angle = fast_atan2(1.0, 0.0);
/// assert!((angle - FRAC_PI_2).abs() < 0.01);
/// ```
#[must_use]
#[inline]
pub fn fast_atan2(y: f32, x: f32) -> f32 {
    use std::f32::consts::{FRAC_PI_2, PI};
    // Minimax rational approximation — reduces to atan(t) on [-1, 1] then reflects.
    // Max error ~0.005 rad over the full circle.
    if x == 0.0 {
        return if y > 0.0 {
            FRAC_PI_2
        } else if y < 0.0 {
            -FRAC_PI_2
        } else {
            0.0
        };
    }
    let atan = |t: f32| -> f32 {
        // Polynomial: atan(t) ≈ t * (PI/4 + 0.273 * (1 - |t|))  for |t| ≤ 1
        // This gives max error ~0.005 rad (classic ATAN approximation by Harris et al.)
        t * (std::f32::consts::FRAC_PI_4 + 0.273 * (1.0 - t.abs()))
    };
    if x.abs() >= y.abs() {
        let t = y / x;
        let a = atan(t);
        if x > 0.0 {
            a
        } else if y >= 0.0 {
            a + PI
        } else {
            a - PI
        }
    } else {
        let t = x / y;
        let a = atan(t);
        if y > 0.0 {
            FRAC_PI_2 - a
        } else {
            -FRAC_PI_2 - a
        }
    }
}

/// A 2-component vector, used for texture coordinates (UVs) and 2D positions.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
///
/// let uv = Vec2::new(0.5, 0.5);
/// assert_eq!(uv.x, 0.5);
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[allow(missing_docs)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    #[allow(missing_docs)]
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    #[allow(missing_docs)]
    pub const ONE: Self = Self { x: 1.0, y: 1.0 };
    #[allow(missing_docs)]
    pub const X: Self = Self { x: 1.0, y: 0.0 };
    #[allow(missing_docs)]
    pub const Y: Self = Self { x: 0.0, y: 1.0 };

    /// Creates a vector with both components set to `v`.
    #[must_use]
    #[inline]
    pub const fn splat(v: f32) -> Self {
        Self { x: v, y: v }
    }

    /// Creates a unit vector from an angle in radians (measured from +X axis, CCW).
    #[must_use]
    #[inline]
    pub fn from_angle(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self { x: c, y: s }
    }

    /// Returns the angle of this vector in radians (from +X axis, CCW), in \[-PI, PI\].
    #[must_use]
    #[inline]
    pub fn to_angle(self) -> f32 {
        fast_atan2(self.y, self.x)
    }

    /// Linearly interpolate between this vector and another.
    ///
    /// The `t` factor dictates the blend: `0.0` returns `self`, `1.0` returns `other`.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::Vec2;
    ///
    /// let start = Vec2::new(0.0, 0.0);
    /// let end = Vec2::new(10.0, 10.0);
    /// let mid = start.lerp(end, 0.5);
    ///
    /// assert_eq!(mid.x, 5.0);
    /// assert_eq!(mid.y, 5.0);
    /// ```
    #[must_use]
    #[inline(always)]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }

    /// Creates a new 2D vector.
    #[must_use]
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Calculates the Euclidean length (magnitude) of the vector.
    ///
    /// ⚡ Bolt: Calculates length using `(x*x + y*y).sqrt()` instead of `f32::hypot` to bypass
    /// expensive C-library safety checks for intermediate overflow/underflow, yielding ~74%
    /// performance improvement for standard coordinate manipulation where values do not
    /// approach f32 bounds.
    #[must_use]
    #[inline]
    #[allow(clippy::imprecise_flops)]
    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    /// Calculates squared length (magnitude²) of the vector.
    #[must_use]
    #[inline]
    pub fn length_sq(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    /// Calculates dot product between two vectors.
    #[must_use]
    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    /// 2D cross product (returns the z-component magnitude).
    ///
    /// Useful for winding/orientation tests and signed area calculations.
    #[must_use]
    #[inline]
    pub fn cross(self, other: Self) -> f32 {
        self.x * other.y - self.y * other.x
    }

    /// Returns a normalized unit vector (length of 1.0).
    ///
    /// For tiny vectors (length² <= `1e-8`), returns the original vector.
    #[must_use]
    #[inline]
    pub fn normalize(self) -> Self {
        let len_sq = self.length_sq();
        if len_sq > 0.000_000_01 {
            let inv_len = len_sq.sqrt().recip();
            Self {
                x: self.x * inv_len,
                y: self.y * inv_len,
            }
        } else {
            self
        }
    }

    /// Returns a perpendicular vector (rotated 90° counter-clockwise).
    #[must_use]
    #[inline]
    pub fn perp(self) -> Self {
        Self {
            x: -self.y,
            y: self.x,
        }
    }

    /// Rotates the vector by `angle` radians.
    ///
    /// Uses [`fast_sin_cos`] to reduce trig overhead in tight animation/render loops.
    #[must_use]
    #[inline]
    pub fn rotate(self, angle: f32) -> Self {
        let (s, c) = fast_sin_cos(angle);
        Self {
            x: self.x * c - self.y * s,
            y: self.x * s + self.y * c,
        }
    }

    /// Distance to another vector.
    #[must_use]
    #[inline]
    #[allow(clippy::imprecise_flops)]
    pub fn distance(self, other: Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Squared distance to another vector.
    #[must_use]
    #[inline]
    pub fn distance_sq(self, other: Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }

    /// Returns a normalized unit vector or zero for tiny inputs.
    ///
    /// Unlike `normalize`, this never returns denormal tiny vectors.
    #[must_use]
    #[inline]
    pub fn normalize_or_zero(self) -> Self {
        let len_sq = self.length_sq();
        if len_sq > 0.000_000_01 {
            let inv_len = len_sq.sqrt().recip();
            Self::new(self.x * inv_len, self.y * inv_len)
        } else {
            Self::ZERO
        }
    }

    /// Moves this point toward `target` by at most `max_delta`.
    #[must_use]
    #[inline]
    pub fn move_towards(self, target: Self, max_delta: f32) -> Self {
        let to = target - self;
        let dist_sq = to.length_sq();
        if dist_sq <= max_delta * max_delta || dist_sq <= f32::EPSILON {
            target
        } else {
            let inv_dist = dist_sq.sqrt().recip();
            self + to * (max_delta * inv_dist)
        }
    }

    /// Projects this vector onto another vector.
    ///
    /// Returns `Vec2::ZERO` when `onto` is near zero to avoid division by tiny values.
    #[must_use]
    #[inline]
    pub fn project_onto(self, onto: Self) -> Self {
        let denom = onto.length_sq();
        if denom <= 0.000_000_01 {
            return Self::ZERO;
        }
        onto * (self.dot(onto) / denom)
    }

    /// Reject this vector from another vector (component orthogonal to `onto`).
    #[must_use]
    #[inline]
    pub fn reject_from(self, onto: Self) -> Self {
        self - self.project_onto(onto)
    }

    /// Returns angle between vectors in radians.
    ///
    /// Returns 0 for near-zero length inputs.
    #[must_use]
    #[inline]
    pub fn angle_between(self, other: Self) -> f32 {
        let denom = self.length() * other.length();
        if denom <= 0.000_000_01 {
            return 0.0;
        }
        (self.dot(other) / denom).clamp(-1.0, 1.0).acos()
    }

    /// Component-wise minimum.
    #[must_use]
    #[inline]
    pub const fn min(self, other: Self) -> Self {
        Self {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
        }
    }

    /// Component-wise maximum.
    #[must_use]
    #[inline]
    pub const fn max(self, other: Self) -> Self {
        Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
        }
    }

    /// Clamp each component between corresponding min/max components.
    #[must_use]
    #[inline]
    pub const fn clamp(self, min: Self, max: Self) -> Self {
        Self {
            x: self.x.clamp(min.x, max.x),
            y: self.y.clamp(min.y, max.y),
        }
    }

    /// Component-wise absolute value.
    #[must_use]
    #[inline]
    pub fn abs(self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }

    /// Component-wise sign: `−1.0`, `0.0`, or `+1.0`.
    #[must_use]
    #[inline]
    pub fn sign(self) -> Self {
        Self {
            x: self.x.signum(),
            y: self.y.signum(),
        }
    }

    /// Component-wise floor (round toward negative infinity).
    #[must_use]
    #[inline]
    pub fn floor(self) -> Self {
        Self {
            x: self.x.floor(),
            y: self.y.floor(),
        }
    }

    /// Component-wise ceiling (round toward positive infinity).
    #[must_use]
    #[inline]
    pub fn ceil(self) -> Self {
        Self {
            x: self.x.ceil(),
            y: self.y.ceil(),
        }
    }

    /// Component-wise round (round to nearest, ties to even).
    #[must_use]
    #[inline]
    pub fn round(self) -> Self {
        Self {
            x: self.x.round(),
            y: self.y.round(),
        }
    }

    /// Component-wise fractional part (`x - floor(x)`), matching GLSL semantics (result in [0, 1)).
    #[must_use]
    #[inline]
    pub fn fract(self) -> Self {
        Self {
            x: self.x - self.x.floor(),
            y: self.y - self.y.floor(),
        }
    }

    /// Component-wise step: returns `1.0` if `self >= edge`, else `0.0`.
    ///
    /// GLSL equivalent of `step(edge, x)`.
    #[must_use]
    #[inline]
    pub fn step(self, edge: Self) -> Self {
        Self {
            x: if self.x >= edge.x { 1.0 } else { 0.0 },
            y: if self.y >= edge.y { 1.0 } else { 0.0 },
        }
    }

    /// Reflect `self` off a surface with the given unit `normal`.
    ///
    /// `normal` should be normalized for correct results.
    #[must_use]
    #[inline]
    pub fn reflect(self, normal: Self) -> Self {
        self - normal * (2.0 * self.dot(normal))
    }

    /// Component-wise Hermite smoothstep between `edge0` and `edge1`.
    ///
    /// Each component of `self` is clamped and smoothed independently,
    /// matching GLSL `smoothstep(edge0, edge1, self)`.
    #[must_use]
    #[inline]
    pub fn smoothstep(self, edge0: Self, edge1: Self) -> Self {
        let tx = ((self.x - edge0.x) / (edge1.x - edge0.x)).clamp(0.0, 1.0);
        let ty = ((self.y - edge0.y) / (edge1.y - edge0.y)).clamp(0.0, 1.0);
        Self::new(tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty))
    }

    /// The smallest of the two components.
    #[must_use]
    #[inline]
    pub fn min_component(self) -> f32 {
        self.x.min(self.y)
    }

    /// The largest of the two components.
    #[must_use]
    #[inline]
    pub fn max_component(self) -> f32 {
        self.x.max(self.y)
    }

    /// Construct from polar coordinates `(r, theta)`.
    ///
    /// `r` is the radius and `theta` is the angle in radians from the +x axis.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::Vec2;
    /// use std::f32::consts::FRAC_PI_2;
    ///
    /// let v = Vec2::from_polar(2.0, FRAC_PI_2);
    /// // Should point in the +y direction
    /// assert!((v.x).abs() < 1e-5);
    /// assert!((v.y - 2.0).abs() < 1e-5);
    /// ```
    #[must_use]
    #[inline]
    pub fn from_polar(r: f32, theta: f32) -> Self {
        let (s, c) = theta.sin_cos();
        Self::new(r * c, r * s)
    }

    /// Convert to polar coordinates `(r, theta)`.
    ///
    /// Returns `(radius, angle_radians)` where angle is in `[−π, π]`.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::Vec2;
    ///
    /// let v = Vec2::new(1.0, 0.0);
    /// let (r, theta) = v.to_polar();
    /// assert!((r - 1.0).abs() < 1e-5);
    /// assert!(theta.abs() < 1e-5);
    /// ```
    #[must_use]
    #[inline]
    pub fn to_polar(self) -> (f32, f32) {
        (self.length(), self.y.atan2(self.x))
    }

    /// Signed angle from `self` to `other` in radians, in `[−π, π]`.
    ///
    /// Positive = counter-clockwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::Vec2;
    /// use std::f32::consts::FRAC_PI_2;
    ///
    /// let right = Vec2::new(1.0, 0.0);
    /// let up    = Vec2::new(0.0, 1.0);
    /// let angle = right.angle_to(up);
    /// assert!((angle - FRAC_PI_2).abs() < 1e-5);
    /// ```
    #[must_use]
    #[inline]
    pub fn angle_to(self, other: Self) -> f32 {
        let cross = self.cross(other); // signed area
        let dot = self.dot(other);
        cross.atan2(dot)
    }
}

impl Add for Vec2 {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Vec2 {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;

    #[inline]
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

impl std::ops::Div<f32> for Vec2 {
    type Output = Self;

    #[inline]
    fn div(self, scalar: f32) -> Self {
        let inv = 1.0 / scalar;
        Self {
            x: self.x * inv,
            y: self.y * inv,
        }
    }
}

/// A 2x2 matrix, primarily used for 2D rotations and transformations.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Mat2, Vec2};
///
/// // Rotate 90 degrees (PI/2)
/// let rot = Mat2::rotation(std::f32::consts::FRAC_PI_2);
/// let v = Vec2::new(1.0, 0.0);
/// let v_prime = rot.transform(v);
///
/// // (1, 0) rotated 90 deg -> (0, 1)
/// assert!((v_prime.x).abs() < 1e-6);
/// assert!((v_prime.y - 1.0).abs() < 1e-6);
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct Mat2 {
    pub m: [[f32; 2]; 2],
}

impl Mat2 {
    /// Creates a 2D rotation matrix.
    ///
    /// * `angle`: Rotation angle in radians (counter-clockwise).
    #[must_use]
    pub fn rotation(angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();

        Self {
            m: [[cos, -sin], [sin, cos]],
        }
    }

    /// Transforms a vector by this matrix.
    ///
    /// # Performance
    ///
    /// Marked `#[inline]` to allow the compiler to optimize call overhead and potentially
    /// vectorize loops that call this function.
    #[must_use]
    #[inline]
    pub fn transform(&self, v: Vec2) -> Vec2 {
        Vec2 {
            x: self.m[0][0] * v.x + self.m[0][1] * v.y,
            y: self.m[1][0] * v.x + self.m[1][1] * v.y,
        }
    }

    /// Transform multiple vectors at once.
    #[must_use]
    pub fn transform_batch(&self, vertices: &[Vec2]) -> Vec<Vec2> {
        let m00 = self.m[0][0];
        let m01 = self.m[0][1];
        let m10 = self.m[1][0];
        let m11 = self.m[1][1];

        vertices
            .iter()
            .map(|&v| Vec2 {
                x: m00 * v.x + m01 * v.y,
                y: m10 * v.x + m11 * v.y,
            })
            .collect()
    }

    /// Transform vertices in place.
    pub fn transform_in_place(&self, vertices: &mut [Vec2]) {
        let m00 = self.m[0][0];
        let m01 = self.m[0][1];
        let m10 = self.m[1][0];
        let m11 = self.m[1][1];

        for v in vertices.iter_mut() {
            let x = v.x;
            let y = v.y;
            v.x = m00 * x + m01 * y;
            v.y = m10 * x + m11 * y;
        }
    }

    /// Identity matrix (no transformation).
    #[must_use]
    #[inline]
    pub const fn identity() -> Self {
        Self {
            m: [[1.0, 0.0], [0.0, 1.0]],
        }
    }

    /// Non-uniform scale matrix.
    #[must_use]
    #[inline]
    pub const fn scale(sx: f32, sy: f32) -> Self {
        Self {
            m: [[sx, 0.0], [0.0, sy]],
        }
    }

    /// Transpose (swap rows and columns).
    #[must_use]
    #[inline]
    pub const fn transpose(&self) -> Self {
        Self {
            m: [[self.m[0][0], self.m[1][0]], [self.m[0][1], self.m[1][1]]],
        }
    }

    /// Determinant: `ad - bc`.
    #[must_use]
    #[inline]
    pub fn determinant(&self) -> f32 {
        self.m[0][0] * self.m[1][1] - self.m[0][1] * self.m[1][0]
    }

    /// Inverse. Returns the zero matrix if the determinant is near zero.
    #[must_use]
    pub fn inverse(&self) -> Self {
        let det = self.determinant();
        if det.abs() < 1e-7 {
            return Self { m: [[0.0; 2]; 2] };
        }
        let inv = 1.0 / det;
        Self {
            m: [
                [self.m[1][1] * inv, -self.m[0][1] * inv],
                [-self.m[1][0] * inv, self.m[0][0] * inv],
            ],
        }
    }
}

impl std::ops::Mul for Mat2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self {
            m: [
                [
                    self.m[0][0] * rhs.m[0][0] + self.m[0][1] * rhs.m[1][0],
                    self.m[0][0] * rhs.m[0][1] + self.m[0][1] * rhs.m[1][1],
                ],
                [
                    self.m[1][0] * rhs.m[0][0] + self.m[1][1] * rhs.m[1][0],
                    self.m[1][0] * rhs.m[0][1] + self.m[1][1] * rhs.m[1][1],
                ],
            ],
        }
    }
}

/// A 3-component vector commonly used for positions, directions, and colors.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec3;
///
/// let v = Vec3::new(1.0, 2.0, 3.0);
/// assert_eq!(v.x, 1.0);
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[allow(missing_docs)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    /// Linearly interpolate between this vector and another.
    ///
    /// `t` is the interpolation factor (0.0 = self, 1.0 = other).
    #[must_use]
    #[inline(always)]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            z: self.z + (other.z - self.z) * t,
        }
    }

    /// Spherical interpolation between this vector and another.
    ///
    /// Inputs are treated as directions and normalized internally.
    /// Falls back to normalized linear interpolation when vectors are nearly parallel.
    #[must_use]
    #[inline]
    pub fn slerp(self, other: Self, t: f32) -> Self {
        let a = self.normalize_or_zero();
        let b = other.normalize_or_zero();
        let dot = a.dot(b).clamp(-1.0, 1.0);

        // For tiny angles, lerp is numerically more stable and faster.
        if dot > 0.999_5 {
            return a.lerp(b, t).normalize_or_zero();
        }

        let theta = dot.acos();
        let sin_theta = theta.sin();
        if sin_theta.abs() <= 1e-6 {
            return a;
        }

        let w0 = ((1.0 - t) * theta).sin() / sin_theta;
        let w1 = (t * theta).sin() / sin_theta;
        (a * w0) + (b * w1)
    }

    #[allow(missing_docs)]
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    #[allow(missing_docs)]
    pub const ONE: Self = Self {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    };
    /// Positive X axis.
    pub const X: Self = Self {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    /// Positive Y axis.
    pub const Y: Self = Self {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    /// Positive Z axis.
    pub const Z: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };
    /// World up direction (+Y).
    pub const UP: Self = Self {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    /// World right direction (+X).
    pub const RIGHT: Self = Self {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    /// World forward direction (-Z, right-handed camera convention).
    pub const FORWARD: Self = Self {
        x: 0.0,
        y: 0.0,
        z: -1.0,
    };

    /// Creates a vector with all components set to `v`.
    #[must_use]
    #[inline]
    pub const fn splat(v: f32) -> Self {
        Self { x: v, y: v, z: v }
    }

    /// Creates a new vector.
    #[must_use]
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Calculates the dot product with another vector.
    ///
    /// The dot product represents the projection of one vector onto another.
    /// *   Positive if pointing in similar direction.
    /// *   Zero if perpendicular.
    /// *   Negative if pointing in opposite directions.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::Vec3;
    ///
    /// let a = Vec3::new(1.0, 0.0, 0.0);
    /// let b = Vec3::new(0.5, 0.0, 0.0);
    /// let c = Vec3::new(0.0, 1.0, 0.0);
    ///
    /// // Parallel vectors
    /// assert_eq!(a.dot(b), 0.5);
    ///
    /// // Perpendicular vectors
    /// assert_eq!(a.dot(c), 0.0);
    /// ```
    ///
    /// # Performance
    ///
    /// Passes `self` by value rather than reference to avoid pointer indirection
    /// and improve register allocation for small `Copy` types.
    #[must_use]
    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Calculates the cross product with another vector.
    ///
    /// Returns a vector perpendicular to both input vectors using the Right-Hand Rule.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::Vec3;
    ///
    /// // X cross Y = Z (Right-Handed)
    /// let x = Vec3::new(1.0, 0.0, 0.0);
    /// let y = Vec3::new(0.0, 1.0, 0.0);
    /// let z = x.cross(y);
    ///
    /// assert_eq!(z, Vec3::new(0.0, 0.0, 1.0));
    /// ```
    ///
    /// # Performance
    ///
    /// Passes `self` by value rather than reference to avoid pointer indirection
    /// and improve register allocation for small `Copy` types.
    #[must_use]
    #[inline]
    pub fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    /// Calculates the Euclidean length (magnitude) of the vector.
    ///
    /// # Performance
    ///
    /// Passes `self` by value rather than reference to avoid pointer indirection
    /// and improve register allocation for small `Copy` types.
    #[must_use]
    #[inline]
    #[allow(clippy::imprecise_flops)]
    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Returns a normalized unit vector (length of 1.0).
    ///
    /// # Behavior for Small Vectors
    ///
    /// If the vector's length is less than `0.0001`, this function returns
    /// the original vector unchanged to avoid division by zero or precision issues.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::Vec3;
    ///
    /// let v = Vec3::new(0.0, 3.0, 4.0); // Length is 5
    /// let n = v.normalize();
    /// assert_eq!(n, Vec3::new(0.0, 0.6, 0.8));
    ///
    /// // Small vector behavior
    /// let tiny = Vec3::new(0.00001, 0.0, 0.0);
    /// assert_eq!(tiny.normalize(), tiny);
    /// ```
    ///
    /// # Performance
    ///
    /// Passes `self` by value rather than reference to avoid pointer indirection
    /// and improve register allocation for small `Copy` types.
    #[must_use]
    #[inline]
    pub fn normalize(self) -> Self {
        let len_sq = self.x * self.x + self.y * self.y + self.z * self.z;
        if len_sq > 0.000_000_01 {
            let inv_len = len_sq.sqrt().recip();
            Self {
                x: self.x * inv_len,
                y: self.y * inv_len,
                z: self.z * inv_len,
            }
        } else {
            self
        }
    }

    /// Returns a normalized unit vector using fast inverse square root approximation.
    ///
    /// This is faster than `normalize()` but slightly less accurate.
    /// Useful for lighting calculations where extreme precision is not required.
    ///
    /// # Performance
    ///
    /// Passes `self` by value rather than reference to avoid pointer indirection
    /// and improve register allocation for small `Copy` types.
    #[must_use]
    #[inline]
    pub fn fast_normalize(self) -> Self {
        let len_sq = self.length_sq();
        if len_sq > 0.000_000_01 {
            let inv_len = fast_inv_sqrt(len_sq);
            Self {
                x: self.x * inv_len,
                y: self.y * inv_len,
                z: self.z * inv_len,
            }
        } else {
            self
        }
    }

    /// Calculates the squared length (magnitude) of the vector.
    ///
    /// Faster than `length()` as it avoids a square root operation.
    /// Useful for comparing distances.
    ///
    /// # Performance
    ///
    /// Passes `self` by value rather than reference to avoid pointer indirection
    /// and improve register allocation for small `Copy` types.
    #[must_use]
    #[inline]
    pub fn length_sq(self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    /// Distance to another vector.
    #[must_use]
    #[inline]
    pub fn distance(self, other: Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Squared distance to another vector.
    #[must_use]
    #[inline]
    pub fn distance_sq(self, other: Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }

    /// Returns a normalized unit vector or zero for tiny inputs.
    ///
    /// Unlike `normalize`, this never returns denormal tiny vectors.
    #[must_use]
    #[inline]
    pub fn normalize_or_zero(self) -> Self {
        let len_sq = self.length_sq();
        if len_sq > 0.000_000_01 {
            let inv_len = len_sq.sqrt().recip();
            Self::new(self.x * inv_len, self.y * inv_len, self.z * inv_len)
        } else {
            Self::ZERO
        }
    }

    /// Moves this point toward `target` by at most `max_delta`.
    #[must_use]
    #[inline]
    pub fn move_towards(self, target: Self, max_delta: f32) -> Self {
        let to = target - self;
        let dist_sq = to.length_sq();
        if dist_sq <= max_delta * max_delta || dist_sq <= f32::EPSILON {
            target
        } else {
            let inv_dist = dist_sq.sqrt().recip();
            self + to * (max_delta * inv_dist)
        }
    }

    /// Reflects this vector around a given normal vector.
    ///
    /// The formula used is $v - 2 \cdot (v \cdot n) \cdot n$.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::Vec3;
    ///
    /// let v = Vec3::new(1.0, -1.0, 0.0);
    /// let n = Vec3::new(0.0, 1.0, 0.0);
    /// let r = v.reflect(n);
    ///
    /// assert!((r.x - 1.0).abs() < 1e-6);
    /// assert!((r.y - 1.0).abs() < 1e-6);
    /// assert!(r.z.abs() < 1e-6);
    /// ```
    ///
    /// # Performance
    ///
    /// This implementation takes `self` by value rather than by reference to avoid pointer indirection
    /// for a small struct. It also manually unfolds scalar components to avoid intermediate struct
    /// allocations and improve scalar instruction pipelining.
    #[must_use]
    #[inline]
    pub fn reflect(self, normal: Self) -> Self {
        // Equivalent to `self - normal * (2.0 * self.dot(normal))`
        // but manually unfolded to avoid intermediate Vec3 allocations
        // and allow better scalar instruction pipelining.
        let dot2 = 2.0 * (self.x * normal.x + self.y * normal.y + self.z * normal.z);
        Self {
            x: self.x - normal.x * dot2,
            y: self.y - normal.y * dot2,
            z: self.z - normal.z * dot2,
        }
    }

    /// Reflects this vector around a *unit-length* normal vector.
    ///
    /// This avoids any extra normalization/division and is ideal for hot shading paths
    /// where normals are already normalized.
    #[must_use]
    #[inline]
    pub fn reflect_normalized(self, unit_normal: Self) -> Self {
        let dot2 = 2.0 * self.dot(unit_normal);
        Self {
            x: self.x - unit_normal.x * dot2,
            y: self.y - unit_normal.y * dot2,
            z: self.z - unit_normal.z * dot2,
        }
    }

    /// Refracts this vector through a surface with the given normal.
    ///
    /// `eta` is the ratio of refractive indices (`n1 / n2`).
    /// Returns `Vec3::ZERO` when total internal reflection occurs.
    #[must_use]
    #[inline]
    pub fn refract(self, normal: Self, eta: f32) -> Self {
        let cos_i = (-self.dot(normal)).clamp(-1.0, 1.0);
        let k = 1.0 - eta * eta * (1.0 - cos_i * cos_i);
        if k < 0.0 {
            Self::ZERO
        } else {
            self * eta + normal * (eta * cos_i - k.sqrt())
        }
    }

    /// Returns this vector oriented to face away from a reference direction.
    ///
    /// Equivalent to GLSL `faceforward(n, i, nref)` when called as
    /// `n.face_forward(i, nref)`.
    #[must_use]
    #[inline]
    pub fn face_forward(self, incident: Self, reference_normal: Self) -> Self {
        if reference_normal.dot(incident) < 0.0 {
            self
        } else {
            self * -1.0
        }
    }

    /// Projects this vector onto another vector.
    ///
    /// Returns `Vec3::ZERO` when `onto` is near zero to avoid division by tiny values.
    #[must_use]
    #[inline]
    pub fn project_onto(self, onto: Self) -> Self {
        let denom = onto.length_sq();
        if denom <= 0.000_000_01 {
            return Self::ZERO;
        }
        onto * (self.dot(onto) / denom)
    }

    /// Projects this vector onto a *unit-length* direction.
    ///
    /// Returns `Vec3::ZERO` for degenerate inputs to avoid amplification of NaNs/Infs.
    #[must_use]
    #[inline]
    pub fn project_onto_normalized(self, onto_unit: Self) -> Self {
        let len_sq = onto_unit.length_sq();
        if len_sq <= 0.000_000_01 {
            return Self::ZERO;
        }
        onto_unit * self.dot(onto_unit)
    }

    /// Reject this vector from another vector (component orthogonal to `onto`).
    #[must_use]
    #[inline]
    pub fn reject_from(self, onto: Self) -> Self {
        self - self.project_onto(onto)
    }

    /// Returns angle between vectors in radians.
    ///
    /// Returns 0 for near-zero length inputs.
    #[must_use]
    #[inline]
    pub fn angle_between(self, other: Self) -> f32 {
        let denom = self.length() * other.length();
        if denom <= 0.000_000_01 {
            return 0.0;
        }
        (self.dot(other) / denom).clamp(-1.0, 1.0).acos()
    }

    /// Builds an orthonormal basis from this direction.
    ///
    /// Returns two unit vectors `(tangent, bitangent)` that are perpendicular
    /// The smallest of the three components.
    ///
    /// # Examples
    /// ```
    /// use abrash_core::math::Vec3;
    /// assert_eq!(Vec3::new(3.0, 1.0, 2.0).min_component(), 1.0);
    /// ```
    #[must_use]
    #[inline]
    pub fn min_component(self) -> f32 {
        self.x.min(self.y).min(self.z)
    }

    /// The largest of the three components.
    ///
    /// # Examples
    /// ```
    /// use abrash_core::math::Vec3;
    /// assert_eq!(Vec3::new(3.0, 1.0, 2.0).max_component(), 3.0);
    /// ```
    #[must_use]
    #[inline]
    pub fn max_component(self) -> f32 {
        self.x.max(self.y).max(self.z)
    }

    /// Component-wise `x^exp`.
    ///
    /// # Examples
    /// ```
    /// use abrash_core::math::Vec3;
    /// let v = Vec3::new(2.0, 3.0, 4.0).pow(2.0);
    /// assert!((v.x - 4.0).abs() < 1e-5);
    /// assert!((v.y - 9.0).abs() < 1e-5);
    /// ```
    #[must_use]
    #[inline]
    pub fn pow(self, exp: f32) -> Self {
        Self::new(self.x.powf(exp), self.y.powf(exp), self.z.powf(exp))
    }

    /// Component-wise `e^x`.
    ///
    /// # Examples
    /// ```
    /// use abrash_core::math::Vec3;
    /// let v = Vec3::new(0.0, 1.0, 2.0).exp();
    /// assert!((v.x - 1.0).abs() < 1e-5);
    /// ```
    #[must_use]
    #[inline]
    pub fn exp(self) -> Self {
        Self::new(self.x.exp(), self.y.exp(), self.z.exp())
    }

    /// Component-wise natural log `ln(x)`.  Returns `-inf` for non-positive components.
    ///
    /// # Examples
    /// ```
    /// use abrash_core::math::Vec3;
    /// let v = Vec3::new(1.0, std::f32::consts::E, 1.0).log();
    /// assert!(v.x.abs() < 1e-5);
    /// assert!((v.y - 1.0).abs() < 1e-4);
    /// ```
    #[must_use]
    #[inline]
    pub fn log(self) -> Self {
        Self::new(self.x.ln(), self.y.ln(), self.z.ln())
    }

    /// to the (normalized) input and each other.
    #[must_use]
    #[inline]
    pub fn orthonormal_basis(self) -> (Self, Self) {
        let n = if self.length_sq() > 0.000_000_01 {
            self.normalize()
        } else {
            Self::new(0.0, 0.0, 1.0)
        };

        let helper = if n.z.abs() < 0.999 {
            Self::new(0.0, 0.0, 1.0)
        } else {
            Self::new(0.0, 1.0, 0.0)
        };

        let tangent = helper.cross(n).normalize();
        let bitangent = n.cross(tangent);
        (tangent, bitangent)
    }

    /// Computes barycentric coordinates of this point relative to triangle `(a, b, c)`.
    ///
    /// Returns `None` for degenerate triangles (near-zero area).
    /// The returned vector stores `(u, v, w)` such that:
    /// `self = a * u + b * v + c * w` and `u + v + w = 1`.
    #[must_use]
    #[inline]
    pub fn barycentric_coordinates(self, a: Self, b: Self, c: Self) -> Option<Self> {
        let v0 = b - a;
        let v1 = c - a;
        let v2 = self - a;

        let d00 = v0.dot(v0);
        let d01 = v0.dot(v1);
        let d11 = v1.dot(v1);
        let d20 = v2.dot(v0);
        let d21 = v2.dot(v1);

        #[allow(clippy::suspicious_operation_groupings)]
        let denom = d00 * d11 - d01 * d01;
        if denom.abs() <= 1e-8 {
            return None;
        }

        let inv_denom = 1.0 / denom;
        let v = (d11 * d20 - d01 * d21) * inv_denom;
        let w = (d00 * d21 - d01 * d20) * inv_denom;
        let u = 1.0 - v - w;
        Some(Self::new(u, v, w))
    }

    /// Reconstructs a point from barycentric coordinates over triangle `(a, b, c)`.
    ///
    /// `bary` stores `(u, v, w)` weights corresponding to vertices `(a, b, c)`.
    #[must_use]
    #[inline]
    pub fn from_barycentric(a: Self, b: Self, c: Self, bary: Self) -> Self {
        a * bary.x + b * bary.y + c * bary.z
    }

    /// Linearly interpolate between this vector and another.
    ///
    /// `t` is the interpolation factor (0.0 = self, 1.0 = other).
    #[must_use]
    #[inline]
    pub const fn min(self, other: Self) -> Self {
        Self {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
            z: self.z.min(other.z),
        }
    }

    /// Returns a new vector containing the maximum value for each component.
    #[must_use]
    #[inline]
    pub const fn max(self, other: Self) -> Self {
        Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
            z: self.z.max(other.z),
        }
    }

    /// Component-wise absolute value.
    #[must_use]
    #[inline]
    pub const fn abs(self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
            z: self.z.abs(),
        }
    }

    /// Component-wise clamp.
    #[must_use]
    #[inline]
    pub const fn clamp(self, min: Self, max: Self) -> Self {
        Self {
            x: self.x.clamp(min.x, max.x),
            y: self.y.clamp(min.y, max.y),
            z: self.z.clamp(min.z, max.z),
        }
    }

    /// Returns true when all components are finite.
    #[must_use]
    #[inline]
    #[allow(clippy::missing_const_for_fn)]
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    /// Clamps vector magnitude to at most `max_length`.
    ///
    /// Useful for velocity limiting and stable iterative solvers.
    #[must_use]
    #[inline]
    pub fn clamp_length(self, max_length: f32) -> Self {
        if max_length <= 0.0 {
            return Self::ZERO;
        }

        let len_sq = self.length_sq();
        let max_sq = max_length * max_length;
        if len_sq <= max_sq || len_sq <= 0.000_000_01 {
            self
        } else {
            let scale = max_length * len_sq.sqrt().recip();
            self * scale
        }
    }

    /// Component-wise sign: `−1.0`, `0.0`, or `+1.0`.
    #[must_use]
    #[inline]
    pub fn sign(self) -> Self {
        Self {
            x: self.x.signum(),
            y: self.y.signum(),
            z: self.z.signum(),
        }
    }

    /// Component-wise floor (round toward negative infinity).
    #[must_use]
    #[inline]
    pub fn floor(self) -> Self {
        Self {
            x: self.x.floor(),
            y: self.y.floor(),
            z: self.z.floor(),
        }
    }

    /// Component-wise ceiling (round toward positive infinity).
    #[must_use]
    #[inline]
    pub fn ceil(self) -> Self {
        Self {
            x: self.x.ceil(),
            y: self.y.ceil(),
            z: self.z.ceil(),
        }
    }

    /// Component-wise round (round to nearest, ties to even).
    #[must_use]
    #[inline]
    pub fn round(self) -> Self {
        Self {
            x: self.x.round(),
            y: self.y.round(),
            z: self.z.round(),
        }
    }

    /// Component-wise fractional part (`x - floor(x)`), matching GLSL semantics (result in [0, 1)).
    #[must_use]
    #[inline]
    pub fn fract(self) -> Self {
        Self {
            x: self.x - self.x.floor(),
            y: self.y - self.y.floor(),
            z: self.z - self.z.floor(),
        }
    }

    /// Component-wise step: returns `1.0` if `self >= edge`, else `0.0`.
    ///
    /// GLSL equivalent of `step(edge, x)`.
    #[must_use]
    #[inline]
    pub fn step(self, edge: Self) -> Self {
        Self {
            x: if self.x >= edge.x { 1.0 } else { 0.0 },
            y: if self.y >= edge.y { 1.0 } else { 0.0 },
            z: if self.z >= edge.z { 1.0 } else { 0.0 },
        }
    }

    /// Component-wise Hermite smoothstep between `edge0` and `edge1`.
    ///
    /// Matches GLSL `smoothstep(edge0, edge1, self)`.
    #[must_use]
    #[inline]
    pub fn smoothstep(self, edge0: Self, edge1: Self) -> Self {
        let tx = ((self.x - edge0.x) / (edge1.x - edge0.x)).clamp(0.0, 1.0);
        let ty = ((self.y - edge0.y) / (edge1.y - edge0.y)).clamp(0.0, 1.0);
        let tz = ((self.z - edge0.z) / (edge1.z - edge0.z)).clamp(0.0, 1.0);
        Self::new(
            tx * tx * (3.0 - 2.0 * tx),
            ty * ty * (3.0 - 2.0 * ty),
            tz * tz * (3.0 - 2.0 * tz),
        )
    }
}

impl Add for Vec3 {
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl Sub for Vec3 {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    #[inline]
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

impl Mul for Vec3 {
    type Output = Self;
    #[inline]
    fn mul(self, other: Self) -> Self {
        Self {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z,
        }
    }
}

impl std::ops::Div<f32> for Vec3 {
    type Output = Self;
    #[inline]
    fn div(self, scalar: f32) -> Self {
        let inv = 1.0 / scalar;
        Self {
            x: self.x * inv,
            y: self.y * inv,
            z: self.z * inv,
        }
    }
}

/// A 4x4 transformation matrix used for 3D graphics.
///
/// stored in **Row-Major** order.
///
/// # Transformation Order
///
/// Since this library uses row vectors ($v \cdot M$), transformations are applied in the order they are multiplied.
/// To achieve the standard "Scale, then Rotate, then Translate" effect for a model matrix, you must multiply in that order:
///
/// $$ M_{model} = M_{scale} \cdot M_{rotate} \cdot M_{translate} $$
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Mat4, Vec3};
/// use std::f32::consts::PI;
///
/// // 1. Scale by 2
/// let scale = Mat4::scale(2.0, 2.0, 2.0);
/// // 2. Rotate 90 degrees around Y
/// let rotation = Mat4::rotation_y(PI / 2.0);
/// // 3. Translate by (10, 5, 0)
/// let translation = Mat4::translation(10.0, 5.0, 0.0);
///
/// // Combine: Scale -> Rotate -> Translate
/// let model_matrix = scale * rotation * translation;
///
/// // Apply to point (1, 0, 0)
/// let p = Vec3::new(1.0, 0.0, 0.0);
/// let (p_prime, _) = model_matrix.transform_point(p);
///
/// // Expected:
/// // (1,0,0) * 2 = (2,0,0)
/// // (2,0,0) rot Y 90 = (0,0,-2) (Right-Hand Rule)
/// // (0,0,-2) + (10,5,0) = (10, 5, -2)
/// assert!((p_prime.x - 10.0).abs() < 0.001);
/// assert!((p_prime.y - 5.0).abs() < 0.001);
/// assert!((p_prime.z - -2.0).abs() < 0.001);
/// ```
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct Mat4 {
    pub m: [[f32; 4]; 4],
}

impl Mat4 {
    /// Creates the standard identity matrix.
    ///
    /// The identity matrix leaves any vector or matrix it is multiplied with unchanged.
    /// Use this as the starting point for building transformation chains.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::{Mat4, Vec3};
    ///
    /// let identity = Mat4::identity();
    /// let v = Vec3::new(1.0, 2.0, 3.0);
    ///
    /// // The transform is a no-op:
    /// let (v_transformed, w) = identity.transform_point(v);
    /// assert_eq!(v_transformed, v);
    /// assert_eq!(w, 1.0);
    /// ```
    #[must_use]
    #[inline]
    pub const fn identity() -> Self {
        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Creates a translation matrix.
    ///
    /// # Arguments
    ///
    /// * `x` - Translation along the X axis.
    /// * `y` - Translation along the Y axis.
    /// * `z` - Translation along the Z axis.
    #[must_use]
    #[inline]
    pub const fn translation(x: f32, y: f32, z: f32) -> Self {
        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [x, y, z, 1.0],
            ],
        }
    }

    /// Creates a scaling matrix.
    #[must_use]
    #[inline]
    pub const fn scale(x: f32, y: f32, z: f32) -> Self {
        Self {
            m: [
                [x, 0.0, 0.0, 0.0],
                [0.0, y, 0.0, 0.0],
                [0.0, 0.0, z, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Returns the transposed matrix.
    #[must_use]
    #[inline]
    pub const fn transpose(&self) -> Self {
        let m = &self.m;
        Self {
            m: [
                [m[0][0], m[1][0], m[2][0], m[3][0]],
                [m[0][1], m[1][1], m[2][1], m[3][1]],
                [m[0][2], m[1][2], m[2][2], m[3][2]],
                [m[0][3], m[1][3], m[2][3], m[3][3]],
            ],
        }
    }

    /// Computes the matrix determinant.
    #[must_use]
    #[inline]
    pub fn determinant(&self) -> f32 {
        let m = &self.m;

        let a2323 = m[2][2] * m[3][3] - m[2][3] * m[3][2];
        let a1323 = m[2][1] * m[3][3] - m[2][3] * m[3][1];
        let a1223 = m[2][1] * m[3][2] - m[2][2] * m[3][1];
        let a0323 = m[2][0] * m[3][3] - m[2][3] * m[3][0];
        let a0223 = m[2][0] * m[3][2] - m[2][2] * m[3][0];
        let a0123 = m[2][0] * m[3][1] - m[2][1] * m[3][0];

        m[0][0] * (m[1][1] * a2323 - m[1][2] * a1323 + m[1][3] * a1223)
            - m[0][1] * (m[1][0] * a2323 - m[1][2] * a0323 + m[1][3] * a0223)
            + m[0][2] * (m[1][0] * a1323 - m[1][1] * a0323 + m[1][3] * a0123)
            - m[0][3] * (m[1][0] * a1223 - m[1][1] * a0223 + m[1][2] * a0123)
    }

    #[inline]
    fn is_affine(&self) -> bool {
        self.m[0][3].abs() <= f32::EPSILON
            && self.m[1][3].abs() <= f32::EPSILON
            && self.m[2][3].abs() <= f32::EPSILON
            && (self.m[3][3] - 1.0).abs() <= f32::EPSILON
    }

    /// Fast inverse for affine transforms (`R*S + T`), common in render loops.
    ///
    /// Falls back to zero matrix if non-invertible.
    #[must_use]
    pub fn inverse_affine(&self) -> Self {
        let m = &self.m;
        let a00 = m[0][0];
        let a01 = m[0][1];
        let a02 = m[0][2];
        let a10 = m[1][0];
        let a11 = m[1][1];
        let a12 = m[1][2];
        let a20 = m[2][0];
        let a21 = m[2][1];
        let a22 = m[2][2];

        let c00 = a11 * a22 - a12 * a21;
        let c01 = -(a10 * a22 - a12 * a20);
        let c02 = a10 * a21 - a11 * a20;
        let c10 = -(a01 * a22 - a02 * a21);
        let c11 = a00 * a22 - a02 * a20;
        let c12 = -(a00 * a21 - a01 * a20);
        let c20 = a01 * a12 - a02 * a11;
        let c21 = -(a00 * a12 - a02 * a10);
        let c22 = a00 * a11 - a01 * a10;

        let det = a00 * c00 + a01 * c01 + a02 * c02;
        if det.abs() < 1e-6 {
            return Self { m: [[0.0; 4]; 4] };
        }
        let inv_det = 1.0 / det;

        // inverse(upper3x3) == adjugate / det
        let b00 = c00 * inv_det;
        let b01 = c10 * inv_det;
        let b02 = c20 * inv_det;
        let b10 = c01 * inv_det;
        let b11 = c11 * inv_det;
        let b12 = c21 * inv_det;
        let b20 = c02 * inv_det;
        let b21 = c12 * inv_det;
        let b22 = c22 * inv_det;

        let tx = m[3][0];
        let ty = m[3][1];
        let tz = m[3][2];

        Self {
            m: [
                [b00, b01, b02, 0.0],
                [b10, b11, b12, 0.0],
                [b20, b21, b22, 0.0],
                [
                    -(tx * b00 + ty * b10 + tz * b20),
                    -(tx * b01 + ty * b11 + tz * b21),
                    -(tx * b02 + ty * b12 + tz * b22),
                    1.0,
                ],
            ],
        }
    }

    /// Normal matrix: the inverse-transpose of the upper-left 3×3.
    ///
    /// Used to correctly transform surface normals when the model matrix
    /// contains non-uniform scale.  A uniform-scale rotation matrix has
    /// `normal_matrix == rotation_part`, but non-uniform scale would shear
    /// normals without this correction.
    ///
    /// Returns `Mat3::identity()` if the matrix is singular.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::{Mat4, Mat3, Vec3};
    ///
    /// // Pure rotation: normal_matrix == upper-3x3
    /// let m = Mat4::rotation_y(0.5);
    /// let nm = m.normal_matrix();
    /// let n = Vec3::new(1.0, 0.0, 0.0);
    /// let by_nm  = nm.transform(n);
    /// let by_rot = Mat3::from_mat4(&m).transform(n);
    /// assert!((by_nm.x - by_rot.x).abs() < 1e-4);
    /// ```
    #[must_use]
    pub fn normal_matrix(&self) -> Mat3 {
        Mat3::from_mat4(self).inverse_transpose()
    }

    /// Creates a rotation matrix around the X axis.
    ///
    /// * `angle` - The angle in radians.
    #[must_use]
    #[inline]
    pub fn rotation_x(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, c, s, 0.0],
                [0.0, -s, c, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Creates a rotation matrix around the Y axis.
    ///
    /// * `angle` - The angle in radians.
    #[must_use]
    #[inline]
    pub fn rotation_y(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self {
            m: [
                [c, 0.0, -s, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [s, 0.0, c, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Creates a rotation matrix around the Z axis.
    ///
    /// * `angle` - The angle in radians.
    #[must_use]
    #[inline]
    pub fn rotation_z(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self {
            m: [
                [c, s, 0.0, 0.0],
                [-s, c, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Creates a rotation matrix around an arbitrary axis.
    ///
    /// If `axis` is near zero, returns identity.
    #[must_use]
    #[inline]
    pub fn rotation_axis(axis: Vec3, angle: f32) -> Self {
        let n = axis.normalize_or_zero();
        if n.length_sq() <= 1e-8 {
            return Self::identity();
        }

        let (s, c) = angle.sin_cos();
        let one_minus_c = 1.0 - c;
        let x = n.x;
        let y = n.y;
        let z = n.z;

        // Row-major for row-vector convention.
        Self {
            m: [
                [
                    c + x * x * one_minus_c,
                    x * y * one_minus_c + z * s,
                    x * z * one_minus_c - y * s,
                    0.0,
                ],
                [
                    y * x * one_minus_c - z * s,
                    c + y * y * one_minus_c,
                    y * z * one_minus_c + x * s,
                    0.0,
                ],
                [
                    z * x * one_minus_c + y * s,
                    z * y * one_minus_c - x * s,
                    c + z * z * one_minus_c,
                    0.0,
                ],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Creates an orthographic projection matrix.
    ///
    /// # Arguments
    ///
    /// * `left` - Left plane.
    /// * `right` - Right plane.
    /// * `bottom` - Bottom plane.
    /// * `top` - Top plane.
    /// * `near` - Distance to near clipping plane.
    /// * `far` - Distance to far clipping plane.
    #[must_use]
    #[inline]
    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let w = 1.0 / (right - left);
        let h = 1.0 / (top - bottom);
        let d = 1.0 / (near - far);

        Self {
            m: [
                [2.0 * w, 0.0, 0.0, 0.0],
                [0.0, 2.0 * h, 0.0, 0.0],
                [0.0, 0.0, 2.0 * d, 0.0],
                [
                    -(right + left) * w,
                    -(top + bottom) * h,
                    (far + near) * d,
                    1.0,
                ],
            ],
        }
    }

    /// Creates a perspective projection matrix.
    ///
    /// # Arguments
    ///
    /// * `fov` - Vertical field of view in radians.
    /// * `aspect` - Aspect ratio (width / height).
    /// * `near` - Distance to near clipping plane.
    /// * `far` - Distance to far clipping plane.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::Mat4;
    /// use std::f32::consts::PI;
    ///
    /// let proj = Mat4::perspective(PI / 4.0, 1.33, 0.1, 100.0);
    /// ```
    #[must_use]
    #[inline]
    pub fn perspective(fov: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (fov / 2.0).tan();
        let nf = 1.0 / (near - far);
        Self {
            m: [
                [f / aspect, 0.0, 0.0, 0.0],
                [0.0, f, 0.0, 0.0],
                [0.0, 0.0, (far + near) * nf, -1.0],
                [0.0, 0.0, 2.0 * far * near * nf, 0.0],
            ],
        }
    }

    /// Creates a View matrix (`LookAt`) for a camera.
    ///
    /// * `eye` - Position of the camera.
    /// * `target` - Point the camera is looking at.
    /// * `up` - The "up" direction in the world (usually Y-up).
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::{Mat4, Vec3};
    ///
    /// let eye = Vec3::new(0.0, 0.0, 5.0);
    /// let target = Vec3::new(0.0, 0.0, 0.0);
    /// let up = Vec3::new(0.0, 1.0, 0.0);
    /// let view = Mat4::look_at(eye, target, up);
    /// ```
    #[must_use]
    #[inline]
    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let f = (target - eye).normalize();
        let s = f.cross(up).normalize();
        let u = s.cross(f);
        Self {
            m: [
                [s.x, u.x, -f.x, 0.0],
                [s.y, u.y, -f.y, 0.0],
                [s.z, u.z, -f.z, 0.0],
                [-s.dot(eye), -u.dot(eye), f.dot(eye), 1.0],
            ],
        }
    }

    /// 3D shear matrix — skews one axis as a linear function of the other two.
    ///
    /// The six parameters shear the axes in pairs:
    /// - `xy`: X shifts by `xy * Y`
    /// - `xz`: X shifts by `xz * Z`
    /// - `yx`: Y shifts by `yx * X`
    /// - `yz`: Y shifts by `yz * Z`
    /// - `zx`: Z shifts by `zx * X`
    /// - `zy`: Z shifts by `zy * Y`
    ///
    /// Uses row-vector convention (`v * M`).
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::{Mat4, Vec3};
    ///
    /// // Shear X by 0.5*Y
    /// let m = Mat4::shear(0.5, 0.0, 0.0, 0.0, 0.0, 0.0);
    /// let v = Vec3::new(0.0, 2.0, 0.0);
    /// let (result, _) = m.transform_point(v);
    /// assert!((result.x - 1.0).abs() < 1e-5); // x = 0 + 0.5 * 2 = 1
    /// assert!((result.y - 2.0).abs() < 1e-5);
    /// ```
    #[must_use]
    pub fn shear(xy: f32, xz: f32, yx: f32, yz: f32, zx: f32, zy: f32) -> Self {
        // Row-vector: v * M.  Column j of M is the destination for basis vector j.
        // Row 0 = X basis:   x → x + yx*y + zx*z
        // Row 1 = Y basis:   y → xy*x + y + zy*z
        // Row 2 = Z basis:   z → xz*x + yz*y + z
        // Row-vector: result[j] = sum_i v[i] * m[i][j]
        // m[1][0] = xy gives:  x_out = x + xy*y  (X shifts by xy*Y)
        Self {
            m: [
                [1.0, yx, zx, 0.0],
                [xy, 1.0, zy, 0.0],
                [xz, yz, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Householder reflection matrix: reflect through the plane with given `normal`.
    ///
    /// The plane passes through `point` and has `normal` as its unit normal.
    /// Points on the plane are unchanged; points off the plane are mirrored.
    ///
    /// Uses row-vector convention (`v * M`).
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::{Mat4, Vec3};
    ///
    /// // Mirror through the XZ plane (normal = +Y, point = origin)
    /// let m = Mat4::reflect_plane(Vec3::new(0.0, 1.0, 0.0), Vec3::ZERO);
    /// let v = Vec3::new(1.0, 3.0, 2.0);
    /// let (r, _) = m.transform_point(v);
    /// assert!((r.x - 1.0).abs() < 1e-5);
    /// assert!((r.y + 3.0).abs() < 1e-5); // y flipped
    /// assert!((r.z - 2.0).abs() < 1e-5);
    /// ```
    #[must_use]
    pub fn reflect_plane(normal: Vec3, point: Vec3) -> Self {
        // Householder: R = I - 2 * n⊗n (for plane through origin)
        // For plane through `point`: translate to origin, reflect, translate back.
        // Expand: p' = p - 2*(p·n - d)*n  where d = point·n
        let n = normal.normalize();
        let nx = n.x;
        let ny = n.y;
        let nz = n.z;
        let d = point.dot(n); // signed distance from origin to plane
        // Row-vector form: v' = v * M
        // M = I - 2 * n⊗n (for plane through origin), with translation folded in
        Self {
            m: [
                [1.0 - 2.0 * nx * nx, -2.0 * ny * nx, -2.0 * nz * nx, 0.0],
                [-2.0 * nx * ny, 1.0 - 2.0 * ny * ny, -2.0 * nz * ny, 0.0],
                [-2.0 * nx * nz, -2.0 * ny * nz, 1.0 - 2.0 * nz * nz, 0.0],
                [2.0 * d * nx, 2.0 * d * ny, 2.0 * d * nz, 1.0],
            ],
        }
    }

    /// Unproject a screen-space point back to a world-space ray direction.
    ///
    /// This is the inverse of the full `v·(view * proj)` pipeline.
    /// Pass the combined view-projection matrix; the function inverts it and
    /// converts the NDC point back to world space.
    ///
    /// `screen_x` and `screen_y` are in `[0, width)` / `[0, height)` pixels (top-left origin).
    /// Returns the world-space ray direction (not normalized).
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::{Mat4, Vec3};
    ///
    /// let proj = Mat4::orthographic(-1.0, 1.0, -1.0, 1.0, 0.1, 100.0);
    /// // Unproject screen center
    /// let dir = Mat4::unproject(0.5, 0.5, 800, 600, &proj);
    /// ```
    #[must_use]
    pub fn unproject(
        screen_x: f32,
        screen_y: f32,
        viewport_width: u32,
        viewport_height: u32,
        view_proj: &Self,
    ) -> Vec3 {
        // Convert screen pixels to NDC [-1, 1]
        let ndc_x = (screen_x / viewport_width as f32) * 2.0 - 1.0;
        let ndc_y = 1.0 - (screen_y / viewport_height as f32) * 2.0; // flip Y
        // Use near (z=0 in NDC) and far (z=1 in NDC) points and invert VP
        let inv = view_proj.inverse();
        let near_h = Vec3::new(ndc_x, ndc_y, 0.0);
        let far_h = Vec3::new(ndc_x, ndc_y, 1.0);
        let (near_w, near_ww) = inv.transform_point(near_h);
        let (far_w, far_ww) = inv.transform_point(far_h);
        let near_pos = near_w * (1.0 / near_ww);
        let far_pos = far_w * (1.0 / far_ww);
        far_pos - near_pos
    }

    /// Transforms a point by this matrix.
    ///
    /// Returns a tuple `(transformed_point, w_component)`.
    /// The `w` component is the Homogeneous W coordinate, used for perspective division.
    /// In the rasterization pipeline, vertices are kept in this `(Vec3, w)` format
    /// until the very last moment (viewport mapping) to preserve perspective correctness.
    ///
    /// # Performance
    ///
    /// Marked `#[inline]` to allow the compiler to optimize call overhead and potentially
    /// vectorize loops that call this function.
    ///
    /// # Safety
    ///
    /// The SIMD implementation uses `_mm_load_ps` (load aligned packed single) for maximum performance.
    /// `Mat4` is marked `#[repr(align(16))]`, ensuring 16-byte alignment for all rows.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::{Mat4, Vec3};
    ///
    /// let m = Mat4::translation(1.0, 2.0, 3.0);
    /// let p = Vec3::new(0.0, 0.0, 0.0);
    /// let (p_prime, w) = m.transform_point(p);
    ///
    /// assert_eq!(p_prime, Vec3::new(1.0, 2.0, 3.0));
    /// assert_eq!(w, 1.0);
    ///
    /// // Example with Perspective Projection (where w != 1.0)
    /// let proj = Mat4::perspective(1.57, 1.0, 1.0, 10.0);
    /// let p_view = Vec3::new(0.0, 0.0, -5.0); // Point in front of camera
    /// let (p_clip, w_clip) = proj.transform_point(p_view);
    ///
    /// // In standard perspective projection, w_clip = -z_view
    /// assert_eq!(w_clip, 5.0);
    /// ```
    ///
    /// Marked `#[inline]` to allow cross-crate inlining and auto-vectorization by the compiler.
    #[must_use]
    #[inline]
    pub fn transform_point(&self, v: Vec3) -> (Vec3, f32) {
        #[cfg(all(target_arch = "x86_64", feature = "simd"))]
        // SAFETY: Mat4 is 16-byte aligned, so _mm_load_ps is safe and optimal.
        unsafe {
            use std::arch::x86_64::{
                _mm_add_ps, _mm_load_ps, _mm_mul_ps, _mm_set1_ps, _mm_storeu_ps,
            };

            let row0 = _mm_load_ps(self.m[0].as_ptr());
            let row1 = _mm_load_ps(self.m[1].as_ptr());
            let row2 = _mm_load_ps(self.m[2].as_ptr());
            let row3 = _mm_load_ps(self.m[3].as_ptr());

            let vx = _mm_set1_ps(v.x);
            let vy = _mm_set1_ps(v.y);
            let vz = _mm_set1_ps(v.z);

            let t0 = _mm_mul_ps(vx, row0);
            let t1 = _mm_mul_ps(vy, row1);
            let t2 = _mm_mul_ps(vz, row2);

            let res = _mm_add_ps(_mm_add_ps(t0, t1), _mm_add_ps(t2, row3));

            let mut out = [0.0; 4];
            _mm_storeu_ps(out.as_mut_ptr(), res);

            (Vec3::new(out[0], out[1], out[2]), out[3])
        }

        #[cfg(not(all(target_arch = "x86_64", feature = "simd")))]
        {
            let x = self.m[0][0] * v.x + self.m[1][0] * v.y + self.m[2][0] * v.z + self.m[3][0];
            let y = self.m[0][1] * v.x + self.m[1][1] * v.y + self.m[2][1] * v.z + self.m[3][1];
            let z = self.m[0][2] * v.x + self.m[1][2] * v.y + self.m[2][2] * v.z + self.m[3][2];
            let w = self.m[0][3] * v.x + self.m[1][3] * v.y + self.m[2][3] * v.z + self.m[3][3];
            (Vec3::new(x, y, z), w)
        }
    }

    /// Transforms a direction vector (w=0), ignoring translation.
    #[must_use]
    #[inline]
    pub fn transform_vector(&self, v: Vec3) -> Vec3 {
        let x = self.m[0][0] * v.x + self.m[1][0] * v.y + self.m[2][0] * v.z;
        let y = self.m[0][1] * v.x + self.m[1][1] * v.y + self.m[2][1] * v.z;
        let z = self.m[0][2] * v.x + self.m[1][2] * v.y + self.m[2][2] * v.z;
        Vec3::new(x, y, z)
    }

    /// Transforms a batch of direction vectors (w=0), ignoring translation.
    ///
    /// This hoists matrix elements out of the loop to reduce indexing overhead
    /// in hot inner loops.
    ///
    /// # Panics
    ///
    /// Panics if `vectors.len() != output.len()`.
    pub fn transform_vectors(&self, vectors: &[Vec3], output: &mut [Vec3]) {
        assert_eq!(vectors.len(), output.len());

        let m00 = self.m[0][0];
        let m01 = self.m[0][1];
        let m02 = self.m[0][2];
        let m10 = self.m[1][0];
        let m11 = self.m[1][1];
        let m12 = self.m[1][2];
        let m20 = self.m[2][0];
        let m21 = self.m[2][1];
        let m22 = self.m[2][2];

        for (v, out) in vectors.iter().zip(output.iter_mut()) {
            *out = Vec3::new(
                v.x * m00 + v.y * m10 + v.z * m20,
                v.x * m01 + v.y * m11 + v.z * m21,
                v.x * m02 + v.y * m12 + v.z * m22,
            );
        }
    }

    /// Fast path for transforming points by affine matrices (`w` remains 1).
    ///
    /// When the matrix is affine, this avoids computing/storing the homogeneous `w`
    /// component for every vertex.
    ///
    /// # Panics
    ///
    /// Panics if `points.len() != output.len()`.
    pub fn transform_points_affine(&self, points: &[Vec3], output: &mut [Vec3]) {
        assert_eq!(points.len(), output.len());

        if !self.is_affine() {
            for (point, out) in points.iter().zip(output.iter_mut()) {
                *out = self.transform_point(*point).0;
            }
            return;
        }

        let m00 = self.m[0][0];
        let m01 = self.m[0][1];
        let m02 = self.m[0][2];
        let m10 = self.m[1][0];
        let m11 = self.m[1][1];
        let m12 = self.m[1][2];
        let m20 = self.m[2][0];
        let m21 = self.m[2][1];
        let m22 = self.m[2][2];
        let m30 = self.m[3][0];
        let m31 = self.m[3][1];
        let m32 = self.m[3][2];

        for (p, out) in points.iter().zip(output.iter_mut()) {
            *out = Vec3::new(
                p.x * m00 + p.y * m10 + p.z * m20 + m30,
                p.x * m01 + p.y * m11 + p.z * m21 + m31,
                p.x * m02 + p.y * m12 + p.z * m22 + m32,
            );
        }
    }

    /// Transforms multiple points by this matrix.
    ///
    /// Output buffer must have same length as input points.
    /// Returns (`transformed_point`, `w_component`) for each point.
    ///
    /// # Panics
    ///
    /// Panics if `points.len()` does not equal `output.len()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::{Mat4, Vec3};
    ///
    /// let m = Mat4::scale(2.0, 2.0, 2.0);
    /// let points = [Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)];
    /// let mut output = vec![(Vec3::default(), 0.0); 2];
    ///
    /// m.transform_points(&points, &mut output);
    ///
    /// assert_eq!(output[0].0, Vec3::new(2.0, 0.0, 0.0));
    /// assert_eq!(output[1].0, Vec3::new(0.0, 2.0, 0.0));
    /// ```
    pub fn transform_points(&self, points: &[Vec3], output: &mut [(Vec3, f32)]) {
        // SAFETY: (Vec3, f32) has same layout as MaybeUninit<(Vec3, f32)>
        let output_uninit = unsafe {
            &mut *(std::ptr::from_mut::<[(Vec3, f32)]>(output) as *mut [MaybeUninit<(Vec3, f32)>])
        };
        self.transform_points_uninit(points, output_uninit);
    }

    /// Transforms multiple points by this matrix into uninitialized memory.
    ///
    /// # Panics
    ///
    /// Panics if `points.len()` does not equal `output.len()`.
    pub fn transform_points_uninit(
        &self,
        points: &[Vec3],
        output: &mut [MaybeUninit<(Vec3, f32)>],
    ) {
        assert_eq!(points.len(), output.len());

        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        if is_x86_feature_detected!("avx2") {
            unsafe {
                self.transform_points_avx2(points, output);
            }
            return;
        }

        self.transform_points_scalar_uninit(points, output);
    }

    #[inline]
    fn transform_points_scalar_uninit(
        &self,
        points: &[Vec3],
        output: &mut [MaybeUninit<(Vec3, f32)>],
    ) {
        let m00 = self.m[0][0];
        let m01 = self.m[0][1];
        let m02 = self.m[0][2];
        let m03 = self.m[0][3];
        let m10 = self.m[1][0];
        let m11 = self.m[1][1];
        let m12 = self.m[1][2];
        let m13 = self.m[1][3];
        let m20 = self.m[2][0];
        let m21 = self.m[2][1];
        let m22 = self.m[2][2];
        let m23 = self.m[2][3];
        let m30 = self.m[3][0];
        let m31 = self.m[3][1];
        let m32 = self.m[3][2];
        let m33 = self.m[3][3];

        for (p, out) in points.iter().zip(output.iter_mut()) {
            let x = p.x * m00 + p.y * m10 + p.z * m20 + m30;
            let y = p.x * m01 + p.y * m11 + p.z * m21 + m31;
            let z = p.x * m02 + p.y * m12 + p.z * m22 + m32;
            let w = p.x * m03 + p.y * m13 + p.z * m23 + m33;
            out.write((Vec3::new(x, y, z), w));
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    #[target_feature(enable = "avx2")]
    #[allow(clippy::wildcard_imports)]
    unsafe fn transform_points_avx2(
        &self,
        points: &[Vec3],
        output: &mut [MaybeUninit<(Vec3, f32)>],
    ) {
        use std::arch::x86_64::*;

        #[cfg(debug_assertions)]
        {
            assert_eq!(
                std::mem::size_of::<(Vec3, f32)>(),
                16,
                "Layout mismatch: (Vec3, f32) size != 16"
            );
            assert_eq!(
                std::mem::align_of::<(Vec3, f32)>(),
                4,
                "Layout mismatch: (Vec3, f32) align != 4"
            );
            // Verify offsets
            let dummy: (Vec3, f32) = (Vec3::new(0.0, 0.0, 0.0), 0.0);
            let base = &raw const dummy as usize;
            let x_ptr = &raw const dummy.0.x as usize;
            let w_ptr = &raw const dummy.1 as usize;
            assert_eq!(x_ptr - base, 0, "Offset of Vec3.x must be 0");
            assert_eq!(w_ptr - base, 12, "Offset of f32 must be 12");
        }

        let len = points.len();
        let mut i = 0;

        let m00 = _mm256_set1_ps(self.m[0][0]);
        let m01 = _mm256_set1_ps(self.m[0][1]);
        let m02 = _mm256_set1_ps(self.m[0][2]);
        let m03 = _mm256_set1_ps(self.m[0][3]);

        let m10 = _mm256_set1_ps(self.m[1][0]);
        let m11 = _mm256_set1_ps(self.m[1][1]);
        let m12 = _mm256_set1_ps(self.m[1][2]);
        let m13 = _mm256_set1_ps(self.m[1][3]);

        let m20 = _mm256_set1_ps(self.m[2][0]);
        let m21 = _mm256_set1_ps(self.m[2][1]);
        let m22 = _mm256_set1_ps(self.m[2][2]);
        let m23 = _mm256_set1_ps(self.m[2][3]);

        let m30 = _mm256_set1_ps(self.m[3][0]);
        let m31 = _mm256_set1_ps(self.m[3][1]);
        let m32 = _mm256_set1_ps(self.m[3][2]);
        let m33 = _mm256_set1_ps(self.m[3][3]);

        while i + 8 <= len {
            // SAFETY: Memory access is bounded by loop condition and caller guarantees.
            unsafe {
                let p_ptr = points.as_ptr().add(i).cast::<f32>();

                // Load 8 Vec3s (96 bytes) as 3 chunks of 32 bytes? No, SSE loads of 16 bytes.
                // 8 points * 12 bytes = 96 bytes.
                // Load first 4 points (48 bytes) -> 3 * 16 bytes
                let r0 = _mm_loadu_ps(p_ptr); // x0 y0 z0 x1
                let r1 = _mm_loadu_ps(p_ptr.add(4)); // y1 z1 x2 y2
                let r2 = _mm_loadu_ps(p_ptr.add(8)); // z2 x3 y3 z3

                // Shuffle to SOA (x_lo, y_lo, z_lo)
                // _MM_SHUFFLE(z, y, x, w) -> (z << 6) | (y << 4) | (x << 2) | w
                let t_x0x1 = _mm_shuffle_ps(r0, r0, 0b11_00_11_00); // 3, 0, 3, 0
                let t_x2x3 = _mm_shuffle_ps(r1, r2, 0b01_01_10_10); // 1, 1, 2, 2
                // Mask for x_lo: a[0], a[1], b[0], b[2] -> 2, 0, 1, 0 -> 0x84
                let x_lo = _mm_shuffle_ps(t_x0x1, t_x2x3, 0b10_00_01_00);

                let t_y0y1 = _mm_shuffle_ps(r0, r1, 0b00_00_01_01); // 0, 0, 1, 1
                let t_y2y3 = _mm_shuffle_ps(r1, r2, 0b10_10_11_11); // 2, 2, 3, 3
                // Mask for y_lo: a[0], a[2], b[0], b[2] -> 2, 0, 2, 0 -> 0x88
                let y_lo = _mm_shuffle_ps(t_y0y1, t_y2y3, 0b10_00_10_00);

                let t_z0z1 = _mm_shuffle_ps(r0, r1, 0b01_01_10_10); // 1, 1, 2, 2
                let t_z2z3 = _mm_shuffle_ps(r2, r2, 0b11_00_11_00); // 3, 0, 3, 0
                // Mask for z_lo: a[0], a[2], b[0], b[1] -> 1, 0, 2, 0 -> 0x48
                let z_lo = _mm_shuffle_ps(t_z0z1, t_z2z3, 0b01_00_10_00);

                // Load next 4 points
                let r3 = _mm_loadu_ps(p_ptr.add(12)); // x4 y4 z4 x5
                let r4 = _mm_loadu_ps(p_ptr.add(16)); // y5 z5 x6 y6
                let r5 = _mm_loadu_ps(p_ptr.add(20)); // z6 x7 y7 z7

                let t_x4x5 = _mm_shuffle_ps(r3, r3, 0b11_00_11_00);
                let t_x6x7 = _mm_shuffle_ps(r4, r5, 0b01_01_10_10);
                let x_hi = _mm_shuffle_ps(t_x4x5, t_x6x7, 0b10_00_01_00);

                let t_y4y5 = _mm_shuffle_ps(r3, r4, 0b00_00_01_01);
                let t_y6y7 = _mm_shuffle_ps(r4, r5, 0b10_10_11_11);
                let y_hi = _mm_shuffle_ps(t_y4y5, t_y6y7, 0b10_00_10_00);

                let t_z4z5 = _mm_shuffle_ps(r3, r4, 0b01_01_10_10);
                let t_z6z7 = _mm_shuffle_ps(r5, r5, 0b11_00_11_00);
                let z_hi = _mm_shuffle_ps(t_z4z5, t_z6z7, 0b01_00_10_00);

                // Combine to AVX
                let vx = _mm256_insertf128_ps(_mm256_castps128_ps256(x_lo), x_hi, 1);
                let vy = _mm256_insertf128_ps(_mm256_castps128_ps256(y_lo), y_hi, 1);
                let vz = _mm256_insertf128_ps(_mm256_castps128_ps256(z_lo), z_hi, 1);

                // Matrix Multiplication
                // Match scalar order: ((x*m0 + y*m1) + z*m2) + m3
                let res_x = _mm256_add_ps(
                    _mm256_add_ps(
                        _mm256_add_ps(_mm256_mul_ps(vx, m00), _mm256_mul_ps(vy, m10)),
                        _mm256_mul_ps(vz, m20),
                    ),
                    m30,
                );

                let res_y = _mm256_add_ps(
                    _mm256_add_ps(
                        _mm256_add_ps(_mm256_mul_ps(vx, m01), _mm256_mul_ps(vy, m11)),
                        _mm256_mul_ps(vz, m21),
                    ),
                    m31,
                );

                let res_z = _mm256_add_ps(
                    _mm256_add_ps(
                        _mm256_add_ps(_mm256_mul_ps(vx, m02), _mm256_mul_ps(vy, m12)),
                        _mm256_mul_ps(vz, m22),
                    ),
                    m32,
                );

                let res_w = _mm256_add_ps(
                    _mm256_add_ps(
                        _mm256_add_ps(_mm256_mul_ps(vx, m03), _mm256_mul_ps(vy, m13)),
                        _mm256_mul_ps(vz, m23),
                    ),
                    m33,
                );

                // Transpose back to AOS (8x4)
                let t0 = _mm256_unpacklo_ps(res_x, res_z); // x0 z0 x1 z1 ...
                let t1 = _mm256_unpackhi_ps(res_x, res_z); // x2 z2 x3 z3 ...
                let t2 = _mm256_unpacklo_ps(res_y, res_w); // y0 w0 y1 w1 ...
                let t3 = _mm256_unpackhi_ps(res_y, res_w); // y2 w2 y3 w3 ...

                let out0 = _mm256_unpacklo_ps(t0, t2); // x0 y0 z0 w0 ...
                let out1 = _mm256_unpackhi_ps(t0, t2); // x1 y1 z1 w1 ...
                let out2 = _mm256_unpacklo_ps(t1, t3); // x2 y2 z2 w2 ...
                let out3 = _mm256_unpackhi_ps(t1, t3); // x3 y3 z3 w3 ...

                // Order: p0, p1, p2, p3, p4, p5, p6, p7
                let final0 = _mm256_permute2f128_ps(out0, out1, 0x20); // p0 | p1
                let final1 = _mm256_permute2f128_ps(out2, out3, 0x20); // p2 | p3
                let final2 = _mm256_permute2f128_ps(out0, out1, 0x31); // p4 | p5
                let final3 = _mm256_permute2f128_ps(out2, out3, 0x31); // p6 | p7

                // Store
                let out_ptr = output.as_mut_ptr().add(i).cast::<f32>();
                _mm256_storeu_ps(out_ptr, final0);
                _mm256_storeu_ps(out_ptr.add(8), final1);
                _mm256_storeu_ps(out_ptr.add(16), final2);
                _mm256_storeu_ps(out_ptr.add(24), final3);
            }

            i += 8;
        }

        while i < len {
            output[i].write(self.transform_point(points[i]));
            i += 1;
        }
    }

    /// Transforms multiple points by this matrix in parallel (if `parallel` feature is enabled).
    ///
    /// Output buffer must have same length as input points.
    /// Returns (`transformed_point`, `w_component`) for each point.
    ///
    /// # Panics
    ///
    /// Panics if `points.len()` does not equal `output.len()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::{Mat4, Vec3};
    ///
    /// let m = Mat4::translation(10.0, 0.0, 0.0);
    /// let points = [Vec3::new(0.0, 0.0, 0.0); 100];
    /// let mut output = vec![(Vec3::default(), 0.0); 100];
    ///
    /// m.transform_points_parallel(&points, &mut output);
    ///
    /// assert_eq!(output[0].0, Vec3::new(10.0, 0.0, 0.0));
    /// ```
    pub fn transform_points_parallel(&self, points: &[Vec3], output: &mut [(Vec3, f32)]) {
        // SAFETY: (Vec3, f32) has same layout as MaybeUninit<(Vec3, f32)>
        let output_uninit = unsafe {
            &mut *(std::ptr::from_mut::<[(Vec3, f32)]>(output) as *mut [MaybeUninit<(Vec3, f32)>])
        };
        self.transform_points_uninit_parallel(points, output_uninit);
    }

    /// Transforms multiple points by this matrix into uninitialized memory in parallel.
    ///
    /// # Panics
    ///
    /// Panics if `points.len()` does not equal `output.len()`.
    pub fn transform_points_uninit_parallel(
        &self,
        points: &[Vec3],
        output: &mut [MaybeUninit<(Vec3, f32)>],
    ) {
        assert_eq!(points.len(), output.len());

        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;

            // Chunk size of 4096 ensures we amortize task overhead and keep the AVX2
            // implementation fed with enough data to be efficient.
            const CHUNK_SIZE: usize = 4096;

            // Fallback to scalar for small inputs to avoid Rayon overhead
            if points.len() < 1024 {
                self.transform_points_uninit(points, output);
                return;
            }

            points
                .par_chunks(CHUNK_SIZE)
                .zip(output.par_chunks_mut(CHUNK_SIZE))
                .for_each(|(p_chunk, out_chunk)| {
                    self.transform_points_uninit(p_chunk, out_chunk);
                });
        }

        #[cfg(not(feature = "parallel"))]
        {
            self.transform_points_uninit(points, output);
        }
    }

    /// Transform a normal vector (ignores translation, uses upper-left 3x3).
    ///
    /// This is essential for correct lighting calculations after transformation.
    ///
    /// # Performance
    ///
    /// Marked `#[inline]` to allow the compiler to optimize call overhead and potentially
    /// vectorize loops that call this function.
    #[must_use]
    #[inline]
    pub fn transform_normal(&self, n: Vec3) -> Vec3 {
        let x = self.m[0][0] * n.x + self.m[1][0] * n.y + self.m[2][0] * n.z;
        let y = self.m[0][1] * n.x + self.m[1][1] * n.y + self.m[2][1] * n.z;
        let z = self.m[0][2] * n.x + self.m[1][2] * n.y + self.m[2][2] * n.z;
        Vec3::new(x, y, z).normalize()
    }

    /// Calculates the inverse of the matrix.
    ///
    /// Returns a zero matrix if the matrix is not invertible.
    #[must_use]
    pub fn inverse(&self) -> Self {
        if self.is_affine() {
            return self.inverse_affine();
        }

        #[cfg(all(target_arch = "x86_64", feature = "simd"))]
        {
            unsafe {
                use std::arch::x86_64::{
                    _mm_add_ps, _mm_castsi128_ps, _mm_div_ps, _mm_load_ps, _mm_mul_ps,
                    _mm_set_epi32, _mm_set1_ps, _mm_shuffle_ps, _mm_store_ps, _mm_storeu_ps,
                    _mm_sub_ps, _mm_xor_ps,
                };

                let row0 = _mm_load_ps(self.m[0].as_ptr());
                let row1 = _mm_load_ps(self.m[1].as_ptr());
                let row2 = _mm_load_ps(self.m[2].as_ptr());
                let row3 = _mm_load_ps(self.m[3].as_ptr());

                let t0 = _mm_shuffle_ps(row0, row1, 0x44);
                let t1 = _mm_shuffle_ps(row2, row3, 0x44);
                let t2 = _mm_shuffle_ps(row0, row1, 0xEE);
                let t3 = _mm_shuffle_ps(row2, row3, 0xEE);

                let c0 = _mm_shuffle_ps(t0, t1, 0x88);
                let c1 = _mm_shuffle_ps(t0, t1, 0xDD);
                let c2 = _mm_shuffle_ps(t2, t3, 0x88);
                let c3 = _mm_shuffle_ps(t2, t3, 0xDD);

                let mut fac0 = _mm_shuffle_ps(c2, c2, 0x50);
                let mut fac1 = _mm_shuffle_ps(c3, c3, 0xEE);
                let mut fac2 = _mm_shuffle_ps(c2, c2, 0x05);
                let mut fac3 = _mm_shuffle_ps(c3, c3, 0xAF);

                let mut v0 = _mm_mul_ps(fac0, fac1);
                v0 = _mm_sub_ps(v0, _mm_mul_ps(fac2, fac3));

                fac0 = _mm_shuffle_ps(c1, c1, 0x50);
                fac1 = _mm_shuffle_ps(c3, c3, 0xEE);
                fac2 = _mm_shuffle_ps(c1, c1, 0x05);
                fac3 = _mm_shuffle_ps(c3, c3, 0xAF);
                let mut v1 = _mm_mul_ps(fac0, fac1);
                v1 = _mm_sub_ps(v1, _mm_mul_ps(fac2, fac3));

                fac0 = _mm_shuffle_ps(c1, c1, 0x50);
                fac1 = _mm_shuffle_ps(c2, c2, 0xEE);
                fac2 = _mm_shuffle_ps(c1, c1, 0x05);
                fac3 = _mm_shuffle_ps(c2, c2, 0xAF);
                let mut v2 = _mm_mul_ps(fac0, fac1);
                v2 = _mm_sub_ps(v2, _mm_mul_ps(fac2, fac3));

                let sign_a =
                    _mm_castsi128_ps(_mm_set_epi32(-2_147_483_648_i32, 0, -2_147_483_648_i32, 0));
                let sign_b =
                    _mm_castsi128_ps(_mm_set_epi32(0, -2_147_483_648_i32, 0, -2_147_483_648_i32));

                let mut inv0 = _mm_mul_ps(c1, _mm_shuffle_ps(v0, v0, 0x39));
                inv0 = _mm_sub_ps(inv0, _mm_mul_ps(c2, _mm_shuffle_ps(v1, v1, 0x39)));
                inv0 = _mm_add_ps(inv0, _mm_mul_ps(c3, _mm_shuffle_ps(v2, v2, 0x39)));
                inv0 = _mm_xor_ps(inv0, sign_b);

                let mut inv1 = _mm_mul_ps(c0, _mm_shuffle_ps(v0, v0, 0x39));
                inv1 = _mm_sub_ps(inv1, _mm_mul_ps(c2, _mm_shuffle_ps(v1, v1, 0x8E)));
                inv1 = _mm_add_ps(inv1, _mm_mul_ps(c3, _mm_shuffle_ps(v2, v2, 0x8E)));
                inv1 = _mm_xor_ps(inv1, sign_a);

                fac0 = _mm_shuffle_ps(c0, c0, 0x50);
                fac1 = _mm_shuffle_ps(c3, c3, 0xEE);
                fac2 = _mm_shuffle_ps(c0, c0, 0x05);
                fac3 = _mm_shuffle_ps(c3, c3, 0xAF);
                v0 = _mm_mul_ps(fac0, fac1);
                v0 = _mm_sub_ps(v0, _mm_mul_ps(fac2, fac3));

                fac0 = _mm_shuffle_ps(c0, c0, 0x50);
                fac1 = _mm_shuffle_ps(c2, c2, 0xEE);
                fac2 = _mm_shuffle_ps(c0, c0, 0x05);
                fac3 = _mm_shuffle_ps(c2, c2, 0xAF);
                v1 = _mm_mul_ps(fac0, fac1);
                v1 = _mm_sub_ps(v1, _mm_mul_ps(fac2, fac3));

                fac0 = _mm_shuffle_ps(c0, c0, 0x50);
                fac1 = _mm_shuffle_ps(c1, c1, 0xEE);
                fac2 = _mm_shuffle_ps(c0, c0, 0x05);
                fac3 = _mm_shuffle_ps(c1, c1, 0xAF);
                v2 = _mm_mul_ps(fac0, fac1);
                v2 = _mm_sub_ps(v2, _mm_mul_ps(fac2, fac3));

                let mut inv2 = _mm_mul_ps(c0, _mm_shuffle_ps(v0, v0, 0x8E));
                inv2 = _mm_sub_ps(inv2, _mm_mul_ps(c1, _mm_shuffle_ps(v1, v1, 0x39)));
                inv2 = _mm_add_ps(inv2, _mm_mul_ps(c3, _mm_shuffle_ps(v2, v2, 0x8E)));
                inv2 = _mm_xor_ps(inv2, sign_b);

                let mut inv3 = _mm_mul_ps(c0, _mm_shuffle_ps(v0, v0, 0x39));
                inv3 = _mm_sub_ps(inv3, _mm_mul_ps(c1, _mm_shuffle_ps(v1, v1, 0x8E)));
                inv3 = _mm_add_ps(inv3, _mm_mul_ps(c2, _mm_shuffle_ps(v2, v2, 0x8E)));
                inv3 = _mm_xor_ps(inv3, sign_a);

                let dot0 = _mm_mul_ps(c0, inv0);
                let dot1 = _mm_shuffle_ps(dot0, dot0, 0x39);
                let dot2 = _mm_shuffle_ps(dot0, dot0, 0x4E);
                let dot3 = _mm_shuffle_ps(dot0, dot0, 0x93);
                let det = _mm_add_ps(_mm_add_ps(dot0, dot1), _mm_add_ps(dot2, dot3));

                let mut det_arr = [0.0f32; 4];
                _mm_storeu_ps(det_arr.as_mut_ptr(), det);
                if det_arr[0].abs() < 1e-6 {
                    return Self { m: [[0.0; 4]; 4] };
                }

                let rcp_det = _mm_div_ps(_mm_set1_ps(1.0), det);

                let mut out = Self::default();
                _mm_store_ps(out.m[0].as_mut_ptr(), _mm_mul_ps(inv0, rcp_det));
                _mm_store_ps(out.m[1].as_mut_ptr(), _mm_mul_ps(inv1, rcp_det));
                _mm_store_ps(out.m[2].as_mut_ptr(), _mm_mul_ps(inv2, rcp_det));
                _mm_store_ps(out.m[3].as_mut_ptr(), _mm_mul_ps(inv3, rcp_det));
                return out;
            }
        }

        #[cfg(not(all(target_arch = "x86_64", feature = "simd")))]
        {
            let m = &self.m;
            let a2323 = m[2][2] * m[3][3] - m[2][3] * m[3][2];
            let a1323 = m[2][1] * m[3][3] - m[2][3] * m[3][1];
            let a1223 = m[2][1] * m[3][2] - m[2][2] * m[3][1];
            let a0323 = m[2][0] * m[3][3] - m[2][3] * m[3][0];
            let a0223 = m[2][0] * m[3][2] - m[2][2] * m[3][0];
            let a0123 = m[2][0] * m[3][1] - m[2][1] * m[3][0];

            let mut inv = Self::default();

            inv.m[0][0] = m[1][1] * a2323 - m[1][2] * a1323 + m[1][3] * a1223;
            inv.m[0][1] = -(m[0][1] * a2323 - m[0][2] * a1323 + m[0][3] * a1223);
            inv.m[0][2] = m[0][1] * (m[1][2] * m[3][3] - m[1][3] * m[3][2])
                - m[0][2] * (m[1][1] * m[3][3] - m[1][3] * m[3][1])
                + m[0][3] * (m[1][1] * m[3][2] - m[1][2] * m[3][1]);
            inv.m[0][3] = -(m[0][1] * (m[1][2] * m[2][3] - m[1][3] * m[2][2])
                - m[0][2] * (m[1][1] * m[2][3] - m[1][3] * m[2][1])
                + m[0][3] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]));

            inv.m[1][0] = -(m[1][0] * a2323 - m[1][2] * a0323 + m[1][3] * a0223);
            inv.m[1][1] = m[0][0] * a2323 - m[0][2] * a0323 + m[0][3] * a0223;
            inv.m[1][2] = -(m[0][0] * (m[1][2] * m[3][3] - m[1][3] * m[3][2])
                - m[0][2] * (m[1][0] * m[3][3] - m[1][3] * m[3][0])
                + m[0][3] * (m[1][0] * m[3][2] - m[1][2] * m[3][0]));
            inv.m[1][3] = m[0][0] * (m[1][2] * m[2][3] - m[1][3] * m[2][2])
                - m[0][2] * (m[1][0] * m[2][3] - m[1][3] * m[2][0])
                + m[0][3] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]);

            inv.m[2][0] = m[1][0] * a1323 - m[1][1] * a0323 + m[1][3] * a0123;
            inv.m[2][1] = -(m[0][0] * a1323 - m[0][1] * a0323 + m[0][3] * a0123);
            inv.m[2][2] = m[0][0] * (m[1][1] * m[3][3] - m[1][3] * m[3][1])
                - m[0][1] * (m[1][0] * m[3][3] - m[1][3] * m[3][0])
                + m[0][3] * (m[1][0] * m[3][1] - m[1][1] * m[3][0]);
            inv.m[2][3] = -(m[0][0] * (m[1][1] * m[2][3] - m[1][3] * m[2][1])
                - m[0][1] * (m[1][0] * m[2][3] - m[1][3] * m[2][0])
                + m[0][3] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]));

            inv.m[3][0] = -(m[1][0] * a1223 - m[1][1] * a0223 + m[1][2] * a0123);
            inv.m[3][1] = m[0][0] * a1223 - m[0][1] * a0223 + m[0][2] * a0123;
            inv.m[3][2] = -(m[0][0] * (m[1][1] * m[3][2] - m[1][2] * m[3][1])
                - m[0][1] * (m[1][0] * m[3][2] - m[1][2] * m[3][0])
                + m[0][2] * (m[1][0] * m[3][1] - m[1][1] * m[3][0]));
            inv.m[3][3] = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
                - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
                + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);

            let det = m[0][0] * inv.m[0][0]
                + m[0][1] * inv.m[1][0]
                + m[0][2] * inv.m[2][0]
                + m[0][3] * inv.m[3][0];

            if det.abs() < 1e-6 {
                return Self { m: [[0.0; 4]; 4] };
            }

            let inv_det = 1.0 / det;
            for i in 0..4 {
                for j in 0..4 {
                    inv.m[i][j] *= inv_det;
                }
            }
            inv
        }
    }

    /// Retrieve a column by index (0-3).
    ///
    /// # Panics
    ///
    /// Panics if index is out of bounds.
    #[must_use]
    #[inline]
    pub const fn col(&self, index: usize) -> Vec4 {
        Vec4::new(
            self.m[0][index],
            self.m[1][index],
            self.m[2][index],
            self.m[3][index],
        )
    }

    /// Return the given row as a [`Vec4`].
    ///
    /// Row 3 is the translation row in this library's row-vector convention.
    ///
    /// # Panics
    ///
    /// Panics if `index >= 4`.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::{Mat4, Vec4};
    ///
    /// let m = Mat4::identity();
    /// assert_eq!(m.row(0), Vec4::new(1.0, 0.0, 0.0, 0.0));
    /// assert_eq!(m.row(3), Vec4::new(0.0, 0.0, 0.0, 1.0));
    /// ```
    #[must_use]
    #[inline]
    pub fn row(&self, index: usize) -> Vec4 {
        Vec4::new(
            self.m[index][0],
            self.m[index][1],
            self.m[index][2],
            self.m[index][3],
        )
    }
}

impl Default for Mat4 {
    fn default() -> Self {
        Self::identity()
    }
}

impl Mul for Mat4 {
    type Output = Self;

    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    #[inline]
    fn mul(self, other: Self) -> Self {
        unsafe {
            use std::arch::x86_64::{
                _mm_add_ps, _mm_load_ps, _mm_mul_ps, _mm_shuffle_ps, _mm_store_ps,
            };
            let mut result = Self { m: [[0.0; 4]; 4] };

            // Load rows of B
            let b0 = _mm_load_ps(other.m[0].as_ptr());
            let b1 = _mm_load_ps(other.m[1].as_ptr());
            let b2 = _mm_load_ps(other.m[2].as_ptr());
            let b3 = _mm_load_ps(other.m[3].as_ptr());

            for i in 0..4 {
                // Load row i of A
                let row_a = _mm_load_ps(self.m[i].as_ptr());

                // Broadcast A[i][0]
                let a0 = _mm_shuffle_ps(row_a, row_a, 0x00);
                let mut row_res = _mm_mul_ps(a0, b0);

                // Broadcast A[i][1]
                let a1 = _mm_shuffle_ps(row_a, row_a, 0x55);
                row_res = _mm_add_ps(row_res, _mm_mul_ps(a1, b1));

                // Broadcast A[i][2]
                let a2 = _mm_shuffle_ps(row_a, row_a, 0xAA);
                row_res = _mm_add_ps(row_res, _mm_mul_ps(a2, b2));

                // Broadcast A[i][3]
                let a3 = _mm_shuffle_ps(row_a, row_a, 0xFF);
                row_res = _mm_add_ps(row_res, _mm_mul_ps(a3, b3));

                _mm_store_ps(result.m[i].as_mut_ptr(), row_res);
            }
            result
        }
    }

    #[cfg(not(all(target_arch = "x86_64", feature = "simd")))]
    fn mul(self, other: Self) -> Self {
        let mut result = Self { m: [[0.0; 4]; 4] };
        for i in 0..4 {
            for k in 0..4 {
                let s = self.m[i][k];
                for j in 0..4 {
                    result.m[i][j] += s * other.m[k][j];
                }
            }
        }
        result
    }
}

/// A 3D point that has been projected into 2D screen coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(missing_docs)]
pub struct ScreenPoint {
    pub x: i32,
    pub y: i32,
    pub z: f32,
    /// Reciprocal of the Homogeneous W coordinate ($1/w$).
    ///
    /// This value is critical for perspective-correct texture mapping and attribute interpolation.
    /// By storing $1/w$, the rasterizer can interpolate attributes in screen space linearly
    /// (e.g., $u/w$, $v/w$) and then recover the true perspective-correct value per pixel
    /// by dividing by the interpolated $1/w$.
    ///
    /// Storing this avoids recomputing the division during triangle setup,
    /// saving ~10-20 CPU cycles per vertex.
    pub inv_w: f32,
}

/// Project a 3D point to screen coordinates using pre-calculated half-dimensions.
///
/// This avoids repetitive integer-to-float conversions and divisions.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{project_to_screen_optimized, Vec3};
///
/// let point = Vec3::new(1.0, 1.0, 5.0);
/// let w = 5.0; // Assume we already have w from projection
/// let half_width = 400.0;
/// let half_height = 300.0;
///
/// let screen_point = project_to_screen_optimized(point, w, half_width, half_height);
///
/// // NDC x = 1/5 = 0.2
/// // Screen x = (0.2 + 1.0) * 400 = 480
/// assert_eq!(screen_point.x, 480);
/// ```
#[must_use]
#[inline]
pub fn project_to_screen_optimized(
    v: Vec3,
    w: f32,
    half_width: f32,
    half_height: f32,
) -> ScreenPoint {
    const MAX_VAL: f32 = 2_147_483_520.0;
    const MIN_VAL: f32 = -2_147_483_520.0;

    // Perspective divide
    let inv_w = if w.abs() > 0.0001 { 1.0 / w } else { 1.0 };
    let ndc_x = v.x * inv_w;
    let ndc_y = v.y * inv_w;
    let depth = v.z * inv_w;

    // NDC to screen coordinates
    // Clamp to [i32::MIN + 1, i32::MAX] to avoid integer overflow when negating i32::MIN.
    // We clamp the float value BEFORE casting to i32 to avoid Undefined Behavior with NaN/Inf.
    // 2147483520.0 is the largest f32 strictly less than i32::MAX + 1 that is exactly representable.

    let screen_x_f = (ndc_x + 1.0) * half_width;
    let screen_y_f = (1.0 - ndc_y) * half_height; // Flip Y

    // Optimization: Branchless clamp to avoid stalls.
    // If NaN, max(MIN) returns MIN (because max propagates non-NaN).
    // Then min(MIN, MAX) returns MIN.
    // Result is always in [MIN, MAX] (or MIN if NaN).
    // Note: f32::clamp() returns NaN for NaN inputs, which makes casting to i32 undefined/zero.
    // We strictly want MIN_VAL behavior for NaNs here.
    #[allow(clippy::manual_clamp)]
    let screen_x = screen_x_f.max(MIN_VAL).min(MAX_VAL) as i32;
    #[allow(clippy::manual_clamp)]
    let screen_y = screen_y_f.max(MIN_VAL).min(MAX_VAL) as i32;

    ScreenPoint {
        x: screen_x,
        y: screen_y,
        z: depth,
        inv_w,
    }
}

/// Project 3 vertices to screen coordinates in parallel.
#[cfg(target_arch = "x86_64")]
#[must_use]
#[inline]
pub fn project_triangle_to_screen(
    v0: Vec3,
    w0: f32,
    v1: Vec3,
    w1: f32,
    v2: Vec3,
    w2: f32,
    half_width: f32,
    half_height: f32,
) -> (ScreenPoint, ScreenPoint, ScreenPoint) {
    unsafe {
        use std::arch::x86_64::{
            __m128i, _mm_add_ps, _mm_and_ps, _mm_andnot_ps, _mm_cmpgt_ps, _mm_cvttps_epi32,
            _mm_max_ps, _mm_min_ps, _mm_mul_ps, _mm_or_ps, _mm_rcp_ps, _mm_set_ps, _mm_set1_ps,
            _mm_storeu_ps, _mm_storeu_si128, _mm_sub_ps,
        };

        // Load data into SIMD registers
        // Layout: [v2, v1, v0, pad]
        // Note: _mm_set_ps(d, c, b, a) -> [a, b, c, d]
        let x_vec = _mm_set_ps(0.0, v2.x, v1.x, v0.x);
        let y_vec = _mm_set_ps(0.0, v2.y, v1.y, v0.y);
        let z_vec = _mm_set_ps(0.0, v2.z, v1.z, v0.z);
        // Pad w with 1.0 to avoid division by zero in the unused lane
        let w_vec = _mm_set_ps(1.0, w2, w1, w0);

        let one = _mm_set1_ps(1.0);
        let min_val = _mm_set1_ps(0.0001);

        // Check w > epsilon (vectorized)
        // If w.abs() > 0.0001, use w. Otherwise use 1.0.
        // abs_w = w & !(-0.0)
        let abs_w = _mm_andnot_ps(_mm_set1_ps(-0.0), w_vec);
        // _mm_cmpgt_ps is standard SSE
        let mask = _mm_cmpgt_ps(abs_w, min_val);

        // safe_w = blend(1.0, w, mask)
        // Use logical ops for SSE2 compatibility: (w & mask) | (1.0 & ~mask)
        let safe_w = _mm_or_ps(_mm_and_ps(w_vec, mask), _mm_andnot_ps(mask, one));

        // Clamp to avoid Inf * 0 = NaN in Newton-Raphson
        let max_w = _mm_set1_ps(1e30);
        let safe_w = _mm_min_ps(safe_w, max_w);

        // Use fast approximate reciprocal with one Newton-Raphson iteration
        // This avoids the high-latency, unpipelined division instruction,
        // freeing up the divider unit for subsequent gradient setup.
        // y0 = rcp(x)
        let rcp = _mm_rcp_ps(safe_w);
        // y1 = y0 * (2 - x * y0)
        let two = _mm_set1_ps(2.0);
        let inv_w = _mm_mul_ps(rcp, _mm_sub_ps(two, _mm_mul_ps(safe_w, rcp)));

        let ndc_x = _mm_mul_ps(x_vec, inv_w);
        let ndc_y = _mm_mul_ps(y_vec, inv_w);
        let depth = _mm_mul_ps(z_vec, inv_w);

        let hw = _mm_set1_ps(half_width);
        let hh = _mm_set1_ps(half_height);

        // Clamp values to valid i32 range to avoid undefined behavior/overflow in cvttps
        // 2147483520.0 is the largest float strictly less than i32::MAX + 1 that is representable and fits in i32
        let max_val_i32 = _mm_set1_ps(2_147_483_520.0);
        let min_val_i32 = _mm_set1_ps(-2_147_483_520.0);

        // screen_x = (ndc_x + 1.0) * half_width
        let sx = _mm_mul_ps(_mm_add_ps(ndc_x, one), hw);
        // screen_y = (1.0 - ndc_y) * half_height
        let sy = _mm_mul_ps(_mm_sub_ps(one, ndc_y), hh);

        // Clamp before conversion
        let sx = _mm_min_ps(_mm_max_ps(sx, min_val_i32), max_val_i32);
        let sy = _mm_min_ps(_mm_max_ps(sy, min_val_i32), max_val_i32);

        // Convert to int (truncation)
        let sx_i = _mm_cvttps_epi32(sx);
        let sy_i = _mm_cvttps_epi32(sy);

        // Store results to stack array
        let mut x_arr = [0i32; 4];
        let mut y_arr = [0i32; 4];
        let mut z_arr = [0f32; 4];
        let mut iw_arr = [0f32; 4];

        #[allow(clippy::cast_ptr_alignment)]
        {
            _mm_storeu_si128(x_arr.as_mut_ptr().cast::<__m128i>(), sx_i);
            _mm_storeu_si128(y_arr.as_mut_ptr().cast::<__m128i>(), sy_i);
        }
        _mm_storeu_ps(z_arr.as_mut_ptr(), depth);
        _mm_storeu_ps(iw_arr.as_mut_ptr(), inv_w);

        (
            ScreenPoint {
                x: x_arr[0],
                y: y_arr[0],
                z: z_arr[0],
                inv_w: iw_arr[0],
            },
            ScreenPoint {
                x: x_arr[1],
                y: y_arr[1],
                z: z_arr[1],
                inv_w: iw_arr[1],
            },
            ScreenPoint {
                x: x_arr[2],
                y: y_arr[2],
                z: z_arr[2],
                inv_w: iw_arr[2],
            },
        )
    }
}

/// Project 4 vertices to screen coordinates in parallel.
/// Perfect for quads.
#[cfg(target_arch = "x86_64")]
#[must_use]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn project_quad_to_screen(
    v0: Vec3,
    w0: f32,
    v1: Vec3,
    w1: f32,
    v2: Vec3,
    w2: f32,
    v3: Vec3,
    w3: f32,
    half_width: f32,
    half_height: f32,
) -> (ScreenPoint, ScreenPoint, ScreenPoint, ScreenPoint) {
    unsafe {
        use std::arch::x86_64::{
            __m128i, _mm_add_ps, _mm_and_ps, _mm_andnot_ps, _mm_cmpgt_ps, _mm_cvttps_epi32,
            _mm_max_ps, _mm_min_ps, _mm_mul_ps, _mm_or_ps, _mm_rcp_ps, _mm_set_ps, _mm_set1_ps,
            _mm_storeu_ps, _mm_storeu_si128, _mm_sub_ps,
        };

        // Load data into SIMD registers
        // Layout: [v3, v2, v1, v0]
        let x_vec = _mm_set_ps(v3.x, v2.x, v1.x, v0.x);
        let y_vec = _mm_set_ps(v3.y, v2.y, v1.y, v0.y);
        let z_vec = _mm_set_ps(v3.z, v2.z, v1.z, v0.z);
        let w_vec = _mm_set_ps(w3, w2, w1, w0);

        let one = _mm_set1_ps(1.0);
        let min_val = _mm_set1_ps(0.0001);

        // Check w > epsilon (vectorized)
        let abs_w = _mm_andnot_ps(_mm_set1_ps(-0.0), w_vec);
        let mask = _mm_cmpgt_ps(abs_w, min_val);
        let safe_w = _mm_or_ps(_mm_and_ps(w_vec, mask), _mm_andnot_ps(mask, one));

        // Clamp to avoid Inf * 0 = NaN in Newton-Raphson
        let max_w = _mm_set1_ps(1e30);
        let safe_w = _mm_min_ps(safe_w, max_w);

        // Fast reciprocal
        let rcp = _mm_rcp_ps(safe_w);
        let two = _mm_set1_ps(2.0);
        let inv_w = _mm_mul_ps(rcp, _mm_sub_ps(two, _mm_mul_ps(safe_w, rcp)));

        let ndc_x = _mm_mul_ps(x_vec, inv_w);
        let ndc_y = _mm_mul_ps(y_vec, inv_w);
        let depth = _mm_mul_ps(z_vec, inv_w);

        let hw = _mm_set1_ps(half_width);
        let hh = _mm_set1_ps(half_height);

        let max_val_i32 = _mm_set1_ps(2_147_483_520.0);
        let min_val_i32 = _mm_set1_ps(-2_147_483_520.0);

        let sx = _mm_mul_ps(_mm_add_ps(ndc_x, one), hw);
        let sy = _mm_mul_ps(_mm_sub_ps(one, ndc_y), hh);

        let sx = _mm_min_ps(_mm_max_ps(sx, min_val_i32), max_val_i32);
        let sy = _mm_min_ps(_mm_max_ps(sy, min_val_i32), max_val_i32);

        let sx_i = _mm_cvttps_epi32(sx);
        let sy_i = _mm_cvttps_epi32(sy);

        let mut x_arr = [0i32; 4];
        let mut y_arr = [0i32; 4];
        let mut z_arr = [0f32; 4];
        let mut iw_arr = [0f32; 4];

        #[allow(clippy::cast_ptr_alignment)]
        {
            _mm_storeu_si128(x_arr.as_mut_ptr().cast::<__m128i>(), sx_i);
            _mm_storeu_si128(y_arr.as_mut_ptr().cast::<__m128i>(), sy_i);
        }
        _mm_storeu_ps(z_arr.as_mut_ptr(), depth);
        _mm_storeu_ps(iw_arr.as_mut_ptr(), inv_w);

        (
            ScreenPoint {
                x: x_arr[0],
                y: y_arr[0],
                z: z_arr[0],
                inv_w: iw_arr[0],
            },
            ScreenPoint {
                x: x_arr[1],
                y: y_arr[1],
                z: z_arr[1],
                inv_w: iw_arr[1],
            },
            ScreenPoint {
                x: x_arr[2],
                y: y_arr[2],
                z: z_arr[2],
                inv_w: iw_arr[2],
            },
            ScreenPoint {
                x: x_arr[3],
                y: y_arr[3],
                z: z_arr[3],
                inv_w: iw_arr[3],
            },
        )
    }
}

/// Project 4 vertices to screen coordinates (Scalar Fallback).
#[cfg(not(target_arch = "x86_64"))]
#[must_use]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn project_quad_to_screen(
    v0: Vec3,
    w0: f32,
    v1: Vec3,
    w1: f32,
    v2: Vec3,
    w2: f32,
    v3: Vec3,
    w3: f32,
    half_width: f32,
    half_height: f32,
) -> (ScreenPoint, ScreenPoint, ScreenPoint, ScreenPoint) {
    (
        project_to_screen_optimized(v0, w0, half_width, half_height),
        project_to_screen_optimized(v1, w1, half_width, half_height),
        project_to_screen_optimized(v2, w2, half_width, half_height),
        project_to_screen_optimized(v3, w3, half_width, half_height),
    )
}

/// Project 3 vertices to screen coordinates (Scalar Fallback).
#[cfg(not(target_arch = "x86_64"))]
#[must_use]
#[inline]
pub fn project_triangle_to_screen(
    v0: Vec3,
    w0: f32,
    v1: Vec3,
    w1: f32,
    v2: Vec3,
    w2: f32,
    half_width: f32,
    half_height: f32,
) -> (ScreenPoint, ScreenPoint, ScreenPoint) {
    (
        project_to_screen_optimized(v0, w0, half_width, half_height),
        project_to_screen_optimized(v1, w1, half_width, half_height),
        project_to_screen_optimized(v2, w2, half_width, half_height),
    )
}

/// Project a 3D point to screen coordinates
#[must_use]
#[inline]
pub fn project_to_screen(v: Vec3, w: f32, width: u32, height: u32) -> ScreenPoint {
    let half_width = width as f32 * 0.5;
    let half_height = height as f32 * 0.5;
    project_to_screen_optimized(v, w, half_width, half_height)
}

// ── Spline / Curve Interpolation ─────────────────────────────────────────────

/// Evaluate a quadratic Bézier curve at parameter `t ∈ [0, 1]`.
///
/// `p0` is the start, `p1` is the control point, `p2` is the end.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{bezier_quadratic, Vec3};
///
/// let p = bezier_quadratic(Vec3::ZERO, Vec3::new(0.5, 1.0, 0.0), Vec3::ONE, 0.5);
/// // Midpoint of a quadratic curve through (0,0,0)→(0.5,1,0)→(1,1,1)
/// assert!((p.x - 0.5).abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn bezier_quadratic(p0: Vec3, p1: Vec3, p2: Vec3, t: f32) -> Vec3 {
    let u = 1.0 - t;
    p0 * (u * u) + p1 * (2.0 * u * t) + p2 * (t * t)
}

/// Evaluate a cubic Bézier curve at parameter `t ∈ [0, 1]`.
///
/// `p0`/`p3` are endpoints; `p1`/`p2` are control points.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{bezier_cubic, Vec3};
///
/// let p = bezier_cubic(Vec3::ZERO, Vec3::X, Vec3::new(2.0, 1.0, 0.0), Vec3::new(3.0, 0.0, 0.0), 0.0);
/// assert_eq!(p, Vec3::ZERO);
/// let p1 = bezier_cubic(Vec3::ZERO, Vec3::X, Vec3::new(2.0, 1.0, 0.0), Vec3::new(3.0, 0.0, 0.0), 1.0);
/// assert!((p1.x - 3.0).abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn bezier_cubic(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let u = 1.0 - t;
    let u2 = u * u;
    let t2 = t * t;
    p0 * (u2 * u) + p1 * (3.0 * u2 * t) + p2 * (3.0 * u * t2) + p3 * (t2 * t)
}

/// Tangent (derivative) of a cubic Bézier at parameter `t`.
///
/// Returns an **unnormalized** tangent vector.
#[must_use]
#[inline]
pub fn bezier_cubic_tangent(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let u = 1.0 - t;
    (p1 - p0) * (3.0 * u * u) + (p2 - p1) * (6.0 * u * t) + (p3 - p2) * (3.0 * t * t)
}

/// Evaluate a Catmull-Rom spline segment at `t ∈ [0, 1]`.
///
/// `p0`/`p3` are the two outer control points; `p1`/`p2` are the segment endpoints.
/// The curve passes through `p1` at `t=0` and `p2` at `t=1`.
///
/// Uses α=0.5 (centripetal Catmull-Rom) tension.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{catmull_rom, Vec3};
///
/// let p = catmull_rom(Vec3::new(-1.0, 0.0, 0.0), Vec3::ZERO,
///                     Vec3::ONE, Vec3::new(2.0, 1.0, 0.0), 0.0);
/// assert!(p.x.abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn catmull_rom(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;
    // Barry-Goldman formulation with tension 0.5
    p0 * (-0.5 * t3 + t2 - 0.5 * t)
        + p1 * (1.5 * t3 - 2.5 * t2 + 1.0)
        + p2 * (-1.5 * t3 + 2.0 * t2 + 0.5 * t)
        + p3 * (0.5 * t3 - 0.5 * t2)
}

/// Evaluate a cubic Hermite spline between `p0` and `p1` at `t ∈ [0, 1]`.
///
/// `m0` and `m1` are the tangents at the start and end points respectively.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{hermite, Vec3};
///
/// // Zero tangents → linear interpolation (H basis reduces to lerp)
/// // Actually cubic hermite with m=0 gives same endpoints but isn't linear for interior.
/// let p = hermite(Vec3::ZERO, Vec3::ZERO, Vec3::ONE, Vec3::ZERO, 0.0);
/// assert_eq!(p, Vec3::ZERO);
/// let p1 = hermite(Vec3::ZERO, Vec3::ZERO, Vec3::ONE, Vec3::ZERO, 1.0);
/// assert_eq!(p1, Vec3::ONE);
/// ```
#[must_use]
#[inline]
pub fn hermite(p0: Vec3, m0: Vec3, p1: Vec3, m1: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;
    p0 * h00 + m0 * h10 + p1 * h01 + m1 * h11
}

/// Kochanek-Bartels (TCB) spline segment at `t ∈ [0, 1]`.
///
/// A generalization of Catmull-Rom with three parameters per control point:
/// - `tension` `t_val` in [-1, 1]: 1 = tight (no overshoot), -1 = loose.
/// - `continuity` `c_val` in [-1, 1]: 0 = smooth, ±1 = sharp corner.
/// - `bias` `b_val` in [-1, 1]: 0 = symmetric, 1 = pre-weight, -1 = post-weight.
///
/// With all three at 0 this reduces to Catmull-Rom.
/// The curve passes through `p1` at `t=0` and `p2` at `t=1`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{kochanek_bartels, Vec3};
///
/// // All-zero parameters → same as Catmull-Rom; passes through p1 at t=0
/// let p = kochanek_bartels(
///     Vec3::new(-1.0, 0.0, 0.0), Vec3::ZERO, Vec3::ONE, Vec3::new(2.0, 1.0, 0.0),
///     0.0, 0.0, 0.0, 0.0,
/// );
/// assert!(p.x.abs() < 1e-5, "should pass through p1, got x={}", p.x);
/// ```
#[must_use]
#[inline]
pub fn kochanek_bartels(
    p0: Vec3,
    p1: Vec3,
    p2: Vec3,
    p3: Vec3,
    t: f32,
    t_val: f32,
    c_val: f32,
    b_val: f32,
) -> Vec3 {
    // Kochanek-Bartels tangents (incoming/outgoing at p1 and p2)
    let s1 = (1.0 - t_val) * 0.5;
    let d1 = (p1 - p0) * (s1 * (1.0 + c_val) * (1.0 + b_val))
        + (p2 - p1) * (s1 * (1.0 - c_val) * (1.0 - b_val));
    let d2 = (p2 - p1) * (s1 * (1.0 + c_val) * (1.0 - b_val))
        + (p3 - p2) * (s1 * (1.0 - c_val) * (1.0 + b_val));
    hermite(p1, d1, p2, d2, t)
}

/// Subdivide a cubic Bézier at `t` using de Casteljau's algorithm.
///
/// Returns `(left, right)` where `left` and `right` are each four control
/// points of a cubic Bézier that together cover the same arc as the original.
/// `left` covers `[0, t]` and `right` covers `[t, 1]`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{bezier_cubic, bezier_cubic_split, Vec3};
///
/// let (p0, p1, p2, p3) = (Vec3::ZERO, Vec3::new(1.0, 2.0, 0.0),
///                         Vec3::new(2.0, 2.0, 0.0), Vec3::new(3.0, 0.0, 0.0));
/// let (left, right) = bezier_cubic_split(p0, p1, p2, p3, 0.5);
/// // left[3] should equal right[0] == point on original curve at t=0.5
/// let mid = bezier_cubic(p0, p1, p2, p3, 0.5);
/// assert!((left[3].x - mid.x).abs() < 1e-5);
/// assert!((right[0].x - mid.x).abs() < 1e-5);
/// ```
#[must_use]
pub fn bezier_cubic_split(
    p0: Vec3,
    p1: Vec3,
    p2: Vec3,
    p3: Vec3,
    t: f32,
) -> ([Vec3; 4], [Vec3; 4]) {
    // de Casteljau: one level per step
    let q0 = p0.lerp(p1, t);
    let q1 = p1.lerp(p2, t);
    let q2 = p2.lerp(p3, t);
    let r0 = q0.lerp(q1, t);
    let r1 = q1.lerp(q2, t);
    let s = r0.lerp(r1, t);
    ([p0, q0, r0, s], [s, r1, q2, p3])
}

/// Evaluate a uniform cubic B-spline at parameter `t ∈ [0, 1]`.
///
/// The B-spline basis produces a curve that passes **near** (not through)
/// `p1` and `p2` — it has C² continuity unlike Catmull-Rom (C¹).
///
/// `p0`, `p1`, `p2`, `p3` are four consecutive control points.
/// Result is inside the convex hull of the four points.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{bspline_eval, Vec3};
///
/// let p0 = Vec3::new(0.0, 0.0, 0.0);
/// let p1 = Vec3::new(1.0, 2.0, 0.0);
/// let p2 = Vec3::new(2.0, 2.0, 0.0);
/// let p3 = Vec3::new(3.0, 0.0, 0.0);
///
/// // Result must lie within the convex hull
/// let p = bspline_eval(p0, p1, p2, p3, 0.5);
/// assert!(p.x > 0.0 && p.x < 3.0);
/// assert!(p.y > 0.0 && p.y < 3.0);
/// ```
#[must_use]
#[inline]
pub fn bspline_eval(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;
    // Uniform B-spline basis (divide by 6)
    let b0 = (1.0 - 3.0 * t + 3.0 * t2 - t3) / 6.0;
    let b1 = (4.0 - 6.0 * t2 + 3.0 * t3) / 6.0;
    let b2 = (1.0 + 3.0 * t + 3.0 * t2 - 3.0 * t3) / 6.0;
    let b3 = t3 / 6.0;
    p0 * b0 + p1 * b1 + p2 * b2 + p3 * b3
}

/// Approximate arc-length of a cubic Bézier using 5-point Gaussian quadrature.
///
/// More accurate than the common chord-sum approximation.  The relative error
/// for typical curves (aspect ratio ≤ 4) is under 0.01%.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{bezier_cubic_arc_length, Vec3};
///
/// // Straight line from (0,0,0) to (1,0,0) — control points collinear
/// let p0 = Vec3::new(0.0, 0.0, 0.0);
/// let p1 = Vec3::new(1.0/3.0, 0.0, 0.0);
/// let p2 = Vec3::new(2.0/3.0, 0.0, 0.0);
/// let p3 = Vec3::new(1.0, 0.0, 0.0);
/// let len = bezier_cubic_arc_length(p0, p1, p2, p3);
/// assert!((len - 1.0).abs() < 1e-4, "straight line length should be 1.0, got {len}");
/// ```
#[must_use]
pub fn bezier_cubic_arc_length(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3) -> f32 {
    // 5-point Gauss-Legendre nodes and weights on [0, 1]
    const NODES: [f32; 5] = [
        0.046_910_077,
        0.230_765_346,
        0.5,
        0.769_234_654,
        0.953_089_923,
    ];
    const WEIGHTS: [f32; 5] = [
        0.118_463_443,
        0.239_314_335,
        0.284_444_444,
        0.239_314_335,
        0.118_463_443,
    ];
    let mut length = 0.0_f32;
    for (i, &t) in NODES.iter().enumerate() {
        // Derivative of cubic Bézier: B'(t) = 3[(p1-p0)(1-t)² + 2(p2-p1)t(1-t) + (p3-p2)t²]
        let inv_t = 1.0 - t;
        let d = (p1 - p0) * (3.0 * inv_t * inv_t)
            + (p2 - p1) * (6.0 * inv_t * t)
            + (p3 - p2) * (3.0 * t * t);
        length += WEIGHTS[i] * d.length();
    }
    length
}

/// Find the real roots of a quadratic `ax² + bx + c = 0`.
///
/// Returns roots sorted in ascending order in the `[f32; 2]` array and the
/// count of real roots in the `usize`.  Use `roots[..count]` to iterate.
///
/// # Examples
///
/// ```
/// use abrash_core::math::quadratic_solve;
///
/// // x² - 5x + 6 = 0  → roots 2 and 3
/// let (roots, n) = quadratic_solve(1.0, -5.0, 6.0);
/// assert_eq!(n, 2);
/// assert!((roots[0] - 2.0).abs() < 1e-5);
/// assert!((roots[1] - 3.0).abs() < 1e-5);
///
/// // x² + 1 = 0 → no real roots
/// let (_, n) = quadratic_solve(1.0, 0.0, 1.0);
/// assert_eq!(n, 0);
/// ```
#[must_use]
pub fn quadratic_solve(a: f32, b: f32, c: f32) -> ([f32; 2], usize) {
    if a.abs() < 1e-10 {
        if b.abs() > 1e-10 {
            return ([-c / b, 0.0], 1);
        }
        return ([0.0; 2], 0);
    }
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 {
        return ([0.0; 2], 0);
    }
    if disc < 1e-10 {
        return ([-b / (2.0 * a), 0.0], 1);
    }
    let sq = disc.sqrt();
    let inv2a = 1.0 / (2.0 * a);
    let mut r0 = (-b - sq) * inv2a;
    let mut r1 = (-b + sq) * inv2a;
    if r0 > r1 {
        std::mem::swap(&mut r0, &mut r1);
    }
    ([r0, r1], 2)
}

/// Find real roots of a cubic `ax³ + bx² + cx + d = 0` (Cardano / trig method).
///
/// Returns roots sorted in ascending order in the `[f32; 3]` array and the
/// count of real roots.  Use `roots[..count]` to iterate.
///
/// # Examples
///
/// ```
/// use abrash_core::math::cubic_solve;
///
/// // x³ - 6x² + 11x - 6 = 0  → roots 1, 2, 3
/// let (roots, n) = cubic_solve(1.0, -6.0, 11.0, -6.0);
/// assert_eq!(n, 3);
/// assert!((roots[0] - 1.0).abs() < 1e-4);
/// assert!((roots[1] - 2.0).abs() < 1e-4);
/// assert!((roots[2] - 3.0).abs() < 1e-4);
/// ```
#[must_use]
pub fn cubic_solve(a: f32, b: f32, c: f32, d: f32) -> ([f32; 3], usize) {
    if a.abs() < 1e-10 {
        let (qr, n) = quadratic_solve(b, c, d);
        return ([qr[0], qr[1], 0.0], n);
    }
    // Reduce to depressed cubic t³ + pt + q = 0 (substitute x = t - b/3a)
    let inv_a = 1.0 / a;
    let b = b * inv_a;
    let c = c * inv_a;
    let d = d * inv_a;
    let p = c - b * b / 3.0;
    let q = 2.0 * b * b * b / 27.0 - b * c / 3.0 + d;
    let disc = q * q / 4.0 + p * p * p / 27.0;
    let shift = -b / 3.0;
    if disc > 1e-10 {
        // One real root (Cardano)
        let sq = disc.sqrt();
        let u = (-q / 2.0 + sq).cbrt();
        let v = (-q / 2.0 - sq).cbrt();
        ([u + v + shift, 0.0, 0.0], 1)
    } else if disc > -1e-10 {
        // Two distinct real roots (one double root)
        let u = (-q / 2.0).cbrt();
        let mut r = [2.0 * u + shift, -u + shift];
        r.sort_by(f32::total_cmp);
        // Deduplicate if nearly equal
        if (r[0] - r[1]).abs() < 1e-7 {
            ([r[0], 0.0, 0.0], 1)
        } else {
            ([r[0], r[1], 0.0], 2)
        }
    } else {
        // Three distinct real roots (trigonometric method)
        let m = 2.0 * (-p / 3.0).sqrt();
        let theta = (3.0 * q / (p * m)).acos() / 3.0;
        let step = std::f32::consts::TAU / 3.0; // 2π/3
        let mut r = [
            m * theta.cos() + shift,
            m * (theta - step).cos() + shift,
            m * (theta - 2.0 * step).cos() + shift,
        ];
        r.sort_by(f32::total_cmp);
        (r, 3)
    }
}

/// Build an orthonormal tangent-bitangent frame from a surface normal.
///
/// Uses the Duff et al. 2017 ("Building an Orthonormal Basis, Revisited")
/// revision of Frisvad's method.  Branchless, numerically stable for all
/// normals including those near `(0, -1, 0)`.
///
/// Returns `(tangent, bitangent)` such that `(tangent, bitangent, normal)` form
/// a right-handed orthonormal basis.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{basis_from_normal, Vec3};
///
/// let n = Vec3::new(0.0, 1.0, 0.0);
/// let (t, b) = basis_from_normal(n);
/// assert!(t.dot(n).abs() < 1e-5, "t must be perp to n");
/// assert!(b.dot(n).abs() < 1e-5, "b must be perp to n");
/// assert!(t.dot(b).abs() < 1e-5, "t and b must be perp");
/// assert!((t.length() - 1.0).abs() < 1e-5, "t must be unit");
/// assert!((b.length() - 1.0).abs() < 1e-5, "b must be unit");
/// ```
#[must_use]
pub fn basis_from_normal(n: Vec3) -> (Vec3, Vec3) {
    // Duff et al. 2017 — sign(n.z) trick avoids the n.z ≈ -1 singularity
    let sign = n.z.signum(); // ±1, never 0 (signum(0) = 1 in Rust)
    let a = -1.0 / (sign + n.z);
    let b = n.x * n.y * a;
    let tangent = Vec3::new(1.0 + sign * n.x * n.x * a, sign * b, -sign * n.x);
    let bitangent = Vec3::new(b, sign + n.y * n.y * a, -n.y);
    (tangent, bitangent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reflect() {
        let v = Vec3::new(1.0, -1.0, 0.0);
        let n = Vec3::new(0.0, 1.0, 0.0);
        let r = v.reflect(n);
        assert!((r.x - 1.0).abs() < f32::EPSILON);
        assert!((r.y - 1.0).abs() < f32::EPSILON);
        assert!((r.z - 0.0).abs() < f32::EPSILON);

        let v2 = Vec3::new(1.0, 2.0, 3.0);
        let n2 = Vec3::new(0.0, 1.0, 0.0);
        let r2 = v2.reflect(n2);
        assert!((r2.x - 1.0).abs() < f32::EPSILON);
        assert!((r2.y - -2.0).abs() < f32::EPSILON);
        assert!((r2.z - 3.0).abs() < f32::EPSILON);
    }

    // ── Vec2 component ops ────────────────────────────────────────────────────

    #[test]
    fn vec2_abs() {
        let v = Vec2::new(-2.0, 3.0).abs();
        assert_eq!(v, Vec2::new(2.0, 3.0));
    }

    #[test]
    fn vec2_sign() {
        let v = Vec2::new(-5.0, 0.0).sign();
        assert_eq!(v.x, -1.0);
        // f32::signum(0.0) = 1.0 in Rust
        assert!((v.y - 0.0_f32.signum()).abs() < 1e-6);
    }

    #[test]
    fn vec2_floor_ceil_round() {
        let v = Vec2::new(1.6, -1.6);
        assert_eq!(v.floor(), Vec2::new(1.0, -2.0));
        assert_eq!(v.ceil(), Vec2::new(2.0, -1.0));
        assert_eq!(v.round(), Vec2::new(2.0, -2.0));
    }

    #[test]
    fn vec2_fract() {
        let f = Vec2::new(2.75, -1.25).fract();
        assert!((f.x - 0.75).abs() < 1e-6);
    }

    #[test]
    fn vec2_step() {
        let edge = Vec2::new(1.0, 2.0);
        let v = Vec2::new(0.5, 3.0);
        let s = v.step(edge);
        assert_eq!(s, Vec2::new(0.0, 1.0));
    }

    #[test]
    fn vec2_reflect() {
        let v = Vec2::new(1.0, -1.0);
        let n = Vec2::new(0.0, 1.0);
        let r = v.reflect(n);
        assert!((r.x - 1.0).abs() < 1e-6);
        assert!((r.y - 1.0).abs() < 1e-6);
    }

    // ── Vec3 component ops ────────────────────────────────────────────────────

    #[test]
    fn vec3_sign() {
        let v = Vec3::new(-3.0, 0.5, 0.0).sign();
        assert_eq!(v.x, -1.0);
        assert_eq!(v.y, 1.0);
    }

    #[test]
    fn vec3_floor_ceil_round_fract() {
        let v = Vec3::new(1.7, -1.3, 2.5);
        assert_eq!(v.floor(), Vec3::new(1.0, -2.0, 2.0));
        assert_eq!(v.ceil(), Vec3::new(2.0, -1.0, 3.0));
        assert!((v.fract().x - 0.7).abs() < 1e-5);
    }

    #[test]
    fn vec3_step() {
        let e = Vec3::new(1.0, 2.0, 3.0);
        let v = Vec3::new(0.5, 2.0, 5.0);
        let s = v.step(e);
        assert_eq!(s, Vec3::new(0.0, 1.0, 1.0));
    }

    // ── Vec4 component ops ────────────────────────────────────────────────────

    #[test]
    fn vec4_abs_sign() {
        let v = Vec4::new(-1.0, 2.0, -3.0, 0.0);
        let a = v.abs();
        assert_eq!(a, Vec4::new(1.0, 2.0, 3.0, 0.0));
        let s = v.sign();
        assert_eq!(s.x, -1.0);
        assert_eq!(s.y, 1.0);
        assert_eq!(s.z, -1.0);
    }

    #[test]
    fn vec4_floor_fract_roundtrip() {
        let v = Vec4::new(3.7, -0.3, 1.5, 2.9);
        let f = v.floor();
        let frac = v.fract();
        assert!((f.x + frac.x - v.x).abs() < 1e-5);
        assert!((f.y + frac.y - v.y).abs() < 1e-5);
    }

    // ── Spline tests ──────────────────────────────────────────────────────────

    #[test]
    fn bezier_quadratic_endpoints() {
        let p = bezier_quadratic(Vec3::ZERO, Vec3::new(0.5, 1.0, 0.0), Vec3::ONE, 0.0);
        assert!(p.length() < 1e-5);
        let p1 = bezier_quadratic(Vec3::ZERO, Vec3::new(0.5, 1.0, 0.0), Vec3::ONE, 1.0);
        assert!((p1 - Vec3::ONE).length() < 1e-5);
    }

    #[test]
    fn bezier_cubic_endpoints() {
        let a = Vec3::ZERO;
        let d = Vec3::new(3.0, 0.0, 0.0);
        let p0 = bezier_cubic(a, Vec3::X, Vec3::new(2.0, 1.0, 0.0), d, 0.0);
        assert!(p0.length() < 1e-5);
        let p1 = bezier_cubic(a, Vec3::X, Vec3::new(2.0, 1.0, 0.0), d, 1.0);
        assert!((p1 - d).length() < 1e-5);
    }

    #[test]
    fn bezier_cubic_tangent_endpoints() {
        // At t=0 tangent should be 3*(p1-p0)
        let p0 = Vec3::ZERO;
        let p1 = Vec3::X;
        let p2 = Vec3::new(2.0, 0.0, 0.0);
        let p3 = Vec3::new(3.0, 0.0, 0.0);
        let tang = bezier_cubic_tangent(p0, p1, p2, p3, 0.0);
        assert!((tang - Vec3::new(3.0, 0.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn catmull_rom_endpoints() {
        // Passes through p1 at t=0 and p2 at t=1
        let p = catmull_rom(
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::ZERO,
            Vec3::ONE,
            Vec3::new(2.0, 1.0, 0.0),
            0.0,
        );
        assert!(p.length() < 1e-5);
        let p1 = catmull_rom(
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::ZERO,
            Vec3::ONE,
            Vec3::new(2.0, 1.0, 0.0),
            1.0,
        );
        assert!((p1 - Vec3::ONE).length() < 1e-5);
    }

    #[test]
    fn hermite_endpoints() {
        let p = hermite(Vec3::ZERO, Vec3::X, Vec3::ONE, Vec3::X, 0.0);
        assert!(p.length() < 1e-5);
        let p1 = hermite(Vec3::ZERO, Vec3::X, Vec3::ONE, Vec3::X, 1.0);
        assert!((p1 - Vec3::ONE).length() < 1e-5);
    }

    #[test]
    fn hermite_zero_tangent_midpoint() {
        // With zero tangents, midpoint should be at 0.5 on each axis (symmetric)
        let p = hermite(Vec3::ZERO, Vec3::ZERO, Vec3::ONE, Vec3::ZERO, 0.5);
        assert!((p.x - 0.5).abs() < 1e-5);
    }

    // ── Mat4::unproject test ──────────────────────────────────────────────────

    #[test]
    fn unproject_ortho_center() {
        // Orthographic proj centered at origin — screen center should unproject along -Z
        let proj = Mat4::orthographic(-1.0, 1.0, -1.0, 1.0, 0.1, 100.0);
        let dir = Mat4::unproject(400.0, 300.0, 800, 600, &proj);
        // Ortho ray is axis-aligned; x and y should be near 0 at screen center
        assert!(dir.x.abs() < 1e-3, "expected x≈0, got {}", dir.x);
        assert!(dir.y.abs() < 1e-3, "expected y≈0, got {}", dir.y);
    }

    // ── Mat4::row ─────────────────────────────────────────────────────────────

    #[test]
    fn mat4_row_identity() {
        let m = Mat4::identity();
        assert_eq!(m.row(0), Vec4::new(1.0, 0.0, 0.0, 0.0));
        assert_eq!(m.row(1), Vec4::new(0.0, 1.0, 0.0, 0.0));
        assert_eq!(m.row(3), Vec4::new(0.0, 0.0, 0.0, 1.0));
    }

    #[test]
    fn mat4_row_translation() {
        let m = Mat4::translation(5.0, -3.0, 7.0);
        // Translation is in row 3 in row-vector convention
        let r3 = m.row(3);
        assert!((r3.x - 5.0).abs() < 1e-5);
        assert!((r3.y - (-3.0)).abs() < 1e-5);
        assert!((r3.z - 7.0).abs() < 1e-5);
    }

    // ── Vec smoothstep ────────────────────────────────────────────────────────

    #[test]
    fn vec2_smoothstep_endpoints() {
        let e0 = Vec2::ZERO;
        let e1 = Vec2::ONE;
        assert_eq!(Vec2::ZERO.smoothstep(e0, e1), Vec2::ZERO);
        assert_eq!(Vec2::ONE.smoothstep(e0, e1), Vec2::ONE);
    }

    #[test]
    fn vec2_smoothstep_midpoint() {
        let v = Vec2::splat(0.5).smoothstep(Vec2::ZERO, Vec2::ONE);
        assert!((v.x - 0.5).abs() < 1e-5);
    }

    #[test]
    fn vec3_smoothstep_endpoints() {
        let e0 = Vec3::ZERO;
        let e1 = Vec3::ONE;
        let at_zero = Vec3::ZERO.smoothstep(e0, e1);
        let at_one = Vec3::ONE.smoothstep(e0, e1);
        assert!(at_zero.length() < 1e-5);
        assert!((at_one - Vec3::ONE).length() < 1e-5);
    }

    #[test]
    fn test_fast_normalize_accuracy() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        let n1 = v.normalize();
        let n2 = v.fast_normalize();

        let diff = n1 - n2;
        assert!(diff.x.abs() < 0.001);
        assert!(diff.y.abs() < 0.001);
        assert!(diff.z.abs() < 0.001);
    }

    #[test]
    fn test_transform_points_parallel_threshold() {
        // Test parallel implementation properly falls back and maintains correctness
        let points = vec![Vec3::new(1.0, 2.0, 3.0); 100];
        let mut output = vec![(Vec3::default(), 0.0); 100];
        let m = Mat4::translation(5.0, 5.0, 5.0);

        m.transform_points_parallel(&points, &mut output);

        for (p, _) in output {
            assert!((p.x - 6.0).abs() < 0.001);
            assert!((p.y - 7.0).abs() < 0.001);
            assert!((p.z - 8.0).abs() < 0.001);
        }
    }

    #[test]
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    fn test_transform_point_simd_vs_scalar() {
        // Scalar implementation reference
        fn transform_point_scalar(m: &Mat4, v: Vec3) -> (Vec3, f32) {
            let x = m.m[0][0] * v.x + m.m[1][0] * v.y + m.m[2][0] * v.z + m.m[3][0];
            let y = m.m[0][1] * v.x + m.m[1][1] * v.y + m.m[2][1] * v.z + m.m[3][1];
            let z = m.m[0][2] * v.x + m.m[1][2] * v.y + m.m[2][2] * v.z + m.m[3][2];
            let w = m.m[0][3] * v.x + m.m[1][3] * v.y + m.m[2][3] * v.z + m.m[3][3];
            (Vec3::new(x, y, z), w)
        }

        let m = Mat4::rotation_y(0.5) * Mat4::translation(10.0, 5.0, 2.0);
        let v = Vec3::new(1.0, 2.0, 3.0);

        // This uses the SIMD implementation because we are compiling with simd feature
        let (simd_p, simd_w) = m.transform_point(v);
        let (scalar_p, scalar_w) = transform_point_scalar(&m, v);

        let diff_p = simd_p - scalar_p;
        assert!(
            diff_p.x.abs() < 0.0001,
            "X mismatch: {} vs {}",
            simd_p.x,
            scalar_p.x
        );
        assert!(
            diff_p.y.abs() < 0.0001,
            "Y mismatch: {} vs {}",
            simd_p.y,
            scalar_p.y
        );
        assert!(
            diff_p.z.abs() < 0.0001,
            "Z mismatch: {} vs {}",
            simd_p.z,
            scalar_p.z
        );
        assert!(
            (simd_w - scalar_w).abs() < 0.0001,
            "W mismatch: {simd_w} vs {scalar_w}"
        );
    }

    #[test]
    fn test_perspective_projection() {
        use std::f32::consts::PI;
        let fov = PI / 2.0; // 90 degrees
        let aspect = 1.0;
        let near = 1.0;
        let far = 10.0;
        let proj = Mat4::perspective(fov, aspect, near, far);

        // Point on near plane (0, 0, -1) -> should map to w=1, z/w = -1 (OpenGL style: -1 to 1)
        // Wait, standard GL perspective maps -near to -1 and -far to 1 (or 0 to 1 depending on depth range).
        // Let's check the implementation:
        // [0][0] = f / aspect
        // [2][2] = (far + near) / (near - far) (This is typically negative)
        // [2][3] = -1.0
        // [3][2] = 2 * far * near / (near - far)
        //
        // p = (0, 0, -near)
        // x' = 0
        // y' = 0
        // z' = p.z * m[2][2] + m[3][2]
        // w' = p.z * m[2][3] + m[3][3] = -p.z = near
        //
        // z_ndc = z' / w'
        // Let's verify with actual values.

        let p_near = Vec3::new(0.0, 0.0, -near);
        let (p_near_prime, w_near) = proj.transform_point(p_near);

        assert!(
            (w_near - near).abs() < 1e-5,
            "w at near plane should be near"
        );
        // In standard GL, z_ndc at near is -1.0
        let z_ndc_near = p_near_prime.z / w_near;
        assert!(
            (z_ndc_near - (-1.0)).abs() < 1e-5,
            "NDZ z at near should be -1.0, got {z_ndc_near}"
        );

        let p_far = Vec3::new(0.0, 0.0, -far);
        let (p_far_prime, w_far) = proj.transform_point(p_far);
        assert!((w_far - far).abs() < 1e-5, "w at far plane should be far");
        // In standard GL, z_ndc at far is 1.0
        let z_ndc_far = p_far_prime.z / w_far;
        assert!(
            (z_ndc_far - 1.0).abs() < 1e-5,
            "NDC z at far should be 1.0, got {z_ndc_far}"
        );
    }

    #[test]
    fn test_look_at() {
        let eye = Vec3::new(0.0, 0.0, 10.0);
        let target = Vec3::new(0.0, 0.0, 0.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let view = Mat4::look_at(eye, target, up);

        // Point at target (world origin) should map to (0, 0, -10) in camera space
        // because camera is at (0, 0, 10) looking at origin, so origin is 10 units in front (negative Z)
        let p = Vec3::new(0.0, 0.0, 0.0);
        let (p_view, _) = view.transform_point(p);

        // Relaxed tolerance due to fast_inv_sqrt usage in look_at normalization
        let epsilon = 1e-3;
        assert!((p_view.x - 0.0).abs() < epsilon, "X mismatch: {}", p_view.x);
        assert!((p_view.y - 0.0).abs() < epsilon, "Y mismatch: {}", p_view.y);
        assert!(
            (p_view.z - (-10.0)).abs() < epsilon,
            "Z mismatch: {}",
            p_view.z
        );

        // Point at eye should map to (0, 0, 0)
        let (p_eye, _) = view.transform_point(eye);
        assert!(p_eye.length() < 1e-5);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_project_to_screen_optimized_edge_cases() {
        let half_width = 400.0;
        let half_height = 300.0;

        // Test w = 0 (singular)
        // Code falls back to 1.0 if w.abs() <= 0.0001
        let p = Vec3::new(100.0, 100.0, 10.0);
        let sp = project_to_screen_optimized(p, 0.0, half_width, half_height);

        // Expected behavior: inv_w = 1.0, so x = 100.0, y = 100.0
        // ndc_x = 100.0. screen_x = (100+1)*400 = 40400.
        assert!((sp.inv_w - 1.0).abs() < f32::EPSILON);
        assert_eq!(sp.x, 40400);

        // Test very small w (but > epsilon)
        // w = 0.0002. inv_w = 5000.
        // x = 1.0. ndc_x = 5000.
        // screen_x = (5000+1)*400 = 2000400.
        let sp_small =
            project_to_screen_optimized(Vec3::new(1.0, 0.0, 0.0), 0.0002, half_width, half_height);
        assert!((sp_small.inv_w - 5000.0).abs() < 1e-1);
        assert_eq!(sp_small.x, 2000400);

        // Test negative w (behind camera)
        // w = -1.0. inv_w = -1.0.
        // x = 1.0. ndc_x = -1.0.
        // screen_x = (-1+1)*400 = 0.
        let sp_neg =
            project_to_screen_optimized(Vec3::new(1.0, 0.0, 0.0), -1.0, half_width, half_height);
        assert!((sp_neg.inv_w - -1.0).abs() < f32::EPSILON);
        assert_eq!(sp_neg.x, 0);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_vec3_normalize_zero() {
        let v = Vec3::new(0.0, 0.0, 0.0);
        let n = v.normalize();
        assert!((n.x - 0.0).abs() < f32::EPSILON);
        assert!((n.y - 0.0).abs() < f32::EPSILON);
        assert!((n.z - 0.0).abs() < f32::EPSILON);

        let v_small = Vec3::new(1e-5, 0.0, 0.0);
        let n_small = v_small.normalize();
        // Should return original if length < 0.0001
        assert!((n_small.x - 1e-5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_fast_inv_sqrt_sanity() {
        let x = 4.0;
        let y = fast_inv_sqrt(x);
        // 1/sqrt(4) = 0.5
        assert!((y - 0.5).abs() < 0.01);

        let x = 16.0;
        let y = fast_inv_sqrt(x);
        // 1/sqrt(16) = 0.25
        assert!((y - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_project_to_screen_safety() {
        let half_width = 400.0;
        let half_height = 300.0;

        // Test Infinity
        let v_inf = Vec3::new(f32::INFINITY, 0.0, 0.0);
        let sp_inf = project_to_screen_optimized(v_inf, 1.0, half_width, half_height);
        // Expect clamping to max/min range
        assert!(sp_inf.x == 2147483520);

        // Test Negative Infinity
        let v_neg_inf = Vec3::new(f32::NEG_INFINITY, 0.0, 0.0);
        let sp_neg_inf = project_to_screen_optimized(v_neg_inf, 1.0, half_width, half_height);
        assert!(sp_neg_inf.x == -2147483520);

        // Test NaN
        let v_nan = Vec3::new(f32::NAN, 0.0, 0.0);
        let sp_nan = project_to_screen_optimized(v_nan, 1.0, half_width, half_height);
        // Expect clamping to MIN/MAX range (NaN maps to MIN in this implementation)
        assert_eq!(sp_nan.x, -2147483520);

        // Test Large Number (overflowing i32 but finite)
        let v_large = Vec3::new(1e30, 0.0, 0.0);
        let sp_large = project_to_screen_optimized(v_large, 1.0, half_width, half_height);
        // Should clamp to 2147483520 (approx i32::MAX)
        assert_eq!(sp_large.x, 2147483520);
    }

    #[test]
    fn test_mat2_rotation() {
        use std::f32::consts::FRAC_PI_2;
        let m = Mat2::rotation(FRAC_PI_2);
        // cos(90) is approx 0, sin(90) is 1
        assert!(m.m[0][0].abs() < 1e-6);
        assert!((m.m[0][1] - (-1.0)).abs() < 1e-6);
        assert!((m.m[1][0] - 1.0).abs() < 1e-6);
        assert!(m.m[1][1].abs() < 1e-6);
    }

    #[test]
    fn test_mat2_transform() {
        use std::f32::consts::FRAC_PI_2;
        let m = Mat2::rotation(FRAC_PI_2);
        let v = Vec2::new(1.0, 0.0);
        let result = m.transform(v);
        // (1, 0) rotated 90 deg -> (0, 1)
        assert!(result.x.abs() < 1e-6);
        assert!((result.y - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_mat2_transform_batch() {
        use std::f32::consts::PI;
        let m = Mat2::rotation(PI);
        let vertices = vec![Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)];
        let result = m.transform_batch(&vertices);
        // Rotate 180 degrees -> (-x, -y)
        assert!((result[0].x - (-1.0)).abs() < 1e-6);
        assert!(result[0].y.abs() < 1e-6);
        assert!(result[1].x.abs() < 1e-6);
        assert!((result[1].y - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_mat2_transform_in_place() {
        use std::f32::consts::PI;
        let m = Mat2::rotation(PI);
        let mut vertices = vec![Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)];
        m.transform_in_place(&mut vertices);
        // Rotate 180 degrees -> (-x, -y)
        assert!((vertices[0].x - (-1.0)).abs() < 1e-6);
        assert!(vertices[0].y.abs() < 1e-6);
        assert!(vertices[1].x.abs() < 1e-6);
        assert!((vertices[1].y - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_vec2_rotate() {
        let rotated = Vec2::new(1.0, 0.0).rotate(std::f32::consts::FRAC_PI_2);
        assert!(rotated.x.abs() < 0.02);
        assert!((rotated.y - 1.0).abs() < 0.02);
    }

    #[test]
    fn test_vec4_new() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        assert!((v.x - 1.0).abs() < f32::EPSILON);
        assert!((v.y - 2.0).abs() < f32::EPSILON);
        assert!((v.z - 3.0).abs() < f32::EPSILON);
        assert!((v.w - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_vec4_add() {
        let v1 = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let v2 = Vec4::new(5.0, 6.0, 7.0, 8.0);
        let result = v1 + v2;
        assert!((result.x - 6.0).abs() < f32::EPSILON);
        assert!((result.y - 8.0).abs() < f32::EPSILON);
        assert!((result.z - 10.0).abs() < f32::EPSILON);
        assert!((result.w - 12.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_vec4_sub() {
        let v1 = Vec4::new(5.0, 6.0, 7.0, 8.0);
        let v2 = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let result = v1 - v2;
        assert!((result.x - 4.0).abs() < f32::EPSILON);
        assert!((result.y - 4.0).abs() < f32::EPSILON);
        assert!((result.z - 4.0).abs() < f32::EPSILON);
        assert!((result.w - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_vec3_reflect() {
        let v = Vec3::new(1.0, -1.0, 0.0);
        let normal = Vec3::new(0.0, 1.0, 0.0);
        let r = v.reflect(normal);
        assert!((r.x - 1.0).abs() < 1e-6);
        assert!((r.y - 1.0).abs() < 1e-6);
        assert!(r.z.abs() < 1e-6);
    }

    #[test]
    fn test_vec3_reflect_normalized_matches_reflect() {
        let v = Vec3::new(0.25, -0.5, 1.2);
        let n = Vec3::new(0.0, 1.0, 0.0);
        let a = v.reflect(n);
        let b = v.reflect_normalized(n);
        assert!((a.x - b.x).abs() < 1e-6);
        assert!((a.y - b.y).abs() < 1e-6);
        assert!((a.z - b.z).abs() < 1e-6);
    }

    #[test]
    fn test_vec3_refract_air_to_glass() {
        // 45-degree incidence from air to glass.
        let incident = Vec3::new(1.0, -1.0, 0.0).normalize();
        let normal = Vec3::new(0.0, 1.0, 0.0);
        let refracted = incident.refract(normal, 1.0 / 1.5);

        // Should still travel downward, bent toward the normal.
        assert!(refracted.y < 0.0);
        assert!(refracted.length() > 0.99 && refracted.length() < 1.01);
    }

    #[test]
    fn test_vec3_refract_total_internal_reflection() {
        // Steep angle from dense to sparse medium should TIR.
        let incident = Vec3::new(1.0, -0.1, 0.0).normalize();
        let normal = Vec3::new(0.0, 1.0, 0.0);
        let refracted = incident.refract(normal, 1.5);
        assert_eq!(refracted, Vec3::ZERO);
    }

    #[test]
    fn test_vec3_face_forward() {
        let n = Vec3::new(0.0, 1.0, 0.0);
        let i_towards = Vec3::new(0.0, -1.0, 0.0);
        let i_away = Vec3::new(0.0, 1.0, 0.0);
        let nref = Vec3::new(0.0, 1.0, 0.0);

        assert_eq!(n.face_forward(i_towards, nref), n);
        assert_eq!(n.face_forward(i_away, nref), n * -1.0);
    }

    #[test]
    fn test_mat4_orthographic() {
        let proj = Mat4::orthographic(-10.0, 10.0, -5.0, 5.0, 0.1, 100.0);

        let p_center = Vec3::new(0.0, 0.0, -50.0);
        let (p_center_prime, w_center) = proj.transform_point(p_center);
        assert!((w_center - 1.0).abs() < 1e-5);
        assert!(p_center_prime.x.abs() < 1e-5);
        assert!(p_center_prime.y.abs() < 1e-5);

        // Orthographic projection preserves W as 1.0
        // -50 in Z should map between -1 and 1 in NDC
        let z_ndc = p_center_prime.z / w_center;
        assert!(z_ndc >= -1.0 && z_ndc <= 1.0);
    }

    #[test]
    fn test_vec4_mul_scalar() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let result = v * 2.5;
        assert!((result.x - 2.5).abs() < f32::EPSILON);
        assert!((result.y - 5.0).abs() < f32::EPSILON);
        assert!((result.z - 7.5).abs() < f32::EPSILON);
        assert!((result.w - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_vec4_dot_length_normalize() {
        let a = Vec4::new(1.0, 2.0, 2.0, 1.0);
        let b = Vec4::new(-1.0, 0.5, 3.0, 2.0);
        assert!((a.dot(b) - 8.0).abs() < 1e-6);
        assert!((a.length_sq() - 10.0).abs() < 1e-6);
        assert!((a.length() - 10.0_f32.sqrt()).abs() < 1e-6);

        let n = a.normalize();
        assert!((n.length() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_vec4_project_and_reject() {
        let v = Vec4::new(3.0, 4.0, 0.0, 0.0);
        let onto = Vec4::new(1.0, 0.0, 0.0, 0.0);
        let proj = v.project_onto(onto);
        let rej = v.reject_from(onto);

        assert!((proj.x - 3.0).abs() < 1e-6);
        assert!(proj.y.abs() < 1e-6);
        assert!(proj.z.abs() < 1e-6);
        assert!(proj.w.abs() < 1e-6);

        assert!(rej.x.abs() < 1e-6);
        assert!((rej.y - 4.0).abs() < 1e-6);
        assert!(rej.z.abs() < 1e-6);
        assert!(rej.w.abs() < 1e-6);
    }

    #[test]
    fn test_vec4_min_max_clamp_distance() {
        let a = Vec4::new(-1.0, 3.0, 10.0, 0.5);
        let b = Vec4::new(2.0, 1.0, 7.0, 2.0);

        let min = a.min(b);
        let max = a.max(b);
        assert_eq!(min, Vec4::new(-1.0, 1.0, 7.0, 0.5));
        assert_eq!(max, Vec4::new(2.0, 3.0, 10.0, 2.0));

        let clamped = Vec4::new(3.0, 0.0, 8.0, 1.5).clamp(min, max);
        assert_eq!(clamped, Vec4::new(2.0, 1.0, 8.0, 1.5));

        assert!((a.distance_sq(b) - 24.25).abs() < 1e-6);
        assert!((a.distance(b) - 24.25_f32.sqrt()).abs() < 1e-6);
    }

    #[test]
    fn test_vec3_min_max() {
        let a = Vec3::new(1.0, 5.0, -2.0);
        let b = Vec3::new(3.0, 2.0, -1.0);

        let min = a.min(b);
        assert!((min.x - 1.0).abs() < f32::EPSILON);
        assert!((min.y - 2.0).abs() < f32::EPSILON);
        assert!((min.z - -2.0).abs() < f32::EPSILON);

        let max = a.max(b);
        assert!((max.x - 3.0).abs() < f32::EPSILON);
        assert!((max.y - 5.0).abs() < f32::EPSILON);
        assert!((max.z - -1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_vec3_orthonormal_basis() {
        let n = Vec3::new(0.3, 0.5, 0.8).normalize();
        let (t, b) = n.orthonormal_basis();

        assert!((t.length() - 1.0).abs() < 1e-4);
        assert!((b.length() - 1.0).abs() < 1e-4);
        assert!(n.dot(t).abs() < 1e-4);
        assert!(n.dot(b).abs() < 1e-4);
        assert!(t.dot(b).abs() < 1e-4);
    }

    #[test]
    fn test_vec3_orthonormal_basis_degenerate_input() {
        let (t, b) = Vec3::ZERO.orthonormal_basis();
        assert!((t.length() - 1.0).abs() < 1e-4);
        assert!((b.length() - 1.0).abs() < 1e-4);
        assert!(t.dot(b).abs() < 1e-4);
    }

    #[test]
    fn test_vec3_slerp_midpoint() {
        let x = Vec3::new(1.0, 0.0, 0.0);
        let y = Vec3::new(0.0, 1.0, 0.0);
        let mid = x.slerp(y, 0.5);
        let inv_sqrt2 = 1.0 / 2.0_f32.sqrt();
        assert!((mid.x - inv_sqrt2).abs() < 1e-4);
        assert!((mid.y - inv_sqrt2).abs() < 1e-4);
        assert!(mid.z.abs() < 1e-4);
        assert!((mid.length() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_vec3_project_onto_normalized_matches_regular_projection() {
        let v = Vec3::new(3.0, 4.0, 5.0);
        let unit = Vec3::new(2.0, -1.0, 3.0).normalize();
        let a = v.project_onto(unit);
        let b = v.project_onto_normalized(unit);
        assert!((a.x - b.x).abs() < 1e-5);
        assert!((a.y - b.y).abs() < 1e-5);
        assert!((a.z - b.z).abs() < 1e-5);
    }

    #[test]
    fn test_vec3_clamp_length() {
        let v = Vec3::new(3.0, 4.0, 0.0);
        let clamped = v.clamp_length(2.0);
        assert!((clamped.length() - 2.0).abs() < 1e-4);

        let unchanged = v.clamp_length(10.0);
        assert!((unchanged.x - v.x).abs() < 1e-6);
        assert!((unchanged.y - v.y).abs() < 1e-6);
        assert!((unchanged.z - v.z).abs() < 1e-6);
    }

    #[test]
    fn test_vec3_is_finite() {
        assert!(Vec3::new(1.0, -2.0, 3.0).is_finite());
        assert!(!Vec3::new(f32::INFINITY, 0.0, 0.0).is_finite());
        assert!(!Vec3::new(0.0, f32::NAN, 0.0).is_finite());
    }

    #[test]
    fn test_mat4_rotation_axis_matches_rotation_y() {
        use std::f32::consts::FRAC_PI_2;
        let rot_axis = Mat4::rotation_axis(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);
        let rot_y = Mat4::rotation_y(FRAC_PI_2);
        let v = Vec3::new(1.0, 0.0, 0.0);
        let (a, _) = rot_axis.transform_point(v);
        let (b, _) = rot_y.transform_point(v);
        assert!((a.x - b.x).abs() < 1e-5);
        assert!((a.y - b.y).abs() < 1e-5);
        assert!((a.z - b.z).abs() < 1e-5);
    }

    #[test]
    fn test_mat4_transform_vector_ignores_translation() {
        let m = Mat4::rotation_z(1.0) * Mat4::translation(10.0, 20.0, 30.0);
        let v = Vec3::new(2.0, -1.0, 3.0);
        let transformed = m.transform_vector(v);
        let (point_transformed, _) = m.transform_point(v);
        let translated_delta = point_transformed - transformed;
        assert!((translated_delta.x - 10.0).abs() < 1e-4);
        assert!((translated_delta.y - 20.0).abs() < 1e-4);
        assert!((translated_delta.z - 30.0).abs() < 1e-4);
    }

    #[test]
    fn test_vec3_barycentric_roundtrip() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(2.0, 0.0, 0.0);
        let c = Vec3::new(0.0, 2.0, 0.0);
        let p = Vec3::new(0.5, 0.75, 0.0);

        let bary = p.barycentric_coordinates(a, b, c).unwrap();
        let reconstructed = Vec3::from_barycentric(a, b, c, bary);

        assert!((bary.x + bary.y + bary.z - 1.0).abs() < 1e-5);
        assert!((reconstructed.x - p.x).abs() < 1e-5);
        assert!((reconstructed.y - p.y).abs() < 1e-5);
        assert!((reconstructed.z - p.z).abs() < 1e-5);
    }

    #[test]
    fn test_vec3_barycentric_degenerate_triangle_returns_none() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(1.0, 1.0, 1.0);
        let c = Vec3::new(2.0, 2.0, 2.0);
        let p = Vec3::new(0.2, 0.4, 0.6);
        assert!(p.barycentric_coordinates(a, b, c).is_none());
    }

    #[test]
    fn test_mat4_transform_vectors_batch_matches_scalar() {
        let m = Mat4::rotation_y(0.37) * Mat4::rotation_x(-0.22) * Mat4::translation(4.0, 5.0, 6.0);
        let input = vec![
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(-1.0, 0.5, 0.0),
            Vec3::new(0.0, -3.0, 2.0),
        ];
        let mut output = vec![Vec3::ZERO; input.len()];
        m.transform_vectors(&input, &mut output);

        for (i, v) in input.iter().enumerate() {
            let scalar = m.transform_vector(*v);
            assert!((scalar.x - output[i].x).abs() < 1e-5);
            assert!((scalar.y - output[i].y).abs() < 1e-5);
            assert!((scalar.z - output[i].z).abs() < 1e-5);
        }
    }

    #[test]
    fn test_mat4_transform_points_affine_matches_transform_point() {
        let m =
            Mat4::scale(2.0, 3.0, 4.0) * Mat4::rotation_z(0.5) * Mat4::translation(8.0, -2.0, 1.0);
        let input = vec![
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(-4.0, 1.5, 0.25),
            Vec3::new(0.0, 0.0, 0.0),
        ];
        let mut output = vec![Vec3::ZERO; input.len()];
        m.transform_points_affine(&input, &mut output);

        for (i, p) in input.iter().enumerate() {
            let scalar = m.transform_point(*p).0;
            assert!((scalar.x - output[i].x).abs() < 1e-5);
            assert!((scalar.y - output[i].y).abs() < 1e-5);
            assert!((scalar.z - output[i].z).abs() < 1e-5);
        }
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_project_to_screen_simd_consistency() {
        let half_width = 400.0;
        let half_height = 300.0;

        let test_cases = vec![
            (Vec3::new(100.0, 100.0, 10.0), 1.0, "Normal"),
            (Vec3::new(0.0, 0.0, 0.0), 1.0, "Origin"),
            (Vec3::new(1.0, 1.0, 1.0), 0.0000001, "Small w (epsilon)"),
            (Vec3::new(1.0, 1.0, 1.0), 0.0, "Zero w"),
            (Vec3::new(1.0, 1.0, 1.0), -1.0, "Negative w"),
            (Vec3::new(f32::INFINITY, 0.0, 0.0), 1.0, "Inf X"),
            (Vec3::new(f32::NAN, 0.0, 0.0), 1.0, "NaN X"),
            (Vec3::new(1e30, 0.0, 0.0), 1.0, "Large X"),
            (Vec3::new(-1e30, 0.0, 0.0), 1.0, "Large Negative X"),
            (Vec3::new(0.0, 0.0, 0.0), f32::INFINITY, "Inf W"),
        ];

        for (v, w, name) in test_cases {
            // Scalar
            let s_scalar = project_to_screen_optimized(v, w, half_width, half_height);

            // SIMD (Triangle)
            let (s_tri_0, _, _) =
                project_triangle_to_screen(v, w, v, w, v, w, half_width, half_height);

            // Verify X and Y (allow off-by-one due to float precision + truncation)
            assert!(
                (i64::from(s_scalar.x) - i64::from(s_tri_0.x)).abs() <= 1,
                "X mismatch for case {}: {} vs {}",
                name,
                s_scalar.x,
                s_tri_0.x
            );
            assert!(
                (i64::from(s_scalar.y) - i64::from(s_tri_0.y)).abs() <= 1,
                "Y mismatch for case {}: {} vs {}",
                name,
                s_scalar.y,
                s_tri_0.y
            );

            // Check z and inv_w with some tolerance
            let z_diff = (s_scalar.z - s_tri_0.z).abs();
            let inv_w_diff = (s_scalar.inv_w - s_tri_0.inv_w).abs();

            let tolerance = if w.abs() > 1e-4 {
                0.002 // Approximation error
            } else {
                1.0 // Loose tolerance for fallback/singularities
            };

            if s_scalar.z.is_nan() {
                assert!(s_tri_0.z.is_nan(), "Z NaN mismatch for case: {name}");
            } else {
                assert!(
                    z_diff < tolerance || (s_scalar.z.is_infinite() && s_tri_0.z.is_infinite()),
                    "Z mismatch for {}: {} vs {} (diff: {})",
                    name,
                    s_scalar.z,
                    s_tri_0.z,
                    z_diff
                );
            }

            if s_scalar.inv_w.is_nan() {
                assert!(s_tri_0.inv_w.is_nan(), "InvW NaN mismatch for case: {name}");
            } else {
                assert!(
                    inv_w_diff < tolerance
                        || (s_scalar.inv_w.is_infinite() && s_tri_0.inv_w.is_infinite()),
                    "InvW mismatch for {}: {} vs {} (diff: {})",
                    name,
                    s_scalar.inv_w,
                    s_tri_0.inv_w,
                    inv_w_diff
                );
            }
        }
    }
}

/// A 4-component vector, often used for homogeneous coordinates or tangents.
///
/// In the rasterization pipeline, `Vec4` is used for:
/// *   Homogeneous coordinates (x, y, z, w) where w is the perspective term.
/// *   Tangent vectors in Normal Mapping, where w stores the handedness of the tangent basis.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec4;
///
/// let v = Vec4::new(1.0, 2.0, 3.0, 1.0);
/// assert_eq!(v.x, 1.0);
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[allow(missing_docs)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    #[allow(missing_docs)]
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 0.0,
    };
    #[allow(missing_docs)]
    pub const ONE: Self = Self {
        x: 1.0,
        y: 1.0,
        z: 1.0,
        w: 1.0,
    };

    /// Creates a vector with all components set to `v`.
    #[must_use]
    #[inline]
    pub const fn splat(v: f32) -> Self {
        Self {
            x: v,
            y: v,
            z: v,
            w: v,
        }
    }

    /// Returns the xyz components as a `Vec3`.
    #[must_use]
    #[inline]
    pub const fn xyz(self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }

    /// Linearly interpolate between this vector and another.
    ///
    /// The `t` factor dictates the blend: `0.0` returns `self`, `1.0` returns `other`.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::Vec4;
    ///
    /// let start = Vec4::new(0.0, 0.0, 0.0, 0.0);
    /// let end = Vec4::new(10.0, 10.0, 10.0, 10.0);
    /// let mid = start.lerp(end, 0.5);
    ///
    /// assert_eq!(mid.x, 5.0);
    /// assert_eq!(mid.w, 5.0);
    /// ```
    #[must_use]
    #[inline(always)]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            z: self.z + (other.z - self.z) * t,
            w: self.w + (other.w - self.w) * t,
        }
    }

    #[must_use]
    #[inline(always)]

    /// Creates a new 4D vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::Vec4;
    ///
    /// let v = Vec4::new(1.0, 2.0, 3.0, 1.0);
    /// assert_eq!(v.w, 1.0);
    /// ```

    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// Calculates dot product between two vectors.
    #[must_use]
    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }

    /// Calculates squared length (magnitude²) of the vector.
    #[must_use]
    #[inline]
    pub fn length_sq(self) -> f32 {
        self.dot(self)
    }

    /// Calculates the Euclidean length (magnitude) of the vector.
    #[must_use]
    #[inline]
    #[allow(clippy::imprecise_flops)]
    pub fn length(self) -> f32 {
        self.length_sq().sqrt()
    }

    /// Returns a normalized unit vector (length of 1.0).
    ///
    /// For tiny vectors (length² <= `1e-8`), returns the original vector.
    #[must_use]
    #[inline]
    pub fn normalize(self) -> Self {
        let len_sq = self.length_sq();
        if len_sq > 0.000_000_01 {
            let inv_len = len_sq.sqrt().recip();
            Self {
                x: self.x * inv_len,
                y: self.y * inv_len,
                z: self.z * inv_len,
                w: self.w * inv_len,
            }
        } else {
            self
        }
    }

    /// Distance to another vector.
    #[must_use]
    #[inline]
    pub fn distance(self, other: Self) -> f32 {
        (self - other).length()
    }

    /// Squared distance to another vector.
    #[must_use]
    #[inline]
    pub fn distance_sq(self, other: Self) -> f32 {
        (self - other).length_sq()
    }

    /// Component-wise minimum.
    #[must_use]
    #[inline]
    pub const fn min(self, other: Self) -> Self {
        Self {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
            z: self.z.min(other.z),
            w: self.w.min(other.w),
        }
    }

    /// Component-wise maximum.
    #[must_use]
    #[inline]
    pub const fn max(self, other: Self) -> Self {
        Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
            z: self.z.max(other.z),
            w: self.w.max(other.w),
        }
    }

    /// Clamp each component between corresponding min/max components.
    #[must_use]
    #[inline]
    pub const fn clamp(self, min: Self, max: Self) -> Self {
        Self {
            x: self.x.clamp(min.x, max.x),
            y: self.y.clamp(min.y, max.y),
            z: self.z.clamp(min.z, max.z),
            w: self.w.clamp(min.w, max.w),
        }
    }

    /// Projects this vector onto another vector.
    ///
    /// Returns `Vec4::ZERO` when `onto` is near zero to avoid division by tiny values.
    #[must_use]
    #[inline]
    pub fn project_onto(self, onto: Self) -> Self {
        let denom = onto.length_sq();
        if denom <= 0.000_000_01 {
            return Self::ZERO;
        }
        onto * (self.dot(onto) / denom)
    }

    /// Reject this vector from another vector (component orthogonal to `onto`).
    #[must_use]
    #[inline]
    pub fn reject_from(self, onto: Self) -> Self {
        self - self.project_onto(onto)
    }

    /// Component-wise absolute value.
    #[must_use]
    #[inline]
    pub fn abs(self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
            z: self.z.abs(),
            w: self.w.abs(),
        }
    }

    /// Component-wise sign: `−1.0`, `0.0`, or `+1.0`.
    #[must_use]
    #[inline]
    pub fn sign(self) -> Self {
        Self {
            x: self.x.signum(),
            y: self.y.signum(),
            z: self.z.signum(),
            w: self.w.signum(),
        }
    }

    /// Component-wise floor (round toward negative infinity).
    #[must_use]
    #[inline]
    pub fn floor(self) -> Self {
        Self {
            x: self.x.floor(),
            y: self.y.floor(),
            z: self.z.floor(),
            w: self.w.floor(),
        }
    }

    /// Component-wise ceiling (round toward positive infinity).
    #[must_use]
    #[inline]
    pub fn ceil(self) -> Self {
        Self {
            x: self.x.ceil(),
            y: self.y.ceil(),
            z: self.z.ceil(),
            w: self.w.ceil(),
        }
    }

    /// Component-wise round (round to nearest, ties to even).
    #[must_use]
    #[inline]
    pub fn round(self) -> Self {
        Self {
            x: self.x.round(),
            y: self.y.round(),
            z: self.z.round(),
            w: self.w.round(),
        }
    }

    /// Component-wise fractional part (`x - floor(x)`), matching GLSL semantics (result in [0, 1)).
    #[must_use]
    #[inline]
    pub fn fract(self) -> Self {
        Self {
            x: self.x - self.x.floor(),
            y: self.y - self.y.floor(),
            z: self.z - self.z.floor(),
            w: self.w - self.w.floor(),
        }
    }

    /// Smallest of the four components.
    #[must_use]
    #[inline]
    pub fn min_component(self) -> f32 {
        self.x.min(self.y).min(self.z).min(self.w)
    }

    /// Largest of the four components.
    #[must_use]
    #[inline]
    pub fn max_component(self) -> f32 {
        self.x.max(self.y).max(self.z).max(self.w)
    }

    /// Component-wise `x^exp`.
    #[must_use]
    #[inline]
    pub fn pow(self, exp: f32) -> Self {
        Self::new(
            self.x.powf(exp),
            self.y.powf(exp),
            self.z.powf(exp),
            self.w.powf(exp),
        )
    }

    /// Component-wise `e^x`.
    #[must_use]
    #[inline]
    pub fn exp(self) -> Self {
        Self::new(self.x.exp(), self.y.exp(), self.z.exp(), self.w.exp())
    }

    /// Component-wise natural log.
    #[must_use]
    #[inline]
    pub fn log(self) -> Self {
        Self::new(self.x.ln(), self.y.ln(), self.z.ln(), self.w.ln())
    }
}

/// Multiply vector by scalar.
impl std::ops::Mul<f32> for Vec4 {
    type Output = Self;
    #[inline]
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
            w: self.w * scalar,
        }
    }
}

/// Component-wise multiply.
impl std::ops::Mul for Vec4 {
    type Output = Self;
    #[inline]
    fn mul(self, other: Self) -> Self {
        Self {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z,
            w: self.w * other.w,
        }
    }
}

/// Component-wise addition.
impl std::ops::Add for Vec4 {
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
            w: self.w + other.w,
        }
    }
}

/// Component-wise subtraction.
impl std::ops::Sub for Vec4 {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
            w: self.w - other.w,
        }
    }
}

/// A 3×3 matrix for normals, 2D homogeneous transforms, and upper-left extraction.
///
/// Row-major, row-vector convention: `v' = v * M`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Mat3, Vec3};
///
/// let m = Mat3::identity();
/// let v = Vec3::new(1.0, 2.0, 3.0);
/// assert_eq!(m.transform(v), v);
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(missing_docs)]
pub struct Mat3 {
    pub m: [[f32; 3]; 3],
}

impl Mat3 {
    /// Identity matrix.
    #[must_use]
    #[inline]
    pub const fn identity() -> Self {
        Self {
            m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        }
    }

    /// Extract the upper-left 3×3 from a `Mat4`.
    #[must_use]
    #[inline]
    pub const fn from_mat4(m: &Mat4) -> Self {
        Self {
            m: [
                [m.m[0][0], m.m[0][1], m.m[0][2]],
                [m.m[1][0], m.m[1][1], m.m[1][2]],
                [m.m[2][0], m.m[2][1], m.m[2][2]],
            ],
        }
    }

    /// Rotation around the X axis.
    #[must_use]
    pub fn rotation_x(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self {
            m: [[1.0, 0.0, 0.0], [0.0, c, s], [0.0, -s, c]],
        }
    }

    /// Rotation around the Y axis.
    #[must_use]
    pub fn rotation_y(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self {
            m: [[c, 0.0, -s], [0.0, 1.0, 0.0], [s, 0.0, c]],
        }
    }

    /// Rotation around the Z axis.
    #[must_use]
    pub fn rotation_z(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self {
            m: [[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]],
        }
    }

    /// Uniform scale.
    #[must_use]
    #[inline]
    pub const fn scale(sx: f32, sy: f32, sz: f32) -> Self {
        Self {
            m: [[sx, 0.0, 0.0], [0.0, sy, 0.0], [0.0, 0.0, sz]],
        }
    }

    /// Transpose.
    #[must_use]
    #[inline]
    pub const fn transpose(&self) -> Self {
        let m = &self.m;
        Self {
            m: [
                [m[0][0], m[1][0], m[2][0]],
                [m[0][1], m[1][1], m[2][1]],
                [m[0][2], m[1][2], m[2][2]],
            ],
        }
    }

    /// Determinant.
    #[must_use]
    #[inline]
    pub fn determinant(&self) -> f32 {
        let m = &self.m;
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    }

    /// Inverse. Returns zero matrix if not invertible (det ≈ 0).
    #[must_use]
    pub fn inverse(&self) -> Self {
        let m = &self.m;
        let c00 = m[1][1] * m[2][2] - m[1][2] * m[2][1];
        let c01 = -(m[1][0] * m[2][2] - m[1][2] * m[2][0]);
        let c02 = m[1][0] * m[2][1] - m[1][1] * m[2][0];
        let det = m[0][0] * c00 + m[0][1] * c01 + m[0][2] * c02;
        if det.abs() < 1e-6 {
            return Self { m: [[0.0; 3]; 3] };
        }
        let inv = 1.0 / det;
        Self {
            m: [
                [
                    c00 * inv,
                    (-(m[0][1] * m[2][2] - m[0][2] * m[2][1])) * inv,
                    (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv,
                ],
                [
                    c01 * inv,
                    (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv,
                    (-(m[0][0] * m[1][2] - m[0][2] * m[1][0])) * inv,
                ],
                [
                    c02 * inv,
                    (-(m[0][0] * m[2][1] - m[0][1] * m[2][0])) * inv,
                    (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv,
                ],
            ],
        }
    }

    /// Inverse-transpose — used to transform normal vectors correctly under non-uniform scaling.
    #[must_use]
    #[inline]
    pub fn inverse_transpose(&self) -> Self {
        self.inverse().transpose()
    }

    /// Transform a `Vec3` by this matrix (row-vector: `v' = v * M`).
    #[must_use]
    #[inline]
    pub fn transform(&self, v: Vec3) -> Vec3 {
        Vec3::new(
            v.x * self.m[0][0] + v.y * self.m[1][0] + v.z * self.m[2][0],
            v.x * self.m[0][1] + v.y * self.m[1][1] + v.z * self.m[2][1],
            v.x * self.m[0][2] + v.y * self.m[1][2] + v.z * self.m[2][2],
        )
    }
}

impl std::ops::Mul for Mat3 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        let mut result = Self { m: [[0.0; 3]; 3] };
        for i in 0..3 {
            for k in 0..3 {
                let s = self.m[i][k];
                for j in 0..3 {
                    result.m[i][j] += s * rhs.m[k][j];
                }
            }
        }
        result
    }
}

impl Default for Mat3 {
    fn default() -> Self {
        Self::identity()
    }
}

impl std::ops::Div<f32> for Vec4 {
    type Output = Self;
    #[inline]
    fn div(self, scalar: f32) -> Self {
        let inv = 1.0 / scalar;
        Self {
            x: self.x * inv,
            y: self.y * inv,
            z: self.z * inv,
            w: self.w * inv,
        }
    }
}

#[cfg(test)]
mod tests_fast_sin {
    use super::*;

    #[test]
    fn test_fast_sin_cos() {
        use std::f32::consts::PI;
        for i in 0..360 {
            let rad = (i as f32) * PI / 180.0;
            let (fs, fc) = fast_sin_cos(rad);
            let s = rad.sin();
            let c = rad.cos();

            assert!((fs - s).abs() < 0.005, "sin mismatch at {i}: {fs} vs {s}");
            assert!((fc - c).abs() < 0.005, "cos mismatch at {i}: {fc} vs {c}");
        }
    }
}

#[cfg(test)]
mod tests_scalar_utils {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn test_lerp() {
        assert_eq!(lerp(0.0, 10.0, 0.0), 0.0);
        assert_eq!(lerp(0.0, 10.0, 1.0), 10.0);
        assert_eq!(lerp(0.0, 10.0, 0.5), 5.0);
        assert_eq!(lerp(-5.0, 5.0, 0.5), 0.0);
    }

    #[test]
    fn test_saturate() {
        assert_eq!(saturate(-1.0), 0.0);
        assert_eq!(saturate(2.0), 1.0);
        assert_eq!(saturate(0.5), 0.5);
    }

    #[test]
    fn test_smoothstep_edges() {
        assert_eq!(smoothstep(0.0, 1.0, 0.0), 0.0);
        assert_eq!(smoothstep(0.0, 1.0, 1.0), 1.0);
        assert!((smoothstep(0.0, 1.0, 0.5) - 0.5).abs() < 1e-6);
        // Below edge0
        assert_eq!(smoothstep(2.0, 4.0, 1.0), 0.0);
        // Above edge1
        assert_eq!(smoothstep(2.0, 4.0, 5.0), 1.0);
    }

    #[test]
    fn test_smootherstep_edges() {
        assert_eq!(smootherstep(0.0, 1.0, 0.0), 0.0);
        assert_eq!(smootherstep(0.0, 1.0, 1.0), 1.0);
        // Smoother step is also symmetric
        assert!((smootherstep(0.0, 1.0, 0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_remap() {
        assert_eq!(remap(5.0, 0.0, 10.0, 0.0, 1.0), 0.5);
        assert_eq!(remap(0.0, 0.0, 10.0, 0.0, 100.0), 0.0);
        assert_eq!(remap(10.0, 0.0, 10.0, 0.0, 100.0), 100.0);
    }

    #[test]
    fn test_fast_atan2_accuracy() {
        for deg in (0..360).step_by(10) {
            let rad = (deg as f32) * PI / 180.0;
            let y = rad.sin();
            let x = rad.cos();
            let expected = y.atan2(x);
            let got = fast_atan2(y, x);
            assert!(
                (got - expected).abs() < 0.005,
                "atan2 mismatch at {deg}°: got={got} expected={expected}"
            );
        }
    }

    // ── ping_pong / map_range / wrap ─────────────────────────────────────────

    #[test]
    fn ping_pong_basic() {
        assert!((ping_pong(0.0, 1.0) - 0.0).abs() < 1e-5);
        assert!((ping_pong(0.5, 1.0) - 0.5).abs() < 1e-5);
        assert!((ping_pong(1.0, 1.0) - 1.0).abs() < 1e-5);
        assert!((ping_pong(1.5, 1.0) - 0.5).abs() < 1e-5);
        assert!((ping_pong(2.0, 1.0) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn map_range_basic() {
        assert!((map_range(5.0, 0.0, 10.0, 0.0, 1.0) - 0.5).abs() < 1e-5);
        assert!((map_range(0.0, 0.0, 10.0, -1.0, 1.0) - (-1.0)).abs() < 1e-5);
        assert!((map_range(10.0, 0.0, 10.0, -1.0, 1.0) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn wrap_basic() {
        assert!((wrap(1.5, 0.0, 1.0) - 0.5).abs() < 1e-5);
        assert!((wrap(-0.5, 0.0, 1.0) - 0.5).abs() < 1e-5);
        assert!((wrap(0.3, 0.0, 1.0) - 0.3).abs() < 1e-5);
    }

    // ── Bézier eval ──────────────────────────────────────────────────────────

    #[test]
    fn quadratic_bezier_endpoints() {
        let p0 = Vec3::ZERO;
        let p1 = Vec3::new(0.5, 1.0, 0.0);
        let p2 = Vec3::new(1.0, 0.0, 0.0);
        let at0 = quadratic_bezier_eval(p0, p1, p2, 0.0);
        let at1 = quadratic_bezier_eval(p0, p1, p2, 1.0);
        assert!((at0 - p0).length() < 1e-5);
        assert!((at1 - p2).length() < 1e-5);
    }

    #[test]
    fn cubic_bezier_endpoints() {
        let p0 = Vec3::ZERO;
        let p1 = Vec3::new(0.0, 1.0, 0.0);
        let p2 = Vec3::new(1.0, 1.0, 0.0);
        let p3 = Vec3::new(1.0, 0.0, 0.0);
        let at0 = cubic_bezier_eval(p0, p1, p2, p3, 0.0);
        let at1 = cubic_bezier_eval(p0, p1, p2, p3, 1.0);
        assert!((at0 - p0).length() < 1e-5);
        assert!((at1 - p3).length() < 1e-5);
    }

    // ── Vec2 polar / angle_to ────────────────────────────────────────────────

    #[test]
    fn vec2_from_to_polar_roundtrip() {
        let orig = Vec2::new(3.0, 4.0);
        let (r, theta) = orig.to_polar();
        let back = Vec2::from_polar(r, theta);
        assert!((back.x - orig.x).abs() < 1e-5);
        assert!((back.y - orig.y).abs() < 1e-5);
    }

    #[test]
    fn vec2_angle_to_ccw() {
        let right = Vec2::new(1.0, 0.0);
        let up = Vec2::new(0.0, 1.0);
        let a = right.angle_to(up);
        assert!(
            (a - std::f32::consts::FRAC_PI_2).abs() < 1e-5,
            "expected π/2, got {a}"
        );
    }

    #[test]
    fn vec2_angle_to_cw_negative() {
        let up = Vec2::new(0.0, 1.0);
        let right = Vec2::new(1.0, 0.0);
        let a = up.angle_to(right);
        assert!(
            (a + std::f32::consts::FRAC_PI_2).abs() < 1e-5,
            "expected -π/2, got {a}"
        );
    }

    #[test]
    fn test_vec2_splat_and_axes() {
        assert_eq!(Vec2::splat(3.0), Vec2::new(3.0, 3.0));
        assert_eq!(Vec2::X, Vec2::new(1.0, 0.0));
        assert_eq!(Vec2::Y, Vec2::new(0.0, 1.0));
    }

    #[test]
    fn test_vec2_from_to_angle() {
        let v = Vec2::from_angle(PI / 4.0);
        let angle = v.to_angle();
        assert!((angle - PI / 4.0).abs() < 0.001);
    }

    #[test]
    fn test_vec3_constants() {
        assert_eq!(Vec3::X, Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(Vec3::Y, Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(Vec3::Z, Vec3::new(0.0, 0.0, 1.0));
        assert_eq!(Vec3::UP, Vec3::Y);
        assert_eq!(Vec3::RIGHT, Vec3::X);
        assert_eq!(Vec3::FORWARD, Vec3::new(0.0, 0.0, -1.0));
        assert_eq!(Vec3::splat(5.0), Vec3::new(5.0, 5.0, 5.0));
    }

    #[test]
    fn test_vec4_completions() {
        let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let b = Vec4::new(2.0, 0.0, 0.0, 0.0);
        assert_eq!(a.dot(b), 2.0);
        assert!((a.length() - (1.0f32 + 4.0 + 9.0 + 16.0).sqrt()).abs() < 1e-5);
        assert_eq!(a.xyz(), Vec3::new(1.0, 2.0, 3.0));
        let n = Vec4::new(1.0, 0.0, 0.0, 0.0).normalize();
        assert!((n.x - 1.0).abs() < 0.01, "normalize x: {}", n.x);
    }
}

#[cfg(test)]
mod tests_mat3 {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    fn approx_eq_vec3(a: Vec3, b: Vec3) -> bool {
        (a.x - b.x).abs() < 1e-5 && (a.y - b.y).abs() < 1e-5 && (a.z - b.z).abs() < 1e-5
    }

    #[test]
    fn test_mat3_identity_transform() {
        let m = Mat3::identity();
        let v = Vec3::new(1.0, 2.0, 3.0);
        assert!(approx_eq_vec3(m.transform(v), v));
    }

    #[test]
    fn test_mat3_rotation_y_90() {
        let m = Mat3::rotation_y(FRAC_PI_2);
        // X rotates to -Z in right-handed system
        let v = m.transform(Vec3::X);
        assert!(approx_eq_vec3(v, Vec3::new(0.0, 0.0, -1.0)));
    }

    #[test]
    fn test_mat3_from_mat4() {
        let m4 = Mat4::rotation_x(FRAC_PI_2);
        let m3 = Mat3::from_mat4(&m4);
        let v = Vec3::new(0.0, 1.0, 0.0);
        let via_mat4 = m4.transform_point(v).0;
        let via_mat3 = m3.transform(v);
        assert!(approx_eq_vec3(via_mat3, via_mat4));
    }

    #[test]
    fn test_mat3_inverse_identity() {
        let m = Mat3::identity();
        let inv = m.inverse();
        assert!((inv.m[0][0] - 1.0).abs() < 1e-5);
        assert!((inv.m[1][1] - 1.0).abs() < 1e-5);
        assert!((inv.m[2][2] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_mat3_inverse_transpose_matches_normal_transform() {
        // For a rotation matrix, inverse-transpose == original (orthonormal)
        let m = Mat3::rotation_z(0.7);
        let it = m.inverse_transpose();
        let v = Vec3::new(1.0, 0.5, 0.25).normalize();
        let via_m = m.transform(v);
        let via_it = it.transform(v);
        assert!(approx_eq_vec3(via_m, via_it));
    }

    #[test]
    fn test_mat3_mul() {
        let rx = Mat3::rotation_x(FRAC_PI_2);
        let ry = Mat3::rotation_y(FRAC_PI_2);
        let combined = rx * ry;
        let v = Vec3::X;
        let expected = ry.transform(rx.transform(v));
        let got = combined.transform(v);
        assert!(approx_eq_vec3(got, expected));
    }

    #[test]
    fn test_mat3_transpose() {
        let m = Mat3::rotation_y(0.5);
        let mt = m.transpose();
        // For rotation, transpose == inverse
        let v = Vec3::new(0.3, -0.7, 1.2);
        let rotated = m.transform(v);
        let restored = mt.transform(rotated);
        assert!(approx_eq_vec3(restored, v));
    }

    // ── Mat2 completions ─────────────────────────────────────────────────────

    #[test]
    fn mat2_identity_is_noop() {
        let i = Mat2::identity();
        let v = Vec2::new(3.0, -4.0);
        let out = i.transform(v);
        assert!((out.x - v.x).abs() < 1e-6);
        assert!((out.y - v.y).abs() < 1e-6);
    }

    #[test]
    fn mat2_determinant_rotation() {
        let m = Mat2::rotation(1.2);
        assert!((m.determinant() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn mat2_inverse_roundtrip() {
        let m = Mat2::scale(2.0, 3.0);
        let inv = m.inverse();
        let prod = m * inv;
        let id = Mat2::identity();
        for i in 0..2 {
            for j in 0..2 {
                assert!((prod.m[i][j] - id.m[i][j]).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn mat2_mul_rotations_compose() {
        let a = Mat2::rotation(0.4);
        let b = Mat2::rotation(-0.4);
        let prod = a * b;
        let id = Mat2::identity();
        for i in 0..2 {
            for j in 0..2 {
                assert!((prod.m[i][j] - id.m[i][j]).abs() < 1e-5, "[{i}][{j}]");
            }
        }
    }

    // ── Polynomial solvers ────────────────────────────────────────────────────

    #[test]
    fn quadratic_two_roots() {
        let (r, n) = quadratic_solve(1.0, -5.0, 6.0);
        assert_eq!(n, 2);
        assert!((r[0] - 2.0).abs() < 1e-5);
        assert!((r[1] - 3.0).abs() < 1e-5);
    }

    #[test]
    fn quadratic_no_real_roots() {
        let (_, n) = quadratic_solve(1.0, 0.0, 1.0);
        assert_eq!(n, 0);
    }

    #[test]
    fn quadratic_double_root() {
        let (r, n) = quadratic_solve(1.0, -4.0, 4.0);
        assert_eq!(n, 1);
        assert!((r[0] - 2.0).abs() < 1e-5);
    }

    #[test]
    fn cubic_three_roots() {
        let (r, n) = cubic_solve(1.0, -6.0, 11.0, -6.0);
        assert_eq!(n, 3);
        assert!((r[0] - 1.0).abs() < 1e-3, "r[0]={}", r[0]);
        assert!((r[1] - 2.0).abs() < 1e-3, "r[1]={}", r[1]);
        assert!((r[2] - 3.0).abs() < 1e-3, "r[2]={}", r[2]);
    }

    #[test]
    fn cubic_one_real_root() {
        let (r, n) = cubic_solve(1.0, 0.0, 0.0, -8.0);
        assert_eq!(n, 1);
        assert!((r[0] - 2.0).abs() < 1e-4, "r[0]={}", r[0]);
    }

    // ── Splines ───────────────────────────────────────────────────────────────

    #[test]
    fn kochanek_bartels_zero_params_matches_catmull_rom() {
        let (p0, p1, p2, p3) = (
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::ZERO,
            Vec3::ONE,
            Vec3::new(2.0, 1.0, 0.0),
        );
        let tcb = kochanek_bartels(p0, p1, p2, p3, 0.5, 0.0, 0.0, 0.0);
        let cr = catmull_rom(p0, p1, p2, p3, 0.5);
        assert!(
            (tcb.x - cr.x).abs() < 1e-5,
            "x mismatch: tcb={} cr={}",
            tcb.x,
            cr.x
        );
        assert!((tcb.y - cr.y).abs() < 1e-5);
    }

    #[test]
    fn bezier_split_midpoint_matches_curve() {
        let (p0, p1, p2, p3) = (
            Vec3::ZERO,
            Vec3::new(1.0, 2.0, 0.0),
            Vec3::new(2.0, 2.0, 0.0),
            Vec3::new(3.0, 0.0, 0.0),
        );
        let (left, right) = bezier_cubic_split(p0, p1, p2, p3, 0.5);
        let mid = bezier_cubic(p0, p1, p2, p3, 0.5);
        assert!((left[3].x - mid.x).abs() < 1e-5);
        assert!((right[0].x - mid.x).abs() < 1e-5);
    }

    #[test]
    fn bezier_split_endpoints_preserved() {
        let (p0, p1, p2, p3) = (
            Vec3::ZERO,
            Vec3::ONE,
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(3.0, 1.0, 0.0),
        );
        let (left, right) = bezier_cubic_split(p0, p1, p2, p3, 0.3);
        assert!((left[0].x - p0.x).abs() < 1e-5);
        assert!((right[3].x - p3.x).abs() < 1e-5);
    }

    // ── Vec3 min/max component ────────────────────────────────────────────────

    #[test]
    fn vec3_min_max_component() {
        let v = Vec3::new(3.0, 1.0, 2.0);
        assert_eq!(v.min_component(), 1.0);
        assert_eq!(v.max_component(), 3.0);
    }

    #[test]
    fn vec3_min_max_negative() {
        let v = Vec3::new(-1.0, -5.0, -2.0);
        assert_eq!(v.min_component(), -5.0);
        assert_eq!(v.max_component(), -1.0);
    }

    // ── Mat4::normal_matrix ──────────────────────────────────────────────────

    #[test]
    fn normal_matrix_pure_rotation_is_same() {
        let m = Mat4::rotation_y(0.7);
        let nm = m.normal_matrix();
        let upper = Mat3::from_mat4(&m);
        let n = Vec3::new(1.0, 0.0, 0.0).normalize();
        let by_nm = nm.transform(n);
        let by_upper = upper.transform(n);
        assert!(
            (by_nm.x - by_upper.x).abs() < 1e-4,
            "x: {} vs {}",
            by_nm.x,
            by_upper.x
        );
    }

    #[test]
    fn normal_matrix_non_uniform_scale_corrects() {
        // With non-uniform scale, normals transformed by model matrix get sheared;
        // normal_matrix corrects this.
        let model = Mat4::scale(2.0, 1.0, 1.0);
        let nm = model.normal_matrix();
        // The face normal (1,0,0) of an X-scaled box should still point in (1,0,0)
        let n = Vec3::new(1.0, 0.0, 0.0);
        let corrected = nm.transform(n).normalize();
        assert!((corrected.x - 1.0).abs() < 0.01, "x={}", corrected.x);
        assert!(corrected.y.abs() < 0.01);
    }

    // ── basis_from_normal ────────────────────────────────────────────────────

    #[test]
    fn basis_from_normal_orthonormal() {
        for n in [
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.577, 0.577, 0.577).normalize(),
        ] {
            let (t, b) = basis_from_normal(n);
            assert!(t.dot(n).abs() < 1e-4, "t⊥n failed for {:?}", n);
            assert!(b.dot(n).abs() < 1e-4, "b⊥n failed for {:?}", n);
            assert!(t.dot(b).abs() < 1e-4, "t⊥b failed for {:?}", n);
            assert!((t.length() - 1.0).abs() < 1e-4);
            assert!((b.length() - 1.0).abs() < 1e-4);
        }
    }

    // ── Vec2 min/max_component ───────────────────────────────────────────────

    #[test]
    fn vec2_min_max_component() {
        let v = Vec2::new(3.0, -1.0);
        assert_eq!(v.min_component(), -1.0);
        assert_eq!(v.max_component(), 3.0);
        let v = Vec2::new(5.0, 5.0);
        assert_eq!(v.min_component(), 5.0);
        assert_eq!(v.max_component(), 5.0);
    }

    // ── Vec3 pow/exp/log ─────────────────────────────────────────────────────

    #[test]
    fn vec3_pow_exp_log() {
        let v = Vec3::new(4.0, 9.0, 16.0);
        let sq = v.pow(0.5);
        assert!((sq.x - 2.0).abs() < 1e-5);
        assert!((sq.y - 3.0).abs() < 1e-5);
        assert!((sq.z - 4.0).abs() < 1e-5);
        let one = Vec3::ZERO.exp();
        assert!((one.x - 1.0).abs() < 1e-5);
        let e_val = Vec3::new(1.0_f32.exp(), 1.0, 1.0);
        let l = e_val.log();
        assert!((l.x - 1.0).abs() < 1e-5);
    }

    // ── Vec4 min/max_component / pow/exp/log ─────────────────────────────────

    #[test]
    fn vec4_component_ops() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(v.min_component(), 1.0);
        assert_eq!(v.max_component(), 4.0);
        let sq = v.pow(2.0);
        assert!((sq.x - 1.0).abs() < 1e-5);
        assert!((sq.w - 16.0).abs() < 1e-5);
    }

    // ── lerp_angle / angle_diff ──────────────────────────────────────────────

    #[test]
    fn lerp_angle_basic() {
        use std::f32::consts::PI;
        // Lerp from 0 to 90° — unambiguous short path
        let a = lerp_angle(0.0, PI / 2.0, 0.5);
        assert!((a - PI / 4.0).abs() < 1e-5, "expected π/4 got {a}");
    }

    #[test]
    fn lerp_angle_wraps() {
        // lerping from 350° to 10° should go through 0° (short path), not 180°
        let deg350 = 350.0_f32.to_radians();
        let deg10 = 10.0_f32.to_radians();
        let mid = lerp_angle(deg350, deg10, 0.5);
        let mid_deg = mid.to_degrees().rem_euclid(360.0);
        assert!(
            mid_deg < 20.0 || mid_deg > 340.0,
            "expected near 0°, got {mid_deg}°"
        );
    }

    #[test]
    fn angle_diff_sign() {
        use std::f32::consts::PI;
        let d = angle_diff(0.0, PI / 2.0);
        assert!((d - PI / 2.0).abs() < 1e-5);
        let d2 = angle_diff(PI / 2.0, 0.0);
        assert!((d2 + PI / 2.0).abs() < 1e-5);
    }

    // ── bspline_eval ─────────────────────────────────────────────────────────

    #[test]
    fn bspline_convex_hull() {
        let p0 = Vec3::new(0.0, 0.0, 0.0);
        let p1 = Vec3::new(0.0, 2.0, 0.0);
        let p2 = Vec3::new(2.0, 2.0, 0.0);
        let p3 = Vec3::new(2.0, 0.0, 0.0);
        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let p = bspline_eval(p0, p1, p2, p3, t);
            assert!(p.x >= -0.01 && p.x <= 2.01, "x out of hull: {}", p.x);
            assert!(p.y >= -0.01 && p.y <= 2.01, "y out of hull: {}", p.y);
        }
    }

    // ── bezier_cubic_arc_length ───────────────────────────────────────────────

    #[test]
    fn bezier_arc_length_straight_line() {
        let p0 = Vec3::new(0.0, 0.0, 0.0);
        let p1 = Vec3::new(1.0 / 3.0, 0.0, 0.0);
        let p2 = Vec3::new(2.0 / 3.0, 0.0, 0.0);
        let p3 = Vec3::new(1.0, 0.0, 0.0);
        let len = bezier_cubic_arc_length(p0, p1, p2, p3);
        assert!((len - 1.0).abs() < 1e-4, "straight line, got {len}");
    }

    #[test]
    fn bezier_arc_length_positive() {
        let p0 = Vec3::ZERO;
        let p1 = Vec3::new(0.0, 1.0, 0.0);
        let p2 = Vec3::new(1.0, 1.0, 0.0);
        let p3 = Vec3::new(1.0, 0.0, 0.0);
        let len = bezier_cubic_arc_length(p0, p1, p2, p3);
        assert!(len > 0.0);
        // Arc length must be >= straight-line distance
        assert!(len >= (p3 - p0).length());
    }

    // ── spherical / cylindrical coordinates ──────────────────────────────────

    #[test]
    fn spherical_round_trip() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        let (r, theta, phi) = cartesian_to_spherical(v);
        let v2 = spherical_to_cartesian(r, theta, phi);
        assert!((v2 - v).length() < 1e-5, "round trip: {v2:?} vs {v:?}");
    }

    #[test]
    fn spherical_poles() {
        // +Y pole: theta = 0
        let (r, theta, _phi) = cartesian_to_spherical(Vec3::new(0.0, 5.0, 0.0));
        assert!((r - 5.0).abs() < 1e-5);
        assert!(theta.abs() < 1e-5);
        // -Y pole: theta = pi
        let (r2, theta2, _) = cartesian_to_spherical(Vec3::new(0.0, -3.0, 0.0));
        assert!((r2 - 3.0).abs() < 1e-5);
        assert!((theta2 - std::f32::consts::PI).abs() < 1e-5);
    }

    #[test]
    fn spherical_equator() {
        // Point on equator (+X axis): theta = pi/2, phi = 0
        let (r, theta, phi) = cartesian_to_spherical(Vec3::new(2.0, 0.0, 0.0));
        assert!((r - 2.0).abs() < 1e-5);
        assert!((theta - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        assert!(phi.abs() < 1e-5 || (phi - std::f32::consts::TAU).abs() < 1e-5);
    }

    #[test]
    fn cylindrical_round_trip() {
        let v = Vec3::new(1.5, 4.0, -2.0);
        let (r, theta, y) = cartesian_to_cylindrical(v);
        let v2 = cylindrical_to_cartesian(r, theta, y);
        assert!((v2 - v).length() < 1e-5, "round trip: {v2:?} vs {v:?}");
    }

    #[test]
    fn cylindrical_y_preserved() {
        let v = Vec3::new(3.0, 7.0, 4.0);
        let (r, theta, y) = cartesian_to_cylindrical(v);
        assert!((y - 7.0).abs() < 1e-5);
        assert!((r - 5.0).abs() < 1e-5); // sqrt(9+16)
        let v2 = cylindrical_to_cartesian(r, theta, y);
        assert!((v2.y - 7.0).abs() < 1e-5);
    }

    // ── Mat4::shear ──────────────────────────────────────────────────────────

    #[test]
    fn shear_x_by_y() {
        let m = Mat4::shear(0.5, 0.0, 0.0, 0.0, 0.0, 0.0);
        let v = Vec3::new(0.0, 2.0, 0.0);
        let (result, _) = m.transform_point(v);
        assert!(
            (result.x - 1.0).abs() < 1e-5,
            "x = 0 + 0.5*2 = 1, got {}",
            result.x
        );
        assert!((result.y - 2.0).abs() < 1e-5);
        assert!((result.z).abs() < 1e-5);
    }

    #[test]
    fn shear_identity_zero_params() {
        let m = Mat4::shear(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let v = Vec3::new(3.0, -1.0, 2.0);
        let (result, _) = m.transform_point(v);
        assert!((result - v).length() < 1e-5);
    }

    #[test]
    fn shear_y_by_x() {
        let m = Mat4::shear(0.0, 0.0, 2.0, 0.0, 0.0, 0.0); // yx = 2
        let v = Vec3::new(1.0, 0.0, 0.0);
        let (result, _) = m.transform_point(v);
        assert!(
            (result.y - 2.0).abs() < 1e-5,
            "y = 0 + 2*1 = 2, got {}",
            result.y
        );
        assert!((result.x - 1.0).abs() < 1e-5);
    }

    // ── Mat4::reflect_plane ──────────────────────────────────────────────────

    #[test]
    fn reflect_through_origin_xz_plane() {
        // Reflect through XZ plane (normal = +Y, point = origin)
        let m = Mat4::reflect_plane(Vec3::Y, Vec3::ZERO);
        let v = Vec3::new(1.0, 3.0, 2.0);
        let (result, _) = m.transform_point(v);
        assert!((result.x - 1.0).abs() < 1e-5);
        assert!((result.y + 3.0).abs() < 1e-5, "y flipped: {}", result.y);
        assert!((result.z - 2.0).abs() < 1e-5);
    }

    #[test]
    fn reflect_idempotent() {
        // Reflecting twice returns original point
        let n = Vec3::new(1.0, 1.0, 0.0).normalize();
        let m = Mat4::reflect_plane(n, Vec3::new(1.0, 0.0, 0.0));
        let v = Vec3::new(2.0, 3.0, 1.0);
        let (once, _) = m.transform_point(v);
        let (twice, _) = m.transform_point(once);
        assert!(
            (twice - v).length() < 1e-4,
            "double reflect = identity: {twice:?}"
        );
    }

    #[test]
    fn reflect_point_on_plane_unchanged() {
        // A point on the reflection plane should be unchanged
        let n = Vec3::Y;
        let p_on_plane = Vec3::new(3.0, 0.0, -1.0); // y = 0 plane
        let m = Mat4::reflect_plane(n, Vec3::ZERO);
        let (result, _) = m.transform_point(p_on_plane);
        assert!((result - p_on_plane).length() < 1e-5);
    }
}
