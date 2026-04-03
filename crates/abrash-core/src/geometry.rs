//! Geometric primitives and utilities.

use crate::math::{Mat4, Vec3};

/// A 3D ray represented by `origin + direction * t`.
///
/// For best numerical behavior, `direction` should be normalized.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    /// Ray start point.
    pub origin: Vec3,
    /// Ray direction.
    pub direction: Vec3,
}

impl Ray {
    /// Creates a new ray.
    #[must_use]
    #[inline]
    pub const fn new(origin: Vec3, direction: Vec3) -> Self {
        Self { origin, direction }
    }

    /// Evaluate point on ray at parameter `t`.
    #[must_use]
    #[inline]
    pub fn at(self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }

    /// Ray/sphere intersection returning nearest non-negative `t`.
    #[must_use]
    pub fn intersects_sphere(self, center: Vec3, radius: f32) -> Option<f32> {
        let oc = self.origin - center;
        let a = self.direction.dot(self.direction);
        let b = 2.0 * oc.dot(self.direction);
        let c = oc.dot(oc) - radius * radius;
        let disc = b * b - 4.0 * a * c;
        if disc < 0.0 {
            return None;
        }
        let sqrt_disc = disc.sqrt();
        let inv_2a = 0.5 / a;
        let t0 = (-b - sqrt_disc) * inv_2a;
        if t0 >= 0.0 {
            return Some(t0);
        }
        let t1 = (-b + sqrt_disc) * inv_2a;
        (t1 >= 0.0).then_some(t1)
    }

    /// Ray/triangle intersection (Möller–Trumbore).
    ///
    /// Returns hit distance `t` if the intersection lies in front of the ray origin.
    #[must_use]
    pub fn intersects_triangle(self, v0: Vec3, v1: Vec3, v2: Vec3) -> Option<f32> {
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let pvec = self.direction.cross(edge2);
        let det = edge1.dot(pvec);
        if det.abs() < 1.0e-8 {
            return None;
        }
        let inv_det = 1.0 / det;
        let tvec = self.origin - v0;
        let u = tvec.dot(pvec) * inv_det;
        if !(0.0..=1.0).contains(&u) {
            return None;
        }
        let qvec = tvec.cross(edge1);
        let v = self.direction.dot(qvec) * inv_det;
        if v < 0.0 || u + v > 1.0 {
            return None;
        }
        let t = edge2.dot(qvec) * inv_det;
        (t >= 0.0).then_some(t)
    }
}

/// A Bounding Sphere for object-level culling.
///
/// used for coarse intersection tests before checking individual triangles.
///
/// # Examples
///
/// ```
/// use abrash_core::geometry::BoundingSphere;
/// use abrash_core::math::Vec3;
///
/// let sphere = BoundingSphere {
///     center: Vec3::new(0.0, 0.0, 0.0),
///     radius: 1.0,
/// };
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(missing_docs)]
pub struct BoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

impl BoundingSphere {
    /// Compute the smallest bounding sphere enclosing `points` using Ritter's algorithm.
    ///
    /// Ritter's algorithm is an O(n) approximation that typically produces a sphere
    /// within 5% of optimal.  Returns `None` if `points` is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::geometry::BoundingSphere;
    /// use abrash_core::math::Vec3;
    ///
    /// let points = [Vec3::new(1.0, 0.0, 0.0), Vec3::new(-1.0, 0.0, 0.0)];
    /// let sphere = BoundingSphere::from_points(&points).unwrap();
    /// assert!((sphere.center.x).abs() < 1e-5);
    /// assert!((sphere.radius - 1.0).abs() < 1e-5);
    /// ```
    #[must_use]
    pub fn from_points(points: &[Vec3]) -> Option<Self> {
        if points.is_empty() {
            return None;
        }

        // Pass 1: find the most-separated pair (approximate).
        // Pick the point with min/max x, y, z in each dimension, then take the
        // most-separated pair as the initial diameter.
        let mut min_x = points[0];
        let mut max_x = points[0];
        let mut min_y = points[0];
        let mut max_y = points[0];
        let mut min_z = points[0];
        let mut max_z = points[0];

        for &p in points {
            if p.x < min_x.x {
                min_x = p;
            }
            if p.x > max_x.x {
                max_x = p;
            }
            if p.y < min_y.y {
                min_y = p;
            }
            if p.y > max_y.y {
                max_y = p;
            }
            if p.z < min_z.z {
                min_z = p;
            }
            if p.z > max_z.z {
                max_z = p;
            }
        }

        let dx = (max_x - min_x).length_sq();
        let dy = (max_y - min_y).length_sq();
        let dz = (max_z - min_z).length_sq();

        let (p, q) = if dx >= dy && dx >= dz {
            (min_x, max_x)
        } else if dy >= dz {
            (min_y, max_y)
        } else {
            (min_z, max_z)
        };

        let mut center = (p + q) * 0.5;
        let mut radius = (q - p).length() * 0.5;

        // Pass 2: expand sphere to include any points still outside.
        for &pt in points {
            let d = (pt - center).length();
            if d > radius {
                let excess = d - radius;
                radius += excess * 0.5;
                let dir = (pt - center) * (1.0 / d);
                center = center + dir * (excess * 0.5);
            }
        }

        Some(Self { center, radius })
    }

    /// Transform this bounding sphere by a matrix.
    ///
    /// Applies the transformation to the center and scales the radius by the maximum scale factor
    /// extracted from the matrix basis vectors.
    pub fn transform(&mut self, transform: &Mat4) {
        let (new_center, _) = transform.transform_point(self.center);
        self.center = new_center;

        // Extract scale from basis vectors (columns 0, 1, 2)
        // We take the max scale to ensure the sphere fully encloses the transformed object
        let sx = transform.col(0);
        let sy = transform.col(1);
        let sz = transform.col(2);

        // Length of basis vectors = scale factor
        let scale_x_sq = sx.x * sx.x + sx.y * sx.y + sx.z * sx.z;
        let scale_y_sq = sy.x * sy.x + sy.y * sy.y + sy.z * sy.z;
        let scale_z_sq = sz.x * sz.x + sz.y * sz.y + sz.z * sz.z;

        let max_scale_sq = scale_x_sq.max(scale_y_sq).max(scale_z_sq);
        // optimization: avoid sqrt if scale is 1.0
        if (max_scale_sq - 1.0).abs() > 0.0001 {
            self.radius *= max_scale_sq.sqrt();
        }
    }
}

/// Axis-Aligned Bounding Box (AABB) for object-level culling.
///
/// Represents a box aligned with the world axes that fully encloses an object.
/// AABBs are faster to construct and test than Oriented Bounding Boxes (OBB),
/// but may fit less tightly for rotated objects.
///
/// # Examples
///
/// ```
/// use abrash_core::geometry::AABB;
/// use abrash_core::math::Vec3;
///
/// let min = Vec3::new(-1.0, -1.0, -1.0);
/// let max = Vec3::new(1.0, 1.0, 1.0);
/// let aabb = AABB::new(min, max);
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(missing_docs)]
pub struct AABB {
    pub min: Vec3,
    #[doc(hidden)]
    pub pad0: f32, // Padding to align max to 16 bytes offset
    pub max: Vec3,
    #[doc(hidden)]
    pub pad1: f32, // Padding to make total size 32 bytes
}

impl AABB {
    /// Create a new AABB from min and max points.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::geometry::AABB;
    /// use abrash_core::math::Vec3;
    ///
    /// let aabb = AABB::new(Vec3::ZERO, Vec3::ONE);
    /// ```
    #[must_use]
    pub const fn new(min: Vec3, max: Vec3) -> Self {
        Self {
            min,
            pad0: 0.0,
            max,
            pad1: 0.0,
        }
    }

    /// Create an AABB from a center point and half-extents.
    #[must_use]
    pub const fn from_center_extents(center: Vec3, extents: Vec3) -> Self {
        Self::new(
            Vec3::new(
                center.x - extents.x,
                center.y - extents.y,
                center.z - extents.z,
            ),
            Vec3::new(
                center.x + extents.x,
                center.y + extents.y,
                center.z + extents.z,
            ),
        )
    }

    /// Calculate AABB from a list of points.
    ///
    /// Returns a default zero-sized AABB if the input list is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::geometry::AABB;
    /// use abrash_core::math::Vec3;
    ///
    /// let points = [
    ///     Vec3::new(1.0, 0.0, 0.0),
    ///     Vec3::new(-1.0, 2.0, 0.0),
    /// ];
    /// let aabb = AABB::from_points(&points);
    ///
    /// assert_eq!(aabb.min.x, -1.0);
    /// assert_eq!(aabb.max.y, 2.0);
    /// ```
    /// Calculate AABB from a list of points.
    ///
    /// Returns a default zero-sized AABB if the input list is empty.
    ///
    /// Optimization: Uses `Vec3::min` and `Vec3::max` to leverage underlying fast
    /// floating point operations (`minss`/`maxss`) instead of branchy component-wise checks.
    /// This provides a small but measurable speedup for bounding box calculations on large meshes.
    #[must_use]
    pub fn from_points(points: &[Vec3]) -> Self {
        if points.is_empty() {
            return Self {
                min: Vec3::default(),
                pad0: 0.0,
                max: Vec3::default(),
                pad1: 0.0,
            };
        }

        let mut min = points[0];
        let mut max = points[0];

        for &p in points.iter().skip(1) {
            min = min.min(p);
            max = max.max(p);
        }

        Self {
            min,
            pad0: 0.0,
            max,
            pad1: 0.0,
        }
    }

    /// Get the center of the AABB.
    #[must_use]
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Get the extents (half-size) of the AABB.
    #[must_use]
    pub fn extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    /// Returns `true` if the point lies inside or on the boundary of the AABB.
    #[must_use]
    #[inline]
    pub fn contains_point(&self, point: Vec3) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }

    /// Returns `true` if the other AABB is fully enclosed by this one.
    #[must_use]
    #[inline]
    pub fn contains_aabb(&self, other: &Self) -> bool {
        self.contains_point(other.min) && self.contains_point(other.max)
    }

    /// Returns `true` if the two AABBs overlap or touch.
    #[must_use]
    #[inline]
    pub fn intersects(&self, other: &Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    /// Returns the smallest AABB that encloses both inputs.
    #[must_use]
    #[inline]
    pub const fn union(&self, other: &Self) -> Self {
        Self::new(self.min.min(other.min), self.max.max(other.max))
    }

    /// Returns the smallest AABB that encloses this box and the given point.
    #[must_use]
    #[inline]
    pub const fn include_point(&self, point: Vec3) -> Self {
        Self::new(self.min.min(point), self.max.max(point))
    }

    /// Surface area of the box.
    #[must_use]
    #[inline]
    pub fn surface_area(&self) -> f32 {
        let size = self.max - self.min;
        2.0 * (size.x * size.y + size.x * size.z + size.y * size.z)
    }

    /// Closest point on (or inside) this AABB to `point`.
    #[must_use]
    #[inline]
    pub const fn closest_point(&self, point: Vec3) -> Vec3 {
        point.clamp(self.min, self.max)
    }

    /// Squared distance from `point` to this AABB.
    ///
    /// Returns 0 when the point lies inside the box.
    #[must_use]
    #[inline]
    pub fn distance_sq_to_point(&self, point: Vec3) -> f32 {
        let clamped = self.closest_point(point);
        (point - clamped).length_sq()
    }

    /// Returns `true` if the sphere intersects this AABB.
    #[must_use]
    #[inline]
    pub fn intersects_sphere(&self, center: Vec3, radius: f32) -> bool {
        self.distance_sq_to_point(center) <= radius * radius
    }

    /// Ray/AABB intersection that accepts a full [`Ray`].
    ///
    /// Returns `(t_min, t_max)` on hit, where hit points are `ray.at(t)`.
    #[must_use]
    #[inline]
    pub fn intersects_ray_struct(&self, ray: Ray) -> Option<(f32, f32)> {
        self.intersects_ray(ray.origin, ray.direction)
    }

    /// Ray/AABB intersection using the branch-light slab algorithm.
    ///
    /// Returns `(t_min, t_max)` on hit, where ray points are `origin + dir * t`.
    /// Caller can filter hits behind origin by checking `t_max >= 0.0`.
    #[must_use]
    pub fn intersects_ray(&self, origin: Vec3, dir: Vec3) -> Option<(f32, f32)> {
        #[inline]
        fn update_axis(
            min: f32,
            max: f32,
            origin: f32,
            dir: f32,
            t_min: &mut f32,
            t_max: &mut f32,
        ) -> bool {
            if dir.abs() <= f32::EPSILON {
                return origin >= min && origin <= max;
            }
            let inv = 1.0 / dir;
            let mut t0 = (min - origin) * inv;
            let mut t1 = (max - origin) * inv;
            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
            }
            *t_min = (*t_min).max(t0);
            *t_max = (*t_max).min(t1);
            *t_min <= *t_max
        }

        let mut t_min = f32::NEG_INFINITY;
        let mut t_max = f32::INFINITY;

        if !update_axis(
            self.min.x, self.max.x, origin.x, dir.x, &mut t_min, &mut t_max,
        ) {
            return None;
        }
        if !update_axis(
            self.min.y, self.max.y, origin.y, dir.y, &mut t_min, &mut t_max,
        ) {
            return None;
        }
        if !update_axis(
            self.min.z, self.max.z, origin.z, dir.z, &mut t_min, &mut t_max,
        ) {
            return None;
        }
        Some((t_min, t_max))
    }

    /// Transform this AABB by a matrix.
    ///
    /// Calculates the new Axis-Aligned Bounding Box in the new coordinate space.
    /// Note: This results in a loose-fitting AABB (AABB of the OBB).
    ///
    /// Optimized using Arvo's algorithm (Transforming Center & Extents) to avoid
    /// transforming all 8 corners explicitly.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::geometry::AABB;
    /// use abrash_core::math::{Mat4, Vec3};
    ///
    /// let aabb = AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
    /// let transform = Mat4::translation(10.0, 0.0, 0.0);
    /// let transformed_aabb = aabb.transform(&transform);
    ///
    /// assert_eq!(transformed_aabb.min.x, 10.0);
    /// ```
    #[must_use]
    pub fn transform(&self, transform: &Mat4) -> Self {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        if is_x86_feature_detected!("avx2") {
            // SAFETY: We checked feature detection.
            unsafe {
                return self.transform_avx2(transform);
            }
        }

        #[cfg(all(target_arch = "x86_64", feature = "simd"))]
        if is_x86_feature_detected!("sse2") {
            // SAFETY: We checked feature detection.
            unsafe {
                return self.transform_simd(transform);
            }
        }

        let center = self.center();
        let extents = self.extents();
        let (world_center, _) = transform.transform_point(center);
        let m = &transform.m;

        let world_extents = Vec3::new(
            m[0][0].abs() * extents.x + m[1][0].abs() * extents.y + m[2][0].abs() * extents.z,
            m[0][1].abs() * extents.x + m[1][1].abs() * extents.y + m[2][1].abs() * extents.z,
            m[0][2].abs() * extents.x + m[1][2].abs() * extents.y + m[2][2].abs() * extents.z,
        );

        Self::new(world_center - world_extents, world_center + world_extents)
    }

    /// AVX2-optimized implementation of Arvo's algorithm.
    ///
    /// Using AVX2 256-bit registers allows processing min and max calculations
    /// completely in parallel within a single register.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "avx2")]
    #[must_use]
    pub unsafe fn transform_avx2(&self, transform: &Mat4) -> Self {
        #[cfg(target_arch = "x86")]
        use std::arch::x86::{
            _mm_load_ps, _mm_set_ps, _mm256_add_ps, _mm256_max_ps, _mm256_min_ps, _mm256_mul_ps,
            _mm256_permute_ps, _mm256_permute2f128_ps, _mm256_set_m128, _mm256_storeu_ps,
        };
        #[cfg(target_arch = "x86_64")]
        use std::arch::x86_64::{
            _mm_load_ps, _mm_set_ps, _mm256_add_ps, _mm256_max_ps, _mm256_min_ps, _mm256_mul_ps,
            _mm256_permute_ps, _mm256_permute2f128_ps, _mm256_set_m128, _mm256_storeu_ps,
        };

        unsafe {
            let m = &transform.m;

            // Load transformation matrix rows
            let r_128 = _mm_load_ps(m[0].as_ptr());
            let u_128 = _mm_load_ps(m[1].as_ptr());
            let b_128 = _mm_load_ps(m[2].as_ptr());
            let t_128 = _mm_load_ps(m[3].as_ptr());

            // Duplicate 128-bit lanes to 256-bit registers: [R | R], [U | U], [B | B], [T | T]
            let r = _mm256_set_m128(r_128, r_128);
            let u = _mm256_set_m128(u_128, u_128);
            let b = _mm256_set_m128(b_128, b_128);
            let t = _mm256_set_m128(t_128, t_128);

            // Load min/max. Vec3 is x, y, z. Pad w with 0.0.
            let min_v = _mm_set_ps(0.0, self.min.z, self.min.y, self.min.x);
            let max_v = _mm_set_ps(0.0, self.max.z, self.max.y, self.max.x);

            // Combine into a single 256-bit register: [max | min]
            let bounds = _mm256_set_m128(max_v, min_v);

            // Broadcast x, y, z to all lanes for both min and max
            // _MM_SHUFFLE(0, 0, 0, 0) = 0x00
            let bounds_x = _mm256_permute_ps(bounds, 0x00); // [max.x, max.x, max.x, max.x | min.x, min.x, min.x, min.x]

            // _MM_SHUFFLE(1, 1, 1, 1) = 0x55
            let bounds_y = _mm256_permute_ps(bounds, 0x55); // [max.y, max.y, max.y, max.y | min.y, min.y, min.y, min.y]

            // _MM_SHUFFLE(2, 2, 2, 2) = 0xAA
            let bounds_z = _mm256_permute_ps(bounds, 0xAA); // [max.z, max.z, max.z, max.z | min.z, min.z, min.z, min.z]

            // Calculate multiplied terms
            let term_x = _mm256_mul_ps(r, bounds_x); // [r * max.x | r * min.x]
            let term_y = _mm256_mul_ps(u, bounds_y); // [u * max.y | u * min.y]
            let term_z = _mm256_mul_ps(b, bounds_z); // [b * max.z | b * min.z]

            // Now, we need to extract the min and max for each component.
            // Arvo's algorithm: new_min = trans + sum(min(r*min.x, r*max.x), min(u*min.y, u*max.y), min(b*min.z, b*max.z))
            // The `term_x` register holds both `r * max.x` (high 128) and `r * min.x` (low 128).
            // We want to perform min/max across the 128-bit lanes.

            // We can do this by swapping the 128-bit lanes and then applying min/max.
            // Swap high and low 128-bit lanes:
            // _mm256_permute2f128_ps(a, a, 1) -> swaps the 128-bit lanes of a.

            let term_x_swapped = _mm256_permute2f128_ps(term_x, term_x, 1);
            let min_term_x = _mm256_min_ps(term_x, term_x_swapped);
            let max_term_x = _mm256_max_ps(term_x, term_x_swapped);

            let term_y_swapped = _mm256_permute2f128_ps(term_y, term_y, 1);
            let min_term_y = _mm256_min_ps(term_y, term_y_swapped);
            let max_term_y = _mm256_max_ps(term_y, term_y_swapped);

            let term_z_swapped = _mm256_permute2f128_ps(term_z, term_z, 1);
            let min_term_z = _mm256_min_ps(term_z, term_z_swapped);
            let max_term_z = _mm256_max_ps(term_z, term_z_swapped);

            // Both high and low 128-bit lanes of `min_term_x` now hold the min value.
            // But we only need to sum them up. We can just use the low 128-bit lane for min and high for max?
            // Actually, `min_term_x` has `min(a, b)` in both low and high lanes.
            // We want to add them together with translation `t`.
            // sum_min = min_x + min_y + min_z + t
            let sum_min = _mm256_add_ps(
                min_term_x,
                _mm256_add_ps(min_term_y, _mm256_add_ps(min_term_z, t)),
            );

            let sum_max = _mm256_add_ps(
                max_term_x,
                _mm256_add_ps(max_term_y, _mm256_add_ps(max_term_z, t)),
            );

            // Now, sum_min has the new min in both low and high lanes.
            // sum_max has the new max in both low and high lanes.

            let mut min_arr = [0.0; 8];
            let mut max_arr = [0.0; 8];

            _mm256_storeu_ps(min_arr.as_mut_ptr(), sum_min);
            _mm256_storeu_ps(max_arr.as_mut_ptr(), sum_max);

            Self::new(
                Vec3::new(min_arr[0], min_arr[1], min_arr[2]),
                Vec3::new(max_arr[0], max_arr[1], max_arr[2]),
            )
        }
    }

    /// SIMD-optimized implementation of Arvo's algorithm using SSE.
    ///
    /// This vectorized version processes x, y, and z components of the result simultaneously,
    /// avoiding the overhead of scalar component-wise operations.
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    #[target_feature(enable = "sse2")]
    unsafe fn transform_simd(&self, transform: &Mat4) -> Self {
        use std::arch::x86_64::{
            _mm_add_ps, _mm_load_ps, _mm_max_ps, _mm_min_ps, _mm_mul_ps, _mm_set_ps,
            _mm_shuffle_ps, _mm_storeu_ps,
        };

        // SAFETY:
        // 1. SSE2 intrinsics are safe if feature is detected (checked by caller or cfg).
        // 2. `transform.m` is `Mat4` which is `#[repr(align(16))]`, ensuring 16-byte alignment
        //    required by `_mm_load_ps`.
        unsafe {
            let m = &transform.m;
            // Load rows. Mat4 is 16-byte aligned.
            let r = _mm_load_ps(m[0].as_ptr());
            let u = _mm_load_ps(m[1].as_ptr());
            let b = _mm_load_ps(m[2].as_ptr());
            let t = _mm_load_ps(m[3].as_ptr());

            let min = &self.min;
            let max = &self.max;

            // Load min/max safely. Vec3 is x, y, z.
            // We set w to 0.0 to avoid affecting translation (which has w=1.0).
            let min_v = _mm_set_ps(0.0, min.z, min.y, min.x);
            let max_v = _mm_set_ps(0.0, max.z, max.y, max.x);

            // Broadcast components
            // x
            let min_x = _mm_shuffle_ps(min_v, min_v, 0x00); // 0,0,0,0
            let max_x = _mm_shuffle_ps(max_v, max_v, 0x00);

            // y
            let min_y = _mm_shuffle_ps(min_v, min_v, 0x55); // 1,1,1,1
            let max_y = _mm_shuffle_ps(max_v, max_v, 0x55);

            // z
            let min_z = _mm_shuffle_ps(min_v, min_v, 0xAA); // 2,2,2,2
            let max_z = _mm_shuffle_ps(max_v, max_v, 0xAA);

            // X axis terms (Row 0)
            let xa = _mm_mul_ps(r, min_x);
            let xb = _mm_mul_ps(r, max_x);
            let min_term_x = _mm_min_ps(xa, xb);
            let max_term_x = _mm_max_ps(xa, xb);

            // Y axis terms (Row 1)
            let ya = _mm_mul_ps(u, min_y);
            let yb = _mm_mul_ps(u, max_y);
            let min_term_y = _mm_min_ps(ya, yb);
            let max_term_y = _mm_max_ps(ya, yb);

            // Z axis terms (Row 2)
            let za = _mm_mul_ps(b, min_z);
            let zb = _mm_mul_ps(b, max_z);
            let min_term_z = _mm_min_ps(za, zb);
            let max_term_z = _mm_max_ps(za, zb);

            // Sum everything
            let sum_min = _mm_add_ps(
                min_term_x,
                _mm_add_ps(min_term_y, _mm_add_ps(min_term_z, t)),
            );
            let sum_max = _mm_add_ps(
                max_term_x,
                _mm_add_ps(max_term_y, _mm_add_ps(max_term_z, t)),
            );

            // Store back to Vec3
            // We can extract via storeu
            let mut min_arr = [0.0; 4];
            let mut max_arr = [0.0; 4];
            _mm_storeu_ps(min_arr.as_mut_ptr(), sum_min);
            _mm_storeu_ps(max_arr.as_mut_ptr(), sum_max);

            Self::new(
                Vec3::new(min_arr[0], min_arr[1], min_arr[2]),
                Vec3::new(max_arr[0], max_arr[1], max_arr[2]),
            )
        }
    }
}

// ── Triangle ─────────────────────────────────────────────────────────────────

/// A 3D triangle defined by three vertices `a`, `b`, `c`.
///
/// Provides common geometric queries needed for mesh processing, collision
/// detection, and rasterization support.
///
/// # Examples
///
/// ```
/// use abrash_core::geometry::Triangle;
/// use abrash_core::math::Vec3;
///
/// let tri = Triangle::new(
///     Vec3::new(0.0, 0.0, 0.0),
///     Vec3::new(1.0, 0.0, 0.0),
///     Vec3::new(0.0, 1.0, 0.0),
/// );
/// assert!((tri.area() - 0.5).abs() < 1e-5);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Triangle {
    /// First vertex.
    pub a: Vec3,
    /// Second vertex.
    pub b: Vec3,
    /// Third vertex.
    pub c: Vec3,
}

impl Triangle {
    /// Creates a new triangle from three vertices.
    #[must_use]
    #[inline]
    pub const fn new(a: Vec3, b: Vec3, c: Vec3) -> Self {
        Self { a, b, c }
    }

    /// Unnormalized face normal (`(b-a) × (c-a)`).
    ///
    /// Length equals twice the triangle's area.  For a unit normal use
    /// [`Triangle::normal`].
    #[must_use]
    #[inline]
    pub fn normal_unnormalized(self) -> Vec3 {
        (self.b - self.a).cross(self.c - self.a)
    }

    /// Unit face normal.
    ///
    /// Returns `Vec3::ZERO` for degenerate (zero-area) triangles.
    #[must_use]
    #[inline]
    pub fn normal(self) -> Vec3 {
        self.normal_unnormalized().normalize_or_zero()
    }

    /// Area of the triangle.
    #[must_use]
    #[inline]
    pub fn area(self) -> f32 {
        self.normal_unnormalized().length() * 0.5
    }

    /// Centroid (arithmetic mean of the three vertices).
    #[must_use]
    #[inline]
    pub fn centroid(self) -> Vec3 {
        (self.a + self.b + self.c) * (1.0 / 3.0)
    }

    /// Barycentric coordinates of `point` relative to this triangle.
    ///
    /// Returns `(u, v, w)` such that `point ≈ u*a + v*b + w*c` and `u+v+w = 1`.
    /// If the triangle is degenerate, all three coordinates are `1/3`.
    ///
    /// The point is inside the triangle when all coordinates are in `[0, 1]`.
    #[must_use]
    pub fn barycentric(self, point: Vec3) -> (f32, f32, f32) {
        let v0 = self.b - self.a;
        let v1 = self.c - self.a;
        let v2 = point - self.a;

        let d00 = v0.dot(v0);
        let d01 = v0.dot(v1);
        let d11 = v1.dot(v1);
        let d20 = v2.dot(v0);
        let d21 = v2.dot(v1);

        let denom = d00 * d11 - d01 * d01;
        if denom.abs() < 1e-10 {
            return (1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0);
        }
        let inv = 1.0 / denom;
        let v = (d11 * d20 - d01 * d21) * inv;
        let w = (d00 * d21 - d01 * d20) * inv;
        let u = 1.0 - v - w;
        (u, v, w)
    }

    /// Returns `true` if `point` (projected onto the triangle's plane) lies inside.
    #[must_use]
    #[inline]
    pub fn contains_projected_point(self, point: Vec3) -> bool {
        let (u, v, w) = self.barycentric(point);
        u >= 0.0 && v >= 0.0 && w >= 0.0
    }

    /// Closest point on the triangle surface to `point`.
    ///
    /// Uses the Ericson / Real-Time Collision Detection algorithm.
    #[must_use]
    pub fn closest_point(self, point: Vec3) -> Vec3 {
        let ab = self.b - self.a;
        let ac = self.c - self.a;
        let ap = point - self.a;

        let d1 = ab.dot(ap);
        let d2 = ac.dot(ap);
        if d1 <= 0.0 && d2 <= 0.0 {
            return self.a;
        }

        let bp = point - self.b;
        let d3 = ab.dot(bp);
        let d4 = ac.dot(bp);
        if d3 >= 0.0 && d4 <= d3 {
            return self.b;
        }

        let vc = d1 * d4 - d3 * d2;
        if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
            let v = d1 / (d1 - d3);
            return self.a + ab * v;
        }

        let cp = point - self.c;
        let d5 = ab.dot(cp);
        let d6 = ac.dot(cp);
        if d6 >= 0.0 && d5 <= d6 {
            return self.c;
        }

        let vb = d5 * d2 - d1 * d6;
        if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
            let w = d2 / (d2 - d6);
            return self.a + ac * w;
        }

        let va = d3 * d6 - d5 * d4;
        if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
            let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
            return self.b + (self.c - self.b) * w;
        }

        let denom = 1.0 / (va + vb + vc);
        let v = vb * denom;
        let w = vc * denom;
        self.a + ab * v + ac * w
    }

    /// Squared distance from `point` to the triangle surface.
    #[must_use]
    #[inline]
    pub fn distance_sq_to_point(self, point: Vec3) -> f32 {
        (point - self.closest_point(point)).length_sq()
    }

    /// Circumcenter of the triangle (center of the circumscribed circle).
    ///
    /// Returns `None` for degenerate triangles.
    #[must_use]
    pub fn circumcenter(self) -> Option<Vec3> {
        let ac = self.c - self.a;
        let ab = self.b - self.a;
        let ab_cross_ac = ab.cross(ac);

        let len_sq = ab_cross_ac.length_sq();
        if len_sq < 1e-10 {
            return None;
        }

        let to_circumcenter = (ab_cross_ac.cross(ab) * ac.length_sq()
            + ac.cross(ab_cross_ac) * ab.length_sq())
            * (1.0 / (2.0 * len_sq));

        Some(self.a + to_circumcenter)
    }

    /// Smallest AABB enclosing this triangle.
    #[must_use]
    #[inline]
    pub fn to_aabb(self) -> AABB {
        AABB::new(
            self.a.min(self.b).min(self.c),
            self.a.max(self.b).max(self.c),
        )
    }
}

// ── OBB ───────────────────────────────────────────────────────────────────────

/// Oriented Bounding Box (OBB) for tight-fit culling and collision detection.
///
/// Unlike an [`AABB`], an OBB stores three local axes and can be arbitrarily
/// rotated to fit an object. The `axes` field stores the three unit-length
/// local X, Y, Z axes expressed in world space (row-vector convention).
///
/// # Conventions
///
/// - `axes[0]` is the local X axis (right).
/// - `axes[1]` is the local Y axis (up).
/// - `axes[2]` is the local Z axis (back).
/// - Axes must be orthonormal; behaviour is undefined otherwise.
///
/// # Examples
///
/// ```
/// use abrash_core::geometry::OBB;
/// use abrash_core::math::Vec3;
///
/// let obb = OBB::from_center_extents(Vec3::ZERO, Vec3::ONE);
/// assert!(obb.contains_point(Vec3::new(0.5, 0.5, 0.5)));
/// assert!(!obb.contains_point(Vec3::new(2.0, 0.0, 0.0)));
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OBB {
    /// Center of the box in world space.
    pub center: Vec3,
    /// Half-extents along each local axis.
    pub half_extents: Vec3,
    /// Local axes in world space — `[right, up, back]`. Must be orthonormal.
    pub axes: [Vec3; 3],
}

impl OBB {
    /// Creates an axis-aligned OBB (identical orientation to an AABB).
    #[must_use]
    #[inline]
    pub fn from_center_extents(center: Vec3, half_extents: Vec3) -> Self {
        Self {
            center,
            half_extents,
            axes: [Vec3::X, Vec3::Y, Vec3::Z],
        }
    }

    /// Creates an OBB that covers an [`AABB`] with identity orientation.
    #[must_use]
    #[inline]
    pub fn from_aabb(aabb: &AABB) -> Self {
        Self::from_center_extents(aabb.center(), aabb.extents())
    }

    /// Creates an OBB by transforming an [`AABB`].
    ///
    /// Rotation and scale are extracted from `transform`; the resulting OBB
    /// fits the rotated box exactly, with scale baked into `half_extents`.
    #[must_use]
    pub fn from_aabb_transform(aabb: &AABB, transform: &Mat4) -> Self {
        let m = &transform.m;
        let row0 = Vec3::new(m[0][0], m[0][1], m[0][2]);
        let row1 = Vec3::new(m[1][0], m[1][1], m[1][2]);
        let row2 = Vec3::new(m[2][0], m[2][1], m[2][2]);

        let sx = row0.length();
        let sy = row1.length();
        let sz = row2.length();

        let ax = if sx > 1e-8 {
            row0 * (1.0 / sx)
        } else {
            Vec3::X
        };
        let ay = if sy > 1e-8 {
            row1 * (1.0 / sy)
        } else {
            Vec3::Y
        };
        let az = if sz > 1e-8 {
            row2 * (1.0 / sz)
        } else {
            Vec3::Z
        };

        let extents = aabb.extents();
        let (world_center, _) = transform.transform_point(aabb.center());

        Self {
            center: world_center,
            half_extents: Vec3::new(extents.x * sx, extents.y * sy, extents.z * sz),
            axes: [ax, ay, az],
        }
    }

    /// Project `point` into OBB-local coordinates (signed distances along each axis).
    #[inline]
    fn local_coords(&self, point: Vec3) -> Vec3 {
        let d = point - self.center;
        Vec3::new(
            d.dot(self.axes[0]),
            d.dot(self.axes[1]),
            d.dot(self.axes[2]),
        )
    }

    /// Returns `true` if `point` lies inside or on the OBB surface.
    #[must_use]
    #[inline]
    pub fn contains_point(&self, point: Vec3) -> bool {
        let l = self.local_coords(point);
        l.x.abs() <= self.half_extents.x
            && l.y.abs() <= self.half_extents.y
            && l.z.abs() <= self.half_extents.z
    }

    /// Closest point on (or inside) the OBB to `point`.
    #[must_use]
    #[inline]
    pub fn closest_point(&self, point: Vec3) -> Vec3 {
        let l = self.local_coords(point);
        let c = Vec3::new(
            l.x.clamp(-self.half_extents.x, self.half_extents.x),
            l.y.clamp(-self.half_extents.y, self.half_extents.y),
            l.z.clamp(-self.half_extents.z, self.half_extents.z),
        );
        self.center + self.axes[0] * c.x + self.axes[1] * c.y + self.axes[2] * c.z
    }

    /// Squared distance from `point` to this OBB (0 if inside).
    #[must_use]
    #[inline]
    pub fn distance_sq_to_point(&self, point: Vec3) -> f32 {
        (point - self.closest_point(point)).length_sq()
    }

    /// Returns `true` if the sphere `(center, radius)` overlaps this OBB.
    #[must_use]
    #[inline]
    pub fn intersects_sphere(&self, center: Vec3, radius: f32) -> bool {
        self.distance_sq_to_point(center) <= radius * radius
    }

    /// Ray/OBB intersection using the slab method in OBB-local space.
    ///
    /// Returns `(t_entry, t_exit)` on hit.  Filter hits behind the origin with
    /// `t_exit >= 0`.
    #[must_use]
    pub fn intersects_ray(&self, origin: Vec3, dir: Vec3) -> Option<(f32, f32)> {
        let d = origin - self.center;
        let local_origin = Vec3::new(
            d.dot(self.axes[0]),
            d.dot(self.axes[1]),
            d.dot(self.axes[2]),
        );
        let local_dir = Vec3::new(
            dir.dot(self.axes[0]),
            dir.dot(self.axes[1]),
            dir.dot(self.axes[2]),
        );
        AABB::from_center_extents(Vec3::ZERO, self.half_extents)
            .intersects_ray(local_origin, local_dir)
    }

    /// Smallest AABB that encloses this OBB.
    #[must_use]
    pub fn to_aabb(&self) -> AABB {
        let e = self.half_extents;
        let world_extents = Vec3::new(
            self.axes[0].x.abs() * e.x + self.axes[1].x.abs() * e.y + self.axes[2].x.abs() * e.z,
            self.axes[0].y.abs() * e.x + self.axes[1].y.abs() * e.y + self.axes[2].y.abs() * e.z,
            self.axes[0].z.abs() * e.x + self.axes[1].z.abs() * e.y + self.axes[2].z.abs() * e.z,
        );
        AABB::from_center_extents(self.center, world_extents)
    }

    /// Half-extent of this OBB projected onto `axis` (which must be a unit vector).
    #[inline]
    fn project_extent(&self, axis: Vec3) -> f32 {
        let e = self.half_extents;
        self.axes[0].dot(axis).abs() * e.x
            + self.axes[1].dot(axis).abs() * e.y
            + self.axes[2].dot(axis).abs() * e.z
    }

    /// SAT overlap test on one separating axis candidate.
    ///
    /// Returns `false` (separated) when the axis is valid and shows a gap.
    #[inline]
    fn sat_separated(a: &OBB, b: &OBB, axis: Vec3) -> bool {
        let len_sq = axis.dot(axis);
        if len_sq < 1e-10 {
            return false; // degenerate (parallel edges) — not separating
        }
        let n = axis * (1.0 / len_sq.sqrt());
        let dist = (b.center - a.center).dot(n).abs();
        dist > a.project_extent(n) + b.project_extent(n)
    }

    /// Returns `true` if this OBB overlaps `other` (Separating Axis Theorem, 15 axes).
    #[must_use]
    pub fn intersects_obb(&self, other: &OBB) -> bool {
        // 3 face normals of self
        for i in 0..3 {
            if Self::sat_separated(self, other, self.axes[i]) {
                return false;
            }
        }
        // 3 face normals of other
        for i in 0..3 {
            if Self::sat_separated(self, other, other.axes[i]) {
                return false;
            }
        }
        // 9 edge cross products
        for i in 0..3 {
            for j in 0..3 {
                if Self::sat_separated(self, other, self.axes[i].cross(other.axes[j])) {
                    return false;
                }
            }
        }
        true
    }

    /// Returns `true` if this OBB overlaps `aabb`.
    #[must_use]
    #[inline]
    pub fn intersects_aabb(&self, aabb: &AABB) -> bool {
        self.intersects_obb(&OBB::from_aabb(aabb))
    }
}

// ── Capsule ───────────────────────────────────────────────────────────────────

/// A 3D capsule: a line segment `[a, b]` swept by a sphere of `radius`.
///
/// Capsules are the go-to shape for character controllers and swept-sphere
/// collision because their distance queries are closed-form and cheap.
///
/// # Examples
///
/// ```
/// use abrash_core::geometry::Capsule;
/// use abrash_core::math::Vec3;
///
/// let capsule = Capsule::new(Vec3::new(0.0, -1.0, 0.0), Vec3::new(0.0, 1.0, 0.0), 0.5);
/// assert!(capsule.contains_point(Vec3::ZERO));
/// assert!(!capsule.contains_point(Vec3::new(1.0, 0.0, 0.0)));
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Capsule {
    /// First endpoint of the interior segment.
    pub a: Vec3,
    /// Radius of the capsule.
    pub radius: f32,
    /// Second endpoint of the interior segment.
    pub b: Vec3,
    #[doc(hidden)]
    pub _pad: f32,
}

impl Capsule {
    /// Creates a new capsule from two endpoints and a radius.
    #[must_use]
    #[inline]
    pub const fn new(a: Vec3, b: Vec3, radius: f32) -> Self {
        Self {
            a,
            radius,
            b,
            _pad: 0.0,
        }
    }

    /// Creates a vertical capsule centered at `center`.
    ///
    /// `half_height` is measured from center to the *tip* of a hemisphere
    /// (i.e., the full half-height including the radius).
    /// The interior segment half-length is `half_height - radius`.
    #[must_use]
    pub fn from_center(center: Vec3, half_height: f32, radius: f32) -> Self {
        let inner = (half_height - radius).max(0.0);
        let offset = Vec3::new(0.0, inner, 0.0);
        Self::new(center - offset, center + offset, radius)
    }

    /// Closest point on the interior segment `[a, b]` to `point`.
    ///
    /// Also returns the interpolation parameter `t ∈ [0, 1]`.
    #[must_use]
    #[inline]
    pub fn closest_point_on_segment(&self, point: Vec3) -> (Vec3, f32) {
        let ab = self.b - self.a;
        let len_sq = ab.dot(ab);
        if len_sq < 1e-10 {
            return (self.a, 0.0); // degenerate capsule = sphere
        }
        let t = ((point - self.a).dot(ab) / len_sq).clamp(0.0, 1.0);
        (self.a + ab * t, t)
    }

    /// Squared distance from `point` to the interior segment.
    #[must_use]
    #[inline]
    pub fn segment_dist_sq(&self, point: Vec3) -> f32 {
        let (cp, _) = self.closest_point_on_segment(point);
        (point - cp).length_sq()
    }

    /// Returns `true` if `point` is inside or on the capsule surface.
    #[must_use]
    #[inline]
    pub fn contains_point(&self, point: Vec3) -> bool {
        self.segment_dist_sq(point) <= self.radius * self.radius
    }

    /// Returns `true` if the sphere `(center, radius)` overlaps this capsule.
    #[must_use]
    #[inline]
    pub fn intersects_sphere(&self, center: Vec3, radius: f32) -> bool {
        let r = self.radius + radius;
        self.segment_dist_sq(center) <= r * r
    }

    /// Returns `true` if this capsule overlaps `other`.
    ///
    /// Tests whether the minimum distance between the two interior segments is
    /// less than the sum of radii.
    #[must_use]
    pub fn intersects_capsule(&self, other: &Capsule) -> bool {
        let dist_sq = segment_segment_dist_sq(self.a, self.b, other.a, other.b);
        let r = self.radius + other.radius;
        dist_sq <= r * r
    }

    /// Ray/capsule intersection.
    ///
    /// Returns the entry distance `t ≥ 0` on hit, or `None` on miss.
    /// Handles both the cylindrical body and the spherical end-caps correctly.
    #[must_use]
    pub fn intersects_ray(&self, origin: Vec3, dir: Vec3) -> Option<f32> {
        let v = self.b - self.a; // segment direction (unnormalized)
        let w = origin - self.a;

        let vv = v.dot(v);
        let dv = dir.dot(v);
        let wv = w.dot(v);
        let wd = w.dot(dir);
        let ww = w.dot(w);
        let dd = dir.dot(dir);

        // Quadratic coefficients for the infinite-cylinder intersection
        let a = vv * dd - dv * dv;
        let half_b = vv * wd - wv * dv;
        let c = vv * ww - wv * wv - self.radius * self.radius * vv;

        let mut t_min = f32::MAX;

        if a.abs() > 1e-10 {
            let disc = half_b * half_b - a * c;
            if disc >= 0.0 {
                let sqrt_disc = disc.sqrt();
                let inv_a = 1.0 / a;
                for &sign in &[-1.0_f32, 1.0_f32] {
                    let t = (-half_b + sign * sqrt_disc) * inv_a;
                    if t >= 0.0 {
                        // Accept only hits where the projection falls inside [a, b]
                        let s = (wv + t * dv) / vv;
                        if (0.0..=1.0).contains(&s) {
                            t_min = t_min.min(t);
                        }
                    }
                }
            }
        }

        // End-cap hemispheres: treat as sphere hits, filter to the correct side
        let ray = Ray::new(origin, dir);
        if let Some(t) = ray.intersects_sphere(self.a, self.radius) {
            // Accept if hit projects onto the a-side (projection ≤ 0)
            let s = (wv + t * dv) / vv;
            if s <= 0.0 {
                t_min = t_min.min(t);
            }
        }
        if let Some(t) = ray.intersects_sphere(self.b, self.radius) {
            // Accept if hit projects onto the b-side (projection ≥ 1)
            let s = (wv + t * dv) / vv;
            if s >= 1.0 {
                t_min = t_min.min(t);
            }
        }

        if t_min < f32::MAX { Some(t_min) } else { None }
    }

    /// Smallest AABB enclosing this capsule.
    #[must_use]
    pub fn to_aabb(&self) -> AABB {
        let r = Vec3::new(self.radius, self.radius, self.radius);
        AABB::new(self.a.min(self.b) - r, self.a.max(self.b) + r)
    }
}

/// Squared distance between line segments `[p0, p1]` and `[q0, q1]`.
///
/// Handles all degenerate cases (one or both segments collapsed to a point).
fn segment_segment_dist_sq(p0: Vec3, p1: Vec3, q0: Vec3, q1: Vec3) -> f32 {
    let d1 = p1 - p0;
    let d2 = q1 - q0;
    let r = p0 - q0;

    let a = d1.dot(d1);
    let e = d2.dot(d2);
    let f = d2.dot(r);

    let (s, t) = if a <= 1e-10 && e <= 1e-10 {
        (0.0_f32, 0.0_f32)
    } else if a <= 1e-10 {
        (0.0, (f / e).clamp(0.0, 1.0))
    } else {
        let c = d1.dot(r);
        if e <= 1e-10 {
            ((-c / a).clamp(0.0, 1.0), 0.0)
        } else {
            let b_dot = d1.dot(d2);
            let denom = a * e - b_dot * b_dot;
            let s = if denom.abs() > 1e-10 {
                ((b_dot * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let t_unclamped = (b_dot * s + f) / e;
            if t_unclamped < 0.0 {
                let s2 = (-c / a).clamp(0.0, 1.0);
                (s2, 0.0)
            } else if t_unclamped > 1.0 {
                let s2 = ((b_dot - c) / a).clamp(0.0, 1.0);
                (s2, 1.0)
            } else {
                (s, t_unclamped)
            }
        }
    };

    let cp = p0 + d1 * s;
    let cq = q0 + d2 * t;
    (cp - cq).length_sq()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{Mat4, Vec3};
    use std::f32::consts::PI;

    // ── BoundingSphere::from_points ───────────────────────────────────────────

    #[test]
    fn bounding_sphere_from_points_empty_is_none() {
        assert!(BoundingSphere::from_points(&[]).is_none());
    }

    #[test]
    fn bounding_sphere_from_points_two_antipodal() {
        let pts = [Vec3::new(-1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)];
        let s = BoundingSphere::from_points(&pts).unwrap();
        assert!(s.center.x.abs() < 1e-5, "center should be at origin");
        assert!((s.radius - 1.0).abs() < 1e-5, "radius should be 1");
    }

    #[test]
    fn bounding_sphere_from_points_encloses_all() {
        let pts = [
            Vec3::new(3.0, 0.0, 0.0),
            Vec3::new(-1.0, 2.0, 0.0),
            Vec3::new(0.0, 0.0, -2.0),
        ];
        let s = BoundingSphere::from_points(&pts).unwrap();
        for &p in &pts {
            let d = (p - s.center).length();
            assert!(
                d <= s.radius + 1e-4,
                "point {p:?} at dist {d} outside sphere radius {}",
                s.radius
            );
        }
    }

    // ── Triangle ─────────────────────────────────────────────────────────────

    #[test]
    fn triangle_area_unit_right_angle() {
        let t = Triangle::new(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        assert!((t.area() - 0.5).abs() < 1e-5);
    }

    #[test]
    fn triangle_normal_unit_xy_plane() {
        let t = Triangle::new(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let n = t.normal();
        assert!(n.z.abs() > 0.99, "normal should point along Z: {n:?}");
    }

    #[test]
    fn triangle_centroid() {
        let t = Triangle::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(3.0, 0.0, 0.0),
            Vec3::new(0.0, 3.0, 0.0),
        );
        let c = t.centroid();
        assert!((c.x - 1.0).abs() < 1e-5);
        assert!((c.y - 1.0).abs() < 1e-5);
    }

    #[test]
    fn triangle_barycentric_vertices() {
        let t = Triangle::new(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let (u, v, w) = t.barycentric(t.a);
        assert!((u - 1.0).abs() < 1e-5 && v.abs() < 1e-5 && w.abs() < 1e-5);
        let (u, v, w) = t.barycentric(t.b);
        assert!(u.abs() < 1e-5 && (v - 1.0).abs() < 1e-5 && w.abs() < 1e-5);
    }

    #[test]
    fn triangle_closest_point_inside() {
        let t = Triangle::new(
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        // Point directly above centroid — closest point should be the projection
        let p = Vec3::new(0.0, 0.3, 1.0);
        let cp = t.closest_point(p);
        assert!((cp.z).abs() < 1e-5, "closest point should be on XY plane");
    }

    #[test]
    fn triangle_closest_point_to_vertex() {
        let t = Triangle::new(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        // Point well beyond vertex A
        let p = Vec3::new(-1.0, -1.0, 0.0);
        let cp = t.closest_point(p);
        assert!((cp - Vec3::ZERO).length() < 1e-5);
    }

    #[test]
    fn triangle_circumcenter_equilateral() {
        // Equilateral triangle — circumcenter should be the centroid.
        let side = 2.0_f32;
        let h = (side * side - 1.0_f32).sqrt();
        let t = Triangle::new(
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, h, 0.0),
        );
        let cc = t.circumcenter().unwrap();
        // All three vertices should be equidistant from circumcenter
        let r0 = (t.a - cc).length();
        let r1 = (t.b - cc).length();
        let r2 = (t.c - cc).length();
        assert!((r0 - r1).abs() < 1e-4, "r0={r0} r1={r1}");
        assert!((r1 - r2).abs() < 1e-4, "r1={r1} r2={r2}");
    }

    #[test]
    fn triangle_to_aabb() {
        let t = Triangle::new(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(-1.0, 0.0, 1.0),
            Vec3::new(0.0, 3.0, 2.0),
        );
        let aabb = t.to_aabb();
        assert!((aabb.min.x - (-1.0)).abs() < 1e-5);
        assert!((aabb.max.y - 3.0).abs() < 1e-5);
    }

    #[test]
    fn test_aabb_new() {
        let min = Vec3::new(-1.0, -2.0, -3.0);
        let max = Vec3::new(1.0, 2.0, 3.0);
        let aabb = AABB::new(min, max);

        assert_eq!(aabb.min, min);
        assert_eq!(aabb.max, max);
    }

    #[test]
    fn test_aabb_from_points_empty() {
        let points = [];
        let aabb = AABB::from_points(&points);

        assert_eq!(aabb.min, Vec3::default());
        assert_eq!(aabb.max, Vec3::default());
    }

    #[test]
    fn test_aabb_from_points_single() {
        let p = Vec3::new(5.0, -5.0, 10.0);
        let points = [p];
        let aabb = AABB::from_points(&points);

        assert_eq!(aabb.min, p);
        assert_eq!(aabb.max, p);
    }

    #[test]
    fn test_aabb_from_points_multiple() {
        let points = [
            Vec3::new(1.0, 5.0, -2.0),
            Vec3::new(-3.0, 0.0, 4.0),
            Vec3::new(2.0, -1.0, 0.0),
        ];
        let aabb = AABB::from_points(&points);

        // Expected min: (-3.0, -1.0, -2.0)
        // Expected max: (2.0, 5.0, 4.0)
        assert_eq!(aabb.min, Vec3::new(-3.0, -1.0, -2.0));
        assert_eq!(aabb.max, Vec3::new(2.0, 5.0, 4.0));
    }

    #[test]
    fn test_aabb_center_extents() {
        let min = Vec3::new(0.0, 0.0, 0.0);
        let max = Vec3::new(10.0, 20.0, 30.0);
        let aabb = AABB::new(min, max);

        let center = aabb.center();
        let extents = aabb.extents();

        assert_eq!(center, Vec3::new(5.0, 10.0, 15.0));
        assert_eq!(extents, Vec3::new(5.0, 10.0, 15.0));
    }

    #[test]
    fn test_aabb_transform_identity() {
        let aabb = AABB::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let transformed = aabb.transform(&Mat4::identity());

        assert_eq!(transformed.min, aabb.min);
        assert_eq!(transformed.max, aabb.max);
    }

    #[test]
    fn test_aabb_transform_translation() {
        let aabb = AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let m = Mat4::translation(10.0, 5.0, -2.0);
        let transformed = aabb.transform(&m);

        assert_eq!(transformed.min, Vec3::new(10.0, 5.0, -2.0));
        assert_eq!(transformed.max, Vec3::new(11.0, 6.0, -1.0));
    }

    #[test]
    fn test_aabb_transform_scale() {
        let aabb = AABB::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let m = Mat4::scale(2.0, 0.5, 3.0);
        let transformed = aabb.transform(&m);

        assert_eq!(transformed.min, Vec3::new(-2.0, -0.5, -3.0));
        assert_eq!(transformed.max, Vec3::new(2.0, 0.5, 3.0));
    }

    #[test]
    fn test_aabb_transform_rotation_90_y() {
        // Rotate 90 deg around Y.
        // Point (1, 0, 0) -> (0, 0, -1) (Right Hand Rule)
        let aabb = AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 1.0, 1.0));
        let m = Mat4::rotation_y(PI / 2.0);
        let transformed = aabb.transform(&m);

        // Original corners: (0,0,0) and (2,1,1)
        // (0,0,0) -> (0,0,0)
        // (2,1,1) -> (0.0, 1.0, -2.0) (approx)
        //
        // AABB of transformed points:
        // x range: min(0,0) to max(0,0) ? No.
        // Wait, Arvo's algorithm handles the extents.
        // AABB covers the OBB.
        //
        // Vertices of AABB:
        // (0,0,0) -> (0,0,0)
        // (2,0,0) -> (0,0,-2)
        // (0,1,0) -> (0,1,0)
        // (0,0,1) -> (1,0,0) -- Wait.
        // (2,1,1) -> (1,1,-2) ?
        //
        // Let's trace manually:
        // Row 0 (Right): (0, 0, -1)  (approx cos=0, sin=1 => c, 0, -s => 0, 0, -1)
        // Row 1 (Up):    (0, 1, 0)
        // Row 2 (Back):  (1, 0, 0)  (s, 0, c => 1, 0, 0)
        //
        // xa = (0,0,-1) * min.x(0) = (0,0,0)
        // xb = (0,0,-1) * max.x(2) = (0,0,-2)
        //
        // ya = (0,1,0) * min.y(0) = (0,0,0)
        // yb = (0,1,0) * max.y(1) = (0,1,0)
        //
        // za = (1,0,0) * min.z(0) = (0,0,0)
        // zb = (1,0,0) * max.z(1) = (1,0,0)
        //
        // min = sum(min(a,b)) = min(0,0,0, 0,0,-2) + min(0,0,0, 0,1,0) + min(0,0,0, 1,0,0)
        //     = (0,0,-2) + (0,0,0) + (0,0,0) = (0, 0, -2)
        //
        // max = sum(max(a,b)) = max(0,0,0, 0,0,-2) + max(0,0,0, 0,1,0) + max(0,0,0, 1,0,0)
        //     = (0,0,0) + (0,1,0) + (1,0,0) = (1, 1, 0)
        //
        // So expected New Min: (0, 0, -2), New Max: (1, 1, 0)

        // Tolerance for float math
        let expected_min = Vec3::new(0.0, 0.0, -2.0);
        let expected_max = Vec3::new(1.0, 1.0, 0.0);

        let diff_min = transformed.min - expected_min;
        let diff_max = transformed.max - expected_max;

        assert!(
            diff_min.length() < 0.001,
            "Min mismatch: {:?}",
            transformed.min
        );
        assert!(
            diff_max.length() < 0.001,
            "Max mismatch: {:?}",
            transformed.max
        );
    }

    #[test]
    fn test_ray_sphere_intersection() {
        let ray = Ray::new(Vec3::new(-5.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
        let t = ray
            .intersects_sphere(Vec3::ZERO, 1.0)
            .expect("expected hit");
        assert!((t - 4.0).abs() < 1e-6);
    }

    #[test]
    fn test_ray_triangle_intersection() {
        let ray = Ray::new(Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 0.0, -1.0));
        let v0 = Vec3::new(-1.0, -1.0, 0.0);
        let v1 = Vec3::new(1.0, -1.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);
        let t = ray
            .intersects_triangle(v0, v1, v2)
            .expect("expected hit on triangle");
        assert!((t - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_aabb_intersects_ray_struct() {
        let aabb = AABB::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let ray = Ray::new(Vec3::new(-2.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
        let (t_min, t_max) = aabb
            .intersects_ray_struct(ray)
            .expect("expected ray/aabb overlap");
        assert!((t_min - 1.0).abs() < 1e-6);
        assert!((t_max - 3.0).abs() < 1e-6);
    }

    // ── OBB tests ─────────────────────────────────────────────────────────────

    #[test]
    fn obb_axis_aligned_contains_point() {
        let obb = OBB::from_center_extents(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
        assert!(obb.contains_point(Vec3::ZERO));
        assert!(obb.contains_point(Vec3::new(1.0, 0.0, 0.0))); // on boundary
        assert!(!obb.contains_point(Vec3::new(1.01, 0.0, 0.0)));
    }

    #[test]
    fn obb_rotated_contains_and_excludes() {
        // 45-degree rotation around Z
        let angle = PI / 4.0;
        let (s, c) = angle.sin_cos();
        let m = Mat4 {
            m: [
                [c, s, 0.0, 0.0],
                [-s, c, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        };
        let aabb = AABB::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let obb = OBB::from_aabb_transform(&aabb, &m);

        // Origin should be inside
        assert!(obb.contains_point(Vec3::ZERO));
        // A point along the original X axis at distance < 1 should still be inside
        assert!(obb.contains_point(Vec3::new(0.5, 0.0, 0.0)));
    }

    #[test]
    fn obb_to_aabb_identity() {
        let obb = OBB::from_center_extents(Vec3::new(5.0, 0.0, 0.0), Vec3::new(2.0, 1.0, 3.0));
        let aabb = obb.to_aabb();
        let expected_min = Vec3::new(3.0, -1.0, -3.0);
        let expected_max = Vec3::new(7.0, 1.0, 3.0);
        assert!((aabb.min.x - expected_min.x).abs() < 1e-5);
        assert!((aabb.min.y - expected_min.y).abs() < 1e-5);
        assert!((aabb.min.z - expected_min.z).abs() < 1e-5);
        assert!((aabb.max.x - expected_max.x).abs() < 1e-5);
        assert!((aabb.max.y - expected_max.y).abs() < 1e-5);
        assert!((aabb.max.z - expected_max.z).abs() < 1e-5);
    }

    #[test]
    fn obb_ray_hit_axis_aligned() {
        let obb = OBB::from_center_extents(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
        let (t_min, t_max) = obb
            .intersects_ray(Vec3::new(-3.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0))
            .expect("should hit");
        assert!((t_min - 2.0).abs() < 1e-5);
        assert!((t_max - 4.0).abs() < 1e-5);
    }

    #[test]
    fn obb_ray_miss() {
        // Ray offset by 2 in Y — it passes above the OBB and will never hit.
        let obb = OBB::from_center_extents(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
        assert!(
            obb.intersects_ray(Vec3::new(-5.0, 2.0, 0.0), Vec3::new(1.0, 0.0, 0.0))
                .is_none()
        );
    }

    #[test]
    fn obb_intersects_sphere() {
        let obb = OBB::from_center_extents(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
        assert!(obb.intersects_sphere(Vec3::new(1.5, 0.0, 0.0), 0.6));
        assert!(!obb.intersects_sphere(Vec3::new(1.5, 0.0, 0.0), 0.4));
    }

    #[test]
    fn obb_intersects_obb_overlapping() {
        let a = OBB::from_center_extents(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
        let b = OBB::from_center_extents(Vec3::new(1.5, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        assert!(a.intersects_obb(&b));
    }

    #[test]
    fn obb_intersects_obb_separated() {
        let a = OBB::from_center_extents(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
        let b = OBB::from_center_extents(Vec3::new(3.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        assert!(!a.intersects_obb(&b));
    }

    #[test]
    fn obb_intersects_aabb() {
        let obb = OBB::from_center_extents(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
        let aabb = AABB::new(Vec3::new(0.5, 0.5, 0.5), Vec3::new(2.0, 2.0, 2.0));
        assert!(obb.intersects_aabb(&aabb));
    }

    // ── Capsule tests ─────────────────────────────────────────────────────────

    #[test]
    fn capsule_contains_point_on_axis() {
        let cap = Capsule::new(Vec3::new(0.0, -1.0, 0.0), Vec3::new(0.0, 1.0, 0.0), 0.5);
        assert!(cap.contains_point(Vec3::ZERO));
        assert!(cap.contains_point(Vec3::new(0.0, 1.5, 0.0))); // inside top hemisphere
        assert!(!cap.contains_point(Vec3::new(0.0, 2.0, 0.0))); // just outside
    }

    #[test]
    fn capsule_contains_point_radially() {
        let cap = Capsule::new(Vec3::ZERO, Vec3::new(0.0, 2.0, 0.0), 1.0);
        assert!(cap.contains_point(Vec3::new(0.9, 1.0, 0.0)));
        assert!(!cap.contains_point(Vec3::new(1.1, 1.0, 0.0)));
    }

    #[test]
    fn capsule_intersects_sphere() {
        let cap = Capsule::new(Vec3::ZERO, Vec3::new(0.0, 2.0, 0.0), 0.5);
        assert!(cap.intersects_sphere(Vec3::new(1.0, 1.0, 0.0), 0.6));
        assert!(!cap.intersects_sphere(Vec3::new(1.0, 1.0, 0.0), 0.4));
    }

    #[test]
    fn capsule_intersects_capsule_crossing() {
        let a = Capsule::new(Vec3::new(-2.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0), 0.3);
        let b = Capsule::new(Vec3::new(0.0, -2.0, 0.0), Vec3::new(0.0, 2.0, 0.0), 0.3);
        assert!(a.intersects_capsule(&b)); // cross at origin
    }

    #[test]
    fn capsule_intersects_capsule_separated() {
        let a = Capsule::new(Vec3::new(-5.0, 0.0, 0.0), Vec3::new(-3.0, 0.0, 0.0), 0.3);
        let b = Capsule::new(Vec3::new(3.0, 0.0, 0.0), Vec3::new(5.0, 0.0, 0.0), 0.3);
        assert!(!a.intersects_capsule(&b));
    }

    #[test]
    fn capsule_ray_hit_body() {
        // Ray along X, capsule along Y — should hit the cylindrical body
        let cap = Capsule::new(Vec3::new(0.0, -2.0, 0.0), Vec3::new(0.0, 2.0, 0.0), 1.0);
        let t = cap
            .intersects_ray(Vec3::new(-3.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0))
            .expect("should hit capsule body");
        assert!((t - 2.0).abs() < 1e-4, "expected t≈2, got {t}");
    }

    #[test]
    fn capsule_ray_hit_end_cap() {
        // Ray along Y from below, hitting the bottom hemisphere
        let cap = Capsule::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 2.0, 0.0), 0.5);
        let t = cap
            .intersects_ray(Vec3::new(0.0, -3.0, 0.0), Vec3::new(0.0, 1.0, 0.0))
            .expect("should hit bottom cap");
        assert!((t - 2.5).abs() < 1e-4, "expected t≈2.5, got {t}");
    }

    #[test]
    fn capsule_ray_miss() {
        let cap = Capsule::new(Vec3::ZERO, Vec3::new(0.0, 2.0, 0.0), 0.5);
        assert!(
            cap.intersects_ray(Vec3::new(2.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0))
                .is_none()
        );
    }

    #[test]
    fn capsule_to_aabb() {
        let cap = Capsule::new(Vec3::new(0.0, -1.0, 0.0), Vec3::new(0.0, 1.0, 0.0), 0.5);
        let aabb = cap.to_aabb();
        assert!((aabb.min.x - (-0.5)).abs() < 1e-5);
        assert!((aabb.min.y - (-1.5)).abs() < 1e-5);
        assert!((aabb.max.x - 0.5).abs() < 1e-5);
        assert!((aabb.max.y - 1.5).abs() < 1e-5);
    }
}
