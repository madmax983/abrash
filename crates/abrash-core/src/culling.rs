//! Frustum Culling primitives.

use crate::geometry::{AABB, BoundingSphere};
use crate::math::{Mat4, Vec3};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use std::arch::x86_64::{
    _CMP_LT_OQ, _mm256_add_ps, _mm256_andnot_si256, _mm256_castps_si256, _mm256_castsi256_ps,
    _mm256_cmp_ps, _mm256_loadu_ps, _mm256_movemask_ps, _mm256_mul_ps, _mm256_set1_epi32,
    _mm256_set1_ps, _mm256_setzero_ps, _mm256_unpackhi_ps, _mm256_unpacklo_ps,
};

/// A geometric plane defined by a normal and a distance from the origin.
/// Equation: `normal . point + distance = 0`
#[derive(Debug, Clone, Copy)]
pub struct Plane {
    /// The normal vector of the plane.
    pub normal: Vec3,
    /// The distance from the origin to the plane along its normal.
    pub distance: f32,
}

impl Plane {
    /// Normalize the plane equation so that the normal has length 1.
    ///
    /// This is necessary because plane extraction from a matrix produces planes with non-unit normal vectors.
    /// Normalizing the planes allows the frustum to calculate accurate distances to points and bounding volumes.
    pub fn normalize(&mut self) {
        let len = self.normal.length();
        if len > 0.0001 {
            let inv_len = 1.0 / len;
            self.normal = self.normal * inv_len;
            self.distance *= inv_len;
        }
    }

    /// Calculates the shortest signed distance from a point to the plane.
    ///
    /// The distance is positive if the point is completely inside the frustum.
    /// If the point is outside the frustum, the distance will be negative.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::culling::Plane;
    /// use abrash_core::math::Vec3;
    ///
    /// let mut plane = Plane {
    ///     normal: Vec3::new(1.0, 0.0, 0.0),
    ///     distance: 0.0,
    /// };
    /// plane.normalize();
    ///
    /// let point = Vec3::new(1.0, 0.0, 0.0);
    /// let dist = plane.distance_to_point(point);
    /// assert!(dist > 0.0);
    /// ```
    #[must_use]
    pub fn distance_to_point(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.distance
    }
}

/// A View Frustum defined by 6 planes.
/// Used for object-level culling.
pub struct Frustum {
    /// The 6 planes that define the view frustum (Left, Right, Bottom, Top, Near, Far).
    pub planes: [Plane; 6],
}

impl Frustum {
    /// Extract frustum planes from View-Projection matrix.
    /// Assumes Row-Major matrix where `v_clip = v_world * M`.
    ///
    /// The planes are extracted such that the normal points **inside** the frustum.
    #[must_use]
    pub fn from_matrix(m: Mat4) -> Self {
        // In Row-Vector convention v' = v * M,
        // x' = v . Col0
        // y' = v . Col1
        // z' = v . Col2
        // w' = v . Col3
        let c0 = m.col(0);
        let c1 = m.col(1);
        let c2 = m.col(2);
        let c3 = m.col(3);

        // Frustum planes in Clip Space: -w <= x,y,z <= w
        // Left:   x >= -w  => x + w >= 0
        // Right:  x <= w   => w - x >= 0
        // Bottom: y >= -w  => y + w >= 0
        // Top:    y <= w   => w - y >= 0
        // Near:   z >= -w  => z + w >= 0
        // Far:    z <= w   => w - z >= 0

        let mut planes = [
            // Left
            Plane {
                normal: Vec3::new(c3.x + c0.x, c3.y + c0.y, c3.z + c0.z),
                distance: c3.w + c0.w,
            },
            // Right
            Plane {
                normal: Vec3::new(c3.x - c0.x, c3.y - c0.y, c3.z - c0.z),
                distance: c3.w - c0.w,
            },
            // Bottom
            Plane {
                normal: Vec3::new(c3.x + c1.x, c3.y + c1.y, c3.z + c1.z),
                distance: c3.w + c1.w,
            },
            // Top
            Plane {
                normal: Vec3::new(c3.x - c1.x, c3.y - c1.y, c3.z - c1.z),
                distance: c3.w - c1.w,
            },
            // Near
            Plane {
                normal: Vec3::new(c3.x + c2.x, c3.y + c2.y, c3.z + c2.z),
                distance: c3.w + c2.w,
            },
            // Far
            Plane {
                normal: Vec3::new(c3.x - c2.x, c3.y - c2.y, c3.z - c2.z),
                distance: c3.w - c2.w,
            },
        ];

        for p in &mut planes {
            p.normalize();
        }

        Self { planes }
    }

    /// Check if a sphere intersects or is inside the frustum.
    /// Returns `true` if the sphere is visible (partially or fully).
    /// Returns `false` if the sphere is fully outside any plane.
    #[must_use]
    pub fn intersects(&self, sphere: &BoundingSphere) -> bool {
        for plane in &self.planes {
            // Distance is positive inside, negative outside.
            // If center distance is less than -radius, it's fully outside.
            if plane.distance_to_point(sphere.center) < -sphere.radius {
                return false;
            }
        }
        true
    }

    /// Check if an AABB intersects or is inside the frustum.
    /// uses the p-vertex optimization.
    #[must_use]
    pub fn intersects_aabb(&self, aabb: &AABB) -> bool {
        for plane in &self.planes {
            // Find the p-vertex (the vertex furthest along the normal direction)
            // If this vertex is behind the plane (negative distance), the whole box is outside.
            let px = if plane.normal.x >= 0.0 {
                aabb.max.x
            } else {
                aabb.min.x
            };
            let py = if plane.normal.y >= 0.0 {
                aabb.max.y
            } else {
                aabb.min.y
            };
            let pz = if plane.normal.z >= 0.0 {
                aabb.max.z
            } else {
                aabb.min.z
            };

            let dist =
                plane.normal.x * px + plane.normal.y * py + plane.normal.z * pz + plane.distance;

            if dist < 0.0 {
                return false;
            }
        }
        true
    }

    /// Check multiple spheres against the frustum using SIMD optimizations.
    /// Returns a `Vec<bool>` where `true` means the sphere is visible.
    #[must_use]
    pub fn cull_spheres(&self, spheres: &[BoundingSphere]) -> Vec<bool> {
        let mut results = vec![true; spheres.len()];
        self.cull_spheres_prealloc(spheres, &mut results);
        results
    }

    /// Check multiple spheres against the frustum using SIMD optimizations.
    /// Writes results into the provided slice.
    ///
    /// The `results` slice must be the same length as `spheres`.
    ///
    /// # Panics
    ///
    /// Panics if `spheres.len() != results.len()`.
    pub fn cull_spheres_prealloc(&self, spheres: &[BoundingSphere], results: &mut [bool]) {
        assert_eq!(spheres.len(), results.len());

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        if is_x86_feature_detected!("avx2") {
            unsafe {
                self.cull_spheres_avx2(spheres, results);
            }
            return;
        }

        // Scalar fallback
        for (i, sphere) in spheres.iter().enumerate() {
            results[i] = self.intersects(sphere);
        }
    }

    /// Check multiple AABBs against the frustum.
    /// Writes results into the provided slice.
    ///
    /// # Panics
    ///
    /// Panics if `aabbs.len() != results.len()`.
    pub fn cull_aabbs_prealloc(&self, aabbs: &[AABB], results: &mut [bool]) {
        assert_eq!(aabbs.len(), results.len());

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        if is_x86_feature_detected!("avx2") {
            unsafe {
                self.cull_aabbs_avx2(aabbs, results);
            }
            return;
        }

        for (i, aabb) in aabbs.iter().enumerate() {
            results[i] = self.intersects_aabb(aabb);
        }
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "avx2")]
    unsafe fn cull_spheres_avx2(&self, spheres: &[BoundingSphere], results: &mut [bool]) {
        let len = spheres.len();
        let mut i = 0;

        // Prepare Plane vectors
        // We broadcast plane components to AVX vectors
        // SAFETY: intrinsics are unsafe but guarded by target_feature/is_x86_feature_detected
        unsafe {
            let mut plane_x = [_mm256_setzero_ps(); 6];
            let mut plane_y = [_mm256_setzero_ps(); 6];
            let mut plane_z = [_mm256_setzero_ps(); 6];
            let mut plane_d = [_mm256_setzero_ps(); 6];

            for (j, plane) in self.planes.iter().enumerate() {
                plane_x[j] = _mm256_set1_ps(plane.normal.x);
                plane_y[j] = _mm256_set1_ps(plane.normal.y);
                plane_z[j] = _mm256_set1_ps(plane.normal.z);
                plane_d[j] = _mm256_set1_ps(plane.distance);
            }

            while i + 8 <= len {
                let ptr = spheres.as_ptr().add(i).cast::<f32>();

                // Load 8 spheres (4 registers of 2 spheres each)
                // Each BoundingSphere is 4 floats: x, y, z, r
                let row0 = _mm256_loadu_ps(ptr);
                let row1 = _mm256_loadu_ps(ptr.add(8));
                let row2 = _mm256_loadu_ps(ptr.add(16));
                let row3 = _mm256_loadu_ps(ptr.add(24));

                // Transpose to get structure of arrays
                // Step 1: Interleave low/high 128-bit lanes
                // unpacklo(row0, row1) -> x0 x2 y0 y2 | x1 x3 y1 y3
                let tmp0 = _mm256_unpacklo_ps(row0, row1);
                let tmp1 = _mm256_unpackhi_ps(row0, row1);
                let tmp2 = _mm256_unpacklo_ps(row2, row3);
                let tmp3 = _mm256_unpackhi_ps(row2, row3);

                // Step 2: Interleave again to get xxxx yyyy
                let t0 = _mm256_unpacklo_ps(tmp0, tmp2); // x0 x4 x2 x6 | x1 x5 x3 x7
                let t1 = _mm256_unpackhi_ps(tmp0, tmp2); // y0 y4 y2 y6 | y1 y5 y3 y7
                let t2 = _mm256_unpacklo_ps(tmp1, tmp3); // z0 z4 z2 z6 | z1 z5 z3 z7
                let t3 = _mm256_unpackhi_ps(tmp1, tmp3); // r0 r4 r2 r6 | r1 r5 r3 r7

                let x_vec = t0;
                let y_vec = t1;
                let z_vec = t2;
                let r_vec = t3;

                let mut active_mask = _mm256_set1_epi32(-1); // All ones (true)

                for k in 0..6 {
                    // dot = x*nx + y*ny + z*nz + d
                    // Using mul/add sequence to be safe without FMA
                    let term_x = _mm256_mul_ps(x_vec, plane_x[k]);
                    let term_y = _mm256_mul_ps(y_vec, plane_y[k]);
                    let term_z = _mm256_mul_ps(z_vec, plane_z[k]);

                    let dot = _mm256_add_ps(
                        term_x,
                        _mm256_add_ps(term_y, _mm256_add_ps(term_z, plane_d[k])),
                    );

                    let val = _mm256_add_ps(dot, r_vec);

                    // If val < 0, it is outside (culled)
                    // _CMP_LT_OQ check
                    let outside = _mm256_cmp_ps(val, _mm256_setzero_ps(), _CMP_LT_OQ);
                    let outside_int = _mm256_castps_si256(outside);

                    // If outside, sphere is not visible
                    active_mask = _mm256_andnot_si256(outside_int, active_mask);
                }

                let mask_bits = _mm256_movemask_ps(_mm256_castsi256_ps(active_mask));

                // Map permuted indices back to 0..7
                // Permutation was: 0, 4, 2, 6, 1, 5, 3, 7
                results[i] = (mask_bits & 1) != 0;
                results[i + 4] = (mask_bits & 2) != 0;
                results[i + 2] = (mask_bits & 4) != 0;
                results[i + 6] = (mask_bits & 8) != 0;
                results[i + 1] = (mask_bits & 16) != 0;
                results[i + 5] = (mask_bits & 32) != 0;
                results[i + 3] = (mask_bits & 64) != 0;
                results[i + 7] = (mask_bits & 128) != 0;

                i += 8;
            }
        }

        // Tail
        while i < len {
            results[i] = self.intersects(&spheres[i]);
            i += 1;
        }
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "avx2")]
    unsafe fn cull_aabbs_avx2(&self, aabbs: &[AABB], results: &mut [bool]) {
        use std::arch::x86_64::{
            _CMP_GE_OQ, _CMP_LT_OQ, _mm256_add_ps, _mm256_andnot_si256, _mm256_blendv_ps,
            _mm256_castpd_ps, _mm256_castps_pd, _mm256_castps_si256, _mm256_castsi256_ps,
            _mm256_cmp_ps, _mm256_loadu_ps, _mm256_movemask_ps, _mm256_mul_ps,
            _mm256_permute2f128_ps, _mm256_set1_epi32, _mm256_set1_ps, _mm256_setzero_ps,
            _mm256_unpackhi_pd, _mm256_unpackhi_ps, _mm256_unpacklo_pd, _mm256_unpacklo_ps,
        };

        let len = aabbs.len();
        let mut i = 0;

        // Prepare Plane vectors
        unsafe {
            let mut plane_x = [_mm256_setzero_ps(); 6];
            let mut plane_y = [_mm256_setzero_ps(); 6];
            let mut plane_z = [_mm256_setzero_ps(); 6];
            let mut plane_d = [_mm256_setzero_ps(); 6];

            for (j, plane) in self.planes.iter().enumerate() {
                plane_x[j] = _mm256_set1_ps(plane.normal.x);
                plane_y[j] = _mm256_set1_ps(plane.normal.y);
                plane_z[j] = _mm256_set1_ps(plane.normal.z);
                plane_d[j] = _mm256_set1_ps(plane.distance);
            }

            let zero = _mm256_setzero_ps();

            while i + 8 <= len {
                let ptr = aabbs.as_ptr().add(i).cast::<f32>();

                // Load 8 AABBs (8 * 8 floats = 64 floats)
                // AABB layout: [min_x, min_y, min_z, pad0, max_x, max_y, max_z, pad1]
                let r0 = _mm256_loadu_ps(ptr);
                let r1 = _mm256_loadu_ps(ptr.add(8));
                let r2 = _mm256_loadu_ps(ptr.add(16));
                let r3 = _mm256_loadu_ps(ptr.add(24));
                let r4 = _mm256_loadu_ps(ptr.add(32));
                let r5 = _mm256_loadu_ps(ptr.add(40));
                let r6 = _mm256_loadu_ps(ptr.add(48));
                let r7 = _mm256_loadu_ps(ptr.add(56));

                // Transpose 8x8 using _mm256_unpack and _mm256_permute
                // Stage 1: unpacklo/hi (32-bit granularity)
                let t0 = _mm256_unpacklo_ps(r0, r1); // 0a 1a 0b 1b ...
                let t1 = _mm256_unpackhi_ps(r0, r1);
                let t2 = _mm256_unpacklo_ps(r2, r3);
                let t3 = _mm256_unpackhi_ps(r2, r3);
                let t4 = _mm256_unpacklo_ps(r4, r5);
                let t5 = _mm256_unpackhi_ps(r4, r5);
                let t6 = _mm256_unpacklo_ps(r6, r7);
                let t7 = _mm256_unpackhi_ps(r6, r7);

                // Stage 2: unpacklo/hi (64-bit granularity)
                // Use doubles to shuffle 64-bit blocks
                let t0_d = _mm256_castps_pd(t0);
                let t2_d = _mm256_castps_pd(t2);
                let t1_d = _mm256_castps_pd(t1);
                let t3_d = _mm256_castps_pd(t3);
                let t4_d = _mm256_castps_pd(t4);
                let t6_d = _mm256_castps_pd(t6);
                let t5_d = _mm256_castps_pd(t5);
                let t7_d = _mm256_castps_pd(t7);

                let u0 = _mm256_castpd_ps(_mm256_unpacklo_pd(t0_d, t2_d));
                let u1 = _mm256_castpd_ps(_mm256_unpackhi_pd(t0_d, t2_d));
                let u2 = _mm256_castpd_ps(_mm256_unpacklo_pd(t1_d, t3_d));
                let u3 = _mm256_castpd_ps(_mm256_unpackhi_pd(t1_d, t3_d));
                let u4 = _mm256_castpd_ps(_mm256_unpacklo_pd(t4_d, t6_d));
                let u5 = _mm256_castpd_ps(_mm256_unpackhi_pd(t4_d, t6_d));
                let u6 = _mm256_castpd_ps(_mm256_unpacklo_pd(t5_d, t7_d));
                let u7 = _mm256_castpd_ps(_mm256_unpackhi_pd(t5_d, t7_d));

                // Stage 3: permute2f128 (128-bit granularity) to swap AVX lanes
                let v0 = _mm256_permute2f128_ps(u0, u4, 0x20); // min_x
                let v1 = _mm256_permute2f128_ps(u0, u4, 0x31); // min_y
                let v2 = _mm256_permute2f128_ps(u1, u5, 0x20); // min_z
                // let v3 = ... pad0 (ignored)
                let v4 = _mm256_permute2f128_ps(u2, u6, 0x20); // max_x
                let v5 = _mm256_permute2f128_ps(u2, u6, 0x31); // max_y
                let v6 = _mm256_permute2f128_ps(u3, u7, 0x20); // max_z

                // Registers now contain SoA data for 8 AABBs
                let min_x = v0;
                let min_y = v1;
                let min_z = v2;
                let max_x = v4;
                let max_y = v5;
                let max_z = v6;

                let mut active_mask = _mm256_set1_epi32(-1);

                for k in 0..6 {
                    // Select p-vertex based on normal sign
                    // px = if n.x >= 0 { max.x } else { min.x }
                    let mask_pos_x = _mm256_cmp_ps(plane_x[k], zero, _CMP_GE_OQ);
                    let mask_pos_y = _mm256_cmp_ps(plane_y[k], zero, _CMP_GE_OQ);
                    let mask_pos_z = _mm256_cmp_ps(plane_z[k], zero, _CMP_GE_OQ);

                    let px = _mm256_blendv_ps(min_x, max_x, mask_pos_x);
                    let py = _mm256_blendv_ps(min_y, max_y, mask_pos_y);
                    let pz = _mm256_blendv_ps(min_z, max_z, mask_pos_z);

                    // dot = px*nx + py*ny + pz*nz + d
                    let term_x = _mm256_mul_ps(px, plane_x[k]);
                    let term_y = _mm256_mul_ps(py, plane_y[k]);
                    let term_z = _mm256_mul_ps(pz, plane_z[k]);

                    let dist = _mm256_add_ps(
                        term_x,
                        _mm256_add_ps(term_y, _mm256_add_ps(term_z, plane_d[k])),
                    );

                    // If dist < 0, outside
                    let outside = _mm256_cmp_ps(dist, zero, _CMP_LT_OQ);
                    let outside_int = _mm256_castps_si256(outside);

                    active_mask = _mm256_andnot_si256(outside_int, active_mask);
                }

                let mask_bits = _mm256_movemask_ps(_mm256_castsi256_ps(active_mask));

                // The transpose method used here (unpack + permute2f128) keeps the order correct within lanes?
                // Standard 8x8 transpose logic:
                // r0: A0 A1 A2 A3 A4 A5 A6 A7  (Input was actually A0..A7 in consecutive floats)
                // Wait, input was A0..A7 in consecutive addresses.
                // r0 loaded indices 0..7.
                // Transpose converts rows to columns.
                // Result T0 contains column 0: elem 0 from r0, elem 0 from r1 ...
                // elem 0 of r0 is min_x of AABB 0.
                // elem 0 of r1 is min_x of AABB 1.
                // So T0 is min_x for all 8 AABBs.
                // The order is 0..7.
                // So results[i+k] corresponds to k-th bit.

                for k in 0..8 {
                    results[i + k] = (mask_bits & (1 << k)) != 0;
                }

                i += 8;
            }
        }

        // Tail
        while i < len {
            results[i] = self.intersects_aabb(&aabbs[i]);
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::BoundingSphere;
    use crate::math::{Mat4, Vec3};

    #[test]
    fn test_cull_spheres_prealloc() {
        let m = Mat4::identity();
        let frustum = Frustum::from_matrix(m);
        let spheres = vec![
            BoundingSphere {
                center: Vec3::new(0.0, 0.0, 0.0),
                radius: 0.5,
            },
            BoundingSphere {
                center: Vec3::new(2.0, 0.0, 0.0),
                radius: 0.5,
            },
        ];
        let mut results = vec![false; 2]; // Initialize with false to ensure it writes true
        frustum.cull_spheres_prealloc(&spheres, &mut results);

        assert!(results[0]);
        assert!(!results[1]);
    }

    #[test]
    fn test_aabb_intersection() {
        let m = Mat4::identity();
        let frustum = Frustum::from_matrix(m);

        // 1. AABB Inside [-1, 1]x[-1, 1]x[-1, 1]
        let aabb_inside = AABB::new(Vec3::new(-0.5, -0.5, -0.5), Vec3::new(0.5, 0.5, 0.5));
        assert!(frustum.intersects_aabb(&aabb_inside));

        // 2. AABB Intersecting boundary (x=1)
        // Min at 0.8, Max at 1.2
        let aabb_intersect = AABB::new(Vec3::new(0.8, -0.5, -0.5), Vec3::new(1.2, 0.5, 0.5));
        assert!(frustum.intersects_aabb(&aabb_intersect));

        // 3. AABB Outside
        // Min at 1.1, Max at 1.5
        let aabb_outside = AABB::new(Vec3::new(1.1, -0.5, -0.5), Vec3::new(1.5, 0.5, 0.5));
        assert!(!frustum.intersects_aabb(&aabb_outside));
    }

    #[test]
    fn test_cull_spheres_matches_scalar() {
        // Setup Frustum
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 50.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let proj = Mat4::perspective(1.57, 1.0, 0.1, 100.0);
        let vp = view * proj;
        let frustum = Frustum::from_matrix(vp);

        // Create random spheres
        // We use deterministic loop to avoid randomness dependency in test
        let mut spheres = Vec::with_capacity(101); // 100 random + 1 definitely outside
        for i in 0..100 {
            let x = ((i % 20) as f32) - 10.0;
            let y = ((i / 20) as f32) - 2.0;
            let z = 0.0;
            spheres.push(BoundingSphere {
                center: Vec3::new(x, y, z),
                radius: 0.5,
            });
        }

        // Add some definitely outside
        spheres.push(BoundingSphere {
            center: Vec3::new(1000.0, 0.0, 0.0),
            radius: 1.0,
        });

        // Run SIMD culling
        let simd_results = frustum.cull_spheres(&spheres);

        // Run Scalar manually
        let mut scalar_results = Vec::with_capacity(spheres.len());
        for sphere in &spheres {
            scalar_results.push(frustum.intersects(sphere));
        }

        assert_eq!(simd_results.len(), scalar_results.len());
        for (i, (simd, scalar)) in simd_results.iter().zip(scalar_results.iter()).enumerate() {
            assert_eq!(
                *simd, *scalar,
                "Mismatch at index {i}: simd={simd}, scalar={scalar}"
            );
        }
    }

    #[test]
    fn test_plane_normalize() {
        // Create a plane with a non-normalized normal (length 3)
        // Normal: (3, 0, 0), Distance: 3.0
        // Normalized should be: (1, 0, 0), Distance: 1.0
        let mut plane = Plane {
            normal: Vec3::new(3.0, 0.0, 0.0),
            distance: 3.0,
        };

        plane.normalize();

        assert!((plane.normal.x - 1.0).abs() < 1e-5);
        assert!(plane.normal.y.abs() < 1e-5);
        assert!(plane.normal.z.abs() < 1e-5);
        assert!((plane.distance - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_plane_distance() {
        // Plane with normal (0, 1, 0) and distance -2.0
        // Equation: y - 2 = 0 -> y = 2
        // Normal points UP (positive Y).
        // Distance is d = n . p + D
        // d = y - 2
        let plane = Plane {
            normal: Vec3::new(0.0, 1.0, 0.0),
            distance: -2.0,
        };

        // Point at (0, 3, 0): d = 3 - 2 = 1 (Inside/Positive)
        let p_inside = Vec3::new(0.0, 3.0, 0.0);
        assert!((plane.distance_to_point(p_inside) - 1.0).abs() < 1e-5);

        // Point at (0, 1, 0): d = 1 - 2 = -1 (Outside/Negative)
        let p_outside = Vec3::new(0.0, 1.0, 0.0);
        assert!((plane.distance_to_point(p_outside) - (-1.0)).abs() < 1e-5);

        // Point at (0, 2, 0): d = 2 - 2 = 0 (On plane)
        let p_on = Vec3::new(0.0, 2.0, 0.0);
        assert!(plane.distance_to_point(p_on).abs() < 1e-5);
    }

    #[test]
    fn test_frustum_intersection() {
        // Identity matrix corresponds to canonical view volume [-1, 1]
        let m = Mat4::identity();
        let frustum = Frustum::from_matrix(m);

        // 1. Sphere strictly inside
        let s_inside = BoundingSphere {
            center: Vec3::new(0.0, 0.0, 0.0),
            radius: 0.5,
        };
        assert!(
            frustum.intersects(&s_inside),
            "Sphere inside should be visible"
        );

        // 2. Sphere intersecting boundary
        // Right plane is at x=1, normal pointing left (-1, 0, 0).
        // Center at (1.2, 0, 0), radius 0.5.
        // Closest point on sphere is at x = 1.2 - 0.5 = 0.7 (inside volume)
        // Or checking distance:
        // Plane: -x + 1 = 0
        // Dist = -1.2 + 1 = -0.2
        // -0.2 >= -0.5 is TRUE, so it intersects.
        let s_intersect = BoundingSphere {
            center: Vec3::new(1.2, 0.0, 0.0),
            radius: 0.5,
        };
        assert!(
            frustum.intersects(&s_intersect),
            "Intersecting sphere should be visible"
        );

        // 3. Sphere strictly outside
        // Center at (2.0, 0, 0), radius 0.5.
        // Closest point x = 1.5 (still outside)
        // Dist = -2.0 + 1 = -1.0
        // -1.0 < -0.5 is TRUE (it is strictly outside the negative-radius threshold)
        // wait, `intersects` returns FALSE if `dist < -radius`.
        // -1.0 < -0.5 is true, so it returns false.
        let s_outside = BoundingSphere {
            center: Vec3::new(2.0, 0.0, 0.0),
            radius: 0.5,
        };
        assert!(
            !frustum.intersects(&s_outside),
            "Outside sphere should be culled"
        );
    }

    #[test]
    fn test_cull_aabbs_matches_scalar() {
        // Setup Frustum
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 50.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let proj = Mat4::perspective(1.57, 1.0, 0.1, 100.0);
        let vp = view * proj;
        let frustum = Frustum::from_matrix(vp);

        // Create random AABBs
        // Use a simple pseudo-random generator to ensure deterministic tests
        let mut rng_seed = 12345u32;
        let mut rand_f32 = || {
            rng_seed = rng_seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (rng_seed as f32) / (u32::MAX as f32)
        };

        let mut aabbs = Vec::with_capacity(205); // 200 random + 5 edge cases
        for _ in 0..200 {
            // Random position in [-20, 20]
            let x = rand_f32() * 40.0 - 20.0;
            let y = rand_f32() * 40.0 - 20.0;
            let z = rand_f32() * 40.0 - 20.0;
            // Random size in [0.1, 5.0]
            let sx = rand_f32() * 4.9 + 0.1;
            let sy = rand_f32() * 4.9 + 0.1;
            let sz = rand_f32() * 4.9 + 0.1;

            aabbs.push(AABB::new(
                Vec3::new(x - sx, y - sy, z - sz),
                Vec3::new(x + sx, y + sy, z + sz),
            ));
        }

        // Add edge cases
        // 1. Fully inside small
        aabbs.push(AABB::new(
            Vec3::new(-0.1, -0.1, -0.1),
            Vec3::new(0.1, 0.1, 0.1),
        ));
        // 2. Fully inside large (spanning)
        aabbs.push(AABB::new(
            Vec3::new(-0.8, -0.8, -0.8),
            Vec3::new(0.8, 0.8, 0.8),
        ));
        // 3. Fully outside (Right)
        aabbs.push(AABB::new(
            Vec3::new(2.0, -1.0, -1.0),
            Vec3::new(3.0, 1.0, 1.0),
        ));
        // 4. Fully outside (Left)
        aabbs.push(AABB::new(
            Vec3::new(-3.0, -1.0, -1.0),
            Vec3::new(-2.0, 1.0, 1.0),
        ));
        // 5. Intersecting Right Plane
        aabbs.push(AABB::new(
            Vec3::new(0.5, -0.5, -0.5),
            Vec3::new(1.5, 0.5, 0.5),
        ));

        // Run Prealloc culling (which uses SIMD if available)
        let mut simd_results = vec![false; aabbs.len()];
        frustum.cull_aabbs_prealloc(&aabbs, &mut simd_results);

        // Run Scalar manually
        let mut scalar_results = Vec::with_capacity(aabbs.len());
        for aabb in &aabbs {
            scalar_results.push(frustum.intersects_aabb(aabb));
        }

        assert_eq!(simd_results.len(), scalar_results.len());
        for (i, (simd, scalar)) in simd_results.iter().zip(scalar_results.iter()).enumerate() {
            assert_eq!(
                *simd, *scalar,
                "Mismatch at index {i}: simd={simd}, scalar={scalar}"
            );
        }
    }
}

#[test]
#[should_panic(expected = "assertion `left == right` failed")]
fn test_cull_spheres_prealloc_safety_avx2() {
    let vp = Mat4::identity();
    let frustum = Frustum::from_matrix(vp);
    let spheres = vec![BoundingSphere {
        center: Vec3::ZERO,
        radius: 1.0,
    }];
    let mut results = vec![false; 0]; // Too small
    frustum.cull_spheres_prealloc(&spheres, &mut results);
}

#[test]
#[should_panic(expected = "assertion `left == right` failed")]
fn test_cull_aabbs_prealloc_safety_avx2() {
    let vp = Mat4::identity();
    let frustum = Frustum::from_matrix(vp);
    let aabbs = vec![AABB::new(Vec3::ZERO, Vec3::ZERO)];
    let mut results = vec![false; 0]; // Too small
    frustum.cull_aabbs_prealloc(&aabbs, &mut results);
}
