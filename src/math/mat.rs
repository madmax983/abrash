use std::mem::MaybeUninit;
use std::ops::{Mul};
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
use super::{Vec2, Vec3, Vec4};

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
        vertices.iter().map(|&v| self.transform(v)).collect()
    }

    /// Transform vertices in place.
    pub fn transform_in_place(&self, vertices: &mut [Vec2]) {
        for v in vertices.iter_mut() {
            *v = self.transform(*v);
        }
    }
}

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

            #[allow(clippy::items_after_statements)]
            use rayon::prelude::*;
            // Chunk size of 4096 ensures we amortize task overhead and keep the AVX2
            // implementation fed with enough data to be efficient.
            #[allow(clippy::items_after_statements)]
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