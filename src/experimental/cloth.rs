//! Cloth Simulation Module
//!
//! Implements a mass-spring system using Verlet integration for cloth simulation.

use crate::math::{Vec2, Vec3};
use crate::mesh::Mesh;

/// A single particle in the cloth grid.
#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub pos: Vec3,
    pub old_pos: Vec3,
    pub acc: Vec3,
    pub pinned: bool,
    pub uv: Vec2,
}

/// A structural constraint between two particles.
#[derive(Clone, Copy, Debug)]
pub struct Constraint {
    pub p1: usize,
    pub p2: usize,
    pub rest_length: f32,
}

/// A simulatable cloth object.
pub struct Cloth {
    pub particles: Vec<Particle>,
    pub constraints: Vec<Constraint>,
    pub width: usize,
    pub height: usize,
    /// Pre-calculated triangle indices for mesh generation.
    pub indices: Vec<[usize; 3]>,
}

impl Cloth {
    /// Creates a new cloth grid.
    ///
    /// # Arguments
    ///
    /// * `width` - Number of particles in X direction.
    /// * `height` - Number of particles in Y direction.
    /// * `spacing` - Distance between particles.
    ///
    /// ⚡ Bolt Optimization: Pre-allocates internal vectors (`particles`, `constraints`, `indices`)
    /// with exact capacities to prevent intermediate heap allocations and memory fragmentation
    /// during the initialization loops.
    #[must_use]
    pub fn new(width: usize, height: usize, spacing: f32) -> Self {
        let mut particles = Vec::with_capacity(width * height);

        let horizontal_constraints = (width.saturating_sub(1)) * height;
        let vertical_constraints = width * (height.saturating_sub(1));
        let shear_constraints = 2 * (width.saturating_sub(1)) * (height.saturating_sub(1));
        let num_constraints = horizontal_constraints + vertical_constraints + shear_constraints;
        let num_indices = 2 * (width.saturating_sub(1)) * (height.saturating_sub(1));

        let mut constraints = Vec::with_capacity(num_constraints);
        let mut indices = Vec::with_capacity(num_indices);

        // 1. Create Particles
        for y in 0..height {
            for x in 0..width {
                let pos = Vec3::new(
                    x as f32 * spacing - (width as f32 * spacing) / 2.0,
                    y as f32 * spacing - (height as f32 * spacing) / 2.0,
                    0.0,
                );

                // UVs from 0.0 to 1.0
                let uv = Vec2::new(
                    x as f32 / (width - 1) as f32,
                    y as f32 / (height - 1) as f32,
                );

                particles.push(Particle {
                    pos,
                    old_pos: pos,
                    acc: Vec3::default(),
                    pinned: false,
                    uv,
                });
            }
        }

        // 2. Create Constraints (Structural + Shear)
        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;

                // Horizontal (Right)
                if x < width - 1 {
                    constraints.push(Constraint {
                        p1: idx,
                        p2: idx + 1,
                        rest_length: spacing,
                    });
                }

                // Vertical (Down)
                if y < height - 1 {
                    constraints.push(Constraint {
                        p1: idx,
                        p2: idx + width,
                        rest_length: spacing,
                    });
                }

                // Shear (Diagonal Down-Right)
                if x < width - 1 && y < height - 1 {
                    let diag_dist = (spacing * spacing * 2.0).sqrt();
                    constraints.push(Constraint {
                        p1: idx,
                        p2: idx + width + 1,
                        rest_length: diag_dist,
                    });
                    // Shear (Diagonal Down-Left)
                    constraints.push(Constraint {
                        p1: idx + 1,
                        p2: idx + width,
                        rest_length: diag_dist,
                    });
                }
            }
        }

        // 3. Generate Indices
        for y in 0..height - 1 {
            for x in 0..width - 1 {
                let tl = y * width + x;
                let tr = tl + 1;
                let bl = tl + width;
                let br = bl + 1;

                // Two triangles per quad
                // Triangle 1: TL, TR, BL
                indices.push([tl, tr, bl]);
                // Triangle 2: TR, BR, BL
                indices.push([tr, br, bl]);
            }
        }

        Self {
            particles,
            constraints,
            width,
            height,
            indices,
        }
    }

    /// Pins a particle at the given coordinates.
    pub fn pin(&mut self, x: usize, y: usize) {
        if x < self.width && y < self.height {
            let idx = y * self.width + x;
            self.particles[idx].pinned = true;
        }
    }

    /// Unpins a particle at the given coordinates.
    pub fn unpin(&mut self, x: usize, y: usize) {
        if x < self.width && y < self.height {
            let idx = y * self.width + x;
            self.particles[idx].pinned = false;
        }
    }

    /// Updates the cloth simulation.
    pub fn update(&mut self, dt: f32, gravity: Vec3, wind: Vec3) {
        let friction = 0.99;

        // 1. Verlet Integration
        for p in &mut self.particles {
            if p.pinned {
                continue;
            }

            // F = ma => a = F/m (assume mass=1)
            // Add gravity and wind
            // Wind simple approximation: constant force
            // Better wind: dot product with normal, but simple is fine for now
            let total_acc = p.acc + gravity + wind; // + user forces

            let velocity = (p.pos - p.old_pos) * friction;
            p.old_pos = p.pos;
            p.pos = p.pos + velocity + total_acc * dt * dt;
            p.acc = Vec3::default(); // Reset acceleration
        }

        // 2. Constraint Relaxation
        let iterations = 5;
        for _ in 0..iterations {
            for c in &self.constraints {
                let p1 = self.particles[c.p1];
                let p2 = self.particles[c.p2];

                let delta = p2.pos - p1.pos;
                let dist = delta.length();

                if dist < 1e-6 {
                    continue; // Avoid division by zero
                }

                let diff = (dist - c.rest_length) / dist;
                let offset = delta * 0.5 * diff;

                let p1_pinned = p1.pinned;
                let p2_pinned = p2.pinned;

                if !p1_pinned && !p2_pinned {
                    self.particles[c.p1].pos = self.particles[c.p1].pos + offset;
                    self.particles[c.p2].pos = self.particles[c.p2].pos - offset;
                } else if !p1_pinned {
                    self.particles[c.p1].pos = self.particles[c.p1].pos + offset * 2.0;
                } else if !p2_pinned {
                    self.particles[c.p2].pos = self.particles[c.p2].pos - offset * 2.0;
                }
            }
        }
    }

    /// Converts the cloth to a Mesh.
    #[must_use]
    pub fn to_mesh(&self) -> Mesh {
        let mut mesh = Mesh::with_capacity(self.particles.len(), self.indices.len());
        mesh.indices.clone_from(&self.indices);

        for p in &self.particles {
            mesh.vertices.push(p.pos);
            mesh.uvs.push(p.uv);
            mesh.normals.push(Vec3::default());
        }

        for tri in &mesh.indices {
            let i0 = tri[0];
            let i1 = tri[1];
            let i2 = tri[2];

            let v0 = mesh.vertices[i0];
            let v1 = mesh.vertices[i1];
            let v2 = mesh.vertices[i2];

            let edge1 = v1 - v0;
            let edge2 = v2 - v0;
            let normal = edge1.cross(edge2).normalize();

            mesh.normals[i0] = mesh.normals[i0] + normal;
            mesh.normals[i1] = mesh.normals[i1] + normal;
            mesh.normals[i2] = mesh.normals[i2] + normal;
        }

        for n in &mut mesh.normals {
            *n = n.normalize();
        }

        mesh
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloth_initialization() {
        let cloth = Cloth::new(10, 10, 1.0);
        assert_eq!(cloth.particles.len(), 100);
        assert!(!cloth.constraints.is_empty());
        assert!(!cloth.indices.is_empty());
    }

    #[test]
    fn test_cloth_gravity() {
        let mut cloth = Cloth::new(2, 2, 1.0);
        // Pin top row
        cloth.pin(0, 1);
        cloth.pin(1, 1);

        // I want to verify bottom particle falls.
        let idx_bottom = 0; // (0,0)

        // Lift the bottom particle up so it can fall
        cloth.particles[idx_bottom].pos.y += 0.5;
        let y_lifted = cloth.particles[idx_bottom].pos.y;

        // Use a small timestep to avoid constraint over-correction instability
        for _ in 0..10 {
            cloth.update(0.016, Vec3::new(0.0, -9.8, 0.0), Vec3::default());
        }

        let y_end = cloth.particles[idx_bottom].pos.y;
        assert!(
            y_end < y_lifted,
            "Particle should fall from lifted position. Start: {}, End: {}",
            y_lifted,
            y_end
        );
    }
}
