//! Keyframe — tween between two values with easing.

use abrash_core::animatable::Animatable;

use crate::easing::Easing;
use crate::evaluable::Sample;

/// A tween segment that interpolates from one value to another with easing.
///
pub struct Keyframe<T: Animatable> {
    pub from: T,
    pub to: T,
    pub easing: Easing,
    pub duration: f32,
}

impl<T: Animatable> Keyframe<T> {
    #[must_use]
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
    pub fn evaluate(&self, phase: f32) -> Sample<T> {
        let phase = phase.clamp(0.0, 1.0);
        let eased = self.easing.apply(phase);
        let value = self.from.interpolate(&self.to, eased);

        Sample::new(value)
    }

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
    fn natural_duration_matches() {
        let kf = Keyframe::new(0.0_f32, 10.0, Easing::Linear, 2.5);
        assert!((kf.natural_duration() - 2.5).abs() < EPSILON);
    }
}
