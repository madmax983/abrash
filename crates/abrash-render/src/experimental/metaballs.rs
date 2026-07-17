//! Metaballs / 2D Implicit Surfaces Filter
//!
//! A procedural screen-space effect that renders smooth, merging "goo" or blobs
//! known as metaballs. It works by evaluating an implicit distance field for
//! multiple points and thresholding the accumulated value.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec2;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Metaballs effect.
#[derive(Debug, Clone, Copy)]
pub struct MetaballsConfig {
    /// Number of metaballs to simulate.
    pub num_balls: usize,
    /// Threshold for the iso-surface (e.g., 1.0).
    pub threshold: f32,
    /// Color of the "goo".
    pub blob_color: u32,
    /// Background color.
    pub bg_color: u32,
    /// Maximum speed of the balls.
    pub speed: f32,
    /// Maximum size (radius multiplier) of the balls.
    pub max_size: f32,
}

impl Default for MetaballsConfig {
    fn default() -> Self {
        Self {
            num_balls: 5,
            threshold: 1.0,
            blob_color: 0xFF_FF5500, // Orange
            bg_color: 0xFF_000000,
            speed: 5.0,
            max_size: 50.0,
        }
    }
}

/// A single metaball entity.
#[derive(Debug, Clone)]
pub struct Metaball {
    /// Vec2.
    pub position: Vec2,
    /// Vec2.
    pub velocity: Vec2,
    /// F32.
    pub size: f32,
}

/// State for the Metaballs simulation.
pub struct Metaballs {
    /// Metaballsconfig.
    pub config: MetaballsConfig,
    /// `Vec<metaball>.`
    pub balls: Vec<Metaball>,
    initialized: bool,
}

impl Default for Metaballs {
    fn default() -> Self {
        Self::new(MetaballsConfig::default())
    }
}

impl Metaballs {
    #[must_use]
    /// New.
    pub const fn new(config: MetaballsConfig) -> Self {
        Self {
            config,
            balls: Vec::new(),
            initialized: false,
        }
    }

    /// Updates the metaball positions and renders them to the framebuffer.
    pub fn update_and_render(&mut self, fb: &mut Framebuffer) {
        let width = fb.width() as f32;
        let height = fb.height() as f32;

        if width == 0.0 || height == 0.0 {
            return;
        }

        // 1. Initialize balls if this is the first frame
        if !self.initialized {
            let mut rng = abrash_core::utils::XorShift32::new(0xDEAD_BEEF);
            for _ in 0..self.config.num_balls {
                let x = rng.next_f32() * width;
                let y = rng.next_f32() * height;
                let vx = (rng.next_f32() - 0.5) * self.config.speed;
                let vy = (rng.next_f32() - 0.5) * self.config.speed;
                // Minimum size to prevent divide by zero, max size configurable
                let size = 10.0 + rng.next_f32() * self.config.max_size;

                self.balls.push(Metaball {
                    position: Vec2::new(x, y),
                    velocity: Vec2::new(vx, vy),
                    size,
                });
            }
            self.initialized = true;
        }

        // 2. Update physical positions (bouncing off walls)
        for ball in &mut self.balls {
            ball.position = ball.position + ball.velocity;

            if ball.position.x - ball.size < 0.0 {
                ball.position.x = ball.size;
                ball.velocity.x *= -1.0;
            } else if ball.position.x + ball.size > width {
                ball.position.x = width - ball.size;
                ball.velocity.x *= -1.0;
            }

            if ball.position.y - ball.size < 0.0 {
                ball.position.y = ball.size;
                ball.velocity.y *= -1.0;
            } else if ball.position.y + ball.size > height {
                ball.position.y = height - ball.size;
                ball.velocity.y *= -1.0;
            }
        }

        // 3. Render the implicit surface
        let width_u = fb.width() as usize;
        let pixels = fb.as_mut_slice();
        let bg = self.config.bg_color;
        let fg = self.config.blob_color;
        let threshold = self.config.threshold;
        let num_balls = self.balls.len();

        // Cache ball positions and square sizes to avoid repeated property access in hot loop
        let mut b_xs = Vec::with_capacity(num_balls);
        let mut b_ys = Vec::with_capacity(num_balls);
        let mut b_r_sqs = Vec::with_capacity(num_balls);

        for ball in &self.balls {
            b_xs.push(ball.position.x);
            b_ys.push(ball.position.y);
            b_r_sqs.push(ball.size * ball.size);
        }

        #[cfg(feature = "parallel")]
        let iter = pixels.par_chunks_exact_mut(width_u).enumerate();
        #[cfg(not(feature = "parallel"))]
        let iter = pixels.chunks_exact_mut(width_u).enumerate();

        iter.for_each(|(y, row)| {
            let fy = y as f32;
            for (x, pixel) in row.iter_mut().enumerate() {
                let fx = x as f32;

                let mut sum = 0.0;
                for i in 0..num_balls {
                    let dx = fx - b_xs[i];
                    let dy = fy - b_ys[i];
                    let dist_sq = dx * dx + dy * dy;

                    // Prevent divide by zero if exactly on center
                    if dist_sq > 0.001 {
                        // Formula: f(x, y) = r^2 / d^2
                        sum += b_r_sqs[i] / dist_sq;
                    }
                }

                if sum >= threshold {
                    *pixel = fg;
                } else {
                    *pixel = bg;
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metaballs_init() {
        let mut sim = Metaballs::default();
        let mut fb = Framebuffer::new(100, 100).unwrap();
        sim.update_and_render(&mut fb);
        // Initially empty
    }
}
