//! Frustum Culling primitives.

use crate::math::{Mat4, Vec3};
use crate::mesh::{AABB, BoundingSphere};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use std::arch::x86_64::{
    _mm256_add_ps, _mm256_andnot_si256, _mm256_castps_si256, _mm256_castsi256_ps, _mm256_cmp_ps,
    _mm256_loadu_ps, _mm256_movemask_ps, _mm256_mul_ps, _mm256_set1_epi32, _mm256_set1_ps,
    _mm256_setzero_ps, _mm256_unpackhi_ps, _mm256_unpacklo_ps, _CMP_LT_OQ,
};

/// A geometric plane defined by a normal and a distance from the origin.
/// Equation: `normal . point + distance = 0`
#[derive(Debug, Clone, Copy)]
pub struct Plane {
    pub normal: Vec3,
    pub distance: f32,
}

impl Plane {
    /// Normalize the plane equation so that the normal has length 1.
    pub fn normalize(&mut self) {
        let len = self.normal.length();
        if len > 0.0001 {
            let inv_len = 1.0 / len;
            self.normal = self.normal * inv_len;
            self.distance *= inv_len;
        }
    }

    /// Signed distance from a point to the plane.
    /// Positive if on the side of the normal.
    #[must_use]
    pub fn distance_to_point(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.distance
    }
}

/// A View Frustum defined by 6 planes.
/// Used for object-level culling.
pub struct Frustum {
    pub planes: [Plane; 6],
}

impl Frustum {
    /// Extract frustum planes from View-Projection matrix.
    /// Assumes Row-Major matrix where `v_clip = v_world * M`.
    ///
    /// The planes are extracted such that the normal points **inside** the frustum.
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
    pub fn intersects_aabb(&self, aabb: &AABB) -> bool {
        for plane in &self.planes {
            // Find the p-vertex (the vertex furthest along the normal direction)
            // If this vertex is behind the plane (negative distance), the whole box is outside.
            let px = if plane.normal.x >= 0.0 { aabb.max.x } else { aabb.min.x };
            let py = if plane.normal.y >= 0.0 { aabb.max.y } else { aabb.min.y };
            let pz = if plane.normal.z >= 0.0 { aabb.max.z } else { aabb.min.z };

            let dist = plane.normal.x * px + plane.normal.y * py + plane.normal.z * pz + plane.distance;

            if dist < 0.0 {
                return false;
            }
        }
        true
    }

    /// Check multiple spheres against the frustum using SIMD optimizations.
    /// Returns a `Vec<bool>` where `true` means the sphere is visible.
    pub fn cull_spheres(&self, spheres: &[BoundingSphere]) -> Vec<bool> {
        let mut results = vec![true; spheres.len()];
        self.cull_spheres_prealloc(spheres, &mut results);
        results
    }

    /// Check multiple spheres against the frustum using SIMD optimizations.
    /// Writes results into the provided slice.
    ///
    /// The `results` slice must be the same length as `spheres`.
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
    pub fn cull_aabbs_prealloc(&self, aabbs: &[AABB], results: &mut [bool]) {
        assert_eq!(aabbs.len(), results.len());

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
                let ptr = spheres.as_ptr().add(i) as *const f32;

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{Mat4, Vec3};
    use crate::mesh::BoundingSphere;

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

        assert_eq!(results[0], true);
        assert_eq!(results[1], false);
    }

    #[test]
    fn test_aabb_intersection() {
        let m = Mat4::identity();
        let frustum = Frustum::from_matrix(m);

        // 1. AABB Inside [-1, 1]x[-1, 1]x[-1, 1]
        let aabb_inside = AABB::new(
            Vec3::new(-0.5, -0.5, -0.5),
            Vec3::new(0.5, 0.5, 0.5),
        );
        assert!(frustum.intersects_aabb(&aabb_inside));

        // 2. AABB Intersecting boundary (x=1)
        // Min at 0.8, Max at 1.2
        let aabb_intersect = AABB::new(
            Vec3::new(0.8, -0.5, -0.5),
            Vec3::new(1.2, 0.5, 0.5),
        );
        assert!(frustum.intersects_aabb(&aabb_intersect));

        // 3. AABB Outside
        // Min at 1.1, Max at 1.5
        let aabb_outside = AABB::new(
            Vec3::new(1.1, -0.5, -0.5),
            Vec3::new(1.5, 0.5, 0.5),
        );
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
        let mut spheres = Vec::new();
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
        let mut scalar_results = Vec::new();
        for sphere in &spheres {
            scalar_results.push(frustum.intersects(sphere));
        }

        assert_eq!(simd_results.len(), scalar_results.len());
        for (i, (simd, scalar)) in simd_results.iter().zip(scalar_results.iter()).enumerate() {
            assert_eq!(
                *simd, *scalar,
                "Mismatch at index {}: simd={}, scalar={}",
                i, simd, scalar
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
}
