//! Hydro: Smoothed Particle Hydrodynamics (SPH) Simulation.
//!
//! This module implements a basic SPH solver for fluid simulation.
//! It simulates fluid as a collection of particles that interact via pressure and viscosity forces.
//! The simulation can be coupled with the `isosurface` module to render the fluid surface.

use crate::math::Vec3;
use std::f32::consts::PI;

/// A single particle in the fluid simulation.
#[derive(Clone, Copy, Debug)]
pub struct FluidParticle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub force: Vec3,
    pub density: f32,
    pub pressure: f32,
}

impl FluidParticle {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            velocity: Vec3::default(),
            force: Vec3::default(),
            density: 0.0,
            pressure: 0.0,
        }
    }
}

/// The SPH simulation system.
pub struct HydroSystem {
    pub particles: Vec<FluidParticle>,
    /// Smoothing length (h) - radius of influence.
    pub smoothing_length: f32,
    /// Target rest density (rho0).
    pub rest_density: f32,
    /// Stiffness constant for pressure (k).
    pub pressure_stiffness: f32,
    /// Viscosity coefficient (mu).
    pub viscosity: f32,
    /// Gravity vector.
    pub gravity: Vec3,
    /// Particle mass (assumed uniform).
    pub mass: f32,
}

impl HydroSystem {
    pub fn new(smoothing_length: f32, rest_density: f32) -> Self {
        Self {
            particles: Vec::new(),
            smoothing_length,
            rest_density,
            pressure_stiffness: 1000.0, // Default stiffness
            viscosity: 0.1,             // Default viscosity
            gravity: Vec3::new(0.0, -9.8, 0.0),
            mass: 1.0,
        }
    }

    /// Adds a particle to the system.
    pub fn add_particle(&mut self, position: Vec3) {
        self.particles.push(FluidParticle::new(position));
    }

    /// Steps the simulation by dt.
    pub fn step(&mut self, dt: f32) {
        // 1. Compute Density and Pressure
        self.compute_density_pressure();

        // 2. Compute Forces (Pressure, Viscosity, Gravity)
        self.compute_forces();

        // 3. Integrate (Semi-Implicit Euler)
        self.integrate(dt);

        // 4. Resolve Collisions (Boundary)
        self.resolve_collisions();
    }

    /// Returns a closure that evaluates the scalar field (density) at a given point.
    /// This is useful for isosurface extraction.
    ///
    /// # Note
    /// This captures the current state of particles. It is not efficient for large N,
    /// as it iterates all particles for every field evaluation (O(N) per voxel).
    /// For production, a spatial grid or BVH should be used.
    pub fn get_scalar_field(&self) -> impl Fn(Vec3) -> f32 + '_ {
        let h2 = self.smoothing_length * self.smoothing_length;
        let poly6_coeff = 315.0 / (64.0 * PI * self.smoothing_length.powi(9));
        let mass = self.mass;

        move |p: Vec3| {
            let mut density = 0.0;
            for particle in &self.particles {
                let r2 = (p - particle.position).length_sq();
                if r2 < h2 {
                    density += mass * poly6_coeff * (h2 - r2).powi(3);
                }
            }
            // Return negative density so that 0-isosurface works?
            // Usually isosurface is f(x) = 0.
            // If we want surface at density = threshold (e.g. rest_density/2),
            // then f(x) = threshold - density(x).
            // If density > threshold, f < 0 (inside).
            // If density < threshold, f > 0 (outside).
            // So extract_isosurface(threshold - density)
            // But let's just return raw density and let the caller handle thresholding
            // by subtracting threshold in the SDF wrapper.
            density
        }
    }

    fn compute_density_pressure(&mut self) {
        let h2 = self.smoothing_length * self.smoothing_length;
        let poly6_coeff = 315.0 / (64.0 * PI * self.smoothing_length.powi(9));

        for i in 0..self.particles.len() {
            let mut density = 0.0;
            let pi = self.particles[i].position;

            for j in 0..self.particles.len() {
                let pj = self.particles[j].position;
                let r2 = (pi - pj).length_sq();

                if r2 < h2 {
                    // Poly6 Kernel: W(r, h) = coeff * (h^2 - r^2)^3
                    density += self.mass * poly6_coeff * (h2 - r2).powi(3);
                }
            }

            self.particles[i].density = density.max(0.001); // Avoid division by zero
            // State Equation: P = k * (rho - rho0)
            self.particles[i].pressure = self.pressure_stiffness * (density - self.rest_density);
        }
    }

    fn compute_forces(&mut self) {
        let h = self.smoothing_length;
        let spiky_coeff = -45.0 / (PI * h.powi(6));
        let viscosity_coeff = 45.0 / (PI * h.powi(6)); // Laplacian of viscosity kernel

        // Reset forces
        for p in &mut self.particles {
            p.force = Vec3::default();
        }

        for i in 0..self.particles.len() {
            let mut f_pressure = Vec3::default();
            let mut f_viscosity = Vec3::default();

            let pi = self.particles[i];

            for j in 0..self.particles.len() {
                if i == j { continue; }

                let pj = self.particles[j];
                let dist_sq = (pi.position - pj.position).length_sq();
                let dist = dist_sq.sqrt();

                if dist < h && dist > 1e-6 {
                    // Pressure Force (Spiky Gradient)
                    let kernel_grad_mag = spiky_coeff * (h - dist).powi(2);
                    let dir = (pi.position - pj.position).normalize(); // r/|r| pointing away from j

                    let pressure_force_mag = -self.mass * (pi.pressure + pj.pressure) / (2.0 * pj.density) * kernel_grad_mag;
                    f_pressure = f_pressure + dir * pressure_force_mag;

                    // Viscosity Force
                    let laplacian = viscosity_coeff * (h - dist);
                    let vel_diff = pj.velocity - pi.velocity;
                    f_viscosity = f_viscosity + vel_diff * (self.mass / pj.density * laplacian);
                }
            }

            f_viscosity = f_viscosity * self.viscosity;

            // Add Gravity Force (Force Density = rho * g)
            let f_gravity = self.gravity * pi.density;

            self.particles[i].force = f_pressure + f_viscosity + f_gravity;
        }
    }

    fn integrate(&mut self, dt: f32) {
        for p in &mut self.particles {
            // a = F / density
            let accel = p.force / p.density;

            p.velocity = p.velocity + accel * dt;
            p.position = p.position + p.velocity * dt;
        }
    }

    fn resolve_collisions(&mut self) {
        // Simple box bounds [-5, 5]
        let bounds = 5.0;
        let restitution = 0.5;

        for p in &mut self.particles {
            if p.position.y < -bounds {
                p.position.y = -bounds;
                p.velocity.y *= -restitution;
            }
            if p.position.x < -bounds {
                p.position.x = -bounds;
                p.velocity.x *= -restitution;
            }
            if p.position.x > bounds {
                p.position.x = bounds;
                p.velocity.x *= -restitution;
            }
            if p.position.z < -bounds {
                p.position.z = -bounds;
                p.velocity.z *= -restitution;
            }
            if p.position.z > bounds {
                p.position.z = bounds;
                p.velocity.z *= -restitution;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gravity_advection() {
        let mut system = HydroSystem::new(1.0, 1000.0);
        system.add_particle(Vec3::new(0.0, 10.0, 0.0));

        system.step(0.1);

        let p = system.particles[0];
        assert!(p.position.y < 10.0, "Particle should fall");
    }

    #[test]
    fn test_density_accumulation() {
        let mut system = HydroSystem::new(1.0, 1000.0);
        system.mass = 1.0;

        system.add_particle(Vec3::new(0.0, 0.0, 0.0));

        // Compute density for 1 particle
        system.compute_density_pressure();
        let rho1 = system.particles[0].density;

        // Add neighbor
        system.add_particle(Vec3::new(0.1, 0.0, 0.0));

        // Compute density for 2 particles
        system.compute_density_pressure();
        let rho2 = system.particles[0].density;

        assert!(rho2 > rho1, "Density should increase with neighbors. 1: {}, 2: {}", rho1, rho2);
    }

    #[test]
    fn test_scalar_field() {
        let mut system = HydroSystem::new(1.0, 1000.0);
        system.mass = 1.0;
        system.add_particle(Vec3::new(0.0, 0.0, 0.0));

        let field = system.get_scalar_field();

        let density_center = field(Vec3::new(0.0, 0.0, 0.0));
        let density_far = field(Vec3::new(2.0, 0.0, 0.0));

        assert!(density_center > 0.0);
        assert_eq!(density_far, 0.0);
    }
}
