//! 2D and 3D math types for graphics programming.
//!
//! # Coordinate System
//!
//! This library uses a **Right-Handed** coordinate system.
//! *   **X**: Right
//! *   **Y**: Up
//! *   **Z**: Backward (Camera looks down -Z)
//!
//! # Matrix Convention
//!
//! Matrices are stored in **Row-Major** order.
//!
//! Transformations follow the **Row-Vector** convention ($v \cdot M$), meaning vectors are treated as rows and multiplied on the left.
//!
//! $$ v' = v \cdot M $$
//!
//! This implies that the order of multiplication matches the order of transformations:
//!
//! ```
//! # use abrash::math::{Mat4, Vec3};
//! // Scale, then Rotate, then Translate
//! let scale = Mat4::scale(2.0, 2.0, 2.0);
//! let rotate = Mat4::rotation_y(1.57); // 90 degrees
//! let translate = Mat4::translation(10.0, 0.0, 0.0);
//!
//! // Combine transformations
//! let transform = scale * rotate * translate;
//!
//! // Apply to a vector
//! let v = Vec3::new(1.0, 0.0, 0.0);
//! let (v_prime, _) = transform.transform_point(v);
//! ```
//!
//! All operations use `f32` for compatibility with graphics APIs.

use std::ops::{Add, Mul, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl Add for Vec2 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Vec2 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;

    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Mat2 {
    pub m: [[f32; 2]; 2],
}

impl Mat2 {
    #[must_use]
    pub fn rotation(angle: f32) -> Self {
        let cos = angle.cos();
        let sin = angle.sin();

        Self {
            m: [[cos, -sin], [sin, cos]],
        }
    }

    #[must_use]
    pub fn transform(&self, v: Vec2) -> Vec2 {
        Vec2 {
            x: self.m[0][0] * v.x + self.m[0][1] * v.y,
            y: self.m[1][0] * v.x + self.m[1][1] * v.y,
        }
    }

    /// Transform multiple vectors at once
    #[must_use]
    pub fn transform_batch(&self, vertices: &[Vec2]) -> Vec<Vec2> {
        vertices.iter().map(|&v| self.transform(v)).collect()
    }

    /// Transform vertices in place
    pub fn transform_in_place(&self, vertices: &mut [Vec2]) {
        for v in vertices.iter_mut() {
            *v = self.transform(*v);
        }
    }
}

/// A 3-component vector commonly used for positions, directions, and colors.
///
/// # Examples
///
/// ```
/// use abrash::math::Vec3;
///
/// let v = Vec3::new(1.0, 2.0, 3.0);
/// assert_eq!(v.x, 1.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    /// Creates a new vector.
    #[must_use]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Calculates the dot product with another vector.
    ///
    /// The dot product represents the projection of one vector onto another.
    /// *   Positive if pointing in similar direction.
    /// *   Zero if perpendicular.
    /// *   Negative if pointing in opposite directions.
    #[must_use]
    pub fn dot(&self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Calculates the cross product with another vector.
    ///
    /// Returns a vector perpendicular to both input vectors using the Right-Hand Rule.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::math::Vec3;
    ///
    /// // X cross Y = Z (Right-Handed)
    /// let x = Vec3::new(1.0, 0.0, 0.0);
    /// let y = Vec3::new(0.0, 1.0, 0.0);
    /// let z = x.cross(y);
    ///
    /// assert_eq!(z, Vec3::new(0.0, 0.0, 1.0));
    /// ```
    #[must_use]
    pub fn cross(&self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    /// Calculates the Euclidean length (magnitude) of the vector.
    #[must_use]
    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Returns a normalized unit vector (length of 1.0).
    ///
    /// # Behavior for Small Vectors
    ///
    /// If the vector's length is less than `0.0001`, this function returns
    /// the original vector unchanged to avoid division by zero or precision issues.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::math::Vec3;
    ///
    /// let v = Vec3::new(0.0, 3.0, 4.0); // Length is 5
    /// let n = v.normalize();
    /// assert_eq!(n, Vec3::new(0.0, 0.6, 0.8));
    ///
    /// // Small vector behavior
    /// let tiny = Vec3::new(0.00001, 0.0, 0.0);
    /// assert_eq!(tiny.normalize(), tiny);
    /// ```
    #[must_use]
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 0.0001 {
            let inv_len = 1.0 / len;
            Self {
                x: self.x * inv_len,
                y: self.y * inv_len,
                z: self.z * inv_len,
            }
        } else {
            *self
        }
    }
}

impl Add for Vec3 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

impl Mul for Vec3 {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        Self {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z,
        }
    }
}

/// A 4x4 transformation matrix used for 3D graphics.
///
/// stored in **Row-Major** order.
#[derive(Debug, Clone, Copy)]
pub struct Mat4 {
    pub m: [[f32; 4]; 4],
}

impl Mat4 {
    /// Returns the identity matrix.
    #[must_use]
    pub const fn identity() -> Self {
        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Creates a translation matrix.
    ///
    /// # Arguments
    ///
    /// * `x` - Translation along the X axis.
    /// * `y` - Translation along the Y axis.
    /// * `z` - Translation along the Z axis.
    #[must_use]
    pub const fn translation(x: f32, y: f32, z: f32) -> Self {
        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [x, y, z, 1.0],
            ],
        }
    }

    /// Creates a scaling matrix.
    #[must_use]
    pub const fn scale(x: f32, y: f32, z: f32) -> Self {
        Self {
            m: [
                [x, 0.0, 0.0, 0.0],
                [0.0, y, 0.0, 0.0],
                [0.0, 0.0, z, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Creates a rotation matrix around the X axis.
    ///
    /// * `angle` - The angle in radians.
    #[must_use]
    pub fn rotation_x(angle: f32) -> Self {
        let c = angle.cos();
        let s = angle.sin();
        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, c, s, 0.0],
                [0.0, -s, c, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Creates a rotation matrix around the Y axis.
    ///
    /// * `angle` - The angle in radians.
    #[must_use]
    pub fn rotation_y(angle: f32) -> Self {
        let c = angle.cos();
        let s = angle.sin();
        Self {
            m: [
                [c, 0.0, -s, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [s, 0.0, c, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Creates a rotation matrix around the Z axis.
    ///
    /// * `angle` - The angle in radians.
    #[must_use]
    pub fn rotation_z(angle: f32) -> Self {
        let c = angle.cos();
        let s = angle.sin();
        Self {
            m: [
                [c, s, 0.0, 0.0],
                [-s, c, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Creates a perspective projection matrix.
    ///
    /// # Arguments
    ///
    /// * `fov` - Vertical field of view in radians.
    /// * `aspect` - Aspect ratio (width / height).
    /// * `near` - Distance to near clipping plane.
    /// * `far` - Distance to far clipping plane.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::math::Mat4;
    /// use std::f32::consts::PI;
    ///
    /// let proj = Mat4::perspective(PI / 4.0, 1.33, 0.1, 100.0);
    /// ```
    #[must_use]
    pub fn perspective(fov: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (fov / 2.0).tan();
        let nf = 1.0 / (near - far);
        Self {
            m: [
                [f / aspect, 0.0, 0.0, 0.0],
                [0.0, f, 0.0, 0.0],
                [0.0, 0.0, (far + near) * nf, -1.0],
                [0.0, 0.0, 2.0 * far * near * nf, 0.0],
            ],
        }
    }

    /// Creates a View matrix (`LookAt`) for a camera.
    ///
    /// * `eye` - Position of the camera.
    /// * `target` - Point the camera is looking at.
    /// * `up` - The "up" direction in the world (usually Y-up).
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::math::{Mat4, Vec3};
    ///
    /// let eye = Vec3::new(0.0, 0.0, 5.0);
    /// let target = Vec3::new(0.0, 0.0, 0.0);
    /// let up = Vec3::new(0.0, 1.0, 0.0);
    /// let view = Mat4::look_at(eye, target, up);
    /// ```
    #[must_use]
    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let f = (target - eye).normalize();
        let s = f.cross(up).normalize();
        let u = s.cross(f);
        Self {
            m: [
                [s.x, u.x, -f.x, 0.0],
                [s.y, u.y, -f.y, 0.0],
                [s.z, u.z, -f.z, 0.0],
                [-s.dot(eye), -u.dot(eye), f.dot(eye), 1.0],
            ],
        }
    }

    /// Transforms a point by this matrix.
    ///
    /// Returns a tuple `(transformed_point, w_component)`.
    /// The `w` component is used for perspective division.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::math::{Mat4, Vec3};
    ///
    /// let m = Mat4::translation(1.0, 2.0, 3.0);
    /// let p = Vec3::new(0.0, 0.0, 0.0);
    /// let (p_prime, w) = m.transform_point(p);
    ///
    /// assert_eq!(p_prime, Vec3::new(1.0, 2.0, 3.0));
    /// assert_eq!(w, 1.0);
    /// ```
    #[must_use]
    pub fn transform_point(&self, v: Vec3) -> (Vec3, f32) {
        let x = self.m[0][0] * v.x + self.m[1][0] * v.y + self.m[2][0] * v.z + self.m[3][0];
        let y = self.m[0][1] * v.x + self.m[1][1] * v.y + self.m[2][1] * v.z + self.m[3][1];
        let z = self.m[0][2] * v.x + self.m[1][2] * v.y + self.m[2][2] * v.z + self.m[3][2];
        let w = self.m[0][3] * v.x + self.m[1][3] * v.y + self.m[2][3] * v.z + self.m[3][3];
        (Vec3::new(x, y, z), w)
    }

    /// Transforms multiple points by this matrix.
    ///
    /// Output buffer must have same length as input points.
    /// Returns `(transformed_point, w_component)` for each point.
    ///
    /// # Panics
    ///
    /// Panics if `points` and `output` have different lengths.
    pub fn transform_points(&self, points: &[Vec3], output: &mut [(Vec3, f32)]) {
        assert_eq!(points.len(), output.len());
        for (p, out) in points.iter().zip(output.iter_mut()) {
            *out = self.transform_point(*p);
        }
    }

    /// Transform a normal vector (ignores translation, uses upper-left 3x3).
    ///
    /// This is essential for correct lighting calculations after transformation.
    #[must_use]
    pub fn transform_normal(&self, n: Vec3) -> Vec3 {
        let x = self.m[0][0] * n.x + self.m[1][0] * n.y + self.m[2][0] * n.z;
        let y = self.m[0][1] * n.x + self.m[1][1] * n.y + self.m[2][1] * n.z;
        let z = self.m[0][2] * n.x + self.m[1][2] * n.y + self.m[2][2] * n.z;
        Vec3::new(x, y, z).normalize()
    }
}

impl Default for Mat4 {
    fn default() -> Self {
        Self::identity()
    }
}

impl Mul for Mat4 {
    type Output = Self;

    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    #[inline]
    fn mul(self, other: Self) -> Self {
        unsafe {
            use std::arch::x86_64::{
                _mm_add_ps, _mm_loadu_ps, _mm_mul_ps, _mm_shuffle_ps, _mm_storeu_ps,
            };
            let mut result = Self { m: [[0.0; 4]; 4] };

            // Load rows of B
            let b0 = _mm_loadu_ps(other.m[0].as_ptr());
            let b1 = _mm_loadu_ps(other.m[1].as_ptr());
            let b2 = _mm_loadu_ps(other.m[2].as_ptr());
            let b3 = _mm_loadu_ps(other.m[3].as_ptr());

            for i in 0..4 {
                // Load row i of A
                let row_a = _mm_loadu_ps(self.m[i].as_ptr());

                // Broadcast A[i][0]
                let a0 = _mm_shuffle_ps(row_a, row_a, 0x00);
                let mut row_res = _mm_mul_ps(a0, b0);

                // Broadcast A[i][1]
                let a1 = _mm_shuffle_ps(row_a, row_a, 0x55);
                row_res = _mm_add_ps(row_res, _mm_mul_ps(a1, b1));

                // Broadcast A[i][2]
                let a2 = _mm_shuffle_ps(row_a, row_a, 0xAA);
                row_res = _mm_add_ps(row_res, _mm_mul_ps(a2, b2));

                // Broadcast A[i][3]
                let a3 = _mm_shuffle_ps(row_a, row_a, 0xFF);
                row_res = _mm_add_ps(row_res, _mm_mul_ps(a3, b3));

                _mm_storeu_ps(result.m[i].as_mut_ptr(), row_res);
            }
            result
        }
    }

    #[cfg(not(all(target_arch = "x86_64", feature = "simd")))]
    fn mul(self, other: Self) -> Self {
        let mut result = Self { m: [[0.0; 4]; 4] };
        for i in 0..4 {
            for k in 0..4 {
                let s = self.m[i][k];
                for j in 0..4 {
                    result.m[i][j] += s * other.m[k][j];
                }
            }
        }
        result
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenPoint {
    pub x: i32,
    pub y: i32,
    pub z: f32,
}

/// Project a 3D point to screen coordinates using pre-calculated half-dimensions.
///
/// This avoids repetitive integer-to-float conversions and divisions.
#[must_use]
#[inline]
pub fn project_to_screen_optimized(
    v: Vec3,
    w: f32,
    half_width: f32,
    half_height: f32,
) -> ScreenPoint {
    // Perspective divide
    let inv_w = if w.abs() > 0.0001 { 1.0 / w } else { 1.0 };
    let ndc_x = v.x * inv_w;
    let ndc_y = v.y * inv_w;
    let depth = v.z * inv_w;

    // NDC to screen coordinates
    let screen_x = ((ndc_x + 1.0) * half_width) as i32;
    let screen_y = ((1.0 - ndc_y) * half_height) as i32; // Flip Y

    ScreenPoint {
        x: screen_x,
        y: screen_y,
        z: depth,
    }
}

/// Project a 3D point to screen coordinates
#[must_use]
#[inline]
pub fn project_to_screen(v: Vec3, w: f32, width: u32, height: u32) -> ScreenPoint {
    let half_width = width as f32 * 0.5;
    let half_height = height as f32 * 0.5;
    project_to_screen_optimized(v, w, half_width, half_height)
}
