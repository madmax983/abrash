//! Jelly: A Soft-Body Physics Simulation Module.
//!
//! This module implements a mass-spring system for simulating deformable objects ("Soft Bodies").
//! It converts a standard `Mesh` into a physical system where vertices are particles and edges are springs.

#![allow(warnings)]

use crate::sdf::SdfScene;
use crate::math::{Vec3, Vec4};
use crate::mesh::Mesh;
use std::collections::HashSet;

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[target_feature(enable = "avx2")]
unsafe fn load_vec3s_avx(
    ptr: *const f32,
) -> (
    std::arch::x86_64::__m256,
    std::arch::x86_64::__m256,
    std::arch::x86_64::__m256,
) {
    use std::arch::x86_64::*;
    // Batch 1 (4 points)
    let r0 = _mm_loadu_ps(ptr); // x0 y0 z0 x1
    let r1 = _mm_loadu_ps(ptr.add(4)); // y1 z1 x2 y2
    let r2 = _mm_loadu_ps(ptr.add(8)); // z2 x3 y3 z3

    let t_x0x1 = _mm_shuffle_ps(r0, r0, 0b11_00_11_00); // x1 x0 x1 x0
    let t_x2x3 = _mm_shuffle_ps(r1, r2, 0b01_01_10_10); // x3 x2 x3 x2
    let x_lo = _mm_shuffle_ps(t_x0x1, t_x2x3, 0b10_00_01_00); // x3 x2 x1 x0

    let t_y0y1 = _mm_shuffle_ps(r0, r1, 0b00_00_01_01); // y1 y0 y1 y0
    let t_y2y3 = _mm_shuffle_ps(r1, r2, 0b10_10_11_11); // y3 y2 y3 y2
    let y_lo = _mm_shuffle_ps(t_y0y1, t_y2y3, 0b10_00_10_00); // y3 y2 y1 y0

    let t_z0z1 = _mm_shuffle_ps(r0, r1, 0b01_01_10_10); // z1 z0 z1 z0
    let t_z2z3 = _mm_shuffle_ps(r2, r2, 0b11_00_11_00); // z3 z2 z3 z2
    let z_lo = _mm_shuffle_ps(t_z0z1, t_z2z3, 0b01_00_10_00); // z3 z2 z1 z0

    // Batch 2 (4 points)
    let ptr2 = ptr.add(12);
    let r3 = _mm_loadu_ps(ptr2); // x4 y4 z4 x5
    let r4 = _mm_loadu_ps(ptr2.add(4)); // y5 z5 x6 y6
    let r5 = _mm_loadu_ps(ptr2.add(8)); // z6 x7 y7 z7

    let t_x4x5 = _mm_shuffle_ps(r3, r3, 0b11_00_11_00);
    let t_x6x7 = _mm_shuffle_ps(r4, r5, 0b01_01_10_10);
    let x_hi = _mm_shuffle_ps(t_x4x5, t_x6x7, 0b10_00_01_00);

    let t_y4y5 = _mm_shuffle_ps(r3, r4, 0b00_00_01_01);
    let t_y6y7 = _mm_shuffle_ps(r4, r5, 0b10_10_11_11);
    let y_hi = _mm_shuffle_ps(t_y4y5, t_y6y7, 0b10_00_10_00);

    let t_z4z5 = _mm_shuffle_ps(r3, r4, 0b01_01_10_10);
    let t_z6z7 = _mm_shuffle_ps(r5, r5, 0b11_00_11_00);
    let z_hi = _mm_shuffle_ps(t_z4z5, t_z6z7, 0b01_00_10_00);

    // Combine
    let vx = _mm256_insertf128_ps(_mm256_castps128_ps256(x_lo), x_hi, 1);
    let vy = _mm256_insertf128_ps(_mm256_castps128_ps256(y_lo), y_hi, 1);
    let vz = _mm256_insertf128_ps(_mm256_castps128_ps256(z_lo), z_hi, 1);

    (vx, vy, vz)
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[target_feature(enable = "avx2")]
unsafe fn store_vec3s_avx(
    ptr: *mut f32,
    vx: std::arch::x86_64::__m256,
    vy: std::arch::x86_64::__m256,
    vz: std::arch::x86_64::__m256,
) {
    use std::arch::x86_64::*;
    // Temporary buffer to scatter back to AoS
    let mut tmp = [0.0f32; 24]; // 8 * 3
    _mm256_storeu_ps(tmp.as_mut_ptr(), vx);
    _mm256_storeu_ps(tmp.as_mut_ptr().add(8), vy);
    _mm256_storeu_ps(tmp.as_mut_ptr().add(16), vz);

    for i in 0..8 {
        let p = ptr.add(i * 3);
        *p = tmp[i];
        *p.add(1) = tmp[i + 8];
        *p.add(2) = tmp[i + 16];
    }
}

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
    pub fn new(mesh: Mesh, mass: f32, stiffness: f32, damping: f32) -> Result<Self, String> {
        let vertex_count = mesh.vertices.len();
        let velocities = vec![Vec3::default(); vertex_count];
        let forces = vec![Vec3::default(); vertex_count];

        // Validate indices to prevent panics
        for (tri_idx, tri) in mesh.indices.iter().enumerate() {
            for &v_idx in tri.iter() {
                if v_idx >= vertex_count {
                    return Err(format!(
                        "Mesh index {} out of bounds (vertex count: {}) at triangle {}",
                        v_idx, vertex_count, tri_idx
                    ));
                }
            }
        }

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

        Ok(Self {
            mesh,
            velocities,
            forces,
            springs,
            mass,
            stiffness,
            damping,
            drag: 0.01,
        })
    }

    /// Applies an external force to a specific vertex.
    pub fn apply_force(&mut self, index: usize, force: Vec3) {
        if index < self.forces.len() {
            self.forces[index] = self.forces[index] + force;
        }
    }

    /// Updates the physics simulation by one time step.
    pub fn update(&mut self, dt: f32) {
        // Validation: Ensure mesh topology is compatible with physics state
        if self.mesh.vertices.len() != self.velocities.len()
            || self.mesh.vertices.len() != self.forces.len()
        {
            eprintln!(
                "SoftBody Error: Mesh vertex count ({}) mismatch with physics state (v:{}/f:{})",
                self.mesh.vertices.len(),
                self.velocities.len(),
                self.forces.len()
            );
            return;
        }

        #[cfg(all(target_arch = "x86_64", feature = "simd"))]
        if is_x86_feature_detected!("avx2") {
            unsafe {
                self.update_simd(dt);
                return;
            }
        }

        let gravity = Vec3::new(0.0, -9.8, 0.0);

        // 1. Accumulate Forces
        let gravity_force = gravity * self.mass;
        for (force, velocity) in self.forces.iter_mut().zip(&self.velocities) {
            // Gravity + Air Drag
            // Note: force is assumed to be reset to zero at end of previous frame
            *force = *force + gravity_force - *velocity * self.drag;
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
                // Optimization: reuse current_length to normalize, avoiding rsqrt/sqrt
                let direction = delta * (1.0 / current_length);

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

    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    #[target_feature(enable = "avx2")]
    unsafe fn update_simd(&mut self, dt: f32) {
        use std::arch::x86_64::*;
        // Ensure Vec3 layout is tightly packed (12 bytes) as assumed by SIMD loads
        {
            const _ASSERT: () = assert!(std::mem::size_of::<Vec3>() == 12);
        }

        let gravity_vec_y = _mm256_set1_ps(-9.8 * self.mass);
        let drag_vec = _mm256_set1_ps(self.drag);
        let mass_inv_vec = _mm256_set1_ps(1.0 / self.mass);
        let dt_vec = _mm256_set1_ps(dt);
        let zero = _mm256_setzero_ps();

        let len = self.mesh.vertices.len();
        let mut i = 0;

        // 1. Accumulate Forces (Gravity + Drag)
        while i + 8 <= len {
            // Load forces
            let f_ptr = self.forces.as_mut_ptr().add(i).cast::<f32>();
            let (mut fx, mut fy, mut fz) = load_vec3s_avx(f_ptr);

            // Load velocities
            let v_ptr = self.velocities.as_ptr().add(i).cast::<f32>();
            let (vx, vy, vz) = load_vec3s_avx(v_ptr);

            // Apply gravity (only to Y)
            fy = _mm256_add_ps(fy, gravity_vec_y);

            // Apply drag: F -= V * drag
            fx = _mm256_sub_ps(fx, _mm256_mul_ps(vx, drag_vec));
            fy = _mm256_sub_ps(fy, _mm256_mul_ps(vy, drag_vec));
            fz = _mm256_sub_ps(fz, _mm256_mul_ps(vz, drag_vec));

            // Store forces
            store_vec3s_avx(f_ptr, fx, fy, fz);

            i += 8;
        }

        // Remainder for step 1
        let gravity = Vec3::new(0.0, -9.8, 0.0);
        while i < len {
            self.forces[i] = self.forces[i] + gravity * self.mass;
            self.forces[i] = self.forces[i] - self.velocities[i] * self.drag;
            i += 1;
        }

        // 2. Spring Forces (Scalar fallback)
        for spring in &self.springs {
            let p_a = self.mesh.vertices[spring.index_a];
            let p_b = self.mesh.vertices[spring.index_b];
            let v_a = self.velocities[spring.index_a];
            let v_b = self.velocities[spring.index_b];

            let delta = p_b - p_a;
            let current_length = delta.length();

            if current_length > 0.0001 {
                let direction = delta.normalize();

                let displacement = current_length - spring.rest_length;
                let spring_force_mag = -self.stiffness * displacement;

                let v_rel = v_b - v_a;
                let damping_force_mag = -self.damping * v_rel.dot(direction);

                let total_force = direction * (spring_force_mag + damping_force_mag);

                self.forces[spring.index_a] = self.forces[spring.index_a] - total_force;
                self.forces[spring.index_b] = self.forces[spring.index_b] + total_force;
            }
        }

        // 3. Integration
        i = 0;
        while i + 8 <= len {
            let f_ptr = self.forces.as_mut_ptr().add(i).cast::<f32>();
            let (fx, fy, fz) = load_vec3s_avx(f_ptr);

            let v_ptr = self.velocities.as_mut_ptr().add(i).cast::<f32>();
            let (mut vx, mut vy, mut vz) = load_vec3s_avx(v_ptr);

            let p_ptr = self.mesh.vertices.as_mut_ptr().add(i).cast::<f32>();
            let (mut px, mut py, mut pz) = load_vec3s_avx(p_ptr);

            // Accel = F / m
            let ax = _mm256_mul_ps(fx, mass_inv_vec);
            let ay = _mm256_mul_ps(fy, mass_inv_vec);
            let az = _mm256_mul_ps(fz, mass_inv_vec);

            // V += A * dt
            vx = _mm256_add_ps(vx, _mm256_mul_ps(ax, dt_vec));
            vy = _mm256_add_ps(vy, _mm256_mul_ps(ay, dt_vec));
            vz = _mm256_add_ps(vz, _mm256_mul_ps(az, dt_vec));

            // P += V * dt
            px = _mm256_add_ps(px, _mm256_mul_ps(vx, dt_vec));
            py = _mm256_add_ps(py, _mm256_mul_ps(vy, dt_vec));
            pz = _mm256_add_ps(pz, _mm256_mul_ps(vz, dt_vec));

            // Store V and P
            store_vec3s_avx(v_ptr, vx, vy, vz);
            store_vec3s_avx(p_ptr, px, py, pz);

            // Reset Force
            store_vec3s_avx(f_ptr, zero, zero, zero);

            i += 8;
        }

        // Remainder for integration
        while i < len {
            let accel = self.forces[i] * (1.0 / self.mass);
            self.velocities[i] = self.velocities[i] + accel * dt;
            self.mesh.vertices[i] = self.mesh.vertices[i] + self.velocities[i] * dt;
            self.forces[i] = Vec3::default();
            i += 1;
        }

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
        let len = self.mesh.vertices.len();

        // Ensure normals vector is sized correctly
        if self.mesh.normals.len() != len {
            self.mesh.normals.resize(len, Vec3::default());
        }

        // Zero out normals efficiently
        self.mesh.normals.fill(Vec3::default());

        // We need to split borrow self.mesh to access indices (read) and normals (write) simultaneously.
        // This is safe because indices, vertices, and normals are distinct fields of Mesh.
        let mesh = &mut self.mesh;
        let indices = &mesh.indices;
        let vertices = &mesh.vertices;
        let normals = &mut mesh.normals;

        // Accumulate face normals
        for tri in indices {
            let i0 = tri[0];
            let i1 = tri[1];
            let i2 = tri[2];

            if i0 >= len || i1 >= len || i2 >= len {
                continue;
            }

            let v0 = vertices[i0];
            let v1 = vertices[i1];
            let v2 = vertices[i2];

            let edge1 = v1 - v0;
            let edge2 = v2 - v0;
            // Cross product: (v1-v0) x (v2-v0)
            let normal = edge1.cross(edge2).normalize();

            normals[i0] = normals[i0] + normal;
            normals[i1] = normals[i1] + normal;
            normals[i2] = normals[i2] + normal;
        }

        // Normalize
        for n in normals {
            *n = n.normalize();
        }

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

        let jelly = SoftBody::new(mesh, 1.0, 10.0, 0.5).unwrap();

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

        let mut jelly = SoftBody::new(mesh, 1.0, 10.0, 0.5).unwrap();

        // Initial Y
        let y0 = jelly.mesh.vertices[0].y;

        // Update
        jelly.update(0.1);

        // New Y should be lower due to gravity
        let y1 = jelly.mesh.vertices[0].y;

        assert!(y1 < y0, "Vertex should fall due to gravity");
    }
}

#[cfg(test)]
mod correctness_tests {
    use super::*;

    #[test]
    fn test_update_correctness() {
        let mut mesh = Mesh::new();
        // A single vertex at origin
        mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
        // Add a dummy triangle to satisfy indices check (0, 0, 0)
        mesh.indices.push([0, 0, 0]);

        let mut jelly = SoftBody::new(mesh, 1.0, 10.0, 0.5).unwrap();

        // Initial state
        assert_eq!(jelly.velocities[0], Vec3::default());
        assert_eq!(jelly.forces[0], Vec3::default());

        // Update 1 step
        // Gravity is -9.8 y. Mass is 1.0. Force = (0, -9.8, 0).
        // Drag is 0 (velocity is 0).
        // Accel = F / m = (0, -9.8, 0).
        // Velocity += Accel * dt = (0, -9.8 * 0.1, 0) = (0, -0.98, 0).
        // Position += Velocity * dt = (0, -0.98 * 0.1, 0) = (0, -0.098, 0).
        // Force reset to 0.
        jelly.update(0.1);

        let v = jelly.velocities[0];
        let p = jelly.mesh.vertices[0];

        assert!((v.y - -0.98).abs() < 1e-5, "Velocity Y mismatch: {}", v.y);
        assert!((p.y - -0.098).abs() < 1e-5, "Position Y mismatch: {}", p.y);
        assert_eq!(jelly.forces[0], Vec3::default());
    }
}
