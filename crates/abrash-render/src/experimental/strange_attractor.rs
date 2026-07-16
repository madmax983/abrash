//! Strange attractors simulation and rendering.
//!
//! Visualizes chaotic systems like Lorenz and Roessler attractors.

use abrash_core::math::Vec3;
use abrash_core::framebuffer::Framebuffer;
#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Defines the type of strange attractor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AttractorType {
    /// Lorenz attractor.
    Lorenz {
        /// Sigma parameter.
        sigma: f32,
        /// Rho parameter.
        rho: f32,
        /// Beta parameter.
        beta: f32,
    },
    /// Roessler attractor.
    Roessler {
        /// A parameter.
        a: f32,
        /// B parameter.
        b: f32,
        /// C parameter.
        c: f32,
    },
}

/// A single particle in the strange attractor system.
#[derive(Debug, Clone, Copy)]
pub struct AttractorParticle {
    /// The current 3D position.
    pub pos: Vec3,
    /// Color represented as an ARGB u32.
    pub color: u32,
}

/// A strange attractor system.
pub struct StrangeAttractor {
    /// The particles in the system.
    pub particles: Vec<AttractorParticle>,
    /// The attractor parameters.
    pub attractor_type: AttractorType,
    /// Integration time step.
    pub dt: f32,
}

impl StrangeAttractor {
    /// Creates a new strange attractor system.
    #[must_use]
    pub fn new(num_particles: usize, attractor_type: AttractorType, dt: f32) -> Self {
        let mut particles = Vec::with_capacity(num_particles);
        for i in 0..num_particles {
            let offset = (i as f32) / (num_particles as f32);
            particles.push(AttractorParticle {
                pos: Vec3::new(0.1 + offset, 0.1, 0.1),
                color: 0xFFFFFFFF,
            });
        }
        Self {
            particles,
            attractor_type,
            dt,
        }
    }

    /// Step the simulation forward by one time step.
    pub fn step(&mut self) {
        let dt = self.dt;
        match self.attractor_type {
            AttractorType::Lorenz { sigma, rho, beta } => {
                #[cfg(feature = "parallel")]
                self.particles.par_iter_mut().for_each(|p| {
                    let dx = sigma * (p.pos.y - p.pos.x);
                    let dy = p.pos.x * (rho - p.pos.z) - p.pos.y;
                    let dz = p.pos.x * p.pos.y - beta * p.pos.z;
                    p.pos.x += dx * dt;
                    p.pos.y += dy * dt;
                    p.pos.z += dz * dt;
                });
                #[cfg(not(feature = "parallel"))]
                for p in &mut self.particles {
                    let dx = sigma * (p.pos.y - p.pos.x);
                    let dy = p.pos.x * (rho - p.pos.z) - p.pos.y;
                    let dz = p.pos.x * p.pos.y - beta * p.pos.z;
                    p.pos.x += dx * dt;
                    p.pos.y += dy * dt;
                    p.pos.z += dz * dt;
                }
            }
            AttractorType::Roessler { a, b, c } => {
                #[cfg(feature = "parallel")]
                self.particles.par_iter_mut().for_each(|p| {
                    let dx = -p.pos.y - p.pos.z;
                    let dy = p.pos.x + a * p.pos.y;
                    let dz = b + p.pos.z * (p.pos.x - c);
                    p.pos.x += dx * dt;
                    p.pos.y += dy * dt;
                    p.pos.z += dz * dt;
                });
                #[cfg(not(feature = "parallel"))]
                for p in &mut self.particles {
                    let dx = -p.pos.y - p.pos.z;
                    let dy = p.pos.x + a * p.pos.y;
                    let dz = b + p.pos.z * (p.pos.x - c);
                    p.pos.x += dx * dt;
                    p.pos.y += dy * dt;
                    p.pos.z += dz * dt;
                }
            }
        }
    }

    /// Renders the particles to the framebuffer.
    pub fn render(&self, fb: &mut Framebuffer, scale: f32, center_x: f32, center_y: f32) {
        let w = fb.width() as i32;
        let h = fb.height() as i32;
        let slice = fb.as_mut_slice();

        for p in &self.particles {
            let px = (center_x + p.pos.x * scale) as i32;
            let py = (center_y - p.pos.y * scale) as i32; // -y since screen is y-down

            if px >= 0 && px < w && py >= 0 && py < h {
                let idx = (py * w + px) as usize;
                slice[idx] = p.color;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::math::Vec3;
    use abrash_core::framebuffer::Framebuffer;
#[cfg(feature = "parallel")]
use rayon::prelude::*;

    #[test]
    fn test_lorenz_step() {
        let mut sys = StrangeAttractor::new(1, AttractorType::Lorenz { sigma: 10.0, rho: 28.0, beta: 8.0 / 3.0 }, 0.01);
        sys.particles[0].pos = Vec3::new(1.0, 1.0, 1.0);
        sys.step();

        let dx = 10.0 * (1.0 - 1.0) * 0.01;
        let dy = (1.0 * (28.0 - 1.0) - 1.0) * 0.01;
        let dz = (1.0 * 1.0 - (8.0 / 3.0) * 1.0) * 0.01;

        assert!((sys.particles[0].pos.x - (1.0 + dx)).abs() < 1e-4);
        assert!((sys.particles[0].pos.y - (1.0 + dy)).abs() < 1e-4);
        assert!((sys.particles[0].pos.z - (1.0 + dz)).abs() < 1e-4);
    }

    #[test]
    fn test_roessler_step() {
        let mut sys = StrangeAttractor::new(1, AttractorType::Roessler { a: 0.2, b: 0.2, c: 5.7 }, 0.01);
        sys.particles[0].pos = Vec3::new(1.0, 1.0, 1.0);
        sys.step();

        let dx = (-1.0 - 1.0) * 0.01;
        let dy = (1.0 + 0.2 * 1.0) * 0.01;
        let dz = (0.2 + 1.0 * (1.0 - 5.7)) * 0.01;

        assert!((sys.particles[0].pos.x - (1.0 + dx)).abs() < 1e-4);
        assert!((sys.particles[0].pos.y - (1.0 + dy)).abs() < 1e-4);
        assert!((sys.particles[0].pos.z - (1.0 + dz)).abs() < 1e-4);
    }

    #[test]
    fn test_render() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF000000); // clear to black
        let mut sys = StrangeAttractor::new(1, AttractorType::Lorenz { sigma: 10.0, rho: 28.0, beta: 8.0 / 3.0 }, 0.01);
        sys.particles[0].pos = Vec3::new(0.0, 0.0, 0.0);
        sys.render(&mut fb, 10.0, 50.0, 50.0);
        assert_eq!(fb.get_pixel(50, 50), Some(0xFFFFFFFF));
    }
}
