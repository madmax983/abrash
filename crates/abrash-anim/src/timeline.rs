//! Timeline — stateful animation driver.
//!
//! Wraps an `Evaluable<T>` root and an `AnimationClock` to provide a
//! tick-driven animation API with builders for common patterns.

use std::time::Duration;

use abrash_core::animatable::Animatable;

use crate::clock::{AnimationClock, ClockEvent, PlaybackMode};
use crate::easing::Easing;
use crate::evaluable::{Evaluable, Keyframe, Sample};

enum TimelineState<T: Animatable> {
    Playing,
    Completed { final_sample: Sample<T> },
}

/// A stateful animation driver that wraps an `Evaluable<T>` root.
///
/// Call [`tick`](Self::tick) each frame with the delta time to advance
/// the animation and retrieve the current `Sample<T>`.
pub struct Timeline<T: Animatable> {
    root: Evaluable<T>,
    clock: AnimationClock,
    duration: f32,
    playback: PlaybackMode,
    state: TimelineState<T>,
    last_sample: Sample<T>,
}

impl<T: Animatable + Send + Sync + 'static> Timeline<T> {
    /// Create a timeline from an arbitrary evaluable root.
    #[must_use]
    pub fn from_evaluable(root: Evaluable<T>, playback: PlaybackMode) -> Self {
        let duration = root.natural_duration();
        let initial = root.evaluate(0.0);
        Self {
            root,
            clock: AnimationClock::new(),
            duration,
            playback,
            state: TimelineState::Playing,
            last_sample: initial,
        }
    }

    /// Create a simple linear tween between two values.
    #[must_use]
    pub fn tween(from: T, to: T, duration: Duration) -> Self {
        let secs = duration.as_secs_f32();
        Self::from_evaluable(
            Evaluable::Keyframe(Keyframe::new(from, to, Easing::Linear, secs)),
            PlaybackMode::Once,
        )
    }

    /// Override the easing curve.
    ///
    /// Replaces the root evaluable with a new `Keyframe` that tweens
    /// between the start and end values using the given easing.
    #[must_use]
    pub fn easing(mut self, easing: Easing) -> Self {
        let sample_start = self.root.evaluate(0.0);
        let sample_end = self.root.evaluate(1.0);
        self.root = Evaluable::Keyframe(Keyframe::new(
            sample_start.value,
            sample_end.value,
            easing,
            self.duration,
        ));
        self
    }

    /// Set playback mode to infinite loop.
    #[must_use]
    pub const fn loop_forever(mut self) -> Self {
        self.playback = PlaybackMode::Loop;
        self
    }

    /// Set playback mode to ping-pong (alternate forward/backward).
    #[must_use]
    pub const fn ping_pong(mut self) -> Self {
        self.playback = PlaybackMode::PingPong;
        self
    }

    /// Set playback mode to repeat exactly `n` times.
    #[must_use]
    pub const fn count(mut self, n: u32) -> Self {
        self.playback = PlaybackMode::Count(n);
        self
    }

    /// Advance the animation by `delta_secs` and return the current sample.
    ///
    /// Once completed, subsequent ticks return the final sample unchanged.
    pub fn tick(&mut self, delta_secs: f32) -> Sample<T> {
        if let TimelineState::Completed { final_sample } = &self.state {
            return final_sample.clone();
        }

        let sample = {
            let event = self.clock.tick(delta_secs, self.duration);

            match event {
                ClockEvent::Normal => {
                    let phase = self.clock.effective_phase(&self.playback);
                    self.root.evaluate(phase)
                }
                ClockEvent::CycleBoundary { .. } => {
                    if self.clock.is_finished(&self.playback) {
                        let final_sample = self.root.evaluate(1.0);
                        self.state = TimelineState::Completed {
                            final_sample: final_sample.clone(),
                        };
                        final_sample
                    } else {
                        let phase = self.clock.effective_phase(&self.playback);
                        self.root.evaluate(phase)
                    }
                }
            }
        };

        self.last_sample = sample.clone();
        sample
    }

    /// Whether the animation has finished (no more ticks will change the value).
    #[must_use]
    pub const fn is_completed(&self) -> bool {
        matches!(self.state, TimelineState::Completed { .. })
    }

    /// The most recently evaluated value.
    #[must_use]
    pub fn current_value(&self) -> T {
        self.last_sample.value.clone()
    }

    /// Reset the timeline to the beginning.
    pub fn reset(&mut self) {
        self.clock.reset();
        self.state = TimelineState::Playing;
        self.last_sample = self.root.evaluate(0.0);
    }

    /// Total duration in seconds.
    #[must_use]
    pub const fn duration(&self) -> f32 {
        self.duration
    }

    /// Current playback mode.
    #[must_use]
    pub const fn playback_mode(&self) -> &PlaybackMode {
        &self.playback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::math::Vec3;
    use std::time::Duration;

    const EPSILON: f32 = 1e-4;

    /// Advance a `Timeline<f32>` by `total` seconds in small increments
    /// to respect `MAX_DELTA_SECS` (0.1s) clamping in `AnimationClock`.
    fn tick_secs(tl: &mut Timeline<f32>, total: f32) -> Sample<f32> {
        let step = 0.05;
        let steps = (total / step) as u32;
        let remainder = total - (steps as f32 * step);
        let mut last = tl.tick(0.0);
        for _ in 0..steps {
            last = tl.tick(step);
        }
        if remainder > f32::EPSILON {
            last = tl.tick(remainder);
        }
        last
    }

    /// Advance a `Timeline<Vec3>` by `total` seconds in small increments.
    fn tick_secs_vec3(tl: &mut Timeline<Vec3>, total: f32) -> Sample<Vec3> {
        let step = 0.05;
        let steps = (total / step) as u32;
        let remainder = total - (steps as f32 * step);
        let mut last = tl.tick(0.0);
        for _ in 0..steps {
            last = tl.tick(step);
        }
        if remainder > f32::EPSILON {
            last = tl.tick(remainder);
        }
        last
    }

    #[test]
    fn tween_starts_at_from() {
        let mut tl = Timeline::tween(0.0_f32, 10.0, Duration::from_secs(1));
        let s = tl.tick(0.0);
        assert!((s.value).abs() < EPSILON);
    }

    #[test]
    fn tween_reaches_to() {
        let mut tl = Timeline::tween(0.0_f32, 10.0, Duration::from_secs(1));
        tick_secs(&mut tl, 1.0);
        assert!(tl.is_completed());
        assert!((tl.current_value() - 10.0).abs() < EPSILON);
    }

    #[test]
    fn tween_midpoint() {
        let mut tl = Timeline::tween(0.0_f32, 10.0, Duration::from_secs(2));
        let s = tick_secs(&mut tl, 1.0);
        assert!(
            (s.value - 5.0).abs() < 0.1,
            "expected ~5.0, got {}",
            s.value
        );
    }

    #[test]
    fn loop_forever_never_completes() {
        let mut tl = Timeline::tween(0.0_f32, 10.0, Duration::from_secs(1)).loop_forever();
        tick_secs(&mut tl, 1.0);
        assert!(!tl.is_completed());
    }

    #[test]
    fn ping_pong_reverses() {
        let mut tl = Timeline::tween(0.0_f32, 10.0, Duration::from_secs(1)).ping_pong();
        // Advance into second cycle (1.5s total for a 1s animation)
        tick_secs(&mut tl, 1.5);
        assert!(!tl.is_completed());
        let val = tl.current_value();
        assert!(val < 10.0, "PingPong should reverse: got {val}");
    }

    #[test]
    fn easing_modifier() {
        let mut tl = Timeline::tween(0.0_f32, 10.0, Duration::from_secs(2)).easing(Easing::EaseIn);
        let s = tick_secs(&mut tl, 1.0);
        assert!(
            s.value < 5.0,
            "EaseIn should be below linear at midpoint, got {}",
            s.value
        );
    }

    #[test]
    fn count_mode_finishes_after_n() {
        let mut tl = Timeline::tween(0.0_f32, 10.0, Duration::from_secs(1)).count(2);
        tick_secs(&mut tl, 2.0);
        assert!(tl.is_completed());
    }

    #[test]
    fn reset_restarts() {
        let mut tl = Timeline::tween(0.0_f32, 10.0, Duration::from_secs(1));
        tick_secs(&mut tl, 1.0);
        assert!(tl.is_completed());
        tl.reset();
        assert!(!tl.is_completed());
        let s = tl.tick(0.0);
        assert!((s.value).abs() < EPSILON);
    }

    #[test]
    fn sequence_builder() {
        use crate::evaluable::{Hold, Keyframe, Sequence};

        let seq = Sequence::new(vec![
            Evaluable::Keyframe(Keyframe::new(0.0, 10.0, Easing::Linear, 1.0)),
            Evaluable::Hold(Hold::new(10.0, 0.5)),
        ]);
        let tl = Timeline::<f32>::from_evaluable(Evaluable::Sequence(seq), PlaybackMode::Once);

        assert!((tl.duration() - 1.5).abs() < EPSILON);
    }

    #[test]
    fn vec3_tween() {
        let mut tl = Timeline::tween(
            Vec3::ZERO,
            Vec3::new(10.0, 20.0, 30.0),
            Duration::from_secs(2),
        );
        let s = tick_secs_vec3(&mut tl, 1.0);
        assert!(
            (s.value.x - 5.0).abs() < 0.5,
            "x: expected ~5.0, got {}",
            s.value.x
        );
        assert!(
            (s.value.y - 10.0).abs() < 1.0,
            "y: expected ~10.0, got {}",
            s.value.y
        );
        assert!(
            (s.value.z - 15.0).abs() < 1.5,
            "z: expected ~15.0, got {}",
            s.value.z
        );
    }

    #[test]
    fn completed_timeline_stays_at_final_value() {
        let mut tl = Timeline::tween(0.0_f32, 10.0, Duration::from_secs(1));
        tick_secs(&mut tl, 1.0);
        let s1 = tl.tick(0.05);
        let s2 = tl.tick(0.05);
        assert!((s1.value - 10.0).abs() < EPSILON);
        assert!((s2.value - 10.0).abs() < EPSILON);
    }

    #[test]
    fn duration_returns_total() {
        let tl = Timeline::tween(0.0_f32, 10.0, Duration::from_millis(2500));
        assert!((tl.duration() - 2.5).abs() < EPSILON);
    }

    #[test]
    fn playback_mode_accessor() {
        let tl = Timeline::tween(0.0_f32, 10.0, Duration::from_secs(1)).loop_forever();
        assert_eq!(*tl.playback_mode(), PlaybackMode::Loop);
    }
}
