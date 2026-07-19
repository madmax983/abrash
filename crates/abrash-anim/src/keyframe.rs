//! Keyframe — tween between two values with easing.

use abrash_core::animatable::Animatable;

use crate::easing::Easing;
use crate::evaluable::Sample;

/// A tween segment that interpolates from one value to another with easing.
///
/// Velocity is derived analytically from the easing function's derivative.
pub struct Keyframe<T: Animatable> {
    /// The starting value of the keyframe.
    pub from: T,
    /// The ending value of the keyframe.
    pub to: T,
    /// The interpolation curve to use between `from` and `to`.
    pub easing: Easing,
    /// The duration this keyframe spans.
    pub duration: f32,
}

impl<T: Animatable> Keyframe<T> {
    #[must_use]
    /// Constructs a new `Keyframe` interpolation spanning `from` to `to` over `duration` using the given `easing` curve.
    pub const fn new(from: T, to: T, easing: Easing, duration: f32) -> Self {
        Self {
            from,
            to,
            easing,
            duration,
        }
    }
}

impl<T: Animatable + Send + Sync> Keyframe<T> {
    /// Computes the current value and velocity at the specified `phase` along the keyframe.
    pub fn evaluate(&self, phase: f32) -> Sample<T> {
        let phase = phase.clamp(0.0, 1.0);
        let eased = self.easing.apply(phase);
        let value = self.from.interpolate(&self.to, eased);

        let deriv = self.easing.derivative(phase);
        let delta = self.to.anim_sub(&self.from);
        let velocity = if self.duration > f32::EPSILON {
            delta.anim_scale(deriv / self.duration)
        } else {
            T::zero()
        };

        Sample::new(value, velocity)
    }

    /// Returns the specified duration of the keyframe.
    pub const fn natural_duration(&self) -> f32 {
        self.duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    #[test]
    fn evaluate_at_zero_returns_from() {
        let kf = Keyframe::new(0.0_f32, 10.0, Easing::Linear, 1.0);
        let s = kf.evaluate(0.0);
        assert!((s.value).abs() < EPSILON);
    }

    #[test]
    fn evaluate_at_one_returns_to() {
        let kf = Keyframe::new(0.0_f32, 10.0, Easing::Linear, 1.0);
        let s = kf.evaluate(1.0);
        assert!((s.value - 10.0).abs() < EPSILON);
    }

    #[test]
    fn evaluate_midpoint_linear() {
        let kf = Keyframe::new(0.0_f32, 10.0, Easing::Linear, 1.0);
        let s = kf.evaluate(0.5);
        assert!((s.value - 5.0).abs() < EPSILON);
    }

    #[test]
    fn velocity_nonzero_at_midpoint() {
        let kf = Keyframe::new(0.0_f32, 10.0, Easing::Linear, 1.0);
        let s = kf.evaluate(0.5);
        assert!(s.velocity.abs() > EPSILON);
    }

    #[test]
    fn velocity_is_zero_at_ease_in_start() {
        let kf = Keyframe::new(0.0_f32, 10.0, Easing::EaseIn, 1.0);
        let s = kf.evaluate(0.0);
        assert!(s.velocity.abs() < EPSILON);
    }

    #[test]
    fn natural_duration_matches() {
        let kf = Keyframe::new(0.0_f32, 10.0, Easing::Linear, 2.5);
        assert!((kf.natural_duration() - 2.5).abs() < EPSILON);
    }
}
