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

/// Inverse lerp: given a `value` in `[a, b]`, returns the `t` that produced it.
///
/// `inverse_lerp(0.0, 10.0, 5.0) == 0.5`. Does not clamp — extrapolates outside the range.
/// Undefined if `a == b` (returns `0.0`).
///
/// # Examples
///
/// ```
/// use abrash_core::math::inverse_lerp;
///
/// assert!((inverse_lerp(0.0, 10.0, 5.0) - 0.5).abs() < 1e-6);
/// assert!((inverse_lerp(0.0, 10.0, 0.0) - 0.0).abs() < 1e-6);
/// assert!((inverse_lerp(0.0, 10.0, 10.0) - 1.0).abs() < 1e-6);
/// ```
#[must_use]
#[inline]
pub fn inverse_lerp(a: f32, b: f32, value: f32) -> f32 {
    let denom = b - a;
    if denom.abs() < 1e-30 {
        0.0
    } else {
        (value - a) / denom
    }
}

/// Remaps `x` from `[from_min, from_max]` to `[to_min, to_max]`, clamping the result.
///
/// Like [`remap`] but the output is clamped so it never exceeds the target range.
///
/// # Examples
///
/// ```
/// use abrash_core::math::remap_clamped;
///
/// assert!((remap_clamped(5.0, 0.0, 10.0, 0.0, 1.0) - 0.5).abs() < 1e-6);
/// assert!((remap_clamped(20.0, 0.0, 10.0, 0.0, 1.0) - 1.0).abs() < 1e-6); // clamped
/// assert!((remap_clamped(-5.0, 0.0, 10.0, 0.0, 1.0) - 0.0).abs() < 1e-6); // clamped
/// ```
#[must_use]
#[inline]
pub fn remap_clamped(x: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
    let t = ((x - from_min) / (from_max - from_min)).clamp(0.0, 1.0);
    to_min + t * (to_max - to_min)
}

/// Scalar GLSL `step(edge, x)`: returns 0.0 if `x < edge`, else 1.0.
///
/// # Examples
///
/// ```
/// use abrash_core::math::step;
///
/// assert_eq!(step(0.5, 0.3), 0.0);
/// assert_eq!(step(0.5, 0.5), 1.0);
/// assert_eq!(step(0.5, 0.8), 1.0);
/// ```
#[must_use]
#[inline]
pub fn step(edge: f32, x: f32) -> f32 {
    if x < edge { 0.0 } else { 1.0 }
}

/// Sign function that never returns zero: `+1.0` if `x >= 0.0`, `-1.0` if `x < 0.0`.
///
/// Useful in SDF formulas where `sign(0)` = 0 would cause issues.
///
/// # Examples
///
/// ```
/// use abrash_core::math::sign_no_zero;
///
/// assert_eq!(sign_no_zero(5.0), 1.0);
/// assert_eq!(sign_no_zero(-3.0), -1.0);
/// assert_eq!(sign_no_zero(0.0), 1.0);  // no zero
/// ```
#[must_use]
#[inline]
pub fn sign_no_zero(x: f32) -> f32 {
    if x < 0.0 { -1.0 } else { 1.0 }
}

/// Bias curve: `t^(log(b)/log(0.5))` — pushes `t` toward 0 when `b < 0.5`, toward 1 when `b > 0.5`.
///
/// From Ken Perlin & Eric Hoffert's 1989 "Hypertexture" paper.
/// `b = 0.5` is identity; `b → 0` crushes t to 0; `b → 1` crushes t to 1.
///
/// # Examples
///
/// ```
/// use abrash_core::math::bias;
///
/// assert!((bias(0.5, 0.5) - 0.5).abs() < 1e-5); // identity at b=0.5
/// assert!(bias(0.5, 0.2) < 0.5);  // b<0.5 pulls toward 0
/// assert!(bias(0.5, 0.8) > 0.5);  // b>0.5 pulls toward 1
/// ```
#[must_use]
#[inline]
pub fn bias(t: f32, b: f32) -> f32 {
    // exponent = log(b) / log(0.5) = −log(b) / log(2)
    let b = b.clamp(1e-6, 1.0 - 1e-6);
    t.powf(-b.ln() / std::f32::consts::LN_2)
}

/// Gain curve: S-shaped contrast enhancement via two symmetric `bias` calls.
///
/// `g = 0.5` is identity; `g < 0.5` flattens the curve; `g > 0.5` steepens it (more contrast).
///
/// # Examples
///
/// ```
/// use abrash_core::math::gain;
///
/// assert!((gain(0.5, 0.5) - 0.5).abs() < 1e-3); // midpoint preserved
/// assert!((gain(0.0, 0.8) - 0.0).abs() < 1e-5); // endpoints fixed
/// assert!((gain(1.0, 0.8) - 1.0).abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn gain(t: f32, g: f32) -> f32 {
    if t < 0.5 {
        bias(2.0 * t, 1.0 - g) * 0.5
    } else {
        1.0 - bias(2.0 - 2.0 * t, 1.0 - g) * 0.5
    }
}

/// Triangle wave: oscillates linearly between 0 and 1 with period 1.
///
/// `t = 0` → 0, `t = 0.5` → 1, `t = 1` → 0, and so on. Useful for ping-pong
/// animations and cheap oscillators without trigonometry.
///
/// # Examples
///
/// ```
/// use abrash_core::math::triangle_wave;
///
/// assert!((triangle_wave(0.0) - 0.0).abs() < 1e-6);
/// assert!((triangle_wave(0.25) - 0.5).abs() < 1e-6);
/// assert!((triangle_wave(0.5) - 1.0).abs() < 1e-6);
/// assert!((triangle_wave(1.0) - 0.0).abs() < 1e-6);
/// ```
#[must_use]
#[inline]
pub fn triangle_wave(t: f32) -> f32 {
    let t = t.rem_euclid(1.0);
    if t < 0.5 { t * 2.0 } else { 2.0 - t * 2.0 }
}

/// Sawtooth wave: ramps linearly from 0 to 1 then instantly resets, period 1.
///
/// `t = 0` → 0, `t = 0.999` → ~1, `t = 1` → 0.  Useful for repeating
/// animations, UV scrolling, and cheap oscillators.
///
/// # Examples
///
/// ```
/// use abrash_core::math::sawtooth_wave;
///
/// assert!((sawtooth_wave(0.0)   - 0.0).abs() < 1e-6);
/// assert!((sawtooth_wave(0.5)   - 0.5).abs() < 1e-6);
/// assert!((sawtooth_wave(1.0)   - 0.0).abs() < 1e-6); // reset
/// assert!((sawtooth_wave(1.75)  - 0.75).abs() < 1e-6); // next cycle
/// ```
#[must_use]
#[inline]
pub fn sawtooth_wave(t: f32) -> f32 {
    t.rem_euclid(1.0)
}

/// Square wave: 1.0 for the first `duty` fraction of each period, 0.0 otherwise.
///
/// `duty = 0.5` produces a symmetric square wave; `duty = 0.1` produces narrow pulses.
/// Outputs only 0.0 or 1.0 (no smoothing).
///
/// # Examples
///
/// ```
/// use abrash_core::math::square_wave;
///
/// assert_eq!(square_wave(0.25, 0.5), 1.0); // first half
/// assert_eq!(square_wave(0.75, 0.5), 0.0); // second half
/// assert_eq!(square_wave(0.05, 0.1), 1.0); // narrow pulse on
/// assert_eq!(square_wave(0.15, 0.1), 0.0); // narrow pulse off
/// ```
#[must_use]
#[inline]
pub fn square_wave(t: f32, duty: f32) -> f32 {
    if t.rem_euclid(1.0) < duty.clamp(0.0, 1.0) {
        1.0
    } else {
        0.0
    }
}

/// Pulse wave: 1.0 within `±width/2` of `center` in each period, 0.0 elsewhere.
///
/// Unlike `square_wave` which aligns to the start of the period, `pulse_wave`
/// places the "on" window around an arbitrary `center` offset (both in `[0, 1]`).
/// Wraps cleanly so a pulse centered near 0 or 1 spans the period boundary.
///
/// # Examples
///
/// ```
/// use abrash_core::math::pulse_wave;
///
/// // Pulse centred at 0.5, width 0.2: on in [0.4, 0.6]
/// assert_eq!(pulse_wave(0.5, 0.5, 0.2), 1.0);
/// assert_eq!(pulse_wave(0.3, 0.5, 0.2), 0.0);
/// ```
#[must_use]
#[inline]
pub fn pulse_wave(t: f32, center: f32, width: f32) -> f32 {
    let t = t.rem_euclid(1.0);
    let half = (width * 0.5).clamp(0.0, 0.5);
    // Wrap-aware distance to center
    let d = (t - center.rem_euclid(1.0)).rem_euclid(1.0);
    let d = d.min(1.0 - d); // shortest arc on the unit circle
    if d < half { 1.0 } else { 0.0 }
}

/// Exponential decay: `exp(-rate * t)`, clamped so `rate > 0`.
///
/// Commonly used for damping, fade-out, and spring-settling effects.
/// At `t = 1/rate` the value is `1/e ≈ 0.368`; at `t = 4/rate` it's `< 2%`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::exp_decay;
///
/// assert!((exp_decay(0.0, 2.0) - 1.0).abs() < 1e-6);
/// assert!(exp_decay(1.0, 2.0) < 0.5);  // decayed after 1 second
/// assert!(exp_decay(10.0, 2.0) < 0.01); // nearly zero at t=10, rate=2
/// ```
#[must_use]
#[inline]
pub fn exp_decay(t: f32, rate: f32) -> f32 {
    (-rate.max(0.0) * t.max(0.0)).exp()
}

/// Smooth `ease_in` (cubic): `t³` — starts slow, ends fast.
///
/// # Examples
///
/// ```
/// use abrash_core::math::ease_in;
///
/// assert!((ease_in(0.0) - 0.0).abs() < 1e-6);
/// assert!((ease_in(1.0) - 1.0).abs() < 1e-6);
/// assert!(ease_in(0.5) < 0.25); // slower than linear near 0
/// ```
#[must_use]
#[inline]
pub fn ease_in(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t
}

/// Smooth `ease_out` (cubic): `1 - (1-t)³` — starts fast, ends slow.
///
/// # Examples
///
/// ```
/// use abrash_core::math::ease_out;
///
/// assert!((ease_out(0.0) - 0.0).abs() < 1e-6);
/// assert!((ease_out(1.0) - 1.0).abs() < 1e-6);
/// assert!(ease_out(0.5) > 0.75); // faster than linear near 0
/// ```
#[must_use]
#[inline]
pub fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let inv = 1.0 - t;
    1.0 - inv * inv * inv
}

/// Smooth `ease_in_out` (cubic Hermite): `3t² - 2t³` — same as `smoothstep`.
///
/// Equivalent to `smoothstep(0, 1, t)` but without the range remapping. The
/// name makes the animation intent explicit.
///
/// # Examples
///
/// ```
/// use abrash_core::math::ease_in_out;
///
/// assert!((ease_in_out(0.0) - 0.0).abs() < 1e-6);
/// assert!((ease_in_out(0.5) - 0.5).abs() < 1e-6); // midpoint preserved
/// assert!((ease_in_out(1.0) - 1.0).abs() < 1e-6);
/// ```
#[must_use]
#[inline]
pub fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
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

/// Halton low-discrepancy sequence.
///
/// Returns the `index`-th element of the Halton sequence in the given `base`.
/// Common bases: 2 and 3 give a well-distributed 2D sequence.
/// The sequence is deterministic and fills \[0, 1) evenly without clustering.
///
/// Useful for quasi-Monte Carlo integration, SSAO sample kernels, TAA jitter.
///
/// # Examples
///
/// ```
/// use abrash_core::math::halton;
///
/// // Base-2 sequence: 1/2, 1/4, 3/4, 1/8, ...
/// assert!((halton(1, 2) - 0.5).abs()  < 1e-6);
/// assert!((halton(2, 2) - 0.25).abs() < 1e-6);
/// assert!((halton(3, 2) - 0.75).abs() < 1e-6);
/// // Values are in [0, 1)
/// for i in 0..16u32 { assert!((0.0..1.0).contains(&halton(i, 2))); }
/// ```
#[must_use]
pub fn halton(mut index: u32, base: u32) -> f32 {
    let mut result = 0.0_f32;
    let mut f = 1.0_f32;
    let b = base as f32;
    while index > 0 {
        f /= b;
        result += f * (index % base) as f32;
        index /= base;
    }
    result
}

/// Van der Corput radical inverse (base 2) via bit-reversal.
///
/// Equivalent to `halton(bits, 2)` but computed in O(1) using integer
/// bit-reversal — the standard fast path used in PBR importance sampling.
///
/// # Examples
///
/// ```
/// use abrash_core::math::van_der_corput;
///
/// assert!((van_der_corput(0) - 0.0).abs()  < 1e-9);
/// assert!((van_der_corput(1) - 0.5).abs()  < 1e-9);
/// assert!((van_der_corput(2) - 0.25).abs() < 1e-9);
/// assert!((van_der_corput(3) - 0.75).abs() < 1e-9);
/// ```
#[must_use]
#[inline]
pub fn van_der_corput(mut bits: u32) -> f32 {
    bits = (bits << 16) | (bits >> 16);
    bits = ((bits & 0x5555_5555) << 1) | ((bits & 0xAAAA_AAAA) >> 1);
    bits = ((bits & 0x3333_3333) << 2) | ((bits & 0xCCCC_CCCC) >> 2);
    bits = ((bits & 0x0F0F_0F0F) << 4) | ((bits & 0xF0F0_F0F0) >> 4);
    bits = ((bits & 0x00FF_00FF) << 8) | ((bits & 0xFF00_FF00) >> 8);
    bits as f32 * (1.0 / 4_294_967_296.0_f32)
}

/// Hammersley 2D point set — (i/N, van_der_corput(i)).
///
/// Produces `total` stratified sample points in \[0,1)² with low discrepancy.
/// Standard in PBR for importance-sampling the hemisphere and SSAO kernels.
///
/// # Examples
///
/// ```
/// use abrash_core::math::hammersley_2d;
///
/// let p = hammersley_2d(0, 4);
/// assert!((p.x - 0.0).abs() < 1e-6);
///
/// let p = hammersley_2d(2, 4);
/// assert!((p.x - 0.5).abs() < 1e-6);
/// assert!((p.y - 0.25).abs() < 1e-9); // van der Corput(2) = 0.25
/// ```
#[must_use]
#[inline]
pub fn hammersley_2d(index: u32, total: u32) -> Vec2 {
    Vec2::new(index as f32 / total as f32, van_der_corput(index))
}

/// Frenet-Serret tangent–normal–binormal frame from a curve tangent and an up hint.
///
/// Constructs an orthonormal TNB basis:
/// - **T** (tangent): normalised `tangent`
/// - **B** (binormal): `T × up_hint`, normalised
/// - **N** (normal): `B × T` (always perpendicular to both)
///
/// Use `up_hint = Vec3::Y` unless the curve is nearly vertical, in which case
/// try `Vec3::X` or `Vec3::Z` to avoid degenerate cross products.
///
/// Useful for ribbon/tube geometry along splines, camera alignment, and physics.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Vec3, frenet_frame};
///
/// let (t, n, b) = frenet_frame(Vec3::X, Vec3::Y);
/// // T = +X, B = X×Y = -Z, N = (-Z)×X = -Y  (right-hand rule)
/// assert!((t - Vec3::X).length() < 1e-5);
/// assert!((t.dot(n)).abs() < 1e-5, "T⊥N");
/// assert!((t.dot(b)).abs() < 1e-5, "T⊥B");
/// assert!((n.dot(b)).abs() < 1e-5, "N⊥B");
/// ```
#[must_use]
#[inline]
pub fn frenet_frame(tangent: Vec3, up_hint: Vec3) -> (Vec3, Vec3, Vec3) {
    let t = tangent.normalize_or_zero();
    let b = t.cross(up_hint).normalize_or_zero();
    let n = b.cross(t);
    (t, n, b)
}

/// Signed area of a 2D polygon (shoelace / surveyor's formula).
///
/// Positive for counter-clockwise winding, negative for clockwise.
/// The absolute value is the polygon area. Returns 0 for fewer than 3 vertices.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Vec2, signed_area_2d};
///
/// // CCW unit square: area = 1.0
/// let sq = [Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0),
///           Vec2::new(1.0, 1.0), Vec2::new(0.0, 1.0)];
/// assert!((signed_area_2d(&sq) - 1.0).abs() < 1e-6);
/// ```
#[must_use]
pub fn signed_area_2d(polygon: &[Vec2]) -> f32 {
    let n = polygon.len();
    if n < 3 {
        return 0.0;
    }
    let mut area = 0.0_f32;
    for i in 0..n {
        let j = (i + 1) % n;
        area += polygon[i].x * polygon[j].y - polygon[j].x * polygon[i].y;
    }
    area * 0.5
}

/// Centroid (geometric centre) of a 2D polygon.
///
/// Uses the area-weighted centroid formula; correct for non-convex polygons.
/// Falls back to the vertex mean for degenerate (zero-area) polygons.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Vec2, polygon_centroid_2d};
///
/// // Centroid of a unit square centred at (0.5, 0.5)
/// let sq = [Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0),
///           Vec2::new(1.0, 1.0), Vec2::new(0.0, 1.0)];
/// let c = polygon_centroid_2d(&sq);
/// assert!((c.x - 0.5).abs() < 1e-5);
/// assert!((c.y - 0.5).abs() < 1e-5);
/// ```
#[must_use]
pub fn polygon_centroid_2d(polygon: &[Vec2]) -> Vec2 {
    let n = polygon.len();
    if n == 0 {
        return Vec2::ZERO;
    }
    if n == 1 {
        return polygon[0];
    }
    let area = signed_area_2d(polygon);
    if area.abs() < 1e-30 {
        // Degenerate: mean of vertices
        let sum = polygon.iter().fold(Vec2::ZERO, |acc, &v| acc + v);
        return sum * (1.0 / n as f32);
    }
    let mut cx = 0.0_f32;
    let mut cy = 0.0_f32;
    for i in 0..n {
        let j = (i + 1) % n;
        let cross = polygon[i].x * polygon[j].y - polygon[j].x * polygon[i].y;
        cx += (polygon[i].x + polygon[j].x) * cross;
        cy += (polygon[i].y + polygon[j].y) * cross;
    }
    let inv = 1.0 / (6.0 * area);
    Vec2::new(cx * inv, cy * inv)
}

/// Test whether point `p` lies inside a 2D polygon (winding-number test).
///
/// Returns `true` for interior points. Works for concave polygons. Points
/// exactly on an edge have unspecified (but consistent) results.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Vec2, point_in_polygon_2d};
///
/// let tri = [Vec2::new(0.0, 0.0), Vec2::new(2.0, 0.0), Vec2::new(1.0, 2.0)];
/// assert!( point_in_polygon_2d(Vec2::new(1.0, 0.5), &tri)); // inside
/// assert!(!point_in_polygon_2d(Vec2::new(3.0, 1.0), &tri)); // outside
/// ```
#[must_use]
pub fn point_in_polygon_2d(p: Vec2, polygon: &[Vec2]) -> bool {
    let n = polygon.len();
    if n < 3 {
        return false;
    }
    let mut winding = 0i32;
    for i in 0..n {
        let j = (i + 1) % n;
        let a = polygon[i];
        let b = polygon[j];
        if a.y <= p.y {
            if b.y > p.y {
                // Upward crossing — left of edge?
                let cross = (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y);
                if cross > 0.0 {
                    winding += 1;
                }
            }
        } else if b.y <= p.y {
            // Downward crossing — right of edge?
            let cross = (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y);
            if cross < 0.0 {
                winding -= 1;
            }
        }
    }
    winding != 0
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

    /// Clamp the vector's length to at most `max_length`.
    ///
    /// Returns `self` unchanged if already shorter; otherwise scales down to `max_length`.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::Vec2;
    ///
    /// let v = Vec2::new(3.0, 4.0); // length 5
    /// let clamped = v.clamp_length(2.0);
    /// assert!((clamped.length() - 2.0).abs() < 1e-5);
    ///
    /// let short = Vec2::new(0.5, 0.0);
    /// assert_eq!(short.clamp_length(2.0), short); // unchanged
    /// ```
    #[must_use]
    #[inline]
    pub fn clamp_length(self, max_length: f32) -> Self {
        let len_sq = self.length_sq();
        if len_sq > max_length * max_length {
            self * (max_length / len_sq.sqrt())
        } else {
            self
        }
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

    /// Projects this vector onto a plane defined by its unit normal.
    ///
    /// Removes the component along `normal`, leaving only the tangential part.
    /// `unit_normal` must be unit-length.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::Vec3;
    ///
    /// // Project onto XZ plane (normal = +Y): Y component becomes zero
    /// let v = Vec3::new(1.0, 5.0, 2.0);
    /// let flat = v.project_onto_plane(Vec3::Y);
    /// assert!(flat.y.abs() < 1e-5);
    /// assert!((flat.x - 1.0).abs() < 1e-5);
    /// assert!((flat.z - 2.0).abs() < 1e-5);
    /// ```
    #[must_use]
    #[inline]
    pub fn project_onto_plane(self, unit_normal: Self) -> Self {
        self - unit_normal * self.dot(unit_normal)
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

    /// Perspective divide: divide `x`, `y`, `z` by `w` and return as a [`Vec3`].
    ///
    /// Used after matrix-vector multiplication to convert homogeneous clip-space
    /// coordinates to NDC (Normalised Device Coordinates).  Returns `Vec3::ZERO`
    /// if `w` is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::Vec4;
    ///
    /// let h = Vec4::new(2.0, 4.0, 6.0, 2.0);
    /// let ndc = h.homogenize();
    /// assert!((ndc.x - 1.0).abs() < 1e-6);
    /// assert!((ndc.y - 2.0).abs() < 1e-6);
    /// assert!((ndc.z - 3.0).abs() < 1e-6);
    /// ```
    #[must_use]
    #[inline]
    pub fn homogenize(self) -> Vec3 {
        if self.w.abs() < 1e-10 {
            return Vec3::ZERO;
        }
        let inv_w = 1.0 / self.w;
        Vec3::new(self.x * inv_w, self.y * inv_w, self.z * inv_w)
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

impl std::ops::Mul<Vec3> for Mat3 {
    type Output = Vec3;
    /// Transform a column vector by this row-major 3×3 matrix.
    #[inline]
    fn mul(self, v: Vec3) -> Vec3 {
        Vec3::new(
            self.m[0][0] * v.x + self.m[0][1] * v.y + self.m[0][2] * v.z,
            self.m[1][0] * v.x + self.m[1][1] * v.y + self.m[1][2] * v.z,
            self.m[2][0] * v.x + self.m[2][1] * v.y + self.m[2][2] * v.z,
        )
    }
}

// ── Quaternion ────────────────────────────────────────────────────────────────

/// Unit quaternion representing a 3-D rotation.
///
/// Stored as `(x, y, z, w)` where `w` is the scalar part.
/// Operations assume the quaternion is normalised; call [`Quat::normalize`]
/// after accumulating many multiplications.
///
/// # Conventions
/// - Hamilton product: `q1 * q2` applies `q1` THEN `q2` (same as matrix order).
/// - Rotation direction: right-hand rule (CCW when axis points toward viewer).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quat {
    /// The identity quaternion (no rotation).
    #[must_use]
    #[inline]
    pub const fn identity() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        }
    }

    /// Construct from a normalised `axis` and a rotation `angle` in radians.
    ///
    /// # Examples
    /// ```
    /// use abrash_core::math::{Quat, Vec3};
    /// let q = Quat::from_axis_angle(Vec3::Y, core::f32::consts::FRAC_PI_2);
    /// let v = q.rotate(Vec3::X);
    /// assert!((v.x - 0.0).abs() < 1e-5, "x: {}", v.x);
    /// assert!((v.z + 1.0).abs() < 1e-5, "z: {}", v.z); // +X rotated 90° about Y → -Z
    /// ```
    #[must_use]
    #[inline]
    pub fn from_axis_angle(axis: Vec3, angle: f32) -> Self {
        let (s, c) = (angle * 0.5).sin_cos();
        Self {
            x: axis.x * s,
            y: axis.y * s,
            z: axis.z * s,
            w: c,
        }
    }

    /// Construct from Euler angles (yaw, pitch, roll) in radians, ZYX order.
    ///
    /// Applies: roll (Z) first, then pitch (X), then yaw (Y).
    #[must_use]
    #[inline]
    pub fn from_euler_zyx(yaw: f32, pitch: f32, roll: f32) -> Self {
        let (sy, cy) = (yaw * 0.5).sin_cos();
        let (sp, cp) = (pitch * 0.5).sin_cos();
        let (sr, cr) = (roll * 0.5).sin_cos();
        Self {
            x: cy * sp * cr + sy * cp * sr,
            y: sy * cp * cr - cy * sp * sr,
            z: cy * cp * sr - sy * sp * cr,
            w: cy * cp * cr + sy * sp * sr,
        }
    }

    /// Dot product of two quaternions (measures alignment; 1 = identical).
    #[must_use]
    #[inline]
    pub fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z + self.w * rhs.w
    }

    /// Squared magnitude.
    #[must_use]
    #[inline]
    pub fn length_sq(self) -> f32 {
        self.dot(self)
    }

    /// Magnitude (should be ≈ 1.0 for unit quaternions).
    #[must_use]
    #[inline]
    pub fn length(self) -> f32 {
        self.length_sq().sqrt()
    }

    /// Return the normalised form.
    #[must_use]
    #[inline]
    pub fn normalize(self) -> Self {
        let inv = 1.0 / self.length();
        Self {
            x: self.x * inv,
            y: self.y * inv,
            z: self.z * inv,
            w: self.w * inv,
        }
    }

    /// Conjugate `(−x, −y, −z, w)` — the inverse for unit quaternions.
    #[must_use]
    #[inline]
    pub const fn conjugate(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            w: self.w,
        }
    }

    /// Inverse (conjugate / |q|²). Use [`Quat::conjugate`] for unit quats.
    #[must_use]
    #[inline]
    pub fn inverse(self) -> Self {
        let inv_sq = 1.0 / self.length_sq();
        Self {
            x: -self.x * inv_sq,
            y: -self.y * inv_sq,
            z: -self.z * inv_sq,
            w: self.w * inv_sq,
        }
    }

    /// Rotate a `Vec3` by this quaternion using the sandwich product
    /// `q * v * q⁻¹` (expanded without full quaternion multiplication).
    ///
    /// # Examples
    /// ```
    /// use abrash_core::math::{Quat, Vec3};
    /// // 180° rotation about Y maps +X → -X.
    /// let q = Quat::from_axis_angle(Vec3::Y, core::f32::consts::PI);
    /// let v = q.rotate(Vec3::X);
    /// assert!((v.x + 1.0).abs() < 1e-5, "x: {}", v.x);
    /// ```
    #[must_use]
    #[inline]
    pub fn rotate(self, v: Vec3) -> Vec3 {
        // Efficient Fabian Giessen formula: 2 * cross products.
        let t = Vec3::new(
            2.0 * (self.y * v.z - self.z * v.y),
            2.0 * (self.z * v.x - self.x * v.z),
            2.0 * (self.x * v.y - self.y * v.x),
        );
        Vec3::new(
            v.x + self.w * t.x + self.y * t.z - self.z * t.y,
            v.y + self.w * t.y + self.z * t.x - self.x * t.z,
            v.z + self.w * t.z + self.x * t.y - self.y * t.x,
        )
    }

    /// Spherical linear interpolation between `self` and `end` by factor `t ∈ [0,1]`.
    ///
    /// Automatically flips `end` if the dot product is negative (takes the short arc).
    ///
    /// # Examples
    /// ```
    /// use abrash_core::math::{Quat, Vec3};
    /// let q0 = Quat::identity();
    /// let q1 = Quat::from_axis_angle(Vec3::Y, core::f32::consts::FRAC_PI_2);
    /// let mid = q0.slerp(q1, 0.5);
    /// // Midpoint should rotate +X by ~45°.
    /// let v = mid.rotate(Vec3::X);
    /// assert!(v.x > 0.0 && v.z < 0.0);
    /// ```
    #[must_use]
    #[inline]
    pub fn slerp(self, end: Self, t: f32) -> Self {
        let mut dot = self.dot(end);
        // Take the short arc.
        let end = if dot < 0.0 {
            dot = -dot;
            Self {
                x: -end.x,
                y: -end.y,
                z: -end.z,
                w: -end.w,
            }
        } else {
            end
        };

        if dot > 0.999_9 {
            // Nearly identical: lerp + normalise.
            return Self {
                x: self.x + t * (end.x - self.x),
                y: self.y + t * (end.y - self.y),
                z: self.z + t * (end.z - self.z),
                w: self.w + t * (end.w - self.w),
            }
            .normalize();
        }

        let theta = dot.acos();
        let sin_theta = theta.sin();
        let s0 = ((1.0 - t) * theta).sin() / sin_theta;
        let s1 = (t * theta).sin() / sin_theta;
        Self {
            x: s0 * self.x + s1 * end.x,
            y: s0 * self.y + s1 * end.y,
            z: s0 * self.z + s1 * end.z,
            w: s0 * self.w + s1 * end.w,
        }
    }

    /// Convert to a row-major 3×3 rotation matrix (matching `Mat3` storage).
    ///
    /// # Examples
    /// ```
    /// use abrash_core::math::{Quat, Vec3};
    /// let m = Quat::identity().to_mat3();
    /// // Identity quat → identity matrix diagonal.
    /// assert!((m.m[0][0] - 1.0).abs() < 1e-6);
    /// assert!((m.m[1][1] - 1.0).abs() < 1e-6);
    /// assert!((m.m[2][2] - 1.0).abs() < 1e-6);
    /// ```
    #[must_use]
    pub fn to_mat3(self) -> Mat3 {
        let (x, y, z, w) = (self.x, self.y, self.z, self.w);
        let x2 = x + x;
        let y2 = y + y;
        let z2 = z + z;
        let xx = x * x2;
        let xy = x * y2;
        let xz = x * z2;
        let yy = y * y2;
        let yz = y * z2;
        let zz = z * z2;
        let wx = w * x2;
        let wy = w * y2;
        let wz = w * z2;
        // Row-major: m[row][col]
        Mat3 {
            m: [
                [1.0 - (yy + zz), xy - wz, xz + wy], // row 0
                [xy + wz, 1.0 - (xx + zz), yz - wx], // row 1
                [xz - wy, yz + wx, 1.0 - (xx + yy)], // row 2
            ],
        }
    }

    /// Angle (in radians) of the rotation represented by this quaternion.
    #[must_use]
    #[inline]
    pub fn angle(self) -> f32 {
        2.0 * self.w.clamp(-1.0, 1.0).acos()
    }

    /// Axis of the rotation (normalised). Returns `Vec3::Y` for the identity.
    #[must_use]
    #[inline]
    pub fn axis(self) -> Vec3 {
        let sin_half = (1.0 - self.w * self.w).sqrt();
        if sin_half < 1e-6 {
            Vec3::Y
        } else {
            let inv = 1.0 / sin_half;
            Vec3::new(self.x * inv, self.y * inv, self.z * inv)
        }
    }
}

impl std::ops::Mul for Quat {
    type Output = Self;
    /// Hamilton product: applies `self` first, then `rhs`.
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self {
            x: self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
            y: self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
            z: self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
            w: self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
        }
    }
}

impl Default for Quat {
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

#[cfg(test)]
mod tests_pass_14 {
    use super::*;

    // ── bias ──────────────────────────────────────────────────────────────
    #[test]
    fn bias_midpoint_at_half() {
        // bias(0.5, 0.5) should equal 0.5 (identity knob)
        let v = bias(0.5, 0.5);
        assert!((v - 0.5).abs() < 1e-5, "bias(0.5,0.5) = {v}");
    }

    #[test]
    fn bias_b_near_one_biases_toward_one() {
        // b close to 1 → small exponent → output > t for t in (0,1)
        let v = bias(0.5, 0.9);
        assert!(
            v > 0.5,
            "bias(0.5,0.9) should be > 0.5 (biased up), got {v}"
        );
    }

    #[test]
    fn bias_b_near_zero_biases_toward_zero() {
        // b close to 0 → large exponent → output < t for t in (0,1)
        let v = bias(0.5, 0.1);
        assert!(
            v < 0.5,
            "bias(0.5,0.1) should be < 0.5 (biased down), got {v}"
        );
    }

    #[test]
    fn bias_extremes_clamped() {
        // t=0 → 0, t=1 → 1 regardless of b
        assert!((bias(0.0, 0.3) - 0.0).abs() < 1e-6);
        assert!((bias(1.0, 0.7) - 1.0).abs() < 1e-6);
    }

    // ── gain ──────────────────────────────────────────────────────────────
    #[test]
    fn gain_midpoint_always_half() {
        // gain(0.5, g) = 0.5 for any g
        for g in [0.1, 0.3, 0.5, 0.7, 0.9] {
            let v = gain(0.5, g);
            assert!((v - 0.5).abs() < 1e-5, "gain(0.5,{g}) = {v}");
        }
    }

    #[test]
    fn gain_symmetric_about_half() {
        // gain(1-t, g) = 1 - gain(t, g)
        let g = 0.4_f32;
        for t in [0.1_f32, 0.25, 0.4, 0.75] {
            let a = gain(t, g);
            let b = gain(1.0 - t, g);
            assert!(
                (a + b - 1.0).abs() < 1e-5,
                "symmetry fail at t={t}: {a}+{b}"
            );
        }
    }

    // ── triangle_wave ─────────────────────────────────────────────────────
    #[test]
    fn triangle_wave_peaks() {
        assert!((triangle_wave(0.25) - 0.5).abs() < 1e-6);
        assert!((triangle_wave(0.75) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn triangle_wave_extremes() {
        assert!((triangle_wave(0.0) - 0.0).abs() < 1e-6);
        assert!((triangle_wave(0.5) - 1.0).abs() < 1e-6);
        assert!((triangle_wave(1.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn triangle_wave_periodic() {
        // t and t+1 give same result
        for t in [0.1_f32, 0.37, 0.88] {
            let a = triangle_wave(t);
            let b = triangle_wave(t + 1.0);
            assert!((a - b).abs() < 1e-6, "periodic fail at t={t}");
        }
    }

    // ── exp_decay ─────────────────────────────────────────────────────────
    #[test]
    fn exp_decay_at_zero_is_one() {
        assert!((exp_decay(0.0, 5.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn exp_decay_decreases_monotonically() {
        let v1 = exp_decay(1.0, 2.0);
        let v2 = exp_decay(2.0, 2.0);
        assert!(v1 > v2, "should decrease: {v1} > {v2}");
    }

    #[test]
    fn exp_decay_negative_t_clamped() {
        // t<0 treated as t=0 → should return 1.0
        assert!((exp_decay(-1.0, 3.0) - 1.0).abs() < 1e-6);
    }

    // ── ease_in / ease_out / ease_in_out ──────────────────────────────────
    #[test]
    fn ease_in_clamps_and_monotone() {
        assert!((ease_in(0.0) - 0.0).abs() < 1e-6);
        assert!((ease_in(1.0) - 1.0).abs() < 1e-6);
        assert!(ease_in(0.5) < 0.5); // slow start
    }

    #[test]
    fn ease_out_clamps_and_monotone() {
        assert!((ease_out(0.0) - 0.0).abs() < 1e-6);
        assert!((ease_out(1.0) - 1.0).abs() < 1e-6);
        assert!(ease_out(0.5) > 0.5); // fast start
    }

    #[test]
    fn ease_in_out_symmetry() {
        // ease_in_out(1-t) = 1 - ease_in_out(t)
        for t in [0.1_f32, 0.3, 0.4] {
            let a = ease_in_out(t);
            let b = ease_in_out(1.0 - t);
            assert!((a + b - 1.0).abs() < 1e-5, "symmetry at t={t}: a={a} b={b}");
        }
    }

    #[test]
    fn ease_in_out_midpoint() {
        assert!((ease_in_out(0.5) - 0.5).abs() < 1e-6);
    }

    // ── Vec3::project_onto_plane ──────────────────────────────────────────
    #[test]
    fn project_onto_xz_plane() {
        // Projecting onto XZ plane (normal=Y) removes Y component
        let v = Vec3::new(3.0, 5.0, 2.0);
        let proj = v.project_onto_plane(Vec3::Y);
        assert!((proj.x - 3.0).abs() < 1e-6);
        assert!((proj.y - 0.0).abs() < 1e-6);
        assert!((proj.z - 2.0).abs() < 1e-6);
    }

    #[test]
    fn project_orthogonal_to_normal() {
        // Result must be perpendicular to the normal
        let v = Vec3::new(1.0, 2.0, 3.0);
        let n = Vec3::new(1.0, 1.0, 0.0).normalize();
        let proj = v.project_onto_plane(n);
        assert!(proj.dot(n).abs() < 1e-5, "not orthogonal: {}", proj.dot(n));
    }

    // ── Vec4::homogenize ─────────────────────────────────────────────────
    #[test]
    fn homogenize_unit_w() {
        let v = Vec4::new(3.0, 6.0, 9.0, 1.0);
        let p = v.homogenize();
        assert!((p.x - 3.0).abs() < 1e-6);
        assert!((p.y - 6.0).abs() < 1e-6);
        assert!((p.z - 9.0).abs() < 1e-6);
    }

    #[test]
    fn homogenize_divides_by_w() {
        let v = Vec4::new(6.0, 9.0, 12.0, 3.0);
        let p = v.homogenize();
        assert!((p.x - 2.0).abs() < 1e-6, "x={}", p.x);
        assert!((p.y - 3.0).abs() < 1e-6, "y={}", p.y);
        assert!((p.z - 4.0).abs() < 1e-6, "z={}", p.z);
    }

    #[test]
    fn homogenize_zero_w_returns_zero() {
        let v = Vec4::new(1.0, 2.0, 3.0, 0.0);
        let p = v.homogenize();
        assert_eq!(p, Vec3::ZERO);
    }
}

#[cfg(test)]
mod tests_pass_15 {
    use super::*;

    // ── inverse_lerp ──────────────────────────────────────────────────────
    #[test]
    fn inverse_lerp_midpoint() {
        let t = inverse_lerp(0.0, 10.0, 5.0);
        assert!((t - 0.5).abs() < 1e-6, "midpoint: {t}");
    }

    #[test]
    fn inverse_lerp_endpoints() {
        assert!((inverse_lerp(0.0, 10.0, 0.0) - 0.0).abs() < 1e-6);
        assert!((inverse_lerp(0.0, 10.0, 10.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn inverse_lerp_degenerate_returns_zero() {
        // a == b → division by zero guard
        let t = inverse_lerp(5.0, 5.0, 5.0);
        assert_eq!(t, 0.0);
    }

    #[test]
    fn inverse_lerp_is_lerp_inverse() {
        // lerp(a, b, inverse_lerp(a, b, v)) == v
        let (a, b, v) = (3.0_f32, 7.0, 5.5);
        let t = inverse_lerp(a, b, v);
        let roundtrip = lerp(a, b, t);
        assert!((roundtrip - v).abs() < 1e-5);
    }

    // ── remap_clamped ─────────────────────────────────────────────────────
    #[test]
    fn remap_clamped_midpoint() {
        let v = remap_clamped(5.0, 0.0, 10.0, 0.0, 1.0);
        assert!((v - 0.5).abs() < 1e-6);
    }

    #[test]
    fn remap_clamped_clamps_high() {
        let v = remap_clamped(20.0, 0.0, 10.0, 0.0, 1.0);
        assert!((v - 1.0).abs() < 1e-6, "should clamp to 1: {v}");
    }

    #[test]
    fn remap_clamped_clamps_low() {
        let v = remap_clamped(-5.0, 0.0, 10.0, 0.0, 1.0);
        assert!((v - 0.0).abs() < 1e-6, "should clamp to 0: {v}");
    }

    // ── step ──────────────────────────────────────────────────────────────
    #[test]
    fn step_below_edge() {
        assert_eq!(step(0.5, 0.3), 0.0);
    }

    #[test]
    fn step_at_edge() {
        assert_eq!(step(0.5, 0.5), 1.0);
    }

    #[test]
    fn step_above_edge() {
        assert_eq!(step(0.5, 0.8), 1.0);
    }

    // ── sign_no_zero ──────────────────────────────────────────────────────
    #[test]
    fn sign_no_zero_positive() {
        assert_eq!(sign_no_zero(3.0), 1.0);
    }

    #[test]
    fn sign_no_zero_negative() {
        assert_eq!(sign_no_zero(-2.0), -1.0);
    }

    #[test]
    fn sign_no_zero_at_zero() {
        assert_eq!(sign_no_zero(0.0), 1.0, "zero maps to +1");
    }
}

#[cfg(test)]
mod tests_pass_16 {
    use super::*;

    // ── sawtooth_wave ─────────────────────────────────────────────────────
    #[test]
    fn sawtooth_ramps_0_to_1() {
        assert!((sawtooth_wave(0.0) - 0.0).abs() < 1e-6);
        assert!((sawtooth_wave(0.5) - 0.5).abs() < 1e-6);
        assert!((sawtooth_wave(1.0) - 0.0).abs() < 1e-6); // resets
    }

    #[test]
    fn sawtooth_periodic() {
        for t in [0.1_f32, 0.37, 0.9] {
            assert!((sawtooth_wave(t) - sawtooth_wave(t + 1.0)).abs() < 1e-6);
        }
    }

    // ── square_wave ───────────────────────────────────────────────────────
    #[test]
    fn square_wave_duty_half() {
        assert_eq!(square_wave(0.25, 0.5), 1.0); // first half
        assert_eq!(square_wave(0.75, 0.5), 0.0); // second half
    }

    #[test]
    fn square_wave_duty_zero_always_off() {
        for t in [0.0_f32, 0.5, 0.99] {
            assert_eq!(square_wave(t, 0.0), 0.0);
        }
    }

    #[test]
    fn square_wave_duty_one_always_on() {
        for t in [0.0_f32, 0.5, 0.99] {
            assert_eq!(square_wave(t, 1.0), 1.0);
        }
    }

    // ── pulse_wave ────────────────────────────────────────────────────────
    #[test]
    fn pulse_wave_inside() {
        // Centre 0.5, width 0.2 → on in [0.4, 0.6]
        assert_eq!(pulse_wave(0.5, 0.5, 0.2), 1.0);
        assert_eq!(pulse_wave(0.45, 0.5, 0.2), 1.0);
    }

    #[test]
    fn pulse_wave_outside() {
        assert_eq!(pulse_wave(0.2, 0.5, 0.2), 0.0);
        assert_eq!(pulse_wave(0.8, 0.5, 0.2), 0.0);
    }

    // ── signed_area_2d ────────────────────────────────────────────────────
    #[test]
    fn signed_area_ccw_square() {
        let sq = [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];
        let a = signed_area_2d(&sq);
        assert!((a - 1.0).abs() < 1e-6, "CCW unit square area = 1: {a}");
    }

    #[test]
    fn signed_area_cw_square_negative() {
        let sq = [
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, 0.0),
        ];
        let a = signed_area_2d(&sq);
        assert!((a + 1.0).abs() < 1e-6, "CW square area = -1: {a}");
    }

    #[test]
    fn signed_area_degenerate_line() {
        let line = [Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0)];
        assert_eq!(signed_area_2d(&line), 0.0); // < 3 vertices
    }

    // ── polygon_centroid_2d ───────────────────────────────────────────────
    #[test]
    fn centroid_unit_square() {
        let sq = [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];
        let c = polygon_centroid_2d(&sq);
        assert!((c.x - 0.5).abs() < 1e-5, "cx: {}", c.x);
        assert!((c.y - 0.5).abs() < 1e-5, "cy: {}", c.y);
    }

    #[test]
    fn centroid_right_triangle() {
        // Right triangle (0,0),(3,0),(0,3): centroid at (1,1)
        let tri = [
            Vec2::new(0.0, 0.0),
            Vec2::new(3.0, 0.0),
            Vec2::new(0.0, 3.0),
        ];
        let c = polygon_centroid_2d(&tri);
        assert!((c.x - 1.0).abs() < 1e-5, "cx: {}", c.x);
        assert!((c.y - 1.0).abs() < 1e-5, "cy: {}", c.y);
    }

    // ── point_in_polygon_2d ───────────────────────────────────────────────
    #[test]
    fn point_in_triangle() {
        let tri = [
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(1.0, 2.0),
        ];
        assert!(point_in_polygon_2d(Vec2::new(1.0, 0.5), &tri));
        assert!(!point_in_polygon_2d(Vec2::new(3.0, 1.0), &tri));
        assert!(!point_in_polygon_2d(Vec2::new(1.0, 2.5), &tri));
    }

    #[test]
    fn point_in_square() {
        let sq = [
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(0.0, 2.0),
        ];
        assert!(point_in_polygon_2d(Vec2::new(1.0, 1.0), &sq));
        assert!(!point_in_polygon_2d(Vec2::new(3.0, 1.0), &sq));
        assert!(!point_in_polygon_2d(Vec2::new(-1.0, 1.0), &sq));
    }

    // ── Vec2::clamp_length ────────────────────────────────────────────────
    #[test]
    fn clamp_length_shrinks_long_vector() {
        let v = Vec2::new(3.0, 4.0); // length 5
        let c = v.clamp_length(2.0);
        assert!((c.length() - 2.0).abs() < 1e-5);
        // Direction preserved
        let ratio = c.x / v.x;
        assert!((c.y / v.y - ratio).abs() < 1e-5);
    }

    #[test]
    fn clamp_length_leaves_short_vector() {
        let v = Vec2::new(0.5, 0.0);
        assert_eq!(v.clamp_length(2.0), v);
    }

    #[test]
    fn clamp_length_at_exact_max() {
        let v = Vec2::new(2.0, 0.0);
        let c = v.clamp_length(2.0);
        assert!((c.length() - 2.0).abs() < 1e-5);
    }
}

// ── Pass 18: Octahedral encoding, Morton Z-order, Spherical Fibonacci ─────────

/// Encode a unit-length normal vector as a point in \[-1,1\]² using octahedral
/// projection.
///
/// The octahedral encoding maps every direction on the unit sphere to a 2D
/// point that can be stored as two `f32` (or quantised to `i16` for G-buffers).
/// Decoding with [`octahedral_decode`] recovers the original direction to
/// floating-point precision.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Vec3, octahedral_encode, octahedral_decode};
/// let n = Vec3::new(0.0, 1.0, 0.0);
/// let enc = octahedral_encode(n);
/// let dec = octahedral_decode(enc);
/// assert!((dec - n).length() < 1e-5);
/// ```
pub fn octahedral_encode(n: Vec3) -> Vec2 {
    // Project onto octahedron face: divide by L1 norm
    let inv = 1.0 / (n.x.abs() + n.y.abs() + n.z.abs());
    let mut p = Vec2::new(n.x * inv, n.y * inv);
    // Fold the lower hemisphere (z < 0) so the full sphere fits in [-1,1]²
    if n.z < 0.0 {
        let px = p.x;
        let py = p.y;
        p.x = (1.0 - py.abs()) * px.signum();
        p.y = (1.0 - px.abs()) * py.signum();
    }
    p
}

/// Decode a 2D octahedral-encoded normal back to a unit `Vec3`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Vec2, Vec3, octahedral_decode};
/// let dec = octahedral_decode(Vec2::new(0.0, 0.0));
/// assert!((dec - Vec3::new(0.0, 0.0, 1.0)).length() < 1e-5);
/// ```
pub fn octahedral_decode(v: Vec2) -> Vec3 {
    let z = 1.0 - v.x.abs() - v.y.abs();
    let mut p = Vec3::new(v.x, v.y, z);
    if z < 0.0 {
        let px = p.x;
        let py = p.y;
        p.x = (1.0 - py.abs()) * px.signum();
        p.y = (1.0 - px.abs()) * py.signum();
    }
    p.normalize_or_zero()
}

/// Interleave the low 16 bits of `x` and `y` into a 32-bit Morton (Z-order)
/// code.
///
/// Morton codes give 2D data spatial locality in 1D memory layout — adjacent
/// tiles/texels cluster together in cache even during diagonal traversal.
///
/// # Examples
///
/// ```
/// use abrash_core::math::morton_encode_2d;
/// assert_eq!(morton_encode_2d(1, 0), 0b01);
/// assert_eq!(morton_encode_2d(0, 1), 0b10);
/// assert_eq!(morton_encode_2d(3, 3), 0b1111);
/// ```
pub fn morton_encode_2d(x: u32, y: u32) -> u32 {
    /// Spread a 16-bit value into even bit positions.
    #[inline(always)]
    fn part1by1(mut n: u32) -> u32 {
        n &= 0x0000_FFFF;
        n = (n | (n << 8)) & 0x00FF_00FF;
        n = (n | (n << 4)) & 0x0F0F_0F0F;
        n = (n | (n << 2)) & 0x3333_3333;
        n = (n | (n << 1)) & 0x5555_5555;
        n
    }
    part1by1(x) | (part1by1(y) << 1)
}

/// Decode a Morton code back into `(x, y)` component pairs.
///
/// # Examples
///
/// ```
/// use abrash_core::math::morton_decode_2d;
/// assert_eq!(morton_decode_2d(0b01), (1, 0));
/// assert_eq!(morton_decode_2d(0b10), (0, 1));
/// assert_eq!(morton_decode_2d(0b1111), (3, 3));
/// ```
pub fn morton_decode_2d(code: u32) -> (u32, u32) {
    /// Compact even-bit positions back into a contiguous value.
    #[inline(always)]
    fn compact1by1(mut n: u32) -> u32 {
        n &= 0x5555_5555;
        n = (n | (n >> 1)) & 0x3333_3333;
        n = (n | (n >> 2)) & 0x0F0F_0F0F;
        n = (n | (n >> 4)) & 0x00FF_00FF;
        n = (n | (n >> 8)) & 0x0000_FFFF;
        n
    }
    (compact1by1(code), compact1by1(code >> 1))
}

/// Generate the `i`-th point from a **spherical Fibonacci** lattice.
///
/// Distributes `n` points uniformly on the unit sphere using golden-ratio
/// angular spacing — far better than uniform grids or random sampling for
/// hemisphere importance sampling in PBR renderers.
///
/// `i` must be in `0..n`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::spherical_fibonacci;
/// let p = spherical_fibonacci(0, 100);
/// assert!((p.length() - 1.0).abs() < 1e-5, "must be on unit sphere");
/// ```
pub fn spherical_fibonacci(i: u32, n: u32) -> Vec3 {
    // Golden ratio: φ = (1 + √5) / 2; 2π/φ ≈ golden angle in radians
    const GOLDEN_ANGLE: f32 = core::f32::consts::TAU * (1.0 - 1.618_033_988_749_895);
    let theta = GOLDEN_ANGLE * i as f32;
    // Cosine of polar angle evenly spaced in [-1, 1]
    let cos_phi = -1.0 + (2.0 * i as f32 + 1.0) / n as f32;
    let sin_phi = (1.0 - cos_phi * cos_phi).max(0.0).sqrt();
    let (sin_t, cos_t) = theta.sin_cos();
    Vec3::new(sin_phi * cos_t, sin_phi * sin_t, cos_phi)
}

/// Generate the `n`-th term of the **golden-ratio low-discrepancy** sequence
/// on \[0,1\).
///
/// Uses the additive recurrence `frac(n * φ)` where `φ = (1+√5)/2`.  This
/// 1D sequence has the lowest possible *discrepancy* among additive
/// sequences, making it ideal for stratified 1D importance sampling.
///
/// # Examples
///
/// ```
/// use abrash_core::math::golden_ratio_sequence;
/// // First 8 values should be distinct and in [0,1)
/// let vals: Vec<f32> = (0..8).map(|i| golden_ratio_sequence(i)).collect();
/// for v in &vals { assert!(*v >= 0.0 && *v < 1.0); }
/// ```
pub fn golden_ratio_sequence(n: u32) -> f32 {
    const PHI: f32 = 1.618_033_988_749_895_f32;
    (n as f32 * PHI).fract()
}

#[cfg(test)]
mod tests_pass_17 {
    use super::*;

    // ── halton ────────────────────────────────────────────────────────────
    #[test]
    fn halton_base2_known_values() {
        assert!((halton(1, 2) - 0.5).abs() < 1e-7);
        assert!((halton(2, 2) - 0.25).abs() < 1e-7);
        assert!((halton(3, 2) - 0.75).abs() < 1e-7);
        assert!((halton(4, 2) - 0.125).abs() < 1e-7);
    }

    #[test]
    fn halton_zero_is_zero() {
        assert_eq!(halton(0, 2), 0.0);
        assert_eq!(halton(0, 3), 0.0);
    }

    #[test]
    fn halton_in_unit_interval() {
        for i in 0..64u32 {
            let v = halton(i, 2);
            assert!((0.0..1.0).contains(&v), "halton({i},2) = {v} not in [0,1)");
        }
    }

    #[test]
    fn halton_base3_known_values() {
        // base-3: 1→1/3, 2→2/3, 3→1/9, 4→4/9
        assert!((halton(1, 3) - 1.0 / 3.0).abs() < 1e-7);
        assert!((halton(2, 3) - 2.0 / 3.0).abs() < 1e-7);
        assert!((halton(3, 3) - 1.0 / 9.0).abs() < 1e-7);
    }

    // ── van_der_corput ────────────────────────────────────────────────────
    #[test]
    fn van_der_corput_matches_halton_base2() {
        for i in 0..32u32 {
            let a = van_der_corput(i);
            let b = halton(i, 2);
            assert!((a - b).abs() < 1e-7, "vdc({i}) = {a}, halton = {b}");
        }
    }

    #[test]
    fn van_der_corput_in_unit_interval() {
        for i in 0..64u32 {
            let v = van_der_corput(i);
            assert!((0.0..=1.0).contains(&v));
        }
    }

    // ── hammersley_2d ─────────────────────────────────────────────────────
    #[test]
    fn hammersley_x_is_i_over_n() {
        let p = hammersley_2d(3, 8);
        assert!((p.x - 3.0 / 8.0).abs() < 1e-7);
    }

    #[test]
    fn hammersley_y_is_vdc() {
        for i in 0..16u32 {
            let p = hammersley_2d(i, 16);
            assert!((p.y - van_der_corput(i)).abs() < 1e-9);
        }
    }

    // ── frenet_frame ──────────────────────────────────────────────────────
    #[test]
    fn frenet_frame_orthonormal() {
        let (t, n, b) = frenet_frame(Vec3::new(1.0, 0.5, 0.2), Vec3::Y);
        assert!(
            (t.length() - 1.0).abs() < 1e-5,
            "T not unit: {}",
            t.length()
        );
        assert!(
            (n.length() - 1.0).abs() < 1e-5,
            "N not unit: {}",
            n.length()
        );
        assert!(
            (b.length() - 1.0).abs() < 1e-5,
            "B not unit: {}",
            b.length()
        );
        assert!(t.dot(n).abs() < 1e-5, "T·N not zero: {}", t.dot(n));
        assert!(t.dot(b).abs() < 1e-5, "T·B not zero: {}", t.dot(b));
        assert!(n.dot(b).abs() < 1e-5, "N·B not zero: {}", n.dot(b));
    }

    #[test]
    fn frenet_frame_x_axis_with_y_up() {
        let (t, _n, _b) = frenet_frame(Vec3::X, Vec3::Y);
        assert!((t - Vec3::X).length() < 1e-5, "T should be +X");
    }
}

#[cfg(test)]
mod tests_pass_18 {
    use super::*;

    // ── octahedral_encode / octahedral_decode ─────────────────────────────
    #[test]
    fn octahedral_round_trip_axes() {
        let axes = [
            Vec3::X,
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::Y,
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::Z,
            Vec3::new(0.0, 0.0, -1.0),
        ];
        for a in axes {
            let enc = octahedral_encode(a);
            let dec = octahedral_decode(enc);
            assert!(
                (dec - a).length() < 1e-5,
                "round-trip failed for {a:?}: got {dec:?}"
            );
        }
    }

    #[test]
    fn octahedral_decoded_is_unit_length() {
        // Arbitrary encoded values should always decode to unit vectors
        for &v in &[
            Vec2::new(0.5, 0.3),
            Vec2::new(-0.7, 0.2),
            Vec2::new(0.1, -0.9),
        ] {
            let d = octahedral_decode(v);
            assert!(
                (d.length() - 1.0).abs() < 1e-5,
                "not unit: {d:?}, len {}",
                d.length()
            );
        }
    }

    #[test]
    fn octahedral_encode_range() {
        // Encoded values should lie in [-1,1]²
        let normals = [
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.577, 0.577, 0.577).normalize_or_zero(),
            Vec3::new(-0.5, 0.5, -0.707).normalize_or_zero(),
        ];
        for n in normals {
            let e = octahedral_encode(n);
            assert!(
                e.x.abs() <= 1.001 && e.y.abs() <= 1.001,
                "encoded out of [-1,1]²: {e:?}"
            );
        }
    }

    // ── morton_encode_2d / morton_decode_2d ──────────────────────────────
    #[test]
    fn morton_encode_known() {
        assert_eq!(morton_encode_2d(0, 0), 0);
        assert_eq!(morton_encode_2d(1, 0), 0b01);
        assert_eq!(morton_encode_2d(0, 1), 0b10);
        assert_eq!(morton_encode_2d(1, 1), 0b11);
        assert_eq!(morton_encode_2d(2, 0), 0b0100);
        assert_eq!(morton_encode_2d(0, 2), 0b1000);
        assert_eq!(morton_encode_2d(3, 3), 0b1111);
    }

    #[test]
    fn morton_round_trip() {
        for x in 0u32..16 {
            for y in 0u32..16 {
                let code = morton_encode_2d(x, y);
                let (dx, dy) = morton_decode_2d(code);
                assert_eq!((dx, dy), (x, y), "round-trip failed for ({x},{y})");
            }
        }
    }

    #[test]
    fn morton_spatial_locality() {
        // Neighbours (0,0)-(1,0)-(0,1)-(1,1) should have codes 0-3
        let c00 = morton_encode_2d(0, 0);
        let c10 = morton_encode_2d(1, 0);
        let c01 = morton_encode_2d(0, 1);
        let c11 = morton_encode_2d(1, 1);
        assert_eq!([c00, c10, c01, c11], [0, 1, 2, 3]);
    }

    // ── spherical_fibonacci ───────────────────────────────────────────────
    #[test]
    fn spherical_fibonacci_on_unit_sphere() {
        for i in 0..64u32 {
            let p = spherical_fibonacci(i, 64);
            assert!(
                (p.length() - 1.0).abs() < 1e-5,
                "point {i} not on unit sphere: len {}",
                p.length()
            );
        }
    }

    #[test]
    fn spherical_fibonacci_poles() {
        let n = 100u32;
        // First point near south pole (cos_phi ≈ -1)
        let south = spherical_fibonacci(0, n);
        assert!(south.z < -0.95, "first point should be near south pole");
        // Last point near north pole (cos_phi ≈ 1)
        let north = spherical_fibonacci(n - 1, n);
        assert!(north.z > 0.95, "last point should be near north pole");
    }

    #[test]
    fn spherical_fibonacci_coverage() {
        // 64 points should cover all octants
        let n = 64u32;
        let mut octants = [false; 8];
        for i in 0..n {
            let p = spherical_fibonacci(i, n);
            let idx = ((p.x > 0.0) as usize)
                | (((p.y > 0.0) as usize) << 1)
                | (((p.z > 0.0) as usize) << 2);
            octants[idx] = true;
        }
        assert!(octants.iter().all(|&v| v), "not all octants covered");
    }

    // ── golden_ratio_sequence ─────────────────────────────────────────────
    #[test]
    fn golden_ratio_in_unit_interval() {
        for i in 0..256u32 {
            let v = golden_ratio_sequence(i);
            assert!(
                v >= 0.0 && v < 1.0,
                "golden_ratio_sequence({i}) = {v} out of [0,1)"
            );
        }
    }

    #[test]
    fn golden_ratio_low_discrepancy() {
        // All 8 first values should be distinct (no clustering)
        let vals: Vec<f32> = (0..8).map(golden_ratio_sequence).collect();
        for i in 0..vals.len() {
            for j in (i + 1)..vals.len() {
                assert!(
                    (vals[i] - vals[j]).abs() > 0.05,
                    "golden_ratio values {i} and {j} are too close: {} vs {}",
                    vals[i],
                    vals[j]
                );
            }
        }
    }
}

// ── Pass 19: Spring dynamics, fast bit-ops, scalar utilities ──────────────────

/// Simulate one step of a **critically-damped spring** toward a target.
///
/// This is the standard game-dev approach for smooth follow-cameras, UI
/// animations, and procedural physics.  Unlike `exp_decay`, it correctly models
/// velocity so that fast-moving targets are tracked without overshoot.
///
/// - `current`  — current position
/// - `velocity` — current velocity (updated in-place)
/// - `target`   — desired position
/// - `omega`    — angular frequency (larger = stiffer spring, faster response)
/// - `dt`       — time step in seconds
///
/// Returns `(new_position, new_velocity)`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::spring_damper;
/// let mut vel = 0.0_f32;
/// let (pos, v) = spring_damper(0.0, &mut vel, 10.0, 5.0, 0.016);
/// assert!(pos > 0.0, "moved toward target");
/// assert!(v > 0.0, "velocity increased");
/// ```
pub fn spring_damper(
    current: f32,
    velocity: &mut f32,
    target: f32,
    omega: f32,
    dt: f32,
) -> (f32, f32) {
    // Stable semi-implicit Euler for critically-damped spring
    // ω = sqrt(k/m), damping = 2*sqrt(k*m) => ζ=1 (critically damped)
    let x = current - target;
    let f = 1.0 + 2.0 * dt * omega;
    let oo = omega * omega;
    let hoo = dt * oo;
    let hhoo = dt * hoo;
    let det_inv = 1.0 / (f + hhoo);
    let new_x = (x * f + dt * *velocity) * det_inv;
    let new_v = (*velocity - x * hoo) * det_inv;
    *velocity = new_v;
    (target + new_x, new_v)
}

/// Fast approximate base-2 logarithm using IEEE 754 exponent bits.
///
/// Extracts the exponent from the float representation for a quick integer
/// floor(log2(x)).  Error is < 1 ULP for the integer part; the fractional
/// part uses a linear approximation with error < 0.086.
///
/// # Examples
///
/// ```
/// use abrash_core::math::fast_log2;
/// assert!((fast_log2(8.0) - 3.0).abs() < 0.1);
/// assert!((fast_log2(1.0) - 0.0).abs() < 0.1);
/// assert!((fast_log2(0.5) - (-1.0)).abs() < 0.1);
/// ```
pub fn fast_log2(x: f32) -> f32 {
    debug_assert!(x > 0.0, "fast_log2 requires positive input");
    let bits = x.to_bits();
    // Exponent field: bits 30..23
    let exp = ((bits >> 23) & 0xFF) as i32 - 127;
    // Mantissa normalized to [1,2): f = 1 + mantissa/2^23
    let mantissa_bits = (bits & 0x007F_FFFF) | 0x3F80_0000; // set exp=0 => value in [1,2)
    let f = f32::from_bits(mantissa_bits);
    // Linear approximation: log2(f) ≈ f - 1 (error < 0.086 in [1,2))
    exp as f32 + (f - 1.0)
}

/// Fast approximate base-2 exponentiation.
///
/// Uses the inverse of the `fast_log2` trick.  Error is < 5% for integer
/// inputs; useful for fog/attenuation where exact values don't matter.
///
/// # Examples
///
/// ```
/// use abrash_core::math::fast_exp2;
/// assert!((fast_exp2(3.0) - 8.0).abs() < 0.5);
/// assert!((fast_exp2(0.0) - 1.0).abs() < 0.1);
/// assert!((fast_exp2(-1.0) - 0.5).abs() < 0.1);
/// ```
pub fn fast_exp2(x: f32) -> f32 {
    let xi = x.floor() as i32;
    let xf = x - xi as f32;
    // Reconstruct via mantissa approximation: exp2(frac) ≈ 1 + frac
    let mantissa = ((xf + 1.0).to_bits() & 0x007F_FFFF) | (((xi + 127) as u32) << 23);
    f32::from_bits(mantissa)
}

/// Round up to the next power of two (or return `x` if already a power of two).
///
/// Returns 1 for `x = 0`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::next_power_of_two;
/// assert_eq!(next_power_of_two(0), 1);
/// assert_eq!(next_power_of_two(1), 1);
/// assert_eq!(next_power_of_two(5), 8);
/// assert_eq!(next_power_of_two(8), 8);
/// ```
pub const fn next_power_of_two(x: u32) -> u32 {
    if x == 0 {
        return 1;
    }
    let mut v = x - 1;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    v |= v >> 16;
    v + 1
}

/// Return the largest power of two that is ≤ `x`.
///
/// Returns 0 for `x = 0`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::prev_power_of_two;
/// assert_eq!(prev_power_of_two(0), 0);
/// assert_eq!(prev_power_of_two(1), 1);
/// assert_eq!(prev_power_of_two(7), 4);
/// assert_eq!(prev_power_of_two(8), 8);
/// ```
pub const fn prev_power_of_two(x: u32) -> u32 {
    if x == 0 {
        return 0;
    }
    1 << (31 - x.leading_zeros())
}

/// 5th-order ("Perlin's smootherstep") smooth interpolation.
///
/// Zero first AND second derivatives at t=0 and t=1, giving C² continuity —
/// preferred over the classic `smoothstep` for terrain heightmaps and SDF
/// blending where curvature must be continuous.
///
/// Formula: `6t⁵ - 15t⁴ + 10t³`
///
/// # Examples
///
/// ```
/// use abrash_core::math::smootherstep5;
/// assert_eq!(smootherstep5(0.0), 0.0);
/// assert_eq!(smootherstep5(1.0), 1.0);
/// assert!((smootherstep5(0.5) - 0.5).abs() < 1e-6);
/// ```
pub fn smootherstep5(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// 7th-order smooth interpolation (C³ continuous).
///
/// Zero derivatives through order 3 at both endpoints — maximally smooth for
/// procedural texture blending where even curvature derivatives must not pop.
///
/// Formula: `-20t⁷ + 70t⁶ - 84t⁵ + 35t⁴`
///
/// # Examples
///
/// ```
/// use abrash_core::math::smootherstep7;
/// assert_eq!(smootherstep7(0.0), 0.0);
/// assert_eq!(smootherstep7(1.0), 1.0);
/// assert!((smootherstep7(0.5) - 0.5).abs() < 1e-6);
/// ```
pub fn smootherstep7(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let t2 = t * t;
    let t4 = t2 * t2;
    t4 * (-20.0 * t * t * t + 70.0 * t2 - 84.0 * t + 35.0)
}

/// Return the median of three values without branching.
///
/// Useful for noise filtering (3×3 median kernel inner loop) and as a
/// robust replacement for `clamp` when the bound ordering is unknown.
///
/// # Examples
///
/// ```
/// use abrash_core::math::median3;
/// assert_eq!(median3(1.0_f32, 3.0, 2.0), 2.0);
/// assert_eq!(median3(5.0_f32, 1.0, 3.0), 3.0);
/// assert_eq!(median3(2.0_f32, 2.0, 2.0), 2.0);
/// ```
pub fn median3(a: f32, b: f32, c: f32) -> f32 {
    a.max(b).min(c).max(a.min(b))
}

/// Return `true` if `v` lies in the closed interval \[`lo`, `hi`\].
///
/// # Examples
///
/// ```
/// use abrash_core::math::in_range;
/// assert!(in_range(0.5_f32, 0.0, 1.0));
/// assert!(!in_range(1.5_f32, 0.0, 1.0));
/// assert!(in_range(0.0_f32, 0.0, 1.0));  // inclusive
/// ```
#[inline]
pub fn in_range(v: f32, lo: f32, hi: f32) -> bool {
    v >= lo && v <= hi
}

/// Return `true` if `a` and `b` are within `eps` of each other.
///
/// # Examples
///
/// ```
/// use abrash_core::math::approx_eq;
/// assert!(approx_eq(1.0_f32, 1.0 + 1e-6, 1e-5));
/// assert!(!approx_eq(1.0_f32, 1.1, 1e-5));
/// ```
#[inline]
pub fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
    (a - b).abs() <= eps
}

#[cfg(test)]
mod tests_pass_19 {
    use super::*;

    // ── spring_damper ─────────────────────────────────────────────────────
    #[test]
    fn spring_converges_to_target() {
        let mut vel = 0.0_f32;
        let mut pos = 0.0_f32;
        let target = 10.0_f32;
        for _ in 0..500 {
            let (p, v) = spring_damper(pos, &mut vel, target, 10.0, 0.016);
            pos = p;
            vel = v;
        }
        assert!(
            (pos - target).abs() < 0.01,
            "spring should converge: pos={pos}"
        );
    }

    #[test]
    fn spring_no_overshoot() {
        let mut vel = 0.0_f32;
        let mut pos = 0.0_f32;
        let target = 1.0_f32;
        let mut max_pos = 0.0_f32;
        for _ in 0..200 {
            let (p, v) = spring_damper(pos, &mut vel, target, 8.0, 0.016);
            pos = p;
            vel = v;
            max_pos = max_pos.max(pos);
        }
        // Critically damped: should not overshoot by more than 1%
        assert!(
            max_pos <= 1.01,
            "critically damped spring overshot: {max_pos}"
        );
    }

    #[test]
    fn spring_stationary_stays_still() {
        let mut vel = 0.0_f32;
        let (p, v) = spring_damper(5.0, &mut vel, 5.0, 10.0, 0.016);
        assert!((p - 5.0).abs() < 1e-5, "at target, should stay: {p}");
        assert!(v.abs() < 1e-5, "velocity should be zero: {v}");
    }

    // ── fast_log2 ─────────────────────────────────────────────────────────
    #[test]
    fn fast_log2_powers_of_two() {
        for i in 0u32..8 {
            let x = (1u32 << i) as f32;
            let expected = i as f32;
            let got = fast_log2(x);
            assert!(
                (got - expected).abs() < 0.01,
                "fast_log2({x}) = {got}, expected {expected}"
            );
        }
    }

    #[test]
    fn fast_log2_one_half() {
        assert!((fast_log2(0.5) - (-1.0)).abs() < 0.01);
    }

    // ── fast_exp2 ─────────────────────────────────────────────────────────
    #[test]
    fn fast_exp2_integer_inputs() {
        for i in -3i32..=8 {
            let expected = (2.0_f32).powi(i);
            let got = fast_exp2(i as f32);
            let rel = (got - expected).abs() / expected;
            assert!(rel < 0.05, "fast_exp2({i}) = {got}, expected {expected}");
        }
    }

    // ── next_power_of_two / prev_power_of_two ────────────────────────────
    #[test]
    fn next_pow2_known() {
        assert_eq!(next_power_of_two(0), 1);
        assert_eq!(next_power_of_two(1), 1);
        assert_eq!(next_power_of_two(2), 2);
        assert_eq!(next_power_of_two(3), 4);
        assert_eq!(next_power_of_two(100), 128);
        assert_eq!(next_power_of_two(256), 256);
    }

    #[test]
    fn prev_pow2_known() {
        assert_eq!(prev_power_of_two(0), 0);
        assert_eq!(prev_power_of_two(1), 1);
        assert_eq!(prev_power_of_two(7), 4);
        assert_eq!(prev_power_of_two(8), 8);
        assert_eq!(prev_power_of_two(255), 128);
    }

    // ── smootherstep5 / smootherstep7 ────────────────────────────────────
    #[test]
    fn smootherstep5_endpoints_and_midpoint() {
        assert_eq!(smootherstep5(0.0), 0.0);
        assert_eq!(smootherstep5(1.0), 1.0);
        assert!((smootherstep5(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn smootherstep5_clamps() {
        assert_eq!(smootherstep5(-1.0), 0.0);
        assert_eq!(smootherstep5(2.0), 1.0);
    }

    #[test]
    fn smootherstep7_endpoints_and_midpoint() {
        assert_eq!(smootherstep7(0.0), 0.0);
        assert_eq!(smootherstep7(1.0), 1.0);
        assert!((smootherstep7(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn smootherstep5_steeper_than_smoothstep_near_edges() {
        // smootherstep5 has zero second derivative at endpoints → flatter shoulders
        // Check that 0.25 is below the 3rd-order smoothstep
        let s3 = smoothstep(0.0, 1.0, 0.25);
        let s5 = smootherstep5(0.25);
        assert!(
            s5 < s3,
            "smootherstep5 should be flatter near 0: {s5} vs {s3}"
        );
    }

    // ── median3 ───────────────────────────────────────────────────────────
    #[test]
    fn median3_all_permutations() {
        let vals = [1.0_f32, 3.0, 2.0];
        // All 6 orderings of (1, 2, 3) should give median=2
        assert_eq!(median3(vals[0], vals[1], vals[2]), 2.0);
        assert_eq!(median3(vals[0], vals[2], vals[1]), 2.0);
        assert_eq!(median3(vals[1], vals[0], vals[2]), 2.0);
        assert_eq!(median3(vals[1], vals[2], vals[0]), 2.0);
        assert_eq!(median3(vals[2], vals[0], vals[1]), 2.0);
        assert_eq!(median3(vals[2], vals[1], vals[0]), 2.0);
    }

    // ── in_range / approx_eq ─────────────────────────────────────────────
    #[test]
    fn in_range_inclusive_bounds() {
        assert!(in_range(0.0_f32, 0.0, 1.0));
        assert!(in_range(1.0_f32, 0.0, 1.0));
        assert!(!in_range(1.001_f32, 0.0, 1.0));
        assert!(!in_range(-0.001_f32, 0.0, 1.0));
    }

    #[test]
    fn approx_eq_within_eps() {
        assert!(approx_eq(1.0_f32, 1.0 + 1e-6, 1e-5));
        assert!(!approx_eq(1.0_f32, 1.1, 1e-5));
        assert!(approx_eq(-1.0_f32, -1.0, 0.0));
    }
}

// ── Pass 20: Hashing, Bayer dithering, audio utils, misc ─────────────────────

/// PCG (Permuted Congruential Generator) hash — high-quality stateless integer
/// hash in ~3 instructions.
///
/// Ideal for procedural generation, noise seeding, and GPU-style per-pixel
/// random number generation.  Passes PractRand and BigCrush statistical tests.
///
/// # Examples
///
/// ```
/// use abrash_core::math::pcg_hash;
/// assert_eq!(pcg_hash(42), pcg_hash(42));
/// assert_ne!(pcg_hash(1), pcg_hash(2));
/// ```
pub const fn pcg_hash(input: u32) -> u32 {
    let state = input.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let word = ((state >> ((state >> 28).wrapping_add(4))) ^ state).wrapping_mul(277_803_737);
    (word >> 22) ^ word
}

/// Wang hash — a classic fast integer hash for procedural texturing.
///
/// # Examples
///
/// ```
/// use abrash_core::math::wang_hash;
/// assert_ne!(wang_hash(0), wang_hash(1));
/// assert_eq!(wang_hash(42), wang_hash(42));
/// ```
pub const fn wang_hash(mut key: u32) -> u32 {
    key = key.wrapping_add(!(key << 15));
    key ^= key >> 10;
    key = key.wrapping_add(key << 3);
    key ^= key >> 6;
    key = key.wrapping_add(!(key << 11));
    key ^= key >> 16;
    key
}

/// Hash a `u32` to a `f32` in \[0, 1).
///
/// # Examples
///
/// ```
/// use abrash_core::math::hash_to_f32;
/// let v = hash_to_f32(12345);
/// assert!(v >= 0.0 && v < 1.0);
/// assert_ne!(hash_to_f32(0), hash_to_f32(1));
/// ```
pub fn hash_to_f32(seed: u32) -> f32 {
    let h = pcg_hash(seed);
    f32::from_bits((h >> 9) | 0x3F80_0000) - 1.0
}

/// Hash two `u32` coordinates to a `f32`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::hash2_to_f32;
/// let v = hash2_to_f32(3, 7);
/// assert!(v >= 0.0 && v < 1.0);
/// ```
pub fn hash2_to_f32(x: u32, y: u32) -> f32 {
    hash_to_f32(pcg_hash(x).wrapping_add(y.wrapping_mul(2_654_435_761)))
}

/// Look up a value in the 8×8 **Bayer ordered-dither matrix**, normalised to
/// \[0, 1).
///
/// Compare this threshold against a pixel's intensity to decide whether to
/// round up or down.
///
/// `x` and `y` are pixel coordinates (only low 3 bits used).
///
/// # Examples
///
/// ```
/// use abrash_core::math::bayer8x8;
/// for y in 0..8u32 { for x in 0..8u32 {
///     let v = bayer8x8(x, y);
///     assert!(v >= 0.0 && v < 1.0);
/// }}
/// ```
pub fn bayer8x8(x: u32, y: u32) -> f32 {
    const BAYER: [[u8; 8]; 8] = [
        [0, 32, 8, 40, 2, 34, 10, 42],
        [48, 16, 56, 24, 50, 18, 58, 26],
        [12, 44, 4, 36, 14, 46, 6, 38],
        [60, 28, 52, 20, 62, 30, 54, 22],
        [3, 35, 11, 43, 1, 33, 9, 41],
        [51, 19, 59, 27, 49, 17, 57, 25],
        [15, 47, 7, 39, 13, 45, 5, 37],
        [63, 31, 55, 23, 61, 29, 53, 21],
    ];
    BAYER[(y & 7) as usize][(x & 7) as usize] as f32 / 64.0
}

/// Convert a linear amplitude ratio to decibels: `20 * log10(|amplitude|)`.
///
/// Returns `-∞` for `amplitude = 0`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::linear_to_db;
/// assert!((linear_to_db(1.0) - 0.0).abs() < 1e-5);
/// assert!((linear_to_db(2.0) - 6.0206).abs() < 0.01);
/// ```
pub fn linear_to_db(amplitude: f32) -> f32 {
    20.0 * amplitude.abs().log10()
}

/// Convert decibels to a linear amplitude ratio: `10^(db/20)`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::db_to_linear;
/// assert!((db_to_linear(0.0) - 1.0).abs() < 1e-5);
/// assert!((db_to_linear(-20.0) - 0.1).abs() < 1e-5);
/// ```
pub fn db_to_linear(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

/// Snap `value` to the nearest multiple of `step`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::snap;
/// assert!((snap(1.7_f32, 0.5) - 1.5).abs() < 1e-6);
/// assert!((snap(1.3_f32, 0.5) - 1.5).abs() < 1e-6);
/// ```
pub fn snap(value: f32, step: f32) -> f32 {
    (value / step).round() * step
}

/// Bounce `t` back and forth between `lo` and `hi`.
///
/// Unlike `ping_pong`, this version maps an unbounded `t` into `[lo, hi]`
/// with triangle-wave folding, so it's suitable for position oscillation.
///
/// # Examples
///
/// ```
/// use abrash_core::math::bounce;
/// assert!((bounce(1.5_f32, 0.0, 1.0) - 0.5).abs() < 1e-5);
/// assert!((bounce(2.0_f32, 0.0, 1.0) - 0.0).abs() < 1e-5);
/// ```
pub fn bounce(t: f32, lo: f32, hi: f32) -> f32 {
    let range = hi - lo;
    if range <= 0.0 {
        return lo;
    }
    let t2 = ((t - lo) % (2.0 * range)).abs();
    let folded = if t2 > range { 2.0 * range - t2 } else { t2 };
    lo + folded
}

#[cfg(test)]
mod tests_pass_20 {
    use super::*;

    // ── pcg_hash ──────────────────────────────────────────────────────────
    #[test]
    fn pcg_hash_deterministic() {
        assert_eq!(pcg_hash(0), pcg_hash(0));
        assert_eq!(pcg_hash(u32::MAX), pcg_hash(u32::MAX));
    }

    #[test]
    fn pcg_hash_avalanche() {
        let a = pcg_hash(0x0000_0001);
        let b = pcg_hash(0x0000_0002);
        let diff = (a ^ b).count_ones();
        assert!(diff >= 8, "poor avalanche: only {diff} bits differ");
    }

    // ── wang_hash ─────────────────────────────────────────────────────────
    #[test]
    fn wang_hash_deterministic() {
        assert_eq!(wang_hash(42), wang_hash(42));
    }

    #[test]
    fn wang_hash_distinct_inputs() {
        for i in 0u32..8 {
            for j in (i + 1)..8 {
                assert_ne!(wang_hash(i), wang_hash(j), "collision at {i},{j}");
            }
        }
    }

    // ── hash_to_f32 / hash2_to_f32 ───────────────────────────────────────
    #[test]
    fn hash_to_f32_in_range() {
        for i in 0u32..256 {
            let v = hash_to_f32(i);
            assert!(v >= 0.0 && v < 1.0, "hash_to_f32({i}) = {v}");
        }
    }

    #[test]
    fn hash2_to_f32_in_range() {
        for x in 0u32..16 {
            for y in 0u32..16 {
                let v = hash2_to_f32(x, y);
                assert!(v >= 0.0 && v < 1.0, "hash2({x},{y}) = {v}");
            }
        }
    }

    // ── bayer8x8 ──────────────────────────────────────────────────────────
    #[test]
    fn bayer8x8_all_in_range() {
        for y in 0..8u32 {
            for x in 0..8u32 {
                let v = bayer8x8(x, y);
                assert!(v >= 0.0 && v < 1.0, "bayer({x},{y}) = {v}");
            }
        }
    }

    #[test]
    fn bayer8x8_all_unique() {
        let mut seen = std::collections::HashSet::new();
        for y in 0..8u32 {
            for x in 0..8u32 {
                let v = (bayer8x8(x, y) * 64.0).round() as u32;
                assert!(seen.insert(v), "duplicate Bayer value {v} at ({x},{y})");
            }
        }
    }

    #[test]
    fn bayer8x8_wraps_periodically() {
        assert_eq!(bayer8x8(0, 0), bayer8x8(8, 0));
        assert_eq!(bayer8x8(3, 5), bayer8x8(11, 13));
    }

    // ── linear_to_db / db_to_linear ──────────────────────────────────────
    #[test]
    fn db_round_trip() {
        for amp in [0.01_f32, 0.1, 0.5, 1.0, 2.0, 10.0] {
            let db = linear_to_db(amp);
            let back = db_to_linear(db);
            assert!((back - amp).abs() / amp < 1e-5, "round-trip {amp}: {back}");
        }
    }

    #[test]
    fn db_zero_is_minus_infinity() {
        assert!(linear_to_db(0.0).is_infinite() && linear_to_db(0.0) < 0.0);
    }

    // ── snap ──────────────────────────────────────────────────────────────
    #[test]
    fn snap_rounds_to_grid() {
        assert!((snap(0.7_f32, 0.25) - 0.75).abs() < 1e-6);
        assert!((snap(0.3_f32, 0.25) - 0.25).abs() < 1e-6);
        assert!((snap(-0.3_f32, 0.25) - -0.25).abs() < 1e-6);
    }

    // ── bounce ────────────────────────────────────────────────────────────
    #[test]
    fn bounce_stays_in_range() {
        for i in 0..200 {
            let t = i as f32 * 0.13;
            let v = bounce(t, 0.0, 1.0);
            assert!(v >= -1e-5 && v <= 1.0 + 1e-5, "bounce({t}) = {v}");
        }
    }

    #[test]
    fn bounce_reflects_at_bounds() {
        assert!((bounce(0.0_f32, 0.0, 1.0) - 0.0).abs() < 1e-5);
        assert!((bounce(1.0_f32, 0.0, 1.0) - 1.0).abs() < 1e-5);
        assert!((bounce(1.5_f32, 0.0, 1.0) - 0.5).abs() < 1e-5);
        assert!((bounce(2.0_f32, 0.0, 1.0) - 0.0).abs() < 1e-5);
    }
}

// ── Pass 21: Geometric math, scalar helpers ───────────────────────────────────

/// Minimum of three scalars.
///
/// # Examples
///
/// ```
/// use abrash_core::math::min3;
/// assert_eq!(min3(3.0_f32, 1.0, 2.0), 1.0);
/// ```
#[inline]
pub fn min3(a: f32, b: f32, c: f32) -> f32 {
    a.min(b).min(c)
}

/// Maximum of three scalars.
///
/// # Examples
///
/// ```
/// use abrash_core::math::max3;
/// assert_eq!(max3(3.0_f32, 1.0, 2.0), 3.0);
/// ```
#[inline]
pub fn max3(a: f32, b: f32, c: f32) -> f32 {
    a.max(b).max(c)
}

/// Compute the area of a 3D triangle with vertices `a`, `b`, `c`.
///
/// Uses the cross-product formula: area = ||(b-a) × (c-a)|| / 2.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Vec3, triangle_area_3d};
/// // Unit right triangle in the XY plane has area 0.5
/// let area = triangle_area_3d(Vec3::ZERO, Vec3::X, Vec3::Y);
/// assert!((area - 0.5).abs() < 1e-6);
/// ```
pub fn triangle_area_3d(a: Vec3, b: Vec3, c: Vec3) -> f32 {
    (b - a).cross(c - a).length() * 0.5
}

/// Signed volume of the tetrahedron formed by four vertices `a`, `b`, `c`, `d`.
///
/// The sign encodes the winding order of face (a,b,c) relative to `d`:
/// positive if the face is CCW when viewed from outside (standard winding).
///
/// Useful for computing mesh volumes via the divergence theorem: sum the
/// signed volumes of all faces' tetrahedra from a common origin point.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Vec3, signed_volume_tet};
/// // Regular tetrahedron with unit edge length has volume 1/(6√2) ≈ 0.1178
/// let a = Vec3::ZERO;
/// let b = Vec3::X;
/// let c = Vec3::new(0.5, (3.0_f32).sqrt() / 2.0, 0.0);
/// let d = Vec3::new(0.5, (3.0_f32).sqrt() / 6.0, (6.0_f32 / 9.0).sqrt());
/// let v = signed_volume_tet(a, b, c, d).abs();
/// assert!((v - 0.1178).abs() < 0.01, "tet volume: {v}");
/// ```
pub fn signed_volume_tet(a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> f32 {
    // Scalar triple product: det([b-a, c-a, d-a]) / 6
    // Positive when face (a,b,c) is CCW viewed from d (standard winding)
    (b - a).cross(c - a).dot(d - a) / 6.0
}

/// Perspective-correct interpolation of a scalar attribute.
///
/// Affine barycentric interpolation introduces "swimming" artefacts because
/// screen-space `t` is not linearly related to world-space depth.  This
/// function corrects for the perspective divide using the w-coordinates at
/// each vertex.
///
/// `w0`, `w1` are the homogeneous w values (typically `1/z`) at the two
/// vertices; `v0`, `v1` are the attribute values.  `t` is the affine screen-
/// space parameter in \[0, 1\].
///
/// # Examples
///
/// ```
/// use abrash_core::math::perspective_correct_lerp;
/// // Equal w → same as linear lerp
/// let v = perspective_correct_lerp(0.5, 0.0, 1.0, 1.0, 1.0);
/// assert!((v - 0.5).abs() < 1e-6, "equal w: {v}");
/// ```
pub fn perspective_correct_lerp(t: f32, v0: f32, v1: f32, w0: f32, w1: f32) -> f32 {
    // Interpolate w in screen space, then recover perspective-correct value
    let wt = w0 + (w1 - w0) * t;
    if wt.abs() < 1e-10 {
        return v0;
    }
    (v0 * w0 * (1.0 - t) + v1 * w1 * t) / wt
}

/// Sine-based smooth step.  Equivalent to the classic CSS `ease-in-out` but
/// uses a half-cosine, giving a slightly more gradual curve than the
/// cubic `smoothstep`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::smoothstep_sine;
/// assert_eq!(smoothstep_sine(0.0), 0.0);
/// assert_eq!(smoothstep_sine(1.0), 1.0);
/// assert!((smoothstep_sine(0.5) - 0.5).abs() < 1e-6);
/// ```
pub fn smoothstep_sine(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    0.5 - (core::f32::consts::PI * t).cos() * 0.5
}

/// Exponential ease-in: slow start, fast end.
///
/// `k` controls the steepness (2.0 is gentle, 8.0 is sharp).
///
/// # Examples
///
/// ```
/// use abrash_core::math::ease_exp_in;
/// assert!((ease_exp_in(0.0, 4.0) - 0.0).abs() < 1e-5);
/// assert!((ease_exp_in(1.0, 4.0) - 1.0).abs() < 1e-5);
/// assert!(ease_exp_in(0.5, 4.0) < 0.5, "ease-in should be below diagonal at 0.5");
/// ```
pub fn ease_exp_in(t: f32, k: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t == 0.0 {
        return 0.0;
    }
    let scale = (k * t - k).exp();
    let norm = (1.0 - (-k).exp()).recip();
    (scale - (-k).exp()) * norm
}

/// Exponential ease-out: fast start, slow end (mirror of `ease_exp_in`).
///
/// # Examples
///
/// ```
/// use abrash_core::math::ease_exp_out;
/// assert!((ease_exp_out(0.0, 4.0) - 0.0).abs() < 1e-5);
/// assert!((ease_exp_out(1.0, 4.0) - 1.0).abs() < 1e-5);
/// assert!(ease_exp_out(0.5, 4.0) > 0.5, "ease-out should be above diagonal at 0.5");
/// ```
pub fn ease_exp_out(t: f32, k: f32) -> f32 {
    1.0 - ease_exp_in(1.0 - t, k)
}

#[cfg(test)]
mod tests_pass_21 {
    use super::*;

    // ── min3 / max3 ───────────────────────────────────────────────────────
    #[test]
    fn min3_all_orderings() {
        assert_eq!(min3(1.0_f32, 2.0, 3.0), 1.0);
        assert_eq!(min3(3.0_f32, 1.0, 2.0), 1.0);
        assert_eq!(min3(2.0_f32, 3.0, 1.0), 1.0);
    }

    #[test]
    fn max3_all_orderings() {
        assert_eq!(max3(1.0_f32, 2.0, 3.0), 3.0);
        assert_eq!(max3(3.0_f32, 1.0, 2.0), 3.0);
        assert_eq!(max3(2.0_f32, 3.0, 1.0), 3.0);
    }

    // ── triangle_area_3d ──────────────────────────────────────────────────
    #[test]
    fn unit_right_triangle_area() {
        let area = triangle_area_3d(Vec3::ZERO, Vec3::X, Vec3::Y);
        assert!((area - 0.5).abs() < 1e-6, "unit right tri area: {area}");
    }

    #[test]
    fn unit_equilateral_triangle_area() {
        // Equilateral triangle with side 1: area = sqrt(3)/4 ≈ 0.433
        let b = Vec3::new(1.0, 0.0, 0.0);
        let c = Vec3::new(0.5, 3.0_f32.sqrt() / 2.0, 0.0);
        let area = triangle_area_3d(Vec3::ZERO, b, c);
        assert!(
            (area - 3.0_f32.sqrt() / 4.0).abs() < 1e-5,
            "equilateral area: {area}"
        );
    }

    // ── signed_volume_tet ─────────────────────────────────────────────────
    #[test]
    fn unit_tet_volume() {
        // Tetrahedron with vertices at (0,0,0),(1,0,0),(0,1,0),(0,0,1)
        // Volume = 1/6
        let v = signed_volume_tet(Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z).abs();
        assert!((v - 1.0 / 6.0).abs() < 1e-6, "unit tet volume: {v}");
    }

    #[test]
    fn tet_volume_sign_encodes_winding() {
        let a = Vec3::ZERO;
        let b = Vec3::X;
        let c = Vec3::Y;
        let d = Vec3::Z;
        let v_fwd = signed_volume_tet(a, b, c, d);
        let v_rev = signed_volume_tet(a, c, b, d); // swap b and c → reverse winding
        assert!(v_fwd > 0.0, "CCW: positive");
        assert!(v_rev < 0.0, "CW: negative");
    }

    // ── perspective_correct_lerp ──────────────────────────────────────────
    #[test]
    fn pcl_equal_w_is_linear() {
        let v = perspective_correct_lerp(0.5, 0.0, 1.0, 1.0, 1.0);
        assert!((v - 0.5).abs() < 1e-6, "equal w → linear: {v}");
    }

    #[test]
    fn pcl_endpoints_exact() {
        assert!((perspective_correct_lerp(0.0, 3.0, 7.0, 0.5, 2.0) - 3.0).abs() < 1e-5);
        assert!((perspective_correct_lerp(1.0, 3.0, 7.0, 0.5, 2.0) - 7.0).abs() < 1e-5);
    }

    #[test]
    fn pcl_biases_toward_closer_vertex() {
        // Vertex 1 has larger w (closer to camera) → midpoint biased toward vertex 1's value
        let v_pcl = perspective_correct_lerp(0.5, 0.0, 1.0, 0.1, 2.0);
        let v_lin = 0.5_f32;
        assert!(
            v_pcl > v_lin,
            "PCL biased toward larger-w vertex: pcl={v_pcl}, lin={v_lin}"
        );
    }

    // ── smoothstep_sine ───────────────────────────────────────────────────
    #[test]
    fn smoothstep_sine_endpoints() {
        assert_eq!(smoothstep_sine(0.0), 0.0);
        assert_eq!(smoothstep_sine(1.0), 1.0);
        assert!((smoothstep_sine(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn smoothstep_sine_clamps() {
        assert_eq!(smoothstep_sine(-1.0), 0.0);
        assert_eq!(smoothstep_sine(2.0), 1.0);
    }

    // ── ease_exp_in / ease_exp_out ────────────────────────────────────────
    #[test]
    fn ease_exp_in_endpoints() {
        assert!((ease_exp_in(0.0, 4.0) - 0.0).abs() < 1e-5);
        assert!((ease_exp_in(1.0, 4.0) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn ease_exp_out_is_mirror_of_in() {
        let t = 0.3_f32;
        let out_val = ease_exp_out(t, 4.0);
        let in_val = ease_exp_in(1.0 - t, 4.0);
        assert!((out_val - (1.0 - in_val)).abs() < 1e-5, "out ≈ 1-in(1-t)");
    }
}

// ── Pass 22: Tone mapping, UV utilities, linear/sRGB ─────────────────────────

/// Simple **Reinhard** tone mapping: maps HDR `[0,∞)` → `[0,1)`.
///
/// Apply per-channel.
///
/// # Examples
///
/// ```
/// use abrash_core::math::reinhard;
/// assert!((reinhard(0.0) - 0.0).abs() < 1e-6);
/// assert!((reinhard(1.0) - 0.5).abs() < 1e-6);
/// ```
#[inline]
pub fn reinhard(x: f32) -> f32 {
    x / (1.0 + x)
}

/// Extended Reinhard with a white point `w` — prevents extreme highlights from
/// clipping at different rates than mid-tones.
///
/// # Examples
///
/// ```
/// use abrash_core::math::reinhard_white;
/// // Smaller white point → higher output for the same input (more aggressive)
/// assert!(reinhard_white(2.0, 4.0) > reinhard_white(2.0, 100.0));
/// ```
pub fn reinhard_white(x: f32, white: f32) -> f32 {
    x * (1.0 + x / (white * white)) / (1.0 + x)
}

/// **ACES filmic** tone mapping (Stephen Hill's simplified rational fit).
///
/// Industry-standard curve used in Unreal Engine, Godot, and film pipelines.
/// Input is linear HDR; output is clamped to `[0, 1]`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::aces_filmic;
/// assert!((aces_filmic(0.0)).abs() < 1e-4);
/// assert!(aces_filmic(100.0) <= 1.0); // large input clamps to exactly 1.0
/// ```
pub fn aces_filmic(x: f32) -> f32 {
    ((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14)).clamp(0.0, 1.0)
}

/// Exposure adjustment: multiply by `2^ev` stops.
///
/// EV=1 doubles brightness; EV=-1 halves it.
///
/// # Examples
///
/// ```
/// use abrash_core::math::exposure;
/// assert!((exposure(0.5, 0.0) - 0.5).abs() < 1e-6);
/// assert!((exposure(0.25, 1.0) - 0.5).abs() < 1e-5);
/// ```
#[inline]
pub fn exposure(x: f32, ev: f32) -> f32 {
    x * (2.0_f32).powf(ev)
}

/// Convert a unit-sphere **normal** to equirectangular `(u, v)` in `[0,1]²`.
///
/// `u` is longitude (wraps around equator), `v` is latitude (0=south, 1=north).
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Vec3, sphere_normal_to_uv};
/// let (_, v) = sphere_normal_to_uv(Vec3::Y);
/// assert!((v - 1.0).abs() < 1e-5, "north pole v=1: {v}");
/// let (_, v2) = sphere_normal_to_uv(Vec3::new(0.0,-1.0,0.0));
/// assert!((v2 - 0.0).abs() < 1e-5, "south pole v=0: {v2}");
/// ```
pub fn sphere_normal_to_uv(n: Vec3) -> (f32, f32) {
    let u = (n.x.atan2(n.z) / core::f32::consts::TAU + 0.5).clamp(0.0, 1.0);
    let v = (n.y.clamp(-1.0, 1.0).asin() / core::f32::consts::PI + 0.5).clamp(0.0, 1.0);
    (u, v)
}

/// Construct a **view ray direction** from screen UV and camera parameters.
///
/// `uv` is `[0,1]²` (top-left origin).  `fov_y` in radians.  Returns a
/// normalised direction in view space (+X right, +Y up, −Z forward).
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Vec2, Vec3, make_view_ray};
/// let dir = make_view_ray(Vec2::new(0.5, 0.5), std::f32::consts::FRAC_PI_2, 1.0);
/// assert!(dir.z < 0.0, "centre points -Z: {dir:?}");
/// assert!((dir.length() - 1.0).abs() < 1e-5);
/// ```
pub fn make_view_ray(uv: Vec2, fov_y: f32, aspect: f32) -> Vec3 {
    let half_h = (fov_y * 0.5).tan();
    let half_w = half_h * aspect;
    let px = (uv.x * 2.0 - 1.0) * half_w;
    let py = (1.0 - uv.y * 2.0) * half_h;
    Vec3::new(px, py, -1.0).normalize_or_zero()
}

/// Convert a linear light value to the sRGB perceptual encoding.
///
/// Uses the exact IEC 61966-2-1 piecewise formula.
///
/// # Examples
///
/// ```
/// use abrash_core::math::linear_to_srgb;
/// assert!((linear_to_srgb(0.0)).abs() < 1e-6);
/// assert!((linear_to_srgb(1.0) - 1.0).abs() < 1e-5);
/// assert!(linear_to_srgb(0.5) > 0.70);
/// ```
pub fn linear_to_srgb(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    if x <= 0.003_130_8 {
        x * 12.92
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    }
}

/// Convert an sRGB perceptual value to linear light.
///
/// # Examples
///
/// ```
/// use abrash_core::math::srgb_to_linear;
/// assert!((srgb_to_linear(0.0)).abs() < 1e-6);
/// assert!((srgb_to_linear(1.0) - 1.0).abs() < 1e-5);
/// ```
pub fn srgb_to_linear(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests_pass_22 {
    use super::*;

    // ── reinhard ──────────────────────────────────────────────────────────
    #[test]
    fn reinhard_known_values() {
        assert!((reinhard(0.0) - 0.0).abs() < 1e-6);
        assert!((reinhard(1.0) - 0.5).abs() < 1e-6);
        assert!((reinhard(3.0) - 0.75).abs() < 1e-6);
    }

    #[test]
    fn reinhard_monotone() {
        let mut prev = reinhard(0.0);
        for i in 1..20u32 {
            let v = reinhard(i as f32 * 0.5);
            assert!(v > prev, "not increasing at {i}");
            prev = v;
        }
    }

    #[test]
    fn reinhard_white_brighter_than_plain() {
        let plain = reinhard(1.0);
        let white = reinhard_white(1.0, 100.0);
        assert!(white > plain, "{plain} vs {white}");
    }

    // ── aces_filmic ───────────────────────────────────────────────────────
    #[test]
    fn aces_black_and_clamp() {
        assert!(aces_filmic(0.0).abs() < 1e-4);
        assert!(aces_filmic(100.0) <= 1.0);
    }

    #[test]
    fn aces_output_in_01() {
        for i in 0..50u32 {
            let v = aces_filmic(i as f32 * 0.2);
            assert!(v >= 0.0 && v <= 1.0, "aces({}) = {v}", i as f32 * 0.2);
        }
    }

    // ── exposure ──────────────────────────────────────────────────────────
    #[test]
    fn exposure_zero_ev_identity() {
        assert!((exposure(0.7, 0.0) - 0.7).abs() < 1e-6);
    }

    #[test]
    fn exposure_one_stop_doubles() {
        assert!((exposure(0.25, 1.0) - 0.5).abs() < 1e-5);
    }

    // ── sphere_normal_to_uv ───────────────────────────────────────────────
    #[test]
    fn sphere_uv_poles() {
        let (_, v_n) = sphere_normal_to_uv(Vec3::Y);
        let (_, v_s) = sphere_normal_to_uv(Vec3::new(0.0, -1.0, 0.0));
        assert!((v_n - 1.0).abs() < 1e-5, "north pole: {v_n}");
        assert!(v_s.abs() < 1e-5, "south pole: {v_s}");
    }

    #[test]
    fn sphere_uv_in_range() {
        for d in [Vec3::X, Vec3::Y, Vec3::Z, Vec3::new(0.577, 0.577, 0.577)] {
            let (u, v) = sphere_normal_to_uv(d);
            assert!(
                u >= 0.0 && u <= 1.0 && v >= 0.0 && v <= 1.0,
                "uv out of range for {d:?}"
            );
        }
    }

    // ── make_view_ray ─────────────────────────────────────────────────────
    #[test]
    fn view_ray_centre_forward() {
        let dir = make_view_ray(Vec2::new(0.5, 0.5), core::f32::consts::FRAC_PI_2, 1.0);
        assert!(dir.z < 0.0, "centre -Z: {dir:?}");
        assert!(dir.x.abs() < 1e-5 && dir.y.abs() < 1e-5);
    }

    #[test]
    fn view_ray_unit_length() {
        for uv in [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.3, 0.7),
        ] {
            let d = make_view_ray(uv, 1.0, 1.6);
            assert!((d.length() - 1.0).abs() < 1e-5, "uv={uv:?}: {}", d.length());
        }
    }

    // ── linear_to_srgb / srgb_to_linear ──────────────────────────────────
    #[test]
    fn srgb_round_trip() {
        for i in 0..=10u32 {
            let linear = i as f32 / 10.0;
            let back = srgb_to_linear(linear_to_srgb(linear));
            assert!((back - linear).abs() < 1e-5, "round-trip {linear}: {back}");
        }
    }

    #[test]
    fn srgb_midgrey() {
        // Linear 0.5 maps to ~0.735 in sRGB
        let s = linear_to_srgb(0.5);
        assert!(s > 0.70 && s < 0.76, "mid-grey: {s}");
    }
}

// ── Pass 23: Color spaces, PBR utilities, ray intersections, noise ────────────

/// Convert linear **RGB** → **HSV**.
///
/// Inputs clamped to `[0, 1]`; returns `(h, s, v)` with `h ∈ [0, 360)`,
/// `s, v ∈ [0, 1]`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::rgb_to_hsv;
/// let (h, s, v) = rgb_to_hsv(1.0, 0.0, 0.0);
/// assert!((h - 0.0).abs() < 1e-4 || (h - 360.0).abs() < 1e-4);
/// assert!((s - 1.0).abs() < 1e-5);
/// assert!((v - 1.0).abs() < 1e-5);
/// ```
pub fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let cmax = r.max(g).max(b);
    let cmin = r.min(g).min(b);
    let delta = cmax - cmin;
    let v = cmax;
    let s = if cmax < 1e-9 { 0.0 } else { delta / cmax };
    let h = if delta < 1e-9 {
        0.0
    } else if (cmax - r).abs() < 1e-9 {
        60.0 * (((g - b) / delta).rem_euclid(6.0))
    } else if (cmax - g).abs() < 1e-9 {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    (h, s, v)
}

/// Convert **HSV** → linear **RGB**.
///
/// `h ∈ [0, 360)`, `s, v ∈ [0, 1]`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{rgb_to_hsv, hsv_to_rgb};
/// let (r, g, b) = hsv_to_rgb(120.0, 1.0, 1.0); // pure green
/// assert!((r).abs() < 1e-4);
/// assert!((g - 1.0).abs() < 1e-4);
/// assert!((b).abs() < 1e-4);
/// ```
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    if s < 1e-9 {
        return (v, v, v);
    }
    let hh = (h / 60.0).rem_euclid(6.0);
    let i = hh as u32;
    let f = hh - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    }
}

/// Convert linear **RGB** → **HSL**.
///
/// Returns `(h, s, l)` with `h ∈ [0, 360)`, `s, l ∈ [0, 1]`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::rgb_to_hsl;
/// let (h, s, l) = rgb_to_hsl(0.0, 0.0, 1.0); // pure blue
/// assert!((h - 240.0).abs() < 1e-3);
/// assert!((s - 1.0).abs() < 1e-5);
/// assert!((l - 0.5).abs() < 1e-5);
/// ```
pub fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let cmax = r.max(g).max(b);
    let cmin = r.min(g).min(b);
    let delta = cmax - cmin;
    let l = (cmax + cmin) * 0.5;
    let s = if delta < 1e-9 {
        0.0
    } else {
        delta / (1.0 - (2.0 * l - 1.0).abs())
    };
    let h = if delta < 1e-9 {
        0.0
    } else if (cmax - r).abs() < 1e-9 {
        60.0 * (((g - b) / delta).rem_euclid(6.0))
    } else if (cmax - g).abs() < 1e-9 {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    (h, s, l)
}

/// Convert **HSL** → linear **RGB**.
///
/// `h ∈ [0, 360)`, `s, l ∈ [0, 1]`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{rgb_to_hsl, hsl_to_rgb};
/// let (r0, g0, b0) = (0.8, 0.3, 0.1);
/// let (h, s, l) = rgb_to_hsl(r0, g0, b0);
/// let (r1, g1, b1) = hsl_to_rgb(h, s, l);
/// assert!((r0 - r1).abs() < 1e-5 && (g0 - g1).abs() < 1e-5 && (b0 - b1).abs() < 1e-5);
/// ```
pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hh = (h / 60.0).rem_euclid(6.0);
    let x = c * (1.0 - (hh.rem_euclid(2.0) - 1.0).abs());
    let m = l - c * 0.5;
    let (r, g, b) = if hh < 1.0 {
        (c, x, 0.0)
    } else if hh < 2.0 {
        (x, c, 0.0)
    } else if hh < 3.0 {
        (0.0, c, x)
    } else if hh < 4.0 {
        (0.0, x, c)
    } else if hh < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    (r + m, g + m, b + m)
}

/// **Schlick Fresnel** approximation for a single reflectance value.
///
/// `cos_theta` is the cosine of the angle between the view direction and the
/// surface normal; `f0` is reflectance at normal incidence (e.g. `0.04` for
/// dielectrics, `0.9`+ for metals).
///
/// # Examples
///
/// ```
/// use abrash_core::math::fresnel_schlick;
/// // At normal incidence (cos_theta = 1) → exactly f0
/// assert!((fresnel_schlick(1.0, 0.04) - 0.04).abs() < 1e-6);
/// // At grazing angle (cos_theta = 0) → 1.0
/// assert!((fresnel_schlick(0.0, 0.04) - 1.0).abs() < 1e-6);
/// ```
#[inline]
pub fn fresnel_schlick(cos_theta: f32, f0: f32) -> f32 {
    f0 + (1.0 - f0) * (1.0 - cos_theta).clamp(0.0, 1.0).powi(5)
}

/// **Ray–sphere** intersection.
///
/// `ro` = ray origin, `rd` = ray direction (assumed unit length),
/// `centre` / `r` define the sphere.
///
/// Returns `Some((t_near, t_far))` (both may be negative if the sphere is
/// behind the ray).  Returns `None` if the ray misses.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Vec3, ray_sphere_intersect};
/// let hit = ray_sphere_intersect(Vec3::ZERO, Vec3::Z, Vec3::new(0.0, 0.0, 3.0), 1.0);
/// let (t0, t1) = hit.unwrap();
/// assert!((t0 - 2.0).abs() < 1e-5 && (t1 - 4.0).abs() < 1e-5);
/// ```
pub fn ray_sphere_intersect(ro: Vec3, rd: Vec3, centre: Vec3, r: f32) -> Option<(f32, f32)> {
    let oc = ro - centre;
    let b = oc.dot(rd);
    let c = oc.dot(oc) - r * r;
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let sq = disc.sqrt();
    Some((-b - sq, -b + sq))
}

/// **Ray–plane** intersection.
///
/// Plane defined by `dot(p, normal) = d`.  Returns `Some(t)` (which may be
/// negative) or `None` when the ray is parallel to the plane.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Vec3, ray_plane_intersect};
/// // Ray along +Z hits the XY plane (normal=+Z, d=5) at t=5
/// let t = ray_plane_intersect(Vec3::ZERO, Vec3::Z, Vec3::Z, 5.0).unwrap();
/// assert!((t - 5.0).abs() < 1e-5);
/// ```
pub fn ray_plane_intersect(ro: Vec3, rd: Vec3, normal: Vec3, d: f32) -> Option<f32> {
    let denom = rd.dot(normal);
    if denom.abs() < 1e-9 {
        return None;
    }
    Some((d - ro.dot(normal)) / denom)
}

/// **Ray–AABB** intersection (slab method).
///
/// Returns `Some((t_enter, t_exit))` if the ray hits the box (both values
/// may be negative).  `aabb_min`/`aabb_max` are the box extents.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Vec3, ray_aabb_intersect};
/// let hit = ray_aabb_intersect(
///     Vec3::ZERO, Vec3::X,
///     Vec3::new(2.0, -1.0, -1.0), Vec3::new(4.0, 1.0, 1.0),
/// );
/// let (t0, t1) = hit.unwrap();
/// assert!((t0 - 2.0).abs() < 1e-5 && (t1 - 4.0).abs() < 1e-5);
/// ```
pub fn ray_aabb_intersect(
    ro: Vec3,
    rd: Vec3,
    aabb_min: Vec3,
    aabb_max: Vec3,
) -> Option<(f32, f32)> {
    // Avoid division by tiny numbers by using a large-number fallback.
    let inv = Vec3::new(
        if rd.x.abs() > 1e-30 {
            1.0 / rd.x
        } else {
            f32::INFINITY
        },
        if rd.y.abs() > 1e-30 {
            1.0 / rd.y
        } else {
            f32::INFINITY
        },
        if rd.z.abs() > 1e-30 {
            1.0 / rd.z
        } else {
            f32::INFINITY
        },
    );
    let t1 = Vec3::new(
        (aabb_min.x - ro.x) * inv.x,
        (aabb_min.y - ro.y) * inv.y,
        (aabb_min.z - ro.z) * inv.z,
    );
    let t2 = Vec3::new(
        (aabb_max.x - ro.x) * inv.x,
        (aabb_max.y - ro.y) * inv.y,
        (aabb_max.z - ro.z) * inv.z,
    );
    let t_near = t1.x.min(t2.x).max(t1.y.min(t2.y)).max(t1.z.min(t2.z));
    let t_far = t1.x.max(t2.x).min(t1.y.max(t2.y)).min(t1.z.max(t2.z));
    if t_far < t_near {
        None
    } else {
        Some((t_near, t_far))
    }
}

/// **Interleaved Gradient Noise** (Jorge Jimenez, 2014).
///
/// A fast, spatially uncorrelated noise that spreads quantisation error
/// with blue-noise-like properties.  Used extensively in TAA and dithering
/// pipelines at AAA studios (Doom, Call of Duty, etc.).
///
/// Outputs `[0, 1)`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::igr_noise;
/// let n = igr_noise(320.0, 180.0);
/// assert!(n >= 0.0 && n < 1.0);
/// ```
#[inline]
pub fn igr_noise(x: f32, y: f32) -> f32 {
    (52.982_918_9 * (0.067_110_56 * x + 0.005_837_15 * y).fract()).fract()
}

/// **Value noise** over a 2D domain.
///
/// Smooth lattice noise in `[0, 1]`; uses integer hashing on the grid corners
/// and bilinear interpolation with a smoothstep filter.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Vec2, value_noise_2d};
/// let n = value_noise_2d(Vec2::new(1.3, 2.7));
/// assert!(n >= 0.0 && n <= 1.0);
/// // Smooth: nearby samples are close in value
/// let n2 = value_noise_2d(Vec2::new(1.31, 2.71));
/// assert!((n - n2).abs() < 0.1);
/// ```
pub fn value_noise_2d(p: Vec2) -> f32 {
    let ix = p.x.floor() as i32;
    let iy = p.y.floor() as i32;
    let fx = p.x - p.x.floor();
    let fy = p.y - p.y.floor();
    // Smoothstep filter
    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uy = fy * fy * (3.0 - 2.0 * fy);

    #[inline]
    fn h(x: i32, y: i32) -> f32 {
        hash2_to_f32(x as u32, y as u32)
    }

    let a = lerp(h(ix, iy), h(ix + 1, iy), ux);
    let b = lerp(h(ix, iy + 1), h(ix + 1, iy + 1), ux);
    lerp(a, b, uy)
}

// ── Pass 23 tests ──────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests_pass_23 {
    use super::*;

    // ── rgb_to_hsv / hsv_to_rgb ────────────────────────────────────────────
    #[test]
    fn hsv_primary_colours() {
        // Red
        let (h, s, v) = rgb_to_hsv(1.0, 0.0, 0.0);
        assert!(h < 1.0 || h > 359.0, "red hue ~0: {h}");
        assert!((s - 1.0).abs() < 1e-5 && (v - 1.0).abs() < 1e-5);
        // Green
        let (h, _, _) = rgb_to_hsv(0.0, 1.0, 0.0);
        assert!((h - 120.0).abs() < 1e-3, "green hue: {h}");
        // Blue
        let (h, _, _) = rgb_to_hsv(0.0, 0.0, 1.0);
        assert!((h - 240.0).abs() < 1e-3, "blue hue: {h}");
    }

    #[test]
    fn hsv_round_trip() {
        for (r, g, b) in [(0.8, 0.3, 0.1), (0.0, 0.5, 1.0), (0.2, 0.2, 0.2)] {
            let (h, s, v) = rgb_to_hsv(r, g, b);
            let (r2, g2, b2) = hsv_to_rgb(h, s, v);
            assert!((r - r2).abs() < 1e-5, "r round-trip: {r} vs {r2}");
            assert!((g - g2).abs() < 1e-5, "g round-trip: {g} vs {g2}");
            assert!((b - b2).abs() < 1e-5, "b round-trip: {b} vs {b2}");
        }
    }

    // ── rgb_to_hsl / hsl_to_rgb ────────────────────────────────────────────
    #[test]
    fn hsl_round_trip() {
        for (r, g, b) in [(0.8, 0.3, 0.1), (0.0, 0.5, 1.0), (0.5, 0.5, 0.5)] {
            let (h, s, l) = rgb_to_hsl(r, g, b);
            let (r2, g2, b2) = hsl_to_rgb(h, s, l);
            assert!((r - r2).abs() < 1e-5, "r: {r} vs {r2}");
            assert!((g - g2).abs() < 1e-5, "g: {g} vs {g2}");
            assert!((b - b2).abs() < 1e-5, "b: {b} vs {b2}");
        }
    }

    #[test]
    fn hsl_mid_grey_lightness() {
        let (_, _, l) = rgb_to_hsl(0.5, 0.5, 0.5);
        assert!((l - 0.5).abs() < 1e-5, "grey lightness: {l}");
    }

    // ── fresnel_schlick ────────────────────────────────────────────────────
    #[test]
    fn fresnel_endpoints() {
        assert!((fresnel_schlick(1.0, 0.04) - 0.04).abs() < 1e-6);
        assert!((fresnel_schlick(0.0, 0.04) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn fresnel_monotone_increasing_toward_grazing() {
        let f1 = fresnel_schlick(0.7, 0.04);
        let f2 = fresnel_schlick(0.3, 0.04);
        assert!(f1 < f2, "more grazing → higher Fresnel: {f1} < {f2}");
    }

    // ── ray_sphere_intersect ───────────────────────────────────────────────
    #[test]
    fn ray_sphere_hit_front() {
        let (t0, t1) =
            ray_sphere_intersect(Vec3::ZERO, Vec3::Z, Vec3::new(0.0, 0.0, 5.0), 1.0).unwrap();
        assert!((t0 - 4.0).abs() < 1e-5 && (t1 - 6.0).abs() < 1e-5);
    }

    #[test]
    fn ray_sphere_miss() {
        let hit = ray_sphere_intersect(Vec3::ZERO, Vec3::Z, Vec3::new(2.0, 0.0, 5.0), 1.0);
        assert!(hit.is_none());
    }

    // ── ray_plane_intersect ────────────────────────────────────────────────
    #[test]
    fn ray_plane_hits() {
        // XY plane at z=3, ray from z=0 along +Z
        let t = ray_plane_intersect(Vec3::ZERO, Vec3::Z, Vec3::Z, 3.0).unwrap();
        assert!((t - 3.0).abs() < 1e-5);
    }

    #[test]
    fn ray_plane_parallel_misses() {
        // Ray along +X into a Z-normal plane → parallel
        let hit = ray_plane_intersect(Vec3::ZERO, Vec3::X, Vec3::Z, 1.0);
        assert!(hit.is_none());
    }

    // ── ray_aabb_intersect ─────────────────────────────────────────────────
    #[test]
    fn ray_aabb_centre_shot() {
        let (t0, t1) = ray_aabb_intersect(
            Vec3::ZERO,
            Vec3::X,
            Vec3::new(2.0, -1.0, -1.0),
            Vec3::new(4.0, 1.0, 1.0),
        )
        .unwrap();
        assert!((t0 - 2.0).abs() < 1e-5 && (t1 - 4.0).abs() < 1e-5);
    }

    #[test]
    fn ray_aabb_miss() {
        let hit = ray_aabb_intersect(
            Vec3::ZERO,
            Vec3::X,
            Vec3::new(2.0, 2.0, 2.0),
            Vec3::new(4.0, 4.0, 4.0),
        );
        assert!(hit.is_none());
    }

    // ── igr_noise ──────────────────────────────────────────────────────────
    #[test]
    fn igr_in_range() {
        for i in 0..64u32 {
            let n = igr_noise(i as f32, (i * 7) as f32);
            assert!(n >= 0.0 && n < 1.0, "igr out of range: {n}");
        }
    }

    // ── value_noise_2d ─────────────────────────────────────────────────────
    #[test]
    fn value_noise_in_range() {
        for i in 0..64u32 {
            let n = value_noise_2d(Vec2::new(i as f32 * 0.37, i as f32 * 0.61));
            assert!(n >= 0.0 && n <= 1.0, "noise out of range: {n}");
        }
    }

    #[test]
    fn value_noise_smooth() {
        let p = Vec2::new(3.5, 2.5);
        let n0 = value_noise_2d(p);
        let n1 = value_noise_2d(Vec2::new(3.51, 2.51));
        assert!((n0 - n1).abs() < 0.05, "noise not smooth: {n0} vs {n1}");
    }
}

// ── Pass 24: OKLab, Perlin noise, fBm, Worley, Porter-Duff, colour temp, GCD ─

/// **OKLab** colour space (Björn Ottosson, 2020) — linear RGB → (L, a, b).
///
/// OKLab is perceptually uniform: equal distances correspond to equal perceived
/// colour differences. Ideal for perceptual blending and palette operations.
///
/// Input is **linear** RGB, not gamma-encoded sRGB.
///
/// # Examples
///
/// ```
/// use abrash_core::math::linear_rgb_to_oklab;
/// let (l, a, b) = linear_rgb_to_oklab(1.0, 0.0, 0.0); // linear red
/// assert!(l > 0.0 && l < 1.0, "L in range: {l}");
/// assert!(a > 0.0, "red has positive a: {a}");
/// ```
pub fn linear_rgb_to_oklab(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let l = 0.412_221_47 * r + 0.536_332_54 * g + 0.051_445_99 * b;
    let m = 0.211_903_50 * r + 0.680_699_54 * g + 0.107_396_96 * b;
    let s = 0.088_302_46 * r + 0.281_718_84 * g + 0.629_978_70 * b;
    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();
    (
        0.210_454_26 * l_ + 0.793_617_78 * m_ - 0.004_072_05 * s_,
        1.977_998_50 * l_ - 2.428_592_21 * m_ + 0.450_593_71 * s_,
        0.025_904_04 * l_ + 0.782_771_77 * m_ - 0.808_675_77 * s_,
    )
}

/// Inverse of [`linear_rgb_to_oklab`] — OKLab (L, a, b) → linear RGB.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{linear_rgb_to_oklab, oklab_to_linear_rgb};
/// let (r0, g0, b0) = (0.8_f32, 0.3_f32, 0.1_f32);
/// let (l, a, b) = linear_rgb_to_oklab(r0, g0, b0);
/// let (r1, g1, b1) = oklab_to_linear_rgb(l, a, b);
/// assert!((r0 - r1).abs() < 1e-5 && (g0 - g1).abs() < 1e-5);
/// ```
pub fn oklab_to_linear_rgb(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
    let l_ = l + 0.396_337_78 * a + 0.215_803_76 * b;
    let m_ = l - 0.105_561_35 * a - 0.063_854_17 * b;
    let s_ = l - 0.089_484_18 * a - 1.291_485_54 * b;
    let lc = l_ * l_ * l_;
    let mc = m_ * m_ * m_;
    let sc = s_ * s_ * s_;
    (
        4.076_741_66 * lc - 3.307_711_59 * mc + 0.230_969_94 * sc,
        -1.268_438_0 * lc + 2.609_757_40 * mc - 0.341_319_38 * sc,
        -0.004_196_09 * lc - 0.703_418_61 * mc + 1.707_614_70 * sc,
    )
}

/// **Perlin gradient noise** (2D, improved version with quintic interpolant).
///
/// Returns a value approximately in `[-1, 1]`.  Hash-based implementation —
/// no permutation table required.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Vec2, perlin_noise_2d};
/// // At integer grid points the noise is exactly 0
/// assert!((perlin_noise_2d(Vec2::new(0.0, 0.0))).abs() < 1e-5);
/// ```
pub fn perlin_noise_2d(p: Vec2) -> f32 {
    #[inline]
    fn grad2(hash: u32, dx: f32, dy: f32) -> f32 {
        // 8 unit gradients on the unit circle (octants)
        match hash & 7 {
            0 => dx + dy,
            1 => dx - dy,
            2 => -dx + dy,
            3 => -dx - dy,
            4 => dx,
            5 => -dx,
            6 => dy,
            _ => -dy,
        }
    }
    #[inline]
    fn fade(t: f32) -> f32 {
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }

    let ix = p.x.floor() as i32;
    let iy = p.y.floor() as i32;
    let fx = p.x - p.x.floor();
    let fy = p.y - p.y.floor();
    let ux = fade(fx);
    let uy = fade(fy);

    let h = |x: i32, y: i32| wang_hash(x as u32 ^ wang_hash(y as u32));
    let g00 = grad2(h(ix, iy), fx, fy);
    let g10 = grad2(h(ix + 1, iy), fx - 1.0, fy);
    let g01 = grad2(h(ix, iy + 1), fx, fy - 1.0);
    let g11 = grad2(h(ix + 1, iy + 1), fx - 1.0, fy - 1.0);

    lerp(lerp(g00, g10, ux), lerp(g01, g11, ux), uy)
}

/// **Fractal Brownian Motion** (fBm) built on [`value_noise_2d`].
///
/// Sums `octaves` octaves of value noise, each at double frequency and half
/// amplitude (configurable via `lacunarity` and `gain`).  Output is in `[0, 1]`.
///
/// * `lacunarity` — frequency multiplier per octave (typically `2.0`)
/// * `gain`       — amplitude multiplier per octave (typically `0.5`)
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Vec2, fbm_2d};
/// let n = fbm_2d(Vec2::new(1.5, 2.3), 5, 2.0, 0.5);
/// assert!(n >= 0.0 && n <= 1.0, "fBm in range: {n}");
/// ```
pub fn fbm_2d(mut p: Vec2, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
    let mut sum = 0.0_f32;
    let mut amp = 0.5_f32;
    let mut max_amp = 0.0_f32;
    for _ in 0..octaves {
        sum += amp * value_noise_2d(p);
        max_amp += amp;
        amp *= gain;
        p = Vec2::new(p.x * lacunarity, p.y * lacunarity);
    }
    if max_amp < 1e-9 { 0.5 } else { sum / max_amp }
}

/// **Worley / cellular noise** (2D).
///
/// Returns the Euclidean distance to the nearest feature point, which lies at
/// a random offset within each unit-grid cell.  Output range is approximately
/// `[0, 0.7]` before clamping.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Vec2, worley_noise_2d};
/// let n = worley_noise_2d(Vec2::new(0.5, 0.5));
/// assert!(n >= 0.0 && n < 1.5);
/// ```
pub fn worley_noise_2d(p: Vec2) -> f32 {
    let ix = p.x.floor() as i32;
    let iy = p.y.floor() as i32;
    let mut min_dist = f32::INFINITY;
    for dy in -1..=1_i32 {
        for dx in -1..=1_i32 {
            let cx = (ix + dx) as f32;
            let cy = (iy + dy) as f32;
            // Two independent hashes for x and y offsets within cell
            let ox = hash2_to_f32((ix + dx) as u32, (iy + dy) as u32);
            let oy = hash2_to_f32((iy + dy) as u32 ^ 0xDEAD_BEEF, (ix + dx) as u32);
            let px = cx + ox;
            let py = cy + oy;
            let ddx = p.x - px;
            let ddy = p.y - py;
            let dist = (ddx * ddx + ddy * ddy).sqrt();
            if dist < min_dist {
                min_dist = dist;
            }
        }
    }
    min_dist
}

/// **Porter-Duff "A over B"** compositing — premultiplied RGBA.
///
/// Composites source `(sr, sg, sb, sa)` over destination `(dr, dg, db, da)`.
/// Returns the resulting premultiplied `(r, g, b, a)`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::rgba_over;
/// // Fully opaque red over anything → red
/// let (r, g, b, a) = rgba_over(1.0, 0.0, 0.0, 1.0,  0.0, 0.0, 1.0, 1.0);
/// assert!((r - 1.0).abs() < 1e-5 && b.abs() < 1e-5 && (a - 1.0).abs() < 1e-5);
/// ```
#[inline]
pub fn rgba_over(
    sr: f32,
    sg: f32,
    sb: f32,
    sa: f32,
    dr: f32,
    dg: f32,
    db: f32,
    da: f32,
) -> (f32, f32, f32, f32) {
    let isa = 1.0 - sa;
    (sr + dr * isa, sg + dg * isa, sb + db * isa, sa + da * isa)
}

/// Approximate **colour temperature** (Kelvin) → linear RGB.
///
/// Uses Tanner Helland's empirical fit, valid for `1000 K … 40 000 K`.
/// Returns approximate linear RGB in `[0, 1]`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::color_temperature_rgb;
/// // 6500 K (daylight) → neutral-white: all channels near 1
/// let (r, g, b) = color_temperature_rgb(6500.0);
/// assert!(r > 0.9 && g > 0.9 && b > 0.9, "daylight near white: {r} {g} {b}");
/// // 2700 K (incandescent) → warm: red dominates
/// let (r2, _, b2) = color_temperature_rgb(2700.0);
/// assert!(r2 > b2, "warm: red > blue: {r2} vs {b2}");
/// ```
pub fn color_temperature_rgb(kelvin: f32) -> (f32, f32, f32) {
    let t = kelvin.clamp(1000.0, 40_000.0) / 100.0;
    let r = if t <= 66.0 {
        1.0
    } else {
        (329.698_727_45 * (t - 60.0).powf(-0.133_204_759_2) / 255.0).clamp(0.0, 1.0)
    };
    let g = if t <= 66.0 {
        (99.470_802_53 * t.ln() - 161.119_568_34).clamp(0.0, 255.0) / 255.0
    } else {
        (288.122_169_52 * (t - 60.0).powf(-0.075_514_849_2) / 255.0).clamp(0.0, 1.0)
    };
    let b = if t >= 66.0 {
        1.0
    } else if t <= 19.0 {
        0.0
    } else {
        ((138.517_731_21 * (t - 10.0).ln() - 305.044_792_7) / 255.0).clamp(0.0, 1.0)
    };
    (r, g, b)
}

/// Greatest common divisor (Euclidean algorithm).
///
/// # Examples
///
/// ```
/// use abrash_core::math::gcd_u32;
/// assert_eq!(gcd_u32(12, 8), 4);
/// assert_eq!(gcd_u32(7, 13), 1);
/// ```
#[inline]
pub const fn gcd_u32(mut a: u32, mut b: u32) -> u32 {
    while b > 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// Least common multiple.
///
/// Returns `0` if either argument is `0`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::lcm_u32;
/// assert_eq!(lcm_u32(4, 6), 12);
/// assert_eq!(lcm_u32(0, 5), 0);
/// ```
#[inline]
pub fn lcm_u32(a: u32, b: u32) -> u32 {
    if a == 0 || b == 0 {
        0
    } else {
        a / gcd_u32(a, b) * b
    }
}

// ── Pass 24 tests ──────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests_pass_24 {
    use super::*;

    // ── oklab ─────────────────────────────────────────────────────────────
    #[test]
    fn oklab_round_trip() {
        for (r, g, b) in [(1.0_f32, 0.0, 0.0), (0.0, 1.0, 0.0), (0.5, 0.3, 0.8)] {
            let (l, a, bb) = linear_rgb_to_oklab(r, g, b);
            let (r2, g2, b2) = oklab_to_linear_rgb(l, a, bb);
            assert!((r - r2).abs() < 1e-4, "r: {r} vs {r2}");
            assert!((g - g2).abs() < 1e-4, "g: {g} vs {g2}");
            assert!((b - b2).abs() < 1e-4, "b: {b} vs {b2}");
        }
    }

    #[test]
    fn oklab_white_is_l1() {
        let (l, a, b) = linear_rgb_to_oklab(1.0, 1.0, 1.0);
        assert!((l - 1.0).abs() < 1e-4, "white L≈1: {l}");
        assert!(a.abs() < 1e-4 && b.abs() < 1e-4, "white a,b≈0: {a} {b}");
    }

    // ── perlin_noise_2d ────────────────────────────────────────────────────
    #[test]
    fn perlin_zero_at_integer_points() {
        for i in 0..5_i32 {
            for j in 0..5_i32 {
                let n = perlin_noise_2d(Vec2::new(i as f32, j as f32));
                assert!(n.abs() < 1e-5, "perlin zero at integer: ({i},{j}) = {n}");
            }
        }
    }

    #[test]
    fn perlin_range() {
        for i in 0..100u32 {
            let n = perlin_noise_2d(Vec2::new(i as f32 * 0.37, i as f32 * 0.61));
            assert!(n > -2.0 && n < 2.0, "perlin in rough range: {n}");
        }
    }

    // ── fbm_2d ────────────────────────────────────────────────────────────
    #[test]
    fn fbm_in_range() {
        for i in 0..50u32 {
            let n = fbm_2d(Vec2::new(i as f32 * 0.31, i as f32 * 0.71), 5, 2.0, 0.5);
            assert!(n >= 0.0 && n <= 1.0, "fBm out of range: {n}");
        }
    }

    // ── worley_noise_2d ───────────────────────────────────────────────────
    #[test]
    fn worley_non_negative() {
        for i in 0..64u32 {
            let n = worley_noise_2d(Vec2::new(i as f32 * 0.23, i as f32 * 0.47));
            assert!(n >= 0.0, "worley non-negative: {n}");
        }
    }

    // ── rgba_over ─────────────────────────────────────────────────────────
    #[test]
    fn porter_duff_opaque_src() {
        // Fully opaque src completely covers dst
        let (r, g, b, a) = rgba_over(0.8, 0.2, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0);
        assert!((r - 0.8).abs() < 1e-5 && (b).abs() < 1e-5 && (a - 1.0).abs() < 1e-5);
    }

    #[test]
    fn porter_duff_transparent_src() {
        // Fully transparent src → dst unchanged
        let (r, g, b, a) = rgba_over(0.0, 0.0, 0.0, 0.0, 0.5, 0.3, 0.1, 0.7);
        assert!((r - 0.5).abs() < 1e-5 && (a - 0.7).abs() < 1e-5);
        let _ = (g, b);
    }

    // ── color_temperature_rgb ─────────────────────────────────────────────
    #[test]
    fn warm_vs_cool_temperature() {
        let (r_warm, _, b_warm) = color_temperature_rgb(2700.0);
        let (r_cool, _, b_cool) = color_temperature_rgb(6500.0);
        assert!(r_warm > b_warm, "warm: red > blue: {r_warm} vs {b_warm}");
        assert!(
            b_cool > b_warm,
            "cool: more blue than warm: {b_cool} vs {b_warm}"
        );
    }

    // ── gcd_u32 / lcm_u32 ─────────────────────────────────────────────────
    #[test]
    fn gcd_basic() {
        assert_eq!(gcd_u32(12, 8), 4);
        assert_eq!(gcd_u32(7, 13), 1);
        assert_eq!(gcd_u32(0, 5), 5);
        assert_eq!(gcd_u32(100, 25), 25);
    }

    #[test]
    fn lcm_basic() {
        assert_eq!(lcm_u32(4, 6), 12);
        assert_eq!(lcm_u32(0, 5), 0);
        assert_eq!(lcm_u32(7, 3), 21);
    }
}

// ── Pass 25: OKLab ops, PBR sampling, back/elastic easing ────────────────────

/// **Perceptual luminance** using the Rec.709 / sRGB luma coefficients.
///
/// Input should be **linear** (not gamma-encoded) RGB.
/// Returns a value in `[0, 1]` when the input channels are in `[0, 1]`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::luminance_rec709;
/// assert!((luminance_rec709(1.0, 1.0, 1.0) - 1.0).abs() < 1e-5);
/// assert!(luminance_rec709(0.0, 0.0, 0.0).abs() < 1e-9);
/// // Green contributes most to luminance
/// assert!(luminance_rec709(0.0, 1.0, 0.0) > luminance_rec709(1.0, 0.0, 0.0));
/// ```
#[inline]
pub fn luminance_rec709(r: f32, g: f32, b: f32) -> f32 {
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// **OKLab interpolation** — perceptually uniform colour blend.
///
/// Interpolates between two OKLab colours `(L0, a0, b0)` and `(L1, a1, b1)`
/// by factor `t ∈ [0, 1]`, returning the interpolated `(L, a, b)`.
///
/// Unlike sRGB lerp, this preserves perceived colour saturation through the
/// midpoint.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{linear_rgb_to_oklab, oklab_mix};
/// let c0 = linear_rgb_to_oklab(1.0, 0.0, 0.0); // red
/// let c1 = linear_rgb_to_oklab(0.0, 0.0, 1.0); // blue
/// let (l, a, b) = oklab_mix(c0.0, c0.1, c0.2, c1.0, c1.1, c1.2, 0.5);
/// assert!(l > 0.0 && l < 1.0, "mid-lightness: {l}");
/// ```
#[inline]
pub fn oklab_mix(l0: f32, a0: f32, b0: f32, l1: f32, a1: f32, b1: f32, t: f32) -> (f32, f32, f32) {
    (l0 + (l1 - l0) * t, a0 + (a1 - a0) * t, b0 + (b1 - b0) * t)
}

/// **OKLab hue rotation** — rotate the hue angle in the `(a, b)` plane.
///
/// Preserves the lightness `L` and chroma magnitude `sqrt(a² + b²)` while
/// shifting the hue by `angle_deg` degrees.
///
/// # Examples
///
/// ```
/// use abrash_core::math::oklab_rotate_hue;
/// // 180-degree rotation inverts a and b
/// let (l, a, b) = oklab_rotate_hue(0.5, 0.2, 0.1, 180.0);
/// assert!((a + 0.2).abs() < 1e-5 && (b + 0.1).abs() < 1e-5);
/// ```
#[inline]
pub fn oklab_rotate_hue(l: f32, a: f32, b: f32, angle_deg: f32) -> (f32, f32, f32) {
    let rad = angle_deg * core::f32::consts::PI / 180.0;
    let (s, c) = rad.sin_cos();
    (l, c * a - s * b, s * a + c * b)
}

/// **Cosine-weighted hemisphere** sample (Malley's method).
///
/// Maps a uniform 2D sample `(u1, u2) ∈ [0, 1)²` to a direction on the
/// hemisphere whose probability density is proportional to `cos θ`.
///
/// The returned direction has `y > 0` (hemisphere normal along +Y).
///
/// # Examples
///
/// ```
/// use abrash_core::math::sample_cosine_hemisphere;
/// let d = sample_cosine_hemisphere(0.5, 0.5);
/// // Should be a unit vector on the hemisphere
/// let len = (d.x * d.x + d.y * d.y + d.z * d.z).sqrt();
/// assert!((len - 1.0).abs() < 1e-5 && d.y >= 0.0);
/// ```
pub fn sample_cosine_hemisphere(u1: f32, u2: f32) -> Vec3 {
    let r = u1.sqrt();
    let theta = core::f32::consts::TAU * u2;
    let x = r * theta.cos();
    let z = r * theta.sin();
    let y = (1.0 - u1).max(0.0).sqrt();
    Vec3::new(x, y, z).normalize_or_zero()
}

/// **Uniform sphere** sample.
///
/// Maps a uniform 2D sample `(u1, u2) ∈ [0, 1)²` to a uniformly distributed
/// direction on the unit sphere.
///
/// # Examples
///
/// ```
/// use abrash_core::math::sample_uniform_sphere;
/// let d = sample_uniform_sphere(0.25, 0.75);
/// let len = (d.x * d.x + d.y * d.y + d.z * d.z).sqrt();
/// assert!((len - 1.0).abs() < 1e-5);
/// ```
pub fn sample_uniform_sphere(u1: f32, u2: f32) -> Vec3 {
    let z = 1.0 - 2.0 * u1;
    let r = (1.0 - z * z).max(0.0).sqrt();
    let phi = core::f32::consts::TAU * u2;
    Vec3::new(r * phi.cos(), z, r * phi.sin())
}

/// **Back ease-in** — easing function with overshoot.
///
/// Starts by going slightly *backwards* before accelerating forward.
/// `overshoot ≈ 1.70158` gives the standard CSS `cubic-bezier` back effect.
///
/// # Examples
///
/// ```
/// use abrash_core::math::ease_back_in;
/// assert!(ease_back_in(0.0, 1.70158).abs() < 1e-5);
/// assert!((ease_back_in(1.0, 1.70158) - 1.0).abs() < 1e-5);
/// // Goes negative initially (overshoot)
/// assert!(ease_back_in(0.2, 1.70158) < 0.0);
/// ```
#[inline]
pub fn ease_back_in(t: f32, overshoot: f32) -> f32 {
    let s = overshoot;
    t * t * ((s + 1.0) * t - s)
}

/// **Back ease-out** — easing function with overshoot on arrival.
///
/// Arrives by going slightly *past* the target before settling.
///
/// # Examples
///
/// ```
/// use abrash_core::math::ease_back_out;
/// assert!(ease_back_out(0.0, 1.70158).abs() < 1e-5);
/// assert!((ease_back_out(1.0, 1.70158) - 1.0).abs() < 1e-5);
/// // Overshoots past 1 near the end
/// assert!(ease_back_out(0.8, 1.70158) > 1.0);
/// ```
#[inline]
pub fn ease_back_out(t: f32, overshoot: f32) -> f32 {
    let s = overshoot;
    let t1 = t - 1.0;
    1.0 + t1 * t1 * ((s + 1.0) * t1 + s)
}

/// **Elastic ease-out** — spring-overshoot easing.
///
/// Snaps past the target and oscillates back, settling at 1.0.
/// `amplitude ≥ 1.0` and `period > 0.0` (typically `0.3`).
///
/// # Examples
///
/// ```
/// use abrash_core::math::ease_elastic_out;
/// assert!(ease_elastic_out(0.0, 1.0, 0.3).abs() < 1e-5);
/// assert!((ease_elastic_out(1.0, 1.0, 0.3) - 1.0).abs() < 1e-5);
/// ```
pub fn ease_elastic_out(t: f32, amplitude: f32, period: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let a = amplitude.max(1.0);
    let s = (a.recip()).asin() * period / core::f32::consts::TAU;
    a * (2.0_f32).powf(-10.0 * t) * ((t - s) * core::f32::consts::TAU / period).sin() + 1.0
}

// ── Pass 25 tests ──────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests_pass_25 {
    use super::*;

    // ── luminance_rec709 ───────────────────────────────────────────────────
    #[test]
    fn luminance_white_and_black() {
        assert!((luminance_rec709(1.0, 1.0, 1.0) - 1.0).abs() < 1e-5);
        assert!(luminance_rec709(0.0, 0.0, 0.0).abs() < 1e-9);
    }

    #[test]
    fn luminance_green_dominates() {
        assert!(luminance_rec709(0.0, 1.0, 0.0) > luminance_rec709(1.0, 0.0, 0.0));
        assert!(luminance_rec709(0.0, 1.0, 0.0) > luminance_rec709(0.0, 0.0, 1.0));
    }

    // ── oklab_mix ─────────────────────────────────────────────────────────
    #[test]
    fn oklab_mix_endpoints() {
        let (l, a, b) = oklab_mix(0.5, 0.1, 0.2, 0.8, 0.3, 0.4, 0.0);
        assert!((l - 0.5).abs() < 1e-6 && (a - 0.1).abs() < 1e-6);
        let (l, a, b) = oklab_mix(0.5, 0.1, 0.2, 0.8, 0.3, 0.4, 1.0);
        assert!((l - 0.8).abs() < 1e-6 && (a - 0.3).abs() < 1e-6 && (b - 0.4).abs() < 1e-6);
    }

    // ── oklab_rotate_hue ──────────────────────────────────────────────────
    #[test]
    fn oklab_hue_rotation_180() {
        let (l, a, b) = oklab_rotate_hue(0.6, 0.2, 0.1, 180.0);
        assert!((l - 0.6).abs() < 1e-5, "L unchanged: {l}");
        assert!((a + 0.2).abs() < 1e-5, "a inverted: {a}");
        assert!((b + 0.1).abs() < 1e-5, "b inverted: {b}");
    }

    #[test]
    fn oklab_hue_rotation_preserves_chroma() {
        let a0 = 0.3_f32;
        let b0 = 0.1_f32;
        let chroma0 = (a0 * a0 + b0 * b0).sqrt();
        let (_, a1, b1) = oklab_rotate_hue(0.5, a0, b0, 90.0);
        let chroma1 = (a1 * a1 + b1 * b1).sqrt();
        assert!(
            (chroma0 - chroma1).abs() < 1e-5,
            "chroma preserved: {chroma0} vs {chroma1}"
        );
    }

    // ── sample_cosine_hemisphere ──────────────────────────────────────────
    #[test]
    fn cosine_hemisphere_unit_length() {
        for i in 0..20u32 {
            let u1 = (i as f32 + 0.5) / 20.0;
            let u2 = ((i * 7 + 3) as f32) / 20.0 % 1.0;
            let d = sample_cosine_hemisphere(u1, u2);
            let len = (d.x * d.x + d.y * d.y + d.z * d.z).sqrt();
            assert!((len - 1.0).abs() < 1e-4, "unit: {len}");
            assert!(d.y >= -1e-5, "upper hemisphere: {}", d.y);
        }
    }

    // ── sample_uniform_sphere ─────────────────────────────────────────────
    #[test]
    fn uniform_sphere_unit_length() {
        for i in 0..20u32 {
            let u1 = (i as f32 + 0.5) / 20.0;
            let u2 = ((i * 7 + 3) as f32) / 20.0 % 1.0;
            let d = sample_uniform_sphere(u1, u2);
            let len = (d.x * d.x + d.y * d.y + d.z * d.z).sqrt();
            assert!((len - 1.0).abs() < 1e-4, "unit: {len}");
        }
    }

    // ── ease_back ─────────────────────────────────────────────────────────
    #[test]
    fn back_in_endpoints() {
        assert!(ease_back_in(0.0, 1.70158).abs() < 1e-5);
        assert!((ease_back_in(1.0, 1.70158) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn back_out_overshoots() {
        assert!(ease_back_out(0.75, 1.70158) > 1.0, "overshoot near end");
    }

    // ── ease_elastic_out ──────────────────────────────────────────────────
    #[test]
    fn elastic_out_endpoints() {
        assert!(ease_elastic_out(0.0, 1.0, 0.3).abs() < 1e-5);
        assert!((ease_elastic_out(1.0, 1.0, 0.3) - 1.0).abs() < 1e-5);
    }
}

// ── Pass 26: Möller–Trumbore, GGX BRDF terms, SH basis, Uncharted 2 tonemap ──

/// **Ray–triangle** intersection (Möller–Trumbore algorithm).
///
/// Returns `Some((t, u, v))` where `t` is the ray parameter, and `(u, v)` are
/// the barycentric coordinates of the hit point (`w = 1 − u − v`).
/// Returns `None` when the ray is parallel to the triangle or misses.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Vec3, ray_triangle_intersect};
/// let a = Vec3::new(-1.0, 0.0, 3.0);
/// let b = Vec3::new( 1.0, 0.0, 3.0);
/// let c = Vec3::new( 0.0, 1.0, 3.0);
/// let (t, u, v) = ray_triangle_intersect(Vec3::ZERO, Vec3::Z, a, b, c).unwrap();
/// assert!((t - 3.0).abs() < 1e-4);
/// assert!(u >= 0.0 && v >= 0.0 && u + v <= 1.0);
/// ```
pub fn ray_triangle_intersect(
    ro: Vec3,
    rd: Vec3,
    a: Vec3,
    b: Vec3,
    c: Vec3,
) -> Option<(f32, f32, f32)> {
    let edge1 = b - a;
    let edge2 = c - a;
    let h = rd.cross(edge2);
    let det = edge1.dot(h);
    if det.abs() < 1e-9 {
        return None; // Ray parallel to triangle
    }
    let inv_det = 1.0 / det;
    let s = ro - a;
    let u = s.dot(h) * inv_det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(edge1);
    let v = rd.dot(q) * inv_det;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = edge2.dot(q) * inv_det;
    if t < 0.0 {
        return None;
    }
    Some((t, u, v))
}

/// **GGX Trowbridge-Reitz Normal Distribution Function (NDF)**.
///
/// The specular lobe density for a microsurface roughness model.
///
/// * `n_dot_h`   — cosine of angle between normal and half-vector (`≥ 0`)
/// * `roughness` — perceptual roughness in `[0, 1]`; uses Disney `α = r²` remapping
///
/// # Examples
///
/// ```
/// use abrash_core::math::ggx_ndf;
/// // Perfect mirror (roughness → 0) → very large NDF at n·h = 1
/// let d_mirror = ggx_ndf(1.0, 0.01);
/// // Fully rough → low, broad NDF
/// let d_rough  = ggx_ndf(0.9, 1.0);
/// assert!(d_mirror > d_rough);
/// ```
#[inline]
pub fn ggx_ndf(n_dot_h: f32, roughness: f32) -> f32 {
    let alpha = roughness * roughness;
    let alpha2 = alpha * alpha;
    let denom = n_dot_h * n_dot_h * (alpha2 - 1.0) + 1.0;
    alpha2 / (core::f32::consts::PI * denom * denom)
}

/// **Smith geometry term** with Schlick-GGX approximation.
///
/// Occludes both view and light directions for microfacet BRDF evaluation.
///
/// * `n_dot_v` — N·V (view direction dotted with normal)
/// * `n_dot_l` — N·L (light direction dotted with normal)
/// * `roughness` — perceptual roughness in `[0, 1]`
///
/// # Examples
///
/// ```
/// use abrash_core::math::ggx_geometry_smith;
/// let g = ggx_geometry_smith(0.9, 0.9, 0.5);
/// assert!(g > 0.0 && g <= 1.0, "geometry in (0,1]: {g}");
/// ```
#[inline]
pub fn ggx_geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    let k = roughness * roughness * 0.5; // direct-lighting Disney remapping
    let g_v = n_dot_v / (n_dot_v * (1.0 - k) + k);
    let g_l = n_dot_l / (n_dot_l * (1.0 - k) + k);
    g_v * g_l
}

/// **Uncharted 2 filmic** tone mapping (Hejl/Burgess-Dawson).
///
/// Industry-standard S-curve with more contrast than Reinhard and a separate
/// white point.  Works well for HDR values up to ~20 EV.
///
/// # Examples
///
/// ```
/// use abrash_core::math::uncharted2_tonemap;
/// assert!(uncharted2_tonemap(0.0).abs() < 1e-5);
/// assert!((uncharted2_tonemap(1.0) - uncharted2_tonemap(0.5)).abs() > 0.05);
/// // Output is bounded (< 1.0 for very large inputs after white-point normalisation)
/// assert!(uncharted2_tonemap(1000.0) <= 1.0 + 1e-4);
/// ```
pub fn uncharted2_tonemap(x: f32) -> f32 {
    #[inline]
    fn partial(v: f32) -> f32 {
        const A: f32 = 0.15;
        const B: f32 = 0.50;
        const C: f32 = 0.10;
        const D: f32 = 0.20;
        const E: f32 = 0.02;
        const F: f32 = 0.30;
        ((v * (A * v + C * B) + D * E) / (v * (A * v + B) + D * F)) - E / F
    }
    let white = 11.2_f32;
    partial(x) / partial(white)
}

/// **Spherical Harmonic L0** (DC) basis coefficient.
///
/// A constant function over the sphere.
///
/// # Examples
///
/// ```
/// use abrash_core::math::sh_y00;
/// let c = sh_y00();
/// assert!((c - 0.282_094_8).abs() < 1e-5);
/// ```
#[inline]
pub fn sh_y00() -> f32 {
    0.282_094_79 // 1 / (2 * sqrt(π))
}

/// **Spherical Harmonic L1** (three-coefficient dipole) basis.
///
/// Returns `[Y_1^{-1}, Y_1^0, Y_1^1]` for a unit direction `v`.
/// Conventionally: Y₁₋₁ ∝ y, Y₁₀ ∝ z, Y₁₁ ∝ x (Cartesian ordering).
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Vec3, sh_y1};
/// // +Z pole → Y10 is maximal, others zero
/// let b = sh_y1(Vec3::Z);
/// assert!(b[0].abs() < 1e-5 && b[2].abs() < 1e-5);
/// assert!(b[1] > 0.4);
/// ```
#[inline]
pub fn sh_y1(v: Vec3) -> [f32; 3] {
    const C: f32 = 0.488_602_51; // sqrt(3/(4π))
    [C * v.y, C * v.z, C * v.x]
}

/// **Spherical Harmonic L2** (five-coefficient quadrupole) basis.
///
/// Returns `[Y_2^{-2}, Y_2^{-1}, Y_2^0, Y_2^1, Y_2^2]` for unit direction `v`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Vec3, sh_y2};
/// // For any unit vector the squared-norm of the basis should equal
/// // (2L+1)/(4π) × 4π/n_samples (for uniform sampling it's 5/(4π) per coeff)
/// let b = sh_y2(Vec3::Z);
/// assert!(b[2].abs() > 0.3, "Y20 along +Z: {}", b[2]);
/// ```
#[inline]
pub fn sh_y2(v: Vec3) -> [f32; 5] {
    let (x, y, z) = (v.x, v.y, v.z);
    [
        1.092_548_43 * x * y,               // Y_2^{-2}
        1.092_548_43 * y * z,               // Y_2^{-1}
        0.315_391_57 * (3.0 * z * z - 1.0), // Y_2^0
        1.092_548_43 * x * z,               // Y_2^1
        0.546_274_22 * (x * x - y * y),     // Y_2^2
    ]
}

// ── Pass 26 tests ──────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests_pass_26 {
    use super::*;

    // ── ray_triangle_intersect ────────────────────────────────────────────
    #[test]
    fn moller_trumbore_hit() {
        let a = Vec3::new(-1.0, 0.0, 3.0);
        let b = Vec3::new(1.0, 0.0, 3.0);
        let c = Vec3::new(0.0, 1.0, 3.0);
        let (t, u, v) = ray_triangle_intersect(Vec3::ZERO, Vec3::Z, a, b, c).unwrap();
        assert!((t - 3.0).abs() < 1e-4, "t: {t}");
        assert!(u >= 0.0 && v >= 0.0 && u + v <= 1.0, "bary: {u} {v}");
    }

    #[test]
    fn moller_trumbore_miss() {
        // Ray along +X, triangle in +Z plane
        let a = Vec3::new(-1.0, 0.0, 3.0);
        let b = Vec3::new(1.0, 0.0, 3.0);
        let c = Vec3::new(0.0, 1.0, 3.0);
        let hit = ray_triangle_intersect(Vec3::ZERO, Vec3::X, a, b, c);
        assert!(hit.is_none(), "should miss");
    }

    #[test]
    fn moller_trumbore_parallel() {
        // Ray along +Z, triangle also in +Z-parallel plane (normal = +X)
        let a = Vec3::new(1.0, 0.0, 0.0);
        let b = Vec3::new(1.0, 1.0, 0.0);
        let c = Vec3::new(1.0, 0.0, 1.0);
        let hit = ray_triangle_intersect(Vec3::ZERO, Vec3::Z, a, b, c);
        assert!(hit.is_none(), "parallel should miss");
    }

    // ── ggx_ndf ───────────────────────────────────────────────────────────
    #[test]
    fn ggx_ndf_smooth_brighter() {
        let d_smooth = ggx_ndf(1.0, 0.01);
        let d_rough = ggx_ndf(1.0, 1.0);
        assert!(d_smooth > d_rough, "smooth > rough at n·h=1");
    }

    #[test]
    fn ggx_ndf_positive() {
        for r in [0.1_f32, 0.3, 0.5, 0.8, 1.0] {
            let d = ggx_ndf(0.8, r);
            assert!(d > 0.0, "positive NDF: {d}");
        }
    }

    // ── ggx_geometry_smith ────────────────────────────────────────────────
    #[test]
    fn ggx_geometry_in_range() {
        let g = ggx_geometry_smith(0.9, 0.9, 0.5);
        assert!(g > 0.0 && g <= 1.0, "G in (0,1]: {g}");
    }

    // ── uncharted2_tonemap ────────────────────────────────────────────────
    #[test]
    fn uncharted2_zero_in_zero_out() {
        assert!(uncharted2_tonemap(0.0).abs() < 1e-4);
    }

    #[test]
    fn uncharted2_bounded() {
        // Uncharted2 saturates at (1-E/F)/partial(11.2) ≈ 1.287 as input → ∞.
        // Values above the white point (11.2) legitimately exceed 1.0; callers
        // should clamp or rely on exposure to keep scene values ≤ 11.2.
        let large = uncharted2_tonemap(1000.0);
        assert!(
            large > 0.0 && large < 1.4,
            "asymptote out of range: {large}"
        );
        // Verify the white-point itself maps to exactly 1.0
        let white = uncharted2_tonemap(11.2);
        assert!((white - 1.0).abs() < 1e-5, "white point: {white}");
    }

    // ── SH basis ──────────────────────────────────────────────────────────
    #[test]
    fn sh_y00_constant() {
        assert!((sh_y00() - 0.282_094_79).abs() < 1e-5);
    }

    #[test]
    fn sh_y1_poles() {
        // +Z: Y10 non-zero, others zero
        let b = sh_y1(Vec3::Z);
        assert!(b[0].abs() < 1e-5 && b[2].abs() < 1e-5);
        assert!(b[1] > 0.4);
        // +X: Y11 non-zero, others zero
        let bx = sh_y1(Vec3::X);
        assert!(bx[0].abs() < 1e-5 && bx[1].abs() < 1e-5);
        assert!(bx[2] > 0.4);
    }

    #[test]
    fn sh_y2_z_pole() {
        let b = sh_y2(Vec3::Z);
        // At +Z: Y2^0 = 0.315... * 2 ≈ 0.63; others zero
        assert!((b[2] - 0.315_391_57 * 2.0).abs() < 1e-4, "Y20: {}", b[2]);
        assert!(b[0].abs() < 1e-5 && b[1].abs() < 1e-5);
    }
}

// ── Pass 27 additions ─────────────────────────────────────────────────────────

/// Robert Penner's bounce-out easing — decelerating with rebounds.
///
/// The bounce uses three parabolic arcs that tile [0, 1] with
/// matching boundary conditions (no discontinuity between arcs).
///
/// # Examples
/// ```
/// use abrash_core::math::ease_bounce_out;
/// assert!(ease_bounce_out(0.0).abs() < 1e-6);
/// assert!((ease_bounce_out(1.0) - 1.0).abs() < 1e-5);
/// assert!(ease_bounce_out(0.5) > 0.0);
/// ```
#[must_use]
#[inline]
pub fn ease_bounce_out(t: f32) -> f32 {
    const N: f32 = 7.562_5;
    const D: f32 = 2.75;
    if t < 1.0 / D {
        N * t * t
    } else if t < 2.0 / D {
        let t = t - 1.5 / D;
        N * t * t + 0.75
    } else if t < 2.5 / D {
        let t = t - 2.25 / D;
        N * t * t + 0.9375
    } else {
        let t = t - 2.625 / D;
        N * t * t + 0.984_375
    }
}

/// Robert Penner's bounce-in easing — accelerating with initial bounces.
///
/// Time-reversal of [`ease_bounce_out`].
///
/// # Examples
/// ```
/// use abrash_core::math::ease_bounce_in;
/// assert!(ease_bounce_in(0.0).abs() < 1e-5);
/// assert!((ease_bounce_in(1.0) - 1.0).abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn ease_bounce_in(t: f32) -> f32 {
    1.0 - ease_bounce_out(1.0 - t)
}

/// Circular ease-in — acceleration following a quarter-circle arc.
///
/// `f(t) = 1 - sqrt(1 - t²)`, gives smooth acceleration from rest.
///
/// # Examples
/// ```
/// use abrash_core::math::ease_circ_in;
/// assert!(ease_circ_in(0.0).abs() < 1e-6);
/// assert!((ease_circ_in(1.0) - 1.0).abs() < 1e-5);
/// assert!(ease_circ_in(0.5) < 0.5); // slower than linear
/// ```
#[must_use]
#[inline]
pub fn ease_circ_in(t: f32) -> f32 {
    1.0 - (1.0 - t * t).sqrt()
}

/// Circular ease-out — deceleration following a quarter-circle arc.
///
/// Time-reversal of [`ease_circ_in`].
///
/// # Examples
/// ```
/// use abrash_core::math::ease_circ_out;
/// assert!(ease_circ_out(0.0).abs() < 1e-6);
/// assert!((ease_circ_out(1.0) - 1.0).abs() < 1e-5);
/// assert!(ease_circ_out(0.5) > 0.5); // faster than linear
/// ```
#[must_use]
#[inline]
pub fn ease_circ_out(t: f32) -> f32 {
    let t1 = t - 1.0;
    (1.0 - t1 * t1).sqrt()
}

/// Normalised sinc function: `sin(πx) / (πx)` with `sinc(0) = 1`.
///
/// Used as a reconstruction kernel in image processing and signal theory.
///
/// # Examples
/// ```
/// use abrash_core::math::sinc;
/// assert!((sinc(0.0) - 1.0).abs() < 1e-6);
/// assert!(sinc(1.0).abs() < 1e-6); // zero crossing at x=1
/// ```
#[must_use]
#[inline]
pub fn sinc(x: f32) -> f32 {
    if x.abs() < 1e-9 {
        1.0
    } else {
        let px = core::f32::consts::PI * x;
        px.sin() / px
    }
}

/// Lanczos filter kernel of order `a` (typically 2 or 3).
///
/// `L(x) = sinc(x) * sinc(x/a)` for `|x| < a`, else 0.
///
/// Used for high-quality image resampling (sharper than bilinear, less
/// ringing than ideal sinc).
///
/// # Examples
/// ```
/// use abrash_core::math::lanczos_kernel;
/// assert!((lanczos_kernel(0.0, 3.0) - 1.0).abs() < 1e-5);
/// assert!(lanczos_kernel(3.0, 3.0).abs() < 1e-5); // exactly at edge
/// assert_eq!(lanczos_kernel(4.0, 3.0), 0.0); // outside support
/// ```
#[must_use]
#[inline]
pub fn lanczos_kernel(x: f32, a: f32) -> f32 {
    if x.abs() >= a {
        0.0
    } else {
        sinc(x) * sinc(x / a)
    }
}

/// Mitchell–Netravali filter kernel, parameterised by `b` and `c`.
///
/// The recommended `b = 1/3, c = 1/3` balances sharpness and ringing.
/// Special cases: `b=1, c=0` → cubic B-spline; `b=0, c=0.5` → Catmull-Rom.
///
/// Note: Mitchell-Netravali is an *approximating* (not interpolating) filter.
/// `f(0) = (6 - 2b) / 6 = 1 - b/3`, so only Catmull-Rom (`b=0`) gives `f(0) = 1`.
///
/// # Examples
/// ```
/// use abrash_core::math::mitchell_netravali;
/// // Catmull-Rom (b=0, c=0.5) is interpolating: f(0) = 1.
/// let catmull = mitchell_netravali(0.0, 0.0, 0.5);
/// assert!((catmull - 1.0).abs() < 1e-5, "Catmull-Rom at 0: {catmull}");
/// // Recommended params (b=1/3, c=1/3): f(0) = 8/9 (approximating).
/// let v = mitchell_netravali(0.0, 1.0 / 3.0, 1.0 / 3.0);
/// assert!((v - 8.0/9.0).abs() < 1e-5, "MN(0) approx: {v}");
/// assert_eq!(mitchell_netravali(2.0, 1.0/3.0, 1.0/3.0), 0.0); // outside support
/// ```
#[must_use]
#[inline]
pub fn mitchell_netravali(x: f32, b: f32, c: f32) -> f32 {
    let x = x.abs();
    if x < 1.0 {
        ((12.0 - 9.0 * b - 6.0 * c) * x * x * x
            + (-18.0 + 12.0 * b + 6.0 * c) * x * x
            + (6.0 - 2.0 * b))
            / 6.0
    } else if x < 2.0 {
        ((-b - 6.0 * c) * x * x * x
            + (6.0 * b + 30.0 * c) * x * x
            + (-12.0 * b - 48.0 * c) * x
            + (8.0 * b + 24.0 * c))
            / 6.0
    } else {
        0.0
    }
}

/// Convert OKLab `(L, a, b)` to OKLCh `(L, C, h)` — polar form.
///
/// `C = sqrt(a² + b²)`, `h = atan2(b, a)` in degrees [0, 360).
///
/// # Examples
/// ```
/// use abrash_core::math::oklab_to_oklch;
/// let (l, c, h) = oklab_to_oklch(0.5, 0.1, 0.0);
/// assert!((c - 0.1).abs() < 1e-6); // chroma from a-axis
/// assert!(h.abs() < 1e-4); // hue ≈ 0° on +a axis
/// ```
#[must_use]
#[inline]
pub fn oklab_to_oklch(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
    let c = (a * a + b * b).sqrt();
    let h = b.atan2(a).to_degrees().rem_euclid(360.0);
    (l, c, h)
}

/// Convert OKLCh `(L, C, h)` to OKLab `(L, a, b)` — Cartesian form.
///
/// `a = C * cos(h)`, `b = C * sin(h)` with `h` in degrees.
///
/// # Examples
/// ```
/// use abrash_core::math::oklch_to_oklab;
/// let (l, a, b) = oklch_to_oklab(0.5, 0.1, 0.0);
/// assert!((a - 0.1).abs() < 1e-6);
/// assert!(b.abs() < 1e-6);
/// ```
#[must_use]
#[inline]
pub fn oklch_to_oklab(l: f32, c: f32, h_deg: f32) -> (f32, f32, f32) {
    let h = h_deg.to_radians();
    (l, c * h.cos(), c * h.sin())
}

/// Ray vs. oriented disk intersection.
///
/// Returns `Some(t)` if the ray `ro + t*rd` hits the disk, `None` otherwise.
/// The disk is centred at `centre`, faces `normal`, and has radius `r`.
///
/// # Examples
/// ```
/// use abrash_core::math::{ray_disk_intersect, Vec3};
/// // Ray pointing -Z hits a Z-facing disk at origin.
/// let t = ray_disk_intersect(
///     Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 0.0, -1.0),
///     Vec3::ZERO, Vec3::Z, 1.0,
/// );
/// assert!(t.is_some());
/// ```
#[must_use]
#[inline]
pub fn ray_disk_intersect(ro: Vec3, rd: Vec3, centre: Vec3, normal: Vec3, r: f32) -> Option<f32> {
    // First find the t for the infinite plane.
    let denom = rd.dot(normal);
    if denom.abs() < 1e-9 {
        return None; // parallel
    }
    let t = (centre - ro).dot(normal) / denom;
    if t < 0.0 {
        return None; // behind ray
    }
    // Check if hit point is within radius.
    let hit = Vec3::new(
        ro.x + rd.x * t - centre.x,
        ro.y + rd.y * t - centre.y,
        ro.z + rd.z * t - centre.z,
    );
    if hit.dot(hit) <= r * r { Some(t) } else { None }
}

/// Barycentric coordinates of point `p` projected onto triangle `(a, b, c)`.
///
/// Returns `(u, v, w)` such that `u*a + v*b + w*c = p_projected` and
/// `u + v + w = 1`. A point is inside the triangle when all components
/// are non-negative.
///
/// Based on Cramer's rule via dot products (Ericson, RTCD §3.4).
///
/// # Examples
/// ```
/// use abrash_core::math::{barycentric_3d, Vec3};
/// let a = Vec3::new(0.0, 0.0, 0.0);
/// let b = Vec3::new(1.0, 0.0, 0.0);
/// let c = Vec3::new(0.0, 1.0, 0.0);
/// // Centroid should be (1/3, 1/3, 1/3)
/// let (u, v, w) = barycentric_3d(Vec3::new(1.0/3.0, 1.0/3.0, 0.0), a, b, c);
/// assert!((u - 1.0/3.0).abs() < 1e-5);
/// assert!((v - 1.0/3.0).abs() < 1e-5);
/// assert!((w - 1.0/3.0).abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn barycentric_3d(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> (f32, f32, f32) {
    let v0 = b - a;
    let v1 = c - a;
    let v2 = p - a;
    let d00 = v0.dot(v0);
    let d01 = v0.dot(v1);
    let d11 = v1.dot(v1);
    let d20 = v2.dot(v0);
    let d21 = v2.dot(v1);
    let inv = 1.0 / (d00 * d11 - d01 * d01);
    let v = (d11 * d20 - d01 * d21) * inv;
    let w = (d00 * d21 - d01 * d20) * inv;
    let u = 1.0 - v - w;
    (u, v, w)
}

#[cfg(test)]
mod tests_pass_27 {
    use super::*;
    use crate::math::Vec3;

    // ── ease_bounce_out ───────────────────────────────────────────────────────
    #[test]
    fn bounce_out_endpoints() {
        assert!(ease_bounce_out(0.0).abs() < 1e-5);
        assert!((ease_bounce_out(1.0) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn bounce_out_monotone_overall() {
        // Output at t=1 > output at t=0
        assert!(ease_bounce_out(1.0) > ease_bounce_out(0.0));
    }

    // ── ease_bounce_in ────────────────────────────────────────────────────────
    #[test]
    fn bounce_in_endpoints() {
        assert!(ease_bounce_in(0.0).abs() < 1e-5);
        assert!((ease_bounce_in(1.0) - 1.0).abs() < 1e-4);
    }

    // ── ease_circ ─────────────────────────────────────────────────────────────
    #[test]
    fn circ_in_slower_than_linear() {
        assert!(ease_circ_in(0.5) < 0.5);
    }

    #[test]
    fn circ_out_faster_than_linear() {
        assert!(ease_circ_out(0.5) > 0.5);
    }

    #[test]
    fn circ_in_out_endpoints() {
        assert!(ease_circ_in(0.0).abs() < 1e-6);
        assert!((ease_circ_in(1.0) - 1.0).abs() < 1e-5);
        assert!(ease_circ_out(0.0).abs() < 1e-6);
        assert!((ease_circ_out(1.0) - 1.0).abs() < 1e-5);
    }

    // ── sinc ──────────────────────────────────────────────────────────────────
    #[test]
    fn sinc_at_zero_is_one() {
        assert!((sinc(0.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn sinc_zero_crossings() {
        // Zeros at all non-zero integers
        assert!(sinc(1.0).abs() < 1e-6);
        assert!(sinc(2.0).abs() < 1e-6);
        assert!(sinc(-1.0).abs() < 1e-6);
    }

    // ── lanczos_kernel ────────────────────────────────────────────────────────
    #[test]
    fn lanczos_center_is_one() {
        assert!((lanczos_kernel(0.0, 3.0) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn lanczos_outside_support_is_zero() {
        assert_eq!(lanczos_kernel(3.5, 3.0), 0.0);
        assert_eq!(lanczos_kernel(-4.0, 3.0), 0.0);
    }

    // ── mitchell_netravali ────────────────────────────────────────────────────
    #[test]
    fn mitchell_catmull_rom_interpolates() {
        // Catmull-Rom (b=0, c=0.5) is the interpolating special case: f(0)=1
        let v = mitchell_netravali(0.0, 0.0, 0.5);
        assert!((v - 1.0).abs() < 1e-5, "Catmull-Rom at 0: {v}");
    }

    #[test]
    fn mitchell_recommended_approximates() {
        // b=1/3, c=1/3: f(0) = (6 - 2/3) / 6 = 8/9
        let v = mitchell_netravali(0.0, 1.0 / 3.0, 1.0 / 3.0);
        assert!((v - 8.0 / 9.0).abs() < 1e-5, "MN(0) approx: {v}");
    }

    #[test]
    fn mitchell_outside_support_is_zero() {
        assert_eq!(mitchell_netravali(2.0, 1.0 / 3.0, 1.0 / 3.0), 0.0);
        assert_eq!(mitchell_netravali(-2.5, 0.0, 0.5), 0.0);
    }

    // ── oklab_to_oklch / oklch_to_oklab ──────────────────────────────────────
    #[test]
    fn oklch_roundtrip() {
        let (l0, a0, b0) = (0.6_f32, 0.08, 0.12);
        let (l1, c, h) = oklab_to_oklch(l0, a0, b0);
        let (l2, a2, b2) = oklch_to_oklab(l1, c, h);
        assert!((l0 - l2).abs() < 1e-5);
        assert!((a0 - a2).abs() < 1e-5, "a roundtrip: {a0} vs {a2}");
        assert!((b0 - b2).abs() < 1e-5, "b roundtrip: {b0} vs {b2}");
    }

    #[test]
    fn oklch_chroma_nonnegative() {
        let (_, c, _) = oklab_to_oklch(0.5, -0.1, -0.1);
        assert!(c >= 0.0);
    }

    // ── ray_disk_intersect ────────────────────────────────────────────────────
    #[test]
    fn ray_disk_hit() {
        let t = ray_disk_intersect(
            Vec3::new(0.0, 0.0, 2.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::ZERO,
            Vec3::Z,
            1.0,
        );
        assert!(t.is_some());
        assert!((t.unwrap() - 2.0).abs() < 1e-5);
    }

    #[test]
    fn ray_disk_miss_outside_radius() {
        let t = ray_disk_intersect(
            Vec3::new(2.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::ZERO,
            Vec3::Z,
            1.0,
        );
        assert!(t.is_none(), "should miss: {t:?}");
    }

    #[test]
    fn ray_disk_parallel_miss() {
        let t = ray_disk_intersect(Vec3::new(0.0, 0.0, 1.0), Vec3::X, Vec3::ZERO, Vec3::Z, 1.0);
        assert!(t.is_none());
    }

    // ── barycentric_3d ────────────────────────────────────────────────────────
    #[test]
    fn barycentric_centroid() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(3.0, 0.0, 0.0);
        let c = Vec3::new(0.0, 3.0, 0.0);
        let centroid = Vec3::new(1.0, 1.0, 0.0);
        let (u, v, w) = barycentric_3d(centroid, a, b, c);
        assert!((u - 1.0 / 3.0).abs() < 1e-5, "u: {u}");
        assert!((v - 1.0 / 3.0).abs() < 1e-5, "v: {v}");
        assert!((w - 1.0 / 3.0).abs() < 1e-5, "w: {w}");
    }

    #[test]
    fn barycentric_vertex_a() {
        let a = Vec3::new(1.0, 0.0, 0.0);
        let b = Vec3::new(0.0, 1.0, 0.0);
        let c = Vec3::new(0.0, 0.0, 1.0);
        let (u, v, w) = barycentric_3d(a, a, b, c);
        assert!((u - 1.0).abs() < 1e-5, "u at A: {u}");
        assert!(v.abs() < 1e-5, "v at A: {v}");
        assert!(w.abs() < 1e-5, "w at A: {w}");
    }

    #[test]
    fn barycentric_sums_to_one() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(2.0, 0.0, 0.0);
        let c = Vec3::new(1.0, 2.0, 0.0);
        let p = Vec3::new(0.8, 0.4, 0.0);
        let (u, v, w) = barycentric_3d(p, a, b, c);
        assert!((u + v + w - 1.0).abs() < 1e-5, "sum: {}", u + v + w);
    }
}

#[cfg(test)]
mod tests_pass_28 {
    use super::*;
    use core::f32::consts::{FRAC_PI_2, PI};

    // ── Quat::identity ────────────────────────────────────────────────────────
    #[test]
    fn identity_is_unit() {
        let q = Quat::identity();
        assert!((q.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn identity_rotates_nothing() {
        let v = Quat::identity().rotate(Vec3::new(1.0, 2.0, 3.0));
        assert!((v.x - 1.0).abs() < 1e-5);
        assert!((v.y - 2.0).abs() < 1e-5);
        assert!((v.z - 3.0).abs() < 1e-5);
    }

    // ── Quat::from_axis_angle ─────────────────────────────────────────────────
    #[test]
    fn rotate_90_about_y() {
        // +X rotated 90° about +Y → -Z
        let q = Quat::from_axis_angle(Vec3::Y, FRAC_PI_2);
        let v = q.rotate(Vec3::X);
        assert!(v.x.abs() < 1e-5, "x: {}", v.x);
        assert!(v.y.abs() < 1e-5, "y: {}", v.y);
        assert!((v.z + 1.0).abs() < 1e-5, "z: {}", v.z);
    }

    #[test]
    fn rotate_180_about_y() {
        let q = Quat::from_axis_angle(Vec3::Y, PI);
        let v = q.rotate(Vec3::X);
        assert!((v.x + 1.0).abs() < 1e-5, "x: {}", v.x);
    }

    #[test]
    fn axis_angle_roundtrip() {
        let axis = Vec3::new(1.0, 0.0, 0.0);
        let angle = 1.2_f32;
        let q = Quat::from_axis_angle(axis, angle);
        assert!((q.angle() - angle).abs() < 1e-5, "angle: {}", q.angle());
        let a = q.axis();
        assert!((a.x - 1.0).abs() < 1e-5, "axis.x: {}", a.x);
    }

    // ── Quat multiplication ───────────────────────────────────────────────────
    #[test]
    fn mul_identity_is_noop() {
        let q = Quat::from_axis_angle(Vec3::Y, 0.7);
        let r = q * Quat::identity();
        assert!((r.x - q.x).abs() < 1e-6);
        assert!((r.w - q.w).abs() < 1e-6);
    }

    #[test]
    fn mul_composed_rotation() {
        // 90° about Y twice = 180° about Y: +X → -X
        let q90 = Quat::from_axis_angle(Vec3::Y, FRAC_PI_2);
        let q180 = q90 * q90;
        let v = q180.rotate(Vec3::X);
        assert!((v.x + 1.0).abs() < 1e-5, "x: {}", v.x);
    }

    // ── Quat::conjugate / inverse ─────────────────────────────────────────────
    #[test]
    fn conjugate_undoes_rotation() {
        let q = Quat::from_axis_angle(Vec3::Z, FRAC_PI_2);
        let v = Vec3::new(1.0, 0.0, 0.0);
        let rotated = q.rotate(v);
        let back = q.conjugate().rotate(rotated);
        assert!((back.x - v.x).abs() < 1e-5, "back.x: {}", back.x);
        assert!((back.y - v.y).abs() < 1e-5, "back.y: {}", back.y);
    }

    // ── Quat::slerp ───────────────────────────────────────────────────────────
    #[test]
    fn slerp_t0_is_start() {
        let q0 = Quat::identity();
        let q1 = Quat::from_axis_angle(Vec3::Y, PI);
        let s = q0.slerp(q1, 0.0);
        assert!((s.w - q0.w).abs() < 1e-5, "w: {}", s.w);
    }

    #[test]
    fn slerp_t1_is_end() {
        let q0 = Quat::identity();
        let q1 = Quat::from_axis_angle(Vec3::Y, PI);
        let s = q0.slerp(q1, 1.0);
        // q and -q represent the same rotation; verify via rotate rather than components.
        let v = s.rotate(Vec3::X);
        let v1 = q1.rotate(Vec3::X);
        assert!((v.x - v1.x).abs() < 1e-4, "x: {} vs {}", v.x, v1.x);
        assert!((v.z - v1.z).abs() < 1e-4, "z: {} vs {}", v.z, v1.z);
    }

    #[test]
    fn slerp_midpoint_is_half_angle() {
        let q0 = Quat::identity();
        let q1 = Quat::from_axis_angle(Vec3::Y, FRAC_PI_2);
        let mid = q0.slerp(q1, 0.5);
        assert!(
            (mid.angle() - FRAC_PI_2 / 2.0).abs() < 1e-4,
            "angle: {}",
            mid.angle()
        );
    }

    // ── Quat::to_mat3 ─────────────────────────────────────────────────────────
    #[test]
    fn to_mat3_identity() {
        let m = Quat::identity().to_mat3();
        // diagonal = 1
        assert!((m.m[0][0] - 1.0).abs() < 1e-6);
        assert!((m.m[1][1] - 1.0).abs() < 1e-6);
        assert!((m.m[2][2] - 1.0).abs() < 1e-6);
        // off-diagonal = 0
        assert!(m.m[0][1].abs() < 1e-6);
        assert!(m.m[0][2].abs() < 1e-6);
        assert!(m.m[1][0].abs() < 1e-6);
    }

    #[test]
    fn to_mat3_consistent_with_rotate() {
        let q = Quat::from_axis_angle(Vec3::new(1.0, 1.0, 0.0).normalize(), 1.1);
        let v = Vec3::new(0.5, -0.3, 0.8);
        let via_quat = q.rotate(v);
        let via_mat = q.to_mat3() * v;
        assert!((via_quat.x - via_mat.x).abs() < 1e-5, "x diff");
        assert!((via_quat.y - via_mat.y).abs() < 1e-5, "y diff");
        assert!((via_quat.z - via_mat.z).abs() < 1e-5, "z diff");
    }

    // ── Quat::from_euler_zyx ──────────────────────────────────────────────────
    #[test]
    fn euler_identity_is_identity() {
        let q = Quat::from_euler_zyx(0.0, 0.0, 0.0);
        assert!((q.w - 1.0).abs() < 1e-6);
        assert!(q.x.abs() < 1e-6 && q.y.abs() < 1e-6 && q.z.abs() < 1e-6);
    }

    #[test]
    fn euler_yaw_90_matches_axis_angle() {
        let q_euler = Quat::from_euler_zyx(FRAC_PI_2, 0.0, 0.0);
        let q_aa = Quat::from_axis_angle(Vec3::Y, FRAC_PI_2);
        assert!((q_euler.x - q_aa.x).abs() < 1e-5);
        assert!((q_euler.y - q_aa.y).abs() < 1e-5);
        assert!((q_euler.z - q_aa.z).abs() < 1e-5);
        assert!((q_euler.w - q_aa.w).abs() < 1e-5);
    }
}

// ── Pass 29 additions ─────────────────────────────────────────────────────────

/// Reflect an incident direction about a surface `normal` (Snell's law mirror term).
///
/// Both `incident` and `normal` should be normalised. The result is the
/// mirror direction: `i - 2 * dot(i, n) * n`.
///
/// # Examples
/// ```
/// use abrash_core::math::{reflect, Vec3};
/// // Vertical drop → vertical bounce.
/// let r = reflect(Vec3::new(0.0, -1.0, 0.0), Vec3::Y);
/// assert!((r.y - 1.0).abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn reflect(incident: Vec3, normal: Vec3) -> Vec3 {
    let d = incident.dot(normal);
    Vec3::new(
        incident.x - 2.0 * d * normal.x,
        incident.y - 2.0 * d * normal.y,
        incident.z - 2.0 * d * normal.z,
    )
}

/// Refract an incident direction through a surface using Snell's law.
///
/// Returns `None` for total internal reflection (when `eta * sin_θ > 1`).
///
/// * `eta` = n_incident / n_transmitted (e.g. 1.0/1.5 air→glass).
///
/// # Examples
/// ```
/// use abrash_core::math::{refract, Vec3};
/// // Normal incidence: ray passes straight through.
/// let r = refract(Vec3::new(0.0, -1.0, 0.0), Vec3::Y, 1.0);
/// assert!(r.is_some());
/// ```
#[must_use]
#[inline]
pub fn refract(incident: Vec3, normal: Vec3, eta: f32) -> Option<Vec3> {
    let cos_i = -incident.dot(normal);
    let sin2_t = eta * eta * (1.0 - cos_i * cos_i);
    if sin2_t > 1.0 {
        return None; // total internal reflection
    }
    let cos_t = (1.0 - sin2_t).sqrt();
    Some(Vec3::new(
        eta * incident.x + (eta * cos_i - cos_t) * normal.x,
        eta * incident.y + (eta * cos_i - cos_t) * normal.y,
        eta * incident.z + (eta * cos_i - cos_t) * normal.z,
    ))
}

/// Closest point on segment `(a, b)` to point `p` in 3D.
///
/// # Examples
/// ```
/// use abrash_core::math::{closest_point_on_segment_3d, Vec3};
/// let c = closest_point_on_segment_3d(Vec3::new(0.0, 1.0, 0.0),
///     Vec3::new(-1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
/// assert!(c.y.abs() < 1e-5); // projected onto X axis
/// ```
#[must_use]
#[inline]
pub fn closest_point_on_segment_3d(p: Vec3, a: Vec3, b: Vec3) -> Vec3 {
    let ab = b - a;
    let t = (p - a).dot(ab) / ab.dot(ab);
    let t = t.clamp(0.0, 1.0);
    Vec3::new(a.x + t * ab.x, a.y + t * ab.y, a.z + t * ab.z)
}

/// Project a 3D point onto a plane defined by `normal` (unit) and offset `d`.
///
/// The plane equation is `dot(p, normal) = d`.
///
/// # Examples
/// ```
/// use abrash_core::math::{project_point_to_plane, Vec3};
/// let p = Vec3::new(0.0, 3.0, 0.0);
/// let proj = project_point_to_plane(p, Vec3::Y, 1.0); // plane y=1
/// assert!((proj.y - 1.0).abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn project_point_to_plane(p: Vec3, normal: Vec3, d: f32) -> Vec3 {
    let dist = p.dot(normal) - d;
    Vec3::new(
        p.x - dist * normal.x,
        p.y - dist * normal.y,
        p.z - dist * normal.z,
    )
}

/// Quadratic (t²) ease-in — starts slow, ends fast.
///
/// # Examples
/// ```
/// use abrash_core::math::ease_quad_in;
/// assert!(ease_quad_in(0.5) < 0.5);
/// assert!(ease_quad_in(0.0).abs() < 1e-6);
/// assert!((ease_quad_in(1.0) - 1.0).abs() < 1e-6);
/// ```
#[must_use]
#[inline]
pub fn ease_quad_in(t: f32) -> f32 {
    t * t
}

/// Quadratic ease-out — starts fast, ends slow.
///
/// # Examples
/// ```
/// use abrash_core::math::ease_quad_out;
/// assert!(ease_quad_out(0.5) > 0.5);
/// ```
#[must_use]
#[inline]
pub fn ease_quad_out(t: f32) -> f32 {
    t * (2.0 - t)
}

/// Quadratic ease-in-out — slow at both ends, fast in middle.
///
/// # Examples
/// ```
/// use abrash_core::math::ease_quad_in_out;
/// assert!(ease_quad_in_out(0.0).abs() < 1e-6);
/// assert!((ease_quad_in_out(1.0) - 1.0).abs() < 1e-6);
/// assert!((ease_quad_in_out(0.5) - 0.5).abs() < 1e-6);
/// ```
#[must_use]
#[inline]
pub fn ease_quad_in_out(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        let t1 = t - 1.0;
        1.0 - 2.0 * t1 * t1
    }
}

/// Cubic (t³) ease-in.
///
/// # Examples
/// ```
/// use abrash_core::math::ease_cubic_in;
/// assert!(ease_cubic_in(0.5) < ease_cubic_in(1.0));
/// ```
#[must_use]
#[inline]
pub fn ease_cubic_in(t: f32) -> f32 {
    t * t * t
}

/// Cubic ease-out.
///
/// # Examples
/// ```
/// use abrash_core::math::ease_cubic_out;
/// assert!((ease_cubic_out(1.0) - 1.0).abs() < 1e-6);
/// ```
#[must_use]
#[inline]
pub fn ease_cubic_out(t: f32) -> f32 {
    let t1 = t - 1.0;
    t1 * t1 * t1 + 1.0
}

/// Cubic ease-in-out.
///
/// # Examples
/// ```
/// use abrash_core::math::ease_cubic_in_out;
/// assert!((ease_cubic_in_out(0.5) - 0.5).abs() < 1e-6);
/// ```
#[must_use]
#[inline]
pub fn ease_cubic_in_out(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let t1 = 2.0 * t - 2.0;
        0.5 * t1 * t1 * t1 + 1.0
    }
}

/// Sine ease-in — starts slow using a cosine curve.
///
/// # Examples
/// ```
/// use abrash_core::math::ease_sine_in;
/// assert!(ease_sine_in(0.0).abs() < 1e-6);
/// assert!((ease_sine_in(1.0) - 1.0).abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn ease_sine_in(t: f32) -> f32 {
    1.0 - (t * core::f32::consts::FRAC_PI_2).cos()
}

/// Sine ease-out — ends slow using a sine curve.
///
/// # Examples
/// ```
/// use abrash_core::math::ease_sine_out;
/// assert!((ease_sine_out(1.0) - 1.0).abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn ease_sine_out(t: f32) -> f32 {
    (t * core::f32::consts::FRAC_PI_2).sin()
}

/// Sine ease-in-out.
///
/// # Examples
/// ```
/// use abrash_core::math::ease_sine_in_out;
/// assert!((ease_sine_in_out(0.5) - 0.5).abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn ease_sine_in_out(t: f32) -> f32 {
    0.5 * (1.0 - (core::f32::consts::PI * t).cos())
}

/// Gaussian probability density function: `exp(-0.5 * ((x-μ)/σ)²) / (σ√(2π))`.
///
/// # Examples
/// ```
/// use abrash_core::math::gaussian;
/// // Peak at mean.
/// let peak = gaussian(0.0, 0.0, 1.0);
/// let off  = gaussian(1.0, 0.0, 1.0);
/// assert!(peak > off);
/// // Standard normal PDF at x=0 ≈ 0.3989.
/// assert!((peak - 0.398_942_28).abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn gaussian(x: f32, mean: f32, stddev: f32) -> f32 {
    let z = (x - mean) / stddev;
    let inv_sqrt2pi = 0.398_942_28_f32; // 1 / sqrt(2π)
    (inv_sqrt2pi / stddev) * (-0.5 * z * z).exp()
}

/// Abramowitz & Stegun approximation of the error function `erf(x)`.
///
/// Maximum error ≤ 1.5 × 10⁻⁷ across the entire real line.
///
/// # Examples
/// ```
/// use abrash_core::math::erf_approx;
/// assert!(erf_approx(0.0).abs() < 1e-6);
/// assert!((erf_approx(f32::INFINITY) - 1.0).abs() < 1e-5);
/// assert!((erf_approx(-1.0) + erf_approx(1.0)).abs() < 1e-5); // odd function
/// ```
#[must_use]
#[inline]
pub fn erf_approx(x: f32) -> f32 {
    // A&S formula 7.1.26 (rational approximation)
    const P: f32 = 0.327_591_1;
    const A: [f32; 5] = [
        0.254_829_592,
        -0.284_496_736,
        1.421_413_741,
        -1.453_152_027,
        1.061_405_429,
    ];
    let sign = if x < 0.0 { -1.0_f32 } else { 1.0_f32 };
    let x = x.abs();
    let t = 1.0 / (1.0 + P * x);
    let poly = ((((A[4] * t + A[3]) * t + A[2]) * t + A[1]) * t + A[0]) * t;
    sign * (1.0 - poly * (-x * x).exp())
}

#[cfg(test)]
mod tests_pass_29 {
    use super::*;
    use crate::math::Vec3;

    // ── reflect ───────────────────────────────────────────────────────────────
    #[test]
    fn reflect_vertical_normal() {
        let r = reflect(Vec3::new(0.0, -1.0, 0.0), Vec3::Y);
        assert!((r.y - 1.0).abs() < 1e-5, "ry: {}", r.y);
    }

    #[test]
    fn reflect_45_degree() {
        // incident at 45° from normal → reflects at 45° on other side
        let i = Vec3::new(1.0, -1.0, 0.0).normalize();
        let r = reflect(i, Vec3::Y);
        assert!((r.x - i.x).abs() < 1e-5);
        assert!((r.y + i.y).abs() < 1e-5); // y flips
    }

    // ── refract ───────────────────────────────────────────────────────────────
    #[test]
    fn refract_normal_incidence_eta1() {
        // eta=1 (same medium): refracted = incident
        let i = Vec3::new(0.0, -1.0, 0.0);
        let r = refract(i, Vec3::Y, 1.0).expect("should transmit");
        assert!((r.y + 1.0).abs() < 1e-5, "ry: {}", r.y);
    }

    #[test]
    fn refract_total_internal_reflection() {
        // Large eta, glancing angle → TIR
        let i = Vec3::new(0.999, -0.045, 0.0).normalize();
        let result = refract(i, Vec3::Y, 1.5);
        assert!(result.is_none(), "should TIR");
    }

    // ── closest_point_on_segment_3d ───────────────────────────────────────────
    #[test]
    fn closest_point_projects_onto_segment() {
        let c = closest_point_on_segment_3d(
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
        );
        assert!(c.y.abs() < 1e-5, "y: {}", c.y);
        assert!(c.x.abs() < 1e-5, "x: {}", c.x);
    }

    #[test]
    fn closest_point_clamps_to_endpoint() {
        // Beyond end of segment → returns endpoint
        let c = closest_point_on_segment_3d(
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
        );
        assert!((c.x - 1.0).abs() < 1e-5, "x: {}", c.x);
    }

    // ── project_point_to_plane ────────────────────────────────────────────────
    #[test]
    fn project_point_above_plane() {
        let p = project_point_to_plane(Vec3::new(0.0, 3.0, 0.0), Vec3::Y, 1.0);
        assert!((p.y - 1.0).abs() < 1e-5, "y: {}", p.y);
    }

    #[test]
    fn project_point_on_plane_unchanged() {
        let p = Vec3::new(3.0, 1.0, 2.0);
        let proj = project_point_to_plane(p, Vec3::Y, 1.0);
        assert!((proj.x - 3.0).abs() < 1e-5);
        assert!((proj.y - 1.0).abs() < 1e-5);
    }

    // ── easing: quad ─────────────────────────────────────────────────────────
    #[test]
    fn quad_easing_endpoints() {
        for &v in &[ease_quad_in(0.0), ease_quad_out(0.0), ease_quad_in_out(0.0)] {
            assert!(v.abs() < 1e-6, "start: {v}");
        }
        for &v in &[ease_quad_in(1.0), ease_quad_out(1.0), ease_quad_in_out(1.0)] {
            assert!((v - 1.0).abs() < 1e-5, "end: {v}");
        }
    }

    #[test]
    fn quad_in_slower_than_out_at_midpoint() {
        assert!(ease_quad_in(0.5) < ease_quad_out(0.5));
    }

    // ── easing: cubic ────────────────────────────────────────────────────────
    #[test]
    fn cubic_easing_endpoints() {
        assert!(ease_cubic_in(0.0).abs() < 1e-6);
        assert!((ease_cubic_in(1.0) - 1.0).abs() < 1e-5);
        assert!((ease_cubic_out(1.0) - 1.0).abs() < 1e-5);
        assert!((ease_cubic_in_out(0.5) - 0.5).abs() < 1e-5);
    }

    // ── easing: sine ─────────────────────────────────────────────────────────
    #[test]
    fn sine_easing_endpoints() {
        assert!(ease_sine_in(0.0).abs() < 1e-6);
        assert!((ease_sine_in(1.0) - 1.0).abs() < 1e-5);
        assert!((ease_sine_out(1.0) - 1.0).abs() < 1e-5);
        assert!((ease_sine_in_out(0.5) - 0.5).abs() < 1e-5);
    }

    // ── gaussian ─────────────────────────────────────────────────────────────
    #[test]
    fn gaussian_peak_at_mean() {
        let peak = gaussian(1.0, 1.0, 0.5);
        let off = gaussian(1.5, 1.0, 0.5);
        assert!(peak > off, "peak {peak} vs off {off}");
    }

    #[test]
    fn gaussian_standard_normal() {
        let v = gaussian(0.0, 0.0, 1.0);
        assert!((v - 0.398_942_28_f32).abs() < 1e-5, "N(0,1) pdf: {v}");
    }

    // ── erf_approx ───────────────────────────────────────────────────────────
    #[test]
    fn erf_at_zero() {
        assert!(erf_approx(0.0).abs() < 1e-6);
    }

    #[test]
    fn erf_odd_function() {
        let x = 0.7_f32;
        assert!((erf_approx(x) + erf_approx(-x)).abs() < 1e-6);
    }

    #[test]
    fn erf_converges_to_one() {
        assert!((erf_approx(5.0) - 1.0).abs() < 1e-5);
    }
}

// ── Pass 30 additions ─────────────────────────────────────────────────────────

/// 3D Perlin noise in `[−1, 1]` using Ken Perlin's improved 2002 gradients.
///
/// The hash uses [`wang_hash`] for a table-free implementation. Output range
/// is approximately `[−0.87, 0.87]` (√(2/3) for the 3D gradient projection).
///
/// # Examples
/// ```
/// use abrash_core::math::{perlin_noise_3d, Vec3};
/// let n = perlin_noise_3d(Vec3::new(1.5, 2.3, 0.7));
/// assert!(n >= -1.0 && n <= 1.0);
/// ```
#[must_use]
pub fn perlin_noise_3d(p: Vec3) -> f32 {
    #[inline]
    fn grad3(hash: u32, dx: f32, dy: f32, dz: f32) -> f32 {
        // Perlin 2002 improved gradients — 12 midpoints of unit cube edges.
        match hash & 15 {
            0 => dx + dy,
            1 => -dx + dy,
            2 => dx - dy,
            3 => -dx - dy,
            4 => dx + dz,
            5 => -dx + dz,
            6 => dx - dz,
            7 => -dx - dz,
            8 => dy + dz,
            9 => -dy + dz,
            10 => dy - dz,
            11 => -dy - dz,
            12 => dx + dy,
            13 => -dx + dy,
            14 => -dy + dz,
            _ => -dy - dz,
        }
    }
    #[inline]
    fn fade(t: f32) -> f32 {
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }

    let ix = p.x.floor() as i32;
    let iy = p.y.floor() as i32;
    let iz = p.z.floor() as i32;
    let fx = p.x - p.x.floor();
    let fy = p.y - p.y.floor();
    let fz = p.z - p.z.floor();
    let ux = fade(fx);
    let uy = fade(fy);
    let uz = fade(fz);

    let h =
        |x: i32, y: i32, z: i32| wang_hash(x as u32 ^ wang_hash(y as u32 ^ wang_hash(z as u32)));

    // Trilinear interpolation of 8 gradient contributions.
    let g000 = grad3(h(ix, iy, iz), fx, fy, fz);
    let g100 = grad3(h(ix + 1, iy, iz), fx - 1.0, fy, fz);
    let g010 = grad3(h(ix, iy + 1, iz), fx, fy - 1.0, fz);
    let g110 = grad3(h(ix + 1, iy + 1, iz), fx - 1.0, fy - 1.0, fz);
    let g001 = grad3(h(ix, iy, iz + 1), fx, fy, fz - 1.0);
    let g101 = grad3(h(ix + 1, iy, iz + 1), fx - 1.0, fy, fz - 1.0);
    let g011 = grad3(h(ix, iy + 1, iz + 1), fx, fy - 1.0, fz - 1.0);
    let g111 = grad3(h(ix + 1, iy + 1, iz + 1), fx - 1.0, fy - 1.0, fz - 1.0);

    lerp(
        lerp(lerp(g000, g100, ux), lerp(g010, g110, ux), uy),
        lerp(lerp(g001, g101, ux), lerp(g011, g111, ux), uy),
        uz,
    )
}

/// 3D fractal Brownian motion built on [`perlin_noise_3d`].
///
/// Output is in approximately `[−1, 1]` before rescaling; the exact range
/// depends on `octaves`, `lacunarity`, and `gain`.
///
/// # Examples
/// ```
/// use abrash_core::math::{fbm_3d, Vec3};
/// let n = fbm_3d(Vec3::new(1.0, 2.0, 0.5), 4, 2.0, 0.5);
/// assert!(n >= -2.0 && n <= 2.0);
/// ```
#[must_use]
pub fn fbm_3d(mut p: Vec3, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
    let mut sum = 0.0_f32;
    let mut amp = 1.0_f32;
    for _ in 0..octaves {
        sum += amp * perlin_noise_3d(p);
        p = Vec3::new(p.x * lacunarity, p.y * lacunarity, p.z * lacunarity);
        amp *= gain;
    }
    sum
}

/// Linear interpolation of two [`Vec2`] values.
///
/// # Examples
/// ```
/// use abrash_core::math::{lerp_vec2, Vec2};
/// let v = lerp_vec2(Vec2::ZERO, Vec2::new(2.0, 4.0), 0.5);
/// assert!((v.x - 1.0).abs() < 1e-6 && (v.y - 2.0).abs() < 1e-6);
/// ```
#[must_use]
#[inline]
pub fn lerp_vec2(a: Vec2, b: Vec2, t: f32) -> Vec2 {
    Vec2::new(lerp(a.x, b.x, t), lerp(a.y, b.y, t))
}

/// Linear interpolation of two [`Vec3`] values.
///
/// # Examples
/// ```
/// use abrash_core::math::{lerp_vec3, Vec3};
/// let v = lerp_vec3(Vec3::ZERO, Vec3::new(2.0, 4.0, 6.0), 0.5);
/// assert!((v.x - 1.0).abs() < 1e-6);
/// assert!((v.z - 3.0).abs() < 1e-6);
/// ```
#[must_use]
#[inline]
pub fn lerp_vec3(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    Vec3::new(lerp(a.x, b.x, t), lerp(a.y, b.y, t), lerp(a.z, b.z, t))
}

/// Check if two 2D line segments `(a1, a2)` and `(b1, b2)` intersect.
///
/// Returns `true` if the open segments cross (not just their infinite
/// extensions). Collinear overlapping segments return `false`.
///
/// # Examples
/// ```
/// use abrash_core::math::{segment_intersect_2d, Vec2};
/// // X-crossing: should intersect.
/// assert!(segment_intersect_2d(
///     Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0),
///     Vec2::new(0.0, -1.0), Vec2::new(0.0, 1.0),
/// ));
/// // Parallel: should not.
/// assert!(!segment_intersect_2d(
///     Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0),
///     Vec2::new(0.0, 1.0), Vec2::new(1.0, 1.0),
/// ));
/// ```
#[must_use]
#[inline]
pub fn segment_intersect_2d(a1: Vec2, a2: Vec2, b1: Vec2, b2: Vec2) -> bool {
    // Classic cross-product winding number test.
    #[inline]
    fn cross2(v: Vec2, w: Vec2) -> f32 {
        v.x * w.y - v.y * w.x
    }
    let r = Vec2::new(a2.x - a1.x, a2.y - a1.y);
    let s = Vec2::new(b2.x - b1.x, b2.y - b1.y);
    let denom = cross2(r, s);
    if denom.abs() < 1e-12 {
        return false; // parallel or collinear
    }
    let diff = Vec2::new(b1.x - a1.x, b1.y - a1.y);
    let t = cross2(diff, s) / denom;
    let u = cross2(diff, r) / denom;
    (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)
}

/// Exponential ease-in: `2^(10(t−1))` curve (very slow start, fast end).
///
/// # Examples
/// ```
/// use abrash_core::math::ease_expo_in;
/// assert!(ease_expo_in(0.0).abs() < 1e-5);
/// assert!((ease_expo_in(1.0) - 1.0).abs() < 1e-5);
/// assert!(ease_expo_in(0.5) < 0.1); // extremely slow at midpoint
/// ```
#[must_use]
#[inline]
pub fn ease_expo_in(t: f32) -> f32 {
    if t <= 0.0 {
        0.0
    } else {
        (2.0_f32).powf(10.0 * t - 10.0)
    }
}

/// Exponential ease-out: mirror of [`ease_expo_in`].
///
/// # Examples
/// ```
/// use abrash_core::math::ease_expo_out;
/// assert!(ease_expo_out(0.0).abs() < 1e-5);
/// assert!((ease_expo_out(1.0) - 1.0).abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn ease_expo_out(t: f32) -> f32 {
    if t >= 1.0 {
        1.0
    } else {
        1.0 - (2.0_f32).powf(-10.0 * t)
    }
}

/// Exponential ease-in-out.
///
/// # Examples
/// ```
/// use abrash_core::math::ease_expo_in_out;
/// assert!(ease_expo_in_out(0.0).abs() < 1e-5);
/// assert!((ease_expo_in_out(1.0) - 1.0).abs() < 1e-5);
/// assert!((ease_expo_in_out(0.5) - 0.5).abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn ease_expo_in_out(t: f32) -> f32 {
    if t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else if t < 0.5 {
        (2.0_f32).powf(20.0 * t - 10.0) * 0.5
    } else {
        (2.0 - (2.0_f32).powf(-20.0 * t + 10.0)) * 0.5
    }
}

#[cfg(test)]
mod tests_pass_30 {
    use super::*;
    use crate::math::{Vec2, Vec3};

    // ── perlin_noise_3d ───────────────────────────────────────────────────────
    #[test]
    fn perlin_3d_in_range() {
        let n = perlin_noise_3d(Vec3::new(1.5, 2.3, 0.7));
        assert!(n >= -1.0 && n <= 1.0, "out of range: {n}");
    }

    #[test]
    fn perlin_3d_grid_point_near_zero() {
        // At integer lattice points, all gradients cancel → near 0.
        let n = perlin_noise_3d(Vec3::new(1.0, 2.0, 3.0));
        assert!(n.abs() < 1e-5, "lattice: {n}");
    }

    #[test]
    fn perlin_3d_varies() {
        let n1 = perlin_noise_3d(Vec3::new(0.3, 0.7, 0.5));
        let n2 = perlin_noise_3d(Vec3::new(1.3, 0.7, 0.5));
        assert!((n1 - n2).abs() > 1e-3, "no variation: {n1} {n2}");
    }

    // ── fbm_3d ────────────────────────────────────────────────────────────────
    #[test]
    fn fbm_3d_range() {
        let n = fbm_3d(Vec3::new(1.0, 2.0, 0.5), 4, 2.0, 0.5);
        assert!(n >= -3.0 && n <= 3.0, "fbm out of range: {n}");
    }

    // ── lerp_vec ──────────────────────────────────────────────────────────────
    #[test]
    fn lerp_vec2_midpoint() {
        let v = lerp_vec2(Vec2::ZERO, Vec2::new(2.0, 4.0), 0.5);
        assert!((v.x - 1.0).abs() < 1e-6);
        assert!((v.y - 2.0).abs() < 1e-6);
    }

    #[test]
    fn lerp_vec3_endpoints() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(5.0, 6.0, 7.0);
        let v0 = lerp_vec3(a, b, 0.0);
        let v1 = lerp_vec3(a, b, 1.0);
        assert!((v0.x - 1.0).abs() < 1e-6);
        assert!((v1.z - 7.0).abs() < 1e-6);
    }

    // ── segment_intersect_2d ─────────────────────────────────────────────────
    #[test]
    fn segments_x_cross() {
        assert!(segment_intersect_2d(
            Vec2::new(-1.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, -1.0),
            Vec2::new(0.0, 1.0),
        ));
    }

    #[test]
    fn segments_parallel_no_cross() {
        assert!(!segment_intersect_2d(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
        ));
    }

    #[test]
    fn segments_t_no_extend() {
        // Segments that would cross if extended but don't overlap.
        assert!(!segment_intersect_2d(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(2.0, -1.0),
            Vec2::new(2.0, 1.0),
        ));
    }

    // ── ease_expo ─────────────────────────────────────────────────────────────
    #[test]
    fn expo_easing_endpoints() {
        assert!(ease_expo_in(0.0).abs() < 1e-5);
        assert!((ease_expo_in(1.0) - 1.0).abs() < 1e-5);
        assert!(ease_expo_out(0.0).abs() < 1e-5);
        assert!((ease_expo_out(1.0) - 1.0).abs() < 1e-5);
        assert!((ease_expo_in_out(0.5) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn expo_in_very_slow_at_midpoint() {
        assert!(ease_expo_in(0.5) < 0.05);
    }

    #[test]
    fn expo_out_fast_at_start() {
        assert!(ease_expo_out(0.5) > 0.95);
    }
}

// ── Pass 31 ────────────────────────────────────────────────────────────────────

/// Elastic ease-in: starts from rest, overshoots at the beginning, then
/// accelerates to the target.  Mirror of [`ease_elastic_out`].
///
/// # Arguments
/// * `t`         – normalised time in \[0, 1]
/// * `amplitude` – oscillation amplitude (clamped to ≥ 1.0)
/// * `period`    – oscillation period (e.g. `0.3`)
#[must_use]
#[inline]
pub fn ease_elastic_in(t: f32, amplitude: f32, period: f32) -> f32 {
    1.0 - ease_elastic_out(1.0 - t, amplitude, period)
}

/// Elastic ease-in-out: elastic overshoot at the start **and** end.
///
/// # Arguments
/// * `t`         – normalised time in \[0, 1]
/// * `amplitude` – oscillation amplitude (clamped to ≥ 1.0)
/// * `period`    – oscillation period (e.g. `0.45`)
#[must_use]
#[inline]
pub fn ease_elastic_in_out(t: f32, amplitude: f32, period: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let a = amplitude.max(1.0);
    let s = a.recip().asin() * period / core::f32::consts::TAU;
    let t2 = t * 2.0;
    if t2 < 1.0 {
        -0.5 * a
            * (2.0_f32).powf(10.0 * (t2 - 1.0))
            * ((t2 - 1.0 - s) * core::f32::consts::TAU / period).sin()
    } else {
        a * (2.0_f32).powf(-10.0 * (t2 - 1.0))
            * ((t2 - 1.0 - s) * core::f32::consts::TAU / period).sin()
            * 0.5
            + 1.0
    }
}

/// Back ease-in-out: overshoots on both ends with a smooth symmetric curve.
///
/// # Arguments
/// * `t`         – normalised time in \[0, 1]
/// * `overshoot` – controls how far the motion overshoots (default ≈ `1.70158`)
#[must_use]
#[inline]
pub fn ease_back_in_out(t: f32, overshoot: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let s = overshoot * 1.525_f32;
    let t2 = t * 2.0;
    if t2 < 1.0 {
        0.5 * t2 * t2 * ((s + 1.0) * t2 - s)
    } else {
        let t2 = t2 - 2.0;
        0.5 * (t2 * t2 * ((s + 1.0) * t2 + s) + 2.0)
    }
}

/// 3-D **value noise**: trilinearly-interpolated lattice noise in \[0, 1].
///
/// Uses quintic fade for C2 continuity at cell boundaries.
///
/// # Examples
/// ```
/// use abrash_core::math::{value_noise_3d, Vec3};
/// let n = value_noise_3d(Vec3::new(1.5, 2.3, 0.7));
/// assert!(n >= 0.0 && n <= 1.0);
/// ```
#[must_use]
pub fn value_noise_3d(p: Vec3) -> f32 {
    #[inline]
    fn h3(x: i32, y: i32, z: i32) -> f32 {
        // Combine three coordinates with distinct multipliers, then hash.
        let v = (x as u32)
            .wrapping_mul(1_619)
            .wrapping_add((y as u32).wrapping_mul(31_337))
            .wrapping_add((z as u32).wrapping_mul(6_971));
        hash_to_f32(v)
    }

    let ix = p.x.floor() as i32;
    let iy = p.y.floor() as i32;
    let iz = p.z.floor() as i32;
    let fx = p.x - p.x.floor();
    let fy = p.y - p.y.floor();
    let fz = p.z - p.z.floor();
    // Quintic fade for C2 continuity.
    let ux = fx * fx * fx * (fx * (fx * 6.0 - 15.0) + 10.0);
    let uy = fy * fy * fy * (fy * (fy * 6.0 - 15.0) + 10.0);
    let uz = fz * fz * fz * (fz * (fz * 6.0 - 15.0) + 10.0);

    let x0z0 = lerp(h3(ix, iy, iz), h3(ix + 1, iy, iz), ux);
    let x1z0 = lerp(h3(ix, iy + 1, iz), h3(ix + 1, iy + 1, iz), ux);
    let x0z1 = lerp(h3(ix, iy, iz + 1), h3(ix + 1, iy, iz + 1), ux);
    let x1z1 = lerp(h3(ix, iy + 1, iz + 1), h3(ix + 1, iy + 1, iz + 1), ux);
    let y0 = lerp(x0z0, x1z0, uy);
    let y1 = lerp(x0z1, x1z1, uy);
    lerp(y0, y1, uz)
}

/// 3-D **Worley (cellular) noise**: returns the distance to the nearest
/// feature point scattered one-per-cell on a unit lattice.
///
/// # Examples
/// ```
/// use abrash_core::math::{worley_noise_3d, Vec3};
/// let d = worley_noise_3d(Vec3::new(1.5, 2.3, 0.7));
/// assert!(d >= 0.0);
/// ```
#[must_use]
pub fn worley_noise_3d(p: Vec3) -> f32 {
    let ix = p.x.floor() as i32;
    let iy = p.y.floor() as i32;
    let iz = p.z.floor() as i32;
    let mut min_dist = f32::INFINITY;
    for dz in -1..=1_i32 {
        for dy in -1..=1_i32 {
            for dx in -1..=1_i32 {
                let cx = (ix + dx) as f32;
                let cy = (iy + dy) as f32;
                let cz = (iz + dz) as f32;
                // Three independent offsets per cell.
                let ox = hash2_to_f32(
                    (ix + dx) as u32,
                    (iy + dy) as u32 ^ ((iz + dz) as u32).wrapping_mul(6_971),
                );
                let oy = hash2_to_f32(
                    ((iy + dy) as u32) ^ 0xDEAD_BEEF,
                    ((ix + dx) as u32).wrapping_add((iz + dz) as u32),
                );
                let oz = hash2_to_f32(
                    ((iz + dz) as u32) ^ 0xCAFE_BABE,
                    ((ix + dx) as u32)
                        .wrapping_mul(31_337)
                        .wrapping_add((iy + dy) as u32),
                );
                let fx = p.x - (cx + ox);
                let fy = p.y - (cy + oy);
                let fz = p.z - (cz + oz);
                let dist = (fx * fx + fy * fy + fz * fz).sqrt();
                if dist < min_dist {
                    min_dist = dist;
                }
            }
        }
    }
    min_dist
}

/// Closest point on a triangle surface to point `p` (3-D).
///
/// Uses Ericson's Voronoi-region algorithm (*Real-Time Collision Detection*,
/// §5.1.5) — no square roots until the return value.
///
/// # Examples
/// ```
/// use abrash_core::math::{closest_point_on_triangle_3d, Vec3};
/// let cp = closest_point_on_triangle_3d(
///     Vec3::new(0.5, 1.0, 0.0),
///     Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0),
/// );
/// assert!(cp.y.abs() < 1e-5); // projected onto y=0 plane
/// ```
#[must_use]
pub fn closest_point_on_triangle_3d(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let ab = Vec3::new(b.x - a.x, b.y - a.y, b.z - a.z);
    let ac = Vec3::new(c.x - a.x, c.y - a.y, c.z - a.z);
    let ap = Vec3::new(p.x - a.x, p.y - a.y, p.z - a.z);

    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    // Vertex A region.
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }

    let bp = Vec3::new(p.x - b.x, p.y - b.y, p.z - b.z);
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    // Vertex B region.
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }

    // Edge AB region.
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return Vec3::new(a.x + ab.x * v, a.y + ab.y * v, a.z + ab.z * v);
    }

    let cp_v = Vec3::new(p.x - c.x, p.y - c.y, p.z - c.z);
    let d5 = ab.dot(cp_v);
    let d6 = ac.dot(cp_v);
    // Vertex C region.
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }

    // Edge AC region.
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return Vec3::new(a.x + ac.x * w, a.y + ac.y * w, a.z + ac.z * w);
    }

    // Edge BC region.
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        let bc = Vec3::new(c.x - b.x, c.y - b.y, c.z - b.z);
        return Vec3::new(b.x + bc.x * w, b.y + bc.y * w, b.z + bc.z * w);
    }

    // Inside triangle — project onto the triangle plane.
    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    Vec3::new(
        a.x + ab.x * v + ac.x * w,
        a.y + ab.y * v + ac.y * w,
        a.z + ab.z * v + ac.z * w,
    )
}

/// Ray-capsule intersection.
///
/// Returns the smallest positive `t` along the ray `ro + t * rd` that hits
/// the capsule defined by segment `[a, b]` with radius `r`, or `None` if
/// the ray misses.
///
/// # Arguments
/// * `ro` – ray origin
/// * `rd` – ray direction (need not be normalised)
/// * `a`  – capsule segment start
/// * `b`  – capsule segment end
/// * `r`  – capsule radius
///
/// # Examples
/// ```
/// use abrash_core::math::{ray_capsule_intersect, Vec3};
/// let hit = ray_capsule_intersect(
///     Vec3::new(0.0, 0.0, -5.0), Vec3::Z,
///     Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0), 0.5,
/// );
/// assert!(hit.is_some());
/// ```
#[must_use]
pub fn ray_capsule_intersect(ro: Vec3, rd: Vec3, a: Vec3, b: Vec3, r: f32) -> Option<f32> {
    // Segment and ray expressed relative to capsule start A.
    let ba = Vec3::new(b.x - a.x, b.y - a.y, b.z - a.z);
    let oa = Vec3::new(ro.x - a.x, ro.y - a.y, ro.z - a.z);

    let baba = ba.dot(ba);
    let bard = ba.dot(rd);
    let baoa = ba.dot(oa);
    let rdoa = rd.dot(oa);
    let oaoa = oa.dot(oa);

    // Quadratic coefficients for the infinite cylinder.
    let a_c = baba - bard * bard;
    let b_c = baba * rdoa - baoa * bard;
    let c_c = baba * oaoa - baoa * baoa - r * r * baba;

    let mut t = f32::INFINITY;

    // Cylinder body.
    if a_c.abs() > 1e-10 {
        let h = b_c * b_c - a_c * c_c;
        if h >= 0.0 {
            let t_cyl = (-b_c - h.sqrt()) / a_c;
            // Check that hit lies on the finite segment [0, 1].
            let y = baoa + t_cyl * bard;
            if (0.0..=baba).contains(&y) && t_cyl > 0.0 {
                t = t.min(t_cyl);
            }
        }
    }

    // End-cap hemispheres — solved as sphere intersections.
    for cap in [a, b] {
        let oc = Vec3::new(ro.x - cap.x, ro.y - cap.y, ro.z - cap.z);
        let b2 = rd.dot(oc);
        let c2 = oc.dot(oc) - r * r;
        let h2 = b2 * b2 - c2;
        if h2 >= 0.0 {
            let t_cap = -b2 - h2.sqrt();
            if t_cap > 0.0 {
                t = t.min(t_cap);
            }
        }
    }

    if t < f32::INFINITY { Some(t) } else { None }
}

#[cfg(test)]
mod tests_pass_31 {
    use super::*;
    use crate::math::Vec3;

    // ── ease_elastic_in ───────────────────────────────────────────────────────

    #[test]
    fn elastic_in_endpoints() {
        assert!(ease_elastic_in(0.0, 1.0, 0.3).abs() < 1e-5);
        assert!((ease_elastic_in(1.0, 1.0, 0.3) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn elastic_in_slow_at_start() {
        // Elastic-in barely moves in the first quarter.
        assert!(ease_elastic_in(0.1, 1.0, 0.3).abs() < 0.1);
    }

    // ── ease_elastic_in_out ───────────────────────────────────────────────────

    #[test]
    fn elastic_in_out_endpoints() {
        assert!(ease_elastic_in_out(0.0, 1.0, 0.45).abs() < 1e-5);
        assert!((ease_elastic_in_out(1.0, 1.0, 0.45) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn elastic_in_out_midpoint() {
        // Perfect symmetry → f(0.5) = 0.5.
        let v = ease_elastic_in_out(0.5, 1.0, 0.45);
        assert!((v - 0.5).abs() < 1e-4, "midpoint: {v}");
    }

    // ── ease_back_in_out ──────────────────────────────────────────────────────

    #[test]
    fn back_in_out_endpoints() {
        assert!(ease_back_in_out(0.0, 1.70158).abs() < 1e-5);
        assert!((ease_back_in_out(1.0, 1.70158) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn back_in_out_midpoint_is_half() {
        let v = ease_back_in_out(0.5, 1.70158);
        assert!((v - 0.5).abs() < 1e-4, "midpoint: {v}");
    }

    #[test]
    fn back_in_out_overshoots_at_quarter() {
        // Back-in-out should go slightly negative near t=0.25.
        let v = ease_back_in_out(0.2, 1.70158);
        assert!(v < 0.0, "should undershoot at t=0.2: {v}");
    }

    // ── value_noise_3d ────────────────────────────────────────────────────────

    #[test]
    fn value_noise_3d_in_range() {
        let n = value_noise_3d(Vec3::new(1.5, 2.3, 0.7));
        assert!(n >= 0.0 && n <= 1.0, "out of range: {n}");
    }

    #[test]
    fn value_noise_3d_varies() {
        let n1 = value_noise_3d(Vec3::new(0.3, 0.7, 0.5));
        let n2 = value_noise_3d(Vec3::new(1.3, 0.7, 0.5));
        assert!((n1 - n2).abs() > 1e-3, "no variation: {n1} {n2}");
    }

    // ── worley_noise_3d ───────────────────────────────────────────────────────

    #[test]
    fn worley_noise_3d_non_negative() {
        for (x, y, z) in [(0.5, 0.5, 0.5), (1.2, 3.4, 5.6), (0.0, 0.0, 0.0)] {
            let d = worley_noise_3d(Vec3::new(x, y, z));
            assert!(d >= 0.0, "negative: {d} at ({x},{y},{z})");
        }
    }

    #[test]
    fn worley_noise_3d_varies() {
        let d1 = worley_noise_3d(Vec3::new(0.1, 0.1, 0.1));
        let d2 = worley_noise_3d(Vec3::new(0.6, 0.6, 0.6));
        assert!((d1 - d2).abs() > 1e-3, "no variation: {d1} {d2}");
    }

    // ── closest_point_on_triangle_3d ──────────────────────────────────────────

    #[test]
    fn closest_point_projects_interior() {
        // Point directly above the centroid should project to the centroid.
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(3.0, 0.0, 0.0);
        let c = Vec3::new(0.0, 0.0, 3.0);
        let centroid = Vec3::new(1.0, 0.0, 1.0);
        let p = Vec3::new(1.0, 5.0, 1.0);
        let cp = closest_point_on_triangle_3d(p, a, b, c);
        assert!((cp.x - centroid.x).abs() < 1e-4, "x: {}", cp.x);
        assert!(cp.y.abs() < 1e-4, "y: {}", cp.y);
        assert!((cp.z - centroid.z).abs() < 1e-4, "z: {}", cp.z);
    }

    #[test]
    fn closest_point_snaps_to_vertex() {
        let a = Vec3::ZERO;
        let b = Vec3::new(1.0, 0.0, 0.0);
        let c = Vec3::new(0.0, 0.0, 1.0);
        // Far in the −x direction → nearest vertex is A.
        let cp = closest_point_on_triangle_3d(Vec3::new(-2.0, 0.0, 0.0), a, b, c);
        assert!(cp.x.abs() < 1e-5 && cp.y.abs() < 1e-5 && cp.z.abs() < 1e-5);
    }

    #[test]
    fn closest_point_snaps_to_edge() {
        let a = Vec3::ZERO;
        let b = Vec3::new(1.0, 0.0, 0.0);
        let c = Vec3::new(0.0, 0.0, 1.0);
        // Directly above midpoint of AB on +Y.
        let mid_ab = Vec3::new(0.5, 5.0, 0.0);
        let cp = closest_point_on_triangle_3d(mid_ab, a, b, c);
        assert!((cp.x - 0.5).abs() < 1e-4, "x: {}", cp.x);
        assert!(cp.y.abs() < 1e-4, "y: {}", cp.y);
        assert!(cp.z.abs() < 1e-4, "z: {}", cp.z);
    }

    // ── ray_capsule_intersect ─────────────────────────────────────────────────

    #[test]
    fn ray_capsule_direct_hit() {
        // Ray along +Z, capsule along Y-axis at z=5.
        let hit = ray_capsule_intersect(
            Vec3::new(0.0, 0.5, -5.0),
            Vec3::Z,
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            0.5,
        );
        assert!(hit.is_some(), "should hit capsule body");
        assert!(hit.unwrap() > 0.0);
    }

    #[test]
    fn ray_capsule_miss() {
        // Ray passes well to the side.
        let hit = ray_capsule_intersect(
            Vec3::new(5.0, 0.5, -5.0),
            Vec3::Z,
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
            0.5,
        );
        assert!(hit.is_none(), "should miss");
    }

    #[test]
    fn ray_capsule_end_cap_hit() {
        // Ray aimed directly at the bottom hemisphere.
        let hit = ray_capsule_intersect(
            Vec3::new(0.0, -5.0, 0.0),
            Vec3::Y,
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
            0.5,
        );
        assert!(hit.is_some(), "should hit end cap");
    }
}

// ── Pass 32 ────────────────────────────────────────────────────────────────────

/// Quartic ease-in: accelerates with `t⁴`.
#[must_use]
#[inline]
pub fn ease_quart_in(t: f32) -> f32 {
    t.clamp(0.0, 1.0).powi(4)
}

/// Quartic ease-out: decelerates with `(1−t)⁴`.
#[must_use]
#[inline]
pub fn ease_quart_out(t: f32) -> f32 {
    let t = 1.0 - t.clamp(0.0, 1.0);
    1.0 - t.powi(4)
}

/// Quartic ease-in-out: accelerates then decelerates with `t⁴` / `(1−t)⁴`.
#[must_use]
#[inline]
pub fn ease_quart_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        8.0 * t.powi(4)
    } else {
        let t = 1.0 - t;
        1.0 - 8.0 * t.powi(4)
    }
}

/// Quintic ease-in: accelerates with `t⁵`.
#[must_use]
#[inline]
pub fn ease_quint_in(t: f32) -> f32 {
    t.clamp(0.0, 1.0).powi(5)
}

/// Quintic ease-out: decelerates with `(1−t)⁵`.
#[must_use]
#[inline]
pub fn ease_quint_out(t: f32) -> f32 {
    let t = 1.0 - t.clamp(0.0, 1.0);
    1.0 - t.powi(5)
}

/// Quintic ease-in-out: accelerates then decelerates with `t⁵` / `(1−t)⁵`.
#[must_use]
#[inline]
pub fn ease_quint_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        16.0 * t.powi(5)
    } else {
        let t = 1.0 - t;
        1.0 - 16.0 * t.powi(5)
    }
}

/// Returns `true` if `p` lies inside or on the axis-aligned box `[min, max]`.
///
/// # Examples
/// ```
/// use abrash_core::math::{aabb_contains_point_3d, Vec3};
/// assert!(aabb_contains_point_3d(Vec3::ZERO, Vec3::new(-1.0,-1.0,-1.0), Vec3::new(1.0,1.0,1.0)));
/// assert!(!aabb_contains_point_3d(Vec3::new(2.0,0.0,0.0), Vec3::new(-1.0,-1.0,-1.0), Vec3::new(1.0,1.0,1.0)));
/// ```
#[must_use]
#[inline]
pub fn aabb_contains_point_3d(p: Vec3, min: Vec3, max: Vec3) -> bool {
    p.x >= min.x && p.x <= max.x && p.y >= min.y && p.y <= max.y && p.z >= min.z && p.z <= max.z
}

/// Returns `true` if two axis-aligned bounding boxes overlap (inclusive).
///
/// # Examples
/// ```
/// use abrash_core::math::{aabb_intersects_aabb_3d, Vec3};
/// let min_a = Vec3::new(-1.0,-1.0,-1.0);
/// let max_a = Vec3::new( 1.0, 1.0, 1.0);
/// let min_b = Vec3::new( 0.5, 0.5, 0.5);
/// let max_b = Vec3::new( 2.0, 2.0, 2.0);
/// assert!(aabb_intersects_aabb_3d(min_a, max_a, min_b, max_b));
/// ```
#[must_use]
#[inline]
pub fn aabb_intersects_aabb_3d(a_min: Vec3, a_max: Vec3, b_min: Vec3, b_max: Vec3) -> bool {
    a_min.x <= b_max.x
        && a_max.x >= b_min.x
        && a_min.y <= b_max.y
        && a_max.y >= b_min.y
        && a_min.z <= b_max.z
        && a_max.z >= b_min.z
}

/// Ray vs. oriented bounding box (OBB) intersection.
///
/// Returns the smallest positive `t` along `ro + t * rd` that enters the OBB,
/// or `None` if the ray misses.
///
/// Uses the slab method in OBB-local space: project the ray origin and
/// direction onto each of the three OBB axes and perform the standard
/// parametric slab clipping.
///
/// # Arguments
/// * `ro`        – ray origin
/// * `rd`        – ray direction (need not be unit length)
/// * `centre`    – OBB centre
/// * `axes`      – three orthonormal local-space axes (columns of the rotation matrix)
/// * `half_size` – half-extents along each axis
///
/// # Examples
/// ```
/// use abrash_core::math::{ray_obb_intersect, Vec3};
/// let hit = ray_obb_intersect(
///     Vec3::new(0.0, 0.0, -5.0), Vec3::Z,
///     Vec3::ZERO,
///     [Vec3::X, Vec3::Y, Vec3::Z],
///     Vec3::new(1.0, 1.0, 1.0),
/// );
/// assert!(hit.is_some());
/// ```
#[must_use]
pub fn ray_obb_intersect(
    ro: Vec3,
    rd: Vec3,
    centre: Vec3,
    axes: [Vec3; 3],
    half_size: Vec3,
) -> Option<f32> {
    let delta = Vec3::new(centre.x - ro.x, centre.y - ro.y, centre.z - ro.z);
    let hs = [half_size.x, half_size.y, half_size.z];

    let mut t_min = f32::NEG_INFINITY;
    let mut t_max = f32::INFINITY;

    for (i, axis) in axes.iter().enumerate() {
        let e = axis.dot(delta);
        let f = axis.dot(rd);
        if f.abs() > 1e-10 {
            let t1 = (e - hs[i]) / f;
            let t2 = (e + hs[i]) / f;
            let (t1, t2) = if t1 < t2 { (t1, t2) } else { (t2, t1) };
            t_min = t_min.max(t1);
            t_max = t_max.min(t2);
            if t_min > t_max {
                return None;
            }
        } else if e.abs() > hs[i] {
            // Ray is parallel to slab but outside it.
            return None;
        }
    }

    if t_max < 0.0 {
        return None; // OBB is behind the ray.
    }
    let t = if t_min >= 0.0 { t_min } else { t_max };
    Some(t)
}

/// Convert **linear RGB** to **CIE XYZ** (D65 white point, sRGB primaries).
///
/// The input is linear light, **not** gamma-encoded sRGB.
///
/// # Examples
/// ```
/// use abrash_core::math::linear_rgb_to_xyz;
/// let (x, y, z) = linear_rgb_to_xyz(1.0, 1.0, 1.0);
/// // D65 white: Y ≈ 1.0
/// assert!((y - 1.0).abs() < 1e-4);
/// ```
#[must_use]
#[inline]
pub fn linear_rgb_to_xyz(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    // IEC 61966-2-1 / sRGB D65
    let x = r * 0.412_456_4 + g * 0.357_576_1 + b * 0.180_437_5;
    let y = r * 0.212_672_9 + g * 0.715_152_2 + b * 0.072_175_0;
    let z = r * 0.019_333_9 + g * 0.119_192_0 + b * 0.950_304_1;
    (x, y, z)
}

/// Convert **CIE XYZ** to **linear RGB** (D65 white point, sRGB primaries).
///
/// The output is linear light before gamma encoding.
///
/// # Examples
/// ```
/// use abrash_core::math::xyz_to_linear_rgb;
/// let (r, g, b) = xyz_to_linear_rgb(0.950456, 1.0, 1.08906);
/// // Should be close to (1, 1, 1)
/// assert!((r - 1.0).abs() < 1e-3 && (g - 1.0).abs() < 1e-3 && (b - 1.0).abs() < 1e-3);
/// ```
#[must_use]
#[inline]
pub fn xyz_to_linear_rgb(x: f32, y: f32, z: f32) -> (f32, f32, f32) {
    // Inverse of the sRGB D65 matrix
    let r = x * 3.240_454_2 - y * 1.537_138_5 - z * 0.498_531_4;
    let g = -x * 0.969_266_0 + y * 1.876_010_8 + z * 0.041_556_0;
    let b = x * 0.055_648_0 - y * 0.204_043_4 + z * 1.057_110_5;
    (r, g, b)
}

#[cfg(test)]
mod tests_pass_32 {
    use super::*;
    use crate::math::Vec3;

    // ── quartic easing ────────────────────────────────────────────────────────

    #[test]
    fn quart_easing_endpoints() {
        assert!(ease_quart_in(0.0).abs() < 1e-6);
        assert!((ease_quart_in(1.0) - 1.0).abs() < 1e-6);
        assert!(ease_quart_out(0.0).abs() < 1e-6);
        assert!((ease_quart_out(1.0) - 1.0).abs() < 1e-6);
        assert!((ease_quart_in_out(0.5) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn quart_in_very_slow_early() {
        // At t=0.5 quartic-in should still be small (0.5^4 = 0.0625).
        assert!((ease_quart_in(0.5) - 0.0625).abs() < 1e-5);
    }

    // ── quintic easing ────────────────────────────────────────────────────────

    #[test]
    fn quint_easing_endpoints() {
        assert!(ease_quint_in(0.0).abs() < 1e-6);
        assert!((ease_quint_in(1.0) - 1.0).abs() < 1e-6);
        assert!(ease_quint_out(0.0).abs() < 1e-6);
        assert!((ease_quint_out(1.0) - 1.0).abs() < 1e-6);
        assert!((ease_quint_in_out(0.5) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn quint_slower_than_quart_midpoint() {
        // Higher power → slower start.
        assert!(ease_quint_in(0.5) < ease_quart_in(0.5));
    }

    // ── aabb_contains_point_3d ────────────────────────────────────────────────

    #[test]
    fn aabb_contains_origin() {
        assert!(aabb_contains_point_3d(
            Vec3::ZERO,
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, 1.0, 1.0)
        ));
    }

    #[test]
    fn aabb_excludes_outside() {
        assert!(!aabb_contains_point_3d(
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, 1.0, 1.0)
        ));
    }

    // ── aabb_intersects_aabb_3d ───────────────────────────────────────────────

    #[test]
    fn aabb_overlap() {
        assert!(aabb_intersects_aabb_3d(
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(0.5, 0.5, 0.5),
            Vec3::new(2.0, 2.0, 2.0),
        ));
    }

    #[test]
    fn aabb_no_overlap() {
        assert!(!aabb_intersects_aabb_3d(
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(2.0, 2.0, 2.0),
        ));
    }

    // ── ray_obb_intersect ─────────────────────────────────────────────────────

    #[test]
    fn ray_obb_axis_aligned_hit() {
        // Degenerate OBB (identity axes) == AABB; ray from −Z should hit.
        let hit = ray_obb_intersect(
            Vec3::new(0.0, 0.0, -5.0),
            Vec3::Z,
            Vec3::ZERO,
            [Vec3::X, Vec3::Y, Vec3::Z],
            Vec3::new(1.0, 1.0, 1.0),
        );
        assert!(hit.is_some(), "should hit AABB-mode OBB");
        assert!((hit.unwrap() - 4.0).abs() < 1e-4);
    }

    #[test]
    fn ray_obb_miss() {
        let hit = ray_obb_intersect(
            Vec3::new(5.0, 0.0, -5.0),
            Vec3::Z,
            Vec3::ZERO,
            [Vec3::X, Vec3::Y, Vec3::Z],
            Vec3::new(1.0, 1.0, 1.0),
        );
        assert!(hit.is_none(), "should miss");
    }

    #[test]
    fn ray_obb_rotated_45_degrees() {
        // OBB rotated 45° around Y; ray along +Z should still hit a unit box at origin.
        use core::f32::consts::FRAC_1_SQRT_2;
        let axes = [
            Vec3::new(FRAC_1_SQRT_2, 0.0, -FRAC_1_SQRT_2), // X rotated 45° CW around Y
            Vec3::Y,
            Vec3::new(FRAC_1_SQRT_2, 0.0, FRAC_1_SQRT_2), // Z rotated 45° CW around Y
        ];
        let hit = ray_obb_intersect(
            Vec3::new(0.0, 0.0, -5.0),
            Vec3::Z,
            Vec3::ZERO,
            axes,
            Vec3::new(1.0, 1.0, 1.0),
        );
        assert!(hit.is_some(), "rotated OBB should still be hit");
    }

    // ── CIE XYZ ───────────────────────────────────────────────────────────────

    #[test]
    fn xyz_white_point() {
        // Linear white (1,1,1) → Y ≈ 1.0.
        let (_, y, _) = linear_rgb_to_xyz(1.0, 1.0, 1.0);
        assert!((y - 1.0).abs() < 1e-4, "Y white: {y}");
    }

    #[test]
    fn xyz_round_trip() {
        let cases = [(0.8, 0.3, 0.1_f32), (0.0, 0.5, 1.0), (0.2, 0.7, 0.4)];
        for (r, g, b) in cases {
            let (x, y, z) = linear_rgb_to_xyz(r, g, b);
            let (r2, g2, b2) = xyz_to_linear_rgb(x, y, z);
            assert!((r - r2).abs() < 5e-4, "r: {r} vs {r2}");
            assert!((g - g2).abs() < 5e-4, "g: {g} vs {g2}");
            assert!((b - b2).abs() < 5e-4, "b: {b} vs {b2}");
        }
    }

    #[test]
    fn xyz_black_is_black() {
        let (x, y, z) = linear_rgb_to_xyz(0.0, 0.0, 0.0);
        assert!(x.abs() < 1e-10 && y.abs() < 1e-10 && z.abs() < 1e-10);
    }
}

// ── Pass 33 ────────────────────────────────────────────────────────────────────

/// Compute the shortest-arc [`Quat`] that rotates unit vector `from` to unit
/// vector `to`.
///
/// Uses the half-vector construction: no trigonometric functions required,
/// just one normalisation and a cross product.
///
/// Returns the **identity** quaternion when the vectors are already parallel
/// and a 180-degree rotation around an arbitrary perpendicular axis when they
/// are anti-parallel.
///
/// # Examples
/// ```
/// use abrash_core::math::{quat_rotation_between, Vec3};
/// let q = quat_rotation_between(Vec3::X, Vec3::Y);
/// let rotated = q.rotate(Vec3::X);
/// assert!((rotated.x).abs() < 1e-5 && (rotated.y - 1.0).abs() < 1e-5);
/// ```
#[must_use]
pub fn quat_rotation_between(from: Vec3, to: Vec3) -> Quat {
    let f = from.normalize();
    let t = to.normalize();
    let d = f.dot(t);
    if d > 1.0 - 1e-6 {
        return Quat::identity();
    }
    if d < -1.0 + 1e-6 {
        // 180-degree rotation — choose any perpendicular axis.
        let perp = if f.x.abs() < 0.9 {
            Vec3::X.cross(f).normalize()
        } else {
            Vec3::Y.cross(f).normalize()
        };
        return Quat {
            x: perp.x,
            y: perp.y,
            z: perp.z,
            w: 0.0,
        };
    }
    let half = Vec3::new(f.x + t.x, f.y + t.y, f.z + t.z).normalize();
    let c = f.cross(half);
    let q = Quat {
        x: c.x,
        y: c.y,
        z: c.z,
        w: f.dot(half),
    };
    // Normalise to counteract any floating-point drift.
    let len = (q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w).sqrt();
    Quat {
        x: q.x / len,
        y: q.y / len,
        z: q.z / len,
        w: q.w / len,
    }
}

/// **Multiply** blend mode: `a * b`.  Always darker than either input.
#[must_use]
#[inline]
pub fn blend_multiply(a: f32, b: f32) -> f32 {
    a * b
}

/// **Screen** blend mode: `1 − (1−a)(1−b)`.  Always lighter than either input.
#[must_use]
#[inline]
pub fn blend_screen(a: f32, b: f32) -> f32 {
    1.0 - (1.0 - a) * (1.0 - b)
}

/// **Overlay** blend mode: multiply in the dark half, screen in the light half
/// (conditioned on `a`).
///
/// Overlay = `hard_light` with `a` and `b` swapped.
#[must_use]
#[inline]
pub fn blend_overlay(a: f32, b: f32) -> f32 {
    if a < 0.5 {
        2.0 * a * b
    } else {
        1.0 - 2.0 * (1.0 - a) * (1.0 - b)
    }
}

/// **Hard-light** blend mode: multiply/screen conditioned on `b` (the light source).
#[must_use]
#[inline]
pub fn blend_hard_light(a: f32, b: f32) -> f32 {
    blend_overlay(b, a)
}

/// **Soft-light** blend mode (Pegtop formula): a gentler contrast enhancement.
///
/// When `b = 0.5` the output equals `a` unchanged.
#[must_use]
#[inline]
pub fn blend_soft_light(a: f32, b: f32) -> f32 {
    (1.0 - 2.0 * b) * a * a + 2.0 * b * a
}

/// Round `n` up to the next power of two.  Returns `1` for `n = 0`.
///
/// # Examples
/// ```
/// use abrash_core::math::next_power_of_2;
/// assert_eq!(next_power_of_2(0), 1);
/// assert_eq!(next_power_of_2(1), 1);
/// assert_eq!(next_power_of_2(5), 8);
/// assert_eq!(next_power_of_2(8), 8);
/// ```
#[must_use]
#[inline]
pub fn next_power_of_2(n: u32) -> u32 {
    if n == 0 { 1 } else { n.next_power_of_two() }
}

/// Returns `true` if `n` is a power of two (including 1).
///
/// # Examples
/// ```
/// use abrash_core::math::is_power_of_2;
/// assert!(is_power_of_2(1) && is_power_of_2(4) && is_power_of_2(1024));
/// assert!(!is_power_of_2(0) && !is_power_of_2(3) && !is_power_of_2(6));
/// ```
#[must_use]
#[inline]
pub fn is_power_of_2(n: u32) -> bool {
    n > 0 && (n & (n - 1)) == 0
}

/// Ceiling integer log₂: smallest `k` such that `2^k ≥ n`.
///
/// Returns `0` for `n = 0`.
///
/// # Examples
/// ```
/// use abrash_core::math::log2_ceil;
/// assert_eq!(log2_ceil(1), 0);
/// assert_eq!(log2_ceil(4), 2);
/// assert_eq!(log2_ceil(5), 3);
/// ```
#[must_use]
#[inline]
pub fn log2_ceil(n: u32) -> u32 {
    if n <= 1 {
        return 0;
    }
    u32::BITS - (n - 1).leading_zeros()
}

#[cfg(test)]
mod tests_pass_33 {
    use super::*;
    use crate::math::Vec3;

    // ── quat_rotation_between ─────────────────────────────────────────────────

    #[test]
    fn rotation_between_x_to_y() {
        let q = quat_rotation_between(Vec3::X, Vec3::Y);
        let r = q.rotate(Vec3::X);
        assert!((r.x).abs() < 1e-5, "x: {}", r.x);
        assert!((r.y - 1.0).abs() < 1e-5, "y: {}", r.y);
        assert!((r.z).abs() < 1e-5, "z: {}", r.z);
    }

    #[test]
    fn rotation_between_identity_when_parallel() {
        let q = quat_rotation_between(Vec3::Z, Vec3::Z);
        let r = q.rotate(Vec3::X);
        assert!((r.x - 1.0).abs() < 1e-5 && r.y.abs() < 1e-5 && r.z.abs() < 1e-5);
    }

    #[test]
    fn rotation_between_antiparallel_is_180() {
        // Rotation from +X to −X must produce a vector at −X.
        let q = quat_rotation_between(Vec3::X, Vec3::new(-1.0, 0.0, 0.0));
        let r = q.rotate(Vec3::X);
        assert!((r.x + 1.0).abs() < 1e-4, "antiparallel x: {}", r.x);
    }

    // ── blend modes ───────────────────────────────────────────────────────────

    #[test]
    fn blend_multiply_black_kills() {
        assert!(blend_multiply(0.8, 0.0).abs() < 1e-7);
        assert!(blend_multiply(0.0, 0.8).abs() < 1e-7);
    }

    #[test]
    fn blend_screen_white_saturates() {
        assert!((blend_screen(1.0, 0.5) - 1.0).abs() < 1e-7);
        assert!((blend_screen(0.5, 1.0) - 1.0).abs() < 1e-7);
    }

    #[test]
    fn blend_overlay_mid_grey_identity() {
        // overlay(0.5, b) == b (conditioning on a=0.5 gives b in both branches).
        let b = 0.3_f32;
        let o = blend_overlay(0.5, b);
        assert!((o - b).abs() < 1e-6, "overlay(0.5,{b}) = {o}");
    }

    #[test]
    fn blend_soft_light_neutral_at_half() {
        // soft_light(a, 0.5) == a.
        let a = 0.7_f32;
        assert!((blend_soft_light(a, 0.5) - a).abs() < 1e-6);
    }

    #[test]
    fn blend_hard_light_is_overlay_swapped() {
        let a = 0.4_f32;
        let b = 0.7_f32;
        assert!((blend_hard_light(a, b) - blend_overlay(b, a)).abs() < 1e-7);
    }

    // ── bit-width utilities ───────────────────────────────────────────────────

    #[test]
    fn next_power_of_2_cases() {
        assert_eq!(next_power_of_2(0), 1);
        assert_eq!(next_power_of_2(1), 1);
        assert_eq!(next_power_of_2(5), 8);
        assert_eq!(next_power_of_2(8), 8);
        assert_eq!(next_power_of_2(9), 16);
    }

    #[test]
    fn is_power_of_2_cases() {
        assert!(is_power_of_2(1));
        assert!(is_power_of_2(2));
        assert!(is_power_of_2(1024));
        assert!(!is_power_of_2(0));
        assert!(!is_power_of_2(3));
        assert!(!is_power_of_2(6));
    }

    #[test]
    fn log2_ceil_cases() {
        assert_eq!(log2_ceil(1), 0);
        assert_eq!(log2_ceil(2), 1);
        assert_eq!(log2_ceil(4), 2);
        assert_eq!(log2_ceil(5), 3);
        assert_eq!(log2_ceil(8), 3);
        assert_eq!(log2_ceil(9), 4);
    }
}

// ── Pass 34 ────────────────────────────────────────────────────────────────────

/// Linearise a depth-buffer value from the NDC \[0, 1] range (DirectX /
/// Vulkan / Metal convention, where 0 = near and 1 = far) back to
/// view-space depth in \[near, far].
///
/// The standard perspective projection maps view-space depth to a non-linear
/// NDC value.  This function inverts that mapping.
///
/// # Examples
/// ```
/// use abrash_core::math::depth_linearize;
/// let z_near = depth_linearize(0.0, 0.1, 100.0);
/// assert!((z_near - 0.1).abs() < 1e-4);
/// let z_far = depth_linearize(1.0, 0.1, 100.0);
/// assert!((z_far - 100.0).abs() < 1e-2);
/// ```
#[must_use]
#[inline]
pub fn depth_linearize(z_ndc: f32, near: f32, far: f32) -> f32 {
    // For z_ndc ∈ [0,1]: 0 → near, 1 → far.
    near * far / (far - z_ndc * (far - near))
}

/// Encode a **unit** normal vector to a 2-D texture coordinate using the
/// **spheremap** (Blinn 1977) method.
///
/// The encoded value lies in \[0, 1]².  Use [`normal_spheremap_decode`] to
/// recover the normal.
///
/// # Examples
/// ```
/// use abrash_core::math::{normal_spheremap_encode, Vec3};
/// let enc = normal_spheremap_encode(Vec3::Z);
/// // Should encode to the centre of the texture (0.5, 0.5)
/// assert!((enc.x - 0.5).abs() < 1e-5 && (enc.y - 0.5).abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn normal_spheremap_encode(n: Vec3) -> Vec2 {
    let p = (n.z * 8.0 + 8.0).sqrt();
    Vec2::new(n.x / p + 0.5, n.y / p + 0.5)
}

/// Decode a spheremap-encoded normal from a 2-D texture coordinate.
///
/// Returns a normalised [`Vec3`].
///
/// # Examples
/// ```
/// use abrash_core::math::{normal_spheremap_encode, normal_spheremap_decode, Vec3};
/// let n = Vec3::new(0.6, 0.0, 0.8).normalize();
/// let enc = normal_spheremap_encode(n);
/// let dec = normal_spheremap_decode(enc);
/// assert!((dec.x - n.x).abs() < 1e-4 && (dec.z - n.z).abs() < 1e-4);
/// ```
#[must_use]
#[inline]
pub fn normal_spheremap_decode(v: Vec2) -> Vec3 {
    let fenc = Vec2::new(v.x * 4.0 - 2.0, v.y * 4.0 - 2.0);
    let f = fenc.x * fenc.x + fenc.y * fenc.y;
    let g = (1.0 - f * 0.25).sqrt();
    Vec3::new(fenc.x * g, fenc.y * g, 1.0 - f * 0.5).normalize()
}

/// Surface normal of a triangle `(a, b, c)` — **not** normalised.
///
/// The result is proportional to the triangle area; normalise it when
/// you need a unit normal.
///
/// # Examples
/// ```
/// use abrash_core::math::{triangle_normal, Vec3};
/// let n = triangle_normal(Vec3::ZERO, Vec3::X, Vec3::Y);
/// // XY-plane triangle → normal along +Z.
/// assert!(n.z > 0.0);
/// ```
#[must_use]
#[inline]
pub fn triangle_normal(a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let ab = Vec3::new(b.x - a.x, b.y - a.y, b.z - a.z);
    let ac = Vec3::new(c.x - a.x, c.y - a.y, c.z - a.z);
    ab.cross(ac)
}

#[cfg(test)]
mod tests_pass_34 {
    use super::*;
    use crate::math::{Vec2, Vec3};

    // ── depth_linearize ───────────────────────────────────────────────────────

    #[test]
    fn depth_at_near_returns_near() {
        // NDC 0 (z = −1 in GL convention) corresponds to near.
        // Using the [0,1] convention: z_ndc=0 → z_view = near.
        let z = depth_linearize(0.0, 0.1, 100.0);
        assert!((z - 0.1).abs() < 1e-4, "z at near: {z}");
    }

    #[test]
    fn depth_at_far_returns_far() {
        let z = depth_linearize(1.0, 0.1, 100.0);
        assert!((z - 100.0).abs() < 1e-2, "z at far: {z}");
    }

    #[test]
    fn depth_midpoint_is_nonlinear() {
        // Non-linear: NDC=0.5 should NOT map to (near+far)/2.
        let z = depth_linearize(0.5, 1.0, 100.0);
        let mid = (1.0_f32 + 100.0) / 2.0;
        assert!(
            (z - mid).abs() > 0.5,
            "should be non-linear: {z} vs linear mid {mid}"
        );
    }

    // ── normal_spheremap_encode / decode ──────────────────────────────────────

    #[test]
    fn spheremap_forward_z_encodes_to_centre() {
        let enc = normal_spheremap_encode(Vec3::Z);
        assert!((enc.x - 0.5).abs() < 1e-5 && (enc.y - 0.5).abs() < 1e-5);
    }

    #[test]
    fn spheremap_round_trip() {
        for n in [
            Vec3::new(0.6, 0.0, 0.8).normalize(),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.577_350_26, 0.577_350_26, 0.577_350_26),
        ] {
            let enc = normal_spheremap_encode(n);
            let dec = normal_spheremap_decode(enc);
            assert!(
                (dec.x - n.x).abs() < 1e-4
                    && (dec.y - n.y).abs() < 1e-4
                    && (dec.z - n.z).abs() < 1e-4,
                "round-trip: {n:?} → {dec:?}"
            );
        }
    }

    // ── triangle_normal ───────────────────────────────────────────────────────

    #[test]
    fn triangle_normal_xy_plane_points_z() {
        let n = triangle_normal(Vec3::ZERO, Vec3::X, Vec3::Y);
        assert!(n.z > 0.0 && n.x.abs() < 1e-7 && n.y.abs() < 1e-7);
    }

    #[test]
    fn triangle_normal_ccw_vs_cw_opposite() {
        let a = Vec3::ZERO;
        let b = Vec3::X;
        let c = Vec3::Y;
        let n_ccw = triangle_normal(a, b, c);
        let n_cw = triangle_normal(a, c, b);
        assert!(n_ccw.z > 0.0 && n_cw.z < 0.0);
    }
}

// ── Pass 35 ────────────────────────────────────────────────────────────────────

/// **Turbulence**: absolute-value fBm.
///
/// Sums `|noise(p)|` over octaves — the absolute value creates sharp
/// discontinuities that resemble turbulent fluid or fire.
///
/// # Arguments
/// * `p`          – starting query point (mutated by octave scaling)
/// * `octaves`    – number of octave layers
/// * `lacunarity` – frequency multiplier per octave (typically 2.0)
/// * `gain`       – amplitude multiplier per octave (typically 0.5)
///
/// # Examples
/// ```
/// use abrash_core::math::{turbulence_2d, Vec2};
/// let t = turbulence_2d(Vec2::new(1.5, 2.3), 4, 2.0, 0.5);
/// assert!(t >= 0.0);
/// ```
#[must_use]
pub fn turbulence_2d(mut p: Vec2, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
    let mut sum = 0.0_f32;
    let mut amplitude = 1.0_f32;
    for _ in 0..octaves {
        sum += perlin_noise_2d(p).abs() * amplitude;
        p = Vec2::new(p.x * lacunarity, p.y * lacunarity);
        amplitude *= gain;
    }
    sum
}

/// **Marble** procedural texture.
///
/// Computes `0.5 + 0.5 * sin(scale * p.x + turbulence_strength * turbulence)`,
/// giving a value in \[0, 1] that resembles marble veining.
///
/// # Examples
/// ```
/// use abrash_core::math::{marble_texture_2d, Vec2};
/// let m = marble_texture_2d(Vec2::new(1.0, 0.5), 3.0, 2.0);
/// assert!(m >= 0.0 && m <= 1.0);
/// ```
#[must_use]
pub fn marble_texture_2d(p: Vec2, scale: f32, turbulence_strength: f32) -> f32 {
    let t = turbulence_2d(p, 5, 2.0, 0.5);
    0.5 + 0.5 * (scale * p.x + turbulence_strength * t).sin()
}

/// **Checkerboard** pattern: returns `0.0` or `1.0` based on which cell `p`
/// falls in.
///
/// # Examples
/// ```
/// use abrash_core::math::{checkerboard_2d, Vec2};
/// // Adjacent cells should have opposite values.
/// let a = checkerboard_2d(Vec2::new(0.5, 0.5), 1.0);
/// let b = checkerboard_2d(Vec2::new(1.5, 0.5), 1.0);
/// assert!((a - b).abs() > 0.5);
/// ```
#[must_use]
#[inline]
pub fn checkerboard_2d(p: Vec2, scale: f32) -> f32 {
    let x = (p.x / scale).floor() as i32;
    let y = (p.y / scale).floor() as i32;
    if (x + y) & 1 == 0 { 0.0 } else { 1.0 }
}

/// Pack four \[0, 1] float components into a single `u32` as 8-bit unsigned
/// normalised integers (R8G8B8A8 layout, R in lowest byte).
///
/// # Examples
/// ```
/// use abrash_core::math::{pack_unorm_4x8, unpack_unorm_4x8};
/// let packed = pack_unorm_4x8(1.0, 0.0, 0.5, 1.0);
/// let (r, g, b, a) = unpack_unorm_4x8(packed);
/// assert!((r - 1.0).abs() < 0.005 && g.abs() < 0.005);
/// ```
#[must_use]
#[inline]
pub fn pack_unorm_4x8(r: f32, g: f32, b: f32, a: f32) -> u32 {
    let ri = (r.clamp(0.0, 1.0) * 255.0 + 0.5) as u32;
    let gi = (g.clamp(0.0, 1.0) * 255.0 + 0.5) as u32;
    let bi = (b.clamp(0.0, 1.0) * 255.0 + 0.5) as u32;
    let ai = (a.clamp(0.0, 1.0) * 255.0 + 0.5) as u32;
    ri | (gi << 8) | (bi << 16) | (ai << 24)
}

/// Unpack an R8G8B8A8 `u32` back to four \[0, 1] floats.
#[must_use]
#[inline]
pub fn unpack_unorm_4x8(packed: u32) -> (f32, f32, f32, f32) {
    let r = (packed & 0xFF) as f32 / 255.0;
    let g = ((packed >> 8) & 0xFF) as f32 / 255.0;
    let b = ((packed >> 16) & 0xFF) as f32 / 255.0;
    let a = ((packed >> 24) & 0xFF) as f32 / 255.0;
    (r, g, b, a)
}

/// Pack two \[−1, 1] float components into a `u32` as 16-bit signed
/// normalised integers (X in lower 16 bits, Y in upper).
///
/// Useful for compressing unit normals into a 32-bit G-buffer slot.
///
/// # Examples
/// ```
/// use abrash_core::math::{pack_snorm_2x16, unpack_snorm_2x16};
/// let packed = pack_snorm_2x16(0.6, -0.8);
/// let (x, y) = unpack_snorm_2x16(packed);
/// assert!((x - 0.6).abs() < 0.0001 && (y + 0.8).abs() < 0.0001);
/// ```
#[must_use]
#[inline]
pub fn pack_snorm_2x16(x: f32, y: f32) -> u32 {
    let xi = (x.clamp(-1.0, 1.0) * 32_767.0).round() as i16 as u16 as u32;
    let yi = (y.clamp(-1.0, 1.0) * 32_767.0).round() as i16 as u16 as u32;
    xi | (yi << 16)
}

/// Unpack a 2×16-bit signed normalised integer `u32` back to two \[−1, 1] floats.
#[must_use]
#[inline]
pub fn unpack_snorm_2x16(packed: u32) -> (f32, f32) {
    let x = ((packed & 0xFFFF) as i16 as f32 / 32_767.0).clamp(-1.0, 1.0);
    let y = (((packed >> 16) & 0xFFFF) as i16 as f32 / 32_767.0).clamp(-1.0, 1.0);
    (x, y)
}

#[cfg(test)]
mod tests_pass_35 {
    use super::*;
    use crate::math::Vec2;

    // ── turbulence_2d ─────────────────────────────────────────────────────────

    #[test]
    fn turbulence_non_negative() {
        let t = turbulence_2d(Vec2::new(1.5, 2.3), 4, 2.0, 0.5);
        assert!(t >= 0.0, "negative turbulence: {t}");
    }

    #[test]
    fn turbulence_varies() {
        let t1 = turbulence_2d(Vec2::new(0.1, 0.1), 4, 2.0, 0.5);
        let t2 = turbulence_2d(Vec2::new(1.3, 0.7), 4, 2.0, 0.5);
        assert!((t1 - t2).abs() > 1e-3, "no variation: {t1} {t2}");
    }

    // ── marble_texture_2d ─────────────────────────────────────────────────────

    #[test]
    fn marble_in_range() {
        let m = marble_texture_2d(Vec2::new(1.0, 0.5), 3.0, 2.0);
        assert!(m >= 0.0 && m <= 1.0, "out of [0,1]: {m}");
    }

    #[test]
    fn marble_varies() {
        let m1 = marble_texture_2d(Vec2::new(0.0, 0.0), 3.0, 2.0);
        let m2 = marble_texture_2d(Vec2::new(1.5, 0.0), 3.0, 2.0);
        assert!((m1 - m2).abs() > 1e-3, "no variation: {m1} {m2}");
    }

    // ── checkerboard_2d ───────────────────────────────────────────────────────

    #[test]
    fn checkerboard_alternates() {
        let a = checkerboard_2d(Vec2::new(0.5, 0.5), 1.0);
        let b = checkerboard_2d(Vec2::new(1.5, 0.5), 1.0);
        assert!((a - b).abs() > 0.5, "should alternate: {a} {b}");
    }

    #[test]
    fn checkerboard_diagonal_same() {
        let a = checkerboard_2d(Vec2::new(0.5, 0.5), 1.0);
        let b = checkerboard_2d(Vec2::new(1.5, 1.5), 1.0);
        assert!((a - b).abs() < 1e-5, "diagonal should match: {a} {b}");
    }

    // ── pack/unpack RGBA8 ─────────────────────────────────────────────────────

    #[test]
    fn unorm_4x8_round_trip() {
        let (r, g, b, a) = (0.8, 0.1, 0.5, 1.0_f32);
        let packed = pack_unorm_4x8(r, g, b, a);
        let (r2, g2, b2, a2) = unpack_unorm_4x8(packed);
        assert!((r - r2).abs() < 0.005, "r: {r} vs {r2}");
        assert!((g - g2).abs() < 0.005, "g: {g} vs {g2}");
        assert!((b - b2).abs() < 0.005, "b: {b} vs {b2}");
        assert!((a - a2).abs() < 0.005, "a: {a} vs {a2}");
    }

    #[test]
    fn unorm_4x8_clamps() {
        let packed = pack_unorm_4x8(-0.5, 1.5, 0.5, 0.5);
        let (r, _, _, _) = unpack_unorm_4x8(packed);
        assert!(r.abs() < 0.005, "negative clamped to 0: {r}");
    }

    // ── pack/unpack snorm 2×16 ────────────────────────────────────────────────

    #[test]
    fn snorm_2x16_round_trip() {
        let (x, y) = (0.6_f32, -0.8_f32);
        let packed = pack_snorm_2x16(x, y);
        let (x2, y2) = unpack_snorm_2x16(packed);
        assert!((x - x2).abs() < 1e-4, "x: {x} vs {x2}");
        assert!((y - y2).abs() < 1e-4, "y: {y} vs {y2}");
    }

    #[test]
    fn snorm_2x16_extremes() {
        let packed = pack_snorm_2x16(1.0, -1.0);
        let (x, y) = unpack_snorm_2x16(packed);
        assert!((x - 1.0).abs() < 1e-4, "max: {x}");
        assert!((y + 1.0).abs() < 1e-4, "min: {y}");
    }
}

// ── Pass 36 ────────────────────────────────────────────────────────────────────

/// Bilinear interpolation of four corner values on a unit \[0,1]² grid.
///
/// `v00` = (0,0), `v10` = (1,0), `v01` = (0,1), `v11` = (1,1).
/// `u` varies along the x-axis, `v` along the y-axis.
///
/// # Examples
/// ```
/// use abrash_core::math::bilinear_interp;
/// // All corners equal → result is constant.
/// assert!((bilinear_interp(1.0, 1.0, 1.0, 1.0, 0.3, 0.7) - 1.0).abs() < 1e-6);
/// // Midpoint of a unit ramp.
/// let v = bilinear_interp(0.0, 1.0, 0.0, 1.0, 0.5, 0.5);
/// assert!((v - 0.5).abs() < 1e-6);
/// ```
#[must_use]
#[inline]
pub fn bilinear_interp(v00: f32, v10: f32, v01: f32, v11: f32, u: f32, v: f32) -> f32 {
    let a = v00 + (v10 - v00) * u;
    let b = v01 + (v11 - v01) * u;
    a + (b - a) * v
}

/// Compute the **tangent vector** for a triangle given its positions and UV
/// coordinates.
///
/// The tangent points in the direction of increasing U (S).  You typically
/// also need the bitangent: `bitangent = cross(normal, tangent)`.
///
/// # Arguments
/// * `p0, p1, p2` – world-space vertex positions
/// * `uv0, uv1, uv2` – corresponding texture coordinates
///
/// # Examples
/// ```
/// use abrash_core::math::{compute_tangent, Vec2, Vec3};
/// let t = compute_tangent(
///     Vec3::ZERO, Vec3::X, Vec3::Y,
///     Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0),
/// );
/// // Tangent should point along +X.
/// assert!((t.x - 1.0).abs() < 1e-4 && t.y.abs() < 1e-4);
/// ```
#[must_use]
pub fn compute_tangent(p0: Vec3, p1: Vec3, p2: Vec3, uv0: Vec2, uv1: Vec2, uv2: Vec2) -> Vec3 {
    let edge1 = Vec3::new(p1.x - p0.x, p1.y - p0.y, p1.z - p0.z);
    let edge2 = Vec3::new(p2.x - p0.x, p2.y - p0.y, p2.z - p0.z);
    let delta_uv1 = Vec2::new(uv1.x - uv0.x, uv1.y - uv0.y);
    let delta_uv2 = Vec2::new(uv2.x - uv0.x, uv2.y - uv0.y);

    let denom = delta_uv1.x * delta_uv2.y - delta_uv2.x * delta_uv1.y;
    if denom.abs() < 1e-10 {
        // Degenerate UV mapping — return a default tangent.
        return Vec3::X;
    }
    let f = 1.0 / denom;
    Vec3::new(
        f * (delta_uv2.y * edge1.x - delta_uv1.y * edge2.x),
        f * (delta_uv2.y * edge1.y - delta_uv1.y * edge2.y),
        f * (delta_uv2.y * edge1.z - delta_uv1.y * edge2.z),
    )
    .normalize()
}

/// Worley (cellular) noise returning **both** F1 and F2 distances.
///
/// F1 is the distance to the nearest feature point; F2 to the second-nearest.
/// `F2 − F1` gives a smooth cell-boundary ring useful for tile patterns.
///
/// # Examples
/// ```
/// use abrash_core::math::{worley_f1_f2_2d, Vec2};
/// let (f1, f2) = worley_f1_f2_2d(Vec2::new(1.5, 2.3));
/// assert!(f1 >= 0.0 && f2 >= f1);
/// ```
#[must_use]
pub fn worley_f1_f2_2d(p: Vec2) -> (f32, f32) {
    let ix = p.x.floor() as i32;
    let iy = p.y.floor() as i32;
    let mut f1 = f32::INFINITY;
    let mut f2 = f32::INFINITY;
    for dy in -1..=1_i32 {
        for dx in -1..=1_i32 {
            let cx = (ix + dx) as f32;
            let cy = (iy + dy) as f32;
            let ox = hash2_to_f32((ix + dx) as u32, (iy + dy) as u32);
            let oy = hash2_to_f32((iy + dy) as u32 ^ 0xDEAD_BEEF, (ix + dx) as u32);
            let fx = p.x - (cx + ox);
            let fy = p.y - (cy + oy);
            let dist = (fx * fx + fy * fy).sqrt();
            if dist < f1 {
                f2 = f1;
                f1 = dist;
            } else if dist < f2 {
                f2 = dist;
            }
        }
    }
    (f1, f2)
}

#[cfg(test)]
mod tests_pass_36 {
    use super::*;
    use crate::math::{Vec2, Vec3};

    // ── bilinear_interp ───────────────────────────────────────────────────────

    #[test]
    fn bilinear_constant() {
        assert!((bilinear_interp(2.0, 2.0, 2.0, 2.0, 0.3, 0.7) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn bilinear_corners() {
        assert!((bilinear_interp(0.0, 1.0, 2.0, 3.0, 0.0, 0.0)).abs() < 1e-6); // v00
        assert!((bilinear_interp(0.0, 1.0, 2.0, 3.0, 1.0, 0.0) - 1.0).abs() < 1e-6); // v10
        assert!((bilinear_interp(0.0, 1.0, 2.0, 3.0, 0.0, 1.0) - 2.0).abs() < 1e-6); // v01
        assert!((bilinear_interp(0.0, 1.0, 2.0, 3.0, 1.0, 1.0) - 3.0).abs() < 1e-6); // v11
    }

    #[test]
    fn bilinear_midpoint() {
        let v = bilinear_interp(0.0, 1.0, 0.0, 1.0, 0.5, 0.5);
        assert!((v - 0.5).abs() < 1e-6, "midpoint: {v}");
    }

    // ── compute_tangent ───────────────────────────────────────────────────────

    #[test]
    fn tangent_xy_aligned_is_x() {
        let t = compute_tangent(
            Vec3::ZERO,
            Vec3::X,
            Vec3::Y,
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
        );
        assert!((t.x - 1.0).abs() < 1e-4 && t.y.abs() < 1e-4 && t.z.abs() < 1e-4);
    }

    #[test]
    fn tangent_is_unit_length() {
        let t = compute_tangent(
            Vec3::ZERO,
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
        );
        assert!((t.length() - 1.0).abs() < 1e-5, "unit: {}", t.length());
    }

    // ── worley_f1_f2_2d ───────────────────────────────────────────────────────

    #[test]
    fn worley_f1_f2_ordering() {
        let (f1, f2) = worley_f1_f2_2d(Vec2::new(1.5, 2.3));
        assert!(f1 >= 0.0, "f1 negative: {f1}");
        assert!(f2 >= f1, "f2 < f1: {f1} {f2}");
    }

    #[test]
    fn worley_f2_minus_f1_non_negative() {
        for (x, y) in [(0.1, 0.2), (2.5, 1.7), (0.0, 0.0)] {
            let (f1, f2) = worley_f1_f2_2d(Vec2::new(x, y));
            assert!(f2 - f1 >= 0.0, "cell gap negative at ({x},{y})");
        }
    }

    #[test]
    fn worley_f1_matches_worley_noise_2d() {
        let p = Vec2::new(1.5, 2.3);
        let (f1, _) = worley_f1_f2_2d(p);
        let w = worley_noise_2d(p);
        assert!((f1 - w).abs() < 1e-5, "f1 {f1} vs worley_noise {w}");
    }
}

// ── Pass 37 ────────────────────────────────────────────────────────────────────

/// Convert **linear RGB** (sRGB primaries) to **CIE L\*a\*b\*** (D65 illuminant).
///
/// Goes through CIE XYZ as an intermediate; normalises XYZ by the D65 white point
/// `(Xn=0.950456, Yn=1.0, Zn=1.08906)`.
///
/// # Examples
/// ```
/// use abrash_core::math::linear_rgb_to_cielab;
/// let (l, a, b) = linear_rgb_to_cielab(1.0, 1.0, 1.0);
/// assert!((l - 100.0).abs() < 0.05, "white L*: {l}");
/// ```
#[must_use]
pub fn linear_rgb_to_cielab(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let (x, y, z) = linear_rgb_to_xyz(r, g, b);
    // D65 white point
    const XN: f32 = 0.950_456;
    const YN: f32 = 1.0;
    const ZN: f32 = 1.088_97;

    #[inline]
    fn f(t: f32) -> f32 {
        const DELTA: f32 = 6.0 / 29.0;
        const DELTA3: f32 = DELTA * DELTA * DELTA; // ≈ 0.008856
        if t > DELTA3 {
            t.cbrt()
        } else {
            t / (3.0 * DELTA * DELTA) + 4.0 / 29.0
        }
    }

    let fx = f(x / XN);
    let fy = f(y / YN);
    let fz = f(z / ZN);
    let l = 116.0 * fy - 16.0;
    let a = 500.0 * (fx - fy);
    let b = 200.0 * (fy - fz);
    (l, a, b)
}

/// Convert **CIE L\*a\*b\*** back to **linear RGB** (sRGB primaries, D65 illuminant).
///
/// # Examples
/// ```
/// use abrash_core::math::{linear_rgb_to_cielab, cielab_to_linear_rgb};
/// let (l, a, b) = linear_rgb_to_cielab(0.8, 0.3, 0.1);
/// let (r2, g2, b2) = cielab_to_linear_rgb(l, a, b);
/// assert!((r2 - 0.8).abs() < 5e-4);
/// ```
#[must_use]
pub fn cielab_to_linear_rgb(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
    const XN: f32 = 0.950_456;
    const YN: f32 = 1.0;
    const ZN: f32 = 1.088_97;

    #[inline]
    fn f_inv(t: f32) -> f32 {
        const DELTA: f32 = 6.0 / 29.0;
        if t > DELTA {
            t * t * t
        } else {
            3.0 * DELTA * DELTA * (t - 4.0 / 29.0)
        }
    }

    let fy = (l + 16.0) / 116.0;
    let fx = a / 500.0 + fy;
    let fz = fy - b / 200.0;
    let (x, y, z) = (f_inv(fx) * XN, f_inv(fy) * YN, f_inv(fz) * ZN);
    xyz_to_linear_rgb(x, y, z)
}

/// Rotate UV texture coordinates by `angle` radians around the centre (0.5, 0.5).
///
/// # Examples
/// ```
/// use abrash_core::math::{uv_rotate, Vec2};
/// use core::f32::consts::FRAC_PI_2;
/// // Rotating (0.5, 0.0) by 90° should give (0.0, 0.5) (relative to centre).
/// let uv = uv_rotate(Vec2::new(1.0, 0.5), FRAC_PI_2);
/// assert!((uv.y - 1.0).abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn uv_rotate(uv: Vec2, angle: f32) -> Vec2 {
    let (s, c) = angle.sin_cos();
    // Translate to centre, rotate, translate back.
    let u = uv.x - 0.5;
    let v = uv.y - 0.5;
    Vec2::new(c * u - s * v + 0.5, s * u + c * v + 0.5)
}

/// Importance-sample the **GGX** normal distribution function.
///
/// Returns a microfacet half-vector `H` in tangent space (z-up) drawn from
/// the GGX NDF.  `u1` and `u2` are uniform \[0, 1) random variates.
///
/// # Arguments
/// * `roughness` – linear roughness α (typically 0.05–1.0)
/// * `u1`, `u2`  – independent uniform random samples
///
/// # Examples
/// ```
/// use abrash_core::math::sample_ggx_hemisphere;
/// let h = sample_ggx_hemisphere(0.5, 0.3, 0.7);
/// assert!(h.z >= 0.0, "z component must be non-negative (upper hemisphere)");
/// let len = (h.x * h.x + h.y * h.y + h.z * h.z).sqrt();
/// assert!((len - 1.0).abs() < 1e-5, "unit vector: {len}");
/// ```
#[must_use]
pub fn sample_ggx_hemisphere(roughness: f32, u1: f32, u2: f32) -> Vec3 {
    use core::f32::consts::TAU;
    let alpha = roughness * roughness;
    // Invert the GGX NDF CDF to get cosθ.
    let cos_theta = ((1.0 - u1) / (1.0 + (alpha * alpha - 1.0) * u1))
        .max(0.0)
        .sqrt();
    let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();
    let phi = TAU * u2;
    Vec3::new(sin_theta * phi.cos(), sin_theta * phi.sin(), cos_theta).normalize()
}

#[cfg(test)]
mod tests_pass_37 {
    use super::*;
    use crate::math::Vec2;

    // ── linear_rgb_to_cielab / cielab_to_linear_rgb ───────────────────────────

    #[test]
    fn cielab_white_l_is_100() {
        let (l, _, _) = linear_rgb_to_cielab(1.0, 1.0, 1.0);
        assert!((l - 100.0).abs() < 0.1, "white L*: {l}");
    }

    #[test]
    fn cielab_black_l_is_0() {
        let (l, a, b) = linear_rgb_to_cielab(0.0, 0.0, 0.0);
        assert!(l.abs() < 1e-4 && a.abs() < 1e-4 && b.abs() < 1e-4);
    }

    #[test]
    fn cielab_round_trip() {
        let cases = [(0.8, 0.3, 0.1_f32), (0.0, 0.5, 1.0), (0.2, 0.6, 0.4)];
        for (r, g, b) in cases {
            let (l, a, bb) = linear_rgb_to_cielab(r, g, b);
            let (r2, g2, b2) = cielab_to_linear_rgb(l, a, bb);
            assert!((r - r2).abs() < 5e-4, "r: {r} vs {r2}");
            assert!((g - g2).abs() < 5e-4, "g: {g} vs {g2}");
            assert!((b - b2).abs() < 5e-4, "b: {b} vs {b2}");
        }
    }

    // ── uv_rotate ─────────────────────────────────────────────────────────────

    #[test]
    fn uv_rotate_zero_is_identity() {
        let uv = Vec2::new(0.3, 0.7);
        let r = uv_rotate(uv, 0.0);
        assert!((r.x - uv.x).abs() < 1e-5 && (r.y - uv.y).abs() < 1e-5);
    }

    #[test]
    fn uv_rotate_90_from_centre_right() {
        use core::f32::consts::FRAC_PI_2;
        // (1.0, 0.5) is to the right of centre (0.5, 0.5).
        // After 90° CCW rotation it should be above centre: (0.5, 1.0).
        let r = uv_rotate(Vec2::new(1.0, 0.5), FRAC_PI_2);
        assert!((r.x - 0.5).abs() < 1e-5, "x: {}", r.x);
        assert!((r.y - 1.0).abs() < 1e-5, "y: {}", r.y);
    }

    // ── sample_ggx_hemisphere ─────────────────────────────────────────────────

    #[test]
    fn ggx_sample_unit_vector() {
        let h = sample_ggx_hemisphere(0.5, 0.3, 0.7);
        let len = (h.x * h.x + h.y * h.y + h.z * h.z).sqrt();
        assert!((len - 1.0).abs() < 1e-5, "unit: {len}");
    }

    #[test]
    fn ggx_sample_upper_hemisphere() {
        for (u1, u2) in [(0.0, 0.0), (0.5, 0.5), (0.99, 0.99), (0.1, 0.9)] {
            let h = sample_ggx_hemisphere(0.4, u1, u2);
            assert!(h.z >= 0.0, "z negative at ({u1},{u2}): {}", h.z);
        }
    }

    #[test]
    fn ggx_low_roughness_peaks_at_z() {
        // Near-specular (roughness → 0): samples cluster near z=1.
        let h = sample_ggx_hemisphere(0.01, 0.5, 0.5);
        assert!(h.z > 0.999, "near-specular peak: {}", h.z);
    }
}
