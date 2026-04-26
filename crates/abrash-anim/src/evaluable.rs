//! Core animation evaluation trait and sample type.

use abrash_core::animatable::Animatable;

use crate::easing::Easing;

/// A value paired with its instantaneous velocity.
///
/// Every `Evaluable` returns both value and velocity, enabling
/// future velocity-preserving spring interruption.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample<T: Animatable> {
    pub value: T,
    pub velocity: T,
}

impl<T: Animatable> Sample<T> {
    /// Create a sample with a value and velocity.
    #[must_use]
    pub const fn new(value: T, velocity: T) -> Self {
        Self { value, velocity }
    }

    /// Create a sample at rest (zero velocity).
    #[must_use]
    pub fn at_rest(value: T) -> Self {
        Self {
            value,
            velocity: T::zero(),
        }
    }
}

/// A tween segment that interpolates from one value to another with easing.
///
/// Velocity is derived analytically from the easing function's derivative.
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

        let deriv = self.easing.derivative(phase);
        let delta = self.to.anim_sub(&self.from);
        let velocity = if self.duration > f32::EPSILON {
            delta.anim_scale(deriv / self.duration)
        } else {
            T::zero()
        };

        Sample::new(value, velocity)
    }

    #[must_use]
    pub const fn natural_duration(&self) -> f32 {
        self.duration
    }
}

/// A segment that holds a constant value for a given duration.
///
/// Always returns `Sample::at_rest(value)` — zero velocity.
pub struct Hold<T: Animatable> {
    pub value: T,
    pub duration: f32,
}

impl<T: Animatable> Hold<T> {
    #[must_use]
    pub const fn new(value: T, duration: f32) -> Self {
        Self { value, duration }
    }
}

impl<T: Animatable + Send + Sync> Hold<T> {
    pub fn evaluate(&self, _phase: f32) -> Sample<T> {
        Sample::at_rest(self.value.clone())
    }

    #[must_use]
    pub const fn natural_duration(&self) -> f32 {
        self.duration
    }
}

/// Chains multiple `Evaluable` segments end-to-end.
///
/// Each child gets a proportional slice of the 0.0--1.0 phase range
/// based on `natural_duration() / total_duration`.
pub struct Sequence<T: Animatable> {
    segments: Vec<Evaluable<T>>,
    boundaries: Vec<(f32, f32)>,
    total_duration: f32,
}

impl<T: Animatable + Send + Sync> Sequence<T> {
    /// Create a sequence from a list of evaluable segments.
    ///
    /// # Panics
    /// Panics if `segments` is empty.
    #[must_use]
    pub fn new(segments: Vec<Evaluable<T>>) -> Self {
        assert!(
            !segments.is_empty(),
            "Sequence requires at least one segment"
        );

        let total_duration: f32 = segments.iter().map(Evaluable::natural_duration).sum();
        let mut boundaries = Vec::with_capacity(segments.len());
        let mut cursor = 0.0_f32;

        for seg in &segments {
            let proportion = if total_duration > f32::EPSILON {
                seg.natural_duration() / total_duration
            } else {
                1.0 / segments.len() as f32
            };
            boundaries.push((cursor, cursor + proportion));
            cursor += proportion;
        }

        Self {
            segments,
            boundaries,
            total_duration,
        }
    }
}

impl<T: Animatable + Send + Sync> Sequence<T> {
    #[must_use]
    pub fn evaluate(&self, phase: f32) -> Sample<T> {
        let phase = phase.clamp(0.0, 1.0);

        for (i, &(start, end)) in self.boundaries.iter().enumerate() {
            if phase < end || i == self.segments.len() - 1 {
                let span = end - start;
                let local_phase = if span > f32::EPSILON {
                    ((phase - start) / span).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                return self.segments[i].evaluate(local_phase);
            }
        }

        unreachable!(
            "The loop always returns because the last segment condition `i == self.segments.len() - 1` is always met"
        )
    }

    #[must_use]
    pub const fn natural_duration(&self) -> f32 {
        self.total_duration
    }
}

// Safety: `Evaluable` trait requires `Send + Sync`, so all boxed segments are `Send + Sync`.
unsafe impl<T: Animatable + Send + Sync> Send for Sequence<T> {}
unsafe impl<T: Animatable + Send + Sync> Sync for Sequence<T> {}

/// A pure function from normalized phase (0.0–1.0) to a `Sample<T>`.
///
/// This is the core animation abstraction. Evaluables are composable:
/// `Keyframe`, `Hold`, and `Sequence` are available variants.
pub enum Evaluable<T: Animatable> {
    Keyframe(Keyframe<T>),
    Hold(Hold<T>),
    Sequence(Sequence<T>),
}

impl<T: Animatable + Send + Sync> Evaluable<T> {
    /// Evaluate the animation at a normalized phase (0.0–1.0).
    #[must_use]
    pub fn evaluate(&self, phase: f32) -> Sample<T> {
        match self {
            Self::Keyframe(k) => k.evaluate(phase),
            Self::Hold(h) => h.evaluate(phase),
            Self::Sequence(s) => s.evaluate(phase),
        }
    }

    /// Preferred real-time duration in seconds.
    ///
    /// Used by composition types (e.g. `Sequence`) to allocate proportional
    /// phase ranges.
    #[must_use]
    pub const fn natural_duration(&self) -> f32 {
        match self {
            Self::Keyframe(k) => k.natural_duration(),
            Self::Hold(h) => h.natural_duration(),
            Self::Sequence(s) => s.natural_duration(),
        }
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

    #[test]
    fn two_equal_segments_split_evenly() {
        let seq = Sequence::new(vec![
            Evaluable::Keyframe(Keyframe::new(0.0_f32, 10.0, Easing::Linear, 1.0)),
            Evaluable::Keyframe(Keyframe::new(10.0_f32, 20.0, Easing::Linear, 1.0)),
        ]);

        let s = seq.evaluate(0.0);
        assert!((s.value).abs() < EPSILON);

        let s = seq.evaluate(0.25);
        assert!((s.value - 5.0).abs() < EPSILON);

        let s = seq.evaluate(0.5);
        assert!((s.value - 10.0).abs() < EPSILON);

        let s = seq.evaluate(0.75);
        assert!((s.value - 15.0).abs() < EPSILON);

        let s = seq.evaluate(1.0);
        assert!((s.value - 20.0).abs() < EPSILON);
    }

    #[test]
    fn unequal_durations_proportional() {
        // 1s tween + 3s hold = 4s total
        // Tween occupies 0.0..0.25, hold occupies 0.25..1.0
        let seq = Sequence::new(vec![
            Evaluable::Keyframe(Keyframe::new(0.0_f32, 10.0, Easing::Linear, 1.0)),
            Evaluable::Hold(Hold::new(10.0_f32, 3.0)),
        ]);

        // phase 0.125 is midpoint of first segment (0.0..0.25)
        let s = seq.evaluate(0.125);
        assert!((s.value - 5.0).abs() < EPSILON);

        // phase 0.5 is inside the hold segment
        let s = seq.evaluate(0.5);
        assert!((s.value - 10.0).abs() < EPSILON);
    }

    #[test]
    fn total_duration_is_sum() {
        let seq = Sequence::new(vec![
            Evaluable::Keyframe(Keyframe::new(0.0_f32, 10.0, Easing::Linear, 2.0)),
            Evaluable::Hold(Hold::new(10.0_f32, 3.0)),
        ]);
        assert!((seq.natural_duration() - 5.0).abs() < EPSILON);
    }

    #[test]
    fn single_segment_sequence() {
        let seq = Sequence::new(vec![Evaluable::Keyframe(Keyframe::new(
            0.0_f32,
            10.0,
            Easing::Linear,
            1.0,
        ))]);
        let s = seq.evaluate(0.5);
        assert!((s.value - 5.0).abs() < EPSILON);
    }

    #[test]
    #[should_panic(
        expected = "The loop always returns because the last segment condition `i == self.segments.len() - 1` is always met"
    )]
    fn sequence_evaluate_unreachable_guard() {
        let mut seq = Sequence::new(vec![Evaluable::Keyframe(Keyframe::new(
            0.0_f32,
            10.0,
            Easing::Linear,
            1.0,
        ))]);

        // Manually break the internal invariant: clear the boundaries list
        // so the exhaustive for-loop finishes without returning.
        seq.boundaries.clear();

        // This should trigger the `unreachable!()` guard.
        let _ = seq.evaluate(0.5);
    }
}
