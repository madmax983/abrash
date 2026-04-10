use crate::math::*;

/// A 3D point that has been projected into 2D screen coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(missing_docs)]
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
/// use abrash_core::math::{project_to_screen_optimized, Vec3};
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
    const MAX_VAL: f32 = 2_147_483_520.0;
    const MIN_VAL: f32 = -2_147_483_520.0;

    // Perspective divide
    let inv_w = if w.abs() > 0.0001 { 1.0 / w } else { 1.0 };
    let ndc_x = v.x * inv_w;
    let ndc_y = v.y * inv_w;
    let depth = v.z * inv_w;

    // NDC to screen coordinates
    // Clamp to [i32::MIN + 1, i32::MAX] to avoid integer overflow when negating i32::MIN.
    // We clamp the float value BEFORE casting to i32 to avoid Undefined Behavior with NaN/Inf.
    // 2147483520.0 is the largest f32 strictly less than i32::MAX + 1 that is exactly representable.

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
        use std::arch::x86_64::{
            __m128i, _mm_add_ps, _mm_and_ps, _mm_andnot_ps, _mm_cmpgt_ps, _mm_cvttps_epi32,
            _mm_max_ps, _mm_min_ps, _mm_mul_ps, _mm_or_ps, _mm_rcp_ps, _mm_set_ps, _mm_set1_ps,
            _mm_storeu_ps, _mm_storeu_si128, _mm_sub_ps,
        };

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

        #[allow(clippy::cast_ptr_alignment)]
        {
            _mm_storeu_si128(x_arr.as_mut_ptr().cast::<__m128i>(), sx_i);
            _mm_storeu_si128(y_arr.as_mut_ptr().cast::<__m128i>(), sy_i);
        }
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
        use std::arch::x86_64::{
            __m128i, _mm_add_ps, _mm_and_ps, _mm_andnot_ps, _mm_cmpgt_ps, _mm_cvttps_epi32,
            _mm_max_ps, _mm_min_ps, _mm_mul_ps, _mm_or_ps, _mm_rcp_ps, _mm_set_ps, _mm_set1_ps,
            _mm_storeu_ps, _mm_storeu_si128, _mm_sub_ps,
        };

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

        #[allow(clippy::cast_ptr_alignment)]
        {
            _mm_storeu_si128(x_arr.as_mut_ptr().cast::<__m128i>(), sx_i);
            _mm_storeu_si128(y_arr.as_mut_ptr().cast::<__m128i>(), sy_i);
        }
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

// ── Spline / Curve Interpolation ─────────────────────────────────────────────

/// Evaluate a quadratic Bézier curve at parameter `t ∈ [0, 1]`.
///
/// `p0` is the start, `p1` is the control point, `p2` is the end.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{bezier_quadratic, Vec3};
///
/// let p = bezier_quadratic(Vec3::ZERO, Vec3::new(0.5, 1.0, 0.0), Vec3::ONE, 0.5);
/// // Midpoint of a quadratic curve through (0,0,0)→(0.5,1,0)→(1,1,1)
/// assert!((p.x - 0.5).abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn bezier_quadratic(p0: Vec3, p1: Vec3, p2: Vec3, t: f32) -> Vec3 {
    let u = 1.0 - t;
    p0 * (u * u) + p1 * (2.0 * u * t) + p2 * (t * t)
}

/// Evaluate a cubic Bézier curve at parameter `t ∈ [0, 1]`.
///
/// `p0`/`p3` are endpoints; `p1`/`p2` are control points.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{bezier_cubic, Vec3};
///
/// let p = bezier_cubic(Vec3::ZERO, Vec3::X, Vec3::new(2.0, 1.0, 0.0), Vec3::new(3.0, 0.0, 0.0), 0.0);
/// assert_eq!(p, Vec3::ZERO);
/// let p1 = bezier_cubic(Vec3::ZERO, Vec3::X, Vec3::new(2.0, 1.0, 0.0), Vec3::new(3.0, 0.0, 0.0), 1.0);
/// assert!((p1.x - 3.0).abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn bezier_cubic(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let u = 1.0 - t;
    let u2 = u * u;
    let t2 = t * t;
    p0 * (u2 * u) + p1 * (3.0 * u2 * t) + p2 * (3.0 * u * t2) + p3 * (t2 * t)
}

/// Tangent (derivative) of a cubic Bézier at parameter `t`.
///
/// Returns an **unnormalized** tangent vector.
#[must_use]
#[inline]
pub fn bezier_cubic_tangent(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let u = 1.0 - t;
    (p1 - p0) * (3.0 * u * u) + (p2 - p1) * (6.0 * u * t) + (p3 - p2) * (3.0 * t * t)
}

/// Evaluate a Catmull-Rom spline segment at `t ∈ [0, 1]`.
///
/// `p0`/`p3` are the two outer control points; `p1`/`p2` are the segment endpoints.
/// The curve passes through `p1` at `t=0` and `p2` at `t=1`.
///
/// Uses α=0.5 (centripetal Catmull-Rom) tension.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{catmull_rom, Vec3};
///
/// let p = catmull_rom(Vec3::new(-1.0, 0.0, 0.0), Vec3::ZERO,
///                     Vec3::ONE, Vec3::new(2.0, 1.0, 0.0), 0.0);
/// assert!(p.x.abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn catmull_rom(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;
    // Barry-Goldman formulation with tension 0.5
    p0 * (-0.5 * t3 + t2 - 0.5 * t)
        + p1 * (1.5 * t3 - 2.5 * t2 + 1.0)
        + p2 * (-1.5 * t3 + 2.0 * t2 + 0.5 * t)
        + p3 * (0.5 * t3 - 0.5 * t2)
}

/// Evaluate a cubic Hermite spline between `p0` and `p1` at `t ∈ [0, 1]`.
///
/// `m0` and `m1` are the tangents at the start and end points respectively.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{hermite, Vec3};
///
/// // Zero tangents → linear interpolation (H basis reduces to lerp)
/// // Actually cubic hermite with m=0 gives same endpoints but isn't linear for interior.
/// let p = hermite(Vec3::ZERO, Vec3::ZERO, Vec3::ONE, Vec3::ZERO, 0.0);
/// assert_eq!(p, Vec3::ZERO);
/// let p1 = hermite(Vec3::ZERO, Vec3::ZERO, Vec3::ONE, Vec3::ZERO, 1.0);
/// assert_eq!(p1, Vec3::ONE);
/// ```
#[must_use]
#[inline]
pub fn hermite(p0: Vec3, m0: Vec3, p1: Vec3, m1: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;
    p0 * h00 + m0 * h10 + p1 * h01 + m1 * h11
}

/// Kochanek-Bartels (TCB) spline segment at `t ∈ [0, 1]`.
///
/// A generalization of Catmull-Rom with three parameters per control point:
/// - `tension` `t_val` in [-1, 1]: 1 = tight (no overshoot), -1 = loose.
/// - `continuity` `c_val` in [-1, 1]: 0 = smooth, ±1 = sharp corner.
/// - `bias` `b_val` in [-1, 1]: 0 = symmetric, 1 = pre-weight, -1 = post-weight.
///
/// With all three at 0 this reduces to Catmull-Rom.
/// The curve passes through `p1` at `t=0` and `p2` at `t=1`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{kochanek_bartels, Vec3};
///
/// // All-zero parameters → same as Catmull-Rom; passes through p1 at t=0
/// let p = kochanek_bartels(
///     Vec3::new(-1.0, 0.0, 0.0), Vec3::ZERO, Vec3::ONE, Vec3::new(2.0, 1.0, 0.0),
///     0.0, 0.0, 0.0, 0.0,
/// );
/// assert!(p.x.abs() < 1e-5, "should pass through p1, got x={}", p.x);
/// ```
#[must_use]
#[inline]
pub fn kochanek_bartels(
    p0: Vec3,
    p1: Vec3,
    p2: Vec3,
    p3: Vec3,
    t: f32,
    t_val: f32,
    c_val: f32,
    b_val: f32,
) -> Vec3 {
    // Kochanek-Bartels tangents (incoming/outgoing at p1 and p2)
    let s1 = (1.0 - t_val) * 0.5;
    let d1 = (p1 - p0) * (s1 * (1.0 + c_val) * (1.0 + b_val))
        + (p2 - p1) * (s1 * (1.0 - c_val) * (1.0 - b_val));
    let d2 = (p2 - p1) * (s1 * (1.0 + c_val) * (1.0 - b_val))
        + (p3 - p2) * (s1 * (1.0 - c_val) * (1.0 + b_val));
    hermite(p1, d1, p2, d2, t)
}

/// Subdivide a cubic Bézier at `t` using de Casteljau's algorithm.
///
/// Returns `(left, right)` where `left` and `right` are each four control
/// points of a cubic Bézier that together cover the same arc as the original.
/// `left` covers `[0, t]` and `right` covers `[t, 1]`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{bezier_cubic, bezier_cubic_split, Vec3};
///
/// let (p0, p1, p2, p3) = (Vec3::ZERO, Vec3::new(1.0, 2.0, 0.0),
///                         Vec3::new(2.0, 2.0, 0.0), Vec3::new(3.0, 0.0, 0.0));
/// let (left, right) = bezier_cubic_split(p0, p1, p2, p3, 0.5);
/// // left[3] should equal right[0] == point on original curve at t=0.5
/// let mid = bezier_cubic(p0, p1, p2, p3, 0.5);
/// assert!((left[3].x - mid.x).abs() < 1e-5);
/// assert!((right[0].x - mid.x).abs() < 1e-5);
/// ```
#[must_use]
pub fn bezier_cubic_split(
    p0: Vec3,
    p1: Vec3,
    p2: Vec3,
    p3: Vec3,
    t: f32,
) -> ([Vec3; 4], [Vec3; 4]) {
    // de Casteljau: one level per step
    let q0 = p0.lerp(p1, t);
    let q1 = p1.lerp(p2, t);
    let q2 = p2.lerp(p3, t);
    let r0 = q0.lerp(q1, t);
    let r1 = q1.lerp(q2, t);
    let s = r0.lerp(r1, t);
    ([p0, q0, r0, s], [s, r1, q2, p3])
}

/// Evaluate a uniform cubic B-spline at parameter `t ∈ [0, 1]`.
///
/// The B-spline basis produces a curve that passes **near** (not through)
/// `p1` and `p2` — it has C² continuity unlike Catmull-Rom (C¹).
///
/// `p0`, `p1`, `p2`, `p3` are four consecutive control points.
/// Result is inside the convex hull of the four points.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{bspline_eval, Vec3};
///
/// let p0 = Vec3::new(0.0, 0.0, 0.0);
/// let p1 = Vec3::new(1.0, 2.0, 0.0);
/// let p2 = Vec3::new(2.0, 2.0, 0.0);
/// let p3 = Vec3::new(3.0, 0.0, 0.0);
///
/// // Result must lie within the convex hull
/// let p = bspline_eval(p0, p1, p2, p3, 0.5);
/// assert!(p.x > 0.0 && p.x < 3.0);
/// assert!(p.y > 0.0 && p.y < 3.0);
/// ```
#[must_use]
#[inline]
pub fn bspline_eval(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;
    // Uniform B-spline basis (divide by 6)
    let b0 = (1.0 - 3.0 * t + 3.0 * t2 - t3) / 6.0;
    let b1 = (4.0 - 6.0 * t2 + 3.0 * t3) / 6.0;
    let b2 = (1.0 + 3.0 * t + 3.0 * t2 - 3.0 * t3) / 6.0;
    let b3 = t3 / 6.0;
    p0 * b0 + p1 * b1 + p2 * b2 + p3 * b3
}

/// Approximate arc-length of a cubic Bézier using 5-point Gaussian quadrature.
///
/// More accurate than the common chord-sum approximation.  The relative error
/// for typical curves (aspect ratio ≤ 4) is under 0.01%.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{bezier_cubic_arc_length, Vec3};
///
/// // Straight line from (0,0,0) to (1,0,0) — control points collinear
/// let p0 = Vec3::new(0.0, 0.0, 0.0);
/// let p1 = Vec3::new(1.0/3.0, 0.0, 0.0);
/// let p2 = Vec3::new(2.0/3.0, 0.0, 0.0);
/// let p3 = Vec3::new(1.0, 0.0, 0.0);
/// let len = bezier_cubic_arc_length(p0, p1, p2, p3);
/// assert!((len - 1.0).abs() < 1e-4, "straight line length should be 1.0, got {len}");
/// ```
#[must_use]
pub fn bezier_cubic_arc_length(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3) -> f32 {
    // 5-point Gauss-Legendre nodes and weights on [0, 1]
    const NODES: [f32; 5] = [
        0.046_910_077,
        0.230_765_346,
        0.5,
        0.769_234_654,
        0.953_089_923,
    ];
    const WEIGHTS: [f32; 5] = [
        0.118_463_443,
        0.239_314_335,
        0.284_444_444,
        0.239_314_335,
        0.118_463_443,
    ];
    let mut length = 0.0_f32;
    for (i, &t) in NODES.iter().enumerate() {
        // Derivative of cubic Bézier: B'(t) = 3[(p1-p0)(1-t)² + 2(p2-p1)t(1-t) + (p3-p2)t²]
        let inv_t = 1.0 - t;
        let d = (p1 - p0) * (3.0 * inv_t * inv_t)
            + (p2 - p1) * (6.0 * inv_t * t)
            + (p3 - p2) * (3.0 * t * t);
        length += WEIGHTS[i] * d.length();
    }
    length
}

/// Find the real roots of a quadratic `ax² + bx + c = 0`.
///
/// Returns roots sorted in ascending order in the `[f32; 2]` array and the
/// count of real roots in the `usize`.  Use `roots[..count]` to iterate.
///
/// # Examples
///
/// ```
/// use abrash_core::math::quadratic_solve;
///
/// // x² - 5x + 6 = 0  → roots 2 and 3
/// let (roots, n) = quadratic_solve(1.0, -5.0, 6.0);
/// assert_eq!(n, 2);
/// assert!((roots[0] - 2.0).abs() < 1e-5);
/// assert!((roots[1] - 3.0).abs() < 1e-5);
///
/// // x² + 1 = 0 → no real roots
/// let (_, n) = quadratic_solve(1.0, 0.0, 1.0);
/// assert_eq!(n, 0);
/// ```
#[must_use]
pub fn quadratic_solve(a: f32, b: f32, c: f32) -> ([f32; 2], usize) {
    if a.abs() < 1e-10 {
        if b.abs() > 1e-10 {
            return ([-c / b, 0.0], 1);
        }
        return ([0.0; 2], 0);
    }
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 {
        return ([0.0; 2], 0);
    }
    if disc < 1e-10 {
        return ([-b / (2.0 * a), 0.0], 1);
    }
    let sq = disc.sqrt();
    let inv2a = 1.0 / (2.0 * a);
    let mut r0 = (-b - sq) * inv2a;
    let mut r1 = (-b + sq) * inv2a;
    if r0 > r1 {
        std::mem::swap(&mut r0, &mut r1);
    }
    ([r0, r1], 2)
}

/// Find real roots of a cubic `ax³ + bx² + cx + d = 0` (Cardano / trig method).
///
/// Returns roots sorted in ascending order in the `[f32; 3]` array and the
/// count of real roots.  Use `roots[..count]` to iterate.
///
/// # Examples
///
/// ```
/// use abrash_core::math::cubic_solve;
///
/// // x³ - 6x² + 11x - 6 = 0  → roots 1, 2, 3
/// let (roots, n) = cubic_solve(1.0, -6.0, 11.0, -6.0);
/// assert_eq!(n, 3);
/// assert!((roots[0] - 1.0).abs() < 1e-4);
/// assert!((roots[1] - 2.0).abs() < 1e-4);
/// assert!((roots[2] - 3.0).abs() < 1e-4);
/// ```
#[must_use]
pub fn cubic_solve(a: f32, b: f32, c: f32, d: f32) -> ([f32; 3], usize) {
    if a.abs() < 1e-10 {
        let (qr, n) = quadratic_solve(b, c, d);
        return ([qr[0], qr[1], 0.0], n);
    }
    // Reduce to depressed cubic t³ + pt + q = 0 (substitute x = t - b/3a)
    let inv_a = 1.0 / a;
    let b = b * inv_a;
    let c = c * inv_a;
    let d = d * inv_a;
    let p = c - b * b / 3.0;
    let q = 2.0 * b * b * b / 27.0 - b * c / 3.0 + d;
    let disc = q * q / 4.0 + p * p * p / 27.0;
    let shift = -b / 3.0;
    if disc > 1e-10 {
        // One real root (Cardano)
        let sq = disc.sqrt();
        let u = (-q / 2.0 + sq).cbrt();
        let v = (-q / 2.0 - sq).cbrt();
        ([u + v + shift, 0.0, 0.0], 1)
    } else if disc > -1e-10 {
        // Two distinct real roots (one double root)
        let u = (-q / 2.0).cbrt();
        let mut r = [2.0 * u + shift, -u + shift];
        r.sort_by(f32::total_cmp);
        // Deduplicate if nearly equal
        if (r[0] - r[1]).abs() < 1e-7 {
            ([r[0], 0.0, 0.0], 1)
        } else {
            ([r[0], r[1], 0.0], 2)
        }
    } else {
        // Three distinct real roots (trigonometric method)
        let m = 2.0 * (-p / 3.0).sqrt();
        let theta = (3.0 * q / (p * m)).acos() / 3.0;
        let step = std::f32::consts::TAU / 3.0; // 2π/3
        let mut r = [
            m * theta.cos() + shift,
            m * (theta - step).cos() + shift,
            m * (theta - 2.0 * step).cos() + shift,
        ];
        r.sort_by(f32::total_cmp);
        (r, 3)
    }
}

/// Build an orthonormal tangent-bitangent frame from a surface normal.
///
/// Uses the Duff et al. 2017 ("Building an Orthonormal Basis, Revisited")
/// revision of Frisvad's method.  Branchless, numerically stable for all
/// normals including those near `(0, -1, 0)`.
///
/// Returns `(tangent, bitangent)` such that `(tangent, bitangent, normal)` form
/// a right-handed orthonormal basis.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{basis_from_normal, Vec3};
///
/// let n = Vec3::new(0.0, 1.0, 0.0);
/// let (t, b) = basis_from_normal(n);
/// assert!(t.dot(n).abs() < 1e-5, "t must be perp to n");
/// assert!(b.dot(n).abs() < 1e-5, "b must be perp to n");
/// assert!(t.dot(b).abs() < 1e-5, "t and b must be perp");
/// assert!((t.length() - 1.0).abs() < 1e-5, "t must be unit");
/// assert!((b.length() - 1.0).abs() < 1e-5, "b must be unit");
/// ```
#[must_use]
pub fn basis_from_normal(n: Vec3) -> (Vec3, Vec3) {
    // Duff et al. 2017 — sign(n.z) trick avoids the n.z ≈ -1 singularity
    let sign = n.z.signum(); // ±1, never 0 (signum(0) = 1 in Rust)
    let a = -1.0 / (sign + n.z);
    let b = n.x * n.y * a;
    let tangent = Vec3::new(1.0 + sign * n.x * n.x * a, sign * b, -sign * n.x);
    let bitangent = Vec3::new(b, sign + n.y * n.y * a, -n.y);
    (tangent, bitangent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reflect() {
        let v = Vec3::new(1.0, -1.0, 0.0);
        let n = Vec3::new(0.0, 1.0, 0.0);
        let r = v.reflect(n);
        assert!((r.x - 1.0).abs() < f32::EPSILON);
        assert!((r.y - 1.0).abs() < f32::EPSILON);
        assert!((r.z - 0.0).abs() < f32::EPSILON);

        let v2 = Vec3::new(1.0, 2.0, 3.0);
        let n2 = Vec3::new(0.0, 1.0, 0.0);
        let r2 = v2.reflect(n2);
        assert!((r2.x - 1.0).abs() < f32::EPSILON);
        assert!((r2.y - -2.0).abs() < f32::EPSILON);
        assert!((r2.z - 3.0).abs() < f32::EPSILON);
    }

    // ── Vec2 component ops ────────────────────────────────────────────────────

    #[test]
    fn vec2_abs() {
        let v = Vec2::new(-2.0, 3.0).abs();
        assert_eq!(v, Vec2::new(2.0, 3.0));
    }

    #[test]
    fn vec2_sign() {
        let v = Vec2::new(-5.0, 0.0).sign();
        assert_eq!(v.x, -1.0);
        // f32::signum(0.0) = 1.0 in Rust
        assert!((v.y - 0.0_f32.signum()).abs() < 1e-6);
    }

    #[test]
    fn vec2_floor_ceil_round() {
        let v = Vec2::new(1.6, -1.6);
        assert_eq!(v.floor(), Vec2::new(1.0, -2.0));
        assert_eq!(v.ceil(), Vec2::new(2.0, -1.0));
        assert_eq!(v.round(), Vec2::new(2.0, -2.0));
    }

    #[test]
    fn vec2_fract() {
        let f = Vec2::new(2.75, -1.25).fract();
        assert!((f.x - 0.75).abs() < 1e-6);
    }

    #[test]
    fn vec2_step() {
        let edge = Vec2::new(1.0, 2.0);
        let v = Vec2::new(0.5, 3.0);
        let s = v.step(edge);
        assert_eq!(s, Vec2::new(0.0, 1.0));
    }

    #[test]
    fn vec2_reflect() {
        let v = Vec2::new(1.0, -1.0);
        let n = Vec2::new(0.0, 1.0);
        let r = v.reflect(n);
        assert!((r.x - 1.0).abs() < 1e-6);
        assert!((r.y - 1.0).abs() < 1e-6);
    }

    // ── Vec3 component ops ────────────────────────────────────────────────────

    #[test]
    fn vec3_sign() {
        let v = Vec3::new(-3.0, 0.5, 0.0).sign();
        assert_eq!(v.x, -1.0);
        assert_eq!(v.y, 1.0);
    }

    #[test]
    fn vec3_floor_ceil_round_fract() {
        let v = Vec3::new(1.7, -1.3, 2.5);
        assert_eq!(v.floor(), Vec3::new(1.0, -2.0, 2.0));
        assert_eq!(v.ceil(), Vec3::new(2.0, -1.0, 3.0));
        assert!((v.fract().x - 0.7).abs() < 1e-5);
    }

    #[test]
    fn vec3_step() {
        let e = Vec3::new(1.0, 2.0, 3.0);
        let v = Vec3::new(0.5, 2.0, 5.0);
        let s = v.step(e);
        assert_eq!(s, Vec3::new(0.0, 1.0, 1.0));
    }

    // ── Vec4 component ops ────────────────────────────────────────────────────

    #[test]
    fn vec4_abs_sign() {
        let v = Vec4::new(-1.0, 2.0, -3.0, 0.0);
        let a = v.abs();
        assert_eq!(a, Vec4::new(1.0, 2.0, 3.0, 0.0));
        let s = v.sign();
        assert_eq!(s.x, -1.0);
        assert_eq!(s.y, 1.0);
        assert_eq!(s.z, -1.0);
    }

    #[test]
    fn vec4_floor_fract_roundtrip() {
        let v = Vec4::new(3.7, -0.3, 1.5, 2.9);
        let f = v.floor();
        let frac = v.fract();
        assert!((f.x + frac.x - v.x).abs() < 1e-5);
        assert!((f.y + frac.y - v.y).abs() < 1e-5);
    }

    // ── Spline tests ──────────────────────────────────────────────────────────

    #[test]
    fn bezier_quadratic_endpoints() {
        let p = bezier_quadratic(Vec3::ZERO, Vec3::new(0.5, 1.0, 0.0), Vec3::ONE, 0.0);
        assert!(p.length() < 1e-5);
        let p1 = bezier_quadratic(Vec3::ZERO, Vec3::new(0.5, 1.0, 0.0), Vec3::ONE, 1.0);
        assert!((p1 - Vec3::ONE).length() < 1e-5);
    }

    #[test]
    fn bezier_cubic_endpoints() {
        let a = Vec3::ZERO;
        let d = Vec3::new(3.0, 0.0, 0.0);
        let p0 = bezier_cubic(a, Vec3::X, Vec3::new(2.0, 1.0, 0.0), d, 0.0);
        assert!(p0.length() < 1e-5);
        let p1 = bezier_cubic(a, Vec3::X, Vec3::new(2.0, 1.0, 0.0), d, 1.0);
        assert!((p1 - d).length() < 1e-5);
    }

    #[test]
    fn bezier_cubic_tangent_endpoints() {
        // At t=0 tangent should be 3*(p1-p0)
        let p0 = Vec3::ZERO;
        let p1 = Vec3::X;
        let p2 = Vec3::new(2.0, 0.0, 0.0);
        let p3 = Vec3::new(3.0, 0.0, 0.0);
        let tang = bezier_cubic_tangent(p0, p1, p2, p3, 0.0);
        assert!((tang - Vec3::new(3.0, 0.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn catmull_rom_endpoints() {
        // Passes through p1 at t=0 and p2 at t=1
        let p = catmull_rom(
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::ZERO,
            Vec3::ONE,
            Vec3::new(2.0, 1.0, 0.0),
            0.0,
        );
        assert!(p.length() < 1e-5);
        let p1 = catmull_rom(
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::ZERO,
            Vec3::ONE,
            Vec3::new(2.0, 1.0, 0.0),
            1.0,
        );
        assert!((p1 - Vec3::ONE).length() < 1e-5);
    }

    #[test]
    fn hermite_endpoints() {
        let p = hermite(Vec3::ZERO, Vec3::X, Vec3::ONE, Vec3::X, 0.0);
        assert!(p.length() < 1e-5);
        let p1 = hermite(Vec3::ZERO, Vec3::X, Vec3::ONE, Vec3::X, 1.0);
        assert!((p1 - Vec3::ONE).length() < 1e-5);
    }

    #[test]
    fn hermite_zero_tangent_midpoint() {
        // With zero tangents, midpoint should be at 0.5 on each axis (symmetric)
        let p = hermite(Vec3::ZERO, Vec3::ZERO, Vec3::ONE, Vec3::ZERO, 0.5);
        assert!((p.x - 0.5).abs() < 1e-5);
    }

    // ── Mat4::unproject test ──────────────────────────────────────────────────

    #[test]
    fn unproject_ortho_center() {
        // Orthographic proj centered at origin — screen center should unproject along -Z
        let proj = Mat4::orthographic(-1.0, 1.0, -1.0, 1.0, 0.1, 100.0);
        let dir = Mat4::unproject(400.0, 300.0, 800, 600, &proj);
        // Ortho ray is axis-aligned; x and y should be near 0 at screen center
        assert!(dir.x.abs() < 1e-3, "expected x≈0, got {}", dir.x);
        assert!(dir.y.abs() < 1e-3, "expected y≈0, got {}", dir.y);
    }

    // ── Mat4::row ─────────────────────────────────────────────────────────────

    #[test]
    fn mat4_row_identity() {
        let m = Mat4::identity();
        assert_eq!(m.row(0), Vec4::new(1.0, 0.0, 0.0, 0.0));
        assert_eq!(m.row(1), Vec4::new(0.0, 1.0, 0.0, 0.0));
        assert_eq!(m.row(3), Vec4::new(0.0, 0.0, 0.0, 1.0));
    }

    #[test]
    fn mat4_row_translation() {
        let m = Mat4::translation(5.0, -3.0, 7.0);
        // Translation is in row 3 in row-vector convention
        let r3 = m.row(3);
        assert!((r3.x - 5.0).abs() < 1e-5);
        assert!((r3.y - (-3.0)).abs() < 1e-5);
        assert!((r3.z - 7.0).abs() < 1e-5);
    }

    // ── Vec smoothstep ────────────────────────────────────────────────────────

    #[test]
    fn vec2_smoothstep_endpoints() {
        let e0 = Vec2::ZERO;
        let e1 = Vec2::ONE;
        assert_eq!(Vec2::ZERO.smoothstep(e0, e1), Vec2::ZERO);
        assert_eq!(Vec2::ONE.smoothstep(e0, e1), Vec2::ONE);
    }

    #[test]
    fn vec2_smoothstep_midpoint() {
        let v = Vec2::splat(0.5).smoothstep(Vec2::ZERO, Vec2::ONE);
        assert!((v.x - 0.5).abs() < 1e-5);
    }

    #[test]
    fn vec3_smoothstep_endpoints() {
        let e0 = Vec3::ZERO;
        let e1 = Vec3::ONE;
        let at_zero = Vec3::ZERO.smoothstep(e0, e1);
        let at_one = Vec3::ONE.smoothstep(e0, e1);
        assert!(at_zero.length() < 1e-5);
        assert!((at_one - Vec3::ONE).length() < 1e-5);
    }

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
            "W mismatch: {simd_w} vs {scalar_w}"
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
            "NDZ z at near should be -1.0, got {z_ndc_near}"
        );

        let p_far = Vec3::new(0.0, 0.0, -far);
        let (p_far_prime, w_far) = proj.transform_point(p_far);
        assert!((w_far - far).abs() < 1e-5, "w at far plane should be far");
        // In standard GL, z_ndc at far is 1.0
        let z_ndc_far = p_far_prime.z / w_far;
        assert!(
            (z_ndc_far - 1.0).abs() < 1e-5,
            "NDC z at far should be 1.0, got {z_ndc_far}"
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
    #[allow(clippy::float_cmp)]
    fn test_project_to_screen_optimized_edge_cases() {
        let half_width = 400.0;
        let half_height = 300.0;

        // Test w = 0 (singular)
        // Code falls back to 1.0 if w.abs() <= 0.0001
        let p = Vec3::new(100.0, 100.0, 10.0);
        let sp = project_to_screen_optimized(p, 0.0, half_width, half_height);

        // Expected behavior: inv_w = 1.0, so x = 100.0, y = 100.0
        // ndc_x = 100.0. screen_x = (100+1)*400 = 40400.
        assert!((sp.inv_w - 1.0).abs() < f32::EPSILON);
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
        assert!((sp_neg.inv_w - -1.0).abs() < f32::EPSILON);
        assert_eq!(sp_neg.x, 0);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_vec3_normalize_zero() {
        let v = Vec3::new(0.0, 0.0, 0.0);
        let n = v.normalize();
        assert!((n.x - 0.0).abs() < f32::EPSILON);
        assert!((n.y - 0.0).abs() < f32::EPSILON);
        assert!((n.z - 0.0).abs() < f32::EPSILON);

        let v_small = Vec3::new(1e-5, 0.0, 0.0);
        let n_small = v_small.normalize();
        // Should return original if length < 0.0001
        assert!((n_small.x - 1e-5).abs() < f32::EPSILON);
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
    fn test_mat2_rotation() {
        use std::f32::consts::FRAC_PI_2;
        let m = Mat2::rotation(FRAC_PI_2);
        // cos(90) is approx 0, sin(90) is 1
        assert!(m.m[0][0].abs() < 1e-6);
        assert!((m.m[0][1] - (-1.0)).abs() < 1e-6);
        assert!((m.m[1][0] - 1.0).abs() < 1e-6);
        assert!(m.m[1][1].abs() < 1e-6);
    }

    #[test]
    fn test_mat2_transform() {
        use std::f32::consts::FRAC_PI_2;
        let m = Mat2::rotation(FRAC_PI_2);
        let v = Vec2::new(1.0, 0.0);
        let result = m.transform(v);
        // (1, 0) rotated 90 deg -> (0, 1)
        assert!(result.x.abs() < 1e-6);
        assert!((result.y - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_mat2_transform_batch() {
        use std::f32::consts::PI;
        let m = Mat2::rotation(PI);
        let vertices = vec![Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)];
        let result = m.transform_batch(&vertices);
        // Rotate 180 degrees -> (-x, -y)
        assert!((result[0].x - (-1.0)).abs() < 1e-6);
        assert!(result[0].y.abs() < 1e-6);
        assert!(result[1].x.abs() < 1e-6);
        assert!((result[1].y - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_mat2_transform_in_place() {
        use std::f32::consts::PI;
        let m = Mat2::rotation(PI);
        let mut vertices = vec![Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)];
        m.transform_in_place(&mut vertices);
        // Rotate 180 degrees -> (-x, -y)
        assert!((vertices[0].x - (-1.0)).abs() < 1e-6);
        assert!(vertices[0].y.abs() < 1e-6);
        assert!(vertices[1].x.abs() < 1e-6);
        assert!((vertices[1].y - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_vec2_rotate() {
        let rotated = Vec2::new(1.0, 0.0).rotate(std::f32::consts::FRAC_PI_2);
        assert!(rotated.x.abs() < 0.02);
        assert!((rotated.y - 1.0).abs() < 0.02);
    }

    #[test]
    fn test_vec4_new() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        assert!((v.x - 1.0).abs() < f32::EPSILON);
        assert!((v.y - 2.0).abs() < f32::EPSILON);
        assert!((v.z - 3.0).abs() < f32::EPSILON);
        assert!((v.w - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_vec4_add() {
        let v1 = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let v2 = Vec4::new(5.0, 6.0, 7.0, 8.0);
        let result = v1 + v2;
        assert!((result.x - 6.0).abs() < f32::EPSILON);
        assert!((result.y - 8.0).abs() < f32::EPSILON);
        assert!((result.z - 10.0).abs() < f32::EPSILON);
        assert!((result.w - 12.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_vec4_sub() {
        let v1 = Vec4::new(5.0, 6.0, 7.0, 8.0);
        let v2 = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let result = v1 - v2;
        assert!((result.x - 4.0).abs() < f32::EPSILON);
        assert!((result.y - 4.0).abs() < f32::EPSILON);
        assert!((result.z - 4.0).abs() < f32::EPSILON);
        assert!((result.w - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_vec3_reflect() {
        let v = Vec3::new(1.0, -1.0, 0.0);
        let normal = Vec3::new(0.0, 1.0, 0.0);
        let r = v.reflect(normal);
        assert!((r.x - 1.0).abs() < 1e-6);
        assert!((r.y - 1.0).abs() < 1e-6);
        assert!(r.z.abs() < 1e-6);
    }

    #[test]
    fn test_vec3_reflect_normalized_matches_reflect() {
        let v = Vec3::new(0.25, -0.5, 1.2);
        let n = Vec3::new(0.0, 1.0, 0.0);
        let a = v.reflect(n);
        let b = v.reflect_normalized(n);
        assert!((a.x - b.x).abs() < 1e-6);
        assert!((a.y - b.y).abs() < 1e-6);
        assert!((a.z - b.z).abs() < 1e-6);
    }

    #[test]
    fn test_vec3_refract_air_to_glass() {
        // 45-degree incidence from air to glass.
        let incident = Vec3::new(1.0, -1.0, 0.0).normalize();
        let normal = Vec3::new(0.0, 1.0, 0.0);
        let refracted = incident.refract(normal, 1.0 / 1.5);

        // Should still travel downward, bent toward the normal.
        assert!(refracted.y < 0.0);
        assert!(refracted.length() > 0.99 && refracted.length() < 1.01);
    }

    #[test]
    fn test_vec3_refract_total_internal_reflection() {
        // Steep angle from dense to sparse medium should TIR.
        let incident = Vec3::new(1.0, -0.1, 0.0).normalize();
        let normal = Vec3::new(0.0, 1.0, 0.0);
        let refracted = incident.refract(normal, 1.5);
        assert_eq!(refracted, Vec3::ZERO);
    }

    #[test]
    fn test_vec3_face_forward() {
        let n = Vec3::new(0.0, 1.0, 0.0);
        let i_towards = Vec3::new(0.0, -1.0, 0.0);
        let i_away = Vec3::new(0.0, 1.0, 0.0);
        let nref = Vec3::new(0.0, 1.0, 0.0);

        assert_eq!(n.face_forward(i_towards, nref), n);
        assert_eq!(n.face_forward(i_away, nref), n * -1.0);
    }

    #[test]
    fn test_mat4_orthographic() {
        let proj = Mat4::orthographic(-10.0, 10.0, -5.0, 5.0, 0.1, 100.0);

        let p_center = Vec3::new(0.0, 0.0, -50.0);
        let (p_center_prime, w_center) = proj.transform_point(p_center);
        assert!((w_center - 1.0).abs() < 1e-5);
        assert!(p_center_prime.x.abs() < 1e-5);
        assert!(p_center_prime.y.abs() < 1e-5);

        // Orthographic projection preserves W as 1.0
        // -50 in Z should map between -1 and 1 in NDC
        let z_ndc = p_center_prime.z / w_center;
        assert!(z_ndc >= -1.0 && z_ndc <= 1.0);
    }

    #[test]
    fn test_vec4_mul_scalar() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let result = v * 2.5;
        assert!((result.x - 2.5).abs() < f32::EPSILON);
        assert!((result.y - 5.0).abs() < f32::EPSILON);
        assert!((result.z - 7.5).abs() < f32::EPSILON);
        assert!((result.w - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_vec4_dot_length_normalize() {
        let a = Vec4::new(1.0, 2.0, 2.0, 1.0);
        let b = Vec4::new(-1.0, 0.5, 3.0, 2.0);
        assert!((a.dot(b) - 8.0).abs() < 1e-6);
        assert!((a.length_sq() - 10.0).abs() < 1e-6);
        assert!((a.length() - 10.0_f32.sqrt()).abs() < 1e-6);

        let n = a.normalize();
        assert!((n.length() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_vec4_project_and_reject() {
        let v = Vec4::new(3.0, 4.0, 0.0, 0.0);
        let onto = Vec4::new(1.0, 0.0, 0.0, 0.0);
        let proj = v.project_onto(onto);
        let rej = v.reject_from(onto);

        assert!((proj.x - 3.0).abs() < 1e-6);
        assert!(proj.y.abs() < 1e-6);
        assert!(proj.z.abs() < 1e-6);
        assert!(proj.w.abs() < 1e-6);

        assert!(rej.x.abs() < 1e-6);
        assert!((rej.y - 4.0).abs() < 1e-6);
        assert!(rej.z.abs() < 1e-6);
        assert!(rej.w.abs() < 1e-6);
    }

    #[test]
    fn test_vec4_min_max_clamp_distance() {
        let a = Vec4::new(-1.0, 3.0, 10.0, 0.5);
        let b = Vec4::new(2.0, 1.0, 7.0, 2.0);

        let min = a.min(b);
        let max = a.max(b);
        assert_eq!(min, Vec4::new(-1.0, 1.0, 7.0, 0.5));
        assert_eq!(max, Vec4::new(2.0, 3.0, 10.0, 2.0));

        let clamped = Vec4::new(3.0, 0.0, 8.0, 1.5).clamp(min, max);
        assert_eq!(clamped, Vec4::new(2.0, 1.0, 8.0, 1.5));

        assert!((a.distance_sq(b) - 24.25).abs() < 1e-6);
        assert!((a.distance(b) - 24.25_f32.sqrt()).abs() < 1e-6);
    }

    #[test]
    fn test_vec3_min_max() {
        let a = Vec3::new(1.0, 5.0, -2.0);
        let b = Vec3::new(3.0, 2.0, -1.0);

        let min = a.min(b);
        assert!((min.x - 1.0).abs() < f32::EPSILON);
        assert!((min.y - 2.0).abs() < f32::EPSILON);
        assert!((min.z - -2.0).abs() < f32::EPSILON);

        let max = a.max(b);
        assert!((max.x - 3.0).abs() < f32::EPSILON);
        assert!((max.y - 5.0).abs() < f32::EPSILON);
        assert!((max.z - -1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_vec3_orthonormal_basis() {
        let n = Vec3::new(0.3, 0.5, 0.8).normalize();
        let (t, b) = n.orthonormal_basis();

        assert!((t.length() - 1.0).abs() < 1e-4);
        assert!((b.length() - 1.0).abs() < 1e-4);
        assert!(n.dot(t).abs() < 1e-4);
        assert!(n.dot(b).abs() < 1e-4);
        assert!(t.dot(b).abs() < 1e-4);
    }

    #[test]
    fn test_vec3_orthonormal_basis_degenerate_input() {
        let (t, b) = Vec3::ZERO.orthonormal_basis();
        assert!((t.length() - 1.0).abs() < 1e-4);
        assert!((b.length() - 1.0).abs() < 1e-4);
        assert!(t.dot(b).abs() < 1e-4);
    }

    #[test]
    fn test_vec3_slerp_midpoint() {
        let x = Vec3::new(1.0, 0.0, 0.0);
        let y = Vec3::new(0.0, 1.0, 0.0);
        let mid = x.slerp(y, 0.5);
        let inv_sqrt2 = 1.0 / 2.0_f32.sqrt();
        assert!((mid.x - inv_sqrt2).abs() < 1e-4);
        assert!((mid.y - inv_sqrt2).abs() < 1e-4);
        assert!(mid.z.abs() < 1e-4);
        assert!((mid.length() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_vec3_project_onto_normalized_matches_regular_projection() {
        let v = Vec3::new(3.0, 4.0, 5.0);
        let unit = Vec3::new(2.0, -1.0, 3.0).normalize();
        let a = v.project_onto(unit);
        let b = v.project_onto_normalized(unit);
        assert!((a.x - b.x).abs() < 1e-5);
        assert!((a.y - b.y).abs() < 1e-5);
        assert!((a.z - b.z).abs() < 1e-5);
    }

    #[test]
    fn test_vec3_clamp_length() {
        let v = Vec3::new(3.0, 4.0, 0.0);
        let clamped = v.clamp_length(2.0);
        assert!((clamped.length() - 2.0).abs() < 1e-4);

        let unchanged = v.clamp_length(10.0);
        assert!((unchanged.x - v.x).abs() < 1e-6);
        assert!((unchanged.y - v.y).abs() < 1e-6);
        assert!((unchanged.z - v.z).abs() < 1e-6);
    }

    #[test]
    fn test_vec3_is_finite() {
        assert!(Vec3::new(1.0, -2.0, 3.0).is_finite());
        assert!(!Vec3::new(f32::INFINITY, 0.0, 0.0).is_finite());
        assert!(!Vec3::new(0.0, f32::NAN, 0.0).is_finite());
    }

    #[test]
    fn test_mat4_rotation_axis_matches_rotation_y() {
        use std::f32::consts::FRAC_PI_2;
        let rot_axis = Mat4::rotation_axis(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);
        let rot_y = Mat4::rotation_y(FRAC_PI_2);
        let v = Vec3::new(1.0, 0.0, 0.0);
        let (a, _) = rot_axis.transform_point(v);
        let (b, _) = rot_y.transform_point(v);
        assert!((a.x - b.x).abs() < 1e-5);
        assert!((a.y - b.y).abs() < 1e-5);
        assert!((a.z - b.z).abs() < 1e-5);
    }

    #[test]
    fn test_mat4_transform_vector_ignores_translation() {
        let m = Mat4::rotation_z(1.0) * Mat4::translation(10.0, 20.0, 30.0);
        let v = Vec3::new(2.0, -1.0, 3.0);
        let transformed = m.transform_vector(v);
        let (point_transformed, _) = m.transform_point(v);
        let translated_delta = point_transformed - transformed;
        assert!((translated_delta.x - 10.0).abs() < 1e-4);
        assert!((translated_delta.y - 20.0).abs() < 1e-4);
        assert!((translated_delta.z - 30.0).abs() < 1e-4);
    }

    #[test]
    fn test_vec3_barycentric_roundtrip() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(2.0, 0.0, 0.0);
        let c = Vec3::new(0.0, 2.0, 0.0);
        let p = Vec3::new(0.5, 0.75, 0.0);

        let bary = p.barycentric_coordinates(a, b, c).unwrap();
        let reconstructed = Vec3::from_barycentric(a, b, c, bary);

        assert!((bary.x + bary.y + bary.z - 1.0).abs() < 1e-5);
        assert!((reconstructed.x - p.x).abs() < 1e-5);
        assert!((reconstructed.y - p.y).abs() < 1e-5);
        assert!((reconstructed.z - p.z).abs() < 1e-5);
    }

    #[test]
    fn test_vec3_barycentric_degenerate_triangle_returns_none() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(1.0, 1.0, 1.0);
        let c = Vec3::new(2.0, 2.0, 2.0);
        let p = Vec3::new(0.2, 0.4, 0.6);
        assert!(p.barycentric_coordinates(a, b, c).is_none());
    }

    #[test]
    fn test_mat4_transform_vectors_batch_matches_scalar() {
        let m = Mat4::rotation_y(0.37) * Mat4::rotation_x(-0.22) * Mat4::translation(4.0, 5.0, 6.0);
        let input = vec![
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(-1.0, 0.5, 0.0),
            Vec3::new(0.0, -3.0, 2.0),
        ];
        let mut output = vec![Vec3::ZERO; input.len()];
        m.transform_vectors(&input, &mut output);

        for (i, v) in input.iter().enumerate() {
            let scalar = m.transform_vector(*v);
            assert!((scalar.x - output[i].x).abs() < 1e-5);
            assert!((scalar.y - output[i].y).abs() < 1e-5);
            assert!((scalar.z - output[i].z).abs() < 1e-5);
        }
    }

    #[test]
    fn test_mat4_transform_points_affine_matches_transform_point() {
        let m =
            Mat4::scale(2.0, 3.0, 4.0) * Mat4::rotation_z(0.5) * Mat4::translation(8.0, -2.0, 1.0);
        let input = vec![
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(-4.0, 1.5, 0.25),
            Vec3::new(0.0, 0.0, 0.0),
        ];
        let mut output = vec![Vec3::ZERO; input.len()];
        m.transform_points_affine(&input, &mut output);

        for (i, p) in input.iter().enumerate() {
            let scalar = m.transform_point(*p).0;
            assert!((scalar.x - output[i].x).abs() < 1e-5);
            assert!((scalar.y - output[i].y).abs() < 1e-5);
            assert!((scalar.z - output[i].z).abs() < 1e-5);
        }
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
                assert!(s_tri_0.z.is_nan(), "Z NaN mismatch for case: {name}");
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
                assert!(s_tri_0.inv_w.is_nan(), "InvW NaN mismatch for case: {name}");
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
