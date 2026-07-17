//! Strange Attractor Simulation

#![cfg(feature = "nova")]

#[cfg(feature = "parallel")]
use rayon::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AttractorType {
    Lorenz { sigma: f32, rho: f32, beta: f32 },
    Roessler { a: f32, b: f32, c: f32 },
}

impl Default for AttractorType {
    fn default() -> Self {
        Self::Lorenz {
            sigma: 10.0,
            rho: 28.0,
            beta: 8.0 / 3.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particle {
    pub position: [f32; 3],
}

impl Particle {
    #[must_use]
    pub const fn new(position: [f32; 3]) -> Self {
        Self { position }
    }
}

pub struct AttractorSystem {
    pub particles: Vec<Particle>,
    pub attractor_type: AttractorType,
}

impl AttractorSystem {
    #[must_use]
    pub const fn new(attractor_type: AttractorType) -> Self {
        Self {
            particles: Vec::new(),
            attractor_type,
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        self.run_steps(delta_time, 1);
    }

    /// Batches iterations to avoid the overhead of hot-loop jumps at the call site
    /// and enables the compiler to autovectorize or unroll loops.
    pub fn run_steps(&mut self, delta_time: f32, count: usize) {
        let attractor = self.attractor_type;

        #[cfg(feature = "parallel")]
        let iter = self.particles.par_iter_mut();
        #[cfg(not(feature = "parallel"))]
        let iter = self.particles.iter_mut();

        iter.for_each(|particle| {
            for _ in 0..count {
                let p = particle.position;
                let (dx, dy, dz) = match attractor {
                    AttractorType::Lorenz { sigma, rho, beta } => (
                        sigma * (p[1] - p[0]),
                        p[0] * (rho - p[2]) - p[1],
                        p[0] * p[1] - beta * p[2],
                    ),
                    AttractorType::Roessler { a, b, c } => {
                        (-(p[1] + p[2]), p[0] + a * p[1], b + p[2] * (p[0] - c))
                    }
                };

                particle.position[0] += dx * delta_time;
                particle.position[1] += dy * delta_time;
                particle.position[2] += dz * delta_time;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lorenz_attractor() {
        let mut system = AttractorSystem::new(AttractorType::Lorenz {
            sigma: 10.0,
            rho: 28.0,
            beta: 8.0 / 3.0,
        });

        system.particles.push(Particle::new([1.0, 1.0, 1.0]));
        let initial = system.particles[0].position;
        system.run_steps(0.01, 10);
        let next = system.particles[0].position;

        assert_ne!(initial[0], next[0]);
        assert_ne!(initial[1], next[1]);
        assert_ne!(initial[2], next[2]);
    }
}
