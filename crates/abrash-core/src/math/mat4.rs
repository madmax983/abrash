#[allow(clippy::wildcard_imports)]
use super::*;
use std::mem::MaybeUninit;
use std::ops::{Add, Mul, Sub};

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
/// use abrash_core::math::{Mat4, Vec3};
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
#[allow(missing_docs)]
pub struct Mat4 {
    pub m: [[f32; 4]; 4],
}

impl Mat4 {
    /// Creates the standard identity matrix.
    ///
    /// The identity matrix leaves any vector or matrix it is multiplied with unchanged.
    /// Use this as the starting point for building transformation chains.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::{Mat4, Vec3};
    ///
    /// let identity = Mat4::identity();
    /// let v = Vec3::new(1.0, 2.0, 3.0);
    ///
    /// // The transform is a no-op:
    /// let (v_transformed, w) = identity.transform_point(v);
    /// assert_eq!(v_transformed, v);
    /// assert_eq!(w, 1.0);
    /// ```
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

    /// Returns the transposed matrix.
    #[must_use]
    #[inline]
    pub const fn transpose(&self) -> Self {
        let m = &self.m;
        Self {
            m: [
                [m[0][0], m[1][0], m[2][0], m[3][0]],
                [m[0][1], m[1][1], m[2][1], m[3][1]],
                [m[0][2], m[1][2], m[2][2], m[3][2]],
                [m[0][3], m[1][3], m[2][3], m[3][3]],
            ],
        }
    }

    /// Computes the matrix determinant.
    #[must_use]
    #[inline]
    pub fn determinant(&self) -> f32 {
        let m = &self.m;

        let a2323 = m[2][2] * m[3][3] - m[2][3] * m[3][2];
        let a1323 = m[2][1] * m[3][3] - m[2][3] * m[3][1];
        let a1223 = m[2][1] * m[3][2] - m[2][2] * m[3][1];
        let a0323 = m[2][0] * m[3][3] - m[2][3] * m[3][0];
        let a0223 = m[2][0] * m[3][2] - m[2][2] * m[3][0];
        let a0123 = m[2][0] * m[3][1] - m[2][1] * m[3][0];

        m[0][0] * (m[1][1] * a2323 - m[1][2] * a1323 + m[1][3] * a1223)
            - m[0][1] * (m[1][0] * a2323 - m[1][2] * a0323 + m[1][3] * a0223)
            + m[0][2] * (m[1][0] * a1323 - m[1][1] * a0323 + m[1][3] * a0123)
            - m[0][3] * (m[1][0] * a1223 - m[1][1] * a0223 + m[1][2] * a0123)
    }

    #[inline]
    fn is_affine(&self) -> bool {
        self.m[0][3].abs() <= f32::EPSILON
            && self.m[1][3].abs() <= f32::EPSILON
            && self.m[2][3].abs() <= f32::EPSILON
            && (self.m[3][3] - 1.0).abs() <= f32::EPSILON
    }

    /// Fast inverse for affine transforms (`R*S + T`), common in render loops.
    ///
    /// Falls back to zero matrix if non-invertible.
    #[must_use]
    pub fn inverse_affine(&self) -> Self {
        let m = &self.m;
        let a00 = m[0][0];
        let a01 = m[0][1];
        let a02 = m[0][2];
        let a10 = m[1][0];
        let a11 = m[1][1];
        let a12 = m[1][2];
        let a20 = m[2][0];
        let a21 = m[2][1];
        let a22 = m[2][2];

        let c00 = a11 * a22 - a12 * a21;
        let c01 = -(a10 * a22 - a12 * a20);
        let c02 = a10 * a21 - a11 * a20;
        let c10 = -(a01 * a22 - a02 * a21);
        let c11 = a00 * a22 - a02 * a20;
        let c12 = -(a00 * a21 - a01 * a20);
        let c20 = a01 * a12 - a02 * a11;
        let c21 = -(a00 * a12 - a02 * a10);
        let c22 = a00 * a11 - a01 * a10;

        let det = a00 * c00 + a01 * c01 + a02 * c02;
        if det.abs() < 1e-6 {
            return Self { m: [[0.0; 4]; 4] };
        }
        let inv_det = 1.0 / det;

        // inverse(upper3x3) == adjugate / det
        let b00 = c00 * inv_det;
        let b01 = c10 * inv_det;
        let b02 = c20 * inv_det;
        let b10 = c01 * inv_det;
        let b11 = c11 * inv_det;
        let b12 = c21 * inv_det;
        let b20 = c02 * inv_det;
        let b21 = c12 * inv_det;
        let b22 = c22 * inv_det;

        let tx = m[3][0];
        let ty = m[3][1];
        let tz = m[3][2];

        Self {
            m: [
                [b00, b01, b02, 0.0],
                [b10, b11, b12, 0.0],
                [b20, b21, b22, 0.0],
                [
                    -(tx * b00 + ty * b10 + tz * b20),
                    -(tx * b01 + ty * b11 + tz * b21),
                    -(tx * b02 + ty * b12 + tz * b22),
                    1.0,
                ],
            ],
        }
    }

    /// Normal matrix: the inverse-transpose of the upper-left 3×3.
    ///
    /// Used to correctly transform surface normals when the model matrix
    /// contains non-uniform scale.  A uniform-scale rotation matrix has
    /// `normal_matrix == rotation_part`, but non-uniform scale would shear
    /// normals without this correction.
    ///
    /// Returns `Mat3::identity()` if the matrix is singular.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::{Mat4, Mat3, Vec3};
    ///
    /// // Pure rotation: normal_matrix == upper-3x3
    /// let m = Mat4::rotation_y(0.5);
    /// let nm = m.normal_matrix();
    /// let n = Vec3::new(1.0, 0.0, 0.0);
    /// let by_nm  = nm.transform(n);
    /// let by_rot = Mat3::from_mat4(&m).transform(n);
    /// assert!((by_nm.x - by_rot.x).abs() < 1e-4);
    /// ```
    #[must_use]
    pub fn normal_matrix(&self) -> Mat3 {
        Mat3::from_mat4(self).inverse_transpose()
    }

    /// Creates a rotation matrix around the X axis.
    ///
    /// * `angle` - The angle in radians.
    #[must_use]
    #[inline]
    pub fn rotation_x(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
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
        let (s, c) = angle.sin_cos();
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
        let (s, c) = angle.sin_cos();
        Self {
            m: [
                [c, s, 0.0, 0.0],
                [-s, c, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Creates a rotation matrix around an arbitrary axis.
    ///
    /// If `axis` is near zero, returns identity.
    #[must_use]
    #[inline]
    pub fn rotation_axis(axis: Vec3, angle: f32) -> Self {
        let n = axis.normalize_or_zero();
        if n.length_sq() <= 1e-8 {
            return Self::identity();
        }

        let (s, c) = angle.sin_cos();
        let one_minus_c = 1.0 - c;
        let x = n.x;
        let y = n.y;
        let z = n.z;

        // Row-major for row-vector convention.
        Self {
            m: [
                [
                    c + x * x * one_minus_c,
                    x * y * one_minus_c + z * s,
                    x * z * one_minus_c - y * s,
                    0.0,
                ],
                [
                    y * x * one_minus_c - z * s,
                    c + y * y * one_minus_c,
                    y * z * one_minus_c + x * s,
                    0.0,
                ],
                [
                    z * x * one_minus_c + y * s,
                    z * y * one_minus_c - x * s,
                    c + z * z * one_minus_c,
                    0.0,
                ],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Creates an orthographic projection matrix.
    ///
    /// # Arguments
    ///
    /// * `left` - Left plane.
    /// * `right` - Right plane.
    /// * `bottom` - Bottom plane.
    /// * `top` - Top plane.
    /// * `near` - Distance to near clipping plane.
    /// * `far` - Distance to far clipping plane.
    #[must_use]
    #[inline]
    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let w = 1.0 / (right - left);
        let h = 1.0 / (top - bottom);
        let d = 1.0 / (near - far);

        Self {
            m: [
                [2.0 * w, 0.0, 0.0, 0.0],
                [0.0, 2.0 * h, 0.0, 0.0],
                [0.0, 0.0, 2.0 * d, 0.0],
                [
                    -(right + left) * w,
                    -(top + bottom) * h,
                    (far + near) * d,
                    1.0,
                ],
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
    /// use abrash_core::math::Mat4;
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
    /// use abrash_core::math::{Mat4, Vec3};
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

    /// 3D shear matrix — skews one axis as a linear function of the other two.
    ///
    /// The six parameters shear the axes in pairs:
    /// - `xy`: X shifts by `xy * Y`
    /// - `xz`: X shifts by `xz * Z`
    /// - `yx`: Y shifts by `yx * X`
    /// - `yz`: Y shifts by `yz * Z`
    /// - `zx`: Z shifts by `zx * X`
    /// - `zy`: Z shifts by `zy * Y`
    ///
    /// Uses row-vector convention (`v * M`).
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::{Mat4, Vec3};
    ///
    /// // Shear X by 0.5*Y
    /// let m = Mat4::shear(0.5, 0.0, 0.0, 0.0, 0.0, 0.0);
    /// let v = Vec3::new(0.0, 2.0, 0.0);
    /// let (result, _) = m.transform_point(v);
    /// assert!((result.x - 1.0).abs() < 1e-5); // x = 0 + 0.5 * 2 = 1
    /// assert!((result.y - 2.0).abs() < 1e-5);
    /// ```
    #[must_use]
    pub const fn shear(xy: f32, xz: f32, yx: f32, yz: f32, zx: f32, zy: f32) -> Self {
        // Row-vector: v * M.  Column j of M is the destination for basis vector j.
        // Row 0 = X basis:   x → x + yx*y + zx*z
        // Row 1 = Y basis:   y → xy*x + y + zy*z
        // Row 2 = Z basis:   z → xz*x + yz*y + z
        // Row-vector: result[j] = sum_i v[i] * m[i][j]
        // m[1][0] = xy gives:  x_out = x + xy*y  (X shifts by xy*Y)
        Self {
            m: [
                [1.0, yx, zx, 0.0],
                [xy, 1.0, zy, 0.0],
                [xz, yz, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Householder reflection matrix: reflect through the plane with given `normal`.
    ///
    /// The plane passes through `point` and has `normal` as its unit normal.
    /// Points on the plane are unchanged; points off the plane are mirrored.
    ///
    /// Uses row-vector convention (`v * M`).
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::{Mat4, Vec3};
    ///
    /// // Mirror through the XZ plane (normal = +Y, point = origin)
    /// let m = Mat4::reflect_plane(Vec3::new(0.0, 1.0, 0.0), Vec3::ZERO);
    /// let v = Vec3::new(1.0, 3.0, 2.0);
    /// let (r, _) = m.transform_point(v);
    /// assert!((r.x - 1.0).abs() < 1e-5);
    /// assert!((r.y + 3.0).abs() < 1e-5); // y flipped
    /// assert!((r.z - 2.0).abs() < 1e-5);
    /// ```
    #[must_use]
    pub fn reflect_plane(normal: Vec3, point: Vec3) -> Self {
        // Householder: R = I - 2 * n⊗n (for plane through origin)
        // For plane through `point`: translate to origin, reflect, translate back.
        // Expand: p' = p - 2*(p·n - d)*n  where d = point·n
        let n = normal.normalize();
        let nx = n.x;
        let ny = n.y;
        let nz = n.z;
        let d = point.dot(n); // signed distance from origin to plane
        // Row-vector form: v' = v * M
        // M = I - 2 * n⊗n (for plane through origin), with translation folded in
        Self {
            m: [
                [1.0 - 2.0 * nx * nx, -2.0 * ny * nx, -2.0 * nz * nx, 0.0],
                [-2.0 * nx * ny, 1.0 - 2.0 * ny * ny, -2.0 * nz * ny, 0.0],
                [-2.0 * nx * nz, -2.0 * ny * nz, 1.0 - 2.0 * nz * nz, 0.0],
                [2.0 * d * nx, 2.0 * d * ny, 2.0 * d * nz, 1.0],
            ],
        }
    }

    /// Unproject a screen-space point back to a world-space ray direction.
    ///
    /// This is the inverse of the full `v·(view * proj)` pipeline.
    /// Pass the combined view-projection matrix; the function inverts it and
    /// converts the NDC point back to world space.
    ///
    /// `screen_x` and `screen_y` are in `[0, width)` / `[0, height)` pixels (top-left origin).
    /// Returns the world-space ray direction (not normalized).
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::{Mat4, Vec3};
    ///
    /// let proj = Mat4::orthographic(-1.0, 1.0, -1.0, 1.0, 0.1, 100.0);
    /// // Unproject screen center
    /// let dir = Mat4::unproject(0.5, 0.5, 800, 600, &proj);
    /// ```
    #[must_use]
    pub fn unproject(
        screen_x: f32,
        screen_y: f32,
        viewport_width: u32,
        viewport_height: u32,
        view_proj: &Self,
    ) -> Vec3 {
        // Convert screen pixels to NDC [-1, 1]
        let ndc_x = (screen_x / viewport_width as f32) * 2.0 - 1.0;
        let ndc_y = 1.0 - (screen_y / viewport_height as f32) * 2.0; // flip Y
        // Use near (z=0 in NDC) and far (z=1 in NDC) points and invert VP
        let inv = view_proj.inverse();
        let near_h = Vec3::new(ndc_x, ndc_y, 0.0);
        let far_h = Vec3::new(ndc_x, ndc_y, 1.0);
        let (near_w, near_ww) = inv.transform_point(near_h);
        let (far_w, far_ww) = inv.transform_point(far_h);
        let near_pos = near_w * (1.0 / near_ww);
        let far_pos = far_w * (1.0 / far_ww);
        far_pos - near_pos
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
    /// use abrash_core::math::{Mat4, Vec3};
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

    /// Transforms a direction vector (w=0), ignoring translation.
    #[must_use]
    #[inline]
    pub fn transform_vector(&self, v: Vec3) -> Vec3 {
        let x = self.m[0][0] * v.x + self.m[1][0] * v.y + self.m[2][0] * v.z;
        let y = self.m[0][1] * v.x + self.m[1][1] * v.y + self.m[2][1] * v.z;
        let z = self.m[0][2] * v.x + self.m[1][2] * v.y + self.m[2][2] * v.z;
        Vec3::new(x, y, z)
    }

    /// Transforms a batch of direction vectors (w=0), ignoring translation.
    ///
    /// This hoists matrix elements out of the loop to reduce indexing overhead
    /// in hot inner loops.
    ///
    /// # Panics
    ///
    /// Panics if `vectors.len() != output.len()`.
    pub fn transform_vectors(&self, vectors: &[Vec3], output: &mut [Vec3]) {
        assert_eq!(vectors.len(), output.len());

        let m00 = self.m[0][0];
        let m01 = self.m[0][1];
        let m02 = self.m[0][2];
        let m10 = self.m[1][0];
        let m11 = self.m[1][1];
        let m12 = self.m[1][2];
        let m20 = self.m[2][0];
        let m21 = self.m[2][1];
        let m22 = self.m[2][2];

        for (v, out) in vectors.iter().zip(output.iter_mut()) {
            *out = Vec3::new(
                v.x * m00 + v.y * m10 + v.z * m20,
                v.x * m01 + v.y * m11 + v.z * m21,
                v.x * m02 + v.y * m12 + v.z * m22,
            );
        }
    }

    /// Fast path for transforming points by affine matrices (`w` remains 1).
    ///
    /// When the matrix is affine, this avoids computing/storing the homogeneous `w`
    /// component for every vertex.
    ///
    /// # Panics
    ///
    /// Panics if `points.len() != output.len()`.
    pub fn transform_points_affine(&self, points: &[Vec3], output: &mut [Vec3]) {
        assert_eq!(points.len(), output.len());

        if !self.is_affine() {
            for (point, out) in points.iter().zip(output.iter_mut()) {
                *out = self.transform_point(*point).0;
            }
            return;
        }

        let m00 = self.m[0][0];
        let m01 = self.m[0][1];
        let m02 = self.m[0][2];
        let m10 = self.m[1][0];
        let m11 = self.m[1][1];
        let m12 = self.m[1][2];
        let m20 = self.m[2][0];
        let m21 = self.m[2][1];
        let m22 = self.m[2][2];
        let m30 = self.m[3][0];
        let m31 = self.m[3][1];
        let m32 = self.m[3][2];

        for (p, out) in points.iter().zip(output.iter_mut()) {
            *out = Vec3::new(
                p.x * m00 + p.y * m10 + p.z * m20 + m30,
                p.x * m01 + p.y * m11 + p.z * m21 + m31,
                p.x * m02 + p.y * m12 + p.z * m22 + m32,
            );
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
    /// use abrash_core::math::{Mat4, Vec3};
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

        self.transform_points_scalar_uninit(points, output);
    }

    #[inline]
    fn transform_points_scalar_uninit(
        &self,
        points: &[Vec3],
        output: &mut [MaybeUninit<(Vec3, f32)>],
    ) {
        let m00 = self.m[0][0];
        let m01 = self.m[0][1];
        let m02 = self.m[0][2];
        let m03 = self.m[0][3];
        let m10 = self.m[1][0];
        let m11 = self.m[1][1];
        let m12 = self.m[1][2];
        let m13 = self.m[1][3];
        let m20 = self.m[2][0];
        let m21 = self.m[2][1];
        let m22 = self.m[2][2];
        let m23 = self.m[2][3];
        let m30 = self.m[3][0];
        let m31 = self.m[3][1];
        let m32 = self.m[3][2];
        let m33 = self.m[3][3];

        for (p, out) in points.iter().zip(output.iter_mut()) {
            let x = p.x * m00 + p.y * m10 + p.z * m20 + m30;
            let y = p.x * m01 + p.y * m11 + p.z * m21 + m31;
            let z = p.x * m02 + p.y * m12 + p.z * m22 + m32;
            let w = p.x * m03 + p.y * m13 + p.z * m23 + m33;
            out.write((Vec3::new(x, y, z), w));
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
            let base = &raw const dummy as usize;
            let x_ptr = &raw const dummy.0.x as usize;
            let w_ptr = &raw const dummy.1 as usize;
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
    /// use abrash_core::math::{Mat4, Vec3};
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
            use rayon::prelude::*;

            // Chunk size of 4096 ensures we amortize task overhead and keep the AVX2
            // implementation fed with enough data to be efficient.
            const CHUNK_SIZE: usize = 32768;

            // Fallback to scalar for small inputs to avoid Rayon overhead
            if points.len() < 1024 {
                self.transform_points_uninit(points, output);
                return;
            }

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

    /// Calculates the inverse of the matrix.
    ///
    /// Returns a zero matrix if the matrix is not invertible.
    #[must_use]
    pub fn inverse(&self) -> Self {
        if self.is_affine() {
            return self.inverse_affine();
        }

        #[cfg(all(target_arch = "x86_64", feature = "simd"))]
        {
            unsafe {
                use std::arch::x86_64::{
                    _mm_add_ps, _mm_castsi128_ps, _mm_div_ps, _mm_load_ps, _mm_mul_ps,
                    _mm_set_epi32, _mm_set1_ps, _mm_shuffle_ps, _mm_store_ps, _mm_storeu_ps,
                    _mm_sub_ps, _mm_xor_ps,
                };

                let row0 = _mm_load_ps(self.m[0].as_ptr());
                let row1 = _mm_load_ps(self.m[1].as_ptr());
                let row2 = _mm_load_ps(self.m[2].as_ptr());
                let row3 = _mm_load_ps(self.m[3].as_ptr());

                let t0 = _mm_shuffle_ps(row0, row1, 0x44);
                let t1 = _mm_shuffle_ps(row2, row3, 0x44);
                let t2 = _mm_shuffle_ps(row0, row1, 0xEE);
                let t3 = _mm_shuffle_ps(row2, row3, 0xEE);

                let c0 = _mm_shuffle_ps(t0, t1, 0x88);
                let c1 = _mm_shuffle_ps(t0, t1, 0xDD);
                let c2 = _mm_shuffle_ps(t2, t3, 0x88);
                let c3 = _mm_shuffle_ps(t2, t3, 0xDD);

                let mut fac0 = _mm_shuffle_ps(c2, c2, 0x50);
                let mut fac1 = _mm_shuffle_ps(c3, c3, 0xEE);
                let mut fac2 = _mm_shuffle_ps(c2, c2, 0x05);
                let mut fac3 = _mm_shuffle_ps(c3, c3, 0xAF);

                let mut v0 = _mm_mul_ps(fac0, fac1);
                v0 = _mm_sub_ps(v0, _mm_mul_ps(fac2, fac3));

                fac0 = _mm_shuffle_ps(c1, c1, 0x50);
                fac1 = _mm_shuffle_ps(c3, c3, 0xEE);
                fac2 = _mm_shuffle_ps(c1, c1, 0x05);
                fac3 = _mm_shuffle_ps(c3, c3, 0xAF);
                let mut v1 = _mm_mul_ps(fac0, fac1);
                v1 = _mm_sub_ps(v1, _mm_mul_ps(fac2, fac3));

                fac0 = _mm_shuffle_ps(c1, c1, 0x50);
                fac1 = _mm_shuffle_ps(c2, c2, 0xEE);
                fac2 = _mm_shuffle_ps(c1, c1, 0x05);
                fac3 = _mm_shuffle_ps(c2, c2, 0xAF);
                let mut v2 = _mm_mul_ps(fac0, fac1);
                v2 = _mm_sub_ps(v2, _mm_mul_ps(fac2, fac3));

                let sign_a =
                    _mm_castsi128_ps(_mm_set_epi32(-2_147_483_648_i32, 0, -2_147_483_648_i32, 0));
                let sign_b =
                    _mm_castsi128_ps(_mm_set_epi32(0, -2_147_483_648_i32, 0, -2_147_483_648_i32));

                let mut inv0 = _mm_mul_ps(c1, _mm_shuffle_ps(v0, v0, 0x39));
                inv0 = _mm_sub_ps(inv0, _mm_mul_ps(c2, _mm_shuffle_ps(v1, v1, 0x39)));
                inv0 = _mm_add_ps(inv0, _mm_mul_ps(c3, _mm_shuffle_ps(v2, v2, 0x39)));
                inv0 = _mm_xor_ps(inv0, sign_b);

                let mut inv1 = _mm_mul_ps(c0, _mm_shuffle_ps(v0, v0, 0x39));
                inv1 = _mm_sub_ps(inv1, _mm_mul_ps(c2, _mm_shuffle_ps(v1, v1, 0x8E)));
                inv1 = _mm_add_ps(inv1, _mm_mul_ps(c3, _mm_shuffle_ps(v2, v2, 0x8E)));
                inv1 = _mm_xor_ps(inv1, sign_a);

                fac0 = _mm_shuffle_ps(c0, c0, 0x50);
                fac1 = _mm_shuffle_ps(c3, c3, 0xEE);
                fac2 = _mm_shuffle_ps(c0, c0, 0x05);
                fac3 = _mm_shuffle_ps(c3, c3, 0xAF);
                v0 = _mm_mul_ps(fac0, fac1);
                v0 = _mm_sub_ps(v0, _mm_mul_ps(fac2, fac3));

                fac0 = _mm_shuffle_ps(c0, c0, 0x50);
                fac1 = _mm_shuffle_ps(c2, c2, 0xEE);
                fac2 = _mm_shuffle_ps(c0, c0, 0x05);
                fac3 = _mm_shuffle_ps(c2, c2, 0xAF);
                v1 = _mm_mul_ps(fac0, fac1);
                v1 = _mm_sub_ps(v1, _mm_mul_ps(fac2, fac3));

                fac0 = _mm_shuffle_ps(c0, c0, 0x50);
                fac1 = _mm_shuffle_ps(c1, c1, 0xEE);
                fac2 = _mm_shuffle_ps(c0, c0, 0x05);
                fac3 = _mm_shuffle_ps(c1, c1, 0xAF);
                v2 = _mm_mul_ps(fac0, fac1);
                v2 = _mm_sub_ps(v2, _mm_mul_ps(fac2, fac3));

                let mut inv2 = _mm_mul_ps(c0, _mm_shuffle_ps(v0, v0, 0x8E));
                inv2 = _mm_sub_ps(inv2, _mm_mul_ps(c1, _mm_shuffle_ps(v1, v1, 0x39)));
                inv2 = _mm_add_ps(inv2, _mm_mul_ps(c3, _mm_shuffle_ps(v2, v2, 0x8E)));
                inv2 = _mm_xor_ps(inv2, sign_b);

                let mut inv3 = _mm_mul_ps(c0, _mm_shuffle_ps(v0, v0, 0x39));
                inv3 = _mm_sub_ps(inv3, _mm_mul_ps(c1, _mm_shuffle_ps(v1, v1, 0x8E)));
                inv3 = _mm_add_ps(inv3, _mm_mul_ps(c2, _mm_shuffle_ps(v2, v2, 0x8E)));
                inv3 = _mm_xor_ps(inv3, sign_a);

                let dot0 = _mm_mul_ps(c0, inv0);
                let dot1 = _mm_shuffle_ps(dot0, dot0, 0x39);
                let dot2 = _mm_shuffle_ps(dot0, dot0, 0x4E);
                let dot3 = _mm_shuffle_ps(dot0, dot0, 0x93);
                let det = _mm_add_ps(_mm_add_ps(dot0, dot1), _mm_add_ps(dot2, dot3));

                let mut det_arr = [0.0f32; 4];
                _mm_storeu_ps(det_arr.as_mut_ptr(), det);
                if det_arr[0].abs() < 1e-6 {
                    return Self { m: [[0.0; 4]; 4] };
                }

                let rcp_det = _mm_div_ps(_mm_set1_ps(1.0), det);

                let mut out = Self::default();
                _mm_store_ps(out.m[0].as_mut_ptr(), _mm_mul_ps(inv0, rcp_det));
                _mm_store_ps(out.m[1].as_mut_ptr(), _mm_mul_ps(inv1, rcp_det));
                _mm_store_ps(out.m[2].as_mut_ptr(), _mm_mul_ps(inv2, rcp_det));
                _mm_store_ps(out.m[3].as_mut_ptr(), _mm_mul_ps(inv3, rcp_det));
                return out;
            }
        }

        #[cfg(not(all(target_arch = "x86_64", feature = "simd")))]
        {
            let m = &self.m;
            let a2323 = m[2][2] * m[3][3] - m[2][3] * m[3][2];
            let a1323 = m[2][1] * m[3][3] - m[2][3] * m[3][1];
            let a1223 = m[2][1] * m[3][2] - m[2][2] * m[3][1];
            let a0323 = m[2][0] * m[3][3] - m[2][3] * m[3][0];
            let a0223 = m[2][0] * m[3][2] - m[2][2] * m[3][0];
            let a0123 = m[2][0] * m[3][1] - m[2][1] * m[3][0];

            let mut inv = Self::default();

            inv.m[0][0] = m[1][1] * a2323 - m[1][2] * a1323 + m[1][3] * a1223;
            inv.m[0][1] = -(m[0][1] * a2323 - m[0][2] * a1323 + m[0][3] * a1223);
            inv.m[0][2] = m[0][1] * (m[1][2] * m[3][3] - m[1][3] * m[3][2])
                - m[0][2] * (m[1][1] * m[3][3] - m[1][3] * m[3][1])
                + m[0][3] * (m[1][1] * m[3][2] - m[1][2] * m[3][1]);
            inv.m[0][3] = -(m[0][1] * (m[1][2] * m[2][3] - m[1][3] * m[2][2])
                - m[0][2] * (m[1][1] * m[2][3] - m[1][3] * m[2][1])
                + m[0][3] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]));

            inv.m[1][0] = -(m[1][0] * a2323 - m[1][2] * a0323 + m[1][3] * a0223);
            inv.m[1][1] = m[0][0] * a2323 - m[0][2] * a0323 + m[0][3] * a0223;
            inv.m[1][2] = -(m[0][0] * (m[1][2] * m[3][3] - m[1][3] * m[3][2])
                - m[0][2] * (m[1][0] * m[3][3] - m[1][3] * m[3][0])
                + m[0][3] * (m[1][0] * m[3][2] - m[1][2] * m[3][0]));
            inv.m[1][3] = m[0][0] * (m[1][2] * m[2][3] - m[1][3] * m[2][2])
                - m[0][2] * (m[1][0] * m[2][3] - m[1][3] * m[2][0])
                + m[0][3] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]);

            inv.m[2][0] = m[1][0] * a1323 - m[1][1] * a0323 + m[1][3] * a0123;
            inv.m[2][1] = -(m[0][0] * a1323 - m[0][1] * a0323 + m[0][3] * a0123);
            inv.m[2][2] = m[0][0] * (m[1][1] * m[3][3] - m[1][3] * m[3][1])
                - m[0][1] * (m[1][0] * m[3][3] - m[1][3] * m[3][0])
                + m[0][3] * (m[1][0] * m[3][1] - m[1][1] * m[3][0]);
            inv.m[2][3] = -(m[0][0] * (m[1][1] * m[2][3] - m[1][3] * m[2][1])
                - m[0][1] * (m[1][0] * m[2][3] - m[1][3] * m[2][0])
                + m[0][3] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]));

            inv.m[3][0] = -(m[1][0] * a1223 - m[1][1] * a0223 + m[1][2] * a0123);
            inv.m[3][1] = m[0][0] * a1223 - m[0][1] * a0223 + m[0][2] * a0123;
            inv.m[3][2] = -(m[0][0] * (m[1][1] * m[3][2] - m[1][2] * m[3][1])
                - m[0][1] * (m[1][0] * m[3][2] - m[1][2] * m[3][0])
                + m[0][2] * (m[1][0] * m[3][1] - m[1][1] * m[3][0]));
            inv.m[3][3] = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
                - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
                + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);

            let det = m[0][0] * inv.m[0][0]
                + m[0][1] * inv.m[1][0]
                + m[0][2] * inv.m[2][0]
                + m[0][3] * inv.m[3][0];

            if det.abs() < 1e-6 {
                return Self { m: [[0.0; 4]; 4] };
            }

            let inv_det = 1.0 / det;
            for i in 0..4 {
                for j in 0..4 {
                    inv.m[i][j] *= inv_det;
                }
            }
            inv
        }
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

    /// Return the given row as a [`Vec4`].
    ///
    /// Row 3 is the translation row in this library's row-vector convention.
    ///
    /// # Panics
    ///
    /// Panics if `index >= 4`.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::{Mat4, Vec4};
    ///
    /// let m = Mat4::identity();
    /// assert_eq!(m.row(0), Vec4::new(1.0, 0.0, 0.0, 0.0));
    /// assert_eq!(m.row(3), Vec4::new(0.0, 0.0, 0.0, 1.0));
    /// ```
    #[must_use]
    #[inline]
    pub const fn row(&self, index: usize) -> Vec4 {
        Vec4::new(
            self.m[index][0],
            self.m[index][1],
            self.m[index][2],
            self.m[index][3],
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
