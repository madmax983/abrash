#[allow(clippy::wildcard_imports)]
use super::*;
use std::mem::MaybeUninit;
use std::ops::{Add, Mul, Sub};

/// A 3×3 matrix for normals, 2D homogeneous transforms, and upper-left extraction.
///
/// Row-major, row-vector convention: `v' = v * M`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Mat3, Vec3};
///
/// let m = Mat3::identity();
/// let v = Vec3::new(1.0, 2.0, 3.0);
/// assert_eq!(m.transform(v), v);
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]

/// A 3x3 matrix.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Mat3;
/// let m = Mat3::identity();
/// ```
pub struct Mat3 {
    pub m: [[f32; 3]; 3],
}

impl Mat3 {
    /// Identity matrix.
    #[must_use]
    #[inline]
    pub const fn identity() -> Self {
        Self {
            m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        }
    }

    /// Extract the upper-left 3×3 from a `Mat4`.
    #[must_use]
    #[inline]
    pub const fn from_mat4(m: &Mat4) -> Self {
        Self {
            m: [
                [m.m[0][0], m.m[0][1], m.m[0][2]],
                [m.m[1][0], m.m[1][1], m.m[1][2]],
                [m.m[2][0], m.m[2][1], m.m[2][2]],
            ],
        }
    }

    /// Rotation around the X axis.
    #[must_use]
    pub fn rotation_x(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self {
            m: [[1.0, 0.0, 0.0], [0.0, c, s], [0.0, -s, c]],
        }
    }

    /// Rotation around the Y axis.
    #[must_use]
    pub fn rotation_y(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self {
            m: [[c, 0.0, -s], [0.0, 1.0, 0.0], [s, 0.0, c]],
        }
    }

    /// Rotation around the Z axis.
    #[must_use]
    pub fn rotation_z(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self {
            m: [[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]],
        }
    }

    /// Uniform scale.
    #[must_use]
    #[inline]
    pub const fn scale(sx: f32, sy: f32, sz: f32) -> Self {
        Self {
            m: [[sx, 0.0, 0.0], [0.0, sy, 0.0], [0.0, 0.0, sz]],
        }
    }

    /// Transpose.
    #[must_use]
    #[inline]
    pub const fn transpose(&self) -> Self {
        let m = &self.m;
        Self {
            m: [
                [m[0][0], m[1][0], m[2][0]],
                [m[0][1], m[1][1], m[2][1]],
                [m[0][2], m[1][2], m[2][2]],
            ],
        }
    }

    /// Determinant.
    #[must_use]
    #[inline]
    pub fn determinant(&self) -> f32 {
        let m = &self.m;
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    }

    /// Inverse. Returns zero matrix if not invertible (det ≈ 0).
    #[must_use]
    pub fn inverse(&self) -> Self {
        let m = &self.m;
        let c00 = m[1][1] * m[2][2] - m[1][2] * m[2][1];
        let c01 = -(m[1][0] * m[2][2] - m[1][2] * m[2][0]);
        let c02 = m[1][0] * m[2][1] - m[1][1] * m[2][0];
        let det = m[0][0] * c00 + m[0][1] * c01 + m[0][2] * c02;
        if det.abs() < 1e-6 {
            return Self { m: [[0.0; 3]; 3] };
        }
        let inv = 1.0 / det;
        Self {
            m: [
                [
                    c00 * inv,
                    (-(m[0][1] * m[2][2] - m[0][2] * m[2][1])) * inv,
                    (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv,
                ],
                [
                    c01 * inv,
                    (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv,
                    (-(m[0][0] * m[1][2] - m[0][2] * m[1][0])) * inv,
                ],
                [
                    c02 * inv,
                    (-(m[0][0] * m[2][1] - m[0][1] * m[2][0])) * inv,
                    (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv,
                ],
            ],
        }
    }

    /// Inverse-transpose — used to transform normal vectors correctly under non-uniform scaling.
    #[must_use]
    #[inline]
    pub fn inverse_transpose(&self) -> Self {
        self.inverse().transpose()
    }

    /// Transform a `Vec3` by this matrix (row-vector: `v' = v * M`).
    #[must_use]
    #[inline]
    pub fn transform(&self, v: Vec3) -> Vec3 {
        Vec3::new(
            v.x * self.m[0][0] + v.y * self.m[1][0] + v.z * self.m[2][0],
            v.x * self.m[0][1] + v.y * self.m[1][1] + v.z * self.m[2][1],
            v.x * self.m[0][2] + v.y * self.m[1][2] + v.z * self.m[2][2],
        )
    }
}

impl std::ops::Mul for Mat3 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        let mut result = Self { m: [[0.0; 3]; 3] };
        for i in 0..3 {
            for k in 0..3 {
                let s = self.m[i][k];
                for j in 0..3 {
                    result.m[i][j] += s * rhs.m[k][j];
                }
            }
        }
        result
    }
}

impl Default for Mat3 {
    fn default() -> Self {
        Self::identity()
    }
}

impl std::ops::Mul<Vec3> for Mat3 {
    type Output = Vec3;
    /// Transform a column vector by this row-major 3×3 matrix.
    #[inline]
    fn mul(self, v: Vec3) -> Vec3 {
        Vec3::new(
            self.m[0][0] * v.x + self.m[0][1] * v.y + self.m[0][2] * v.z,
            self.m[1][0] * v.x + self.m[1][1] * v.y + self.m[1][2] * v.z,
            self.m[2][0] * v.x + self.m[2][1] * v.y + self.m[2][2] * v.z,
        )
    }
}
