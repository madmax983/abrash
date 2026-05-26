#[allow(clippy::wildcard_imports)]
use super::*;
use std::mem::MaybeUninit;
use std::ops::{Add, Mul, Sub};

/// A 2x2 matrix, primarily used for 2D rotations and transformations.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Mat2, Vec2};
///
/// // Rotate 90 degrees (PI/2)
/// let rot = Mat2::rotation(std::f32::consts::FRAC_PI_2);
/// let v = Vec2::new(1.0, 0.0);
/// let v_prime = rot.transform(v);
///
/// // (1, 0) rotated 90 deg -> (0, 1)
/// assert!((v_prime.x).abs() < 1e-6);
/// assert!((v_prime.y - 1.0).abs() < 1e-6);
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct Mat2 {
    pub m: [[f32; 2]; 2],
}

impl Mat2 {
    /// Creates a 2D rotation matrix.
    ///
    /// * `angle`: Rotation angle in radians (counter-clockwise).
    #[must_use]
    pub fn rotation(angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();

        Self {
            m: [[cos, -sin], [sin, cos]],
        }
    }

    /// Transforms a vector by this matrix.
    ///
    /// # Performance
    ///
    /// Marked `#[inline]` to allow the compiler to optimize call overhead and potentially
    /// vectorize loops that call this function.
    #[must_use]
    #[inline]
    pub fn transform(&self, v: Vec2) -> Vec2 {
        Vec2 {
            x: self.m[0][0] * v.x + self.m[0][1] * v.y,
            y: self.m[1][0] * v.x + self.m[1][1] * v.y,
        }
    }

    /// Transform multiple vectors at once.
    #[must_use]
    pub fn transform_batch(&self, vertices: &[Vec2]) -> Vec<Vec2> {
        // ⚡ Bolt: Removed intermediate `.collect::<Vec<_>>()` and use `transform_batch_into`
        // to strictly control vector allocation and capacity.
        let mut out = Vec::with_capacity(vertices.len());
        self.transform_batch_into(vertices, &mut out);
        out
    }

    /// Transform a batch of vectors and write into `out`.
    ///
    /// ⚡ Bolt: Allows reusing an existing allocation, completely avoiding
    /// dynamic heap allocations per batch.
    pub fn transform_batch_into(&self, vertices: &[Vec2], out: &mut Vec<Vec2>) {
        let m00 = self.m[0][0];
        let m01 = self.m[0][1];
        let m10 = self.m[1][0];
        let m11 = self.m[1][1];

        out.clear();
        out.extend(vertices.iter().map(|&v| Vec2 {
            x: m00 * v.x + m01 * v.y,
            y: m10 * v.x + m11 * v.y,
        }));
    }

    /// Transform vertices in place.
    pub fn transform_in_place(&self, vertices: &mut [Vec2]) {
        let m00 = self.m[0][0];
        let m01 = self.m[0][1];
        let m10 = self.m[1][0];
        let m11 = self.m[1][1];

        for v in vertices.iter_mut() {
            let x = v.x;
            let y = v.y;
            v.x = m00 * x + m01 * y;
            v.y = m10 * x + m11 * y;
        }
    }

    /// Identity matrix (no transformation).
    #[must_use]
    #[inline]
    pub const fn identity() -> Self {
        Self {
            m: [[1.0, 0.0], [0.0, 1.0]],
        }
    }

    /// Non-uniform scale matrix.
    #[must_use]
    #[inline]
    pub const fn scale(sx: f32, sy: f32) -> Self {
        Self {
            m: [[sx, 0.0], [0.0, sy]],
        }
    }

    /// Transpose (swap rows and columns).
    #[must_use]
    #[inline]
    pub const fn transpose(&self) -> Self {
        Self {
            m: [[self.m[0][0], self.m[1][0]], [self.m[0][1], self.m[1][1]]],
        }
    }

    /// Determinant: `ad - bc`.
    #[must_use]
    #[inline]
    pub fn determinant(&self) -> f32 {
        self.m[0][0] * self.m[1][1] - self.m[0][1] * self.m[1][0]
    }

    /// Inverse. Returns the zero matrix if the determinant is near zero.
    #[must_use]
    pub fn inverse(&self) -> Self {
        let det = self.determinant();
        if det.abs() < 1e-7 {
            return Self { m: [[0.0; 2]; 2] };
        }
        let inv = 1.0 / det;
        Self {
            m: [
                [self.m[1][1] * inv, -self.m[0][1] * inv],
                [-self.m[1][0] * inv, self.m[0][0] * inv],
            ],
        }
    }
}

impl std::ops::Mul for Mat2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self {
            m: [
                [
                    self.m[0][0] * rhs.m[0][0] + self.m[0][1] * rhs.m[1][0],
                    self.m[0][0] * rhs.m[0][1] + self.m[0][1] * rhs.m[1][1],
                ],
                [
                    self.m[1][0] * rhs.m[0][0] + self.m[1][1] * rhs.m[1][0],
                    self.m[1][0] * rhs.m[0][1] + self.m[1][1] * rhs.m[1][1],
                ],
            ],
        }
    }
}
