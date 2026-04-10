//! Unit quaternion for rotation representation.
//!
//! Quaternions avoid gimbal lock and enable smooth interpolation via SLERP.
//! Convention: `(x, y, z, w)` where `w` is the scalar (real) component.

use std::ops::Mul;

use crate::math::{Mat4, Vec3, fast_inv_sqrt};

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
/// use abrash_core::math::Vec3;
/// use abrash_core::quat::Quat;
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

    /// Create a quaternion from the upper-left 3x3 portion of a rotation matrix.
    ///
    /// The matrix is expected to use Abrash's row-major, row-vector convention.
    /// For best results the basis vectors should be orthonormal.
    #[must_use]
    pub fn from_mat4(matrix: Mat4) -> Self {
        let m = matrix.m;
        let trace = m[0][0] + m[1][1] + m[2][2];

        let quat = if trace > 0.0 {
            let s = (trace + 1.0).sqrt() * 2.0;
            Self::new(
                (m[1][2] - m[2][1]) / s,
                (m[2][0] - m[0][2]) / s,
                (m[0][1] - m[1][0]) / s,
                0.25 * s,
            )
        } else if m[0][0] > m[1][1] && m[0][0] > m[2][2] {
            let s = (1.0 + m[0][0] - m[1][1] - m[2][2]).sqrt() * 2.0;
            Self::new(
                0.25 * s,
                (m[0][1] + m[1][0]) / s,
                (m[0][2] + m[2][0]) / s,
                (m[1][2] - m[2][1]) / s,
            )
        } else if m[1][1] > m[2][2] {
            let s = (1.0 + m[1][1] - m[0][0] - m[2][2]).sqrt() * 2.0;
            Self::new(
                (m[0][1] + m[1][0]) / s,
                0.25 * s,
                (m[1][2] + m[2][1]) / s,
                (m[2][0] - m[0][2]) / s,
            )
        } else {
            let s = (1.0 + m[2][2] - m[0][0] - m[1][1]).sqrt() * 2.0;
            Self::new(
                (m[0][2] + m[2][0]) / s,
                (m[1][2] + m[2][1]) / s,
                0.25 * s,
                (m[0][1] - m[1][0]) / s,
            )
        };

        quat.normalize()
    }

    /// Create a quaternion that rotates `from` direction into `to` direction.
    ///
    /// Returns [`Self::identity`] when either input has near-zero length.
    /// Handles the antiparallel case (180° turn) by selecting a stable orthogonal axis.
    #[must_use]
    pub fn from_to_rotation(from: Vec3, to: Vec3) -> Self {
        let from_len_sq = from.length_sq();
        let to_len_sq = to.length_sq();
        if from_len_sq <= 1e-8 || to_len_sq <= 1e-8 {
            return Self::identity();
        }

        let f = from.normalize();
        let t = to.normalize();
        let dot = f.dot(t);

        if dot > 1.0 - 1e-6 {
            return Self::identity();
        }

        if dot < -1.0 + 1e-6 {
            let mut axis = Vec3::new(1.0, 0.0, 0.0).cross(f);
            if axis.length_sq() <= 1e-8 {
                axis = Vec3::new(0.0, 1.0, 0.0).cross(f);
            }
            return Self::from_axis_angle(axis.normalize(), std::f32::consts::PI);
        }

        let c = f.cross(t);
        Self::new(c.x, c.y, c.z, 1.0 + dot).normalize()
    }

    /// Create a quaternion from a forward direction and approximate up direction.
    ///
    /// This is the quaternion equivalent of a camera/object "look rotation".
    /// Returns [`Self::identity`] if `forward` is degenerate.
    #[must_use]
    pub fn look_rotation(forward: Vec3, up: Vec3) -> Self {
        let f_len_sq = forward.length_sq();
        if f_len_sq <= 1e-8 {
            return Self::identity();
        }

        let f = forward.normalize();
        let mut r = up.cross(f);
        if r.length_sq() <= 1e-8 {
            let fallback_up = if f.y.abs() < 0.999 {
                Vec3::new(0.0, 1.0, 0.0)
            } else {
                Vec3::new(1.0, 0.0, 0.0)
            };
            r = fallback_up.cross(f);
        }
        r = r.normalize();
        let u = f.cross(r);

        let m00 = r.x;
        let m01 = u.x;
        let m02 = -f.x;
        let m10 = r.y;
        let m11 = u.y;
        let m12 = -f.y;
        let m20 = r.z;
        let m21 = u.z;
        let m22 = -f.z;

        let trace = m00 + m11 + m22;
        let q = if trace > 0.0 {
            let s = (trace + 1.0).sqrt() * 2.0;
            Self::new((m12 - m21) / s, (m20 - m02) / s, (m01 - m10) / s, 0.25 * s)
        } else if m00 > m11 && m00 > m22 {
            let s = (1.0 + m00 - m11 - m22).sqrt() * 2.0;
            Self::new(0.25 * s, (m01 + m10) / s, (m20 + m02) / s, (m12 - m21) / s)
        } else if m11 > m22 {
            let s = (1.0 + m11 - m00 - m22).sqrt() * 2.0;
            Self::new((m01 + m10) / s, 0.25 * s, (m12 + m21) / s, (m20 - m02) / s)
        } else {
            let s = (1.0 + m22 - m00 - m11).sqrt() * 2.0;
            Self::new((m20 + m02) / s, (m12 + m21) / s, 0.25 * s, (m01 - m10) / s)
        };

        q.normalize()
    }

    /// Spherical linear interpolation between two quaternions.
    ///
    /// Always takes the shortest path (flips `other` if dot product is negative).
    #[must_use]
    pub fn slerp(&self, other: &Self, t: f32) -> Self {
        let mut dot = self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w;

        let other = if dot < 0.0 {
            dot = -dot;
            Self::new(-other.x, -other.y, -other.z, -other.w)
        } else {
            *other
        };

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
        let end = if self.dot(*other) < 0.0 {
            Self::new(-other.x, -other.y, -other.z, -other.w)
        } else {
            *other
        };
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
        let len_sq = self.length_sq();
        if len_sq < f32::EPSILON * f32::EPSILON {
            return Self::identity();
        }
        let inv = len_sq.sqrt().recip();
        Self::new(self.x * inv, self.y * inv, self.z * inv, self.w * inv)
    }

    /// Normalize to unit length using fast inverse square root approximation.
    #[must_use]
    pub fn fast_normalize(self) -> Self {
        let len_sq = self.length_sq();
        if len_sq < f32::EPSILON * f32::EPSILON {
            return Self::identity();
        }
        let inv = fast_inv_sqrt(len_sq);
        Self::new(self.x * inv, self.y * inv, self.z * inv, self.w * inv)
    }

    /// Rotate a vector by this quaternion.
    ///
    /// Computes `q * v * q⁻¹` using the optimized formula.
    #[must_use]
    pub fn rotate_vec3(self, v: Vec3) -> Vec3 {
        let qv = Vec3::new(self.x, self.y, self.z);
        let t = qv.cross(v) * 2.0;
        v + t * self.w + qv.cross(t)
    }

    /// Rotate many vectors with one quaternion.
    ///
    /// This hoists the quaternion-to-matrix expansion out of the loop so each
    /// point only performs a compact 3x3 transform.
    #[must_use]
    pub fn rotate_vec3_batch(self, vectors: &[Vec3]) -> Vec<Vec3> {
        let mut out = Vec::with_capacity(vectors.len());
        self.rotate_vec3_batch_into(vectors, &mut out);
        out
    }

    /// Rotate many vectors with one quaternion and append into `out`.
    ///
    /// `out` is cleared and re-used to avoid temporary allocations in frame loops.
    pub fn rotate_vec3_batch_into(self, vectors: &[Vec3], out: &mut Vec<Vec3>) {
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
        out.resize(vectors.len(), Vec3::ZERO);
        for (dst, &v) in out.iter_mut().zip(vectors.iter()) {
            *dst = Vec3::new(
                v.x * m00 + v.y * m10 + v.z * m20,
                v.x * m01 + v.y * m11 + v.z * m21,
                v.x * m02 + v.y * m12 + v.z * m22,
            );
        }
    }

    /// Rotate many vectors in place with one quaternion.
    pub fn rotate_vec3_batch_in_place(self, vectors: &mut [Vec3]) {
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

        let m00 = 1.0 - (yy + zz);
        let m01 = xy + wz;
        let m02 = xz - wy;
        let m10 = xy - wz;
        let m11 = 1.0 - (xx + zz);
        let m12 = yz + wx;
        let m20 = xz + wy;
        let m21 = yz - wx;
        let m22 = 1.0 - (xx + yy);

        for v in vectors {
            let x = v.x;
            let y = v.y;
            let z = v.z;
            *v = Vec3::new(
                x * m00 + y * m10 + z * m20,
                x * m01 + y * m11 + z * m21,
                x * m02 + y * m12 + z * m22,
            );
        }
    }

    /// Convert this quaternion to axis-angle form `(axis, angle_radians)`.
    #[must_use]
    pub fn to_axis_angle(self) -> (Vec3, f32) {
        let q = self.normalize();
        let angle = 2.0 * q.w.clamp(-1.0, 1.0).acos();
        let s_sq = (1.0 - q.w * q.w).max(0.0);
        if s_sq <= 1e-12 {
            return (Vec3::new(1.0, 0.0, 0.0), 0.0);
        }
        let inv_s = s_sq.sqrt().recip();
        (
            Vec3::new(q.x * inv_s, q.y * inv_s, q.z * inv_s).normalize(),
            angle,
        )
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

    /// The squared magnitude of the quaternion.
    #[must_use]
    #[inline]
    pub fn length_sq(self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w
    }

    /// Quaternion inverse.
    ///
    /// For unit quaternions this is equal to the conjugate.
    #[must_use]
    pub fn inverse(self) -> Self {
        let norm_sq = self.length_sq();
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
        hamilton(&rhs, &self)
    }
}

impl Default for Quat {
    fn default() -> Self {
        Self::identity()
    }
}

// ── Mat4::decompose ───────────────────────────────────────────────────────────

impl Mat4 {
    /// Decompose this matrix into translation, rotation, and scale components.
    ///
    /// Returns `(translation, rotation, scale)`.
    ///
    /// This is the inverse of the `Transform::to_mat4` / `Mat4::scale * rotate * translate`
    /// pattern.  Assumes the matrix was constructed without shear; shear is
    /// silently ignored and folded into the rotation.
    ///
    /// # Notes
    ///
    /// - Sign of scale is lost for reflections; check `determinant() < 0` if
    ///   that matters.
    /// - The matrix uses **row-vector convention**: translation lives in row 3.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::{Mat4, Vec3};
    /// use abrash_core::quat::Quat;
    ///
    /// let m = Mat4::scale(2.0, 3.0, 4.0) * Mat4::translation(1.0, 2.0, 3.0);
    /// let (t, _q, s) = m.decompose();
    /// assert!((t.x - 1.0).abs() < 1e-5);
    /// assert!((s.y - 3.0).abs() < 1e-5);
    /// ```
    #[must_use]
    pub fn decompose(&self) -> (Vec3, Quat, Vec3) {
        let m = &self.m;

        // Row-vector convention: translation in row 3, xyz.
        let translation = Vec3::new(m[3][0], m[3][1], m[3][2]);

        // Scale = length of each basis-vector row.
        let sx = Vec3::new(m[0][0], m[0][1], m[0][2]).length();
        let sy = Vec3::new(m[1][0], m[1][1], m[1][2]).length();
        let sz = Vec3::new(m[2][0], m[2][1], m[2][2]).length();
        let scale = Vec3::new(sx, sy, sz);

        // Rotation = normalised basis rows assembled back into a pure-rotation Mat4.
        let inv_sx = if sx > 1e-8 { 1.0 / sx } else { 0.0 };
        let inv_sy = if sy > 1e-8 { 1.0 / sy } else { 0.0 };
        let inv_sz = if sz > 1e-8 { 1.0 / sz } else { 0.0 };

        let rot = Self {
            m: [
                [m[0][0] * inv_sx, m[0][1] * inv_sx, m[0][2] * inv_sx, 0.0],
                [m[1][0] * inv_sy, m[1][1] * inv_sy, m[1][2] * inv_sy, 0.0],
                [m[2][0] * inv_sz, m[2][1] * inv_sz, m[2][2] * inv_sz, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        };

        let rotation = Quat::from_mat4(rot);
        (translation, rotation, scale)
    }

    /// Build a TRS (Scale→Rotate→Translate) matrix from components.
    ///
    /// This is the inverse of [`Mat4::decompose`].
    /// The resulting matrix follows row-vector convention: `v' = v · M`.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::{Mat4, Vec3};
    /// use abrash_core::quat::Quat;
    ///
    /// let q = Quat::identity();
    /// let m = Mat4::from_srt(Vec3::ONE, q, Vec3::new(1.0, 2.0, 3.0));
    /// let (t, _, _) = m.decompose();
    /// assert!((t.x - 1.0).abs() < 1e-5);
    /// assert!((t.z - 3.0).abs() < 1e-5);
    /// ```
    #[must_use]
    pub fn from_srt(scale: Vec3, rotation: Quat, translation: Vec3) -> Self {
        // Build scale * rotation * translation in row-vector order
        Self::scale(scale.x, scale.y, scale.z)
            * rotation.to_mat4()
            * Self::translation(translation.x, translation.y, translation.z)
    }
}

// ── Dual Quaternion ──────────────────────────────────────────��────────────────

/// A dual quaternion encoding a rigid-body transform (rotation + translation).
///
/// Stored as two quaternions `real` (rotation) and `dual` (translation encoded
/// as `0.5 * t * real` where `t` is the pure-quaternion form of the translation).
///
/// Dual quaternions are the preferred representation for skeletal animation
/// blending (DQS — Dual Quaternion Skinning) because they avoid the "candy
/// wrapper" artifact produced by linear blend skinning (LBS) near joints.
///
/// # Examples
///
/// ```
/// use abrash_core::quat::{DualQuat, Quat};
/// use abrash_core::math::Vec3;
/// use std::f32::consts::PI;
///
/// let q = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), PI / 2.0);
/// let t = Vec3::new(1.0, 0.0, 0.0);
/// let dq = DualQuat::from_rotation_translation(q, t);
/// let v_out = dq.transform_point(Vec3::ZERO);
/// // The origin is just translated
/// assert!((v_out.x - 1.0).abs() < 1e-5);
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DualQuat {
    /// Real part — encodes rotation (must be a unit quaternion).
    pub real: Quat,
    /// Dual part — encodes translation: `dual = 0.5 * t_quat * real`.
    pub dual: Quat,
}

impl DualQuat {
    /// Identity transform (no rotation, no translation).
    #[must_use]
    #[inline]
    pub const fn identity() -> Self {
        Self {
            real: Quat {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
            dual: Quat {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 0.0,
            },
        }
    }

    /// Create from a unit quaternion with no translation.
    #[must_use]
    #[inline]
    pub const fn from_rotation(q: Quat) -> Self {
        Self {
            real: q,
            dual: Quat {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 0.0,
            },
        }
    }

    /// Create from a translation with no rotation.
    #[must_use]
    #[inline]
    pub fn from_translation(t: Vec3) -> Self {
        Self::from_rotation_translation(
            Quat {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
            t,
        )
    }

    /// Create from a rotation quaternion and a translation vector.
    ///
    /// The dual part is computed as `0.5 * (0, t) * real` where `(0, t)` is the
    /// pure quaternion form of the translation.
    #[must_use]
    pub fn from_rotation_translation(q: Quat, t: Vec3) -> Self {
        // t as pure quat: (tx, ty, tz, 0)
        let tx = Quat {
            x: t.x,
            y: t.y,
            z: t.z,
            w: 0.0,
        };
        // dual = 0.5 * t_quat * q
        let tq = tx * q;
        let dual = Quat {
            x: 0.5 * tq.x,
            y: 0.5 * tq.y,
            z: 0.5 * tq.z,
            w: 0.5 * tq.w,
        };
        Self { real: q, dual }
    }

    /// Extract the translation component.
    #[must_use]
    pub fn translation(&self) -> Vec3 {
        // t_quat = 2 * dual * conj(real)
        let r_conj = self.real.inverse();
        let tq = Quat {
            x: 2.0 * self.dual.x,
            y: 2.0 * self.dual.y,
            z: 2.0 * self.dual.z,
            w: 2.0 * self.dual.w,
        } * r_conj;
        Vec3::new(tq.x, tq.y, tq.z)
    }

    /// Normalize so the real part is a unit quaternion.
    #[must_use]
    pub fn normalize(&self) -> Self {
        let inv_len = 1.0 / self.real.length_sq().sqrt();
        let dot = self.real.dot(self.dual);
        Self {
            real: Quat {
                x: self.real.x * inv_len,
                y: self.real.y * inv_len,
                z: self.real.z * inv_len,
                w: self.real.w * inv_len,
            },
            dual: Quat {
                x: (self.dual.x - self.real.x * dot * inv_len) * inv_len,
                y: (self.dual.y - self.real.y * dot * inv_len) * inv_len,
                z: (self.dual.z - self.real.z * dot * inv_len) * inv_len,
                w: (self.dual.w - self.real.w * dot * inv_len) * inv_len,
            },
        }
    }

    /// Linear blend of two dual quaternions (DLB — Dual Linear Blending).
    ///
    /// `t = 0.0` returns `*self`; `t = 1.0` returns `*other`.
    /// The result is normalized to maintain unit length.
    ///
    /// For multi-bone blending use the full DLB formula:
    /// `(w0*dq0 + w1*dq1 + ...).normalize()`.
    #[must_use]
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        // Ensure shortest-path in the real component
        let dot = self.real.dot(other.real);
        let sign = if dot < 0.0 { -1.0_f32 } else { 1.0_f32 };
        let s = 1.0 - t;
        let blended = Self {
            real: Quat {
                x: s * self.real.x + t * sign * other.real.x,
                y: s * self.real.y + t * sign * other.real.y,
                z: s * self.real.z + t * sign * other.real.z,
                w: s * self.real.w + t * sign * other.real.w,
            },
            dual: Quat {
                x: s * self.dual.x + t * sign * other.dual.x,
                y: s * self.dual.y + t * sign * other.dual.y,
                z: s * self.dual.z + t * sign * other.dual.z,
                w: s * self.dual.w + t * sign * other.dual.w,
            },
        };
        blended.normalize()
    }

    /// Transform a 3D point: rotate then translate.
    #[must_use]
    pub fn transform_point(&self, p: Vec3) -> Vec3 {
        // 1. Apply rotation (real part)
        let rotated = self.real.rotate_vec3(p);
        // 2. Add translation
        rotated + self.translation()
    }

    /// Transform a 3D direction vector (rotation only, no translation).
    #[must_use]
    #[inline]
    pub fn transform_vector(&self, v: Vec3) -> Vec3 {
        self.real.rotate_vec3(v)
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
        let rotated = mid.rotate_vec3(Vec3::new(1.0, 0.0, 0.0));
        let expected_x = (FRAC_PI_2 / 2.0).cos();
        let expected_z = -(FRAC_PI_2 / 2.0).sin();
        assert!((rotated.x - expected_x).abs() < EPSILON);
        assert!((rotated.z - expected_z).abs() < EPSILON);
    }

    #[test]
    fn slerp_takes_shortest_path() {
        let a = Quat::identity();
        let b_raw = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);
        let b_neg = Quat::new(-b_raw.x, -b_raw.y, -b_raw.z, -b_raw.w);
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
    fn fast_normalize_accuracy() {
        let q = Quat::new(1.0, 2.0, 3.0, 4.0);
        let n1 = q.normalize();
        let n2 = q.fast_normalize();

        let diff_x = (n1.x - n2.x).abs();
        let diff_y = (n1.y - n2.y).abs();
        let diff_z = (n1.z - n2.z).abs();
        let diff_w = (n1.w - n2.w).abs();

        assert!(diff_x < 0.001);
        assert!(diff_y < 0.001);
        assert!(diff_z < 0.001);
        assert!(diff_w < 0.001);
    }

    #[test]
    fn compose_rotations() {
        let ry = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);
        let rx = Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), FRAC_PI_2);
        let combined = ry * rx;
        let v = combined.rotate_vec3(Vec3::new(1.0, 0.0, 0.0));
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
        let v = q.rotate_vec3(Vec3::new(1.0, 0.0, 0.0));
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

    #[test]
    fn from_to_rotation_rotates_source_to_target() {
        let from = Vec3::new(1.0, 0.0, 0.0);
        let to = Vec3::new(0.0, 0.0, -1.0);
        let q = Quat::from_to_rotation(from, to);
        let rotated = q.rotate_vec3(from);
        assert!((rotated.x - to.x).abs() < EPSILON);
        assert!((rotated.y - to.y).abs() < EPSILON);
        assert!((rotated.z - to.z).abs() < EPSILON);
    }

    #[test]
    fn from_to_rotation_handles_opposite_vectors() {
        let from = Vec3::new(0.0, 1.0, 0.0);
        let to = Vec3::new(0.0, -1.0, 0.0);
        let q = Quat::from_to_rotation(from, to);
        let rotated = q.rotate_vec3(from);
        assert!((rotated.x - to.x).abs() < EPSILON);
        assert!((rotated.y - to.y).abs() < EPSILON);
        assert!((rotated.z - to.z).abs() < EPSILON);
    }

    #[test]
    fn look_rotation_faces_forward_direction() {
        let forward = Vec3::new(0.0, 0.0, -1.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let q = Quat::look_rotation(forward, up);
        let looked = q.rotate_vec3(Vec3::new(0.0, 0.0, -1.0));
        assert!((looked.x - forward.x).abs() < EPSILON);
        assert!((looked.y - forward.y).abs() < EPSILON);
        assert!((looked.z - forward.z).abs() < EPSILON);
    }

    #[test]
    fn look_rotation_handles_parallel_up_and_forward() {
        let forward = Vec3::new(0.0, 1.0, 0.0);
        let q = Quat::look_rotation(forward, Vec3::new(0.0, 1.0, 0.0));
        let looked = q.rotate_vec3(Vec3::new(0.0, 0.0, -1.0));
        assert!((looked.x - forward.x).abs() < 1e-4);
        assert!((looked.y - forward.y).abs() < 1e-4);
        assert!((looked.z - forward.z).abs() < 1e-4);
    }

    #[test]
    fn rotate_vec3_batch_matches_scalar_path() {
        let q = Quat::from_euler(0.3, -0.7, 0.2).normalize();
        let input = [
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(-2.3, 4.1, 0.75),
        ];
        let output = q.rotate_vec3_batch(&input);
        assert_eq!(output.len(), input.len());
        for i in 0..input.len() {
            let scalar = q.rotate_vec3(input[i]);
            let batched = output[i];
            assert!((scalar.x - batched.x).abs() < EPSILON);
            assert!((scalar.y - batched.y).abs() < EPSILON);
            assert!((scalar.z - batched.z).abs() < EPSILON);
        }
    }

    #[test]
    fn rotate_vec3_batch_in_place_matches_allocating_path() {
        let q = Quat::from_euler(0.2, -0.6, 0.4).normalize();
        let mut in_place = vec![
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(-2.0, 1.5, 3.0),
            Vec3::new(0.25, -0.75, 2.5),
        ];
        let expected = q.rotate_vec3_batch(&in_place);
        q.rotate_vec3_batch_in_place(&mut in_place);
        for (a, b) in in_place.iter().zip(expected.iter()) {
            assert!((a.x - b.x).abs() < EPSILON);
            assert!((a.y - b.y).abs() < EPSILON);
            assert!((a.z - b.z).abs() < EPSILON);
        }
    }

    #[test]
    fn decompose_translation_roundtrip() {
        let t = Vec3::new(3.0, -1.5, 7.0);
        let m = Mat4::translation(t.x, t.y, t.z);
        let (out_t, _q, out_s) = m.decompose();
        assert!((out_t.x - t.x).abs() < 1e-5, "tx");
        assert!((out_t.y - t.y).abs() < 1e-5, "ty");
        assert!((out_t.z - t.z).abs() < 1e-5, "tz");
        assert!((out_s.x - 1.0).abs() < 1e-5, "sx should be 1");
    }

    #[test]
    fn decompose_scale_roundtrip() {
        let m = Mat4::scale(2.0, 3.0, 0.5);
        let (_t, _q, s) = m.decompose();
        assert!((s.x - 2.0).abs() < 1e-5, "sx");
        assert!((s.y - 3.0).abs() < 1e-5, "sy");
        assert!((s.z - 0.5).abs() < 1e-5, "sz");
    }

    #[test]
    fn from_srt_roundtrip() {
        use std::f32::consts::PI;
        let q = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), PI / 6.0).normalize();
        let s = Vec3::new(2.0, 1.5, 0.5);
        let t = Vec3::new(-3.0, 1.0, 4.0);
        let m = Mat4::from_srt(s, q, t);
        let (t_out, q_out, s_out) = m.decompose();
        assert!((t_out.x - t.x).abs() < 1e-4, "tx {}", t_out.x);
        assert!((t_out.z - t.z).abs() < 1e-4, "tz {}", t_out.z);
        assert!((s_out.x - s.x).abs() < 1e-4, "sx {}", s_out.x);
        // rotation should preserve vector rotation
        let v = Vec3::X;
        let a = q.rotate_vec3(v);
        let b = q_out.rotate_vec3(v);
        assert!((a.x - b.x).abs() < 1e-4);
    }

    #[test]
    fn decompose_trs_roundtrip() {
        use std::f32::consts::PI;
        let axis = Vec3::new(0.0, 1.0, 0.0);
        let angle = PI / 4.0;
        let q_in = Quat::from_axis_angle(axis, angle).normalize();
        let s_in = Vec3::new(2.0, 2.0, 2.0);
        let t_in = Vec3::new(5.0, -3.0, 1.0);

        // Build TRS matrix (row-vector convention: scale * rotate * translate)
        let m = Mat4::scale(s_in.x, s_in.y, s_in.z)
            * q_in.to_mat4()
            * Mat4::translation(t_in.x, t_in.y, t_in.z);

        let (t_out, q_out, s_out) = m.decompose();

        assert!((t_out.x - t_in.x).abs() < 1e-4, "tx {}", t_out.x);
        assert!((t_out.y - t_in.y).abs() < 1e-4, "ty {}", t_out.y);
        assert!((t_out.z - t_in.z).abs() < 1e-4, "tz {}", t_out.z);
        assert!((s_out.x - s_in.x).abs() < 1e-4, "sx {}", s_out.x);
        assert!((s_out.y - s_in.y).abs() < 1e-4, "sy {}", s_out.y);
        assert!((s_out.z - s_in.z).abs() < 1e-4, "sz {}", s_out.z);

        // Rotation: verify a test vector rotated by q_in ≈ rotated by q_out
        let v = Vec3::new(1.0, 0.0, 0.0);
        let a = q_in.rotate_vec3(v);
        let b = q_out.rotate_vec3(v);
        assert!((a.x - b.x).abs() < 1e-4, "qx");
        assert!((a.y - b.y).abs() < 1e-4, "qy");
        assert!((a.z - b.z).abs() < 1e-4, "qz");
    }

    #[test]
    fn to_axis_angle_roundtrip_preserves_rotation() {
        let axis = Vec3::new(0.3, -0.4, 0.5).normalize();
        let angle = 1.234;
        let q = Quat::from_axis_angle(axis, angle).normalize();
        let (out_axis, out_angle) = q.to_axis_angle();
        let reconstructed = Quat::from_axis_angle(out_axis, out_angle).normalize();
        let v = Vec3::new(0.7, -0.2, 0.5);
        let a = q.rotate_vec3(v);
        let b = reconstructed.rotate_vec3(v);
        assert!((a.x - b.x).abs() < EPSILON);
        assert!((a.y - b.y).abs() < EPSILON);
        assert!((a.z - b.z).abs() < EPSILON);
    }

    // ── DualQuat tests ──────────────────────��──────────────────────────��──────

    #[test]
    fn dual_quat_identity_transform() {
        let dq = DualQuat::identity();
        let v = Vec3::new(1.0, 2.0, 3.0);
        let out = dq.transform_point(v);
        assert!((out.x - v.x).abs() < 1e-5);
        assert!((out.y - v.y).abs() < 1e-5);
        assert!((out.z - v.z).abs() < 1e-5);
    }

    #[test]
    fn dual_quat_from_rotation_no_translation() {
        use std::f32::consts::PI;
        let q = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), PI / 2.0);
        let dq = DualQuat::from_rotation(q);
        let v = Vec3::new(1.0, 0.0, 0.0);
        let out = dq.transform_point(v);
        let expected = q.rotate_vec3(v);
        assert!(
            (out.x - expected.x).abs() < 1e-5,
            "x: {} vs {}",
            out.x,
            expected.x
        );
        assert!(
            (out.z - expected.z).abs() < 1e-5,
            "z: {} vs {}",
            out.z,
            expected.z
        );
    }

    #[test]
    fn dual_quat_translation() {
        let t = Vec3::new(3.0, -1.0, 2.0);
        let dq = DualQuat::from_translation(t);
        let v = Vec3::new(1.0, 1.0, 1.0);
        let out = dq.transform_point(v);
        assert!((out.x - (v.x + t.x)).abs() < 1e-5);
        assert!((out.y - (v.y + t.y)).abs() < 1e-5);
        assert!((out.z - (v.z + t.z)).abs() < 1e-5);
    }

    #[test]
    fn dual_quat_from_rotation_translation_roundtrip() {
        use std::f32::consts::PI;
        let q = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), PI / 3.0).normalize();
        let t = Vec3::new(1.0, 2.0, -3.0);
        let dq = DualQuat::from_rotation_translation(q, t);
        let v = Vec3::new(1.0, 0.0, 0.0);
        // Manual: rotate then translate
        let expected = q.rotate_vec3(v) + t;
        let out = dq.transform_point(v);
        assert!(
            (out.x - expected.x).abs() < 1e-4,
            "x: {} vs {}",
            out.x,
            expected.x
        );
        assert!(
            (out.y - expected.y).abs() < 1e-4,
            "y: {} vs {}",
            out.y,
            expected.y
        );
        assert!(
            (out.z - expected.z).abs() < 1e-4,
            "z: {} vs {}",
            out.z,
            expected.z
        );
    }

    #[test]
    fn dual_quat_lerp_midpoint_is_intermediate() {
        let a = DualQuat::from_translation(Vec3::ZERO);
        let b = DualQuat::from_translation(Vec3::new(2.0, 0.0, 0.0));
        let mid = a.lerp(&b, 0.5);
        let out = mid.transform_point(Vec3::ZERO);
        assert!((out.x - 1.0).abs() < 1e-4, "x: {}", out.x);
    }
}
