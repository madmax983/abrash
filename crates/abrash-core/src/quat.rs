//! Unit quaternion for rotation representation.
//!
//! Quaternions avoid gimbal lock and enable smooth interpolation via SLERP.
//! Convention: `(x, y, z, w)` where `w` is the scalar (real) component.

use std::ops::Mul;

use crate::math::{Mat4, Vec3};

/// A unit quaternion representing a 3D rotation.
///
/// Stored as `(x, y, z, w)` where `w` is the scalar part.
/// For a rotation of angle θ around axis (ax, ay, az):
/// - `w = cos(θ/2)`
/// - `x = ax * sin(θ/2)`
/// - `y = ay * sin(θ/2)`
/// - `z = az * sin(θ/2)`
///
/// # Examples
///
/// ```
/// use abrash_core::quat::Quat;
/// use abrash_core::math::Vec3;
/// use std::f32::consts::PI;
///
/// // Create a quaternion for a 90-degree rotation around the Y-axis.
/// let rot = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), PI / 2.0);
/// let v = Vec3::new(1.0, 0.0, 0.0);
///
/// // Rotate the vector: (1, 0, 0) rotated 90 degrees around Y becomes (0, 0, -1)
/// let v_prime = rot.rotate_vec3(v);
/// assert!((v_prime.z + 1.0).abs() < 1e-6);
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(missing_docs)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quat {
    /// Create a quaternion from components.
    #[must_use]
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// The identity quaternion (no rotation).
    #[must_use]
    #[inline]
    pub const fn identity() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        }
    }

    /// Create a quaternion from an axis and angle (radians).
    ///
    /// The axis should be normalized. The rotation follows the right-hand rule.
    #[must_use]
    pub fn from_axis_angle(axis: Vec3, angle: f32) -> Self {
        let half = angle * 0.5;
        let (sin_half, cos_half) = half.sin_cos();
        Self {
            x: axis.x * sin_half,
            y: axis.y * sin_half,
            z: axis.z * sin_half,
            w: cos_half,
        }
    }

    /// Create a quaternion from Euler angles (pitch, yaw, roll) in radians.
    ///
    /// Rotation order: Yaw (Y) * Pitch (X) * Roll (Z), matching the
    /// row-vector convention where transformations read left-to-right.
    #[must_use]
    pub fn from_euler(pitch: f32, yaw: f32, roll: f32) -> Self {
        let qy = Self::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), yaw);
        let qx = Self::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), pitch);
        let qz = Self::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), roll);
        qy * qx * qz
    }

    /// Spherical linear interpolation between two quaternions.
    ///
    /// Always takes the shortest path (flips `other` if dot product is negative).
    #[must_use]
    pub fn slerp(&self, other: &Self, t: f32) -> Self {
        let mut dot = self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w;

        // Flip to take shortest path
        let other = if dot < 0.0 {
            dot = -dot;
            Self::new(-other.x, -other.y, -other.z, -other.w)
        } else {
            *other
        };

        // If very close, use linear interpolation to avoid division by zero
        if dot > 0.9995 {
            let result = Self::new(
                self.x + (other.x - self.x) * t,
                self.y + (other.y - self.y) * t,
                self.z + (other.z - self.z) * t,
                self.w + (other.w - self.w) * t,
            );
            return result.normalize();
        }

        let theta = dot.clamp(-1.0, 1.0).acos();
        let sin_theta = theta.sin();
        let a = ((1.0 - t) * theta).sin() / sin_theta;
        let b = (t * theta).sin() / sin_theta;

        Self::new(
            self.x * a + other.x * b,
            self.y * a + other.y * b,
            self.z * a + other.z * b,
            self.w * a + other.w * b,
        )
    }

    /// Normalized linear interpolation between two quaternions.
    ///
    /// Faster than [`Self::slerp`] and typically suitable for frame-to-frame blending.
    #[must_use]
    pub fn nlerp(&self, other: &Self, t: f32) -> Self {
        let mut end = *other;
        if self.dot(*other) < 0.0 {
            end = Self::new(-other.x, -other.y, -other.z, -other.w);
        }
        Self::new(
            self.x + (end.x - self.x) * t,
            self.y + (end.y - self.y) * t,
            self.z + (end.z - self.z) * t,
            self.w + (end.w - self.w) * t,
        )
        .normalize()
    }

    /// Normalize to unit length.
    #[must_use]
    pub fn normalize(self) -> Self {
        let len = (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt();
        if len < f32::EPSILON {
            return Self::identity();
        }
        let inv = 1.0 / len;
        Self::new(self.x * inv, self.y * inv, self.z * inv, self.w * inv)
    }

    /// Rotate a vector by this quaternion.
    ///
    /// Computes `q * v * q⁻¹` using the optimized formula.
    #[must_use]
    pub fn rotate_vec3(self, v: Vec3) -> Vec3 {
        // Optimized quaternion-vector rotation (avoids full quaternion multiply):
        // t = 2 * cross(q.xyz, v)
        // result = v + w * t + cross(q.xyz, t)
        let qv = Vec3::new(self.x, self.y, self.z);
        let t = qv.cross(v) * 2.0;
        v + t * self.w + qv.cross(t)
    }

    /// Convert to a 4x4 rotation matrix (row-major, row-vector convention).
    #[must_use]
    pub fn to_mat4(self) -> Mat4 {
        let x2 = self.x + self.x;
        let y2 = self.y + self.y;
        let z2 = self.z + self.z;
        let xx = self.x * x2;
        let xy = self.x * y2;
        let xz = self.x * z2;
        let yy = self.y * y2;
        let yz = self.y * z2;
        let zz = self.z * z2;
        let wx = self.w * x2;
        let wy = self.w * y2;
        let wz = self.w * z2;

        Mat4 {
            m: [
                [1.0 - (yy + zz), xy + wz, xz - wy, 0.0],
                [xy - wz, 1.0 - (xx + zz), yz + wx, 0.0],
                [xz + wy, yz - wx, 1.0 - (xx + yy), 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// The conjugate (inverse for unit quaternions).
    #[must_use]
    #[inline]
    pub const fn conjugate(self) -> Self {
        Self::new(-self.x, -self.y, -self.z, self.w)
    }

    /// Quaternion inverse.
    ///
    /// For unit quaternions this is equal to the conjugate.
    #[must_use]
    pub fn inverse(self) -> Self {
        let norm_sq = self.dot(self);
        if norm_sq <= f32::EPSILON {
            return Self::identity();
        }
        let c = self.conjugate();
        let inv = 1.0 / norm_sq;
        Self::new(c.x * inv, c.y * inv, c.z * inv, c.w * inv)
    }

    /// Dot product between two quaternions.
    #[must_use]
    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }
}

/// Hamilton product helper: computes `a * b` in standard quaternion algebra.
///
/// In standard Hamilton convention, `hamilton(a, b)` applied to a vector via
/// `q * v * q^-1` applies `b` first, then `a`.
fn hamilton(a: &Quat, b: &Quat) -> Quat {
    Quat::new(
        a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
        a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
        a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
        a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
    )
}

/// Compose rotations: `a * b` applies `a` first, then `b` (row-vector convention).
///
/// Internally computes `Hamilton(b, a)` so that `rotate_vec3(a * b, v)` equals
/// applying rotation `a` to `v`, then applying rotation `b` to the result.
impl Mul for Quat {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        // Row-vector convention: "a then b" = Hamilton(rhs, self)
        hamilton(&rhs, &self)
    }
}

impl Default for Quat {
    fn default() -> Self {
        Self::identity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_PI_2, PI};

    const EPSILON: f32 = 1e-5;

    #[test]
    fn identity_has_no_rotation() {
        let q = Quat::identity();
        assert!((q.x).abs() < EPSILON);
        assert!((q.y).abs() < EPSILON);
        assert!((q.z).abs() < EPSILON);
        assert!((q.w - 1.0).abs() < EPSILON);
    }

    #[test]
    fn from_axis_angle_90_degrees_y() {
        let q = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);
        // Should rotate (1,0,0) to (0,0,-1) in right-handed system
        let rotated = q.rotate_vec3(Vec3::new(1.0, 0.0, 0.0));
        assert!((rotated.x).abs() < EPSILON);
        assert!((rotated.y).abs() < EPSILON);
        assert!((rotated.z + 1.0).abs() < EPSILON);
    }

    #[test]
    fn slerp_at_zero_returns_start() {
        let a = Quat::identity();
        let b = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), PI);
        let result = a.slerp(&b, 0.0);
        assert!((result.x - a.x).abs() < EPSILON);
        assert!((result.y - a.y).abs() < EPSILON);
        assert!((result.z - a.z).abs() < EPSILON);
        assert!((result.w - a.w).abs() < EPSILON);
    }

    #[test]
    fn slerp_at_one_returns_end() {
        let a = Quat::identity();
        let b = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);
        let result = a.slerp(&b, 1.0);
        assert!((result.x - b.x).abs() < EPSILON);
        assert!((result.y - b.y).abs() < EPSILON);
        assert!((result.z - b.z).abs() < EPSILON);
        assert!((result.w - b.w).abs() < EPSILON);
    }

    #[test]
    fn slerp_midpoint_is_half_rotation() {
        let a = Quat::identity();
        let b = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);
        let mid = a.slerp(&b, 0.5);
        // Mid should be 45 degree rotation around Y
        let rotated = mid.rotate_vec3(Vec3::new(1.0, 0.0, 0.0));
        let expected_x = (FRAC_PI_2 / 2.0).cos();
        let expected_z = -(FRAC_PI_2 / 2.0).sin();
        assert!((rotated.x - expected_x).abs() < EPSILON);
        assert!((rotated.z - expected_z).abs() < EPSILON);
    }

    #[test]
    fn slerp_takes_shortest_path() {
        // Two quaternions representing the same rotation but with opposite signs
        let a = Quat::identity();
        let b_raw = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);
        let b_neg = Quat::new(-b_raw.x, -b_raw.y, -b_raw.z, -b_raw.w);
        // slerp should produce the same rotation regardless
        let r1 = a.slerp(&b_raw, 0.5).rotate_vec3(Vec3::new(1.0, 0.0, 0.0));
        let r2 = a.slerp(&b_neg, 0.5).rotate_vec3(Vec3::new(1.0, 0.0, 0.0));
        assert!((r1.x - r2.x).abs() < EPSILON);
        assert!((r1.y - r2.y).abs() < EPSILON);
        assert!((r1.z - r2.z).abs() < EPSILON);
    }

    #[test]
    fn nlerp_midpoint_is_normalized() {
        let a = Quat::identity();
        let b = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);
        let mid = a.nlerp(&b, 0.5);
        assert!((mid.dot(mid) - 1.0).abs() < EPSILON);
    }

    #[test]
    fn normalize_preserves_direction() {
        let q = Quat::new(1.0, 2.0, 3.0, 4.0);
        let n = q.normalize();
        let len = (n.x * n.x + n.y * n.y + n.z * n.z + n.w * n.w).sqrt();
        assert!((len - 1.0).abs() < EPSILON);
    }

    #[test]
    fn compose_rotations() {
        // 90 deg around Y then 90 deg around X
        let ry = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);
        let rx = Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), FRAC_PI_2);
        let combined = ry * rx; // Apply ry first, then rx (row-vector convention)
        let v = combined.rotate_vec3(Vec3::new(1.0, 0.0, 0.0));
        // (1,0,0) -> ry -> (0,0,-1) -> rx -> (0,1,0)
        assert!((v.x).abs() < EPSILON);
        assert!((v.y - 1.0).abs() < EPSILON);
        assert!((v.z).abs() < EPSILON);
    }

    #[test]
    fn to_mat4_matches_rotation_y() {
        let angle = 1.23_f32;
        let q = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), angle);
        let qmat = q.to_mat4();
        let rmat = Mat4::rotation_y(angle);
        for row in 0..4 {
            for col in 0..4 {
                assert!(
                    (qmat.m[row][col] - rmat.m[row][col]).abs() < EPSILON,
                    "Mismatch at [{row}][{col}]: quat={} rot={}",
                    qmat.m[row][col],
                    rmat.m[row][col]
                );
            }
        }
    }

    #[test]
    fn from_euler_roundtrip() {
        let pitch = 0.3;
        let yaw = 0.7;
        let roll = 0.0;
        let q = Quat::from_euler(pitch, yaw, roll);
        // Verify by rotating a known vector
        let v = q.rotate_vec3(Vec3::new(1.0, 0.0, 0.0));
        // Same rotation via matrices
        let m = Mat4::rotation_y(yaw) * Mat4::rotation_x(pitch);
        let (v2, _) = m.transform_point(Vec3::new(1.0, 0.0, 0.0));
        assert!((v.x - v2.x).abs() < EPSILON);
        assert!((v.y - v2.y).abs() < EPSILON);
        assert!((v.z - v2.z).abs() < EPSILON);
    }

    #[test]
    fn inverse_undoes_rotation() {
        let q = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);
        let v = Vec3::new(0.4, 0.2, -0.8);
        let vr = q.rotate_vec3(v);
        let back = q.inverse().rotate_vec3(vr);
        assert!((back.x - v.x).abs() < EPSILON);
        assert!((back.y - v.y).abs() < EPSILON);
        assert!((back.z - v.z).abs() < EPSILON);
    }
}
