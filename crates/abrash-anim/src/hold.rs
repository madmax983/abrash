//! Hold — constant value for a duration (pause in sequences).

use abrash_core::animatable::Animatable;

use crate::evaluable::Sample;

/// A segment that holds a constant value for a given duration.
///
/// Always returns `Sample::at_rest(value)` — zero velocity.
pub struct Hold<T: Animatable> {
    /// The constant value held for the duration.
    pub value: T,
    /// The duration in seconds this hold occupies in natural time.
    pub duration: f32,
}

impl<T: Animatable> Hold<T> {
    /// Create a new hold segment with the specified value and duration.
    #[must_use]
    pub const fn new(value: T, duration: f32) -> Self {
        Self { value, duration }
    }
}

impl<T: Animatable + Send + Sync> Hold<T> {
    /// Evaluate the hold at a given normalized phase (0.0-1.0).
    /// Always returns the constant value at rest.
    pub fn evaluate(&self, _phase: f32) -> Sample<T> {
        Sample::at_rest(self.value.clone())
    }

    /// The natural length of this hold segment in seconds.
    pub const fn natural_duration(&self) -> f32 {
        self.duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hold_always_returns_same_value() {
        let h = Hold::new(42.0_f32, 1.0);
        for phase in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let s = h.evaluate(phase);
            assert!((s.value - 42.0).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn hold_velocity_is_zero() {
        let h = Hold::new(42.0_f32, 1.0);
        let s = h.evaluate(0.5);
        assert!(s.velocity.abs() < f32::EPSILON);
    }

    #[test]
    fn hold_natural_duration() {
        let h = Hold::new(0.0_f32, 3.0);
        assert!((h.natural_duration() - 3.0).abs() < f32::EPSILON);
    }
}
