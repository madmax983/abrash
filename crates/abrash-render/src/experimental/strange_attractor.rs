//! Strange Attractor Simulation
//!
//! Simulates particle movement through strange attractors like Lorenz or Thomas.

#![cfg(feature = "nova")]

use abrash_core::math::Vec3;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Type of strange attractor to simulate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AttractorType {
    /// Lorenz strange attractor.
    Lorenz {
        /// Sigma parameter.
        sigma: f32,
        /// Rho parameter.
        rho: f32,
        /// Beta parameter.
        beta: f32,
    },
    /// Thomas' cyclically symmetric attractor.
    Thomas {
        /// B parameter.
        b: f32,
    },
}

/// A particle in the strange attractor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Particle {
    /// Position in 3D space.
    pub position: Vec3,
    /// Color of the particle.
    pub color: u32,
}

/// Simulator for strange attractors.
#[derive(Debug, Clone)]
pub struct StrangeAttractor {
    /// Particles in the simulation.
    pub particles: Vec<Particle>,
    /// Type of attractor.
    pub attractor_type: AttractorType,
    /// Time step per iteration.
    pub dt: f32,
}

impl StrangeAttractor {
    /// Create a new simulator.
    #[must_use]
    pub const fn new(particles: Vec<Particle>, attractor_type: AttractorType, dt: f32) -> Self {
        Self {
            particles,
            attractor_type,
            dt,
        }
    }

    /// Run the simulation for a single step.
    pub fn step(&mut self) {
        self.run_steps(1);
    }

    /// Run the simulation for a given number of steps.
    pub fn run_steps(&mut self, steps: usize) {
        let dt = self.dt;
        let attractor_type = self.attractor_type;

        #[cfg(feature = "parallel")]
        let iter = self.particles.par_iter_mut();
        #[cfg(not(feature = "parallel"))]
        let iter = self.particles.iter_mut();

        iter.for_each(|particle| {
            let mut p = particle.position;
            for _ in 0..steps {
                let dp = match attractor_type {
                    AttractorType::Lorenz { sigma, rho, beta } => Vec3::new(
                        sigma * (p.y - p.x),
                        p.x * (rho - p.z) - p.y,
                        p.x * p.y - beta * p.z,
                    ),
                    AttractorType::Thomas { b } => Vec3::new(
                        p.y.sin() - b * p.x,
                        p.z.sin() - b * p.y,
                        p.x.sin() - b * p.z,
                    ),
                };
                p = Vec3::new(p.x + dp.x * dt, p.y + dp.y * dt, p.z + dp.z * dt);
            }
            particle.position = p;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lorenz_attractor() {
        let p = Particle {
            position: Vec3::new(1.0, 1.0, 1.0),
            color: 0xFF_FFFFFF,
        };
        let mut sim = StrangeAttractor::new(
            vec![p],
            AttractorType::Lorenz {
                sigma: 10.0,
                rho: 28.0,
                beta: 8.0 / 3.0,
            },
            0.01,
        );

        sim.run_steps(1);

        let p_after = sim.particles[0].position;
        assert!(
            p_after.x != 1.0 || p_after.y != 1.0 || p_after.z != 1.0,
            "Particle should move"
        );
    }
}
