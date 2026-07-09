#![allow(dead_code)]
#![allow(clippy::suboptimal_flops)]
#![allow(clippy::should_panic_without_expect)]
#![allow(clippy::ignore_without_reason)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::float_cmp)]
#[allow(clippy::wildcard_imports)]
use super::*;
use std::mem::MaybeUninit;
use std::ops::{Add, Mul, Sub};

/// Unit quaternion representing a 3-D rotation.
///
/// Stored as `(x, y, z, w)` where `w` is the scalar part.
/// Operations assume the quaternion is normalised; call [`Quat::normalize`]
/// after accumulating many multiplications.
///
/// # Conventions
/// - Hamilton product: `q1 * q2` applies `q1` THEN `q2` (same as matrix order).
/// - Rotation direction: right-hand rule (CCW when axis points toward viewer).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    /// The X component of the scaled rotation axis (vector part).
    pub x: f32,
    /// The Y component of the scaled rotation axis (vector part).
    pub y: f32,
    /// The Z component of the scaled rotation axis (vector part).
    pub z: f32,
    /// The scalar real component determining the angle of rotation.
    pub w: f32,
}

impl Quat {
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

    /// Construct from a normalised `axis` and a rotation `angle` in radians.
    ///
    /// # Examples
    /// ```
    /// use abrash_core::math::{Quat, Vec3};
    /// let q = Quat::from_axis_angle(Vec3::Y, core::f32::consts::FRAC_PI_2);
    /// let v = q.rotate(Vec3::X);
    /// assert!((v.x - 0.0).abs() < 1e-5, "x: {}", v.x);
    /// assert!((v.z + 1.0).abs() < 1e-5, "z: {}", v.z); // +X rotated 90° about Y → -Z
    /// ```
    #[must_use]
    #[inline]
    pub fn from_axis_angle(axis: Vec3, angle: f32) -> Self {
        let (s, c) = (angle * 0.5).sin_cos();
        Self {
            x: axis.x * s,
            y: axis.y * s,
            z: axis.z * s,
            w: c,
        }
    }

    /// Construct from Euler angles (yaw, pitch, roll) in radians, ZYX order.
    ///
    /// Applies: roll (Z) first, then pitch (X), then yaw (Y).
    #[must_use]
    #[inline]
    pub fn from_euler_zyx(yaw: f32, pitch: f32, roll: f32) -> Self {
        let (sy, cy) = (yaw * 0.5).sin_cos();
        let (sp, cp) = (pitch * 0.5).sin_cos();
        let (sr, cr) = (roll * 0.5).sin_cos();
        Self {
            x: cy * sp * cr + sy * cp * sr,
            y: sy * cp * cr - cy * sp * sr,
            z: cy * cp * sr - sy * sp * cr,
            w: cy * cp * cr + sy * sp * sr,
        }
    }

    /// Dot product of two quaternions (measures alignment; 1 = identical).
    #[must_use]
    #[inline]
    pub fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z + self.w * rhs.w
    }

    /// Squared magnitude.
    #[must_use]
    #[inline]
    pub fn length_sq(self) -> f32 {
        self.dot(self)
    }

    /// Magnitude (should be ≈ 1.0 for unit quaternions).
    #[must_use]
    #[inline]
    pub fn length(self) -> f32 {
        self.length_sq().sqrt()
    }

    /// Return the normalised form.
    #[must_use]
    #[inline]
    pub fn normalize(self) -> Self {
        let inv = 1.0 / self.length();
        Self {
            x: self.x * inv,
            y: self.y * inv,
            z: self.z * inv,
            w: self.w * inv,
        }
    }

    /// Conjugate `(−x, −y, −z, w)` — the inverse for unit quaternions.
    #[must_use]
    #[inline]
    pub const fn conjugate(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            w: self.w,
        }
    }

    /// Inverse (conjugate / |q|²). Use [`Quat::conjugate`] for unit quats.
    #[must_use]
    #[inline]
    pub fn inverse(self) -> Self {
        let inv_sq = 1.0 / self.length_sq();
        Self {
            x: -self.x * inv_sq,
            y: -self.y * inv_sq,
            z: -self.z * inv_sq,
            w: self.w * inv_sq,
        }
    }

    /// Rotate a `Vec3` by this quaternion using the sandwich product
    /// `q * v * q⁻¹` (expanded without full quaternion multiplication).
    ///
    /// # Examples
    /// ```
    /// use abrash_core::math::{Quat, Vec3};
    /// // 180° rotation about Y maps +X → -X.
    /// let q = Quat::from_axis_angle(Vec3::Y, core::f32::consts::PI);
    /// let v = q.rotate(Vec3::X);
    /// assert!((v.x + 1.0).abs() < 1e-5, "x: {}", v.x);
    /// ```
    #[must_use]
    #[inline]
    pub fn rotate(self, v: Vec3) -> Vec3 {
        // Efficient Fabian Giessen formula: 2 * cross products.
        let t = Vec3::new(
            2.0 * (self.y * v.z - self.z * v.y),
            2.0 * (self.z * v.x - self.x * v.z),
            2.0 * (self.x * v.y - self.y * v.x),
        );
        Vec3::new(
            v.x + self.w * t.x + self.y * t.z - self.z * t.y,
            v.y + self.w * t.y + self.z * t.x - self.x * t.z,
            v.z + self.w * t.z + self.x * t.y - self.y * t.x,
        )
    }

    /// Spherical linear interpolation between `self` and `end` by factor `t ∈ [0,1]`.
    ///
    /// Automatically flips `end` if the dot product is negative (takes the short arc).
    ///
    /// # Examples
    /// ```
    /// use abrash_core::math::{Quat, Vec3};
    /// let q0 = Quat::identity();
    /// let q1 = Quat::from_axis_angle(Vec3::Y, core::f32::consts::FRAC_PI_2);
    /// let mid = q0.slerp(q1, 0.5);
    /// // Midpoint should rotate +X by ~45°.
    /// let v = mid.rotate(Vec3::X);
    /// assert!(v.x > 0.0 && v.z < 0.0);
    /// ```
    #[must_use]
    #[inline]
    pub fn slerp(self, end: Self, t: f32) -> Self {
        let mut dot = self.dot(end);
        // Take the short arc.
        let end = if dot < 0.0 {
            dot = -dot;
            Self {
                x: -end.x,
                y: -end.y,
                z: -end.z,
                w: -end.w,
            }
        } else {
            end
        };

        if dot > 0.999_9 {
            // Nearly identical: lerp + normalise.
            return Self {
                x: self.x + t * (end.x - self.x),
                y: self.y + t * (end.y - self.y),
                z: self.z + t * (end.z - self.z),
                w: self.w + t * (end.w - self.w),
            }
            .normalize();
        }

        let theta = dot.acos();
        let sin_theta = theta.sin();
        let s0 = ((1.0 - t) * theta).sin() / sin_theta;
        let s1 = (t * theta).sin() / sin_theta;
        Self {
            x: s0 * self.x + s1 * end.x,
            y: s0 * self.y + s1 * end.y,
            z: s0 * self.z + s1 * end.z,
            w: s0 * self.w + s1 * end.w,
        }
    }

    /// Convert to a row-major 3×3 rotation matrix (matching `Mat3` storage).
    ///
    /// # Examples
    /// ```
    /// use abrash_core::math::{Quat, Vec3};
    /// let m = Quat::identity().to_mat3();
    /// // Identity quat → identity matrix diagonal.
    /// assert!((m.m[0][0] - 1.0).abs() < 1e-6);
    /// assert!((m.m[1][1] - 1.0).abs() < 1e-6);
    /// assert!((m.m[2][2] - 1.0).abs() < 1e-6);
    /// ```
    #[must_use]
    pub fn to_mat3(self) -> Mat3 {
        let (x, y, z, w) = (self.x, self.y, self.z, self.w);
        let x2 = x + x;
        let y2 = y + y;
        let z2 = z + z;
        let xx = x * x2;
        let xy = x * y2;
        let xz = x * z2;
        let yy = y * y2;
        let yz = y * z2;
        let zz = z * z2;
        let wx = w * x2;
        let wy = w * y2;
        let wz = w * z2;
        // Row-major: m[row][col]
        Mat3 {
            m: [
                [1.0 - (yy + zz), xy - wz, xz + wy], // row 0
                [xy + wz, 1.0 - (xx + zz), yz - wx], // row 1
                [xz - wy, yz + wx, 1.0 - (xx + yy)], // row 2
            ],
        }
    }

    /// Angle (in radians) of the rotation represented by this quaternion.
    #[must_use]
    #[inline]
    pub fn angle(self) -> f32 {
        2.0 * self.w.clamp(-1.0, 1.0).acos()
    }

    /// Axis of the rotation (normalised). Returns `Vec3::Y` for the identity.
    #[must_use]
    #[inline]
    pub fn axis(self) -> Vec3 {
        let sin_half = (1.0 - self.w * self.w).sqrt();
        if sin_half < 1e-6 {
            Vec3::Y
        } else {
            let inv = 1.0 / sin_half;
            Vec3::new(self.x * inv, self.y * inv, self.z * inv)
        }
    }
}

impl std::ops::Mul for Quat {
    type Output = Self;
    /// Hamilton product: applies `self` first, then `rhs`.
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self {
            x: self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
            y: self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
            z: self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
            w: self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
        }
    }
}

impl Default for Quat {
    fn default() -> Self {
        Self::identity()
    }
}
