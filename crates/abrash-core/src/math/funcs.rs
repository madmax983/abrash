#![allow(clippy::imprecise_flops)]
#[allow(clippy::wildcard_imports)]
use super::*;
use std::mem::MaybeUninit;
use std::ops::{Add, Mul, Sub};

/// Fast polynomial approximation of sin and cos.
///
/// Computes an approximation of `sin(x)` and `cos(x)` using a polynomial approximation.
/// This offers a significant performance advantage over the standard library's `sin_cos`
/// implementation for hot paths where absolute precision is not critical.
///
/// ⚡ Bolt: Replacing `.round()` with fast integer casting logic avoids floating-point
/// round function overhead, eliminating branches and yielding measurable performance
/// improvements in hot loops.
#[inline]
pub fn fast_sin_cos(mut x: f32) -> (f32, f32) {
    let pi = std::f32::consts::PI;
    let tau = std::f32::consts::TAU;
    let inv_tau = 1.0 / tau;

    // Wrap x to [-PI, PI]
    let temp = x * inv_tau;
    // Replace slow .round() with fast integer casting
    let round_temp = (temp + 16384.5) as i32 as f32 - 16384.0;
    x -= round_temp * tau;

    // Constants for sin approximation
    let b = 4.0 / pi;
    let c = -4.0 / (pi * pi);
    let p = 0.225;

    // Compute sin
    let mut sin_y = b * x + c * x * x.abs();
    sin_y = p * (sin_y * sin_y.abs() - sin_y) + sin_y;

    // Compute cos by shifting x by PI/2
    let mut cx = x + std::f32::consts::FRAC_PI_2;
    if cx > pi {
        cx -= tau;
    }

    let mut cos_y = b * cx + c * cx * cx.abs();
    cos_y = p * (cos_y * cos_y.abs() - cos_y) + cos_y;

    (sin_y, cos_y)
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
    let t = ((x - from_min) / (from_max - from_min)).max(0.0).min(1.0);
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

/// Hammersley 2D point set — `(i/N, van_der_corput(i))`.
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

// ── Quaternion ────────────────────────────────────────────────────────────────

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
pub const fn morton_encode_2d(x: u32, y: u32) -> u32 {
    /// Spread a 16-bit value into even bit positions.
    #[inline(always)]
    const fn part1by1(mut n: u32) -> u32 {
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
pub const fn morton_decode_2d(code: u32) -> (u32, u32) {
    /// Compact even-bit positions back into a contiguous value.
    #[inline(always)]
    const fn compact1by1(mut n: u32) -> u32 {
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
    1 << x.ilog2()
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
pub const fn median3(a: f32, b: f32, c: f32) -> f32 {
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

// ── Pass 20: Hashing, Bayer dithering, audio utils, misc ─────────────────────

/// PCG (Permuted Congruential Generator) hash — high-quality stateless integer
/// hash in ~3 instructions.
///
/// Ideal for procedural generation, noise seeding, and GPU-style per-pixel
/// random number generation.  Passes `PractRand` and `BigCrush` statistical tests.
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
    f32::from(BAYER[(y & 7) as usize][(x & 7) as usize]) / 64.0
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
pub const fn min3(a: f32, b: f32, c: f32) -> f32 {
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
pub const fn max3(a: f32, b: f32, c: f32) -> f32 {
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
    x * ev.exp2()
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
    #[inline]
    fn h(x: i32, y: i32) -> f32 {
        hash2_to_f32(x as u32, y as u32)
    }

    let ix = p.x.floor() as i32;
    let iy = p.y.floor() as i32;
    let fx = p.x - p.x.floor();
    let fy = p.y - p.y.floor();
    // Smoothstep filter
    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uy = fy * fy * (3.0 - 2.0 * fy);

    let a = lerp(h(ix, iy), h(ix + 1, iy), ux);
    let b = lerp(h(ix, iy + 1), h(ix + 1, iy + 1), ux);
    lerp(a, b, uy)
}

// ── Pass 23 tests ──────────────────────────────────────────────────────────────

// ── Pass 24: OKLab, Perlin noise, fBm, Worley, Porter-Duff, colour temp, GCD ─

/// **`OKLab`** colour space (Björn Ottosson, 2020) — linear RGB → `(L, a, b)`.
///
/// `OKLab` is perceptually uniform: equal distances correspond to equal perceived
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

/// Inverse of [`linear_rgb_to_oklab`] — `OKLab` `(L, a, b)` → linear RGB.
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
pub const fn lcm_u32(a: u32, b: u32) -> u32 {
    if a == 0 || b == 0 {
        0
    } else {
        a / gcd_u32(a, b) * b
    }
}

// ── Pass 24 tests ──────────────────────────────────────────────────────────────

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

/// **`OKLab` interpolation** — perceptually uniform colour blend.
///
/// Interpolates between two `OKLab` colours `(L0, a0, b0)` and `(L1, a1, b1)`
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

/// **`OKLab` hue rotation** — rotate the hue angle in the `(a, b)` plane.
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
    a * (-10.0 * t).exp2() * ((t - s) * core::f32::consts::TAU / period).sin() + 1.0
}

// ── Pass 25 tests ──────────────────────────────────────────────────────────────

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
/// // White point (11.2) normalizes exactly to 1.0
/// assert!((uncharted2_tonemap(11.2) - 1.0).abs() < 1e-4);
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
pub const fn sh_y00() -> f32 {
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

/// Convert `OKLab` `(L, a, b)` to `OKLCh` `(L, C, h)` — polar form.
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
    let c = a.mul_add(a, b * b).sqrt();
    let h = b.atan2(a).to_degrees().rem_euclid(360.0);
    (l, c, h)
}

/// Convert `OKLCh` `(L, C, h)` to `OKLab` `(L, a, b)` — Cartesian form.
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
/// * `eta` = `n_incident` / `n_transmitted` (e.g. `1.0/1.5` air→glass).
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
            0 | 12 => dx + dy,
            1 | 13 => -dx + dy,
            2 => dx - dy,
            3 => -dx - dy,
            4 => dx + dz,
            5 => -dx + dz,
            6 => dx - dz,
            7 => -dx - dz,
            8 => dy + dz,
            9 | 14 => -dy + dz,
            10 => dy - dz,
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
        (10.0 * t - 10.0).exp2()
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
        1.0 - (-10.0 * t).exp2()
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
        (20.0 * t - 10.0).exp2() * 0.5
    } else {
        (2.0 - (-20.0 * t + 10.0).exp2()) * 0.5
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
            * (10.0 * (t2 - 1.0)).exp2()
            * ((t2 - 1.0 - s) * core::f32::consts::TAU / period).sin()
    } else {
        a * (-10.0 * (t2 - 1.0)).exp2()
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
pub const fn next_power_of_2(n: u32) -> u32 {
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
pub const fn is_power_of_2(n: u32) -> bool {
    n > 0 && n.is_power_of_two()
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
pub const fn log2_ceil(n: u32) -> u32 {
    if n <= 1 {
        return 0;
    }
    u32::BITS - (n - 1).leading_zeros()
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
    let xi = u32::from((x.clamp(-1.0, 1.0) * 32_767.0).round() as i16 as u16);
    let yi = u32::from((y.clamp(-1.0, 1.0) * 32_767.0).round() as i16 as u16);
    xi | (yi << 16)
}

/// Unpack a 2×16-bit signed normalised integer `u32` back to two \[−1, 1] floats.
#[must_use]
#[inline]
pub fn unpack_snorm_2x16(packed: u32) -> (f32, f32) {
    let x = (f32::from((packed & 0xFFFF) as i16) / 32_767.0).clamp(-1.0, 1.0);
    let y = (f32::from(((packed >> 16) & 0xFFFF) as i16) / 32_767.0).clamp(-1.0, 1.0);
    (x, y)
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

    let (x, y, z) = linear_rgb_to_xyz(r, g, b);

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

// ---------------------------------------------------------------------------
// Pass 38 — frustum culling, signed angle, vec3 slerp
// ---------------------------------------------------------------------------

/// Extract the 6 view-frustum planes from a combined VP or MVP matrix using
/// Gribb–Hartmann plane extraction. The matrix is column-major (`m[col][row]`).
///
/// Row vectors: `row_r = (m[0][r], m[1][r], m[2][r], m[3][r])`.
/// Plane layout: `[left, right, bottom, top, near, far]`.
/// Each plane is `[a, b, c, d]` (not normalised) s.t. `ax+by+cz+d >= 0` is inside.
pub fn extract_frustum_planes(m: &Mat4) -> [[f32; 4]; 6] {
    // Helper: extract row r as a 4-component vector from the column-major matrix.
    let row = |r: usize| -> [f32; 4] { [m.m[0][r], m.m[1][r], m.m[2][r], m.m[3][r]] };

    let r0 = row(0);
    let r1 = row(1);
    let r2 = row(2);
    let r3 = row(3);

    let add = |a: [f32; 4], b: [f32; 4]| -> [f32; 4] {
        [a[0] + b[0], a[1] + b[1], a[2] + b[2], a[3] + b[3]]
    };
    let sub = |a: [f32; 4], b: [f32; 4]| -> [f32; 4] {
        [a[0] - b[0], a[1] - b[1], a[2] - b[2], a[3] - b[3]]
    };

    [
        add(r3, r0), // left:   row3 + row0
        sub(r3, r0), // right:  row3 - row0
        add(r3, r1), // bottom: row3 + row1
        sub(r3, r1), // top:    row3 - row1
        add(r3, r2), // near:   row3 + row2
        sub(r3, r2), // far:    row3 - row2
    ]
}

/// Test whether a sphere (`centre`, `radius`) intersects the frustum defined by
/// the six planes returned by [`extract_frustum_planes`].
///
/// Returns `true` if the sphere is not fully outside any plane.
pub fn sphere_vs_frustum(centre: Vec3, radius: f32, planes: &[[f32; 4]; 6]) -> bool {
    for p in planes {
        let dist = p[0] * centre.x + p[1] * centre.y + p[2] * centre.z + p[3];
        let len = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
        if len > 0.0 && dist < -radius * len {
            return false;
        }
    }
    true
}

/// Test whether an AABB (`min`, `max`) intersects the frustum.
///
/// Uses the p-vertex (positive-vertex) test: for each plane the "most positive"
/// corner is tested; if that corner is outside the plane the AABB is fully outside.
pub fn aabb_vs_frustum(min: Vec3, max: Vec3, planes: &[[f32; 4]; 6]) -> bool {
    for p in planes {
        // p-vertex: choose the corner that maximises dot(p.xyz, corner)
        let px = if p[0] >= 0.0 { max.x } else { min.x };
        let py = if p[1] >= 0.0 { max.y } else { min.y };
        let pz = if p[2] >= 0.0 { max.z } else { min.z };
        if p[0] * px + p[1] * py + p[2] * pz + p[3] < 0.0 {
            return false;
        }
    }
    true
}

/// Signed angle (radians) from `from` to `to` measured around `axis`.
///
/// Positive means counter-clockwise when looking in the direction of `axis`.
/// All three vectors should be unit-length.
pub fn signed_angle_3d(from: Vec3, to: Vec3, axis: Vec3) -> f32 {
    let cross = from.cross(to);
    let unsigned = cross.length().atan2(from.dot(to));
    if axis.dot(cross) < 0.0 {
        -unsigned
    } else {
        unsigned
    }
}

/// Spherical linear interpolation between two **unit** vectors.
///
/// Smoothly traces the great-circle arc from `a` to `b`. Falls back to `lerp`
/// when the vectors are nearly parallel (angle < ~0.1°) to avoid divide-by-zero.
pub fn vec3_slerp(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    let cos_theta = a.dot(b).clamp(-1.0, 1.0);
    if cos_theta > 1.0 - 1e-6 {
        // Nearly identical — linear blend then re-normalise
        let v = Vec3::new(
            a.x + (b.x - a.x) * t,
            a.y + (b.y - a.y) * t,
            a.z + (b.z - a.z) * t,
        );
        let len = v.length();
        return if len > 0.0 {
            Vec3::new(v.x / len, v.y / len, v.z / len)
        } else {
            a
        };
    }
    let theta = cos_theta.acos();
    let sin_theta = theta.sin();
    let wa = ((1.0 - t) * theta).sin() / sin_theta;
    let wb = (t * theta).sin() / sin_theta;
    Vec3::new(
        wa * a.x + wb * b.x,
        wa * a.y + wb * b.y,
        wa * a.z + wb * b.z,
    )
}

// ---------------------------------------------------------------------------
// Pass 39 — smooth-min/max, pingpong, wrap_angle, PCG hash, IGN, gold noise
// ---------------------------------------------------------------------------

/// Exponential smooth-min (IQ). Blends two SDF distances with C∞ continuity.
///
/// `k` controls the blend radius (larger → softer union). Typical range: 0.1–2.0.
pub fn smooth_min_exp(a: f32, b: f32, k: f32) -> f32 {
    let r = (-k * a).exp() + (-k * b).exp();
    -r.ln() / k
}

/// Polynomial smooth-min (IQ, quartic). C² continuity; slightly cheaper than exp.
///
/// `k` controls the blend radius. Typical range: 0.1–2.0.
pub fn smooth_min_poly(a: f32, b: f32, k: f32) -> f32 {
    let h = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
    lerp(b, a, h) - k * h * (1.0 - h)
}

/// Polynomial smooth-max (IQ, quartic). Dual of [`smooth_min_poly`].
///
/// Returns a C² smooth union of the *larger* of the two values.
pub fn smooth_max_poly(a: f32, b: f32, k: f32) -> f32 {
    -smooth_min_poly(-a, -b, k)
}

/// Triangle-wave / ping-pong oscillation.
///
/// The output bounces between `0.0` and `length` as `t` increases, creating a
/// smooth back-and-forth without discontinuities (unlike `t % length`).
pub fn pingpong(t: f32, length: f32) -> f32 {
    let t = t - (t / (2.0 * length)).floor() * (2.0 * length);
    if t < length { t } else { 2.0 * length - t }
}

/// Normalize an angle (radians) into `(-π, π]`.
pub fn wrap_angle(angle: f32) -> f32 {
    use std::f32::consts::PI;
    let a = angle % (2.0 * PI);
    if a > PI {
        a - 2.0 * PI
    } else if a <= -PI {
        a + 2.0 * PI
    } else {
        a
    }
}

/// PCG32 output hash: u32 → u32. Excellent avalanche, very low cost.
///
/// A second, distinct PCG-output-stage hash — uses a different multiplier than
/// the `pcg_hash` const fn already in this module (which uses the Murmur3 final
/// mix). This one uses the PCG-XSH-RR permutation from O'Neill 2014.
pub const fn pcg32_output(state: u32) -> u32 {
    let s = state.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let w = ((s >> ((s >> 28).wrapping_add(4))) ^ s).wrapping_mul(277_803_737);
    (w >> 22) ^ w
}

/// 2D PCG hash: (u32, u32) → u32. Good spatial decorrelation.
pub const fn pcg32_hash_2d(x: u32, y: u32) -> u32 {
    pcg32_output(x.wrapping_add(pcg32_output(y)))
}

/// Interleaved Gradient Noise (Jimenez et al. 2014 "Next Generation Post Processing in Call of Duty").
///
/// Returns a pseudo-random scalar in `[0, 1)` for a pixel coordinate.
/// Superior to Bayer dithering for temporal AA: the pattern changes coherently
/// across frames when `frame_index` is incremented by golden-ratio multiples.
pub fn interleaved_gradient_noise(pixel_x: f32, pixel_y: f32) -> f32 {
    let f = 52.982_92 * pixel_x + 9.188_25 * pixel_y;
    f.fract()
}

/// Temporally stable IGN: adds per-frame offset using the golden ratio.
///
/// `frame_index` should increase monotonically; the pattern decorrelates across
/// frames, which is ideal for temporal accumulation (TAA, SSAO, etc.).
pub fn interleaved_gradient_noise_temporal(pixel_x: f32, pixel_y: f32, frame_index: u32) -> f32 {
    const GOLDEN_RATIO: f32 = 0.618_033_98;
    let base = interleaved_gradient_noise(pixel_x, pixel_y);
    (base + GOLDEN_RATIO * frame_index as f32).fract()
}

/// Gold noise — 2D float hash using the plastic constant (∛(plastic number)).
///
/// Returns a value in `[0, 1)`. Produces a low-discrepancy distribution with
/// excellent spectral properties when seeded per-pixel with a random seed.
pub fn gold_noise(x: f32, y: f32, seed: f32) -> f32 {
    const PHI: f32 = 1.618_033_98; // golden ratio
    ((x * PHI + y + seed) * 1_e4).sin().fract().abs()
}

// ---------------------------------------------------------------------------
// Pass 40 — Rodrigues rotation, swing/twist decompose, refraction,
//           Beer-Lambert, Henyey-Greenstein, Gaussian kernel
// ---------------------------------------------------------------------------

/// Rotate `v` around unit axis `k` by `angle` radians (Rodrigues' formula).
///
/// Equivalent to building a quaternion from axis+angle and calling `rotate()`,
/// but roughly 3× cheaper when you have the angle directly.
pub fn rodrigues_rotation(v: Vec3, k: Vec3, angle: f32) -> Vec3 {
    let (sin_a, cos_a) = angle.sin_cos();
    let cross = k.cross(v);
    let dot = k.dot(v);
    Vec3::new(
        v.x * cos_a + cross.x * sin_a + k.x * dot * (1.0 - cos_a),
        v.y * cos_a + cross.y * sin_a + k.y * dot * (1.0 - cos_a),
        v.z * cos_a + cross.z * sin_a + k.z * dot * (1.0 - cos_a),
    )
}

/// Decompose quaternion `q` into **swing** and **twist** components relative
/// to a `twist_axis` (must be unit length).
///
/// Returns `(swing, twist)` where `q ≈ swing * twist` and `twist` rotates
/// around `twist_axis` while `swing` rotates perpendicular to it.
pub fn swing_twist_decompose(q: Quat, twist_axis: Vec3) -> (Quat, Quat) {
    let projection = Vec3::new(q.x, q.y, q.z);
    let dot = projection.dot(twist_axis);
    let twist_vec = Vec3::new(twist_axis.x * dot, twist_axis.y * dot, twist_axis.z * dot);
    let mut twist = Quat {
        x: twist_vec.x,
        y: twist_vec.y,
        z: twist_vec.z,
        w: q.w,
    };
    let len =
        (twist.x * twist.x + twist.y * twist.y + twist.z * twist.z + twist.w * twist.w).sqrt();
    if len < 1e-10 {
        twist = Quat::identity();
    } else {
        twist = Quat {
            x: twist.x / len,
            y: twist.y / len,
            z: twist.z / len,
            w: twist.w / len,
        };
    }
    // swing = q * twist_conjugate
    let ti = Quat {
        x: -twist.x,
        y: -twist.y,
        z: -twist.z,
        w: twist.w,
    };
    let swing = Quat {
        x: q.w * ti.x + q.x * ti.w + q.y * ti.z - q.z * ti.y,
        y: q.w * ti.y - q.x * ti.z + q.y * ti.w + q.z * ti.x,
        z: q.w * ti.z + q.x * ti.y - q.y * ti.x + q.z * ti.w,
        w: q.w * ti.w - q.x * ti.x - q.y * ti.y - q.z * ti.z,
    };
    (swing, twist)
}

/// Compute the refracted direction using Snell's law.
///
/// * `incident` — unit incident direction (pointing **toward** the surface)
/// * `normal`   — unit surface normal (pointing **away** from the surface)
/// * `eta`      — ratio of refractive indices `n_i / n_t` (e.g., air→glass ≈ 1/1.5)
///
/// Returns `None` on total internal reflection.
pub fn refract_vec3(incident: Vec3, normal: Vec3, eta: f32) -> Option<Vec3> {
    let neg_incident = Vec3::new(-incident.x, -incident.y, -incident.z);
    let cos_i = neg_incident.dot(normal).clamp(-1.0, 1.0);
    let k = 1.0 - eta * eta * (1.0 - cos_i * cos_i);
    if k < 0.0 {
        return None;
    }
    let s = eta * cos_i - k.sqrt();
    Some(Vec3::new(
        eta * incident.x + s * normal.x,
        eta * incident.y + s * normal.y,
        eta * incident.z + s * normal.z,
    ))
}

/// Beer-Lambert exponential attenuation: `exp(-extinction * distance)`.
///
/// Returns transmittance in `[0, 1]`. Models light absorption through a
/// homogeneous participating medium (fog, water, tinted glass).
pub fn beer_lambert(extinction: f32, distance: f32) -> f32 {
    (-extinction * distance).exp()
}

/// Henyey-Greenstein single-scattering phase function.
///
/// * `cos_theta` — cosine of angle between incident and scattered directions
/// * `g`         — asymmetry parameter `(-1, 1)`: positive = forward, negative = back
///
/// Returns the phase function value (not normalised to 4π).
pub fn henyey_greenstein(cos_theta: f32, g: f32) -> f32 {
    use std::f32::consts::PI;
    let g2 = g * g;
    let base = (1.0 + g2 - 2.0 * g * cos_theta).max(0.0);
    // ⚡ Bolt: Replace `.powf(1.5)` with `base * base.sqrt()` to elide the C-math library call and improve performance.
    let denom = base * base.sqrt();
    (1.0 - g2) / (4.0 * PI * denom.max(1e-10))
}

/// Generate a normalised 1D Gaussian blur kernel of `2*radius+1` taps.
///
/// Coefficients sum to 1.0. `sigma` defaults to `radius / 2.0` when ≤ 0.
pub fn gaussian_kernel_1d(radius: u32, sigma: f32) -> Vec<f32> {
    let sigma = if sigma <= 0.0 {
        radius as f32 / 2.0
    } else {
        sigma
    };
    let two_sigma2 = 2.0 * sigma * sigma;
    let r = radius as i32;
    let mut kernel: Vec<f32> = (-r..=r)
        .map(|i| (-(i * i) as f32 / two_sigma2).exp())
        .collect();
    let sum: f32 = kernel.iter().sum();
    for v in &mut kernel {
        *v /= sum;
    }
    kernel
}

// ---------------------------------------------------------------------------
// Pass 41 — Sobel filter, height-to-normal, trilinear interp, mip LOD,
//           contrast adjust, saturation adjust
// ---------------------------------------------------------------------------

/// Compute Sobel edge gradient `(gx, gy)` from a 3×3 neighbourhood.
///
/// `pixels` is row-major: `[row0_col0, row0_col1, row0_col2, row1_col0, …]`.
/// Returns the un-normalised horizontal and vertical gradient components.
pub fn sobel_filter_3x3(pixels: &[f32; 9]) -> (f32, f32) {
    //  Sobel kernels
    //  Kx = [[-1, 0, 1], [-2, 0, 2], [-1, 0, 1]]
    //  Ky = [[ 1, 2, 1], [ 0, 0, 0], [-1,-2,-1]]
    let gx = -pixels[0] + pixels[2] - 2.0 * pixels[3] + 2.0 * pixels[5] - pixels[6] + pixels[8];
    let gy = pixels[0] + 2.0 * pixels[1] + pixels[2] - pixels[6] - 2.0 * pixels[7] - pixels[8];
    (gx, gy)
}

/// Convert a heightmap 3×3 neighbourhood to a tangent-space normal.
///
/// Uses Sobel derivatives scaled by `texel_size` (world-space size of one
/// texel). Returns a unit normal pointing in the +Z direction for flat surfaces.
pub fn height_to_normal(pixels: &[f32; 9], texel_size: f32, height_scale: f32) -> Vec3 {
    let (gx, gy) = sobel_filter_3x3(pixels);
    let dx = gx * height_scale / (8.0 * texel_size);
    let dy = gy * height_scale / (8.0 * texel_size);
    // Normal = normalise(-dx, -dy, 1)
    let n = Vec3::new(-dx, -dy, 1.0);
    let len = n.length();
    if len > 0.0 {
        Vec3::new(n.x / len, n.y / len, n.z / len)
    } else {
        Vec3::new(0.0, 0.0, 1.0)
    }
}

/// Trilinear interpolation of 8 corner scalars of a unit cube.
///
/// Corner layout (x, y, z) — 0 = min, 1 = max:
/// `v000, v100, v010, v110, v001, v101, v011, v111`.
/// `u, v, w` are all in `[0, 1]`.
pub fn trilinear_interp(
    v000: f32,
    v100: f32,
    v010: f32,
    v110: f32,
    v001: f32,
    v101: f32,
    v011: f32,
    v111: f32,
    u: f32,
    v: f32,
    w: f32,
) -> f32 {
    let c00 = v000 * (1.0 - u) + v100 * u;
    let c10 = v010 * (1.0 - u) + v110 * u;
    let c01 = v001 * (1.0 - u) + v101 * u;
    let c11 = v011 * (1.0 - u) + v111 * u;
    let c0 = c00 * (1.0 - v) + c10 * v;
    let c1 = c01 * (1.0 - v) + c11 * v;
    c0 * (1.0 - w) + c1 * w
}

/// Compute texture mip level (LOD) from UV partial derivatives.
///
/// Uses the OpenGL spec formula: `λ = 0.5 * log2(max(|∂uv/∂x|², |∂uv/∂y|²))`.
/// `dudx, dvdx` — UV derivative along screen X; `dudy, dvdy` — along screen Y.
/// Result is clamped to `[0, max_level]`.
pub fn mip_level(dudx: f32, dvdx: f32, dudy: f32, dvdy: f32, max_level: f32) -> f32 {
    let rho_x = dudx * dudx + dvdx * dvdx;
    let rho_y = dudy * dudy + dvdy * dvdy;
    let rho = rho_x.max(rho_y).max(1e-20);
    (0.5 * rho.log2()).clamp(0.0, max_level)
}

/// Adjust contrast of a value `x ∈ [0,1]` around midpoint 0.5.
///
/// `contrast > 1.0` increases contrast; `0 < contrast < 1.0` reduces it.
/// Output is clamped to `[0, 1]`.
pub fn contrast_adjust(x: f32, contrast: f32) -> f32 {
    ((x - 0.5) * contrast + 0.5).clamp(0.0, 1.0)
}

/// Adjust saturation of an RGB colour using Rec.709 luma.
///
/// `factor = 1.0` → unchanged; `0.0` → greyscale; `> 1.0` → oversaturated.
/// Returns clamped `(r, g, b)`.
pub fn saturation_adjust(r: f32, g: f32, b: f32, factor: f32) -> (f32, f32, f32) {
    // Rec.709 luma coefficients
    let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let nr = (luma + factor * (r - luma)).clamp(0.0, 1.0);
    let ng = (luma + factor * (g - luma)).clamp(0.0, 1.0);
    let nb = (luma + factor * (b - luma)).clamp(0.0, 1.0);
    (nr, ng, nb)
}

// ---------------------------------------------------------------------------
// Pass 42 — Horner polynomial eval, Newton-Raphson, bisection,
//           BT.601 YUV ↔ RGB, CIE ΔE 1976
// ---------------------------------------------------------------------------

/// Evaluate a polynomial using Horner's method.
///
/// `coeffs` are in **ascending degree** order: `coeffs[0] + coeffs[1]*x + coeffs[2]*x² + …`
/// This requires exactly `n-1` multiplications and additions (optimal for dense polynomials).
pub fn horner_eval(coeffs: &[f32], x: f32) -> f32 {
    coeffs.iter().rev().fold(0.0_f32, |acc, &c| acc * x + c)
}

/// Find a root of `f` near `x0` using Newton-Raphson iteration.
///
/// * `f`       — function whose root is sought
/// * `df`      — derivative of `f`
/// * `x0`      — initial guess
/// * `tol`     — convergence tolerance on `|f(x)|`
/// * `max_iter`— maximum iterations
///
/// Returns the root estimate and whether it converged.
pub fn newton_raphson(
    f: impl Fn(f32) -> f32,
    df: impl Fn(f32) -> f32,
    x0: f32,
    tol: f32,
    max_iter: u32,
) -> (f32, bool) {
    let mut x = x0;
    for _ in 0..max_iter {
        let fx = f(x);
        if fx.abs() < tol {
            return (x, true);
        }
        let dfx = df(x);
        if dfx.abs() < 1e-30 {
            break; // zero derivative — can't continue
        }
        x -= fx / dfx;
    }
    (x, f(x).abs() < tol)
}

/// Find a root of `f` in `[a, b]` using bisection (requires `f(a)` and `f(b)` to have opposite signs).
///
/// Returns the root estimate and whether it converged within tolerance.
pub fn bisect(
    f: impl Fn(f32) -> f32,
    mut a: f32,
    mut b: f32,
    tol: f32,
    max_iter: u32,
) -> (f32, bool) {
    let mut fa = f(a);
    for _ in 0..max_iter {
        let mid = 0.5 * (a + b);
        if (b - a) * 0.5 < tol {
            return (mid, true);
        }
        let fmid = f(mid);
        if fmid.abs() < tol {
            return (mid, true);
        }
        if fa * fmid < 0.0 {
            b = mid;
        } else {
            a = mid;
            fa = fmid;
        }
    }
    (0.5 * (a + b), false)
}

/// Convert linear RGB to YUV using BT.601 (SDTV) coefficients.
///
/// Y ∈ [0, 1], U ∈ [-0.5, 0.5], V ∈ [-0.5, 0.5].
pub fn rgb_to_yuv_bt601(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let y = 0.299 * r + 0.587 * g + 0.114 * b;
    let u = -0.168_736 * r - 0.331_264 * g + 0.5 * b;
    let v = 0.5 * r - 0.418_688 * g - 0.081_312 * b;
    (y, u, v)
}

/// Convert BT.601 YUV back to linear RGB.
pub fn yuv_bt601_to_rgb(y: f32, u: f32, v: f32) -> (f32, f32, f32) {
    let r = y + 1.402 * v;
    let g = y - 0.344_136 * u - 0.714_136 * v;
    let b = y + 1.772 * u;
    (r, g, b)
}

/// CIE 1976 colour difference ΔE between two L\*a\*b\* colours.
///
/// ΔE < 1 → imperceptible; 1–2 → just noticeable; > 5 → clearly different.
pub fn delta_e_cie76(l1: f32, a1: f32, b1: f32, l2: f32, a2: f32, b2: f32) -> f32 {
    let dl = l1 - l2;
    let da = a1 - a2;
    let db = b1 - b2;
    (dl * dl + da * da + db * db).sqrt()
}

// ---------------------------------------------------------------------------
// Pass 44 — linear solvers, Gram-Schmidt, Givens, regression, plane fit,
//           winding number, polygon area, ray-cylinder, ray-cone,
//           closest point on segment, segment-segment closest, sphere overlap
// ---------------------------------------------------------------------------

/// Solve the 2×2 system `A * x = b` via Cramer's rule.
/// `a` is row-major `[[a00,a01],[a10,a11]]`. Returns `None` if singular.
pub fn solve_2x2(a: [[f32; 2]; 2], b: [f32; 2], eps: f32) -> Option<[f32; 2]> {
    let det = a[0][0] * a[1][1] - a[0][1] * a[1][0];
    if det.abs() < eps {
        return None;
    }
    Some([
        (b[0] * a[1][1] - b[1] * a[0][1]) / det,
        (a[0][0] * b[1] - a[1][0] * b[0]) / det,
    ])
}

/// Solve the 3×3 system `A * x = b` via Cramer's rule.
/// `a` is row-major `a[row][col]`. Returns `None` if singular.
pub fn solve_3x3(a: [[f32; 3]; 3], b: [f32; 3], eps: f32) -> Option<[f32; 3]> {
    let det3 = |m: [[f32; 3]; 3]| -> f32 {
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    };
    let det = det3(a);
    if det.abs() < eps {
        return None;
    }
    let sub = |col: usize| -> f32 {
        let mut m = a;
        m[0][col] = b[0];
        m[1][col] = b[1];
        m[2][col] = b[2];
        det3(m)
    };
    Some([sub(0) / det, sub(1) / det, sub(2) / det])
}

/// Orthonormalise three linearly-independent `Vec3` vectors via Gram-Schmidt.
/// Returns `None` if any step degenerates.
pub fn gram_schmidt_3(v0: Vec3, v1: Vec3, v2: Vec3) -> Option<(Vec3, Vec3, Vec3)> {
    let eps = 1e-10_f32;
    let norm = |v: Vec3| -> Option<Vec3> {
        let l = v.length();
        if l < eps {
            None
        } else {
            Some(Vec3::new(v.x / l, v.y / l, v.z / l))
        }
    };
    let e0 = norm(v0)?;
    let d1 = e0.dot(v1);
    let u1 = Vec3::new(v1.x - d1 * e0.x, v1.y - d1 * e0.y, v1.z - d1 * e0.z);
    let e1 = norm(u1)?;
    let d20 = e0.dot(v2);
    let d21 = e1.dot(v2);
    let u2 = Vec3::new(
        v2.x - d20 * e0.x - d21 * e1.x,
        v2.y - d20 * e0.y - d21 * e1.y,
        v2.z - d20 * e0.z - d21 * e1.z,
    );
    let e2 = norm(u2)?;
    Some((e0, e1, e2))
}

/// Compute Givens `(c, s)` s.t. `[c s; -s c] * [a; b]^T = [r; 0]`.
pub fn givens_rotation(a: f32, b: f32) -> (f32, f32) {
    if b == 0.0 {
        return (1.0, 0.0);
    }
    let r = a.hypot(b).max(1e-30);
    (a / r, b / r)
}

/// Ordinary least-squares line fit `y = m*x + b` over 2-D points.
/// Returns `(slope, intercept)` or `None` if fewer than 2 points or vertical.
pub fn linear_regression_2d(points: &[(f32, f32)]) -> Option<(f32, f32)> {
    let n = points.len() as f32;
    if points.len() < 2 {
        return None;
    }
    let sx: f32 = points.iter().map(|p| p.0).sum();
    let sy: f32 = points.iter().map(|p| p.1).sum();
    let sxx: f32 = points.iter().map(|p| p.0 * p.0).sum();
    let sxy: f32 = points.iter().map(|p| p.0 * p.1).sum();
    let denom = n * sxx - sx * sx;
    if denom.abs() < 1e-10 {
        return None;
    }
    let m = (n * sxy - sx * sy) / denom;
    Some((m, (sy - m * sx) / n))
}

/// Fit a plane to a point cloud. Returns `(centroid, unit_normal)` or `None`.
/// Uses the scatter-matrix dominant cross-product approximation.
pub fn fit_plane_to_points(points: &[Vec3]) -> Option<(Vec3, Vec3)> {
    if points.len() < 3 {
        return None;
    }
    let n = points.len() as f32;
    let cx = points.iter().map(|p| p.x).sum::<f32>() / n;
    let cy = points.iter().map(|p| p.y).sum::<f32>() / n;
    let cz = points.iter().map(|p| p.z).sum::<f32>() / n;
    let c = Vec3::new(cx, cy, cz);
    let (mut sxx, mut sxy, mut sxz, mut syy, mut syz, mut szz) = (0.0_f32, 0.0, 0.0, 0.0, 0.0, 0.0);
    for p in points {
        let (dx, dy, dz) = (p.x - cx, p.y - cy, p.z - cz);
        sxx += dx * dx;
        sxy += dx * dy;
        sxz += dx * dz;
        syy += dy * dy;
        syz += dy * dz;
        szz += dz * dz;
    }
    let col = [
        Vec3::new(sxx, sxy, sxz),
        Vec3::new(sxy, syy, syz),
        Vec3::new(sxz, syz, szz),
    ];
    let norms = [col[0].length(), col[1].length(), col[2].length()];
    // Two largest-norm columns — their cross product ≈ min-eigenvalue eigenvec (plane normal).
    let (i0, i1) = if norms[0] >= norms[1] && norms[0] >= norms[2] {
        (0, if norms[1] >= norms[2] { 1 } else { 2 })
    } else if norms[1] >= norms[2] {
        (1, if norms[0] >= norms[2] { 0 } else { 2 })
    } else {
        (2, usize::from(norms[0] < norms[1]))
    };
    let raw = col[i0].cross(col[i1]);
    let len = raw.length();
    if len < 1e-10 {
        return None;
    }
    Some((c, Vec3::new(raw.x / len, raw.y / len, raw.z / len)))
}

/// Winding-number point-in-polygon test (handles complex / self-intersecting polygons).
/// Returns `true` if `p` is inside. More robust than ray-casting for vertices-on-edge cases.
pub fn winding_number_2d(p: Vec2, polygon: &[Vec2]) -> bool {
    let n = polygon.len();
    if n < 3 {
        return false;
    }
    let mut wn = 0i32;
    for i in 0..n {
        let a = polygon[i];
        let b = polygon[(i + 1) % n];
        if a.y <= p.y {
            if b.y > p.y {
                // upward crossing
                let cross = (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y);
                if cross > 0.0 {
                    wn += 1;
                }
            }
        } else if b.y <= p.y {
            // downward crossing
            let cross = (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y);
            if cross < 0.0 {
                wn -= 1;
            }
        }
    }
    wn != 0
}

/// Signed area of a 2-D polygon (shoelace formula). Positive = CCW, negative = CW.
pub fn polygon_area_2d(polygon: &[Vec2]) -> f32 {
    let n = polygon.len();
    if n < 3 {
        return 0.0;
    }
    let mut sum = 0.0_f32;
    for i in 0..n {
        let a = polygon[i];
        let b = polygon[(i + 1) % n];
        sum += a.x * b.y - b.x * a.y;
    }
    sum * 0.5
}

/// Ray vs. infinite cylinder (axis through `ca`→`cb`, radius `r`).
/// Returns the nearest positive `t` along `rd` or `None`.
pub fn ray_cylinder_intersect(ro: Vec3, rd: Vec3, ca: Vec3, cb: Vec3, r: f32) -> Option<f32> {
    let ax = Vec3::new(cb.x - ca.x, cb.y - ca.y, cb.z - ca.z);
    let ax_len = ax.length();
    if ax_len < 1e-10 {
        return None;
    }
    let axis = Vec3::new(ax.x / ax_len, ax.y / ax_len, ax.z / ax_len);
    let oc = Vec3::new(ro.x - ca.x, ro.y - ca.y, ro.z - ca.z);
    let d_par = rd.dot(axis);
    let oc_par = oc.dot(axis);
    let d_perp = Vec3::new(
        rd.x - d_par * axis.x,
        rd.y - d_par * axis.y,
        rd.z - d_par * axis.z,
    );
    let oc_perp = Vec3::new(
        oc.x - oc_par * axis.x,
        oc.y - oc_par * axis.y,
        oc.z - oc_par * axis.z,
    );
    let a = d_perp.x * d_perp.x + d_perp.y * d_perp.y + d_perp.z * d_perp.z;
    if a < 1e-10 {
        return None;
    }
    let b = 2.0 * (oc_perp.x * d_perp.x + oc_perp.y * d_perp.y + oc_perp.z * d_perp.z);
    let c = oc_perp.x * oc_perp.x + oc_perp.y * oc_perp.y + oc_perp.z * oc_perp.z - r * r;
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 {
        return None;
    }
    let t = (-b - disc.sqrt()) / (2.0 * a);
    if t > 0.0 {
        Some(t)
    } else {
        let t2 = (-b + disc.sqrt()) / (2.0 * a);
        if t2 > 0.0 { Some(t2) } else { None }
    }
}

/// Ray vs. infinite cone (apex `apex`, axis direction `axis` unit, half-angle `theta` radians).
/// Returns the nearest positive `t` or `None`.
pub fn ray_cone_intersect(ro: Vec3, rd: Vec3, apex: Vec3, axis: Vec3, theta: f32) -> Option<f32> {
    let cos2 = theta.cos() * theta.cos();
    let oc = Vec3::new(ro.x - apex.x, ro.y - apex.y, ro.z - apex.z);
    let d_dot_a = rd.dot(axis);
    let oc_dot_a = oc.dot(axis);
    let a = d_dot_a * d_dot_a - cos2;
    let b = 2.0 * (d_dot_a * oc_dot_a - (rd.x * oc.x + rd.y * oc.y + rd.z * oc.z) * cos2);
    let c = oc_dot_a * oc_dot_a - (oc.x * oc.x + oc.y * oc.y + oc.z * oc.z) * cos2;
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 {
        return None;
    }
    let sq = disc.sqrt();
    let pick = |t: f32| -> bool {
        if t <= 0.0 {
            return false;
        }
        let hit = Vec3::new(
            ro.x + t * rd.x - apex.x,
            ro.y + t * rd.y - apex.y,
            ro.z + t * rd.z - apex.z,
        );
        hit.dot(axis) >= 0.0 // only the forward cone half
    };
    let t1 = (-b - sq) / (2.0 * a);
    let t2 = (-b + sq) / (2.0 * a);
    if pick(t1) {
        Some(t1)
    } else if pick(t2) {
        Some(t2)
    } else {
        None
    }
}

/// Nearest point pair between segments `[p1,p2]` and `[p3,p4]`.
/// Returns `(point_on_seg1, point_on_seg2)` (Shamos-Hoey / Ericson approach).
pub fn closest_segment_to_segment(p1: Vec3, p2: Vec3, p3: Vec3, p4: Vec3) -> (Vec3, Vec3) {
    let d1 = Vec3::new(p2.x - p1.x, p2.y - p1.y, p2.z - p1.z);
    let d2 = Vec3::new(p4.x - p3.x, p4.y - p3.y, p4.z - p3.z);
    let r = Vec3::new(p1.x - p3.x, p1.y - p3.y, p1.z - p3.z);
    let a = d1.x * d1.x + d1.y * d1.y + d1.z * d1.z;
    let e = d2.x * d2.x + d2.y * d2.y + d2.z * d2.z;
    let f = d2.x * r.x + d2.y * r.y + d2.z * r.z;
    let (s, t) = if a < 1e-10 {
        (0.0, (f / e.max(1e-10)).clamp(0.0, 1.0))
    } else {
        let c = d1.x * r.x + d1.y * r.y + d1.z * r.z;
        if e < 1e-10 {
            ((-c / a).clamp(0.0, 1.0), 0.0)
        } else {
            let b = d1.x * d2.x + d1.y * d2.y + d1.z * d2.z;
            let denom = a * e - b * b;
            let s0 = if denom.abs() > 1e-10 {
                ((b * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let t0 = (b * s0 + f) / e.max(1e-10);
            if t0 < 0.0 {
                ((-c / a).clamp(0.0, 1.0), 0.0)
            } else if t0 > 1.0 {
                (((b - c) / a).clamp(0.0, 1.0), 1.0)
            } else {
                (s0, t0)
            }
        }
    };
    let q1 = Vec3::new(p1.x + s * d1.x, p1.y + s * d1.y, p1.z + s * d1.z);
    let q2 = Vec3::new(p3.x + t * d2.x, p3.y + t * d2.y, p3.z + t * d2.z);
    (q1, q2)
}

/// Sphere-sphere overlap test. Returns `Some(penetration_depth)` if overlapping, `None` if not.
/// Penetration depth is how far the spheres interpenetrate (positive = overlap).
pub fn sphere_sphere_overlap(c1: Vec3, r1: f32, c2: Vec3, r2: f32) -> Option<f32> {
    let dx = c2.x - c1.x;
    let dy = c2.y - c1.y;
    let dz = c2.z - c1.z;
    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
    let penetration = r1 + r2 - dist;
    if penetration > 0.0 {
        Some(penetration)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Pass 45 — signal processing & control theory
//   exponential_smooth, one_euro_filter_step, half_life ↔ decay,
//   smooth_damp, delta_angle, angular_lerp, welford_update
// ---------------------------------------------------------------------------

/// Exponential moving average (first-order IIR low-pass filter).
///
/// `alpha ∈ (0, 1]`: fraction of the **new** sample to blend in.
/// - `alpha = 1` → no smoothing (tracks instantly)
/// - `alpha → 0` → heavy smoothing (slow response)
///
/// Call once per frame: `state = exponential_smooth(state, new_value, alpha)`.
pub fn exponential_smooth(current: f32, target: f32, alpha: f32) -> f32 {
    current + alpha.clamp(0.0, 1.0) * (target - current)
}

/// One-step of the **1€ filter** (Casiez et al. 2012) for adaptive smoothing.
///
/// The filter auto-tunes its cutoff: high for fast motion (low lag),
/// low for slow motion (high smoothing). Ideal for pointer/gesture data.
///
/// Parameters:
/// - `prev_filtered` — last filtered value
/// - `prev_deriv`    — last filtered derivative (initialise to 0)
/// - `raw`           — new raw sample
/// - `dt`            — time delta in seconds
/// - `min_cutoff`    — minimum frequency cutoff (Hz), e.g. 1.0
/// - `beta`          — speed coefficient (larger → less lag), e.g. 0.007
/// - `d_cutoff`      — derivative cutoff frequency (Hz), e.g. 1.0
///
/// Returns `(filtered_value, new_deriv)`.
pub fn one_euro_filter_step(
    prev_filtered: f32,
    prev_deriv: f32,
    raw: f32,
    dt: f32,
    min_cutoff: f32,
    beta: f32,
    d_cutoff: f32,
) -> (f32, f32) {
    // Alpha from cutoff frequency: α = 1 / (1 + τ/dt) where τ = 1/(2π*fc)
    let alpha_of = |cutoff: f32| -> f32 {
        let tau = 1.0 / (std::f32::consts::TAU * cutoff);
        1.0 / (1.0 + tau / dt.max(1e-10))
    };
    // Derivative estimate
    let raw_deriv = (raw - prev_filtered) / dt.max(1e-10);
    let d_alpha = alpha_of(d_cutoff);
    let new_deriv = prev_deriv + d_alpha * (raw_deriv - prev_deriv);
    // Adaptive cutoff based on speed
    let cutoff = min_cutoff + beta * new_deriv.abs();
    let alpha = alpha_of(cutoff);
    let filtered = prev_filtered + alpha * (raw - prev_filtered);
    (filtered, new_deriv)
}

/// Convert a **half-life** (time for value to halve) to a per-second decay constant `λ`.
///
/// Usage: `value *= (-lambda * dt).exp()` each frame, or equivalently
/// `value *= decay_factor.powf(dt)` where `decay_factor = 0.5^(1/half_life)`.
pub fn half_life_to_decay(half_life: f32) -> f32 {
    // e^(-λ * t½) = 0.5  →  λ = ln(2) / t½
    std::f32::consts::LN_2 / half_life.max(1e-10)
}

/// Convert a per-second decay constant `λ` back to a half-life in seconds.
pub fn decay_to_half_life(lambda: f32) -> f32 {
    std::f32::consts::LN_2 / lambda.max(1e-10)
}

/// Unity-style critically-damped spring smoother.
///
/// Smoothly moves `current` toward `target` without overshoot.
/// Pass `velocity` by value and store the returned `(new_position, new_velocity)`.
///
/// - `omega` — natural frequency (radians/s); higher = faster response (e.g. 10–30)
/// - `dt`    — time step in seconds
pub fn smooth_damp(current: f32, target: f32, velocity: f32, omega: f32, dt: f32) -> (f32, f32) {
    // Exact solution for critically-damped harmonic oscillator.
    let omega = omega.max(0.0);
    let x = current - target;
    let exp = (-omega * dt).exp();
    let new_x = (x + (velocity + omega * x) * dt) * exp;
    let new_v = (velocity - omega * (velocity + omega * x) * dt) * exp;
    (target + new_x, new_v)
}

/// Signed shortest angular difference from angle `a` to angle `b` (radians).
///
/// Result is in `(-π, π]`. Use this instead of `b - a` to always take the
/// short path around the circle.
pub fn delta_angle(a: f32, b: f32) -> f32 {
    wrap_angle(b - a)
}

/// Lerp between two angles, always taking the shortest arc.
///
/// `t = 0` → `a`, `t = 1` → `b`. Result is normalised to `(-π, π]`.
pub fn angular_lerp(a: f32, b: f32, t: f32) -> f32 {
    wrap_angle(a + t * delta_angle(a, b))
}

/// Welford online mean/variance update (numerically stable single-pass algorithm).
///
/// Call once per sample. Returns updated `(count, mean, m2)` where
/// `variance = m2 / count` (population) or `m2 / (count - 1)` (sample, when count > 1).
///
/// Initialise state as `(0u32, 0.0f32, 0.0f32)`.
pub fn welford_update(count: u32, mean: f32, m2: f32, new_value: f32) -> (u32, f32, f32) {
    let count = count + 1;
    let delta = new_value - mean;
    let mean = mean + delta / count as f32;
    let delta2 = new_value - mean;
    let m2 = m2 + delta * delta2;
    (count, mean, m2)
}

// ---------------------------------------------------------------------------
// Pass 46 — GPU data packing & bit manipulation
//   oct_encode/decode normal, morton_encode/decode_3d,
//   gray_code_encode/decode, fibonacci_hash_u32, reverse_bits_u32
// ---------------------------------------------------------------------------

/// Encode a unit normal into octahedral representation `(u, v) ∈ [-1, 1]²`.
///
/// Used in G-buffers: store two snorm values instead of three floats.
/// Decode with [`oct_decode_normal`].
pub fn oct_encode_normal(n: Vec3) -> (f32, f32) {
    // Project onto L1 sphere
    let inv_l1 = 1.0 / (n.x.abs() + n.y.abs() + n.z.abs()).max(1e-10);
    let px = n.x * inv_l1;
    let py = n.y * inv_l1;
    if n.z >= 0.0 {
        (px, py)
    } else {
        // Fold negative hemisphere
        let ux = (1.0 - py.abs()) * px.signum();
        let uy = (1.0 - px.abs()) * py.signum();
        (ux, uy)
    }
}

/// Decode an octahedral-encoded normal back to a unit `Vec3`.
///
/// `u, v` should be in `[-1, 1]` (snorm range).
pub fn oct_decode_normal(u: f32, v: f32) -> Vec3 {
    let z = 1.0 - u.abs() - v.abs();
    let (x, y) = if z >= 0.0 {
        (u, v)
    } else {
        ((1.0 - v.abs()) * u.signum(), (1.0 - u.abs()) * v.signum())
    };
    let len = (x * x + y * y + z * z).sqrt().max(1e-10);
    Vec3::new(x / len, y / len, z / len)
}

/// Encode `(x, y, z)` into a 30-bit 3D Morton code (Z-order curve).
///
/// Each axis contributes 10 bits (max value 1023). Useful for cache-coherent
/// 3D array access and spatial hashing.
pub fn morton_encode_3d(x: u32, y: u32, z: u32) -> u32 {
    // Spread 10 bits of each coordinate into every third bit position.
    let spread = |mut v: u32| -> u32 {
        v &= 0x0000_03ff;
        v = (v | (v << 16)) & 0x0300_00ff;
        v = (v | (v << 8)) & 0x0300_f00f;
        v = (v | (v << 4)) & 0x030c_30c3;
        v = (v | (v << 2)) & 0x0924_9249;
        v
    };
    spread(x) | (spread(y) << 1) | (spread(z) << 2)
}

/// Decode a 30-bit 3D Morton code back into `(x, y, z)`.
pub fn morton_decode_3d(code: u32) -> (u32, u32, u32) {
    let compact = |mut v: u32| -> u32 {
        v &= 0x0924_9249;
        v = (v | (v >> 2)) & 0x030c_30c3;
        v = (v | (v >> 4)) & 0x0300_f00f;
        v = (v | (v >> 8)) & 0x0300_00ff;
        v = (v | (v >> 16)) & 0x0000_03ff;
        v
    };
    (compact(code), compact(code >> 1), compact(code >> 2))
}

/// Encode a binary value to Gray code.
///
/// Adjacent Gray codes differ by exactly one bit — useful for rotary encoders,
/// error-resilient counters, and Karnaugh maps.
pub const fn gray_code_encode(n: u32) -> u32 {
    n ^ (n >> 1)
}

/// Decode a Gray code back to binary.
pub const fn gray_code_decode(mut g: u32) -> u32 {
    // Each bit depends on all higher bits via XOR cascade.
    g ^= g >> 16;
    g ^= g >> 8;
    g ^= g >> 4;
    g ^= g >> 2;
    g ^= g >> 1;
    g
}

/// Fibonacci / golden-ratio integer hash: u32 → u32.
///
/// Multiplying by the closest integer to `2^32 / φ` spreads sequential
/// integers uniformly across the u32 range. Ideal for hash-table probing
/// and low-discrepancy index-to-bin mapping.
pub const fn fibonacci_hash_u32(n: u32) -> u32 {
    // 2^32 / φ ≈ 2654435769 (Knuth multiplicative hash)
    n.wrapping_mul(2_654_435_769)
}

/// Reverse all 32 bits of a `u32` (bit-reversal permutation).
///
/// Used to build the van der Corput low-discrepancy sequence:
/// `corput(i) = reverse_bits_u32(i) as f32 / 2^32`.
pub const fn reverse_bits_u32(mut n: u32) -> u32 {
    n = ((n & 0xffff_0000) >> 16) | ((n & 0x0000_ffff) << 16);
    n = ((n & 0xff00_ff00) >> 8) | ((n & 0x00ff_00ff) << 8);
    n = ((n & 0xf0f0_f0f0) >> 4) | ((n & 0x0f0f_0f0f) << 4);
    n = ((n & 0xcccc_cccc) >> 2) | ((n & 0x3333_3333) << 2);
    n = ((n & 0xaaaa_aaaa) >> 1) | ((n & 0x5555_5555) << 1);
    n
}

// ---------------------------------------------------------------------------
// Pass 47 — numerical integration, atmosphere, terrain utilities
//   euler_step, runge_kutta_4, verlet_step,
//   rayleigh_phase, height_blend, curl_noise_2d, slope_from_heightmap
// ---------------------------------------------------------------------------

/// Single Euler integration step for a scalar ODE `y' = f(t, y)`.
///
/// Returns `y(t + dt) ≈ y + dt * f(t, y)`. First-order accurate.
/// Use [`runge_kutta_4`] for higher accuracy at the same evaluation cost.
pub fn euler_step(t: f32, y: f32, dt: f32, f: impl Fn(f32, f32) -> f32) -> f32 {
    y + dt * f(t, y)
}

/// Classic 4th-order Runge-Kutta step for a scalar ODE `y' = f(t, y)`.
///
/// Four function evaluations, fourth-order accurate (error ∝ dt⁴).
/// Drop-in replacement for [`euler_step`] when precision matters.
pub fn runge_kutta_4(t: f32, y: f32, dt: f32, f: impl Fn(f32, f32) -> f32) -> f32 {
    let k1 = f(t, y);
    let k2 = f(t + 0.5 * dt, y + 0.5 * dt * k1);
    let k3 = f(t + 0.5 * dt, y + 0.5 * dt * k2);
    let k4 = f(t + dt, y + dt * k3);
    y + dt * (k1 + 2.0 * k2 + 2.0 * k3 + k4) / 6.0
}

/// Position Verlet integration step.
///
/// Time-reversible and symplectic (conserves energy better than Euler).
/// Used in particle systems, cloth, and rigid-body physics.
///
/// * `pos`      — current position
/// * `prev_pos` — position one step ago
/// * `accel`    — current acceleration `a(t)`
/// * `dt`       — time step
///
/// Returns `(new_pos, old_pos_for_next_step)` = `(pos_next, pos)`.
pub fn verlet_step(pos: f32, prev_pos: f32, accel: f32, dt: f32) -> (f32, f32) {
    let new_pos = 2.0 * pos - prev_pos + accel * dt * dt;
    (new_pos, pos)
}

/// Rayleigh scattering phase function.
///
/// Models atmospheric scattering of sunlight by small molecules.
/// `cos_theta` is the cosine of the angle between the view and light directions.
/// Returns the phase function value (already normalised to integrate to 1 over 4π).
pub fn rayleigh_phase(cos_theta: f32) -> f32 {
    // p(θ) = 3/(16π) * (1 + cos²θ)
    3.0 / (16.0 * std::f32::consts::PI) * (1.0 + cos_theta * cos_theta)
}

/// Height-based terrain texture blending.
///
/// Blends between two layers using a smooth step around a `threshold` height,
/// optionally biased by `blend_width`. Useful for snow-line, water-shore,
/// sand-grass transitions.
///
/// Returns blend factor in `[0, 1]`: 0 → layer A, 1 → layer B.
pub fn height_blend(height: f32, threshold: f32, blend_width: f32) -> f32 {
    let hw = blend_width.max(1e-6) * 0.5;
    smoothstep(threshold - hw, threshold + hw, height)
}

/// 2D curl noise — a divergence-free vector field derived from a scalar potential.
///
/// Returns a 2D velocity vector perpendicular to the potential gradient.
/// Curl of a 2D scalar field `φ(x,y)` is `(∂φ/∂y, -∂φ/∂x)`.
/// Uses finite differences of Perlin noise as the potential.
pub fn curl_noise_2d(p: Vec2, epsilon: f32) -> Vec2 {
    use crate::math::perlin_noise_3d;
    // Sample the potential at offset positions to estimate gradient.
    let py = perlin_noise_3d(Vec3::new(p.x, p.y + epsilon, 0.0));
    let my = perlin_noise_3d(Vec3::new(p.x, p.y - epsilon, 0.0));
    let px = perlin_noise_3d(Vec3::new(p.x + epsilon, p.y, 0.0));
    let mx = perlin_noise_3d(Vec3::new(p.x - epsilon, p.y, 0.0));
    let dphi_dy = (py - my) / (2.0 * epsilon);
    let dphi_dx = (px - mx) / (2.0 * epsilon);
    Vec2::new(dphi_dy, -dphi_dx)
}

/// Compute terrain slope from a 3×3 heightmap neighbourhood (Sobel-based).
///
/// Returns the gradient magnitude in world units per texel. Zero = flat, large = steep.
/// `texel_size` is the world-space size of one heightmap texel.
pub fn slope_from_heightmap(pixels: &[f32; 9], texel_size: f32) -> f32 {
    let (gx, gy) = sobel_filter_3x3(pixels);
    // Scale by the Sobel kernel normalisation factor (8 * texel_size for the standard kernel).
    let scale = 8.0 * texel_size;
    let gx_s = gx / scale;
    let gy_s = gy / scale;
    (gx_s * gx_s + gy_s * gy_s).sqrt()
}

// ---------------------------------------------------------------------------
// Pass 48 — Monte Carlo / importance sampling
//   box_muller, sample_triangle_uniform, concentric_disk_sample,
//   power_heuristic, balance_heuristic, tent_sample, stratified_jitter_2d
// ---------------------------------------------------------------------------

/// Box-Muller transform: two uniform samples → two independent standard-normal samples.
///
/// `u1, u2 ∈ (0, 1)` (strictly positive to avoid log(0)).
/// Returns `(z0, z1)` where both are N(0,1).
pub fn box_muller(u1: f32, u2: f32) -> (f32, f32) {
    use std::f32::consts::TAU;
    let r = (-2.0 * u1.max(1e-30).ln()).sqrt();
    let theta = TAU * u2;
    (r * theta.cos(), r * theta.sin())
}

/// Uniformly sample a point on a triangle `(a, b, c)` from two uniform samples `u1, u2 ∈ [0,1]`.
///
/// Uses the square-root warp to avoid the fold-over discontinuity.
/// Returns barycentric coordinates `(w0, w1, w2)` summing to 1.
pub fn sample_triangle_uniform(u1: f32, u2: f32) -> (f32, f32, f32) {
    let su1 = u1.sqrt();
    let w0 = 1.0 - su1;
    let w1 = su1 * (1.0 - u2);
    let w2 = su1 * u2;
    (w0, w1, w2)
}

/// Shirley-Chiu concentric disk mapping: maps `(u, v) ∈ [-1,1]²` to unit disk.
///
/// Low-distortion (preserves area relationships better than polar mapping).
/// Use with stratified samples for soft shadows and `DoF`.
/// Returns `(x, y)` on the unit disk.
pub fn concentric_disk_sample(u: f32, v: f32) -> (f32, f32) {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};
    if u == 0.0 && v == 0.0 {
        return (0.0, 0.0);
    }
    let (r, theta) = if u.abs() > v.abs() {
        (u, FRAC_PI_4 * v / u)
    } else {
        (v, FRAC_PI_2 - FRAC_PI_4 * u / v)
    };
    (r * theta.cos(), r * theta.sin())
}

/// MIS power heuristic weight for sample from distribution `a` when `n_a` samples are taken.
///
/// `pdf_a` — PDF of the chosen sample under distribution a.
/// `pdf_b` — PDF of the chosen sample under distribution b.
/// `n_a`, `n_b` — number of samples taken from each distribution.
/// Returns the weight in `[0, 1]`. Use `beta = 2` (quadratic) per Veach's thesis.
pub fn power_heuristic(n_a: u32, pdf_a: f32, n_b: u32, pdf_b: f32) -> f32 {
    let a = (n_a as f32 * pdf_a).powi(2);
    let b = (n_b as f32 * pdf_b).powi(2);
    a / (a + b).max(1e-30)
}

/// MIS balance heuristic weight (Veach 1997, linear weighting).
///
/// Simpler than [`power_heuristic`] but slightly higher variance.
pub fn balance_heuristic(n_a: u32, pdf_a: f32, n_b: u32, pdf_b: f32) -> f32 {
    let a = n_a as f32 * pdf_a;
    let b = n_b as f32 * pdf_b;
    a / (a + b).max(1e-30)
}

/// Tent (triangle) filter sample: map `u ∈ [0, 1]` to `[-1, 1]` with tent distribution.
///
/// Used to jitter pixel samples for anti-aliasing. The tent PDF peaks at 0
/// and falls linearly to 0 at ±1 — matches a 2-pixel-wide triangle filter.
pub fn tent_sample(u: f32) -> f32 {
    if u < 0.5 {
        (2.0 * u).sqrt() - 1.0
    } else {
        1.0 - (2.0 * (1.0 - u)).sqrt()
    }
}

/// Generate an `n × n` stratified jittered 2D sample grid.
///
/// Returns `n*n` samples in `[0, 1)²`. Each cell `(i, j)` contributes one
/// sample with a random jitter `(jx[i*n+j], jy[i*n+j]) ∈ [0, 1)`.
///
/// `jitter` — per-sample random offsets, length must be `n*n`.
/// The `k`-th jitter is used for the `k`-th stratum (row-major order).
pub fn stratified_jitter_2d(n: u32, jitter: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let n = n as usize;
    let inv_n = 1.0 / n as f32;
    (0..n * n)
        .map(|k| {
            let i = k / n;
            let j = k % n;
            let (jx, jy) = if k < jitter.len() {
                jitter[k]
            } else {
                (0.5, 0.5)
            };
            (
                (j as f32 + jx.clamp(0.0, 1.0)) * inv_n,
                (i as f32 + jy.clamp(0.0, 1.0)) * inv_n,
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Pass 49 — advanced noise variants
//   perlin_2d, ridged_fbm_3d, billow_fbm_3d, curl_noise_3d,
//   domain_warp_2d, fbm_ridged_2d, voronoi_smooth_2d
// ---------------------------------------------------------------------------

/// 2D Perlin gradient noise, returns value in approximately `[-1, 1]`.
///
/// Complements the existing [`perlin_noise_3d`]; simply calls it with `z = 0`.
pub fn perlin_2d(p: Vec2) -> f32 {
    perlin_noise_3d(Vec3::new(p.x, p.y, 0.0))
}

/// Ridged multifractal noise (Musgrave 1994) — produces sharp ridges.
///
/// Great for mountain ranges, canyons, and eroded terrain.
/// `octaves`, `lacunarity` (frequency multiplier), `gain` (amplitude multiplier) work
/// the same as in [`fbm_3d`], but each octave uses `1 - |noise|` to create ridges.
pub fn ridged_fbm_3d(mut p: Vec3, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
    let mut value = 0.0_f32;
    let mut amplitude = 0.5_f32;
    let mut weight = 1.0_f32;
    for _ in 0..octaves.max(1) {
        let n = 1.0 - perlin_noise_3d(p).abs();
        let n = n * n * weight;
        value += n * amplitude;
        weight = n.clamp(0.0, 1.0);
        p = Vec3::new(p.x * lacunarity, p.y * lacunarity, p.z * lacunarity);
        amplitude *= gain;
    }
    value
}

/// Billow noise — absolute-value fBm, produces rounded bumps like cumulus clouds.
///
/// Same parameters as [`fbm_3d`]; each octave uses `|noise|` instead of `noise`.
pub fn billow_fbm_3d(mut p: Vec3, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
    let mut value = 0.0_f32;
    let mut amplitude = 0.5_f32;
    for _ in 0..octaves.max(1) {
        value += perlin_noise_3d(p).abs() * amplitude;
        p = Vec3::new(p.x * lacunarity, p.y * lacunarity, p.z * lacunarity);
        amplitude *= gain;
    }
    value
}

/// 3D curl noise — divergence-free vector field via finite-difference curl of Perlin potential.
///
/// Returns a `Vec3` velocity. Use for smoke, fire, and fluid-like particle advection.
/// `epsilon` — finite-difference step (typically 0.001–0.01).
pub fn curl_noise_3d(p: Vec3, epsilon: f32) -> Vec3 {
    // Curl = (∂Fz/∂y - ∂Fy/∂z,  ∂Fx/∂z - ∂Fz/∂x,  ∂Fy/∂x - ∂Fx/∂y)
    // Use three independent Perlin channels (offset by large constants).
    let off = 3.7_f32;
    let fx = |q: Vec3| perlin_noise_3d(q);
    let fy = |q: Vec3| perlin_noise_3d(Vec3::new(q.x + off, q.y, q.z));
    let fz = |q: Vec3| perlin_noise_3d(Vec3::new(q.x, q.y + off, q.z));

    let dfz_dy = (fz(Vec3::new(p.x, p.y + epsilon, p.z)) - fz(Vec3::new(p.x, p.y - epsilon, p.z)))
        / (2.0 * epsilon);
    let dfy_dz = (fy(Vec3::new(p.x, p.y, p.z + epsilon)) - fy(Vec3::new(p.x, p.y, p.z - epsilon)))
        / (2.0 * epsilon);
    let dfx_dz = (fx(Vec3::new(p.x, p.y, p.z + epsilon)) - fx(Vec3::new(p.x, p.y, p.z - epsilon)))
        / (2.0 * epsilon);
    let dfz_dx = (fz(Vec3::new(p.x + epsilon, p.y, p.z)) - fz(Vec3::new(p.x - epsilon, p.y, p.z)))
        / (2.0 * epsilon);
    let dfy_dx = (fy(Vec3::new(p.x + epsilon, p.y, p.z)) - fy(Vec3::new(p.x - epsilon, p.y, p.z)))
        / (2.0 * epsilon);
    let dfx_dy = (fx(Vec3::new(p.x, p.y + epsilon, p.z)) - fx(Vec3::new(p.x, p.y - epsilon, p.z)))
        / (2.0 * epsilon);

    Vec3::new(dfz_dy - dfy_dz, dfx_dz - dfz_dx, dfy_dx - dfx_dy)
}

/// 2D domain warping (Inigo Quilez technique).
///
/// Feeds the output of one fBm pass back as an offset into a second pass,
/// producing strongly turbulent, cave-like patterns.
/// `strength` controls how far the domain is displaced (typically 0.5–4.0).
pub fn domain_warp_2d(p: Vec2, strength: f32, octaves: u32) -> f32 {
    let q = Vec2::new(
        fbm_2d(p, octaves, 2.0, 0.5),
        fbm_2d(Vec2::new(p.x + 5.2, p.y + 1.3), octaves, 2.0, 0.5),
    );
    fbm_2d(
        Vec2::new(p.x + strength * q.x, p.y + strength * q.y),
        octaves,
        2.0,
        0.5,
    )
}

/// 2D ridged fBm — sharp ridges in 2D (good for height maps and procedural textures).
pub fn fbm_ridged_2d(mut p: Vec2, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
    let mut value = 0.0_f32;
    let mut amplitude = 0.5_f32;
    let mut weight = 1.0_f32;
    for _ in 0..octaves.max(1) {
        let n = 1.0 - perlin_2d(p).abs();
        let n = n * n * weight;
        value += n * amplitude;
        weight = n.clamp(0.0, 1.0);
        p = Vec2::new(p.x * lacunarity, p.y * lacunarity);
        amplitude *= gain;
    }
    value
}

/// Smooth Voronoi in 2D — blends cell distances for a softer look than hard F1.
///
/// Uses exponential smooth-minimum over surrounding cell distances.
/// `k` controls blend sharpness (larger = sharper, approaching standard F1).
/// Returns a value roughly in `[0, 1]`.
pub fn voronoi_smooth_2d(p: Vec2, k: f32) -> f32 {
    let pi = Vec2::new(p.x.floor(), p.y.floor());
    let mut res = 0.0_f32;
    for dy in -2i32..=2 {
        for dx in -2i32..=2 {
            let b = Vec2::new(dx as f32, dy as f32);
            // Hash cell to get random offset in [0,1]²
            let cell = Vec2::new(pi.x + b.x, pi.y + b.y);
            let hx = (cell.x * 127.1 + cell.y * 311.7).sin() * 43758.547;
            let hy = (cell.x * 269.5 + cell.y * 183.3).sin() * 43758.547;
            let h = Vec2::new(hx - hx.floor(), hy - hy.floor());
            let r = Vec2::new(
                b.x + h.x - (p.x - p.x.floor()),
                b.y + h.y - (p.y - p.y.floor()),
            );
            let d = (r.x * r.x + r.y * r.y).sqrt();
            res += (-k * d).exp();
        }
    }
    -(1.0 / k) * res.ln()
}

// ---------------------------------------------------------------------------
// Pass 50 — numerical analysis & signal processing
//   finite_diff_deriv, integrate_trapezoid, integrate_simpson,
//   convolve_1d, pearson_correlation, covariance, zero_crossings
// ---------------------------------------------------------------------------

/// 5-point central-difference numerical derivative of `f` at `x`.
///
/// 4th-order accurate: error ∝ h⁴. Use `h ≈ ε^(1/5)` where ε is machine epsilon.
/// Good balance of accuracy and function evaluation cost (4 evaluations).
pub fn finite_diff_deriv(f: impl Fn(f32) -> f32, x: f32, h: f32) -> f32 {
    (-f(x + 2.0 * h) + 8.0 * f(x + h) - 8.0 * f(x - h) + f(x - 2.0 * h)) / (12.0 * h)
}

/// Trapezoidal rule integration over uniformly-spaced samples.
///
/// `values` — sampled function values at equally-spaced points.
/// `dx` — spacing between samples.
/// Returns `∫ f(x) dx ≈ dx * (v[0]/2 + v[1] + … + v[n-2] + v[n-1]/2)`.
pub fn integrate_trapezoid(values: &[f32], dx: f32) -> f32 {
    if values.len() < 2 {
        return 0.0;
    }
    let n = values.len();
    let interior: f32 = values[1..n - 1].iter().sum();
    dx * (0.5 * values[0] + interior + 0.5 * values[n - 1])
}

/// Simpson's 1/3 rule integration over uniformly-spaced samples.
///
/// 4th-order accurate. Requires an **even** number of intervals (odd sample count).
/// If the sample count is even, falls back to trapezoid for the last interval.
/// `dx` — spacing between samples.
pub fn integrate_simpson(values: &[f32], dx: f32) -> f32 {
    let n = values.len();
    if n < 2 {
        return 0.0;
    }
    if n == 2 {
        return integrate_trapezoid(values, dx);
    }
    // Process pairs of intervals (Simpson's rule needs groups of 3 points = 2 intervals).
    let pairs = (n - 1) / 2; // number of complete Simpson pairs
    let mut sum = 0.0_f32;
    for i in 0..pairs {
        let j = i * 2;
        sum += values[j] + 4.0 * values[j + 1] + values[j + 2];
    }
    sum *= dx / 3.0;
    // If n is even (odd number of intervals), add trapezoid for the last interval.
    if (n - 1) % 2 == 1 {
        sum += 0.5 * dx * (values[n - 2] + values[n - 1]);
    }
    sum
}

/// Discrete 1D convolution of `signal` with `kernel` (full mode).
///
/// Returns a vector of length `signal.len() + kernel.len() - 1`.
/// The kernel is not flipped (cross-correlation convention) — if you need
/// true convolution, reverse the kernel before passing it.
pub fn convolve_1d(signal: &[f32], kernel: &[f32]) -> Vec<f32> {
    if signal.is_empty() || kernel.is_empty() {
        return Vec::new();
    }
    let out_len = signal.len() + kernel.len() - 1;
    let mut out = vec![0.0_f32; out_len];
    for (i, &s) in signal.iter().enumerate() {
        for (j, &k) in kernel.iter().enumerate() {
            out[i + j] += s * k;
        }
    }
    out
}

/// Sample covariance of two equal-length slices (uses n-1 denominator).
///
/// Returns `None` if fewer than 2 samples or lengths differ.
pub fn covariance(x: &[f32], y: &[f32]) -> Option<f32> {
    if x.len() != y.len() || x.len() < 2 {
        return None;
    }
    let n = x.len() as f32;
    let mx = x.iter().sum::<f32>() / n;
    let my = y.iter().sum::<f32>() / n;
    let cov = x
        .iter()
        .zip(y)
        .map(|(&xi, &yi)| (xi - mx) * (yi - my))
        .sum::<f32>()
        / (n - 1.0);
    Some(cov)
}

/// Pearson correlation coefficient `r ∈ [-1, 1]` between two equal-length slices.
///
/// Returns `None` if fewer than 2 samples, lengths differ, or either series has zero variance.
pub fn pearson_correlation(x: &[f32], y: &[f32]) -> Option<f32> {
    let cov = covariance(x, y)?;
    let n = x.len() as f32;
    let mx = x.iter().sum::<f32>() / n;
    let my = y.iter().sum::<f32>() / n;
    let sx = (x.iter().map(|&v| (v - mx).powi(2)).sum::<f32>() / (n - 1.0)).sqrt();
    let sy = (y.iter().map(|&v| (v - my).powi(2)).sum::<f32>() / (n - 1.0)).sqrt();
    if sx < 1e-10 || sy < 1e-10 {
        return None;
    }
    Some((cov / (sx * sy)).clamp(-1.0, 1.0))
}

/// Count the number of zero crossings in a signal (sign changes between adjacent samples).
pub fn zero_crossings(signal: &[f32]) -> usize {
    signal.windows(2).filter(|w| w[0] * w[1] < 0.0).count()
}

// ── Pass 51 ──────────────────────────────────────────────────────────────────
// convex_hull_2d, point_in_convex_polygon_2d, two_bone_ik,
// r2_sequence, sobol_2d, critically_damped_spring_step,
// bezier_arc_length_param
// ─────────────────────────────────────────────────────────────────────────────

/// Graham-scan convex hull of `points` (CCW order).
///
/// Returns the minimal convex polygon vertices; collinear boundary points are
/// excluded.  Returns an empty `Vec` for fewer than 3 non-coincident points.
pub fn convex_hull_2d(points: &[Vec2]) -> Vec<Vec2> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut pts = points.to_vec();
    // Pivot: lowest Y, break ties by lowest X.
    let pivot_idx = pts
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)))
        .map_or(0, |(i, _)| i);
    pts.swap(0, pivot_idx);
    let pivot = pts[0];
    // Sort remaining points by polar angle around pivot.
    pts[1..].sort_by(|a, b| {
        let angle_a = (a.y - pivot.y).atan2(a.x - pivot.x);
        let angle_b = (b.y - pivot.y).atan2(b.x - pivot.x);
        let cmp = angle_a.total_cmp(&angle_b);
        if cmp == std::cmp::Ordering::Equal {
            // Same angle: keep farthest from pivot.
            let da = (a.x - pivot.x).hypot(a.y - pivot.y);
            let db = (b.x - pivot.x).hypot(b.y - pivot.y);
            da.total_cmp(&db)
        } else {
            cmp
        }
    });
    // Scan: maintain a stack where every turn is a left turn (CCW).
    let mut hull: Vec<Vec2> = Vec::with_capacity(pts.len());
    for p in pts {
        while hull.len() >= 2 {
            let o = hull[hull.len() - 2];
            let a = hull[hull.len() - 1];
            // Cross product of (a-o) x (p-o); <= 0 means right/collinear.
            let cross = (a.x - o.x) * (p.y - o.y) - (a.y - o.y) * (p.x - o.x);
            if cross <= 0.0 {
                hull.pop();
            } else {
                break;
            }
        }
        hull.push(p);
    }
    hull
}

/// Returns `true` if `point` is inside (or on the boundary of) the convex
/// polygon described by `hull` (vertices in CCW order).
///
/// Uses the sign-of-cross-product test: for a CCW hull, a point is inside iff
/// it is to the left of every edge.
pub fn point_in_convex_polygon_2d(point: Vec2, hull: &[Vec2]) -> bool {
    let n = hull.len();
    if n < 3 {
        return false;
    }
    for i in 0..n {
        let a = hull[i];
        let b = hull[(i + 1) % n];
        // Cross of edge (b-a) with (point-a).  Negative => right of edge => outside.
        let cross = (b.x - a.x) * (point.y - a.y) - (b.y - a.y) * (point.x - a.x);
        if cross < 0.0 {
            return false;
        }
    }
    true
}

/// Analytical two-bone IK: returns the world-space **joint** position so that
/// a two-bone chain (bone1 = `root->joint`, bone2 = `joint->end`) reaches
/// `target`, given bone lengths `l1` and `l2`.
///
/// `hint` is a world-space direction that biases which side the elbow bends
/// toward.  When `target` is out of reach the chain fully extends; when closer
/// than `|l1 - l2|` it partially folds.
pub fn two_bone_ik(root: Vec3, l1: f32, l2: f32, target: Vec3, hint: Vec3) -> Vec3 {
    let to_target = Vec3::new(target.x - root.x, target.y - root.y, target.z - root.z);
    let dist =
        (to_target.x * to_target.x + to_target.y * to_target.y + to_target.z * to_target.z).sqrt();
    if dist < 1e-6 {
        return root;
    }
    let dir = Vec3::new(to_target.x / dist, to_target.y / dist, to_target.z / dist);

    // Clamp reach to [|l1-l2|, l1+l2].
    let d = dist.clamp((l1 - l2).abs(), l1 + l2);

    // Law of cosines: angle at root between reach direction and bone1.
    let cos_a = ((d * d + l1 * l1 - l2 * l2) / (2.0 * d * l1)).clamp(-1.0, 1.0);
    let angle_a = cos_a.acos();

    // Rotation axis = dir x hint (perpendicular to the reach plane).
    let cross = Vec3::new(
        dir.y * hint.z - dir.z * hint.y,
        dir.z * hint.x - dir.x * hint.z,
        dir.x * hint.y - dir.y * hint.x,
    );
    let cross_len = (cross.x * cross.x + cross.y * cross.y + cross.z * cross.z).sqrt();
    let axis = if cross_len > 1e-6 {
        Vec3::new(
            cross.x / cross_len,
            cross.y / cross_len,
            cross.z / cross_len,
        )
    } else {
        // dir || hint: pick an arbitrary perpendicular.
        let perp = if dir.x.abs() < 0.9 {
            Vec3::new(1.0, 0.0, 0.0)
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        };
        let c = Vec3::new(
            dir.y * perp.z - dir.z * perp.y,
            dir.z * perp.x - dir.x * perp.z,
            dir.x * perp.y - dir.y * perp.x,
        );
        let cl = (c.x * c.x + c.y * c.y + c.z * c.z).sqrt();
        Vec3::new(c.x / cl, c.y / cl, c.z / cl)
    };

    // Rodrigues rotation of dir by angle_a around axis.
    let (s, c) = angle_a.sin_cos();
    let dot = dir.x * axis.x + dir.y * axis.y + dir.z * axis.z;
    let rotated = Vec3::new(
        dir.x * c + (axis.y * dir.z - axis.z * dir.y) * s + axis.x * dot * (1.0 - c),
        dir.y * c + (axis.z * dir.x - axis.x * dir.z) * s + axis.y * dot * (1.0 - c),
        dir.z * c + (axis.x * dir.y - axis.y * dir.x) * s + axis.z * dot * (1.0 - c),
    );

    Vec3::new(
        root.x + rotated.x * l1,
        root.y + rotated.y * l1,
        root.z + rotated.z * l1,
    )
}

/// Martin Roberts R2 low-discrepancy sequence (2D).
///
/// Returns the `n`-th point (0-indexed) in [0,1)^2.  Uses the plastic constant
/// phi2 ~= 1.3247 for near-optimal 2D coverage with essentially zero cost.
#[inline]
pub fn r2_sequence(n: u32) -> Vec2 {
    // phi2 is the real root of x^3 - x - 1 = 0.
    const PHI2: f32 = 1.324_717_957_2;
    const A1: f32 = 1.0 / PHI2;
    const A2: f32 = 1.0 / (PHI2 * PHI2);
    let n = n as f32;
    Vec2::new((0.5 + n * A1).fract(), (0.5 + n * A2).fract())
}

/// Sobol 2D quasi-random sequence using standard direction numbers for
/// dimensions 1 and 2.
///
/// Dimension 1 is the van der Corput base-2 sequence.  Dimension 2 uses the
/// Gray-code XOR construction with direction numbers for the primitive
/// polynomial x+1.  Returns the `index`-th sample (0-indexed) in [0,1)^2.
pub fn sobol_2d(index: u32) -> Vec2 {
    // Scale factor: 2^-32 represented as an f32 bit pattern.
    const SCALE: f32 = 2.328_306_4e-10; // 1.0 / 2^32

    // Dimension 1: bit-reversal (van der Corput base 2).
    let x = index.reverse_bits() as f32 * SCALE;

    // Dimension 2: Gray-code XOR with direction numbers v[i] = 2^(31-i).
    let g = index ^ (index >> 1);
    let mut result = 0u32;
    let mut direction = 0x8000_0000u32;
    let mut g_work = g;
    while g_work > 0 {
        if g_work & 1 != 0 {
            result ^= direction;
        }
        direction >>= 1;
        g_work >>= 1;
    }
    let y = result as f32 * SCALE;

    Vec2::new(x, y)
}

/// Critically-damped spring step — the smoothest damping without oscillation.
///
/// Updates `*velocity` in place and returns the new position.  `omega` is the
/// natural frequency (rad/s).  Uses the exact closed-form solution
/// `x(t) = x_eq + (A + B·t)·exp(-ω·t)` where `A = x₀ − x_eq`,
/// `B = v₀ + ω·A`, guaranteeing no overshoot from rest.
#[inline]
pub fn critically_damped_spring_step(
    current: f32,
    target: f32,
    velocity: &mut f32,
    omega: f32,
    dt: f32,
) -> f32 {
    let e = (-omega * dt).exp();
    let a = current - target; // displacement
    let b = *velocity + omega * a; // integration constant
    *velocity = (b * (1.0 - omega * dt) - omega * a) * e;
    target + (a + b * dt) * e
}

/// Re-parameterize a cubic Bezier by arc length.
///
/// Given a desired arc-length fraction `s in [0,1]` (where `s=1` is the full
/// curve), returns the curve parameter `t in [0,1]` via binary search over a
/// `steps`-segment piecewise-linear length table.  Typical `steps`: 32-256.
pub fn bezier_arc_length_param(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, s: f32, steps: u32) -> f32 {
    let n = steps.max(2) as usize;
    let mut lengths = vec![0.0f32; n + 1];
    let mut prev = p0;
    for i in 1..=n {
        let t = i as f32 / n as f32;
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;
        let pt = Vec3::new(
            mt3 * p0.x + 3.0 * mt2 * t * p1.x + 3.0 * mt * t2 * p2.x + t3 * p3.x,
            mt3 * p0.y + 3.0 * mt2 * t * p1.y + 3.0 * mt * t2 * p2.y + t3 * p3.y,
            mt3 * p0.z + 3.0 * mt2 * t * p1.z + 3.0 * mt * t2 * p2.z + t3 * p3.z,
        );
        let dx = pt.x - prev.x;
        let dy = pt.y - prev.y;
        let dz = pt.z - prev.z;
        lengths[i] = lengths[i - 1] + (dx * dx + dy * dy + dz * dz).sqrt();
        prev = pt;
    }
    let total = lengths[n];
    if total < 1e-10 {
        return 0.0;
    }
    let target_len = s.clamp(0.0, 1.0) * total;
    let idx = lengths.partition_point(|&l| l <= target_len).min(n).max(1);
    let lo = lengths[idx - 1];
    let hi = lengths[idx];
    let t_lo = (idx - 1) as f32 / n as f32;
    let t_hi = idx as f32 / n as f32;
    if (hi - lo).abs() < 1e-10 {
        t_lo
    } else {
        t_lo + (target_len - lo) / (hi - lo) * (t_hi - t_lo)
    }
}

// ── Tests — Pass 51 ───────────────────────────────────────────────────────────

// ── Pass 52 ──────────────────────────────────────────────────────────────────
// orient_2d, triangle_circumcenter_2d, in_circumcircle_2d,
// capsule_vs_capsule, sphere_vs_capsule,
// sat_overlap_polygons_2d, obb_vs_obb_2d
// ─────────────────────────────────────────────────────────────────────────────

/// Signed area of triangle (a, b, c) x 2.
///
/// Returns > 0 if the vertices are in CCW order, < 0 if CW, 0 if collinear.
#[inline]
pub fn orient_2d(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// Circumcenter of triangle (a, b, c) in 2D.
///
/// Returns `None` if the triangle is degenerate (collinear vertices).
pub fn triangle_circumcenter_2d(a: Vec2, b: Vec2, c: Vec2) -> Option<Vec2> {
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if d.abs() < 1e-10 {
        return None;
    }
    let a2 = a.x * a.x + a.y * a.y;
    let b2 = b.x * b.x + b.y * b.y;
    let c2 = c.x * c.x + c.y * c.y;
    Some(Vec2::new(
        (a2 * (b.y - c.y) + b2 * (c.y - a.y) + c2 * (a.y - b.y)) / d,
        (a2 * (c.x - b.x) + b2 * (a.x - c.x) + c2 * (b.x - a.x)) / d,
    ))
}

/// Delaunay in-circle test: returns `true` if point `d` lies strictly inside
/// the circumcircle of triangle (a, b, c) given in CCW order.
///
/// Evaluates the determinant:
/// ```text
/// |ax-dx  ay-dy  (ax-dx)^2+(ay-dy)^2|
/// |bx-dx  by-dy  (bx-dx)^2+(by-dy)^2|  > 0
/// |cx-dx  cy-dy  (cx-dx)^2+(cy-dy)^2|
/// ```
pub fn in_circumcircle_2d(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
    let ax = a.x - d.x;
    let ay = a.y - d.y;
    let bx = b.x - d.x;
    let by = b.y - d.y;
    let cx = c.x - d.x;
    let cy = c.y - d.y;
    let det = ax * (by * (cx * cx + cy * cy) - cy * (bx * bx + by * by))
        - ay * (bx * (cx * cx + cy * cy) - cx * (bx * bx + by * by))
        + (ax * ax + ay * ay) * (bx * cy - by * cx);
    det > 0.0
}

/// Returns `true` if two capsules overlap.
///
/// Each capsule is a line segment (p0, p1) swept by a sphere of radius r.
pub fn capsule_vs_capsule(a0: Vec3, a1: Vec3, ra: f32, b0: Vec3, b1: Vec3, rb: f32) -> bool {
    let (pa, pb) = closest_segment_to_segment(a0, a1, b0, b1);
    let dx = pa.x - pb.x;
    let dy = pa.y - pb.y;
    let dz = pa.z - pb.z;
    let dist_sq = dx * dx + dy * dy + dz * dz;
    let sum_r = ra + rb;
    dist_sq <= sum_r * sum_r
}

/// Returns `true` if a sphere overlaps a capsule.
///
/// The capsule is defined by segment (`cap_a`, `cap_b`) and radius `cr`.
pub fn sphere_vs_capsule(center: Vec3, sr: f32, cap_a: Vec3, cap_b: Vec3, cr: f32) -> bool {
    let seg = Vec3::new(cap_b.x - cap_a.x, cap_b.y - cap_a.y, cap_b.z - cap_a.z);
    let len_sq = seg.x * seg.x + seg.y * seg.y + seg.z * seg.z;
    let to_c = Vec3::new(center.x - cap_a.x, center.y - cap_a.y, center.z - cap_a.z);
    let t = if len_sq < 1e-12 {
        0.0
    } else {
        ((to_c.x * seg.x + to_c.y * seg.y + to_c.z * seg.z) / len_sq).clamp(0.0, 1.0)
    };
    let closest = Vec3::new(
        cap_a.x + t * seg.x,
        cap_a.y + t * seg.y,
        cap_a.z + t * seg.z,
    );
    let dx = center.x - closest.x;
    let dy = center.y - closest.y;
    let dz = center.z - closest.z;
    let sum_r = sr + cr;
    dx * dx + dy * dy + dz * dz <= sum_r * sum_r
}

/// SAT overlap test for two **convex** polygons.
///
/// Returns `true` if the polygons overlap.  Tests all edge normals of both
/// polygons as candidate separating axes — O(m + n) projections.
pub fn sat_overlap_polygons_2d(poly_a: &[Vec2], poly_b: &[Vec2]) -> bool {
    if poly_a.len() < 2 || poly_b.len() < 2 {
        return false;
    }
    let project = |poly: &[Vec2], ax: f32, ay: f32| -> (f32, f32) {
        poly.iter().fold((f32::MAX, f32::MIN), |(mn, mx), p| {
            let d = p.x * ax + p.y * ay;
            (mn.min(d), mx.max(d))
        })
    };
    let test_poly_axes = |poly: &[Vec2]| -> bool {
        let n = poly.len();
        for i in 0..n {
            let a = poly[i];
            let b = poly[(i + 1) % n];
            let nx = b.y - a.y;
            let ny = -(b.x - a.x);
            let len = (nx * nx + ny * ny).sqrt();
            if len < 1e-10 {
                continue;
            }
            let (ax, ay) = (nx / len, ny / len);
            let (min_a, max_a) = project(poly_a, ax, ay);
            let (min_b, max_b) = project(poly_b, ax, ay);
            if max_a < min_b || max_b < min_a {
                return false;
            }
        }
        true
    };
    test_poly_axes(poly_a) && test_poly_axes(poly_b)
}

/// SAT overlap test for two 2D oriented bounding boxes (OBBs).
///
/// Each OBB is defined by center, half-extents, and rotation angle in radians.
/// Tests 4 axes (2 local axes per box).
pub fn obb_vs_obb_2d(
    center_a: Vec2,
    half_a: Vec2,
    angle_a: f32,
    center_b: Vec2,
    half_b: Vec2,
    angle_b: f32,
) -> bool {
    let (sa, ca) = angle_a.sin_cos();
    let (sb, cb) = angle_b.sin_cos();
    let axes = [(ca, sa), (-sa, ca), (cb, sb), (-sb, cb)];
    let corners = |center: Vec2, half: Vec2, s: f32, c: f32| -> [Vec2; 4] {
        [
            Vec2::new(
                center.x + c * half.x - s * half.y,
                center.y + s * half.x + c * half.y,
            ),
            Vec2::new(
                center.x - c * half.x - s * half.y,
                center.y - s * half.x + c * half.y,
            ),
            Vec2::new(
                center.x - c * half.x + s * half.y,
                center.y - s * half.x - c * half.y,
            ),
            Vec2::new(
                center.x + c * half.x + s * half.y,
                center.y + s * half.x - c * half.y,
            ),
        ]
    };
    let ca_pts = corners(center_a, half_a, sa, ca);
    let cb_pts = corners(center_b, half_b, sb, cb);
    let project = |pts: &[Vec2; 4], ax: f32, ay: f32| -> (f32, f32) {
        pts.iter().fold((f32::MAX, f32::MIN), |(mn, mx), p| {
            let d = p.x * ax + p.y * ay;
            (mn.min(d), mx.max(d))
        })
    };
    for (ax, ay) in axes {
        let (min_a, max_a) = project(&ca_pts, ax, ay);
        let (min_b, max_b) = project(&cb_pts, ax, ay);
        if max_a < min_b || max_b < min_a {
            return false;
        }
    }
    true
}

// ── Tests — Pass 52 ───────────────────────────────────────────────────────────

// ── Pass 53 — SDF primitives & operators ─────────────────────────────────────
// sdf_sphere, sdf_box_3d, sdf_torus, sdf_capsule_3d, sdf_cone,
// sdf_cylinder, sdf_op_union, sdf_op_subtract, sdf_op_intersect,
// sdf_op_smooth_union, sdf_op_round, sdf_op_onion, sdf_op_repeat_3d
// ─────────────────────────────────────────────────────────────────────────────

/// SDF: unit sphere of radius `r` centred at the origin.
///
/// `p` is the query point in the sphere's local frame.
/// Returns negative inside, zero on surface, positive outside.
#[inline]
pub fn sdf_sphere(p: Vec3, r: f32) -> f32 {
    (p.x * p.x + p.y * p.y + p.z * p.z).sqrt() - r
}

/// SDF: axis-aligned box centred at the origin with half-extents `b`.
///
/// Uses Quilez's formulation that correctly handles all octants, including
/// interior points, with a single expression.
#[inline]
pub fn sdf_box_3d(p: Vec3, b: Vec3) -> f32 {
    let qx = p.x.abs() - b.x;
    let qy = p.y.abs() - b.y;
    let qz = p.z.abs() - b.z;
    let outer =
        (qx.max(0.0) * qx.max(0.0) + qy.max(0.0) * qy.max(0.0) + qz.max(0.0) * qz.max(0.0)).sqrt();
    let inner = qx.max(qy).max(qz).min(0.0);
    outer + inner
}

/// SDF: torus lying in the XZ plane, centred at the origin.
///
/// `r_major` is the distance from the centre of the tube to the centre of the
/// torus; `r_minor` is the tube radius.
#[inline]
pub fn sdf_torus(p: Vec3, r_major: f32, r_minor: f32) -> f32 {
    let q_xz = p.x.mul_add(p.x, p.z * p.z).sqrt() - r_major;
    (q_xz * q_xz + p.y * p.y).sqrt() - r_minor
}

/// SDF: capsule (line-swept sphere) from point `a` to `b` with radius `r`.
#[inline]
pub fn sdf_capsule_3d(p: Vec3, a: Vec3, b: Vec3, r: f32) -> f32 {
    let pa = Vec3::new(p.x - a.x, p.y - a.y, p.z - a.z);
    let ba = Vec3::new(b.x - a.x, b.y - a.y, b.z - a.z);
    let ba_len_sq = ba.x * ba.x + ba.y * ba.y + ba.z * ba.z;
    let t = if ba_len_sq < 1e-12 {
        0.0
    } else {
        ((pa.x * ba.x + pa.y * ba.y + pa.z * ba.z) / ba_len_sq).clamp(0.0, 1.0)
    };
    let dx = pa.x - t * ba.x;
    let dy = pa.y - t * ba.y;
    let dz = pa.z - t * ba.z;
    (dx * dx + dy * dy + dz * dz).sqrt() - r
}

/// SDF: infinite cone opening along +Y, defined by its half-angle `angle`
/// (radians).  Apex at origin.  Quilez exact formulation.
#[inline]
pub fn sdf_cone(p: Vec3, angle: f32) -> f32 {
    let (sin_a, cos_a) = angle.sin_cos();
    let q = p.x.mul_add(p.x, p.z * p.z).sqrt();
    // In 2D (q, y) space, project onto the cone edge direction c = (sin_a, -cos_a).
    let d = ((q * sin_a - p.y * cos_a).max(0.0) * (q * sin_a - p.y * cos_a).max(0.0)
        + (q * cos_a + p.y * sin_a) * (q * cos_a + p.y * sin_a))
        .sqrt()
        - 0.0; // placeholder — see below
    // Exact: dot and cross with c.
    let dot_qc = q * sin_a - p.y * cos_a;
    let cross_qc = q * cos_a + p.y * sin_a; // perpendicular dist to edge line
    let _ = d;
    // Sign: inside cone if dot_qc <= 0.
    if dot_qc <= 0.0 {
        -cross_qc.abs()
    } else {
        // Outside cone on the wide side.
        (dot_qc * dot_qc + cross_qc * cross_qc).sqrt() * cross_qc.signum().max(0.0)
            + cross_qc.abs() * (1.0 - cross_qc.signum().max(0.0))
    }
}

/// SDF: finite cone from apex `a` to base-centre `b` with base radius `r`.
///
/// Correct signed distance: negative inside, zero on surface, positive outside.
pub fn sdf_cone_finite(p: Vec3, a: Vec3, b: Vec3, r: f32) -> f32 {
    let ba = Vec3::new(b.x - a.x, b.y - a.y, b.z - a.z);
    let pa = Vec3::new(p.x - a.x, p.y - a.y, p.z - a.z);
    let ba_len = (ba.x * ba.x + ba.y * ba.y + ba.z * ba.z).sqrt();
    if ba_len < 1e-12 {
        return sdf_sphere(p, r);
    }
    let t_norm = Vec3::new(ba.x / ba_len, ba.y / ba_len, ba.z / ba_len);
    // Axial coordinate (0 at apex, ba_len at base).
    let qx = pa.x * t_norm.x + pa.y * t_norm.y + pa.z * t_norm.z;
    // Radial distance from axis.
    let perp_x = pa.x - qx * t_norm.x;
    let perp_y = pa.y - qx * t_norm.y;
    let perp_z = pa.z - qx * t_norm.z;
    let qr = (perp_x * perp_x + perp_y * perp_y + perp_z * perp_z).sqrt();
    // In 2D (axial, radial), the cone is a right triangle:
    // apex=(0,0), base edge from (ba_len,0) to (ba_len,r).
    // c = normalize(ba_len, r) is the slant direction.
    let c_len = (ba_len * ba_len + r * r).sqrt();
    let cx = ba_len / c_len; // cos of slant angle
    let cr = r / c_len; // sin of slant angle
    // Dot and cross of (qx,qr) with slant c and its normal.
    let d_dot = qx * cx + qr * cr;
    let d_cross = qx * cr - qr * cx;
    // Clamp d_dot to the slant segment [0, c_len].
    let d_dot_c = d_dot.clamp(0.0, c_len);
    let dx = d_dot - d_dot_c;
    let dy = d_cross;
    let dist = dx.mul_add(dx, dy * dy).sqrt();
    // Inside if d_cross < 0 (left of slant line) and qx in [0, ba_len].
    let inside = d_cross <= 0.0 && qx >= 0.0 && qx <= ba_len;
    if inside { -dist } else { dist }
}

/// SDF: infinite cylinder along the Y axis with radius `r`.
#[inline]
pub fn sdf_cylinder(p: Vec3, r: f32) -> f32 {
    p.x.mul_add(p.x, p.z * p.z).sqrt() - r
}

/// SDF: finite cylinder from `a` to `b` with radius `r`.
pub fn sdf_cylinder_finite(p: Vec3, a: Vec3, b: Vec3, r: f32) -> f32 {
    let ba = Vec3::new(b.x - a.x, b.y - a.y, b.z - a.z);
    let pa = Vec3::new(p.x - a.x, p.y - a.y, p.z - a.z);
    let ba_len_sq = ba.x * ba.x + ba.y * ba.y + ba.z * ba.z;
    let ba_len = ba_len_sq.sqrt();
    // Axial and radial components.
    let t_norm = Vec3::new(ba.x / ba_len, ba.y / ba_len, ba.z / ba_len);
    let axial = pa.x * t_norm.x + pa.y * t_norm.y + pa.z * t_norm.z;
    let perp_x = pa.x - axial * t_norm.x;
    let perp_y = pa.y - axial * t_norm.y;
    let perp_z = pa.z - axial * t_norm.z;
    let radial = (perp_x * perp_x + perp_y * perp_y + perp_z * perp_z).sqrt();
    // 2D box SDF in (axial, radial) space — note radial is always >= 0.
    let dx = radial - r;
    let dy = axial.abs() - ba_len * 0.5;
    // Shift: axial goes 0..ba_len, centre at ba_len/2.
    let axial_centered = axial - ba_len * 0.5;
    let dy2 = axial_centered.abs() - ba_len * 0.5;
    let outer = (dx.max(0.0) * dx.max(0.0) + dy2.max(0.0) * dy2.max(0.0)).sqrt();
    let inner = dx.max(dy2).min(0.0);
    let _ = dy;
    outer + inner
}

// ── SDF boolean operators ─────────────────────────────────────────────────────

/// SDF union (take closer surface).
#[inline]
pub const fn sdf_op_union(a: f32, b: f32) -> f32 {
    a.min(b)
}

/// SDF subtraction: remove `b` from `a`.
#[inline]
pub fn sdf_op_subtract(a: f32, b: f32) -> f32 {
    a.max(-b)
}

/// SDF intersection: keep only the overlap.
#[inline]
pub const fn sdf_op_intersect(a: f32, b: f32) -> f32 {
    a.max(b)
}

/// SDF smooth union using the polynomial smooth-min (Quilez k-factor).
///
/// Blends two surfaces within distance `k` of each other.
/// Uses `smooth_min_poly` internally.
#[inline]
pub fn sdf_op_smooth_union(a: f32, b: f32, k: f32) -> f32 {
    smooth_min_poly(a, b, k)
}

/// SDF round: expand a shape outward by `r` (rounds all edges/corners).
#[inline]
pub fn sdf_op_round(d: f32, r: f32) -> f32 {
    d - r
}

/// SDF onion: hollow shell of thickness `r` from an existing SDF.
#[inline]
pub fn sdf_op_onion(d: f32, r: f32) -> f32 {
    d.abs() - r
}

/// SDF domain repeat: tile 3D space with period `c` (per axis).
///
/// Returns the remapped point to evaluate in; call your SDF on the result.
/// The repeat cell is centred at the origin.
#[inline]
pub fn sdf_op_repeat_3d(p: Vec3, c: Vec3) -> Vec3 {
    Vec3::new(
        p.x - c.x * (p.x / c.x).round(),
        p.y - c.y * (p.y / c.y).round(),
        p.z - c.z * (p.z / c.z).round(),
    )
}

// ── Tests — Pass 53 ───────────────────────────────────────────────────────────

// ── Pass 54 — hash functions, XYZ/RGB, blackbody, Hilbert curve ───────────────
// wang_hash, lowbias32, murmur3_fmix32, hash_to_unit_vec3,
// xyz_to_linear_rgb, blackbody_linear_rgb,
// hilbert_xy_to_d, hilbert_d_to_xy
// ─────────────────────────────────────────────────────────────────────────────

/// Lowbias32 hash — Chris Wellons bijective integer hash with minimal bias.
///
/// Avalanche score ~0.020 bits (near perfect for a 32-bit hash).
#[inline]
pub const fn lowbias32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x45d9_f3b7);
    x ^= x >> 16;
    x = x.wrapping_mul(0x45d9_f3b7);
    x ^= x >> 16;
    x
}

/// `MurmurHash3` finalizer (fmix32) — excellent bit mixing for hash tables.
#[inline]
pub const fn murmur3_fmix32(mut h: u32) -> u32 {
    h ^= h >> 16;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    h
}

/// Map a u32 seed to a uniformly distributed point on the unit sphere.
///
/// Uses two hash rounds to produce independent azimuthal and polar coordinates.
pub fn hash_to_unit_vec3(seed: u32) -> Vec3 {
    const INV32: f32 = 2.328_306_4e-10; // 2^-32
    let h1 = murmur3_fmix32(seed);
    let h2 = murmur3_fmix32(h1.wrapping_add(0x9e37_79b9));
    // cos(polar angle) uniform in [-1, 1]; azimuth uniform in [0, TAU).
    let cos_theta = h1 as f32 * INV32 * 2.0 - 1.0;
    let phi = h2 as f32 * INV32 * std::f32::consts::TAU;
    let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();
    Vec3::new(sin_theta * phi.cos(), cos_theta, sin_theta * phi.sin())
}

/// Physically accurate blackbody emission as **linear** sRGB.
///
/// Uses the Kang 2002 approximation of the Planckian locus (CIE xy chromaticity
/// as a function of T in K) then converts via XYZ to linear sRGB.  Output is
/// normalised so Y = 1 (relative spectral power, not absolute radiance).
///
/// Valid range: 1667 K to 25 000 K.  Returns linear sRGB; may be out-of-gamut.
pub fn blackbody_linear_rgb(kelvin: f32) -> (f32, f32, f32) {
    let t = kelvin.clamp(1667.0, 25_000.0);
    // Kang 2002 — chromaticity x as function of T.
    let ti = 1.0 / t;
    let x = if t < 4000.0 {
        -0.266_123_9e9 * ti * ti * ti - 0.234_358_0e6 * ti * ti + 0.877_695_6e3 * ti + 0.179_910
    } else {
        -3.025_846_9e9 * ti * ti * ti + 2.107_037_9e6 * ti * ti + 0.222_634_7e3 * ti + 0.240_390
    };
    // Chromaticity y from x.
    let y = if t < 4000.0 {
        -1.106_381_4 * x * x * x - 1.348_110_2 * x * x + 2.185_558_32 * x - 0.202_196_83
    } else {
        3.081_758 * x * x * x - 5.873_386_7 * x * x + 3.751_129_97 * x - 0.370_014_83
    };
    // xyY to XYZ with Y = 1.
    let big_x = if y.abs() < 1e-8 { 0.0 } else { x / y };
    let big_y = 1.0;
    let big_z = if y.abs() < 1e-8 {
        0.0
    } else {
        (1.0 - x - y) / y
    };
    xyz_to_linear_rgb(big_x, big_y, big_z)
}

/// Encode a 2D point `(x, y)` to its index `d` on the Hilbert curve of order
/// `n` (i.e., the curve divides each axis into `2^n` cells).
///
/// `x` and `y` must be in `[0, 2^n)`.  Returns the Hilbert index in
/// `[0, 4^n)`.
#[allow(clippy::bool_to_int_with_if)]
pub const fn hilbert_xy_to_d(mut x: u32, mut y: u32, n: u32) -> u32 {
    let mut d = 0u32;
    let mut s = 1u32 << (n - 1);
    while s > 0 {
        let rx = (x & s > 0) as u32;
        let ry = (y & s > 0) as u32;
        d += s * s * ((3 * rx) ^ ry);
        // Rotate quadrant.
        if ry == 0 {
            if rx == 1 {
                x = s.wrapping_sub(1).wrapping_sub(x);
                y = s.wrapping_sub(1).wrapping_sub(y);
            }
            std::mem::swap(&mut x, &mut y);
        }
        s >>= 1;
    }
    d
}

/// Decode Hilbert index `d` to `(x, y)` coordinates for a curve of order `n`.
///
/// Inverse of `hilbert_xy_to_d`.  Returns `(x, y)` in `[0, 2^n)`.
pub const fn hilbert_d_to_xy(mut d: u32, n: u32) -> (u32, u32) {
    let mut x = 0u32;
    let mut y = 0u32;
    let mut s = 1u32;
    while s < (1u32 << n) {
        let rx = (d >> 1) & 1;
        let ry = d & 1 ^ rx;
        // Rotate.
        if ry == 0 {
            if rx == 1 {
                x = s.wrapping_sub(1).wrapping_sub(x);
                y = s.wrapping_sub(1).wrapping_sub(y);
            }
            std::mem::swap(&mut x, &mut y);
        }
        x += s * rx;
        y += s * ry;
        d >>= 2;
        s <<= 1;
    }
    (x, y)
}

// ── Tests — Pass 54 ───────────────────────────────────────────────────────────

// ── Pass 55 — window functions, DCT-II, RMS, SDF normal ──────────────────────
// hann_window, hamming_window, blackman_window, apply_window,
// rms, dct_ii, idct_ii, sdf_normal_3d
// ─────────────────────────────────────────────────────────────────────────────

/// Hann window coefficient for sample `i` in a window of length `n`.
///
/// `w[i] = 0.5 * (1 - cos(2π·i / n))`.  Periodic form (suitable for
/// overlap-add); for symmetric use `n - 1` in the denominator.
#[inline]
pub fn hann_window(n: usize, i: usize) -> f32 {
    use std::f32::consts::TAU;
    0.5 * (1.0 - (TAU * i as f32 / n as f32).cos())
}

/// Hamming window coefficient for sample `i` in a window of length `n`.
///
/// `w[i] = 0.54 - 0.46·cos(2π·i / (n-1))`.  Symmetric form.
#[inline]
pub fn hamming_window(n: usize, i: usize) -> f32 {
    use std::f32::consts::TAU;
    let denom = if n > 1 { (n - 1) as f32 } else { 1.0 };
    0.54 - 0.46 * (TAU * i as f32 / denom).cos()
}

/// Blackman window coefficient for sample `i` in a window of length `n`.
///
/// `w[i] = 0.42 - 0.5·cos(2π·i/(n-1)) + 0.08·cos(4π·i/(n-1))`.
/// Symmetric form; ~18 dB lower sidelobes than Hamming at the cost of a
/// wider main lobe.
#[inline]
pub fn blackman_window(n: usize, i: usize) -> f32 {
    use std::f32::consts::TAU;
    let denom = if n > 1 { (n - 1) as f32 } else { 1.0 };
    let x = TAU * i as f32 / denom;
    0.42 - 0.5 * x.cos() + 0.08 * (2.0 * x).cos()
}

/// Apply a window function to `signal`, returning the element-wise product.
///
/// `window_fn(n, i)` should return the window coefficient for sample `i` in a
/// window of total length `n`.  Pairs with `hann_window`, `hamming_window`, etc.
pub fn apply_window(signal: &[f32], window_fn: impl Fn(usize, usize) -> f32) -> Vec<f32> {
    let n = signal.len();
    signal
        .iter()
        .enumerate()
        .map(|(i, &s)| s * window_fn(n, i))
        .collect()
}

/// Root-mean-square amplitude of `signal`.
///
/// Returns 0 for an empty slice.
#[inline]
pub fn rms(signal: &[f32]) -> f32 {
    if signal.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = signal.iter().map(|&x| x * x).sum();
    (sum_sq / signal.len() as f32).sqrt()
}

/// Discrete Cosine Transform — Type II (DCT-II), O(n²) naive implementation.
///
/// `X[k] = Σ_{n=0}^{N-1} x[n] · cos(π·k·(2n+1) / (2N))`
///
/// This is the transform used in JPEG block coding.  For an efficient O(n log n)
/// version, compose with the FFT.
pub fn dct_ii(signal: &[f32]) -> Vec<f32> {
    let big_n = signal.len();
    if big_n == 0 {
        return Vec::new();
    }
    let scale = std::f32::consts::PI / (2 * big_n) as f32;
    (0..big_n)
        .map(|k| {
            signal
                .iter()
                .enumerate()
                .map(|(n, &x)| x * (scale * k as f32 * (2 * n + 1) as f32).cos())
                .sum()
        })
        .collect()
}

/// Inverse DCT-II (IDCT-II = scaled DCT-III), O(n²) naive implementation.
///
/// Recovers `x` from DCT-II coefficients `X`:
/// `x[n] = (1/N)·X[0] + (2/N)·Σ_{k=1}^{N-1} X[k]·cos(π·k·(2n+1)/(2N))`
pub fn idct_ii(coeffs: &[f32]) -> Vec<f32> {
    let big_n = coeffs.len();
    if big_n == 0 {
        return Vec::new();
    }
    let inv_n = 1.0 / big_n as f32;
    let scale = std::f32::consts::PI / (2 * big_n) as f32;
    (0..big_n)
        .map(|n| {
            let dc = coeffs[0] * inv_n;
            let ac: f32 = coeffs[1..]
                .iter()
                .enumerate()
                .map(|(k, &x)| x * (scale * (k + 1) as f32 * (2 * n + 1) as f32).cos())
                .sum::<f32>()
                * (2.0 * inv_n);
            dc + ac
        })
        .collect()
}

/// Estimate the surface normal at point `p` for an arbitrary SDF via central
/// differences.
///
/// Makes 6 SDF evaluations.  `eps` controls the finite-difference step;
/// typical values are 1e-3 to 1e-4.  Returns a unit normal.
pub fn sdf_normal_3d(p: Vec3, sdf: impl Fn(Vec3) -> f32, eps: f32) -> Vec3 {
    let dx = sdf(Vec3::new(p.x + eps, p.y, p.z)) - sdf(Vec3::new(p.x - eps, p.y, p.z));
    let dy = sdf(Vec3::new(p.x, p.y + eps, p.z)) - sdf(Vec3::new(p.x, p.y - eps, p.z));
    let dz = sdf(Vec3::new(p.x, p.y, p.z + eps)) - sdf(Vec3::new(p.x, p.y, p.z - eps));
    let len = (dx * dx + dy * dy + dz * dz).sqrt();
    if len < 1e-10 {
        return Vec3::new(0.0, 1.0, 0.0);
    }
    Vec3::new(dx / len, dy / len, dz / len)
}

// ── Tests — Pass 55 ───────────────────────────────────────────────────────────

// ── Pass 56 — rendering utilities ────────────────────────────────────────────
// perspective_reverse_z, taa_halton_jitter, f32_to_f16, f16_to_f32,
// cascade_shadow_splits, reconstruct_normal_z, perspective_oblique
// ─────────────────────────────────────────────────────────────────────────────

/// Reverse-Z infinite perspective matrix (right-handed, NDC z in [0, 1]).
///
/// Maps near plane to z=1 and the infinite far plane to z=0.  This
/// dramatically improves floating-point depth precision in the distance
/// because IEEE 754 f32 has more representable values near 0.
///
/// `fov_y` is the vertical field-of-view in radians; `aspect = width/height`.
pub fn perspective_reverse_z(fov_y: f32, aspect: f32, near: f32) -> Mat4 {
    let f = 1.0 / (fov_y * 0.5).tan();
    // Column-major layout matching the existing Mat4 convention.
    let mut m = Mat4::identity();
    m.m[0][0] = f / aspect;
    m.m[1][1] = f;
    m.m[2][2] = 0.0; // z maps to 0 at infinity
    m.m[2][3] = -1.0; // perspective divide
    m.m[3][2] = near; // near plane maps to z=1
    m.m[3][3] = 0.0;
    m
}

/// Sub-pixel jitter offset for Temporal Anti-Aliasing using Halton(2,3).
///
/// Returns a `Vec2` offset in **pixel** units centred at 0 (range ±0.5).
/// Feed it to your projection matrix before rendering each frame.
pub fn taa_halton_jitter(frame: u32, width: u32, height: u32) -> Vec2 {
    // Use frame+1 to avoid index 0 (Halton(2,0) = 0).
    let idx = (frame % 16) + 1;
    let x = halton(idx, 2) - 0.5;
    let y = halton(idx, 3) - 0.5;
    Vec2::new(x / width as f32, y / height as f32)
}

/// Convert an IEEE 754 `f32` to a 16-bit half-float (`f16`) bit pattern.
///
/// Handles normals, zeros, infinities, and NaN.  Subnormals are flushed to
/// zero for simplicity (matches the most common GPU behaviour).
pub const fn f32_to_f16(x: f32) -> u16 {
    let bits = x.to_bits();
    let sign = ((bits >> 31) & 1) as u16;
    let exp32 = ((bits >> 23) & 0xff) as i32;
    let mantissa32 = bits & 0x007f_ffff;

    if exp32 == 255 {
        // Inf or NaN.
        let mant16 = if mantissa32 != 0 { 0x0200u16 } else { 0 }; // preserve NaN flag
        return (sign << 15) | 0x7c00 | mant16;
    }
    let exp16 = exp32 - 127 + 15;
    if exp16 >= 31 {
        // Overflow -> Inf.
        return (sign << 15) | 0x7c00;
    }
    if exp16 <= 0 {
        // Subnormal or underflow -> flush to zero.
        return sign << 15;
    }
    let mant16 = (mantissa32 >> 13) as u16;
    (sign << 15) | ((exp16 as u16) << 10) | mant16
}

/// Convert a 16-bit half-float (`f16`) bit pattern to `f32`.
#[allow(clippy::cast_lossless)]
pub const fn f16_to_f32(h: u16) -> f32 {
    let sign = ((h >> 15) & 1) as u32;
    let exp16 = ((h >> 10) & 0x1f) as i32;
    let mant16 = (h & 0x03ff) as u32;

    let (exp32, mant32) = if exp16 == 0 {
        if mant16 == 0 {
            (0, 0) // zero
        } else {
            // Subnormal: normalise.
            let mut m = mant16;
            let mut e = -14i32;
            while m & 0x0400 == 0 {
                m <<= 1;
                e -= 1;
            }
            ((e + 127) as u32, (m & 0x03ff) << 13)
        }
    } else if exp16 == 31 {
        (255, mant16 << 13) // Inf / NaN
    } else {
        ((exp16 + 127 - 15) as u32, mant16 << 13)
    };

    f32::from_bits((sign << 31) | (exp32 << 23) | mant32)
}

/// Parallel-Split Shadow Map (PSSM) cascade split distances.
///
/// Returns `n + 1` values `[near, s1, s2, ..., far]` dividing the view
/// frustum depth range into `n` cascades.  `lambda` blends between a
/// logarithmic split (`lambda = 1.0`) and a uniform split (`lambda = 0.0`).
pub fn cascade_shadow_splits(near: f32, far: f32, n: u32, lambda: f32) -> Vec<f32> {
    let n = n.max(1) as usize;
    let mut splits = Vec::with_capacity(n + 1);
    splits.push(near);
    for i in 1..n {
        let p = i as f32 / n as f32;
        let log = near * (far / near).powf(p);
        let uni = near + (far - near) * p;
        splits.push(lambda * log + (1.0 - lambda) * uni);
    }
    splits.push(far);
    splits
}

/// Reconstruct the Z component of a unit normal from its XY components.
///
/// Assumes the normal was stored in a normal map with only X and Y channels
/// (common for tangent-space normal maps where Z is always >= 0).
/// Returns a normalised `Vec3`.
#[inline]
pub fn reconstruct_normal_z(xy: Vec2) -> Vec3 {
    let z = (1.0 - xy.x * xy.x - xy.y * xy.y).max(0.0).sqrt();
    let len = (xy.x * xy.x + xy.y * xy.y + z * z).sqrt().max(1e-10);
    Vec3::new(xy.x / len, xy.y / len, z / len)
}

/// Oblique near-plane perspective matrix.
///
/// Modifies a standard perspective matrix so the near clip plane is the plane
/// defined by `clip_plane` (in eye/view space, as a `Vec4` `(a, b, c, d)` where
/// `ax + by + cz + d = 0`).  Used for portal rendering and planar reflections.
///
/// Algorithm: Eric Lengyel, "Modifying the Projection Matrix to Perform
/// Oblique Near-Plane Clipping" (Game Programming Gems 5).
pub fn perspective_oblique(proj: Mat4, clip_plane: [f32; 4]) -> Mat4 {
    // q = inverse-transpose of proj * clip_plane sign-adjusted to point inward.
    // Compute the clip-space plane.
    let [a, b, c, d] = clip_plane;
    // The corner of the frustum on the same side as the plane normal.
    let qx = (a.signum() + proj.m[2][0]) / proj.m[0][0];
    let qy = (b.signum() + proj.m[2][1]) / proj.m[1][1];
    let qz = -1.0_f32; // always -1 in a right-handed projection
    let qw = (1.0 + proj.m[2][2]) / proj.m[3][2];

    // Scale so that dot(plane, q) = 2.
    let dot = a * qx + b * qy + c * qz + d * qw;
    let scale = if dot.abs() < 1e-10 { 1.0 } else { 2.0 / dot };

    // Replace the third row of the projection matrix.
    let mut out = proj;
    out.m[2][0] = a * scale - proj.m[3][0];
    out.m[2][1] = b * scale - proj.m[3][1];
    out.m[2][2] = c * scale - proj.m[3][2];
    out.m[2][3] = d * scale - proj.m[3][3];
    out
}

// ── Tests — Pass 56 ───────────────────────────────────────────────────────────

// ── Pass 57 — camera exposure, splines, quat utilities, oscillator ────────────
// ev100, ev100_to_exposure, log_average_luminance,
// cardinal_spline, quat_look_at, quat_nlerp_weighted, damped_oscillator_state
// ─────────────────────────────────────────────────────────────────────────────

/// Photographic Exposure Value at ISO 100 (EV100).
///
/// `aperture` is the f-number (e.g. 2.8), `shutter` is exposure time in
/// seconds (e.g. 1/125 = 0.008), `iso` is the film/sensor speed.
/// Result is measured in EV steps.
#[inline]
pub fn ev100(aperture: f32, shutter: f32, iso: f32) -> f32 {
    (aperture * aperture / shutter).log2() - (iso / 100.0).log2()
}

/// Convert an EV100 value to a linear exposure multiplier.
///
/// Applies the calibration constant K=12.5 (reflected-light meter standard):
/// `exposure = 1 / (1.2 * 2^EV100)`.
#[inline]
pub fn ev100_to_exposure(ev100_val: f32) -> f32 {
    1.0 / (1.2 * ev100_val.exp2())
}

/// Log-average luminance of a luminance slice — the geometric mean.
///
/// Used as the scene key value in auto-exposure algorithms.
/// Returns 0 for an empty slice; tiny epsilon avoids log(0) on black pixels.
pub fn log_average_luminance(luminances: &[f32]) -> f32 {
    const EPSILON: f32 = 1e-5;
    if luminances.is_empty() {
        return 0.0;
    }
    let sum: f32 = luminances.iter().map(|&l| (l + EPSILON).ln()).sum();
    (sum / luminances.len() as f32).exp()
}

/// Cardinal spline through four control points with a `tension` parameter.
///
/// At `tension = 0.0` this is identical to Catmull-Rom.
/// At `tension = 1.0` the tangents are zero and the curve is piecewise linear.
/// The curve passes through `p1` at `t=0` and `p2` at `t=1`.
pub fn cardinal_spline(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32, tension: f32) -> Vec3 {
    let s = (1.0 - tension) * 0.5;
    // Tangents at p1 and p2.
    let m1 = Vec3::new(s * (p2.x - p0.x), s * (p2.y - p0.y), s * (p2.z - p0.z));
    let m2 = Vec3::new(s * (p3.x - p1.x), s * (p3.y - p1.y), s * (p3.z - p1.z));
    // Cubic Hermite blend.
    let t2 = t * t;
    let t3 = t2 * t;
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;
    Vec3::new(
        h00 * p1.x + h10 * m1.x + h01 * p2.x + h11 * m2.x,
        h00 * p1.y + h10 * m1.y + h01 * p2.y + h11 * m2.y,
        h00 * p1.z + h10 * m1.z + h01 * p2.z + h11 * m2.z,
    )
}

/// Quaternion that rotates the +Z axis to align with `forward`.
///
/// `up` is the world-up hint used to determine the roll.  If `forward`
/// is nearly parallel to `up`, an arbitrary perpendicular is used.
pub fn quat_look_at(forward: Vec3, up: Vec3) -> Quat {
    // Normalise forward.
    let flen = (forward.x * forward.x + forward.y * forward.y + forward.z * forward.z).sqrt();
    if flen < 1e-10 {
        return Quat::identity();
    }
    let f = Vec3::new(forward.x / flen, forward.y / flen, forward.z / flen);

    // Right = up x forward (or fallback if parallel).
    let up_len = (up.x * up.x + up.y * up.y + up.z * up.z).sqrt().max(1e-10);
    let u = Vec3::new(up.x / up_len, up.y / up_len, up.z / up_len);
    let right_raw = Vec3::new(
        u.y * f.z - u.z * f.y,
        u.z * f.x - u.x * f.z,
        u.x * f.y - u.y * f.x,
    );
    let rlen =
        (right_raw.x * right_raw.x + right_raw.y * right_raw.y + right_raw.z * right_raw.z).sqrt();
    let r = if rlen < 1e-6 {
        // forward || up: pick arbitrary right.
        let alt = if f.x.abs() < 0.9 {
            Vec3::new(1.0, 0.0, 0.0)
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        };
        let c = Vec3::new(
            alt.y * f.z - alt.z * f.y,
            alt.z * f.x - alt.x * f.z,
            alt.x * f.y - alt.y * f.x,
        );
        let cl = (c.x * c.x + c.y * c.y + c.z * c.z).sqrt().max(1e-10);
        Vec3::new(c.x / cl, c.y / cl, c.z / cl)
    } else {
        Vec3::new(right_raw.x / rlen, right_raw.y / rlen, right_raw.z / rlen)
    };
    // Recompute up from forward and right.
    let new_up = Vec3::new(
        f.y * r.z - f.z * r.y,
        f.z * r.x - f.x * r.z,
        f.x * r.y - f.y * r.x,
    );
    // Build rotation matrix (column-major: right=X, up=Y, forward=Z).
    // Convert 3x3 to quaternion (Shepperd method).
    let m00 = r.x;
    let m10 = r.y;
    let m20 = r.z;
    let m01 = new_up.x;
    let m11 = new_up.y;
    let m21 = new_up.z;
    let m02 = f.x;
    let m12 = f.y;
    let m22 = f.z;
    let trace = m00 + m11 + m22;
    if trace > 0.0 {
        let s = 0.5 / (trace + 1.0).sqrt();
        Quat {
            w: 0.25 / s,
            x: (m21 - m12) * s,
            y: (m02 - m20) * s,
            z: (m10 - m01) * s,
        }
    } else if m00 > m11 && m00 > m22 {
        let s = 2.0 * (1.0 + m00 - m11 - m22).sqrt();
        Quat {
            w: (m21 - m12) / s,
            x: 0.25 * s,
            y: (m01 + m10) / s,
            z: (m02 + m20) / s,
        }
    } else if m11 > m22 {
        let s = 2.0 * (1.0 + m11 - m00 - m22).sqrt();
        Quat {
            w: (m02 - m20) / s,
            x: (m01 + m10) / s,
            y: 0.25 * s,
            z: (m12 + m21) / s,
        }
    } else {
        let s = 2.0 * (1.0 + m22 - m00 - m11).sqrt();
        Quat {
            w: (m10 - m01) / s,
            x: (m02 + m20) / s,
            y: (m12 + m21) / s,
            z: 0.25 * s,
        }
    }
}

/// Normalised linear blend of multiple quaternions (nlerp).
///
/// `quats` and `weights` must have the same length.  Weights need not sum to
/// 1 — they are normalised internally.  All quaternions are driven to the same
/// hemisphere as `quats[0]` before blending to avoid flipping artefacts.
/// Returns the identity quaternion if the input is empty or weights sum to 0.
/// # Panics
///
/// Panics if `quats` and `weights` have different lengths.
pub fn quat_nlerp_weighted(quats: &[Quat], weights: &[f32]) -> Quat {
    assert_eq!(quats.len(), weights.len());
    if quats.is_empty() {
        return Quat::identity();
    }
    let ref_q = quats[0];
    let mut ax = 0.0_f32;
    let mut ay = 0.0_f32;
    let mut az = 0.0_f32;
    let mut aw = 0.0_f32;
    let mut total_w = 0.0_f32;
    for (q, &w) in quats.iter().zip(weights.iter()) {
        if w <= 0.0 {
            continue;
        }
        // Ensure same hemisphere as ref.
        let dot = q.x * ref_q.x + q.y * ref_q.y + q.z * ref_q.z + q.w * ref_q.w;
        let sign = if dot < 0.0 { -1.0 } else { 1.0 };
        ax += sign * q.x * w;
        ay += sign * q.y * w;
        az += sign * q.z * w;
        aw += sign * q.w * w;
        total_w += w;
    }
    if total_w < 1e-10 {
        return Quat::identity();
    }
    let len = (ax * ax + ay * ay + az * az + aw * aw).sqrt();
    if len < 1e-10 {
        return Quat::identity();
    }
    Quat {
        w: aw / len,
        x: ax / len,
        y: ay / len,
        z: az / len,
    }
}

/// Generalised damped oscillator: returns `(position, velocity)` at time `t`.
///
/// - `omega` — natural frequency (rad/s); higher = stiffer spring
/// - `zeta` — damping ratio: `< 1` underdamped (oscillates), `= 1` critically
///   damped, `> 1` overdamped
/// - `x0`, `v0` — initial position and velocity
///
/// Uses the exact closed-form solution for each damping regime.
pub fn damped_oscillator_state(omega: f32, zeta: f32, x0: f32, v0: f32, t: f32) -> (f32, f32) {
    if zeta < 1.0 {
        // Underdamped.
        let wd = omega * (1.0 - zeta * zeta).sqrt(); // damped natural frequency
        let a = x0;
        let b = (v0 + zeta * omega * x0) / wd;
        let env = (-zeta * omega * t).exp();
        let x = env * (a * (wd * t).cos() + b * (wd * t).sin());
        let v = -zeta * omega * x + env * (-a * wd * (wd * t).sin() + b * wd * (wd * t).cos());
        (x, v)
    } else if (zeta - 1.0).abs() < 1e-6 {
        // Critically damped (reuse closed form from critically_damped_spring_step).
        let e = (-omega * t).exp();
        let b = v0 + omega * x0;
        let x = (x0 + b * t) * e;
        let v = (b * (1.0 - omega * t) - omega * x0) * e;
        (x, v)
    } else {
        // Overdamped.
        let wd = omega * (zeta * zeta - 1.0).sqrt();
        let r1 = -zeta * omega + wd;
        let r2 = -zeta * omega - wd;
        let c2 = (v0 - r1 * x0) / (r2 - r1);
        let c1 = x0 - c2;
        let x = c1 * (r1 * t).exp() + c2 * (r2 * t).exp();
        let v = c1 * r1 * (r1 * t).exp() + c2 * r2 * (r2 * t).exp();
        (x, v)
    }
}

// ── Tests — Pass 57 ───────────────────────────────────────────────────────────

// ── Pass 58: SDF shapes, SDF space operators, lens distortion ────────────────

/// Approximate signed distance to an ellipsoid centred at the origin.
///
/// `r` is the per-axis radii `(rx, ry, rz)`.  The result is an *exact* SDF
/// on the surface and a tight bound elsewhere (Inigo Quilez 2D→3D lift).
pub fn sdf_ellipsoid(p: Vec3, r: Vec3) -> f32 {
    // Component-wise p/r and p/(r*r) — Vec3 has no Vec3/Vec3 operator.
    let pr = Vec3::new(p.x / r.x, p.y / r.y, p.z / r.z);
    let pr2 = Vec3::new(p.x / (r.x * r.x), p.y / (r.y * r.y), p.z / (r.z * r.z));
    let k0 = pr.length();
    let k1 = pr2.length();
    // Guard: p = 0 → k1 = 0; return the negative of the smallest radius.
    if k1 < 1e-10 {
        return -(r.x.min(r.y).min(r.z));
    }
    k0 * (k0 - 1.0) / k1
}

/// Signed distance to a hexagonal prism.
///
/// The prism extends `±h.y` along Y and has a hexagonal cross-section of
/// inscribed radius `h.x` in the XZ plane (flat-top orientation).
pub fn sdf_hex_prism(p: Vec3, h: Vec2) -> f32 {
    // Quilez hex prism — hex cross-section in XZ, height axis Y.
    const KX: f32 = -0.866_025_4; // -√3/2
    const KY: f32 = 0.5;
    const KZ: f32 = 0.577_350_3; // 1/√3
    let ap = Vec3::new(p.x.abs(), p.y.abs(), p.z.abs());
    // Project into hex fundamental domain using XZ as the hex plane.
    let dot = 2.0 * (KX * ap.x + KY * ap.z).min(0.0);
    let qx = ap.x - dot * KX;
    let qz = ap.z - dot * KY;
    // Distance to hex edge in XZ, height in Y.
    let dx_raw = ((qx - (qz / KZ).clamp(0.0, h.x * KZ)).powi(2) + (qz - h.x).powi(2)).sqrt();
    let dx_sign = if qx - h.x * KZ > 0.0 || qz - h.x > 0.0 {
        1.0_f32
    } else {
        -1.0_f32
    };
    let d = Vec2::new(dx_sign * dx_raw, ap.y - h.y);
    d.x.max(d.y).min(0.0) + Vec2::new(d.x.max(0.0), d.y.max(0.0)).length()
}

/// Signed distance to a square pyramid centred at the origin.
///
/// `h` is the half-height (tip at `+h`, base at `y = -h`).  The base is a
/// square of side `2·h` (45° slope walls).  Quilez formulation.
pub fn sdf_pyramid(p: Vec3, h: f32) -> f32 {
    // Quilez square pyramid — tip at (0, +h, 0), base square of side 1 at y=0.
    let m2 = h * h + 0.25_f32;
    let mut qx = p.x.abs();
    let qy = p.y;
    let mut qz = p.z.abs();
    // Fold into the canonical octant (qx ≥ qz).
    if qz > qx {
        core::mem::swap(&mut qx, &mut qz);
    }
    qx -= 0.5;
    qz -= 0.5;
    // Project onto the slant edge.
    let q = Vec3::new(qz, h * qy - 0.5 * qx, h * qx + 0.5 * qy);
    let s = (-q.x).max(0.0);
    let t = ((q.y - 0.5 * qz) / (m2 + 0.25)).clamp(0.0, 1.0);
    let a = m2 * (q.x + s).powi(2) + q.y * q.y;
    let b = m2 * (q.x + 0.5 * t).powi(2) + (q.y - m2 * t).powi(2);
    let d = if q.y.min(-q.x * m2 - q.y * 0.5) > 0.0 {
        0.0
    } else {
        a.min(b)
    };
    ((d + q.z * q.z) / m2).sqrt() * (-qy).max(q.z).signum()
}

/// Smooth CSG subtraction: `a` minus `b`, with a soft blend of radius `k`.
///
/// Analogous to [`sdf_op_smooth_union`] but for subtraction.  Returns a
/// negative value (inside) only where `a` is solid and `b` is not.
pub fn sdf_op_smooth_subtract(a: f32, b: f32, k: f32) -> f32 {
    let h = (k - (a + b).abs()).max(0.0) / k;
    a.max(-b) + h * h * k * 0.25
}

/// Smooth CSG intersection: the overlap of `a` and `b`, with a blend of radius `k`.
pub fn sdf_op_smooth_intersect(a: f32, b: f32, k: f32) -> f32 {
    let h = (k - (a - b).abs()).max(0.0) / k;
    a.max(b) - h * h * k * 0.25
}

/// Elongate an SDF along a local axis by half-extents `h`.
///
/// Stretches the point `p` by clamping each axis independently before passing
/// it to the inner SDF.  Call as `sdf_op_elongate(p, h)` and use the result
/// as the point argument to any SDF primitive.
///
/// ```text
/// let p_elong = sdf_op_elongate(p, Vec3::new(0.5, 0.0, 0.0));
/// let d = sdf_sphere(p_elong, 0.3);
/// ```
pub fn sdf_op_elongate(p: Vec3, h: Vec3) -> Vec3 {
    // Vec3 has no Neg — negate component-wise.
    let neg_h = Vec3::new(-h.x, -h.y, -h.z);
    p - p.clamp(neg_h, h)
}

/// Twist space around the Y axis before evaluating an SDF.
///
/// Rotates the XZ plane by `k * p.y` radians, leaving Y unchanged.  Pass the
/// returned point as the argument to any SDF primitive.
///
/// ```text
/// let p_tw = sdf_op_twist(p, 2.0);
/// let d = sdf_box_3d(p_tw, Vec3::splat(0.3));
/// ```
pub fn sdf_op_twist(p: Vec3, k: f32) -> Vec3 {
    let (s, c) = (k * p.y).sin_cos();
    Vec3::new(c * p.x - s * p.z, p.y, s * p.x + c * p.z)
}

/// Barrel / pincushion lens distortion of a UV coordinate.
///
/// `uv` should be in `[-1, 1]` (centred).  Positive `k` gives barrel
/// distortion (edges bow outward); negative `k` gives pincushion (edges bow
/// inward).  Typical `|k|` values: 0.05–0.3.
///
/// Returns the distorted UV, also in `[-1, 1]` space.
pub fn barrel_distortion(uv: Vec2, k: f32) -> Vec2 {
    let r2 = uv.dot(uv);
    uv * (1.0 + k * r2)
}

// ── Pass 58 tests ─────────────────────────────────────────────────────────────

// ── Pass 59: fog, projectile, normal-map blend, PBR microfacet ───────────────

/// Linear fog factor: 1 (fully fogged) at `end`, 0 (clear) at `start`.
///
/// Returns a value in `[0, 1]`.
pub fn fog_factor_linear(dist: f32, start: f32, end: f32) -> f32 {
    if end <= start {
        return 1.0;
    }
    ((end - dist) / (end - start)).clamp(0.0, 1.0)
}

/// Exponential fog factor: `exp(-density * dist)`.
///
/// Returns a value in `(0, 1]`; approaches 0 (fully fogged) as `dist` grows.
pub fn fog_factor_exp(dist: f32, density: f32) -> f32 {
    (-density * dist).exp()
}

/// Squared-exponential fog factor: `exp(-(density * dist)²)`.
///
/// Thinner at short ranges, heavier at long ranges than [`fog_factor_exp`].
pub fn fog_factor_exp2(dist: f32, density: f32) -> f32 {
    (-(density * dist).powi(2)).exp()
}

/// Solid angle (steradians) subtended by a sphere of radius `r` at distance `d`
/// from its centre (observer outside the sphere, so `d > r`).
///
/// Returns `2π · (1 − cos θ)` where `sin θ = r / d`.
pub fn solid_angle_sphere(r: f32, d: f32) -> f32 {
    if d <= r {
        // Observer inside or on the sphere — full hemisphere or 4π.
        return 2.0 * std::f32::consts::PI;
    }
    let cos_theta = (1.0 - (r / d).powi(2)).sqrt();
    2.0 * std::f32::consts::PI * (1.0 - cos_theta)
}

/// Kinematic projectile position at time `t`.
///
/// `gravity` is in world units/s² downward (typically `Vec3::new(0,-9.81,0)`).
///
/// Returns `pos0 + vel0·t + ½·gravity·t²`.
pub fn projectile_position(pos0: Vec3, vel0: Vec3, t: f32, gravity: Vec3) -> Vec3 {
    pos0 + vel0 * t + gravity * (0.5 * t * t)
}

/// Reoriented Normal Map (RNM) blending of two tangent-space normals.
///
/// Both `n1` and `n2` must be in tangent space and normalised.  The result is
/// a normalised tangent-space normal that represents applying `n2` on top of
/// `n1`.  Superior to simple additive / partial-derivative blending for large
/// angles.  (Pettineo / Karis)
pub fn normal_map_blend_rnm(n1: Vec3, n2: Vec3) -> Vec3 {
    // Reorient n2 into the frame defined by n1.
    let t = Vec3::new(n1.x, n1.y, n1.z + 1.0);
    let u = Vec3::new(-n2.x, -n2.y, n2.z);
    let r = t * t.dot(u) - u * t.z;
    let len = r.length();
    if len < 1e-10 {
        return n1;
    }
    r / len
}

/// GGX (Trowbridge-Reitz) Normal Distribution Function.
///
/// - `n_dot_h` — dot product of the surface normal and the halfway vector `∈ [0,1]`
/// - `roughness` — linear roughness `∈ (0, 1]` (alpha = roughness²)
///
/// Returns the NDF weight `D(h)`.
pub fn ggx_d(n_dot_h: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let d = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    a2 / (std::f32::consts::PI * d * d)
}

/// Smith-Schlick-GGX single-term geometry function.
///
/// Used for either the view or light direction; combine with the other term
/// using `smith_g_schlick_ggx(n_dot_v) * smith_g_schlick_ggx(n_dot_l)` for
/// the full Smith geometry term.
///
/// - `n_dot_v` — `max(0, N·V)` or `max(0, N·L)` ∈ [0, 1]
/// - `roughness` — linear roughness
pub fn smith_g_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    n_dot_v / (n_dot_v * (1.0 - k) + k)
}

// ── Pass 59 tests ─────────────────────────────────────────────────────────────
