//! Hold — constant value for a duration (pause in sequences).

use abrash_core::animatable::Animatable;

use crate::evaluable::Sample;

/// A segment that holds a constant value for a given duration.
///
/// Always returns `Sample::at_rest(value)` — zero velocity.
pub struct Hold<T: Animatable> {
    /// The constant value held during this segment.
    pub value: T,
    /// The duration (in seconds) to hold the value.
    pub duration: f32,
}

impl<T: Animatable> Hold<T> {
    /// Locks a value in place for a specified duration, effectively creating a "pause"
    /// or static keyframe within a complex animation sequence.
    ///
    /// ## Examples
    /// ```
    /// use abrash_anim::Hold;
    /// let hold = Hold::new(10.0_f32, 2.0);
    /// assert_eq!(hold.duration, 2.0);
    /// ```
    #[must_use]
    pub const fn new(value: T, duration: f32) -> Self {
        Self { value, duration }
    }
}

impl<T: Animatable + Send + Sync> Hold<T> {
    /// Samples the timeline during the pause. Because a hold implies no movement,
    /// the returned sample is always perfectly at rest (zero velocity) regardless
    /// of the current phase.
    pub fn evaluate(&self, _phase: f32) -> Sample<T> {
        Sample::at_rest(self.value.clone())
    }

    /// The natural duration in seconds of this segment.
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
