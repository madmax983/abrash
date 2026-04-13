//! Easing functions for smooth interpolation.
//!
//! All functions take `t ∈ [0, 1]` and return a value in approximately `[0, 1]`.
//! Some functions (e.g. elastic, back) overshoot outside `[0, 1]`.
//!
//! # Naming convention
//!
//! - `*_in`     — starts slow, ends fast
//! - `*_out`    — starts fast, ends slow
//! - `*_in_out` — starts and ends slow, fast through the middle (symmetric)
//!
//! # Families
//!
//! | Family   | Character                                    |
//! |----------|----------------------------------------------|
//! | `linear` | Constant rate — no easing                    |
//! | `quad`   | t² / quadratic — gentle                      |
//! | `cubic`  | t³ / cubic — moderate                        |
//! | `quart`  | t⁴ / quartic — strong                        |
//! | `quint`  | t⁵ / quintic — very strong                   |
//! | `sine`   | Sinusoidal — smooth, natural                  |
//! | `expo`   | Exponential 2ⁿ — dramatic                    |
//! | `circ`   | Circular arc — smooth start/end               |
//! | `back`   | Slight overshoot (configurable via `s`)       |
//! | `elastic`| Spring-like oscillation                       |
//! | `bounce` | Bouncing ball simulation                      |
//!
//! # Examples
//!
//! ```
//! use abrash_core::easing::{cubic_in_out, elastic_out};
//!
//! let t = 0.3_f32;
//! let smooth = cubic_in_out(t);
//! assert!(smooth >= 0.0 && smooth <= 1.0);
//!
//! // Elastic overshoots — may exceed [0,1]
//! let spring = elastic_out(0.5);
//! ```

// ── Linear ────────────────────────────────────────────────────────────────────

/// No easing — constant rate.
#[must_use]
#[inline]
pub const fn linear(t: f32) -> f32 {
    t
}

// ── Quadratic ─────────────────────────────────────────────────────────────────

/// Quadratic ease-in: starts slow, ends fast.
#[must_use]
#[inline]
pub const fn quad_in(t: f32) -> f32 {
    t * t
}

/// Quadratic ease-out: starts fast, ends slow.
#[must_use]
#[inline]
pub const fn quad_out(t: f32) -> f32 {
    t * (2.0 - t)
}

/// Quadratic ease-in-out: symmetric slow-fast-slow.
#[must_use]
#[inline]
pub const fn quad_in_out(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        -1.0 + (4.0 - 2.0 * t) * t
    }
}

// ── Cubic ─────────────────────────────────────────────────────────────────────

/// Cubic ease-in.
#[must_use]
#[inline]
pub const fn cubic_in(t: f32) -> f32 {
    t * t * t
}

/// Cubic ease-out.
#[must_use]
#[inline]
pub const fn cubic_out(t: f32) -> f32 {
    let t1 = t - 1.0;
    t1 * t1 * t1 + 1.0
}

/// Cubic ease-in-out.
#[must_use]
#[inline]
pub const fn cubic_in_out(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let t1 = 2.0 * t - 2.0;
        0.5 * t1 * t1 * t1 + 1.0
    }
}

// ── Quartic ───────────────────────────────────────────────────────────────────

/// Quartic ease-in.
#[must_use]
#[inline]
pub const fn quart_in(t: f32) -> f32 {
    t * t * t * t
}

/// Quartic ease-out.
#[must_use]
#[inline]
pub const fn quart_out(t: f32) -> f32 {
    let t1 = t - 1.0;
    1.0 - t1 * t1 * t1 * t1
}

/// Quartic ease-in-out.
#[must_use]
#[inline]
pub const fn quart_in_out(t: f32) -> f32 {
    if t < 0.5 {
        8.0 * t * t * t * t
    } else {
        let t1 = t - 1.0;
        1.0 - 8.0 * t1 * t1 * t1 * t1
    }
}

// ── Quintic ───────────────────────────────────────────────────────────────────

/// Quintic ease-in.
#[must_use]
#[inline]
pub const fn quint_in(t: f32) -> f32 {
    t * t * t * t * t
}

/// Quintic ease-out.
#[must_use]
#[inline]
pub const fn quint_out(t: f32) -> f32 {
    let t1 = t - 1.0;
    t1 * t1 * t1 * t1 * t1 + 1.0
}

/// Quintic ease-in-out.
#[must_use]
#[inline]
pub const fn quint_in_out(t: f32) -> f32 {
    if t < 0.5 {
        16.0 * t * t * t * t * t
    } else {
        let t1 = 2.0 * t - 2.0;
        0.5 * t1 * t1 * t1 * t1 * t1 + 1.0
    }
}

// ── Sine ──────────────────────────────────────────────────────────────────────

/// Sine ease-in.
#[must_use]
#[inline]
pub fn sine_in(t: f32) -> f32 {
    use std::f32::consts::FRAC_PI_2;
    1.0 - (t * FRAC_PI_2).cos()
}

/// Sine ease-out.
#[must_use]
#[inline]
pub fn sine_out(t: f32) -> f32 {
    use std::f32::consts::FRAC_PI_2;
    (t * FRAC_PI_2).sin()
}

/// Sine ease-in-out.
#[must_use]
#[inline]
pub fn sine_in_out(t: f32) -> f32 {
    use std::f32::consts::PI;
    0.5 * (1.0 - (PI * t).cos())
}

// ── Exponential ───────────────────────────────────────────────────────────────

/// Exponential ease-in (2^(10t-10)). Returns exactly 0 at t=0.
#[must_use]
#[inline]
pub fn expo_in(t: f32) -> f32 {
    if t <= 0.0 {
        0.0
    } else {
        (2.0_f32).powf(10.0 * t - 10.0)
    }
}

/// Exponential ease-out. Returns exactly 1 at t=1.
#[must_use]
#[inline]
pub fn expo_out(t: f32) -> f32 {
    if t >= 1.0 {
        1.0
    } else {
        1.0 - (2.0_f32).powf(-10.0 * t)
    }
}

/// Exponential ease-in-out.
#[must_use]
#[inline]
pub fn expo_in_out(t: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    if t < 0.5 {
        0.5 * (2.0_f32).powf(20.0 * t - 10.0)
    } else {
        0.5 * (2.0 - (2.0_f32).powf(-20.0 * t + 10.0))
    }
}

// ── Circular ──────────────────────────────────────────────────────────────────

/// Circular ease-in.
#[must_use]
#[inline]
pub fn circ_in(t: f32) -> f32 {
    1.0 - (1.0 - t * t).max(0.0).sqrt()
}

/// Circular ease-out.
#[must_use]
#[inline]
pub fn circ_out(t: f32) -> f32 {
    let t1 = t - 1.0;
    (1.0 - t1 * t1).max(0.0).sqrt()
}

/// Circular ease-in-out.
#[must_use]
#[inline]
pub fn circ_in_out(t: f32) -> f32 {
    if t < 0.5 {
        0.5 * (1.0 - (1.0 - 4.0 * t * t).max(0.0).sqrt())
    } else {
        let t1 = 2.0 * t - 2.0;
        0.5 * ((1.0 - t1 * t1).max(0.0).sqrt() + 1.0)
    }
}

// ── Back (overshoot) ──────────────────────────────────────────────────────────

/// Back ease-in with configurable overshoot `s` (default 1.70158).
///
/// The curve dips slightly below 0 before rising to 1.
#[must_use]
#[inline]
pub fn back_in(t: f32, s: f32) -> f32 {
    t * t * ((s + 1.0) * t - s)
}

/// Back ease-out with configurable overshoot `s`.
#[must_use]
#[inline]
pub fn back_out(t: f32, s: f32) -> f32 {
    let t1 = t - 1.0;
    t1 * t1 * ((s + 1.0) * t1 + s) + 1.0
}

/// Back ease-in-out with configurable overshoot `s`.
#[must_use]
#[inline]
pub fn back_in_out(t: f32, s: f32) -> f32 {
    let s1 = s * 1.525;
    if t < 0.5 {
        let t1 = 2.0 * t;
        0.5 * t1 * t1 * ((s1 + 1.0) * t1 - s1)
    } else {
        let t1 = 2.0 * t - 2.0;
        0.5 * (t1 * t1 * ((s1 + 1.0) * t1 + s1) + 2.0)
    }
}

/// Default overshoot constant for `back_*` functions (Penner's original value).
pub const BACK_DEFAULT_S: f32 = 1.70158;

// ── Elastic ───────────────────────────────────────────────────────────────────

/// Elastic ease-in — spring oscillation that settles at 1.
///
/// May produce values well below 0.
#[must_use]
#[inline]
pub fn elastic_in(t: f32) -> f32 {
    use std::f32::consts::TAU;
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let c4 = TAU / 3.0;
    -(2.0_f32).powf(10.0 * t - 10.0) * ((10.0 * t - 10.75) * c4).sin()
}

/// Elastic ease-out — spring oscillation settling from 0.
///
/// May produce values above 1.
#[must_use]
#[inline]
pub fn elastic_out(t: f32) -> f32 {
    use std::f32::consts::TAU;
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let c4 = TAU / 3.0;
    (2.0_f32).powf(-10.0 * t) * ((10.0 * t - 0.75) * c4).sin() + 1.0
}

/// Elastic ease-in-out.
#[must_use]
#[inline]
pub fn elastic_in_out(t: f32) -> f32 {
    use std::f32::consts::TAU;
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let c5 = TAU / 4.5;
    if t < 0.5 {
        -0.5 * (2.0_f32).powf(20.0 * t - 10.0) * ((20.0 * t - 11.125) * c5).sin()
    } else {
        0.5 * (2.0_f32).powf(-20.0 * t + 10.0) * ((20.0 * t - 11.125) * c5).sin() + 1.0
    }
}

// ── Bounce ────────────────────────────────────────────────────────────────────

/// Bounce ease-out — simulates a ball bouncing to rest.
#[must_use]
#[inline]
pub fn bounce_out(t: f32) -> f32 {
    const N1: f32 = 7.5625;
    const D1: f32 = 2.75;

    if t < 1.0 / D1 {
        N1 * t * t
    } else if t < 2.0 / D1 {
        let t1 = t - 1.5 / D1;
        N1 * t1 * t1 + 0.75
    } else if t < 2.5 / D1 {
        let t1 = t - 2.25 / D1;
        N1 * t1 * t1 + 0.9375
    } else {
        let t1 = t - 2.625 / D1;
        N1 * t1 * t1 + 0.984_375
    }
}

/// Bounce ease-in.
#[must_use]
#[inline]
pub fn bounce_in(t: f32) -> f32 {
    1.0 - bounce_out(1.0 - t)
}

/// Bounce ease-in-out.
#[must_use]
#[inline]
pub fn bounce_in_out(t: f32) -> f32 {
    if t < 0.5 {
        0.5 * (1.0 - bounce_out(1.0 - 2.0 * t))
    } else {
        0.5 * bounce_out(2.0 * t - 1.0) + 0.5
    }
}

// ── Utility ───────────────────────────────────────────────────────────────────

/// Apply an easing function and linearly interpolate between `a` and `b`.
#[must_use]
#[inline]
pub fn ease_lerp(a: f32, b: f32, t: f32, ease: fn(f32) -> f32) -> f32 {
    a + (b - a) * ease(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOL: f32 = 1e-4;

    fn check_endpoints(f: fn(f32) -> f32) {
        assert!((f(0.0)).abs() < TOL, "f(0) should be ~0, got {}", f(0.0));
        assert!(
            (f(1.0) - 1.0).abs() < TOL,
            "f(1) should be ~1, got {}",
            f(1.0)
        );
    }

    fn check_monotone_approx(f: fn(f32) -> f32) {
        // Verify generally increasing (not strictly, elastic/bounce are not)
        let start = f(0.0);
        let end = f(1.0);
        assert!(start <= end + TOL, "should trend from 0 to 1");
    }

    #[test]
    fn linear_endpoints() {
        check_endpoints(linear);
        assert!((linear(0.5) - 0.5).abs() < TOL);
    }

    #[test]
    fn polynomial_endpoints() {
        for f in [
            quad_in as fn(f32) -> f32,
            quad_out,
            quad_in_out,
            cubic_in,
            cubic_out,
            cubic_in_out,
            quart_in,
            quart_out,
            quart_in_out,
            quint_in,
            quint_out,
            quint_in_out,
        ] {
            check_endpoints(f);
        }
    }

    #[test]
    fn sine_endpoints() {
        for f in [sine_in as fn(f32) -> f32, sine_out, sine_in_out] {
            check_endpoints(f);
        }
    }

    #[test]
    fn expo_endpoints() {
        check_endpoints(expo_in);
        check_endpoints(expo_out);
        check_endpoints(expo_in_out);
    }

    #[test]
    fn circ_endpoints() {
        for f in [circ_in as fn(f32) -> f32, circ_out, circ_in_out] {
            check_endpoints(f);
        }
    }

    #[test]
    fn elastic_endpoints() {
        for f in [elastic_in as fn(f32) -> f32, elastic_out, elastic_in_out] {
            check_endpoints(f);
        }
    }

    #[test]
    fn bounce_endpoints() {
        for f in [bounce_in as fn(f32) -> f32, bounce_out, bounce_in_out] {
            check_endpoints(f);
        }
    }

    #[test]
    fn back_endpoints() {
        let s = BACK_DEFAULT_S;
        assert!((back_in(0.0, s)).abs() < TOL);
        assert!((back_in(1.0, s) - 1.0).abs() < TOL);
        assert!((back_out(0.0, s)).abs() < TOL);
        assert!((back_out(1.0, s) - 1.0).abs() < TOL);
        assert!((back_in_out(0.0, s)).abs() < TOL);
        assert!((back_in_out(1.0, s) - 1.0).abs() < TOL);
    }

    #[test]
    fn back_overshoots() {
        // back_in should dip below 0 near the start
        let s = BACK_DEFAULT_S;
        let has_dip = (0..50).any(|i| back_in(i as f32 / 100.0, s) < -1e-3);
        assert!(has_dip, "back_in should dip below 0");
    }

    #[test]
    fn elastic_overshoots() {
        // elastic_out should exceed 1.0 somewhere
        let exceeds = (1..99).any(|i| elastic_out(i as f32 / 100.0) > 1.001);
        assert!(exceeds, "elastic_out should overshoot above 1");
    }

    #[test]
    fn bounce_all_non_negative() {
        // Bounce should stay in [0, 1]
        for i in 0..=100 {
            let t = i as f32 / 100.0;
            let v = bounce_out(t);
            assert!(
                v >= -TOL && v <= 1.0 + TOL,
                "bounce_out({t}) = {v} out of range"
            );
        }
    }

    #[test]
    fn in_out_is_symmetric() {
        // in_out(t) + in_out(1-t) should equal 1 for symmetric functions
        for f in [
            quad_in_out as fn(f32) -> f32,
            cubic_in_out,
            quart_in_out,
            quint_in_out,
        ] {
            for i in 1..=9 {
                let t = i as f32 / 10.0;
                let sum = f(t) + f(1.0 - t);
                assert!(
                    (sum - 1.0).abs() < TOL,
                    "f({t}) + f({}) = {sum} ≠ 1",
                    1.0 - t
                );
            }
        }
    }

    #[test]
    fn ease_lerp_basic() {
        let v = ease_lerp(0.0, 10.0, 0.5, cubic_in_out);
        assert!(v > 0.0 && v < 10.0);
    }

    #[test]
    fn quad_in_out_midpoint() {
        // Symmetric: midpoint should be exactly 0.5
        assert!((quad_in_out(0.5) - 0.5).abs() < TOL);
    }

    #[test]
    fn monotone_families() {
        for f in [
            quad_in as fn(f32) -> f32,
            quad_out,
            cubic_in,
            cubic_out,
            sine_in,
            sine_out,
            expo_in,
            expo_out,
            circ_in,
            circ_out,
            bounce_out,
        ] {
            check_monotone_approx(f);
        }
    }
}
