//! Reaction-Diffusion (Gray-Scott) Simulation
//!
//! Simulates the complex organic Turing patterns formed by two interacting chemicals.

use crate::framebuffer::Framebuffer;
use abrash_core::color;
use abrash_core::utils::XorShift32;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration parameters for the Gray-Scott reaction-diffusion model.
#[derive(Debug, Clone, Copy)]
pub struct ReactionDiffusionConfig {
    /// Diffusion rate of chemical A.
    pub da: f32,
    /// Diffusion rate of chemical B.
    pub db: f32,
    /// Feed rate at which chemical A is added.
    pub feed: f32,
    /// Kill rate at which chemical B is removed.
    pub kill: f32,
    /// Time step size.
    pub dt: f32,
}

impl Default for ReactionDiffusionConfig {
    fn default() -> Self {
        // Classic "coral" or "mitosis" preset
        Self {
            da: 1.0,
            db: 0.5,
            feed: 0.055,
            kill: 0.062,
            dt: 1.0,
        }
    }
}

/// Represents the state of a single cell in the grid.
#[derive(Debug, Clone, Copy, Default)]
struct Cell {
    a: f32,
    b: f32,
}

/// A 2D grid for the reaction-diffusion simulation.
pub struct ReactionDiffusion {
    /// Usize.
    pub width: usize,
    /// Usize.
    pub height: usize,
    grid: Vec<Cell>,
    next_grid: Vec<Cell>,
    /// Reactiondiffusionconfig.
    pub config: ReactionDiffusionConfig,
}

impl ReactionDiffusion {
    /// Creates a new reaction-diffusion simulation of the given dimensions.
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        let size = width * height;
        let mut grid = vec![Cell { a: 1.0, b: 0.0 }; size];
        let next_grid = vec![Cell { a: 1.0, b: 0.0 }; size];

        Self {
            width,
            height,
            grid,
            next_grid,
            config: ReactionDiffusionConfig::default(),
        }
    }

    /// Seeds the simulation with some initial B chemical in a square at the given coordinates.
    pub fn seed(&mut self, cx: usize, cy: usize, size: usize) {
        let half_size = size / 2;
        let start_x = cx.saturating_sub(half_size);
        let end_x = (cx + half_size).min(self.width - 1);
        let start_y = cy.saturating_sub(half_size);
        let end_y = (cy + half_size).min(self.height - 1);

        for y in start_y..=end_y {
            for x in start_x..=end_x {
                let idx = y * self.width + x;
                self.grid[idx].b = 1.0;
            }
        }
    }

    /// Seeds the simulation with random noise in a central region to jump-start the patterns.
    pub fn seed_random(&mut self, rng: &mut XorShift32, region_size: usize) {
        let cx = self.width / 2;
        let cy = self.height / 2;
        let half_size = region_size / 2;

        let start_x = cx.saturating_sub(half_size);
        let end_x = (cx + half_size).min(self.width - 1);
        let start_y = cy.saturating_sub(half_size);
        let end_y = (cy + half_size).min(self.height - 1);

        for y in start_y..=end_y {
            for x in start_x..=end_x {
                if rng.next_f32() > 0.5 {
                    let idx = y * self.width + x;
                    self.grid[idx].b = 1.0;
                }
            }
        }
    }

    /// Advances the simulation by one time step.
    pub fn step(&mut self) {
        // Implementation of the Gray-Scott equations using a 3x3 Laplacian kernel.
        // The Laplacian weights for a 3x3 grid:
        // center = -1.0
        // adjacent = 0.2
        // diagonal = 0.05

        let w = self.width;
        let h = self.height;
        let mut next_grid = std::mem::take(&mut self.next_grid);

        let config = self.config;

        // We skip the 1-pixel border to avoid out-of-bounds checks in the inner loop.
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let idx = y * w + x;

                let c = &self.grid[idx];
                let a = c.a;
                let b = c.b;

                // 3x3 Convolution (Laplacian)
                // Center
                let mut sum_a = a * -1.0;
                let mut sum_b = b * -1.0;

                // Adjacent (top, bottom, left, right)
                sum_a += self.grid[(y - 1) * w + x].a * 0.2;
                sum_a += self.grid[(y + 1) * w + x].a * 0.2;
                sum_a += self.grid[y * w + (x - 1)].a * 0.2;
                sum_a += self.grid[y * w + (x + 1)].a * 0.2;

                sum_b += self.grid[(y - 1) * w + x].b * 0.2;
                sum_b += self.grid[(y + 1) * w + x].b * 0.2;
                sum_b += self.grid[y * w + (x - 1)].b * 0.2;
                sum_b += self.grid[y * w + (x + 1)].b * 0.2;

                // Diagonal
                sum_a += self.grid[(y - 1) * w + (x - 1)].a * 0.05;
                sum_a += self.grid[(y - 1) * w + (x + 1)].a * 0.05;
                sum_a += self.grid[(y + 1) * w + (x - 1)].a * 0.05;
                sum_a += self.grid[(y + 1) * w + (x + 1)].a * 0.05;

                sum_b += self.grid[(y - 1) * w + (x - 1)].b * 0.05;
                sum_b += self.grid[(y - 1) * w + (x + 1)].b * 0.05;
                sum_b += self.grid[(y + 1) * w + (x - 1)].b * 0.05;
                sum_b += self.grid[(y + 1) * w + (x + 1)].b * 0.05;

                // Reaction-Diffusion Equations
                // A' = A + (Da * Laplace(A) - A * B^2 + feed * (1 - A)) * dt
                // B' = B + (Db * Laplace(B) + A * B^2 - (kill + feed) * B) * dt
                let reaction = a * b * b;

                let next_a =
                    a + (config.da * sum_a - reaction + config.feed * (1.0 - a)) * config.dt;
                let next_b = b
                    + (config.db * sum_b + reaction - (config.kill + config.feed) * b) * config.dt;

                next_grid[idx].a = next_a.clamp(0.0, 1.0);
                next_grid[idx].b = next_b.clamp(0.0, 1.0);
            }
        }

        self.next_grid = std::mem::take(&mut self.grid);
        self.grid = next_grid;
    }

    /// Renders the current state of chemical B to the framebuffer.
    /// Pixels where B is high will be bright, mapping to the given color.
    pub fn render(&self, fb: &mut Framebuffer, color1: u32, color2: u32) {
        let fb_w = fb.width() as usize;
        let w = self.width.min(fb_w);
        let h = self.height.min(fb.height() as usize);

        let pixels = fb.as_mut_slice();

        for y in 0..h {
            let row_start = y * fb_w;
            let grid_start = y * self.width;

            for x in 0..w {
                let cell = &self.grid[grid_start + x];
                let b_val = cell.b;

                // b_val is typically 0.0 to ~0.5, we can map it directly or scale it slightly
                // Let's use a nice color gradient based on B
                let t = (b_val * 2.0).clamp(0.0, 1.0);

                let c1 = color::Color::from_argb_u32(color1);
                let c2 = color::Color::from_argb_u32(color2);
                let c = c1.lerp(c2, t).to_argb_u32();

                pixels[row_start + x] = c;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rd_step() {
        let mut rd = ReactionDiffusion::new(10, 10);
        // Initially B is 0 everywhere.
        assert!((rd.grid[15].b - 0.0).abs() < 1e-6);

        rd.seed(5, 5, 2);

        // After seeding, the center should have B.
        let center_idx = 5 * 10 + 5;
        assert!((rd.grid[center_idx].b - 1.0).abs() < 1e-6);

        // A cell adjacent to the seed should initially be 0.
        let adj_idx = 5 * 10 + 3;
        assert!((rd.grid[adj_idx].b - 0.0).abs() < 1e-6);

        // Step the simulation
        rd.step();

        // The chemical B should have diffused outwards.
        // It won't be exactly 0 anymore.
        assert!(rd.grid[adj_idx].b > 0.0);
    }
}
