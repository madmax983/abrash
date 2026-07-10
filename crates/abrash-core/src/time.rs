//! Time utilities for game loop.
//!
//! Provides fixed timestep accumulator for consistent simulation.

#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};
#[cfg(target_arch = "wasm32")]
use web_time::{Duration, Instant};

/// Fixed timestep game loop helper
///
/// # Examples
///
/// ```
/// use abrash_core::time::FixedTimestep;
///
/// let mut timer = FixedTimestep::new(60); // 60 FPS
/// let steps = timer.update();
/// // Perform `steps` logic ticks...
/// ```
pub struct FixedTimestep {
    target_dt: Duration,
    accumulator: Duration,
    last_time: Instant,
}

impl FixedTimestep {
    /// Create a new fixed timestep with target FPS
    #[must_use]
    pub fn new(target_fps: u32) -> Self {
        Self {
            target_dt: Duration::from_secs_f64(1.0 / f64::from(target_fps)),
            accumulator: Duration::ZERO,
            last_time: Instant::now(),
        }
    }

    /// Update the accumulator and return number of fixed steps to run
    pub fn update(&mut self) -> u32 {
        let now = Instant::now();
        let frame_time = now - self.last_time;
        self.last_time = now;

        self.accumulator += frame_time;

        let mut steps = 0;
        while self.accumulator >= self.target_dt {
            self.accumulator -= self.target_dt;
            steps += 1;
        }

        steps
    }

    /// Get the fixed delta time in seconds
    #[must_use]
    pub const fn dt(&self) -> f32 {
        self.target_dt.as_secs_f32()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::unreadable_literal)]
    fn test_fixed_timestep_init() {
        let timer = FixedTimestep::new(60);
        // Target dt for 60 fps is ~0.016666
        assert!((timer.dt() - 0.016666).abs() < 0.0001);
    }

    #[test]
    #[allow(clippy::unreadable_literal)]
    fn test_fixed_timestep_update() {
        let mut timer = FixedTimestep::new(60);

        // Update relies on `now = Instant::now()`.
        // We can test the accumulator behavior by not relying on the time elapsed
        // since `new()`, which is flaky, but rather directly manipulating internal state
        // or just updating `last_time` and `accumulator` before calling `update()`.

        // Reset last_time to exactly now, so `now - last_time` in update() is 0.
        timer.last_time = Instant::now();
        // Add 50ms to the accumulator directly.
        timer.accumulator += Duration::from_millis(50);

        let steps = timer.update();
        // 50ms at 60fps (16.6ms per frame) should be exactly 3 steps.
        // Wait, 50 / 16.666 = 3
        assert_eq!(steps, 3);

        // Accumulator should have remaining fraction: 50 - 3 * 16.666 = 50 - 50 = 0.
        // Wait, 16.666 * 3 = 50.
        // 1/60 * 3 = 0.05. 50ms = 0.05s.
        // So it should be almost exactly 0 or slightly more/less depending on float precision.
        assert!(timer.accumulator.as_secs_f64() < 0.001);
    }
}
