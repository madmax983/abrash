//! Core animation evaluation trait and sample type.

use abrash_core::animatable::Animatable;

use crate::hold::Hold;
use crate::keyframe::Keyframe;
use crate::sequence::Sequence;

/// A value paired with its instantaneous velocity.
///
/// Every `Evaluable` returns both value and velocity, enabling
/// future velocity-preserving spring interruption.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample<T: Animatable> {
    /// The actual evaluated property value at this moment in time.
    pub value: T,
    /// The instantaneous velocity (rate of change) of the value.
    ///
    /// Essential for transferring momentum when interrupting animations.
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
    /// A single tween moving from one state to another with a defined curve.
    ///
    /// ## Examples
    /// ```
    /// use abrash_anim::{Evaluable, Keyframe, Easing};
    /// let e = Evaluable::Keyframe(Keyframe::new(0.0_f32, 10.0, Easing::Linear, 1.0));
    /// ```
    Keyframe(Keyframe<T>),
    /// A pause where the value remains constant for a set duration.
    ///
    /// ## Examples
    /// ```
    /// use abrash_anim::{Evaluable, Hold};
    /// let e = Evaluable::Hold(Hold::new(10.0_f32, 1.0));
    /// ```
    Hold(Hold<T>),
    /// A sequence that chains multiple evaluable segments back to back.
    ///
    /// ## Examples
    /// ```
    /// use abrash_anim::{Evaluable, Hold, Sequence};
    /// let s = Sequence::new(vec![Evaluable::Hold(Hold::new(10.0_f32, 1.0))]);
    /// let e = Evaluable::Sequence(s);
    /// ```
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
