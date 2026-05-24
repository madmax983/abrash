//! Easing functions with analytical derivatives for velocity computation.

/// Standard easing curves.
#[derive(Debug, Clone, Copy)]
pub enum Easing {
    /// Linear interpolation.
    ///
    /// ## Examples
    /// ```
    /// use abrash_anim::Easing;
    /// assert_eq!(Easing::Linear.apply(0.5), 0.5);
    /// ```
    Linear,
    /// Gentle acceleration curve.
    ///
    /// ## Examples
    /// ```
    /// use abrash_anim::Easing;
    /// assert!(Easing::EaseIn.apply(0.5) < 0.5);
    /// ```
    EaseIn,
    /// Gentle deceleration curve.
    ///
    /// ## Examples
    /// ```
    /// use abrash_anim::Easing;
    /// assert!(Easing::EaseOut.apply(0.5) > 0.5);
    /// ```
    EaseOut,
    /// Smooth acceleration followed by deceleration.
    ///
    /// ## Examples
    /// ```
    /// use abrash_anim::Easing;
    /// assert_eq!(Easing::EaseInOut.apply(0.5), 0.5);
    /// ```
    EaseInOut,
    /// Cubic Bezier curve with control points (x1, y1, x2, y2).
    CubicBezier(f32, f32, f32, f32),
}

impl Easing {
    /// Apply the easing function to a linear parameter t (0.0–1.0).
    #[must_use]
    pub fn apply(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::EaseIn => t * t,
            Self::EaseOut => t * (2.0 - t),
            Self::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
            Self::CubicBezier(_x1, y1, _x2, y2) => {
                let t2 = t * t;
                let t3 = t2 * t;
                3.0 * (1.0 - t) * (1.0 - t) * t * y1 + 3.0 * (1.0 - t) * t2 * y2 + t3
            }
        }
    }

    /// Instantaneous rate of change (derivative) of the easing function.
    ///
    /// Used to compute velocity in `Sample<T>`.
    #[must_use]
    pub fn derivative(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => 1.0,
            Self::EaseIn => 2.0 * t,
            Self::EaseOut => 2.0 - 2.0 * t,
            Self::EaseInOut => {
                if t < 0.5 {
                    4.0 * t
                } else {
                    4.0 - 4.0 * t
                }
            }
            Self::CubicBezier(..) => {
                // Numerical derivative via central finite difference
                let h = 0.0001;
                let t0 = (t - h).max(0.0);
                let t1 = (t + h).min(1.0);
                let dt = t1 - t0;
                if dt < f32::EPSILON {
                    return 0.0;
                }
                (self.apply(t1) - self.apply(t0)) / dt
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-4;

    #[test]
    fn all_easings_start_at_zero() {
        for easing in [
            Easing::Linear,
            Easing::EaseIn,
            Easing::EaseOut,
            Easing::EaseInOut,
        ] {
            assert!(
                easing.apply(0.0).abs() < EPSILON,
                "{easing:?} failed at 0.0"
            );
        }
    }

    #[test]
    fn all_easings_end_at_one() {
        for easing in [
            Easing::Linear,
            Easing::EaseIn,
            Easing::EaseOut,
            Easing::EaseInOut,
        ] {
            assert!(
                (easing.apply(1.0) - 1.0).abs() < EPSILON,
                "{easing:?} failed at 1.0"
            );
        }
    }

    #[test]
    fn linear_is_identity() {
        assert!((Easing::Linear.apply(0.5) - 0.5).abs() < EPSILON);
        assert!((Easing::Linear.apply(0.25) - 0.25).abs() < EPSILON);
    }

    #[test]
    fn ease_in_is_slow_start() {
        // EaseIn at 0.5 should be < 0.5 (starts slow)
        assert!(Easing::EaseIn.apply(0.5) < 0.5);
    }

    #[test]
    fn ease_out_is_fast_start() {
        // EaseOut at 0.5 should be > 0.5 (starts fast)
        assert!(Easing::EaseOut.apply(0.5) > 0.5);
    }

    #[test]
    fn linear_derivative_is_constant() {
        assert!((Easing::Linear.derivative(0.0) - 1.0).abs() < EPSILON);
        assert!((Easing::Linear.derivative(0.5) - 1.0).abs() < EPSILON);
        assert!((Easing::Linear.derivative(1.0) - 1.0).abs() < EPSILON);
    }

    #[test]
    fn ease_in_derivative_starts_at_zero() {
        assert!(Easing::EaseIn.derivative(0.0).abs() < EPSILON);
    }

    #[test]
    fn ease_out_derivative_ends_at_zero() {
        assert!(Easing::EaseOut.derivative(1.0).abs() < EPSILON);
    }

    #[test]
    fn cubic_bezier_boundaries() {
        let cb = Easing::CubicBezier(0.25, 0.1, 0.25, 1.0);
        assert!(cb.apply(0.0).abs() < EPSILON);
        assert!((cb.apply(1.0) - 1.0).abs() < EPSILON);
    }
}
