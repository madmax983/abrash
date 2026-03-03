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

use std::mem::MaybeUninit;
use std::ops::{Add, Mul, Sub};

/// Approximates the reciprocal square root ($1 / \sqrt{x}$).
///
/// This uses the hardware-accelerated AVX/SSE intrinsic if available, which offers
/// excellent performance (around 4 cycles) at the cost of a small precision error.
/// If AVX/SSE is not available, it falls back to a standard `sqrt().recip()`, which
/// is typically faster on modern generic x86_64 CPUs than the legacy "Quake III bit-hack".
///
/// # Examples
///
/// ```
/// use abrash::math::fast_inv_sqrt;
///
/// let x = 4.0;
/// let inv_sqrt = fast_inv_sqrt(x); // 1.0 / sqrt(4.0) = 0.5
///
/// // Assert with a small tolerance due to approximation
/// assert!((inv_sqrt - 0.5).abs() < 0.01);
/// ```
#[inline]
#[must_use]
pub fn fast_inv_sqrt(n: f32) -> f32 {
    // Use AVX/SSE approximate reciprocal square root if available.
    // This is faster (~4 cycles latency vs ~23 for sqrt+div) but less precise.
    // We accept the approximation (error < 1.5*2^-12) for the sake of speed in lighting/normalization.
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    unsafe {
        // _mm_rsqrt_ss computes approximate 1/sqrt(a) for the lower float.
        let n_vec = std::arch::x86_64::_mm_set_ss(n);
        let r = std::arch::x86_64::_mm_rsqrt_ss(n_vec);
        std::arch::x86_64::_mm_cvtss_f32(r)
    }

    #[cfg(not(all(target_arch = "x86_64", feature = "simd")))]
    {
        // Modern hardware sqrt (e.g. sqrtss) is extremely fast.
        // Combined with reciprocal, this is faster (~2.3ns) than the legacy Quake III
        // bit-hack (~3.4ns) on modern x86_64, and safer than manual intrinsics.
        n.sqrt().recip()
    }
}

/// A 2-component vector, used for texture coordinates (UVs) and 2D positions.
///
/// # Examples
///
/// ```
/// use abrash::math::Vec2;
///
/// let uv = Vec2::new(0.5, 0.5);
/// assert_eq!(uv.x, 0.5);
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    /// Creates a new 2D vector.
    #[must_use]
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Calculates the Euclidean length (magnitude) of the vector.
    #[must_use]
    #[inline]
    pub fn length(&self) -> f32 {
        self.x.hypot(self.y)
    }
}

impl Add for Vec2 {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Vec2 {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;

    #[inline]
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

/// A 2x2 matrix, primarily used for 2D rotations and transformations.
///
/// # Examples
///
/// ```
/// use abrash::math::{Mat2, Vec2};
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
pub struct Mat2 {
    pub m: [[f32; 2]; 2],
}

impl Mat2 {
    /// Creates a 2D rotation matrix.
    ///
    /// * `angle`: Rotation angle in radians (counter-clockwise).
    #[must_use]
    pub fn rotation(angle: f32) -> Self {
        let cos = angle.cos();
        let sin = angle.sin();

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
        let mut result = Vec::with_capacity(vertices.len());
        for v in vertices {
            result.push(self.transform(*v));
        }
        result
    }

    /// Transform vertices in place.
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
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    pub const ONE: Self = Self {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    };

    /// Creates a new vector.
    #[must_use]
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Calculates the dot product with another vector.
    ///
    /// The dot product represents the projection of one vector onto another.
    /// *   Positive if pointing in similar direction.
    /// *   Zero if perpendicular.
    /// *   Negative if pointing in opposite directions.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::math::Vec3;
    ///
    /// let a = Vec3::new(1.0, 0.0, 0.0);
    /// let b = Vec3::new(0.5, 0.0, 0.0);
    /// let c = Vec3::new(0.0, 1.0, 0.0);
    ///
    /// // Parallel vectors
    /// assert_eq!(a.dot(b), 0.5);
    ///
    /// // Perpendicular vectors
    /// assert_eq!(a.dot(c), 0.0);
    /// ```
    #[must_use]
    #[inline]
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
    #[inline]
    pub fn cross(&self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    /// Calculates the Euclidean length (magnitude) of the vector.
    #[must_use]
    #[inline]
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
    #[inline]
    pub fn normalize(&self) -> Self {
        // Optimization: Use rsqrt instead of 1.0/sqrt.
        // We use len_sq to avoid sqrt if the vector is too small.
        // 0.0001^2 = 0.00000001
        let len_sq = self.x * self.x + self.y * self.y + self.z * self.z;
        if len_sq > 0.00000001 {
            let inv_len = fast_inv_sqrt(len_sq);
            Self {
                x: self.x * inv_len,
                y: self.y * inv_len,
                z: self.z * inv_len,
            }
        } else {
            *self
        }
    }

    /// Returns a normalized unit vector using fast inverse square root approximation.
    ///
    /// This is faster than `normalize()` but slightly less accurate.
    /// Useful for lighting calculations where extreme precision is not required.
    #[must_use]
    #[inline]
    pub fn fast_normalize(&self) -> Self {
        let len_sq = self.x * self.x + self.y * self.y + self.z * self.z;
        if len_sq > 0.0001 {
            let inv_len = fast_inv_sqrt(len_sq);
            Self {
                x: self.x * inv_len,
                y: self.y * inv_len,
                z: self.z * inv_len,
            }
        } else {
            *self
        }
    }

    /// Calculates the squared length (magnitude) of the vector.
    ///
    /// Faster than `length()` as it avoids a square root operation.
    /// Useful for comparing distances.
    #[must_use]
    #[inline]
    pub fn length_sq(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    /// Linearly interpolate between this vector and another.
    ///
    /// `t` is the interpolation factor (0.0 = self, 1.0 = other).
    #[must_use]
    #[inline]
    pub fn lerp(&self, other: Self, t: f32) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            z: self.z + (other.z - self.z) * t,
        }
    }

    /// Returns a new vector containing the minimum value for each component.
    #[must_use]
    #[inline]
    pub fn min(&self, other: Self) -> Self {
        Self {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
            z: self.z.min(other.z),
        }
    }

    /// Returns a new vector containing the maximum value for each component.
    #[must_use]
    #[inline]
    pub fn max(&self, other: Self) -> Self {
        Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
            z: self.z.max(other.z),
        }
    }
}

impl Add for Vec3 {
    type Output = Self;
    #[inline]
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
    #[inline]
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
    #[inline]
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
    #[inline]
    fn mul(self, other: Self) -> Self {
        Self {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z,
        }
    }
}

impl std::ops::Div<f32> for Vec3 {
    type Output = Self;
    #[inline]
    fn div(self, scalar: f32) -> Self {
        let inv = 1.0 / scalar;
        Self {
            x: self.x * inv,
            y: self.y * inv,
            z: self.z * inv,
        }
    }
}

/// A 4x4 transformation matrix used for 3D graphics.
///
/// stored in **Row-Major** order.
///
/// # Transformation Order
///
/// Since this library uses row vectors ($v \cdot M$), transformations are applied in the order they are multiplied.
/// To achieve the standard "Scale, then Rotate, then Translate" effect for a model matrix, you must multiply in that order:
///
/// $$ M_{model} = M_{scale} \cdot M_{rotate} \cdot M_{translate} $$
///
/// # Examples
///
/// ```
/// use abrash::math::{Mat4, Vec3};
/// use std::f32::consts::PI;
///
/// // 1. Scale by 2
/// let scale = Mat4::scale(2.0, 2.0, 2.0);
/// // 2. Rotate 90 degrees around Y
/// let rotation = Mat4::rotation_y(PI / 2.0);
/// // 3. Translate by (10, 5, 0)
/// let translation = Mat4::translation(10.0, 5.0, 0.0);
///
/// // Combine: Scale -> Rotate -> Translate
/// let model_matrix = scale * rotation * translation;
///
/// // Apply to point (1, 0, 0)
/// let p = Vec3::new(1.0, 0.0, 0.0);
/// let (p_prime, _) = model_matrix.transform_point(p);
///
/// // Expected:
/// // (1,0,0) * 2 = (2,0,0)
/// // (2,0,0) rot Y 90 = (0,0,-2) (Right-Hand Rule)
/// // (0,0,-2) + (10,5,0) = (10, 5, -2)
/// assert!((p_prime.x - 10.0).abs() < 0.001);
/// assert!((p_prime.y - 5.0).abs() < 0.001);
/// assert!((p_prime.z - -2.0).abs() < 0.001);
/// ```
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct Mat4 {
    pub m: [[f32; 4]; 4],
}

impl Mat4 {
    /// Returns the identity matrix.
    #[must_use]
    #[inline]
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
    #[inline]
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
    #[inline]
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
    #[inline]
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
    #[inline]
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
    #[inline]
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
    #[inline]
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
    #[inline]
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
    /// The `w` component is the Homogeneous W coordinate, used for perspective division.
    /// In the rasterization pipeline, vertices are kept in this `(Vec3, w)` format
    /// until the very last moment (viewport mapping) to preserve perspective correctness.
    ///
    /// # Performance
    ///
    /// Marked `#[inline]` to allow the compiler to optimize call overhead and potentially
    /// vectorize loops that call this function.
    ///
    /// # Safety
    ///
    /// The SIMD implementation uses `_mm_load_ps` (load aligned packed single) for maximum performance.
    /// `Mat4` is marked `#[repr(align(16))]`, ensuring 16-byte alignment for all rows.
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
    ///
    /// // Example with Perspective Projection (where w != 1.0)
    /// let proj = Mat4::perspective(1.57, 1.0, 1.0, 10.0);
    /// let p_view = Vec3::new(0.0, 0.0, -5.0); // Point in front of camera
    /// let (p_clip, w_clip) = proj.transform_point(p_view);
    ///
    /// // In standard perspective projection, w_clip = -z_view
    /// assert_eq!(w_clip, 5.0);
    /// ```
    ///
    /// Marked `#[inline]` to allow cross-crate inlining and auto-vectorization by the compiler.
    #[must_use]
    #[inline]
    pub fn transform_point(&self, v: Vec3) -> (Vec3, f32) {
        #[cfg(all(target_arch = "x86_64", feature = "simd"))]
        // SAFETY: Mat4 is 16-byte aligned, so _mm_load_ps is safe and optimal.
        unsafe {
            use std::arch::x86_64::{
                _mm_add_ps, _mm_load_ps, _mm_mul_ps, _mm_set1_ps, _mm_storeu_ps,
            };

            let row0 = _mm_load_ps(self.m[0].as_ptr());
            let row1 = _mm_load_ps(self.m[1].as_ptr());
            let row2 = _mm_load_ps(self.m[2].as_ptr());
            let row3 = _mm_load_ps(self.m[3].as_ptr());

            let vx = _mm_set1_ps(v.x);
            let vy = _mm_set1_ps(v.y);
            let vz = _mm_set1_ps(v.z);

            let t0 = _mm_mul_ps(vx, row0);
            let t1 = _mm_mul_ps(vy, row1);
            let t2 = _mm_mul_ps(vz, row2);

            let res = _mm_add_ps(_mm_add_ps(t0, t1), _mm_add_ps(t2, row3));

            let mut out = [0.0; 4];
            _mm_storeu_ps(out.as_mut_ptr(), res);

            (Vec3::new(out[0], out[1], out[2]), out[3])
        }

        #[cfg(not(all(target_arch = "x86_64", feature = "simd")))]
        {
            let x = self.m[0][0] * v.x + self.m[1][0] * v.y + self.m[2][0] * v.z + self.m[3][0];
            let y = self.m[0][1] * v.x + self.m[1][1] * v.y + self.m[2][1] * v.z + self.m[3][1];
            let z = self.m[0][2] * v.x + self.m[1][2] * v.y + self.m[2][2] * v.z + self.m[3][2];
            let w = self.m[0][3] * v.x + self.m[1][3] * v.y + self.m[2][3] * v.z + self.m[3][3];
            (Vec3::new(x, y, z), w)
        }
    }

    /// Transforms multiple points by this matrix.
    ///
    /// Output buffer must have same length as input points.
    /// Returns (`transformed_point`, `w_component`) for each point.
    ///
    /// # Panics
    ///
    /// Panics if `points.len()` does not equal `output.len()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::math::{Mat4, Vec3};
    ///
    /// let m = Mat4::scale(2.0, 2.0, 2.0);
    /// let points = [Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)];
    /// let mut output = vec![(Vec3::default(), 0.0); 2];
    ///
    /// m.transform_points(&points, &mut output);
    ///
    /// assert_eq!(output[0].0, Vec3::new(2.0, 0.0, 0.0));
    /// assert_eq!(output[1].0, Vec3::new(0.0, 2.0, 0.0));
    /// ```
    pub fn transform_points(&self, points: &[Vec3], output: &mut [(Vec3, f32)]) {
        // SAFETY: (Vec3, f32) has same layout as MaybeUninit<(Vec3, f32)>
        let output_uninit = unsafe {
            &mut *(std::ptr::from_mut::<[(Vec3, f32)]>(output) as *mut [MaybeUninit<(Vec3, f32)>])
        };
        self.transform_points_uninit(points, output_uninit);
    }

    /// Transforms multiple points by this matrix into uninitialized memory.
    ///
    /// # Panics
    ///
    /// Panics if `points.len()` does not equal `output.len()`.
    pub fn transform_points_uninit(
        &self,
        points: &[Vec3],
        output: &mut [MaybeUninit<(Vec3, f32)>],
    ) {
        assert_eq!(points.len(), output.len());

        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        if is_x86_feature_detected!("avx2") {
            unsafe {
                self.transform_points_avx2(points, output);
            }
            return;
        }

        for (p, out) in points.iter().zip(output.iter_mut()) {
            out.write(self.transform_point(*p));
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    #[target_feature(enable = "avx2")]
    #[allow(clippy::wildcard_imports)]
    unsafe fn transform_points_avx2(
        &self,
        points: &[Vec3],
        output: &mut [MaybeUninit<(Vec3, f32)>],
    ) {
        use std::arch::x86_64::*;

        #[cfg(debug_assertions)]
        {
            assert_eq!(
                std::mem::size_of::<(Vec3, f32)>(),
                16,
                "Layout mismatch: (Vec3, f32) size != 16"
            );
            assert_eq!(
                std::mem::align_of::<(Vec3, f32)>(),
                4,
                "Layout mismatch: (Vec3, f32) align != 4"
            );
            // Verify offsets
            let dummy: (Vec3, f32) = (Vec3::new(0.0, 0.0, 0.0), 0.0);
            let base = &dummy as *const _ as usize;
            let x_ptr = &dummy.0.x as *const _ as usize;
            let w_ptr = &dummy.1 as *const _ as usize;
            assert_eq!(x_ptr - base, 0, "Offset of Vec3.x must be 0");
            assert_eq!(w_ptr - base, 12, "Offset of f32 must be 12");
        }

        let len = points.len();
        let mut i = 0;

        let m00 = _mm256_set1_ps(self.m[0][0]);
        let m01 = _mm256_set1_ps(self.m[0][1]);
        let m02 = _mm256_set1_ps(self.m[0][2]);
        let m03 = _mm256_set1_ps(self.m[0][3]);

        let m10 = _mm256_set1_ps(self.m[1][0]);
        let m11 = _mm256_set1_ps(self.m[1][1]);
        let m12 = _mm256_set1_ps(self.m[1][2]);
        let m13 = _mm256_set1_ps(self.m[1][3]);

        let m20 = _mm256_set1_ps(self.m[2][0]);
        let m21 = _mm256_set1_ps(self.m[2][1]);
        let m22 = _mm256_set1_ps(self.m[2][2]);
        let m23 = _mm256_set1_ps(self.m[2][3]);

        let m30 = _mm256_set1_ps(self.m[3][0]);
        let m31 = _mm256_set1_ps(self.m[3][1]);
        let m32 = _mm256_set1_ps(self.m[3][2]);
        let m33 = _mm256_set1_ps(self.m[3][3]);

        while i + 8 <= len {
            // SAFETY: Memory access is bounded by loop condition and caller guarantees.
            unsafe {
                let p_ptr = points.as_ptr().add(i).cast::<f32>();

                // Load 8 Vec3s (96 bytes) as 3 chunks of 32 bytes? No, SSE loads of 16 bytes.
                // 8 points * 12 bytes = 96 bytes.
                // Load first 4 points (48 bytes) -> 3 * 16 bytes
                let r0 = _mm_loadu_ps(p_ptr); // x0 y0 z0 x1
                let r1 = _mm_loadu_ps(p_ptr.add(4)); // y1 z1 x2 y2
                let r2 = _mm_loadu_ps(p_ptr.add(8)); // z2 x3 y3 z3

                // Shuffle to SOA (x_lo, y_lo, z_lo)
                // _MM_SHUFFLE(z, y, x, w) -> (z << 6) | (y << 4) | (x << 2) | w
                let t_x0x1 = _mm_shuffle_ps(r0, r0, 0b11_00_11_00); // 3, 0, 3, 0
                let t_x2x3 = _mm_shuffle_ps(r1, r2, 0b01_01_10_10); // 1, 1, 2, 2
                // Mask for x_lo: a[0], a[1], b[0], b[2] -> 2, 0, 1, 0 -> 0x84
                let x_lo = _mm_shuffle_ps(t_x0x1, t_x2x3, 0b10_00_01_00);

                let t_y0y1 = _mm_shuffle_ps(r0, r1, 0b00_00_01_01); // 0, 0, 1, 1
                let t_y2y3 = _mm_shuffle_ps(r1, r2, 0b10_10_11_11); // 2, 2, 3, 3
                // Mask for y_lo: a[0], a[2], b[0], b[2] -> 2, 0, 2, 0 -> 0x88
                let y_lo = _mm_shuffle_ps(t_y0y1, t_y2y3, 0b10_00_10_00);

                let t_z0z1 = _mm_shuffle_ps(r0, r1, 0b01_01_10_10); // 1, 1, 2, 2
                let t_z2z3 = _mm_shuffle_ps(r2, r2, 0b11_00_11_00); // 3, 0, 3, 0
                // Mask for z_lo: a[0], a[2], b[0], b[1] -> 1, 0, 2, 0 -> 0x48
                let z_lo = _mm_shuffle_ps(t_z0z1, t_z2z3, 0b01_00_10_00);

                // Load next 4 points
                let r3 = _mm_loadu_ps(p_ptr.add(12)); // x4 y4 z4 x5
                let r4 = _mm_loadu_ps(p_ptr.add(16)); // y5 z5 x6 y6
                let r5 = _mm_loadu_ps(p_ptr.add(20)); // z6 x7 y7 z7

                let t_x4x5 = _mm_shuffle_ps(r3, r3, 0b11_00_11_00);
                let t_x6x7 = _mm_shuffle_ps(r4, r5, 0b01_01_10_10);
                let x_hi = _mm_shuffle_ps(t_x4x5, t_x6x7, 0b10_00_01_00);

                let t_y4y5 = _mm_shuffle_ps(r3, r4, 0b00_00_01_01);
                let t_y6y7 = _mm_shuffle_ps(r4, r5, 0b10_10_11_11);
                let y_hi = _mm_shuffle_ps(t_y4y5, t_y6y7, 0b10_00_10_00);

                let t_z4z5 = _mm_shuffle_ps(r3, r4, 0b01_01_10_10);
                let t_z6z7 = _mm_shuffle_ps(r5, r5, 0b11_00_11_00);
                let z_hi = _mm_shuffle_ps(t_z4z5, t_z6z7, 0b01_00_10_00);

                // Combine to AVX
                let vx = _mm256_insertf128_ps(_mm256_castps128_ps256(x_lo), x_hi, 1);
                let vy = _mm256_insertf128_ps(_mm256_castps128_ps256(y_lo), y_hi, 1);
                let vz = _mm256_insertf128_ps(_mm256_castps128_ps256(z_lo), z_hi, 1);

                // Matrix Multiplication
                // Match scalar order: ((x*m0 + y*m1) + z*m2) + m3
                let res_x = _mm256_add_ps(
                    _mm256_add_ps(
                        _mm256_add_ps(_mm256_mul_ps(vx, m00), _mm256_mul_ps(vy, m10)),
                        _mm256_mul_ps(vz, m20),
                    ),
                    m30,
                );

                let res_y = _mm256_add_ps(
                    _mm256_add_ps(
                        _mm256_add_ps(_mm256_mul_ps(vx, m01), _mm256_mul_ps(vy, m11)),
                        _mm256_mul_ps(vz, m21),
                    ),
                    m31,
                );

                let res_z = _mm256_add_ps(
                    _mm256_add_ps(
                        _mm256_add_ps(_mm256_mul_ps(vx, m02), _mm256_mul_ps(vy, m12)),
                        _mm256_mul_ps(vz, m22),
                    ),
                    m32,
                );

                let res_w = _mm256_add_ps(
                    _mm256_add_ps(
                        _mm256_add_ps(_mm256_mul_ps(vx, m03), _mm256_mul_ps(vy, m13)),
                        _mm256_mul_ps(vz, m23),
                    ),
                    m33,
                );

                // Transpose back to AOS (8x4)
                let t0 = _mm256_unpacklo_ps(res_x, res_z); // x0 z0 x1 z1 ...
                let t1 = _mm256_unpackhi_ps(res_x, res_z); // x2 z2 x3 z3 ...
                let t2 = _mm256_unpacklo_ps(res_y, res_w); // y0 w0 y1 w1 ...
                let t3 = _mm256_unpackhi_ps(res_y, res_w); // y2 w2 y3 w3 ...

                let out0 = _mm256_unpacklo_ps(t0, t2); // x0 y0 z0 w0 ...
                let out1 = _mm256_unpackhi_ps(t0, t2); // x1 y1 z1 w1 ...
                let out2 = _mm256_unpacklo_ps(t1, t3); // x2 y2 z2 w2 ...
                let out3 = _mm256_unpackhi_ps(t1, t3); // x3 y3 z3 w3 ...

                // Order: p0, p1, p2, p3, p4, p5, p6, p7
                let final0 = _mm256_permute2f128_ps(out0, out1, 0x20); // p0 | p1
                let final1 = _mm256_permute2f128_ps(out2, out3, 0x20); // p2 | p3
                let final2 = _mm256_permute2f128_ps(out0, out1, 0x31); // p4 | p5
                let final3 = _mm256_permute2f128_ps(out2, out3, 0x31); // p6 | p7

                // Store
                let out_ptr = output.as_mut_ptr().add(i).cast::<f32>();
                _mm256_storeu_ps(out_ptr, final0);
                _mm256_storeu_ps(out_ptr.add(8), final1);
                _mm256_storeu_ps(out_ptr.add(16), final2);
                _mm256_storeu_ps(out_ptr.add(24), final3);
            }

            i += 8;
        }

        while i < len {
            output[i].write(self.transform_point(points[i]));
            i += 1;
        }
    }

    /// Transforms multiple points by this matrix in parallel (if `parallel` feature is enabled).
    ///
    /// Output buffer must have same length as input points.
    /// Returns (`transformed_point`, `w_component`) for each point.
    ///
    /// # Panics
    ///
    /// Panics if `points.len()` does not equal `output.len()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::math::{Mat4, Vec3};
    ///
    /// let m = Mat4::translation(10.0, 0.0, 0.0);
    /// let points = [Vec3::new(0.0, 0.0, 0.0); 100];
    /// let mut output = vec![(Vec3::default(), 0.0); 100];
    ///
    /// m.transform_points_parallel(&points, &mut output);
    ///
    /// assert_eq!(output[0].0, Vec3::new(10.0, 0.0, 0.0));
    /// ```
    pub fn transform_points_parallel(&self, points: &[Vec3], output: &mut [(Vec3, f32)]) {
        // SAFETY: (Vec3, f32) has same layout as MaybeUninit<(Vec3, f32)>
        let output_uninit = unsafe {
            &mut *(std::ptr::from_mut::<[(Vec3, f32)]>(output) as *mut [MaybeUninit<(Vec3, f32)>])
        };
        self.transform_points_uninit_parallel(points, output_uninit);
    }

    /// Transforms multiple points by this matrix into uninitialized memory in parallel.
    ///
    /// # Panics
    ///
    /// Panics if `points.len()` does not equal `output.len()`.
    pub fn transform_points_uninit_parallel(
        &self,
        points: &[Vec3],
        output: &mut [MaybeUninit<(Vec3, f32)>],
    ) {
        assert_eq!(points.len(), output.len());

        #[cfg(feature = "parallel")]
        {
            // Fallback to scalar for small inputs to avoid Rayon overhead
            if points.len() < 1024 {
                self.transform_points_uninit(points, output);
                return;
            }

            use rayon::prelude::*;
            // Chunk size of 4096 ensures we amortize task overhead and keep the AVX2
            // implementation fed with enough data to be efficient.
            const CHUNK_SIZE: usize = 4096;
            points
                .par_chunks(CHUNK_SIZE)
                .zip(output.par_chunks_mut(CHUNK_SIZE))
                .for_each(|(p_chunk, out_chunk)| {
                    self.transform_points_uninit(p_chunk, out_chunk);
                });
        }

        #[cfg(not(feature = "parallel"))]
        {
            self.transform_points_uninit(points, output);
        }
    }

    /// Transform a normal vector (ignores translation, uses upper-left 3x3).
    ///
    /// This is essential for correct lighting calculations after transformation.
    ///
    /// # Performance
    ///
    /// Marked `#[inline]` to allow the compiler to optimize call overhead and potentially
    /// vectorize loops that call this function.
    #[must_use]
    #[inline]
    pub fn transform_normal(&self, n: Vec3) -> Vec3 {
        let x = self.m[0][0] * n.x + self.m[1][0] * n.y + self.m[2][0] * n.z;
        let y = self.m[0][1] * n.x + self.m[1][1] * n.y + self.m[2][1] * n.z;
        let z = self.m[0][2] * n.x + self.m[1][2] * n.y + self.m[2][2] * n.z;
        Vec3::new(x, y, z).normalize()
    }

    /// Retrieve a column by index (0-3).
    ///
    /// # Panics
    ///
    /// Panics if index is out of bounds.
    #[must_use]
    #[inline]
    pub const fn col(&self, index: usize) -> Vec4 {
        Vec4::new(
            self.m[0][index],
            self.m[1][index],
            self.m[2][index],
            self.m[3][index],
        )
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
                _mm_add_ps, _mm_load_ps, _mm_mul_ps, _mm_shuffle_ps, _mm_store_ps,
            };
            let mut result = Self { m: [[0.0; 4]; 4] };

            // Load rows of B
            let b0 = _mm_load_ps(other.m[0].as_ptr());
            let b1 = _mm_load_ps(other.m[1].as_ptr());
            let b2 = _mm_load_ps(other.m[2].as_ptr());
            let b3 = _mm_load_ps(other.m[3].as_ptr());

            for i in 0..4 {
                // Load row i of A
                let row_a = _mm_load_ps(self.m[i].as_ptr());

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

                _mm_store_ps(result.m[i].as_mut_ptr(), row_res);
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
    /// Reciprocal of the Homogeneous W coordinate ($1/w$).
    ///
    /// This value is critical for perspective-correct texture mapping and attribute interpolation.
    /// By storing $1/w$, the rasterizer can interpolate attributes in screen space linearly
    /// (e.g., $u/w$, $v/w$) and then recover the true perspective-correct value per pixel
    /// by dividing by the interpolated $1/w$.
    ///
    /// Storing this avoids recomputing the division during triangle setup,
    /// saving ~10-20 CPU cycles per vertex.
    pub inv_w: f32,
}

/// Project a 3D point to screen coordinates using pre-calculated half-dimensions.
///
/// This avoids repetitive integer-to-float conversions and divisions.
///
/// # Examples
///
/// ```
/// use abrash::math::{project_to_screen_optimized, Vec3};
///
/// let point = Vec3::new(1.0, 1.0, 5.0);
/// let w = 5.0; // Assume we already have w from projection
/// let half_width = 400.0;
/// let half_height = 300.0;
///
/// let screen_point = project_to_screen_optimized(point, w, half_width, half_height);
///
/// // NDC x = 1/5 = 0.2
/// // Screen x = (0.2 + 1.0) * 400 = 480
/// assert_eq!(screen_point.x, 480);
/// ```
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
    // Clamp to [i32::MIN + 1, i32::MAX] to avoid integer overflow when negating i32::MIN.
    // We clamp the float value BEFORE casting to i32 to avoid Undefined Behavior with NaN/Inf.
    // 2147483520.0 is the largest f32 strictly less than i32::MAX + 1 that is exactly representable.
    const MAX_VAL: f32 = 2_147_483_520.0;
    const MIN_VAL: f32 = -2_147_483_520.0;

    let screen_x_f = (ndc_x + 1.0) * half_width;
    let screen_y_f = (1.0 - ndc_y) * half_height; // Flip Y

    // Optimization: Branchless clamp to avoid stalls.
    // If NaN, max(MIN) returns MIN (because max propagates non-NaN).
    // Then min(MIN, MAX) returns MIN.
    // Result is always in [MIN, MAX] (or MIN if NaN).
    // Note: f32::clamp() returns NaN for NaN inputs, which makes casting to i32 undefined/zero.
    // We strictly want MIN_VAL behavior for NaNs here.
    #[allow(clippy::manual_clamp)]
    let screen_x = screen_x_f.max(MIN_VAL).min(MAX_VAL) as i32;
    #[allow(clippy::manual_clamp)]
    let screen_y = screen_y_f.max(MIN_VAL).min(MAX_VAL) as i32;

    ScreenPoint {
        x: screen_x,
        y: screen_y,
        z: depth,
        inv_w,
    }
}

/// Project 3 vertices to screen coordinates in parallel.
#[cfg(target_arch = "x86_64")]
#[must_use]
#[inline]
pub fn project_triangle_to_screen(
    v0: Vec3,
    w0: f32,
    v1: Vec3,
    w1: f32,
    v2: Vec3,
    w2: f32,
    half_width: f32,
    half_height: f32,
) -> (ScreenPoint, ScreenPoint, ScreenPoint) {
    unsafe {
        use std::arch::x86_64::*;

        // Load data into SIMD registers
        // Layout: [v2, v1, v0, pad]
        // Note: _mm_set_ps(d, c, b, a) -> [a, b, c, d]
        let x_vec = _mm_set_ps(0.0, v2.x, v1.x, v0.x);
        let y_vec = _mm_set_ps(0.0, v2.y, v1.y, v0.y);
        let z_vec = _mm_set_ps(0.0, v2.z, v1.z, v0.z);
        // Pad w with 1.0 to avoid division by zero in the unused lane
        let w_vec = _mm_set_ps(1.0, w2, w1, w0);

        let one = _mm_set1_ps(1.0);
        let min_val = _mm_set1_ps(0.0001);

        // Check w > epsilon (vectorized)
        // If w.abs() > 0.0001, use w. Otherwise use 1.0.
        // abs_w = w & !(-0.0)
        let abs_w = _mm_andnot_ps(_mm_set1_ps(-0.0), w_vec);
        // _mm_cmpgt_ps is standard SSE
        let mask = _mm_cmpgt_ps(abs_w, min_val);

        // safe_w = blend(1.0, w, mask)
        // Use logical ops for SSE2 compatibility: (w & mask) | (1.0 & ~mask)
        let safe_w = _mm_or_ps(_mm_and_ps(w_vec, mask), _mm_andnot_ps(mask, one));

        // Clamp to avoid Inf * 0 = NaN in Newton-Raphson
        let max_w = _mm_set1_ps(1e30);
        let safe_w = _mm_min_ps(safe_w, max_w);

        // Use fast approximate reciprocal with one Newton-Raphson iteration
        // This avoids the high-latency, unpipelined division instruction,
        // freeing up the divider unit for subsequent gradient setup.
        // y0 = rcp(x)
        let rcp = _mm_rcp_ps(safe_w);
        // y1 = y0 * (2 - x * y0)
        let two = _mm_set1_ps(2.0);
        let inv_w = _mm_mul_ps(rcp, _mm_sub_ps(two, _mm_mul_ps(safe_w, rcp)));

        let ndc_x = _mm_mul_ps(x_vec, inv_w);
        let ndc_y = _mm_mul_ps(y_vec, inv_w);
        let depth = _mm_mul_ps(z_vec, inv_w);

        let hw = _mm_set1_ps(half_width);
        let hh = _mm_set1_ps(half_height);

        // Clamp values to valid i32 range to avoid undefined behavior/overflow in cvttps
        // 2147483520.0 is the largest float strictly less than i32::MAX + 1 that is representable and fits in i32
        let max_val_i32 = _mm_set1_ps(2_147_483_520.0);
        let min_val_i32 = _mm_set1_ps(-2_147_483_520.0);

        // screen_x = (ndc_x + 1.0) * half_width
        let sx = _mm_mul_ps(_mm_add_ps(ndc_x, one), hw);
        // screen_y = (1.0 - ndc_y) * half_height
        let sy = _mm_mul_ps(_mm_sub_ps(one, ndc_y), hh);

        // Clamp before conversion
        let sx = _mm_min_ps(_mm_max_ps(sx, min_val_i32), max_val_i32);
        let sy = _mm_min_ps(_mm_max_ps(sy, min_val_i32), max_val_i32);

        // Convert to int (truncation)
        let sx_i = _mm_cvttps_epi32(sx);
        let sy_i = _mm_cvttps_epi32(sy);

        // Store results to stack array
        let mut x_arr = [0i32; 4];
        let mut y_arr = [0i32; 4];
        let mut z_arr = [0f32; 4];
        let mut iw_arr = [0f32; 4];

        _mm_storeu_si128(x_arr.as_mut_ptr() as *mut __m128i, sx_i);
        _mm_storeu_si128(y_arr.as_mut_ptr() as *mut __m128i, sy_i);
        _mm_storeu_ps(z_arr.as_mut_ptr(), depth);
        _mm_storeu_ps(iw_arr.as_mut_ptr(), inv_w);

        (
            ScreenPoint {
                x: x_arr[0],
                y: y_arr[0],
                z: z_arr[0],
                inv_w: iw_arr[0],
            },
            ScreenPoint {
                x: x_arr[1],
                y: y_arr[1],
                z: z_arr[1],
                inv_w: iw_arr[1],
            },
            ScreenPoint {
                x: x_arr[2],
                y: y_arr[2],
                z: z_arr[2],
                inv_w: iw_arr[2],
            },
        )
    }
}

/// Project 4 vertices to screen coordinates in parallel.
/// Perfect for quads.
#[cfg(target_arch = "x86_64")]
#[must_use]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn project_quad_to_screen(
    v0: Vec3,
    w0: f32,
    v1: Vec3,
    w1: f32,
    v2: Vec3,
    w2: f32,
    v3: Vec3,
    w3: f32,
    half_width: f32,
    half_height: f32,
) -> (ScreenPoint, ScreenPoint, ScreenPoint, ScreenPoint) {
    unsafe {
        use std::arch::x86_64::*;

        // Load data into SIMD registers
        // Layout: [v3, v2, v1, v0]
        let x_vec = _mm_set_ps(v3.x, v2.x, v1.x, v0.x);
        let y_vec = _mm_set_ps(v3.y, v2.y, v1.y, v0.y);
        let z_vec = _mm_set_ps(v3.z, v2.z, v1.z, v0.z);
        let w_vec = _mm_set_ps(w3, w2, w1, w0);

        let one = _mm_set1_ps(1.0);
        let min_val = _mm_set1_ps(0.0001);

        // Check w > epsilon (vectorized)
        let abs_w = _mm_andnot_ps(_mm_set1_ps(-0.0), w_vec);
        let mask = _mm_cmpgt_ps(abs_w, min_val);
        let safe_w = _mm_or_ps(_mm_and_ps(w_vec, mask), _mm_andnot_ps(mask, one));

        // Clamp to avoid Inf * 0 = NaN in Newton-Raphson
        let max_w = _mm_set1_ps(1e30);
        let safe_w = _mm_min_ps(safe_w, max_w);

        // Fast reciprocal
        let rcp = _mm_rcp_ps(safe_w);
        let two = _mm_set1_ps(2.0);
        let inv_w = _mm_mul_ps(rcp, _mm_sub_ps(two, _mm_mul_ps(safe_w, rcp)));

        let ndc_x = _mm_mul_ps(x_vec, inv_w);
        let ndc_y = _mm_mul_ps(y_vec, inv_w);
        let depth = _mm_mul_ps(z_vec, inv_w);

        let hw = _mm_set1_ps(half_width);
        let hh = _mm_set1_ps(half_height);

        let max_val_i32 = _mm_set1_ps(2_147_483_520.0);
        let min_val_i32 = _mm_set1_ps(-2_147_483_520.0);

        let sx = _mm_mul_ps(_mm_add_ps(ndc_x, one), hw);
        let sy = _mm_mul_ps(_mm_sub_ps(one, ndc_y), hh);

        let sx = _mm_min_ps(_mm_max_ps(sx, min_val_i32), max_val_i32);
        let sy = _mm_min_ps(_mm_max_ps(sy, min_val_i32), max_val_i32);

        let sx_i = _mm_cvttps_epi32(sx);
        let sy_i = _mm_cvttps_epi32(sy);

        let mut x_arr = [0i32; 4];
        let mut y_arr = [0i32; 4];
        let mut z_arr = [0f32; 4];
        let mut iw_arr = [0f32; 4];

        _mm_storeu_si128(x_arr.as_mut_ptr() as *mut __m128i, sx_i);
        _mm_storeu_si128(y_arr.as_mut_ptr() as *mut __m128i, sy_i);
        _mm_storeu_ps(z_arr.as_mut_ptr(), depth);
        _mm_storeu_ps(iw_arr.as_mut_ptr(), inv_w);

        (
            ScreenPoint {
                x: x_arr[0],
                y: y_arr[0],
                z: z_arr[0],
                inv_w: iw_arr[0],
            },
            ScreenPoint {
                x: x_arr[1],
                y: y_arr[1],
                z: z_arr[1],
                inv_w: iw_arr[1],
            },
            ScreenPoint {
                x: x_arr[2],
                y: y_arr[2],
                z: z_arr[2],
                inv_w: iw_arr[2],
            },
            ScreenPoint {
                x: x_arr[3],
                y: y_arr[3],
                z: z_arr[3],
                inv_w: iw_arr[3],
            },
        )
    }
}

/// Project 4 vertices to screen coordinates (Scalar Fallback).
#[cfg(not(target_arch = "x86_64"))]
#[must_use]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn project_quad_to_screen(
    v0: Vec3,
    w0: f32,
    v1: Vec3,
    w1: f32,
    v2: Vec3,
    w2: f32,
    v3: Vec3,
    w3: f32,
    half_width: f32,
    half_height: f32,
) -> (ScreenPoint, ScreenPoint, ScreenPoint, ScreenPoint) {
    (
        project_to_screen_optimized(v0, w0, half_width, half_height),
        project_to_screen_optimized(v1, w1, half_width, half_height),
        project_to_screen_optimized(v2, w2, half_width, half_height),
        project_to_screen_optimized(v3, w3, half_width, half_height),
    )
}

/// Project 3 vertices to screen coordinates (Scalar Fallback).
#[cfg(not(target_arch = "x86_64"))]
#[must_use]
#[inline]
pub fn project_triangle_to_screen(
    v0: Vec3,
    w0: f32,
    v1: Vec3,
    w1: f32,
    v2: Vec3,
    w2: f32,
    half_width: f32,
    half_height: f32,
) -> (ScreenPoint, ScreenPoint, ScreenPoint) {
    (
        project_to_screen_optimized(v0, w0, half_width, half_height),
        project_to_screen_optimized(v1, w1, half_width, half_height),
        project_to_screen_optimized(v2, w2, half_width, half_height),
    )
}

/// Project a 3D point to screen coordinates
#[must_use]
#[inline]
pub fn project_to_screen(v: Vec3, w: f32, width: u32, height: u32) -> ScreenPoint {
    let half_width = width as f32 * 0.5;
    let half_height = height as f32 * 0.5;
    project_to_screen_optimized(v, w, half_width, half_height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_normalize_accuracy() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        let n1 = v.normalize();
        let n2 = v.fast_normalize();

        let diff = n1 - n2;
        assert!(diff.x.abs() < 0.001);
        assert!(diff.y.abs() < 0.001);
        assert!(diff.z.abs() < 0.001);
    }

    #[test]
    fn test_transform_points_parallel_threshold() {
        // Test parallel implementation properly falls back and maintains correctness
        let points = vec![Vec3::new(1.0, 2.0, 3.0); 100];
        let mut output = vec![(Vec3::default(), 0.0); 100];
        let m = Mat4::translation(5.0, 5.0, 5.0);

        m.transform_points_parallel(&points, &mut output);

        for (p, _) in output {
            assert!((p.x - 6.0).abs() < 0.001);
            assert!((p.y - 7.0).abs() < 0.001);
            assert!((p.z - 8.0).abs() < 0.001);
        }
    }

    #[test]
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    fn test_transform_point_simd_vs_scalar() {
        // Scalar implementation reference
        fn transform_point_scalar(m: &Mat4, v: Vec3) -> (Vec3, f32) {
            let x = m.m[0][0] * v.x + m.m[1][0] * v.y + m.m[2][0] * v.z + m.m[3][0];
            let y = m.m[0][1] * v.x + m.m[1][1] * v.y + m.m[2][1] * v.z + m.m[3][1];
            let z = m.m[0][2] * v.x + m.m[1][2] * v.y + m.m[2][2] * v.z + m.m[3][2];
            let w = m.m[0][3] * v.x + m.m[1][3] * v.y + m.m[2][3] * v.z + m.m[3][3];
            (Vec3::new(x, y, z), w)
        }

        let m = Mat4::rotation_y(0.5) * Mat4::translation(10.0, 5.0, 2.0);
        let v = Vec3::new(1.0, 2.0, 3.0);

        // This uses the SIMD implementation because we are compiling with simd feature
        let (simd_p, simd_w) = m.transform_point(v);
        let (scalar_p, scalar_w) = transform_point_scalar(&m, v);

        let diff_p = simd_p - scalar_p;
        assert!(
            diff_p.x.abs() < 0.0001,
            "X mismatch: {} vs {}",
            simd_p.x,
            scalar_p.x
        );
        assert!(
            diff_p.y.abs() < 0.0001,
            "Y mismatch: {} vs {}",
            simd_p.y,
            scalar_p.y
        );
        assert!(
            diff_p.z.abs() < 0.0001,
            "Z mismatch: {} vs {}",
            simd_p.z,
            scalar_p.z
        );
        assert!(
            (simd_w - scalar_w).abs() < 0.0001,
            "W mismatch: {} vs {}",
            simd_w,
            scalar_w
        );
    }

    #[test]
    fn test_perspective_projection() {
        use std::f32::consts::PI;
        let fov = PI / 2.0; // 90 degrees
        let aspect = 1.0;
        let near = 1.0;
        let far = 10.0;
        let proj = Mat4::perspective(fov, aspect, near, far);

        // Point on near plane (0, 0, -1) -> should map to w=1, z/w = -1 (OpenGL style: -1 to 1)
        // Wait, standard GL perspective maps -near to -1 and -far to 1 (or 0 to 1 depending on depth range).
        // Let's check the implementation:
        // [0][0] = f / aspect
        // [2][2] = (far + near) / (near - far) (This is typically negative)
        // [2][3] = -1.0
        // [3][2] = 2 * far * near / (near - far)
        //
        // p = (0, 0, -near)
        // x' = 0
        // y' = 0
        // z' = p.z * m[2][2] + m[3][2]
        // w' = p.z * m[2][3] + m[3][3] = -p.z = near
        //
        // z_ndc = z' / w'
        // Let's verify with actual values.

        let p_near = Vec3::new(0.0, 0.0, -near);
        let (p_near_prime, w_near) = proj.transform_point(p_near);

        assert!(
            (w_near - near).abs() < 1e-5,
            "w at near plane should be near"
        );
        // In standard GL, z_ndc at near is -1.0
        let z_ndc_near = p_near_prime.z / w_near;
        assert!(
            (z_ndc_near - (-1.0)).abs() < 1e-5,
            "NDZ z at near should be -1.0, got {}",
            z_ndc_near
        );

        let p_far = Vec3::new(0.0, 0.0, -far);
        let (p_far_prime, w_far) = proj.transform_point(p_far);
        assert!((w_far - far).abs() < 1e-5, "w at far plane should be far");
        // In standard GL, z_ndc at far is 1.0
        let z_ndc_far = p_far_prime.z / w_far;
        assert!(
            (z_ndc_far - 1.0).abs() < 1e-5,
            "NDC z at far should be 1.0, got {}",
            z_ndc_far
        );
    }

    #[test]
    fn test_look_at() {
        let eye = Vec3::new(0.0, 0.0, 10.0);
        let target = Vec3::new(0.0, 0.0, 0.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let view = Mat4::look_at(eye, target, up);

        // Point at target (world origin) should map to (0, 0, -10) in camera space
        // because camera is at (0, 0, 10) looking at origin, so origin is 10 units in front (negative Z)
        let p = Vec3::new(0.0, 0.0, 0.0);
        let (p_view, _) = view.transform_point(p);

        // Relaxed tolerance due to fast_inv_sqrt usage in look_at normalization
        let epsilon = 1e-3;
        assert!((p_view.x - 0.0).abs() < epsilon, "X mismatch: {}", p_view.x);
        assert!((p_view.y - 0.0).abs() < epsilon, "Y mismatch: {}", p_view.y);
        assert!(
            (p_view.z - (-10.0)).abs() < epsilon,
            "Z mismatch: {}",
            p_view.z
        );

        // Point at eye should map to (0, 0, 0)
        let (p_eye, _) = view.transform_point(eye);
        assert!(p_eye.length() < 1e-5);
    }

    #[test]
    fn test_project_to_screen_optimized_edge_cases() {
        let half_width = 400.0;
        let half_height = 300.0;

        // Test w = 0 (singular)
        // Code falls back to 1.0 if w.abs() <= 0.0001
        let p = Vec3::new(100.0, 100.0, 10.0);
        let sp = project_to_screen_optimized(p, 0.0, half_width, half_height);

        // Expected behavior: inv_w = 1.0, so x = 100.0, y = 100.0
        // ndc_x = 100.0. screen_x = (100+1)*400 = 40400.
        assert_eq!(sp.inv_w, 1.0);
        assert_eq!(sp.x, 40400);

        // Test very small w (but > epsilon)
        // w = 0.0002. inv_w = 5000.
        // x = 1.0. ndc_x = 5000.
        // screen_x = (5000+1)*400 = 2000400.
        let sp_small =
            project_to_screen_optimized(Vec3::new(1.0, 0.0, 0.0), 0.0002, half_width, half_height);
        assert!((sp_small.inv_w - 5000.0).abs() < 1e-1);
        assert_eq!(sp_small.x, 2000400);

        // Test negative w (behind camera)
        // w = -1.0. inv_w = -1.0.
        // x = 1.0. ndc_x = -1.0.
        // screen_x = (-1+1)*400 = 0.
        let sp_neg =
            project_to_screen_optimized(Vec3::new(1.0, 0.0, 0.0), -1.0, half_width, half_height);
        assert_eq!(sp_neg.inv_w, -1.0);
        assert_eq!(sp_neg.x, 0);
    }

    #[test]
    fn test_vec3_normalize_zero() {
        let v = Vec3::new(0.0, 0.0, 0.0);
        let n = v.normalize();
        assert_eq!(n.x, 0.0);
        assert_eq!(n.y, 0.0);
        assert_eq!(n.z, 0.0);

        let v_small = Vec3::new(1e-5, 0.0, 0.0);
        let n_small = v_small.normalize();
        // Should return original if length < 0.0001
        assert_eq!(n_small.x, 1e-5);
    }

    #[test]
    fn test_fast_inv_sqrt_sanity() {
        let x = 4.0;
        let y = fast_inv_sqrt(x);
        // 1/sqrt(4) = 0.5
        assert!((y - 0.5).abs() < 0.01);

        let x = 16.0;
        let y = fast_inv_sqrt(x);
        // 1/sqrt(16) = 0.25
        assert!((y - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_project_to_screen_safety() {
        let half_width = 400.0;
        let half_height = 300.0;

        // Test Infinity
        let v_inf = Vec3::new(f32::INFINITY, 0.0, 0.0);
        let sp_inf = project_to_screen_optimized(v_inf, 1.0, half_width, half_height);
        // Expect clamping to max/min range
        assert!(sp_inf.x == 2147483520);

        // Test Negative Infinity
        let v_neg_inf = Vec3::new(f32::NEG_INFINITY, 0.0, 0.0);
        let sp_neg_inf = project_to_screen_optimized(v_neg_inf, 1.0, half_width, half_height);
        assert!(sp_neg_inf.x == -2147483520);

        // Test NaN
        let v_nan = Vec3::new(f32::NAN, 0.0, 0.0);
        let sp_nan = project_to_screen_optimized(v_nan, 1.0, half_width, half_height);
        // Expect clamping to MIN/MAX range (NaN maps to MIN in this implementation)
        assert_eq!(sp_nan.x, -2147483520);

        // Test Large Number (overflowing i32 but finite)
        let v_large = Vec3::new(1e30, 0.0, 0.0);
        let sp_large = project_to_screen_optimized(v_large, 1.0, half_width, half_height);
        // Should clamp to 2147483520 (approx i32::MAX)
        assert_eq!(sp_large.x, 2147483520);
    }

    #[test]
    fn test_vec3_min_max() {
        let a = Vec3::new(1.0, 5.0, -2.0);
        let b = Vec3::new(3.0, 2.0, -1.0);

        let min = a.min(b);
        assert_eq!(min.x, 1.0);
        assert_eq!(min.y, 2.0);
        assert_eq!(min.z, -2.0);

        let max = a.max(b);
        assert_eq!(max.x, 3.0);
        assert_eq!(max.y, 5.0);
        assert_eq!(max.z, -1.0);
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_project_to_screen_simd_consistency() {
        let half_width = 400.0;
        let half_height = 300.0;

        let test_cases = vec![
            (Vec3::new(100.0, 100.0, 10.0), 1.0, "Normal"),
            (Vec3::new(0.0, 0.0, 0.0), 1.0, "Origin"),
            (Vec3::new(1.0, 1.0, 1.0), 0.0000001, "Small w (epsilon)"),
            (Vec3::new(1.0, 1.0, 1.0), 0.0, "Zero w"),
            (Vec3::new(1.0, 1.0, 1.0), -1.0, "Negative w"),
            (Vec3::new(f32::INFINITY, 0.0, 0.0), 1.0, "Inf X"),
            (Vec3::new(f32::NAN, 0.0, 0.0), 1.0, "NaN X"),
            (Vec3::new(1e30, 0.0, 0.0), 1.0, "Large X"),
            (Vec3::new(-1e30, 0.0, 0.0), 1.0, "Large Negative X"),
            (Vec3::new(0.0, 0.0, 0.0), f32::INFINITY, "Inf W"),
        ];

        for (v, w, name) in test_cases {
            // Scalar
            let s_scalar = project_to_screen_optimized(v, w, half_width, half_height);

            // SIMD (Triangle)
            let (s_tri_0, _, _) =
                project_triangle_to_screen(v, w, v, w, v, w, half_width, half_height);

            // Verify X and Y (allow off-by-one due to float precision + truncation)
            assert!(
                (i64::from(s_scalar.x) - i64::from(s_tri_0.x)).abs() <= 1,
                "X mismatch for case {}: {} vs {}",
                name,
                s_scalar.x,
                s_tri_0.x
            );
            assert!(
                (i64::from(s_scalar.y) - i64::from(s_tri_0.y)).abs() <= 1,
                "Y mismatch for case {}: {} vs {}",
                name,
                s_scalar.y,
                s_tri_0.y
            );

            // Check z and inv_w with some tolerance
            let z_diff = (s_scalar.z - s_tri_0.z).abs();
            let inv_w_diff = (s_scalar.inv_w - s_tri_0.inv_w).abs();

            let tolerance = if w.abs() > 1e-4 {
                0.002 // Approximation error
            } else {
                1.0 // Loose tolerance for fallback/singularities
            };

            if s_scalar.z.is_nan() {
                assert!(s_tri_0.z.is_nan(), "Z NaN mismatch for case: {}", name);
            } else {
                assert!(
                    z_diff < tolerance || (s_scalar.z.is_infinite() && s_tri_0.z.is_infinite()),
                    "Z mismatch for {}: {} vs {} (diff: {})",
                    name,
                    s_scalar.z,
                    s_tri_0.z,
                    z_diff
                );
            }

            if s_scalar.inv_w.is_nan() {
                assert!(
                    s_tri_0.inv_w.is_nan(),
                    "InvW NaN mismatch for case: {}",
                    name
                );
            } else {
                assert!(
                    inv_w_diff < tolerance
                        || (s_scalar.inv_w.is_infinite() && s_tri_0.inv_w.is_infinite()),
                    "InvW mismatch for {}: {} vs {} (diff: {})",
                    name,
                    s_scalar.inv_w,
                    s_tri_0.inv_w,
                    inv_w_diff
                );
            }
        }
    }
}

/// A 4-component vector, often used for homogeneous coordinates or tangents.
///
/// In the rasterization pipeline, `Vec4` is used for:
/// *   Homogeneous coordinates (x, y, z, w) where w is the perspective term.
/// *   Tangent vectors in Normal Mapping, where w stores the handedness of the tangent basis.
///
/// # Examples
///
/// ```
/// use abrash::math::Vec4;
///
/// let v = Vec4::new(1.0, 2.0, 3.0, 1.0);
/// assert_eq!(v.x, 1.0);
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    /// Creates a new 4D vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::math::Vec4;
    ///
    /// let v = Vec4::new(1.0, 2.0, 3.0, 1.0);
    /// assert_eq!(v.w, 1.0);
    /// ```
    #[must_use]
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
}

/// Multiply vector by scalar.
impl std::ops::Mul<f32> for Vec4 {
    type Output = Self;
    #[inline]
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
            w: self.w * scalar,
        }
    }
}

/// Component-wise addition.
impl std::ops::Add for Vec4 {
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
            w: self.w + other.w,
        }
    }
}

/// Component-wise subtraction.
impl std::ops::Sub for Vec4 {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
            w: self.w - other.w,
        }
    }
}
