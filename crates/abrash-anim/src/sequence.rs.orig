//! Sequence — proportional end-to-end chaining of evaluable segments.

use abrash_core::animatable::Animatable;

use crate::evaluable::{Evaluable, Sample};

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

        let total_duration: f32 = segments.iter().map(|s| s.natural_duration()).sum();
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

    pub fn natural_duration(&self) -> f32 {
        self.total_duration
    }
}

// Safety: `Evaluable` trait requires `Send + Sync`, so all boxed segments are `Send + Sync`.
unsafe impl<T: Animatable> Send for Sequence<T> {}
unsafe impl<T: Animatable> Sync for Sequence<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::easing::Easing;
    use crate::hold::Hold;
    use crate::keyframe::Keyframe;

    const EPSILON: f32 = 1e-4;

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
