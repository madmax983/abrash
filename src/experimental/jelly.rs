//! Jelly: A Soft-Body Physics Simulation Module.
//!
//! This module implements a mass-spring system for simulating deformable objects ("Soft Bodies").
//! It converts a standard `Mesh` into a physical system where vertices are particles and edges are springs.

#![allow(warnings)]

use super::sdf::SdfScene;
use crate::math::{Vec3, Vec4};
use crate::mesh::Mesh;
use std::collections::HashSet;

/// A spring connecting two vertices.
#[derive(Debug, Clone, Copy)]
pub struct Spring {
    pub index_a: usize,
    pub index_b: usize,
    pub rest_length: f32,
}

/// A soft-body object that can simulate physics.
pub struct SoftBody {
    /// The visual mesh (updated every frame).
    pub mesh: Mesh,
    /// Velocity of each vertex.
    pub velocities: Vec<Vec3>,
    /// Accumulated forces on each vertex for the current frame.
    pub forces: Vec<Vec3>,
    /// List of springs connecting vertices.
    pub springs: Vec<Spring>,
    /// Mass of each vertex (uniform for now).
    pub mass: f32,
    /// Stiffness of springs (k).
    pub stiffness: f32,
    /// Damping factor for springs (d).
    pub damping: f32,
    /// Global drag (air resistance).
    pub drag: f32,
}

impl SoftBody {
    /// Creates a new `SoftBody` from a Mesh.
    ///
    /// Automatically generates springs from the mesh's unique edges.
    #[must_use]
    pub fn new(mesh: Mesh, mass: f32, stiffness: f32, damping: f32) -> Self {
        let vertex_count = mesh.vertices.len();
        let velocities = vec![Vec3::default(); vertex_count];
        let forces = vec![Vec3::default(); vertex_count];

        let mut edges = HashSet::new();
        let mut springs = Vec::new();

        for tri in &mesh.indices {
            let idxs = [tri[0], tri[1], tri[2]];

            // Generate edges (0-1, 1-2, 2-0)
            for i in 0..3 {
                let a = idxs[i];
                let b = idxs[(i + 1) % 3];

                // Sort indices to ensure uniqueness (min, max)
                let pair = if a < b { (a, b) } else { (b, a) };

                if edges.insert(pair) {
                    let p_a = mesh.vertices[a];
                    let p_b = mesh.vertices[b];
                    let dist = (p_b - p_a).length();

                    springs.push(Spring {
                        index_a: a,
                        index_b: b,
                        rest_length: dist,
                    });
                }
            }
        }

        Self {
            mesh,
            velocities,
            forces,
            springs,
            mass,
            stiffness,
            damping,
            drag: 0.01,
        }
    }

    /// Applies an external force to a specific vertex.
    pub fn apply_force(&mut self, index: usize, force: Vec3) {
        if index < self.forces.len() {
            self.forces[index] = self.forces[index] + force;
        }
    }

    /// Updates the physics simulation by one time step.
    pub fn update(&mut self, dt: f32) {
        let gravity = Vec3::new(0.0, -9.8, 0.0);

        // 1. Accumulate Forces
        for i in 0..self.mesh.vertices.len() {
            // Gravity
            self.forces[i] = self.forces[i] + gravity * self.mass;

            // Air Drag
            self.forces[i] = self.forces[i] - self.velocities[i] * self.drag;
        }

        // Spring Forces
        for spring in &self.springs {
            let p_a = self.mesh.vertices[spring.index_a];
            let p_b = self.mesh.vertices[spring.index_b];
            let v_a = self.velocities[spring.index_a];
            let v_b = self.velocities[spring.index_b];

            let delta = p_b - p_a;
            let current_length = delta.length();

            if current_length > 0.0001 {
                let direction = delta.normalize();

                // Hooke's Law: F = -k * (x - x0)
                let displacement = current_length - spring.rest_length;
                let spring_force_mag = -self.stiffness * displacement;

                // Damping Force: Fd = -d * (v_rel . dir)
                let v_rel = v_b - v_a;
                let damping_force_mag = -self.damping * v_rel.dot(direction);

                let total_force = direction * (spring_force_mag + damping_force_mag);

                // Apply equal and opposite forces
                self.forces[spring.index_a] = self.forces[spring.index_a] - total_force;
                self.forces[spring.index_b] = self.forces[spring.index_b] + total_force;
            }
        }

        // 2. Integration (Semi-Implicit Euler)
        for i in 0..self.mesh.vertices.len() {
            let accel = self.forces[i] * (1.0 / self.mass);
            self.velocities[i] = self.velocities[i] + accel * dt;
            self.mesh.vertices[i] = self.mesh.vertices[i] + self.velocities[i] * dt;

            // Reset force accumulator
            self.forces[i] = Vec3::default();
        }

        // 3. Recompute Normals for lighting
        self.recompute_normals();
    }

    /// Resolves collisions with an SDF scene.
    pub fn collide_sdf(&mut self, scene: &SdfScene, restitution: f32) {
        for i in 0..self.mesh.vertices.len() {
            let pos = self.mesh.vertices[i];
            let (dist, _) = scene.map(pos);

            if dist < 0.0 {
                // Collision!
                let normal = scene.normal(pos);
                let penetration = -dist;

                // Push out
                self.mesh.vertices[i] = self.mesh.vertices[i] + normal * penetration;

                // Reflect velocity
                // v_new = v - (1 + e) * (v . n) * n
                let v = self.velocities[i];
                let v_n = v.dot(normal);
                if v_n < 0.0 {
                    let j = -(1.0 + restitution) * v_n;
                    self.velocities[i] = v + normal * j;

                    // Friction
                    // v_t = v - v_n * n
                    // v_t_new = v_t * (1 - friction)
                    let v_t = v - normal * v_n;
                    self.velocities[i] = self.velocities[i] - v_t * 0.1; // Simple friction
                }
            }
        }
    }

    /// Calculates the average stress (strain) at each vertex.
    ///
    /// Returns a vector of stress values corresponding to `mesh.vertices`.
    /// Positive values indicate stretching, negative values indicate compression (if implemented, but here length is unsigned so stress is abs error).
    pub fn get_vertex_stress(&self) -> Vec<f32> {
        let mut stress = vec![0.0; self.mesh.vertices.len()];
        let mut counts = vec![0; self.mesh.vertices.len()];

        for spring in &self.springs {
            let p_a = self.mesh.vertices[spring.index_a];
            let p_b = self.mesh.vertices[spring.index_b];
            let len = (p_b - p_a).length();
            let stretch = (len - spring.rest_length).abs() / spring.rest_length; // Strain

            stress[spring.index_a] += stretch;
            counts[spring.index_a] += 1;
            stress[spring.index_b] += stretch;
            counts[spring.index_b] += 1;
        }

        for i in 0..stress.len() {
            if counts[i] > 0 {
                stress[i] /= counts[i] as f32;
            }
        }
        stress
    }

    /// Recomputes vertex normals based on current face geometry.
    fn recompute_normals(&mut self) {
        // Zero out normals
        let mut new_normals = vec![Vec3::default(); self.mesh.vertices.len()];

        // Accumulate face normals
        for tri in &self.mesh.indices {
            let i0 = tri[0];
            let i1 = tri[1];
            let i2 = tri[2];

            let v0 = self.mesh.vertices[i0];
            let v1 = self.mesh.vertices[i1];
            let v2 = self.mesh.vertices[i2];

            let edge1 = v1 - v0;
            let edge2 = v2 - v0;
            // Cross product: (v1-v0) x (v2-v0)
            let normal = edge1.cross(edge2).normalize();

            new_normals[i0] = new_normals[i0] + normal;
            new_normals[i1] = new_normals[i1] + normal;
            new_normals[i2] = new_normals[i2] + normal;
        }

        // Normalize
        for n in &mut new_normals {
            *n = n.normalize();
        }

        self.mesh.normals = new_normals;

        // Should ideally recompute tangents too if used, but skipping for now as it's expensive.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jelly_creation() {
        // Create a simple triangle
        let mut mesh = Mesh::new();
        mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
        mesh.vertices.push(Vec3::new(1.0, 0.0, 0.0));
        mesh.vertices.push(Vec3::new(0.0, 1.0, 0.0));
        mesh.indices.push([0, 1, 2]);

        let jelly = SoftBody::new(mesh, 1.0, 10.0, 0.5);

        // Should have 3 vertices
        assert_eq!(jelly.mesh.vertices.len(), 3);
        // Should have 3 edges (0-1, 1-2, 2-0) -> 3 springs
        assert_eq!(jelly.springs.len(), 3);
    }

    #[test]
    fn test_jelly_gravity() {
        let mut mesh = Mesh::new();
        mesh.vertices.push(Vec3::new(0.0, 10.0, 0.0)); // High up
        mesh.indices.push([0, 0, 0]); // Dummy index to keep struct valid, though springs won't form on degenerate triangle

        let mut jelly = SoftBody::new(mesh, 1.0, 10.0, 0.5);

        // Initial Y
        let y0 = jelly.mesh.vertices[0].y;

        // Update
        jelly.update(0.1);

        // New Y should be lower due to gravity
        let y1 = jelly.mesh.vertices[0].y;

        assert!(y1 < y0, "Vertex should fall due to gravity");
    }
}
