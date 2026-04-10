#![allow(clippy::return_self_not_must_use)]
//! Timeline — stateful animation driver.
//!
//! Wraps an `Evaluable<T>` root and an `AnimationClock` to provide a
//! tick-driven animation API with builders for common patterns.

use std::time::Duration;

use abrash_core::animatable::Animatable;

use crate::clock::{AnimationClock, ClockEvent, PlaybackMode};
use crate::easing::Easing;
use crate::evaluable::{Evaluable, Sample};
use crate::keyframe::Keyframe;

enum TimelineState<T: Animatable> {
    Playing,
    Completed { final_sample: Sample<T> },
}

/// A stateful animation driver that wraps an `Evaluable<T>` root.
///
/// Call [`tick`](Self::tick) each frame with the delta time to advance
/// the animation and retrieve the current `Sample<T>`.
pub struct Timeline<T: Animatable> {
    root: Box<dyn Evaluable<T>>,
    clock: AnimationClock,
    duration: f32,
    playback: PlaybackMode,
    state: TimelineState<T>,
    last_sample: Sample<T>,
}

impl<T: Animatable + Send + Sync + 'static> Timeline<T> {
    /// Create a timeline from an arbitrary evaluable root.
    #[must_use]
    pub fn from_evaluable(root: Box<dyn Evaluable<T>>, playback: PlaybackMode) -> Self {
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
            Box::new(Keyframe::new(from, to, Easing::Linear, secs)),
            PlaybackMode::Once,
        )
    }

    /// Start building a multi-segment sequence.
    /// Override the easing curve.
    ///
    /// Replaces the root evaluable with a new `Keyframe` that tweens
    /// between the start and end values using the given easing.
    pub fn easing(mut self, easing: Easing) -> Self {
        let sample_start = self.root.evaluate(0.0);
        let sample_end = self.root.evaluate(1.0);
        self.root = Box::new(Keyframe::new(
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
