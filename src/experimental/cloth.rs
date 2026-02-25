//! Cloth Simulation Module
//!
//! Implements a mass-spring system using Verlet integration for cloth simulation.

use crate::math::{Vec2, Vec3};
use crate::mesh::Mesh;

/// A structural constraint between two particles.
#[derive(Clone, Copy, Debug)]
pub struct Constraint {
    pub p1: usize,
    pub p2: usize,
    pub rest_length: f32,
}

/// A simulatable cloth object.
pub struct Cloth {
    // SoA Layout for Particles
    pub pos_x: Vec<f32>,
    pub pos_y: Vec<f32>,
    pub pos_z: Vec<f32>,

    pub old_pos_x: Vec<f32>,
    pub old_pos_y: Vec<f32>,
    pub old_pos_z: Vec<f32>,

    pub acc_x: Vec<f32>,
    pub acc_y: Vec<f32>,
    pub acc_z: Vec<f32>,

    /// Inverse mass of each particle. 0.0 = pinned/infinite mass. 1.0 = default.
    pub inv_mass: Vec<f32>,

    pub uv_x: Vec<f32>,
    pub uv_y: Vec<f32>,

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
    #[must_use]
    pub fn new(width: usize, height: usize, spacing: f32) -> Self {
        let capacity = width * height;

        let mut pos_x = Vec::with_capacity(capacity);
        let mut pos_y = Vec::with_capacity(capacity);
        let mut pos_z = Vec::with_capacity(capacity);

        let mut old_pos_x = Vec::with_capacity(capacity);
        let mut old_pos_y = Vec::with_capacity(capacity);
        let mut old_pos_z = Vec::with_capacity(capacity);

        let mut uv_x = Vec::with_capacity(capacity);
        let mut uv_y = Vec::with_capacity(capacity);

        let mut constraints = Vec::new();
        let mut indices = Vec::new();

        // 1. Create Particles
        for y in 0..height {
            for x in 0..width {
                let px = x as f32 * spacing - (width as f32 * spacing) / 2.0;
                let py = y as f32 * spacing - (height as f32 * spacing) / 2.0;
                let pz = 0.0;

                pos_x.push(px);
                pos_y.push(py);
                pos_z.push(pz);

                old_pos_x.push(px);
                old_pos_y.push(py);
                old_pos_z.push(pz);

                // UVs from 0.0 to 1.0
                let u = x as f32 / (width - 1) as f32;
                let v = y as f32 / (height - 1) as f32;

                uv_x.push(u);
                uv_y.push(v);
            }
        }

        let acc_x = vec![0.0; capacity];
        let acc_y = vec![0.0; capacity];
        let acc_z = vec![0.0; capacity];
        let inv_mass = vec![1.0; capacity];

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
            pos_x,
            pos_y,
            pos_z,
            old_pos_x,
            old_pos_y,
            old_pos_z,
            acc_x,
            acc_y,
            acc_z,
            inv_mass,
            uv_x,
            uv_y,
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
            self.inv_mass[idx] = 0.0;
        }
    }

    /// Unpins a particle at the given coordinates.
    pub fn unpin(&mut self, x: usize, y: usize) {
        if x < self.width && y < self.height {
            let idx = y * self.width + x;
            self.inv_mass[idx] = 1.0;
        }
    }

    /// Updates the cloth simulation.
    pub fn update(&mut self, dt: f32, gravity: Vec3, wind: Vec3) {
        self.integrate(dt, gravity, wind);
        self.relax_constraints();
    }

    fn integrate(&mut self, dt: f32, gravity: Vec3, wind: Vec3) {
        let count = self.pos_x.len();

        #[cfg(all(target_arch = "x86_64", feature = "simd"))]
        if std::is_x86_feature_detected!("avx2") {
            unsafe {
                self.integrate_avx2(dt, gravity, wind);
                return;
            }
        }

        let friction = 0.99;
        let dt_sq = dt * dt;

        for i in 0..count {
            let im = self.inv_mass[i];

            // F = ma => a = F/m (assume mass=1 for dynamics)
            let ax = self.acc_x[i] + gravity.x + wind.x;
            let ay = self.acc_y[i] + gravity.y + wind.y;
            let az = self.acc_z[i] + gravity.z + wind.z;

            let vx = (self.pos_x[i] - self.old_pos_x[i]) * friction;
            let vy = (self.pos_y[i] - self.old_pos_y[i]) * friction;
            let vz = (self.pos_z[i] - self.old_pos_z[i]) * friction;

            self.old_pos_x[i] = self.pos_x[i];
            self.old_pos_y[i] = self.pos_y[i];
            self.old_pos_z[i] = self.pos_z[i];

            // If pinned (im=0), change is 0.
            self.pos_x[i] += (vx + ax * dt_sq) * im;
            self.pos_y[i] += (vy + ay * dt_sq) * im;
            self.pos_z[i] += (vz + az * dt_sq) * im;

            self.acc_x[i] = 0.0;
            self.acc_y[i] = 0.0;
            self.acc_z[i] = 0.0;
        }
    }

    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    #[target_feature(enable = "avx2")]
    unsafe fn integrate_avx2(&mut self, dt: f32, gravity: Vec3, wind: Vec3) {
        use std::arch::x86_64::*;
        unsafe {

        let count = self.pos_x.len();
        let dt_sq = dt * dt;
        let dt_sq_v = _mm256_set1_ps(dt_sq);
        let friction = _mm256_set1_ps(0.99);

        let gx = _mm256_set1_ps(gravity.x + wind.x);
        let gy = _mm256_set1_ps(gravity.y + wind.y);
        let gz = _mm256_set1_ps(gravity.z + wind.z);

        // Process in chunks of 8
        let chunks = count / 8;
        for i in 0..chunks {
            let offset = i * 8;

            // Load pos
            let px_ptr = self.pos_x.as_ptr().add(offset);
            let py_ptr = self.pos_y.as_ptr().add(offset);
            let pz_ptr = self.pos_z.as_ptr().add(offset);

            let px = _mm256_loadu_ps(px_ptr);
            let py = _mm256_loadu_ps(py_ptr);
            let pz = _mm256_loadu_ps(pz_ptr);

            // Load old pos
            let opx_ptr = self.old_pos_x.as_ptr().add(offset);
            let opy_ptr = self.old_pos_y.as_ptr().add(offset);
            let opz_ptr = self.old_pos_z.as_ptr().add(offset);

            let opx = _mm256_loadu_ps(opx_ptr);
            let opy = _mm256_loadu_ps(opy_ptr);
            let opz = _mm256_loadu_ps(opz_ptr);

            // Load acc
            let ax_ptr = self.acc_x.as_ptr().add(offset);
            let ay_ptr = self.acc_y.as_ptr().add(offset);
            let az_ptr = self.acc_z.as_ptr().add(offset);

            let ax_in = _mm256_loadu_ps(ax_ptr);
            let ay_in = _mm256_loadu_ps(ay_ptr);
            let az_in = _mm256_loadu_ps(az_ptr);

            // Load inv mass
            let im_ptr = self.inv_mass.as_ptr().add(offset);
            let im = _mm256_loadu_ps(im_ptr);

            // Calc velocity
            // vx = (pos - old_pos) * friction
            let vx = _mm256_mul_ps(_mm256_sub_ps(px, opx), friction);
            let vy = _mm256_mul_ps(_mm256_sub_ps(py, opy), friction);
            let vz = _mm256_mul_ps(_mm256_sub_ps(pz, opz), friction);

            // Calc accel
            // a = acc + gravity + wind
            let ax = _mm256_add_ps(ax_in, gx);
            let ay = _mm256_add_ps(ay_in, gy);
            let az = _mm256_add_ps(az_in, gz);

            // Calc change
            // change = (vx + ax * dt_sq) * im
            // Use fmadd if available (AVX2 includes FMA usually, but strict AVX2 doesn't imply FMA3)
            // But _mm256_fmadd_ps is FMA3.
            // Safe fallback: a*b + c
            let cx = _mm256_mul_ps(_mm256_add_ps(vx, _mm256_mul_ps(ax, dt_sq_v)), im);
            let cy = _mm256_mul_ps(_mm256_add_ps(vy, _mm256_mul_ps(ay, dt_sq_v)), im);
            let cz = _mm256_mul_ps(_mm256_add_ps(vz, _mm256_mul_ps(az, dt_sq_v)), im);

            // Update old pos = pos
            _mm256_storeu_ps(opx_ptr as *mut f32, px);
            _mm256_storeu_ps(opy_ptr as *mut f32, py);
            _mm256_storeu_ps(opz_ptr as *mut f32, pz);

            // Update pos = pos + change
            let new_px = _mm256_add_ps(px, cx);
            let new_py = _mm256_add_ps(py, cy);
            let new_pz = _mm256_add_ps(pz, cz);

            _mm256_storeu_ps(px_ptr as *mut f32, new_px);
            _mm256_storeu_ps(py_ptr as *mut f32, new_py);
            _mm256_storeu_ps(pz_ptr as *mut f32, new_pz);

            // Reset acc
            let zero = _mm256_setzero_ps();
            _mm256_storeu_ps(ax_ptr as *mut f32, zero);
            _mm256_storeu_ps(ay_ptr as *mut f32, zero);
            _mm256_storeu_ps(az_ptr as *mut f32, zero);
        }

        // Remainder loop
        for i in (chunks * 8)..count {
            let im = self.inv_mass[i];
            let ax = self.acc_x[i] + gravity.x + wind.x;
            let ay = self.acc_y[i] + gravity.y + wind.y;
            let az = self.acc_z[i] + gravity.z + wind.z;

            let vx = (self.pos_x[i] - self.old_pos_x[i]) * 0.99;
            let vy = (self.pos_y[i] - self.old_pos_y[i]) * 0.99;
            let vz = (self.pos_z[i] - self.old_pos_z[i]) * 0.99;

            self.old_pos_x[i] = self.pos_x[i];
            self.old_pos_y[i] = self.pos_y[i];
            self.old_pos_z[i] = self.pos_z[i];

            self.pos_x[i] += (vx + ax * dt_sq) * im;
            self.pos_y[i] += (vy + ay * dt_sq) * im;
            self.pos_z[i] += (vz + az * dt_sq) * im;

            self.acc_x[i] = 0.0;
            self.acc_y[i] = 0.0;
            self.acc_z[i] = 0.0;
        }
        }
    }

    fn relax_constraints(&mut self) {
        let iterations = 5;
        for _ in 0..iterations {
            for c in &self.constraints {
                let p1 = c.p1;
                let p2 = c.p2;

                let w1 = self.inv_mass[p1];
                let w2 = self.inv_mass[p2];
                let w_sum = w1 + w2;

                if w_sum < 1e-6 {
                    continue;
                }

                let dx = self.pos_x[p2] - self.pos_x[p1];
                let dy = self.pos_y[p2] - self.pos_y[p1];
                let dz = self.pos_z[p2] - self.pos_z[p1];

                let dist_sq = dx * dx + dy * dy + dz * dz;
                let dist = dist_sq.sqrt();

                if dist < 1e-6 {
                    continue;
                }

                let correction = (dist - c.rest_length) / (dist * w_sum);

                let cx = dx * correction;
                let cy = dy * correction;
                let cz = dz * correction;

                self.pos_x[p1] += cx * w1;
                self.pos_y[p1] += cy * w1;
                self.pos_z[p1] += cz * w1;

                self.pos_x[p2] -= cx * w2;
                self.pos_y[p2] -= cy * w2;
                self.pos_z[p2] -= cz * w2;
            }
        }
    }

    /// Converts the cloth to a Mesh.
    #[must_use]
    pub fn to_mesh(&self) -> Mesh {
        let count = self.pos_x.len();
        let mut mesh = Mesh::new();
        mesh.vertices.reserve(count);
        mesh.uvs.reserve(count);
        mesh.indices.clone_from(&self.indices);

        for i in 0..count {
            mesh.vertices.push(Vec3::new(self.pos_x[i], self.pos_y[i], self.pos_z[i]));
            mesh.uvs.push(Vec2::new(self.uv_x[i], self.uv_y[i]));
        }

        // Compute normals
        mesh.normals = vec![Vec3::default(); count];

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
        assert_eq!(cloth.pos_x.len(), 100);
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
        cloth.pos_y[idx_bottom] += 0.5;
        let y_lifted = cloth.pos_y[idx_bottom];

        // Use a small timestep to avoid constraint over-correction instability
        for _ in 0..10 {
            cloth.update(0.016, Vec3::new(0.0, -9.8, 0.0), Vec3::default());
        }

        let y_end = cloth.pos_y[idx_bottom];
        assert!(
            y_end < y_lifted,
            "Particle should fall from lifted position. Start: {}, End: {}",
            y_lifted,
            y_end
        );
    }
}
