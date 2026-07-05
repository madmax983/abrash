//! Jelly: A Soft-Body Physics Simulation Module.
//!
//! This module implements a mass-spring system for simulating deformable objects ("Soft Bodies").
//! It converts a standard `Mesh` into a physical system where vertices are particles and edges are springs.

#![allow(warnings)]

use super::sdf::SdfScene;
use crate::math::{Vec3, Vec4};
use crate::mesh::Mesh;
use foldhash::{HashSet, HashSetExt};

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

/// Helper function to gather Vec3 components from SoA indices.
/// Used for fetching positions/velocities of spring endpoints.
#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[target_feature(enable = "avx2")]
unsafe fn gather_vec3s_avx(
    base_ptr: *const f32,
    indices: std::arch::x86_64::__m256i,
) -> (
    std::arch::x86_64::__m256,
    std::arch::x86_64::__m256,
    std::arch::x86_64::__m256,
) {
    use std::arch::x86_64::*;

    // Indices are vertex indices. Vec3 is 3 floats (12 bytes).
    // Gather offset = index * 3 * 4 (bytes) = index * 12.
    // Gather intrinsic scales indices by 4 (sizeof(float)). So we need index * 3.

    // Compute indices * 3
    // index * 3 = index * 2 + index = (index << 1) + index
    let idx_x3 = _mm256_add_epi32(_mm256_slli_epi32(indices, 1), indices);

    // Gather X components (offset + 0)
    let vx = _mm256_i32gather_ps(base_ptr, idx_x3, 4);

    // Gather Y components (offset + 1)
    // We can just add 1 to the gathered indices? No, gather takes offset.
    // We can add 4 bytes (1 float) to base pointer? Gathers are independent.
    // Yes, shifting base_ptr by 1 float is easiest.
    let vy = _mm256_i32gather_ps(base_ptr.add(1), idx_x3, 4);

    // Gather Z components (offset + 2)
    let vz = _mm256_i32gather_ps(base_ptr.add(2), idx_x3, 4);

    (vx, vy, vz)
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
    /// SoA: Indices of the first vertex in each spring.
    pub(crate) spring_indices_a: Vec<usize>,
    /// SoA: Indices of the second vertex in each spring.
    pub(crate) spring_indices_b: Vec<usize>,
    /// SoA: Rest length of each spring.
    pub(crate) spring_rest_lengths: Vec<f32>,
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
    /// Adds a structural or constraint spring between two existing vertices.
    pub fn add_spring(
        &mut self,
        index_a: usize,
        index_b: usize,
        rest_length: f32,
    ) -> Result<(), String> {
        let max_idx = self.mesh.vertices.len();
        if index_a >= max_idx || index_b >= max_idx {
            return Err(format!(
                "Spring indices out of bounds: {}, {}",
                index_a, index_b
            ));
        }
        self.spring_indices_a.push(index_a);
        self.spring_indices_b.push(index_b);
        self.spring_rest_lengths.push(rest_length);
        Ok(())
    }

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

        // ⚡ Bolt: Pre-allocate vectors and sets to prevent dynamic heap reallocations.
        // We know exactly the maximum possible number of edges/springs (3 per triangle).
        // Uses `foldhash::HashSet` instead of the standard library `HashSet` for simple integer tuple keys
        // to eliminate SipHash cryptographic overhead during edge deduplication.
        let max_edges = mesh.indices.len() * 3;
        let mut edges = HashSet::with_capacity(max_edges);
        let mut spring_indices_a = Vec::with_capacity(max_edges);
        let mut spring_indices_b = Vec::with_capacity(max_edges);
        let mut spring_rest_lengths = Vec::with_capacity(max_edges);

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

                    spring_indices_a.push(a);
                    spring_indices_b.push(b);
                    spring_rest_lengths.push(dist);
                }
            }
        }

        Ok(Self {
            mesh,
            velocities,
            forces,
            spring_indices_a,
            spring_indices_b,
            spring_rest_lengths,
            mass,
            stiffness,
            damping,
            drag: 0.01,
        })
    }

    /// Applies an external force to a specific vertex.
    /// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
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

        // Validate spring bounds
        for &idx in self
            .spring_indices_a
            .iter()
            .chain(self.spring_indices_b.iter())
        {
            if idx >= self.mesh.vertices.len() {
                eprintln!("SoftBody Error: Spring index {} out of bounds", idx);
                return;
            }
        }

        #[cfg(all(target_arch = "x86_64", feature = "simd"))]
        if is_x86_feature_detected!("avx2") {
            unsafe {
                self.update_simd(dt);
                return;
            }
        }

        self.update_scalar(dt);
    }

    fn update_scalar(&mut self, dt: f32) {
        let gravity = Vec3::new(0.0, -9.8, 0.0);

        // 1. Accumulate Forces
        let gravity_force = gravity * self.mass;
        for (force, velocity) in self.forces.iter_mut().zip(&self.velocities) {
            // Gravity + Air Drag
            // Note: force is assumed to be reset to zero at end of previous frame
            *force = *force + gravity_force - *velocity * self.drag;
        }

        // Spring Forces (SoA Scalar)
        for i in 0..self.spring_rest_lengths.len() {
            let idx_a = self.spring_indices_a[i];
            let idx_b = self.spring_indices_b[i];
            if idx_a >= self.mesh.vertices.len() || idx_b >= self.mesh.vertices.len() {
                continue;
            }
            let rest_len = self.spring_rest_lengths[i];

            let p_a = self.mesh.vertices[idx_a];
            let p_b = self.mesh.vertices[idx_b];
            let v_a = self.velocities[idx_a];
            let v_b = self.velocities[idx_b];

            let delta = p_b - p_a;
            let current_length = delta.length();

            if current_length > 0.0001 {
                // Optimization: reuse current_length to normalize, avoiding rsqrt/sqrt
                let direction = delta * (1.0 / current_length);

                // Hooke's Law: F = -k * (x - x0)
                let displacement = current_length - rest_len;
                let spring_force_mag = -self.stiffness * displacement;

                // Damping Force: Fd = -d * (v_rel . dir)
                let v_rel = v_b - v_a;
                let damping_force_mag = -self.damping * v_rel.dot(direction);

                let total_force = direction * (spring_force_mag + damping_force_mag);

                // Apply equal and opposite forces
                self.forces[idx_a] = self.forces[idx_a] - total_force;
                self.forces[idx_b] = self.forces[idx_b] + total_force;
            }
        }

        // 2. Integration (Semi-Implicit Euler)
        let mass_inv = 1.0 / self.mass;
        for ((vertex, velocity), force) in self
            .mesh
            .vertices
            .iter_mut()
            .zip(&mut self.velocities)
            .zip(&mut self.forces)
        {
            let accel = *force * mass_inv;
            *velocity = *velocity + accel * dt;
            *vertex = *vertex + *velocity * dt;

            // Reset force accumulator
            *force = Vec3::default();
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
        let epsilon = _mm256_set1_ps(0.0001);
        let stiffness_vec = _mm256_set1_ps(self.stiffness);
        let damping_vec = _mm256_set1_ps(self.damping);
        let neg_one = _mm256_set1_ps(-1.0);

        // 1. Accumulate Forces (Gravity + Drag)
        let len = self.mesh.vertices.len();
        let mut i = 0;

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

        // 2. Spring Forces (SIMD)
        let spring_count = self.spring_rest_lengths.len();
        let mut k = 0;

        // Pointers for gathering
        let pos_base = self.mesh.vertices.as_ptr() as *const f32;
        let vel_base = self.velocities.as_ptr() as *const f32;

        while k + 8 <= spring_count {
            // Load indices
            // We use usize in struct, but gather needs i32.
            // On 64-bit systems usize is 64-bit. We need to cast or load carefully.
            // Since we stored them as Vec<usize>, they are 8 bytes each.
            // _mm256_loadu_si256 loads 256 bits = 4 * 64 bits = 4 usizes.
            // We need 8 indices. So we need 2 loads per index array.

            let idx_a_ptr = self.spring_indices_a.as_ptr().add(k) as *const i64;
            let idx_b_ptr = self.spring_indices_b.as_ptr().add(k) as *const i64;

            // Load 8 usizes (i64) and pack into 8 i32s
            let idx_a_lo = _mm256_loadu_si256(idx_a_ptr as *const __m256i);
            let idx_a_hi = _mm256_loadu_si256(idx_a_ptr.add(4) as *const __m256i);

            let idx_b_lo = _mm256_loadu_si256(idx_b_ptr as *const __m256i);
            let idx_b_hi = _mm256_loadu_si256(idx_b_ptr.add(4) as *const __m256i);

            // Pack i64 to i32. There isn't a direct single instruction for 256->128 pack?
            // Use permutation/shuffle. Or just assume indices fit in i32 and simple shuffle.
            // Since mesh indices fit in 32 bits, we can take the lower 32 bits of each 64-bit int.
            // _mm256_cvtepi64_epi32? (AVX-512). AVX2 doesn't have it.
            // We can use shuffle to pick dwords 0, 2, 4, 6 from each 256-bit reg.

            // Helper to pack 8 i64s to 8 i32s
            let pack_i64_to_i32 = |lo: __m256i, hi: __m256i| -> __m256i {
                // lo: A0 A1 A2 A3 (64-bit)
                // hi: A4 A5 A6 A7
                // Shuffle to move lower 32 bits of each 64-bit element to contiguous block
                // 0x08 = 00 00 10 00 (idx 0, 2, ...) - actually shuffle works on 32-bit dwords.
                // 64-bit 0 is 32-bit (1, 0). We want 0.
                // 64-bit 1 is 32-bit (3, 2). We want 2.
                // Shuffle mask for one 128-bit lane: 0, 2, -, -? No.
                // _mm256_shuffle_epi32 is in-lane.
                // Maybe _mm256_permutevar8x32_epi32? (AVX2)
                let idx_perm = _mm256_setr_epi32(0, 2, 4, 6, 0, 0, 0, 0); // Grab even indices
                let lo_32 = _mm256_permutevar8x32_epi32(lo, idx_perm); // 0 1 2 3 x x x x
                let hi_32 = _mm256_permutevar8x32_epi32(hi, idx_perm); // 4 5 6 7 x x x x

                // lo_32: A0 A1 A2 A3 ...
                // hi_32: A4 A5 A6 A7 ...
                // We want to combine them.
                // Extract low 128 bits from each
                let lo_128 = _mm256_castsi256_si128(lo_32);
                let hi_128 = _mm256_castsi256_si128(hi_32);

                _mm256_insertf128_si256(_mm256_castsi128_si256(lo_128), hi_128, 1)
            };

            let idx_a = pack_i64_to_i32(idx_a_lo, idx_a_hi);
            let idx_b = pack_i64_to_i32(idx_b_lo, idx_b_hi);

            // Gather positions
            let (pa_x, pa_y, pa_z) = gather_vec3s_avx(pos_base, idx_a);
            let (pb_x, pb_y, pb_z) = gather_vec3s_avx(pos_base, idx_b);

            // Gather velocities
            let (va_x, va_y, va_z) = gather_vec3s_avx(vel_base, idx_a);
            let (vb_x, vb_y, vb_z) = gather_vec3s_avx(vel_base, idx_b);

            // Load rest lengths
            let rest_len = _mm256_loadu_ps(self.spring_rest_lengths.as_ptr().add(k));

            // Delta P = Pb - Pa
            let dx = _mm256_sub_ps(pb_x, pa_x);
            let dy = _mm256_sub_ps(pb_y, pa_y);
            let dz = _mm256_sub_ps(pb_z, pa_z);

            // Length sq = dx*dx + dy*dy + dz*dz
            let len_sq = _mm256_add_ps(
                _mm256_add_ps(_mm256_mul_ps(dx, dx), _mm256_mul_ps(dy, dy)),
                _mm256_mul_ps(dz, dz),
            );

            // Mask for length > epsilon
            let mask = _mm256_cmp_ps(len_sq, epsilon, _CMP_GT_OQ);

            // rsqrt(len_sq) -> 1/len
            let rlen = _mm256_rsqrt_ps(len_sq);
            // Refine rsqrt? Optional.

            // len = len_sq * rlen
            let current_len = _mm256_mul_ps(len_sq, rlen);

            // Normalized direction: dir = delta * rlen
            let dir_x = _mm256_mul_ps(dx, rlen);
            let dir_y = _mm256_mul_ps(dy, rlen);
            let dir_z = _mm256_mul_ps(dz, rlen);

            // Spring Force Mag = -k * (len - rest)
            let displacement = _mm256_sub_ps(current_len, rest_len);
            let spring_force_mag = _mm256_mul_ps(stiffness_vec, displacement);
            // Negate later or now? -k means pull.
            // force = dir * (-k * disp)
            let force_mag = _mm256_mul_ps(neg_one, spring_force_mag);

            // Damping Force
            // v_rel = vb - va
            let dv_x = _mm256_sub_ps(vb_x, va_x);
            let dv_y = _mm256_sub_ps(vb_y, va_y);
            let dv_z = _mm256_sub_ps(vb_z, va_z);

            // v_rel . dir
            let v_dot_dir = _mm256_add_ps(
                _mm256_add_ps(_mm256_mul_ps(dv_x, dir_x), _mm256_mul_ps(dv_y, dir_y)),
                _mm256_mul_ps(dv_z, dir_z),
            );

            // damping = -d * dot
            let damping_mag = _mm256_mul_ps(neg_one, _mm256_mul_ps(damping_vec, v_dot_dir));

            // Total mag
            let total_mag = _mm256_add_ps(force_mag, damping_mag);

            // Mask out invalid springs (zero length)
            let total_mag = _mm256_and_ps(total_mag, mask);

            // Force vector
            let tf_x = _mm256_mul_ps(dir_x, total_mag);
            let tf_y = _mm256_mul_ps(dir_y, total_mag);
            let tf_z = _mm256_mul_ps(dir_z, total_mag);

            // Accumulate forces
            // Scatter is not available on AVX2 for floats (scatterps is AVX512).
            // We must extract and accumulate scalar.
            // To be safe, we extract to stack arrays.

            let mut fx_arr = [0.0f32; 8];
            let mut fy_arr = [0.0f32; 8];
            let mut fz_arr = [0.0f32; 8];
            let mut idx_a_arr = [0i32; 8];
            let mut idx_b_arr = [0i32; 8];

            _mm256_storeu_ps(fx_arr.as_mut_ptr(), tf_x);
            _mm256_storeu_ps(fy_arr.as_mut_ptr(), tf_y);
            _mm256_storeu_ps(fz_arr.as_mut_ptr(), tf_z);
            _mm256_storeu_si256(idx_a_arr.as_mut_ptr() as *mut __m256i, idx_a);
            _mm256_storeu_si256(idx_b_arr.as_mut_ptr() as *mut __m256i, idx_b);

            // Scalar accumulation
            for j in 0..8 {
                let ia = idx_a_arr[j] as usize;
                let ib = idx_b_arr[j] as usize;
                let f = Vec3::new(fx_arr[j], fy_arr[j], fz_arr[j]);

                // Force on A -= total_force (action)
                // Force on B += total_force (reaction)
                self.forces[ia] = self.forces[ia] - f;
                self.forces[ib] = self.forces[ib] + f;
            }

            k += 8;
        }

        // Remainder loop
        while k < spring_count {
            let idx_a = self.spring_indices_a[k];
            let idx_b = self.spring_indices_b[k];
            let rest_len = self.spring_rest_lengths[k];

            let p_a = self.mesh.vertices[idx_a];
            let p_b = self.mesh.vertices[idx_b];
            let v_a = self.velocities[idx_a];
            let v_b = self.velocities[idx_b];

            let delta = p_b - p_a;
            let current_length = delta.length();

            if current_length > 0.0001 {
                let direction = delta * (1.0 / current_length);
                let displacement = current_length - rest_len;
                let spring_force_mag = -self.stiffness * displacement;
                let v_rel = v_b - v_a;
                let damping_force_mag = -self.damping * v_rel.dot(direction);
                let total_force = direction * (spring_force_mag + damping_force_mag);

                self.forces[idx_a] = self.forces[idx_a] - total_force;
                self.forces[idx_b] = self.forces[idx_b] + total_force;
            }
            k += 1;
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
        let mass_inv = 1.0 / self.mass;
        for ((vertex, velocity), force) in self.mesh.vertices[i..]
            .iter_mut()
            .zip(&mut self.velocities[i..])
            .zip(&mut self.forces[i..])
        {
            let accel = *force * mass_inv;
            *velocity = *velocity + accel * dt;
            *vertex = *vertex + *velocity * dt;
            *force = Vec3::default();
        }

        self.recompute_normals();
    }

    /// Resolves collisions with an SDF scene.
    pub fn collide_sdf(&mut self, scene: &SdfScene, restitution: f32) {
        if self.mesh.vertices.len() != self.velocities.len() {
            return;
        }

        for (pos_ptr, vel_ptr) in self
            .mesh
            .vertices
            .iter_mut()
            .zip(self.velocities.iter_mut())
        {
            let pos = *pos_ptr;
            let (dist, _) = scene.map(pos);

            if dist < 0.0 {
                // Collision!
                let normal = scene.normal(pos);
                let penetration = -dist;

                // Push out
                *pos_ptr = *pos_ptr + normal * penetration;

                // Reflect velocity
                // v_new = v - (1 + e) * (v . n) * n
                let v = *vel_ptr;
                let v_n = v.dot(normal);
                if v_n < 0.0 {
                    let j = -(1.0 + restitution) * v_n;
                    *vel_ptr = v + normal * j;

                    // Friction
                    // v_t = v - v_n * n
                    // v_t_new = v_t * (1 - friction)
                    let v_t = v - normal * v_n;
                    *vel_ptr = *vel_ptr - v_t * 0.1; // Simple friction
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

        for i in 0..self.spring_rest_lengths.len() {
            let idx_a = self.spring_indices_a[i];
            let idx_b = self.spring_indices_b[i];
            if idx_a >= self.mesh.vertices.len() || idx_b >= self.mesh.vertices.len() {
                continue;
            }
            let rest_len = self.spring_rest_lengths[i];

            let p_a = self.mesh.vertices[idx_a];
            let p_b = self.mesh.vertices[idx_b];
            let len = (p_b - p_a).length();
            let stretch = (len - rest_len).abs() / rest_len; // Strain

            stress[idx_a] += stretch;
            counts[idx_a] += 1;
            stress[idx_b] += stretch;
            counts[idx_b] += 1;
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
            let normal = edge1.cross(edge2).fast_normalize();

            normals[i0] = normals[i0] + normal;
            normals[i1] = normals[i1] + normal;
            normals[i2] = normals[i2] + normal;
        }

        // Normalize
        for n in normals {
            *n = n.fast_normalize();
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
        mesh.vertices.extend([
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ]);
        mesh.indices.push([0, 1, 2]);

        let jelly = SoftBody::new(mesh, 1.0, 10.0, 0.5).unwrap();

        // Should have 3 vertices
        assert_eq!(jelly.mesh.vertices.len(), 3);
        // Should have 3 edges (0-1, 1-2, 2-0) -> 3 springs
        assert_eq!(jelly.spring_rest_lengths.len(), 3);
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
