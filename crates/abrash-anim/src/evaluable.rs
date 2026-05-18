//! Core animation evaluation trait and sample type.

use abrash_core::animatable::Animatable;

use crate::hold::Hold;
use crate::keyframe::Keyframe;
use crate::sequence::Sequence;

/// A snapshot in time containing both an animation's position and its exact velocity.
///
/// A fundamental requirement for fluid UI animation is that interruptions (like user input)
/// shouldn't look jarring. By having every `[`Evaluable`]` return both a value and a
/// velocity, we can hand off animations to a physics spring system seamlessly,
/// preserving their momentum.
///
/// # Examples
///
/// ```
/// use abrash_anim::evaluable::Sample;
///
/// // Create a sample of an object moving at 10 units/sec, currently at position 50.
/// let moving_sample = Sample::new(50.0_f32, 10.0);
///
/// // Create a sample for a resting object.
/// let resting_sample = Sample::at_rest(50.0_f32);
/// assert_eq!(resting_sample.velocity, 0.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample<T: Animatable> {
    /// The actual evaluated value at this instant.
    pub value: T,
    /// The instantaneous velocity (rate of change) at this instant.
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

/// A pure function from normalized phase (0.0–1.0) to a `Sample<T>`.
///
/// This is the core animation abstraction. Evaluables are composable:
/// `Keyframe`, `Hold`, and `Sequence` are available variants.
pub enum Evaluable<T: Animatable> {
    /// A segment that transitions from one value to another with an easing curve.
    Keyframe(Keyframe<T>),
    /// A segment that maintains a constant value for a duration.
    Hold(Hold<T>),
    /// A segment composed of multiple ordered evaluable segments.
    Sequence(Sequence<T>),
}

impl<T: Animatable + Send + Sync> Evaluable<T> {
    /// Evaluate the animation at a normalized phase (0.0–1.0).
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
    pub const fn natural_duration(&self) -> f32 {
        match self {
            Self::Keyframe(k) => k.natural_duration(),
            Self::Hold(h) => h.natural_duration(),
            Self::Sequence(s) => s.natural_duration(),
        }
    }
}
