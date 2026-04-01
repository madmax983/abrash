//! Decomposed TRS transform for smooth animation interpolation.

use crate::math::{Mat4, Vec3};
use crate::quat::Quat;

/// A decomposed Transform-Rotate-Scale (TRS) transform.
///
/// Unlike `Mat4`, this can be smoothly interpolated - position and scale
/// use linear interpolation while rotation uses SLERP via `Quat`.
///
/// # Convention
///
/// Composition order for row-vector convention (`v * M`):
/// `v * Scale * Rotation * Translation`
///
/// # Examples
///
/// ```
/// use abrash_core::transform::Transform;
/// use abrash_core::math::Vec3;
///
/// // Create a simple translation transform
/// let t = Transform::from_position(Vec3::new(10.0, 5.0, 0.0));
/// assert_eq!(t.position.x, 10.0);
///
/// // Or create an identity transform
/// let i = Transform::identity();
/// assert_eq!(i.scale, Vec3::ONE);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(missing_docs)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    /// Create a transform from explicit position, rotation, and scale components.
    #[must_use]
    #[inline]
    pub const fn new(position: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self {
            position,
            rotation,
            scale,
        }
    }

    /// Identity transform: origin, no rotation, uniform scale 1.
    #[must_use]
    #[inline]
    pub const fn identity() -> Self {
        Self::new(Vec3::ZERO, Quat::identity(), Vec3::ONE)
    }

    /// Create a transform with only a position (identity rotation, unit scale).
    #[must_use]
    #[inline]
    pub const fn from_position(position: Vec3) -> Self {
        Self::new(position, Quat::identity(), Vec3::ONE)
    }

    /// Create a transform with only a rotation.
    #[must_use]
    #[inline]
    pub const fn from_rotation(rotation: Quat) -> Self {
        Self::new(Vec3::ZERO, rotation, Vec3::ONE)
    }

    /// Create a transform with only a scale.
    #[must_use]
    #[inline]
    pub const fn from_scale(scale: Vec3) -> Self {
        Self::new(Vec3::ZERO, Quat::identity(), scale)
    }

    /// Compose into a 4x4 matrix: Scale * Rotation * Translation (row-vector convention).
    #[must_use]
    pub fn to_mat4(&self) -> Mat4 {
        let rotation = self.rotation.normalize();

        let x2 = rotation.x + rotation.x;
        let y2 = rotation.y + rotation.y;
        let z2 = rotation.z + rotation.z;
        let xx = rotation.x * x2;
        let xy = rotation.x * y2;
        let xz = rotation.x * z2;
        let yy = rotation.y * y2;
        let yz = rotation.y * z2;
        let zz = rotation.z * z2;
        let wx = rotation.w * x2;
        let wy = rotation.w * y2;
        let wz = rotation.w * z2;

        Mat4 {
            m: [
                [
                    (1.0 - (yy + zz)) * self.scale.x,
                    (xy + wz) * self.scale.x,
                    (xz - wy) * self.scale.x,
                    0.0,
                ],
                [
                    (xy - wz) * self.scale.y,
                    (1.0 - (xx + zz)) * self.scale.y,
                    (yz + wx) * self.scale.y,
                    0.0,
                ],
                [
                    (xz + wy) * self.scale.z,
                    (yz - wx) * self.scale.z,
                    (1.0 - (xx + yy)) * self.scale.z,
                    0.0,
                ],
                [self.position.x, self.position.y, self.position.z, 1.0],
            ],
        }
    }

    /// Linearly blend position and scale while slerping rotation.
    #[must_use]
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self::new(
            self.position.lerp(other.position, t),
            self.rotation.slerp(&other.rotation, t),
            self.scale.lerp(other.scale, t),
        )
    }

    /// Transform a point directly without materializing a matrix.
    #[must_use]
    #[inline]
    pub fn transform_point(&self, point: Vec3) -> Vec3 {
        self.rotation.rotate_vec3(point * self.scale) + self.position
    }

    /// Transform a direction vector (scale then rotate, without translation).
    #[must_use]
    #[inline]
    pub fn transform_vector(&self, vector: Vec3) -> Vec3 {
        self.rotation.rotate_vec3(vector * self.scale)
    }

    /// Apply the inverse transform directly to a point.
    ///
    /// Scale components must be non-zero for the inverse to be well-defined.
    #[must_use]
    #[inline]
    pub fn inverse_transform_point(&self, point: Vec3) -> Vec3 {
        let local = self.rotation.inverse().rotate_vec3(point - self.position);
        Vec3::new(
            local.x / self.scale.x,
            local.y / self.scale.y,
            local.z / self.scale.z,
        )
    }

    /// Apply the inverse transform directly to a direction vector.
    ///
    /// Scale components must be non-zero for the inverse to be well-defined.
    #[must_use]
    #[inline]
    pub fn inverse_transform_vector(&self, vector: Vec3) -> Vec3 {
        let local = self.rotation.inverse().rotate_vec3(vector);
        Vec3::new(
            local.x / self.scale.x,
            local.y / self.scale.y,
            local.z / self.scale.z,
        )
    }

    /// Transform a batch of points, reusing one output allocation.
    #[must_use]
    pub fn transform_points(&self, points: &[Vec3]) -> Vec<Vec3> {
        let mut out = Vec::with_capacity(points.len());
        self.transform_points_into(points, &mut out);
        out
    }

    /// Transform a batch of points and write into `out`.
    ///
    /// This hoists quaternion basis expansion outside the loop.
    pub fn transform_points_into(&self, points: &[Vec3], out: &mut Vec<Vec3>) {
        let rotation = self.rotation.normalize();
        let x2 = rotation.x + rotation.x;
        let y2 = rotation.y + rotation.y;
        let z2 = rotation.z + rotation.z;
        let xx = rotation.x * x2;
        let xy = rotation.x * y2;
        let xz = rotation.x * z2;
        let yy = rotation.y * y2;
        let yz = rotation.y * z2;
        let zz = rotation.z * z2;
        let wx = rotation.w * x2;
        let wy = rotation.w * y2;
        let wz = rotation.w * z2;

        let m00 = (1.0 - (yy + zz)) * self.scale.x;
        let m01 = (xy + wz) * self.scale.x;
        let m02 = (xz - wy) * self.scale.x;
        let m10 = (xy - wz) * self.scale.y;
        let m11 = (1.0 - (xx + zz)) * self.scale.y;
        let m12 = (yz + wx) * self.scale.y;
        let m20 = (xz + wy) * self.scale.z;
        let m21 = (yz - wx) * self.scale.z;
        let m22 = (1.0 - (xx + yy)) * self.scale.z;

        out.clear();
        out.resize(points.len(), Vec3::ZERO);
        for (dst, &p) in out.iter_mut().zip(points.iter()) {
            *dst = Vec3::new(
                p.x * m00 + p.y * m10 + p.z * m20 + self.position.x,
                p.x * m01 + p.y * m11 + p.z * m21 + self.position.y,
                p.x * m02 + p.y * m12 + p.z * m22 + self.position.z,
            );
        }
    }

    /// Transform a batch of direction vectors (scale + rotate, no translation).
    #[must_use]
    pub fn transform_vectors(&self, vectors: &[Vec3]) -> Vec<Vec3> {
        let mut out = Vec::with_capacity(vectors.len());
        self.transform_vectors_into(vectors, &mut out);
        out
    }

    /// Transform a batch of direction vectors and write into `out`.
    pub fn transform_vectors_into(&self, vectors: &[Vec3], out: &mut Vec<Vec3>) {
        let rotation = self.rotation.normalize();
        let x2 = rotation.x + rotation.x;
        let y2 = rotation.y + rotation.y;
        let z2 = rotation.z + rotation.z;
        let xx = rotation.x * x2;
        let xy = rotation.x * y2;
        let xz = rotation.x * z2;
        let yy = rotation.y * y2;
        let yz = rotation.y * z2;
        let zz = rotation.z * z2;
        let wx = rotation.w * x2;
        let wy = rotation.w * y2;
        let wz = rotation.w * z2;

        let m00 = (1.0 - (yy + zz)) * self.scale.x;
        let m01 = (xy + wz) * self.scale.x;
        let m02 = (xz - wy) * self.scale.x;
        let m10 = (xy - wz) * self.scale.y;
        let m11 = (1.0 - (xx + zz)) * self.scale.y;
        let m12 = (yz + wx) * self.scale.y;
        let m20 = (xz + wy) * self.scale.z;
        let m21 = (yz - wx) * self.scale.z;
        let m22 = (1.0 - (xx + yy)) * self.scale.z;

        out.clear();
        out.resize(vectors.len(), Vec3::ZERO);
        for (dst, &v) in out.iter_mut().zip(vectors.iter()) {
            *dst = Vec3::new(
                v.x * m00 + v.y * m10 + v.z * m20,
                v.x * m01 + v.y * m11 + v.z * m21,
                v.x * m02 + v.y * m12 + v.z * m22,
            );
        }
    }

    /// Apply the inverse transform to a batch of points.
    #[must_use]
    pub fn inverse_transform_points(&self, points: &[Vec3]) -> Vec<Vec3> {
        let mut out = Vec::with_capacity(points.len());
        self.inverse_transform_points_into(points, &mut out);
        out
    }

    /// Apply the inverse transform to a batch of points and write into `out`.
    pub fn inverse_transform_points_into(&self, points: &[Vec3], out: &mut Vec<Vec3>) {
        let inv_rotation = self.rotation.inverse().normalize();
        let inv_scale = Vec3::new(1.0 / self.scale.x, 1.0 / self.scale.y, 1.0 / self.scale.z);

        let x2 = inv_rotation.x + inv_rotation.x;
        let y2 = inv_rotation.y + inv_rotation.y;
        let z2 = inv_rotation.z + inv_rotation.z;
        let xx = inv_rotation.x * x2;
        let xy = inv_rotation.x * y2;
        let xz = inv_rotation.x * z2;
        let yy = inv_rotation.y * y2;
        let yz = inv_rotation.y * z2;
        let zz = inv_rotation.z * z2;
        let wx = inv_rotation.w * x2;
        let wy = inv_rotation.w * y2;
        let wz = inv_rotation.w * z2;

        let m00 = 1.0 - (yy + zz);
        let m01 = xy + wz;
        let m02 = xz - wy;
        let m10 = xy - wz;
        let m11 = 1.0 - (xx + zz);
        let m12 = yz + wx;
        let m20 = xz + wy;
        let m21 = yz - wx;
        let m22 = 1.0 - (xx + yy);

        out.clear();
        out.resize(points.len(), Vec3::ZERO);
        for (dst, &p) in out.iter_mut().zip(points.iter()) {
            let local = p - self.position;
            *dst = Vec3::new(
                (local.x * m00 + local.y * m10 + local.z * m20) * inv_scale.x,
                (local.x * m01 + local.y * m11 + local.z * m21) * inv_scale.y,
                (local.x * m02 + local.y * m12 + local.z * m22) * inv_scale.z,
            );
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    const EPSILON: f32 = 1e-5;

    fn assert_vec3_close(actual: Vec3, expected: Vec3) {
        assert!((actual.x - expected.x).abs() < EPSILON);
        assert!((actual.y - expected.y).abs() < EPSILON);
        assert!((actual.z - expected.z).abs() < EPSILON);
    }

    #[test]
    fn identity_produces_identity_matrix() {
        let t = Transform::identity();
        let m = t.to_mat4();
        let i = Mat4::identity();
        for row in 0..4 {
            for col in 0..4 {
                assert!(
                    (m.m[row][col] - i.m[row][col]).abs() < EPSILON,
                    "Mismatch at [{row}][{col}]"
                );
            }
        }
    }

    #[test]
    fn translation_only() {
        let t = Transform::new(Vec3::new(5.0, 10.0, 15.0), Quat::identity(), Vec3::ONE);
        let m = t.to_mat4();
        let expected = Mat4::translation(5.0, 10.0, 15.0);
        for row in 0..4 {
            for col in 0..4 {
                assert!(
                    (m.m[row][col] - expected.m[row][col]).abs() < EPSILON,
                    "Mismatch at [{row}][{col}]: got={} expected={}",
                    m.m[row][col],
                    expected.m[row][col]
                );
            }
        }
    }

    #[test]
    fn scale_only() {
        let t = Transform::new(Vec3::ZERO, Quat::identity(), Vec3::new(2.0, 3.0, 4.0));
        let m = t.to_mat4();
        let expected = Mat4::scale(2.0, 3.0, 4.0);
        for row in 0..4 {
            for col in 0..4 {
                assert!(
                    (m.m[row][col] - expected.m[row][col]).abs() < EPSILON,
                    "Mismatch at [{row}][{col}]"
                );
            }
        }
    }

    #[test]
    fn rotation_only() {
        let t = Transform::new(
            Vec3::ZERO,
            Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2),
            Vec3::ONE,
        );
        let m = t.to_mat4();
        let expected = Mat4::rotation_y(FRAC_PI_2);
        for row in 0..4 {
            for col in 0..4 {
                assert!(
                    (m.m[row][col] - expected.m[row][col]).abs() < EPSILON,
                    "Mismatch at [{row}][{col}]: got={} expected={}",
                    m.m[row][col],
                    expected.m[row][col]
                );
            }
        }
    }

    #[test]
    fn combined_srt() {
        let t = Transform::new(
            Vec3::new(10.0, 0.0, 0.0),
            Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2),
            Vec3::new(2.0, 2.0, 2.0),
        );
        let m = t.to_mat4();
        let expected = Mat4::scale(2.0, 2.0, 2.0)
            * Mat4::rotation_y(FRAC_PI_2)
            * Mat4::translation(10.0, 0.0, 0.0);
        for row in 0..4 {
            for col in 0..4 {
                assert!(
                    (m.m[row][col] - expected.m[row][col]).abs() < EPSILON,
                    "Mismatch at [{row}][{col}]: got={} expected={}",
                    m.m[row][col],
                    expected.m[row][col]
                );
            }
        }
    }

    #[test]
    fn from_position_helper() {
        let t = Transform::from_position(Vec3::new(1.0, 2.0, 3.0));
        assert!((t.position.x - 1.0).abs() < EPSILON);
        assert!((t.rotation.w - 1.0).abs() < EPSILON);
        assert!((t.scale.x - 1.0).abs() < EPSILON);
    }

    #[test]
    fn direct_point_transform_matches_matrix_path() {
        let transform = Transform::new(
            Vec3::new(3.0, -2.0, 5.0),
            Quat::from_euler(0.4, -0.2, 0.1),
            Vec3::new(2.0, 3.0, 0.5),
        );
        let point = Vec3::new(-1.5, 0.25, 2.0);

        let expected = transform.to_mat4().transform_point(point).0;
        assert_vec3_close(transform.transform_point(point), expected);
    }

    #[test]
    fn inverse_point_transform_roundtrips() {
        let transform = Transform::new(
            Vec3::new(-4.0, 1.5, 2.0),
            Quat::from_euler(0.3, -0.6, 0.2),
            Vec3::new(1.5, 0.75, 2.0),
        );
        let point = Vec3::new(0.5, -2.0, 1.25);

        let world = transform.transform_point(point);
        assert_vec3_close(transform.inverse_transform_point(world), point);
    }
}
