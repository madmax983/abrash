//! Strange Attractor Simulation
//!
//! This module provides a massively parallel implementation of the Lorenz Attractor.

use rayon::prelude::*;

/// A single particle in 3D space.
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// The Lorenz Attractor system.
pub struct LorenzAttractor {
    pub sigma: f32,
    pub rho: f32,
    pub beta: f32,
    pub dt: f32,
    pub particles: Vec<Particle>,
}

impl LorenzAttractor {
    /// Creates a new Lorenz Attractor simulation.
    #[must_use]
    pub const fn new(sigma: f32, rho: f32, beta: f32, dt: f32, particles: Vec<Particle>) -> Self {
        Self {
            sigma,
            rho,
            beta,
            dt,
            particles,
        }
    }

    /// Advances the simulation by a single step.
    pub fn step(&mut self) {
        self.run_steps(1);
    }

    /// Advances the simulation by `count` steps.
    /// Batching iterations avoids the overhead of hot-loop jumps and enables
    /// the compiler to autovectorize or unroll loops.
    pub fn run_steps(&mut self, count: usize) {
        let sigma = self.sigma;
        let rho = self.rho;
        let beta = self.beta;
        let dt = self.dt;

        self.particles.par_iter_mut().for_each(|p| {
            for _ in 0..count {
                let dx = sigma * (p.y - p.x);
                let dy = p.x * (rho - p.z) - p.y;
                let dz = p.x * p.y - beta * p.z;

                p.x += dx * dt;
                p.y += dy * dt;
                p.z += dz * dt;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lorenz_step() {
        let mut attractor = LorenzAttractor::new(
            10.0,
            28.0,
            8.0 / 3.0,
            0.01,
            vec![Particle {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            }],
        );

        attractor.step();

        assert!((attractor.particles[0].x - 1.0).abs() < 1e-4);
        assert!((attractor.particles[0].y - 1.26).abs() < 1e-4);
        assert!((attractor.particles[0].z - 0.9833333).abs() < 1e-4);
    }
}
