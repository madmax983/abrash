//! Fluid Simulation Module (Smoothed Particle Hydrodynamics).
//!
//! This module implements a basic 3D SPH solver for simulating fluids.
//! It uses a particle-based approach where density and pressure are computed
//! locally using smoothing kernels.
//!
//! It integrates with `isosurface` extraction to generate "Metaball" meshes
//! from the fluid density field.

use crate::experimental::isosurface::extract_isosurface;
use crate::math::Vec3;
use crate::mesh::Mesh;
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

/// SPH Fluid Solver.
pub struct FluidSolver {
    pub particles: Vec<FluidParticle>,

    // SPH Parameters
    pub smoothing_radius: f32, // h
    pub target_density: f32,   // rho0
    pub pressure_multiplier: f32, // k (gas constant)
    pub viscosity: f32,        // mu
    pub mass: f32,

    // Optimization: Spatial Hashing could be added here, but O(N^2) is fine for small demos.
}

impl FluidSolver {
    pub fn new(smoothing_radius: f32, mass: f32) -> Self {
        Self {
            particles: Vec::new(),
            smoothing_radius,
            target_density: 1000.0, // Water approx
            pressure_multiplier: 1000.0, // Stiffness
            viscosity: 10.0,
            mass,
        }
    }

    /// Adds a particle to the simulation.
    pub fn add_particle(&mut self, position: Vec3) {
        self.particles.push(FluidParticle::new(position));
    }

    /// Updates the simulation by one time step.
    pub fn update(&mut self, dt: f32) {
        // 1. Compute Density and Pressure
        // We need to mutate density/pressure, but read positions.
        let h2 = self.smoothing_radius * self.smoothing_radius;
        let poly6_coeff = 315.0 / (64.0 * PI * self.smoothing_radius.powi(9));

        for i in 0..self.particles.len() {
            let mut density = 0.0;
            let pi_pos = self.particles[i].position;

            for j in 0..self.particles.len() {
                let pj_pos = self.particles[j].position;
                let r2 = (pi_pos - pj_pos).length_sq();

                if r2 < h2 {
                    // Poly6 Kernel: (h^2 - r^2)^3
                    let diff = h2 - r2;
                    density += self.mass * poly6_coeff * diff * diff * diff;
                }
            }

            // Handle self-density or ensure minimum to avoid negative pressure issues
            self.particles[i].density = density.max(self.target_density);

            // Equation of State: P = k * (rho - rho0)
            self.particles[i].pressure = self.pressure_multiplier * (self.particles[i].density - self.target_density);
        }

        // 2. Compute Forces (Pressure + Viscosity)
        let spiky_grad_coeff = -45.0 / (PI * self.smoothing_radius.powi(6));
        let viscosity_lap_coeff = 45.0 / (PI * self.smoothing_radius.powi(6));

        // We'll accumulate into a temporary vector to avoid borrowing issues during iteration
        let mut forces = vec![Vec3::default(); self.particles.len()];

        for i in 0..self.particles.len() {
            let pi = self.particles[i];
            let mut pressure_force = Vec3::default();
            let mut viscosity_force = Vec3::default();

            for j in 0..self.particles.len() {
                if i == j { continue; }
                let pj = self.particles[j];

                let diff = pi.position - pj.position;
                let r = diff.length();

                if r < self.smoothing_radius && r > 0.0001 {
                    // Pressure Force (Spiky Gradient)
                    // Fp = - mass_j * (Pi + Pj) / (2 * rho_j) * Gradient(W)
                    // Gradient(Spiky) = -r_vec/r * (h-r)^2

                    let h_minus_r = self.smoothing_radius - r;
                    let kernel_grad = diff.normalize() * spiky_grad_coeff * h_minus_r * h_minus_r;

                    // Symmetrized pressure term
                    let p_term = (pi.pressure + pj.pressure) / (2.0 * pj.density);
                    pressure_force = pressure_force - kernel_grad * (self.mass * p_term);

                    // Viscosity Force (Viscosity Laplacian)
                    // Fv = mu * mass_j * (vj - vi) / rho_j * Laplacian(W)
                    // Laplacian(Viscosity) = (h-r)
                    let v_diff = pj.velocity - pi.velocity;
                    let kernel_lap = viscosity_lap_coeff * h_minus_r;

                    viscosity_force = viscosity_force + v_diff * (self.mass * kernel_lap / pj.density);
                }
            }

            forces[i] = pressure_force + viscosity_force * self.viscosity + Vec3::new(0.0, -9.8 * self.mass, 0.0);
        }

        // 3. Integrate
        for i in 0..self.particles.len() {
            let p = &mut self.particles[i];
            p.force = forces[i];

            // F = ma => a = F/m
            let accel = p.force * (1.0 / self.mass);

            p.velocity = p.velocity + accel * dt;
            p.position = p.position + p.velocity * dt;

            // Simple Boundary Conditions (Box)
            let box_size = 2.0;
            let restitution = 0.5;

            if p.position.y < -box_size {
                p.position.y = -box_size;
                p.velocity.y *= -restitution;
            }
        }
    }

    /// Evaluates the density field at a given position (for visualization).
    pub fn get_density_at(&self, pos: Vec3) -> f32 {
        let mut density = 0.0;
        let h2 = self.smoothing_radius * self.smoothing_radius;
        let poly6_coeff = 315.0 / (64.0 * PI * self.smoothing_radius.powi(9));

        for p in &self.particles {
            let r2 = (pos - p.position).length_sq();
            if r2 < h2 {
                let diff = h2 - r2;
                density += self.mass * poly6_coeff * diff * diff * diff;
            }
        }
        density
    }

    /// Converts the fluid to a mesh using Isosurface Extraction.
    pub fn to_mesh(&self, resolution: usize) -> Mesh {
        // Determine bounds based on particles
        if self.particles.is_empty() {
            return Mesh::new();
        }

        let mut min = self.particles[0].position;
        let mut max = self.particles[0].position;

        for p in &self.particles {
            min = min.min(p.position);
            max = max.max(p.position);
        }

        // Pad bounds
        let padding = self.smoothing_radius * 1.5;
        min = min - Vec3::new(padding, padding, padding);
        max = max + Vec3::new(padding, padding, padding);

        // Threshold for isosurface (e.g., half target density)
        let threshold = self.target_density * 0.5;

        // Isosurface extracts where val < 0 is inside.
        // We want Inside where density > threshold.
        // So we return (threshold - density).
        // If density > threshold, result is negative (Inside).
        // If density < threshold, result is positive (Outside).
        let sdf = |pos: Vec3| -> f32 {
            threshold - self.get_density_at(pos)
        };

        extract_isosurface(sdf, min, max, resolution)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fluid_initialization() {
        let mut fluid = FluidSolver::new(1.0, 1.0);
        fluid.add_particle(Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(fluid.particles.len(), 1);

        // Initial density/pressure should be 0 before update
        assert_eq!(fluid.particles[0].density, 0.0);
    }

    #[test]
    fn test_fluid_simulation_gravity() {
        let mut fluid = FluidSolver::new(1.0, 1.0);
        fluid.add_particle(Vec3::new(0.0, 10.0, 0.0)); // High up

        // Run simulation step
        fluid.update(0.1);

        // Should fall due to gravity
        // Initial force includes gravity = (0, -9.8, 0) * mass
        // Accel = -9.8
        // Vel = -0.98
        // Pos = 10.0 - 0.098

        let p = fluid.particles[0];
        assert!(p.position.y < 10.0, "Particle should fall");
        assert!((p.velocity.y - -0.98).abs() < 1e-5);
    }

    #[test]
    fn test_fluid_mesh_generation() {
        let mut fluid = FluidSolver::new(1.0, 1.0);

        // Add two particles close to each other
        fluid.add_particle(Vec3::new(-0.2, 0.0, 0.0));
        fluid.add_particle(Vec3::new(0.2, 0.0, 0.0));

        // Need to run update to compute densities?
        // Actually density calculation is part of `to_mesh`'s `get_density_at`.
        // `get_density_at` recomputes density on the fly for the grid point based on particle positions.
        // It does NOT depend on stored `p.density`.

        let mesh = fluid.to_mesh(10);

        // Should generate a metaball mesh around the two particles
        assert!(!mesh.vertices.is_empty(), "Mesh should have vertices");
        assert!(!mesh.indices.is_empty(), "Mesh should have triangles");
    }
}
