//! Time utilities for game loop.
//!
//! Provides fixed timestep accumulator for consistent simulation.

use std::time::{Duration, Instant};

/// Fixed timestep game loop helper
pub struct FixedTimestep {
    target_dt: Duration,
    accumulator: Duration,
    last_time: Instant,
}

impl FixedTimestep {
    /// Create a new fixed timestep with target FPS
    pub fn new(target_fps: u32) -> Self {
        Self {
            target_dt: Duration::from_secs_f64(1.0 / target_fps as f64),
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
    pub fn dt(&self) -> f32 {
        self.target_dt.as_secs_f32()
    }
}
