//! Drift-free animation clock using `(cycle, phase)` model.
//!
//! Inspired by `AletheiaDB`'s hybrid logical clock. The integer `cycle`
//! counter never accumulates floating-point error, while `phase` stays
//! in 0.0–1.0 and resets each cycle.

/// Maximum delta per tick (seconds). Caps frame-time spikes from
/// tab-backgrounding or debugger pauses to prevent massive phase jumps.
const MAX_DELTA_SECS: f32 = 0.1;

/// How the animation repeats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackMode {
    /// Play once and stop.
    Once,
    /// Loop forever.
    Loop,
    /// Alternate forward/backward.
    PingPong,
    /// Play exactly N times.
    Count(u32),
}

/// Events emitted by the clock on each tick to notify systems of boundary crossings.
///
/// This is used heavily for triggering one-shot events, like a footstep sound playing
/// exactly when a walk animation loop crosses from 1.0 back to 0.0 phase.
///
/// # Examples
///
/// ```
/// use abrash_anim::clock::{AnimationClock, ClockEvent};
///
/// let mut clock = AnimationClock::new();
///
/// // Normal tick that doesn't cross a boundary
/// let event = clock.tick(0.5, 1.0);
/// assert_eq!(event, ClockEvent::Normal);
///
/// // A tick that forces the clock past 1.0
/// let event = clock.tick(0.6, 1.0);
/// assert_eq!(event, ClockEvent::CycleBoundary { completed: 1 });
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockEvent {
    /// The clock advanced normally without crossing the 1.0 phase boundary.
    Normal,
    /// The clock crossed the 1.0 phase boundary one or more times.
    CycleBoundary {
        /// The number of times the clock wrapped around from 1.0 to 0.0 in this tick.
        completed: u64,
    },
}

/// A drift-free animation clock.
///
/// Uses `(cycle: u64, phase: f32)` to eliminate floating-point
/// accumulation errors in looping animations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationClock {
    cycle: u64,
    phase: f32,
}

impl AnimationClock {
    /// Create a new clock at time zero.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            cycle: 0,
            phase: 0.0,
        }
    }

    /// Advance the clock by `delta_secs` for an animation of `duration` seconds.
    ///
    /// Returns a `ClockEvent` indicating whether a cycle boundary was crossed.
    /// Delta is clamped to `MAX_DELTA_SECS` to handle tab-backgrounding.
    pub fn tick(&mut self, delta_secs: f32, duration: f32) -> ClockEvent {
        debug_assert!(duration > 0.0, "Clock duration must be positive");

        let clamped = delta_secs.clamp(0.0, MAX_DELTA_SECS);
        let phase_advance = clamped / duration;

        let new_phase = self.phase + phase_advance;
        if new_phase >= 1.0 {
            let whole_cycles = new_phase as u64;
            self.cycle = self.cycle.saturating_add(whole_cycles);
            self.phase = new_phase.fract();
            ClockEvent::CycleBoundary {
                completed: whole_cycles,
            }
        } else {
            self.phase = new_phase;
            ClockEvent::Normal
        }
    }

    /// Current phase within the cycle (0.0–1.0).
    #[must_use]
    #[inline]
    pub const fn phase(&self) -> f32 {
        self.phase
    }

    /// Number of completed cycles.
    #[must_use]
    #[inline]
    pub const fn cycle(&self) -> u64 {
        self.cycle
    }

    /// Phase adjusted for playback mode (e.g. reversed on odd `PingPong` cycles).
    #[must_use]
    pub fn effective_phase(&self, mode: &PlaybackMode) -> f32 {
        match mode {
            PlaybackMode::PingPong if self.cycle % 2 == 1 => 1.0 - self.phase,
            _ => self.phase,
        }
    }

    /// Whether the animation has completed for the given playback mode.
    #[must_use]
    pub fn is_finished(&self, mode: &PlaybackMode) -> bool {
        match mode {
            PlaybackMode::Once => self.cycle >= 1,
            PlaybackMode::Count(n) => self.cycle >= u64::from(*n),
            PlaybackMode::Loop | PlaybackMode::PingPong => false,
        }
    }

    /// Reset to time zero.
    pub const fn reset(&mut self) {
        self.cycle = 0;
        self.phase = 0.0;
    }
}

impl Default for AnimationClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    #[test]
    fn new_clock_starts_at_zero() {
        let c = AnimationClock::new();
        assert_eq!(c.cycle(), 0);
        assert!((c.phase()).abs() < EPSILON);
    }

    /// Helper: tick the clock multiple times with small deltas to reach `target_secs`.
    fn tick_to(c: &mut AnimationClock, target_secs: f32, duration: f32) -> ClockEvent {
        let step = 0.05; // 50ms steps, well under MAX_DELTA_SECS
        let steps = (target_secs / step) as u32;
        let remainder = target_secs - (steps as f32 * step);
        let mut last_event = ClockEvent::Normal;
        for _ in 0..steps {
            let e = c.tick(step, duration);
            if matches!(e, ClockEvent::CycleBoundary { .. }) {
                last_event = e;
            }
        }
        if remainder > f32::EPSILON {
            let e = c.tick(remainder, duration);
            if matches!(e, ClockEvent::CycleBoundary { .. }) {
                last_event = e;
            }
        }
        last_event
    }

    #[test]
    fn tick_advances_phase() {
        let mut c = AnimationClock::new();
        let event = c.tick(0.05, 1.0); // 50ms into 1-second duration
        assert!(matches!(event, ClockEvent::Normal));
        assert!((c.phase() - 0.05).abs() < EPSILON);
    }

    #[test]
    fn tick_past_one_triggers_cycle_boundary() {
        let mut c = AnimationClock::new();
        tick_to(&mut c, 1.5, 1.0);
        assert_eq!(c.cycle(), 1);
        assert!((c.phase() - 0.5).abs() < 0.01);
    }

    #[test]
    fn multiple_cycles() {
        let mut c = AnimationClock::new();
        tick_to(&mut c, 3.5, 1.0);
        assert_eq!(c.cycle(), 3);
        assert!((c.phase() - 0.5).abs() < 0.01);
    }

    #[test]
    fn delta_clamped_to_max() {
        let mut c = AnimationClock::new();
        c.tick(999.0, 1.0); // huge delta, clamped to MAX_DELTA_SECS (0.1)
        assert!((c.phase() - 0.1).abs() < EPSILON);
        assert_eq!(c.cycle(), 0); // 0.1 phase, no cycle crossed
    }

    #[test]
    fn effective_phase_ping_pong_reverses_odd_cycle() {
        let mut c = AnimationClock::new();
        tick_to(&mut c, 1.3, 1.0); // cycle=1, phase≈0.3
        assert_eq!(c.cycle(), 1);
        let ep = c.effective_phase(&PlaybackMode::PingPong);
        // phase should be ~0.3, effective = 1.0 - 0.3 = 0.7
        assert!((ep - 0.7).abs() < 0.01);
    }

    #[test]
    fn effective_phase_loop_is_just_phase() {
        let mut c = AnimationClock::new();
        tick_to(&mut c, 1.3, 1.0);
        let ep = c.effective_phase(&PlaybackMode::Loop);
        assert!((ep - 0.3).abs() < 0.01);
    }

    #[test]
    fn is_finished_once() {
        let mut c = AnimationClock::new();
        assert!(!c.is_finished(&PlaybackMode::Once));
        tick_to(&mut c, 1.0, 1.0);
        assert!(c.is_finished(&PlaybackMode::Once));
    }

    #[test]
    fn is_finished_count() {
        let mut c = AnimationClock::new();
        tick_to(&mut c, 2.0, 1.0);
        assert!(!c.is_finished(&PlaybackMode::Count(3)));
        tick_to(&mut c, 1.0, 1.0);
        assert!(c.is_finished(&PlaybackMode::Count(3)));
    }

    #[test]
    fn loop_never_finishes() {
        let mut c = AnimationClock::new();
        // Tick many times to accumulate cycles
        for _ in 0..200 {
            c.tick(0.05, 0.01); // 5 cycles per tick
        }
        assert!(!c.is_finished(&PlaybackMode::Loop));
    }

    #[test]
    fn reset_clears_state() {
        let mut c = AnimationClock::new();
        tick_to(&mut c, 1.5, 1.0);
        c.reset();
        assert_eq!(c.cycle(), 0);
        assert!((c.phase()).abs() < EPSILON);
    }
}
