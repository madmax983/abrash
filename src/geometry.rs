//! Geometric primitives and utilities.

use crate::math::{Mat4, Vec3};

/// A Bounding Sphere for object-level culling.
///
/// used for coarse intersection tests before checking individual triangles.
///
/// # Examples
///
/// ```
/// use abrash::geometry::BoundingSphere;
/// use abrash::math::Vec3;
///
/// let sphere = BoundingSphere {
///     center: Vec3::new(0.0, 0.0, 0.0),
///     radius: 1.0,
/// };
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingSphere {
    pub center: Vec3,
    pub radius: f32,
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
/// use abrash::geometry::AABB;
/// use abrash::math::Vec3;
///
/// let min = Vec3::new(-1.0, -1.0, -1.0);
/// let max = Vec3::new(1.0, 1.0, 1.0);
/// let aabb = AABB::new(min, max);
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AABB {
    pub min: Vec3,
    pub pad0: f32, // Padding to align max to 16 bytes offset
    pub max: Vec3,
    pub pad1: f32, // Padding to make total size 32 bytes
}

impl AABB {
    /// Create a new AABB from min and max points.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::geometry::AABB;
    /// use abrash::math::Vec3;
    ///
    /// let aabb = AABB::new(Vec3::ZERO, Vec3::ONE);
    /// ```
    #[must_use]
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self {
            min,
            pad0: 0.0,
            max,
            pad1: 0.0,
        }
    }

    /// Calculate AABB from a list of points.
    ///
    /// Returns a default zero-sized AABB if the input list is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::geometry::AABB;
    /// use abrash::math::Vec3;
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
            if p.x < min.x {
                min.x = p.x;
            }
            if p.y < min.y {
                min.y = p.y;
            }
            if p.z < min.z {
                min.z = p.z;
            }

            if p.x > max.x {
                max.x = p.x;
            }
            if p.y > max.y {
                max.y = p.y;
            }
            if p.z > max.z {
                max.z = p.z;
            }
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
    /// use abrash::geometry::AABB;
    /// use abrash::math::{Mat4, Vec3};
    ///
    /// let aabb = AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
    /// let transform = Mat4::translation(10.0, 0.0, 0.0);
    /// let transformed_aabb = aabb.transform(&transform);
    ///
    /// assert_eq!(transformed_aabb.min.x, 10.0);
    /// ```
    #[must_use]
    pub fn transform(&self, transform: &Mat4) -> Self {
        #[cfg(all(target_arch = "x86_64", feature = "simd"))]
        if is_x86_feature_detected!("sse2") {
            // SAFETY: We checked feature detection.
            unsafe {
                return self.transform_simd(transform);
            }
        }

        let min = self.min;
        let max = self.max;
        let m = &transform.m;

        let right = Vec3::new(m[0][0], m[0][1], m[0][2]);
        let up = Vec3::new(m[1][0], m[1][1], m[1][2]);
        let back = Vec3::new(m[2][0], m[2][1], m[2][2]);
        let translation = Vec3::new(m[3][0], m[3][1], m[3][2]);

        let xa = right * min.x;
        let xb = right * max.x;

        let ya = up * min.y;
        let yb = up * max.y;

        let za = back * min.z;
        let zb = back * max.z;

        // Arvo's algorithm:
        // NewMin = Translation + sum(min(a, b))
        // NewMax = Translation + sum(max(a, b))
        // where a = M * min, b = M * max (component-wise)

        let world_min = translation + xa.min(xb) + ya.min(yb) + za.min(zb);
        let world_max = translation + xa.max(xb) + ya.max(yb) + za.max(zb);

        Self::new(world_min, world_max)
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

        assert!(diff_min.length() < 0.001, "Min mismatch: {:?}", transformed.min);
        assert!(diff_max.length() < 0.001, "Max mismatch: {:?}", transformed.max);
    }
}
