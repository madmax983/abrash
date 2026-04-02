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
            _mm256_add_ps, _mm256_max_ps, _mm256_min_ps, _mm256_mul_ps, _mm256_permute2f128_ps,
            _mm256_permute_ps, _mm256_set_m128, _mm256_storeu_ps, _mm_load_ps, _mm_set_ps,
        };
        #[cfg(target_arch = "x86_64")]
        use std::arch::x86_64::{
            _mm256_add_ps, _mm256_max_ps, _mm256_min_ps, _mm256_mul_ps, _mm256_permute2f128_ps,
            _mm256_permute_ps, _mm256_set_m128, _mm256_storeu_ps, _mm_load_ps, _mm_set_ps,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{Mat4, Vec3};
    use std::f32::consts::PI;

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
}
