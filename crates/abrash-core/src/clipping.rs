#![allow(dead_code)]
#![allow(clippy::suboptimal_flops)]
#![allow(clippy::should_panic_without_expect)]
#![allow(clippy::ignore_without_reason)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::float_cmp)]
//! # 3D View Frustum Clipping ✂️
//!
//! This module ensures we only render what the camera can actually see.
//!
//! Without clipping, vertices behind the camera (where $Z < 0$ or $W < 0$) would project
//! incorrectly onto the screen, causing bizarre mirroring and infinite lines. Furthermore,
//! rasterizing geometry that falls far outside the screen bounds wastes precious CPU cycles.
//!
//! ## The Sutherland-Hodgman Algorithm
//!
//! We employ the Sutherland-Hodgman algorithm in Homogeneous Clip Space. The space is defined
//! by six planes: Left, Right, Top, Bottom, Near, and Far.
//!
//! For each plane, the algorithm checks if a vertex is "inside" or "outside":
//! *   **Left Plane**: $x \ge -w$
//! *   **Right Plane**: $x \le w$
//! *   **Top Plane**: $y \le w$
//! *   **Bottom Plane**: $y \ge -w$
//! *   **Near Plane**: $z \ge -w$
//! *   **Far Plane**: $z \le w$
//!
//! When an edge crosses a plane, we compute the exact intersection point using linear interpolation
//! (see the `Lerp` trait) and insert a new vertex. This turns a single triangle into a convex polygon
//! with up to 9 vertices, which is then fan-triangulated back into a list of triangles.

use std::mem::MaybeUninit;
use std::ops::Index;

use crate::math::Vec3;

/// A list of triangles resulting from clipping.
///
/// This struct uses `MaybeUninit` to avoid the overhead of initializing a fixed-size array
/// with dummy values, as clipping is a hot path in the pipeline.
///
/// # Safety
///
/// The `tris` array is partially initialized up to `count`. Accessing elements beyond `count`
/// is Undefined Behavior. The implementation of `Index` performs bounds checking to ensure safety.
pub struct ClippedTriangles<V> {
    tris: [MaybeUninit<V>; 24], // Max 8 triangles = 24 vertices
    /// Number of triangles currently in the list
    count: usize,
}

impl<V> ClippedTriangles<V> {
    // Unsafe because it returns uninitialized data structure
    const fn new_uninit() -> Self {
        Self {
            tris: [const { MaybeUninit::uninit() }; 24],
            count: 0,
        }
    }

    /// Returns the number of clipped triangles.
    pub const fn count(&self) -> usize {
        self.count
    }
}

impl<V> Index<usize> for ClippedTriangles<V> {
    type Output = V;

    fn index(&self, index: usize) -> &Self::Output {
        // We only allow accessing elements that have been marked as valid by `count`.
        // Each triangle has 3 vertices.
        debug_assert!(
            index < self.count * 3,
            "Index {} out of bounds for count {}",
            index,
            self.count
        );
        unsafe { self.tris[index].assume_init_ref() }
    }
}

const NEAR: f32 = 0.001;

/// Clip a triangle against the view frustum (6 planes) in Homogeneous Clip Space.
///
/// Returns a list of triangles (fan triangulation of the clipped polygon).
///
/// # Algorithm
///
/// This implements the **Sutherland-Hodgman Algorithm**.
/// The algorithm works by clipping the polygon against each of the 6 frustum planes in sequence.
///
/// 1.  Start with the input triangle.
/// 2.  Clip against Plane 1. Output is a polygon (triangle or quad).
/// 3.  Clip that polygon against Plane 2. Output is a polygon...
///     ...
/// 7.  Clip against Plane 6.
///
/// The final result is a convex polygon (potentially with many vertices), which is then
/// triangulated into a triangle fan for rasterization.
///
/// # Examples
///
/// ```
/// use abrash_core::clipping::clip_triangle_to_frustum;
/// use abrash_core::math::Vec3;
///
/// // Create a triangle where one vertex is behind the Near Plane (z < -w)
/// let w = 1.0;
/// // Inside
/// let v0 = (Vec3::new(0.0, 0.0, 0.5), w);
/// // Inside
/// let v1 = (Vec3::new(0.5, 0.5, 0.5), w);
/// // Outside Near Plane! (z = -2.0 < -1.0)
/// let v2 = (Vec3::new(0.0, 0.0, -2.0), w);
///
/// let get_pos = |v: &(Vec3, f32)| *v;
/// let result = clip_triangle_to_frustum(v0, v1, v2, get_pos, |a, b, t| (a.0.lerp(b.0, t), a.1 + (b.1 - a.1) * t));
///
/// // The triangle is clipped into a quad, which is triangulated into 2 triangles.
/// assert_eq!(result.count(), 2);
/// ```
pub fn clip_triangle_to_frustum<V: Copy>(
    v0: V,
    v1: V,
    v2: V,
    get_pos: impl Fn(&V) -> (Vec3, f32),
    lerp: impl Fn(V, V, f32) -> V,
) -> ClippedTriangles<V> {
    // Optimization: Trivial Accept/Reject
    // Check if all vertices are inside all planes (Accept) or all outside one plane (Reject)
    let (p0, w0) = get_pos(&v0);
    let (p1, w1) = get_pos(&v1);
    let (p2, w2) = get_pos(&v2);

    let mut active_planes = 0u8;

    #[cfg(target_arch = "x86_64")]
    unsafe {
        use std::arch::x86_64::{
            _mm_and_ps, _mm_cmpge_ps, _mm_cmple_ps, _mm_movemask_ps, _mm_set_ps, _mm_setzero_ps,
            _mm_sub_ps,
        };
        // Layout: [v2, v1, v0, pad] or [v0, v1, v2, pad]?
        // _mm_set_ps(e3, e2, e1, e0) -> [e0, e1, e2, e3]
        // We want lanes 0, 1, 2 to correspond to v0, v1, v2.
        // So we should use _mm_set_ps(pad, v2, v1, v0).

        let vx = _mm_set_ps(0.0, p2.x, p1.x, p0.x);
        let vy = _mm_set_ps(0.0, p2.y, p1.y, p0.y);
        let vz = _mm_set_ps(0.0, p2.z, p1.z, p0.z);
        let vw = _mm_set_ps(1.0, w2, w1, w0);
        let neg_vw = _mm_sub_ps(_mm_setzero_ps(), vw);

        // Plane checks
        // 1. Left: x >= -w
        let m_left = _mm_cmpge_ps(vx, neg_vw);
        // 2. Right: x <= w
        let m_right = _mm_cmple_ps(vx, vw);
        // 3. Bottom: y >= -w
        let m_bottom = _mm_cmpge_ps(vy, neg_vw);
        // 4. Top: y <= w
        let m_top = _mm_cmple_ps(vy, vw);
        // 5. Near: z >= -w
        let m_near = _mm_cmpge_ps(vz, neg_vw);
        // 6. Far: z <= w
        let m_far = _mm_cmple_ps(vz, vw);

        // Trivial Accept: All vertices inside all planes
        // Combine all masks
        let all_planes = _mm_and_ps(
            _mm_and_ps(_mm_and_ps(m_left, m_right), _mm_and_ps(m_bottom, m_top)),
            _mm_and_ps(m_near, m_far),
        );

        // Check if lower 3 bits are set (bits 0, 1, 2)
        if (_mm_movemask_ps(all_planes) & 0x7) == 0x7 {
            let mut result = ClippedTriangles::new_uninit();
            result.tris[0].write(v0);
            result.tris[1].write(v1);
            result.tris[2].write(v2);
            result.count = 1;
            return result;
        }

        // Trivial Reject & Active Plane Detection
        let mask_left = _mm_movemask_ps(m_left) & 0x7;
        let mask_right = _mm_movemask_ps(m_right) & 0x7;
        let mask_bottom = _mm_movemask_ps(m_bottom) & 0x7;
        let mask_top = _mm_movemask_ps(m_top) & 0x7;
        let mask_near = _mm_movemask_ps(m_near) & 0x7;
        let mask_far = _mm_movemask_ps(m_far) & 0x7;

        if mask_left == 0
            || mask_right == 0
            || mask_bottom == 0
            || mask_top == 0
            || mask_near == 0
            || mask_far == 0
        {
            return ClippedTriangles::new_uninit();
        }

        if mask_left != 7 {
            active_planes |= 1;
        }
        if mask_right != 7 {
            active_planes |= 2;
        }
        if mask_bottom != 7 {
            active_planes |= 4;
        }
        if mask_top != 7 {
            active_planes |= 8;
        }
        if mask_near != 7 {
            active_planes |= 16;
        }
        if mask_far != 7 {
            active_planes |= 32;
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        // Unrolled inside mask check
        let mut m0 = 0;
        if p0.x >= -w0 {
            m0 |= 1;
        }
        if p0.x <= w0 {
            m0 |= 2;
        }
        if p0.y >= -w0 {
            m0 |= 4;
        }
        if p0.y <= w0 {
            m0 |= 8;
        }
        if p0.z >= -w0 {
            m0 |= 16;
        }
        if p0.z <= w0 {
            m0 |= 32;
        }

        let mut m1 = 0;
        if p1.x >= -w1 {
            m1 |= 1;
        }
        if p1.x <= w1 {
            m1 |= 2;
        }
        if p1.y >= -w1 {
            m1 |= 4;
        }
        if p1.y <= w1 {
            m1 |= 8;
        }
        if p1.z >= -w1 {
            m1 |= 16;
        }
        if p1.z <= w1 {
            m1 |= 32;
        }

        let mut m2 = 0;
        if p2.x >= -w2 {
            m2 |= 1;
        }
        if p2.x <= w2 {
            m2 |= 2;
        }
        if p2.y >= -w2 {
            m2 |= 4;
        }
        if p2.y <= w2 {
            m2 |= 8;
        }
        if p2.z >= -w2 {
            m2 |= 16;
        }
        if p2.z <= w2 {
            m2 |= 32;
        }

        let all_in = m0 & m1 & m2;
        if all_in == 0x3F {
            // Trivial Accept: All inside
            let mut result = ClippedTriangles::new_uninit();
            result.tris[0].write(v0);
            result.tris[1].write(v1);
            result.tris[2].write(v2);
            result.count = 1;
            return result;
        }

        let any_in = m0 | m1 | m2;
        if any_in != 0x3F {
            // Trivial Reject: All outside at least one plane
            return ClippedTriangles::new_uninit();
        }

        // Active Plane Detection
        if (all_in & 1) == 0 {
            active_planes |= 1;
        }
        if (all_in & 2) == 0 {
            active_planes |= 2;
        }
        if (all_in & 4) == 0 {
            active_planes |= 4;
        }
        if (all_in & 8) == 0 {
            active_planes |= 8;
        }
        if (all_in & 16) == 0 {
            active_planes |= 16;
        }
        if (all_in & 32) == 0 {
            active_planes |= 32;
        }
    }

    // Double buffering for vertex lists
    // A triangle clipped by 6 planes can have at most 9 vertices (usually).
    // We use a safe upper bound of 12 for the polygon vertices.
    // SAFETY: Arrays of MaybeUninit do not require initialization.
    let mut buf1: [MaybeUninit<V>; 12] = [const { MaybeUninit::uninit() }; 12];
    let mut buf2: [MaybeUninit<V>; 12] = [const { MaybeUninit::uninit() }; 12];

    // Initialize input buffer
    buf1[0].write(v0);
    buf1[1].write(v1);
    buf1[2].write(v2);
    let mut count = 3;

    // Macro to handle clipping logic for a plane
    // Reads from $buf_in, writes to $buf_out
    macro_rules! clip_plane {
        ($buf_in:expr, $buf_out:expr, $dist_fn:expr) => {
            if count > 0 {
                let mut out_count = 0;
                let prev_idx = count - 1;
                // SAFETY: We only read up to `count`, which are initialized.
                let mut prev_v = unsafe { $buf_in[prev_idx].assume_init() };
                let (prev_pos, prev_w) = get_pos(&prev_v);
                let mut prev_d = $dist_fn(prev_pos, prev_w);

                for i in 0..count {
                    // SAFETY: i < count
                    let curr_v = unsafe { $buf_in[i].assume_init() };
                    let (curr_pos, curr_w) = get_pos(&curr_v);
                    let curr_d = $dist_fn(curr_pos, curr_w);

                    if curr_d >= 0.0 {
                        // Current is inside
                        if prev_d < 0.0 {
                            // Entered: add intersection
                            let t = prev_d / (prev_d - curr_d);
                            if out_count < 12 {
                                $buf_out[out_count].write(lerp(prev_v, curr_v, t));
                                out_count += 1;
                            }
                        }
                        // Add current
                        if out_count < 12 {
                            $buf_out[out_count].write(curr_v);
                            out_count += 1;
                        }
                    } else {
                        // Current is outside
                        if prev_d >= 0.0 {
                            // Exited: add intersection
                            let t = prev_d / (prev_d - curr_d);
                            if out_count < 12 {
                                $buf_out[out_count].write(lerp(prev_v, curr_v, t));
                                out_count += 1;
                            }
                        }
                    }

                    prev_v = curr_v;
                    prev_d = curr_d;
                }
                count = out_count;
            }
        };
    }

    // Dynamic buffer selection state
    let mut input_is_buf1 = true;

    // Helper to run clipping if plane is active
    macro_rules! run_clip {
        ($plane_bit:expr, $dist_fn:expr) => {
            if (active_planes & $plane_bit) != 0 {
                if input_is_buf1 {
                    clip_plane!(buf1, buf2, $dist_fn);
                } else {
                    clip_plane!(buf2, buf1, $dist_fn);
                }
                input_is_buf1 = !input_is_buf1;
            }
        };
    }

    // 1. Left: x >= -w -> x + w >= 0
    run_clip!(1, |p: Vec3, w: f32| p.x + w);

    // 2. Right: x <= w -> w - x >= 0
    run_clip!(2, |p: Vec3, w: f32| w - p.x);

    // 3. Bottom: y >= -w -> y + w >= 0
    run_clip!(4, |p: Vec3, w: f32| p.y + w);

    // 4. Top: y <= w -> w - y >= 0
    run_clip!(8, |p: Vec3, w: f32| w - p.y);

    // 5. Near: z >= -w -> z + w >= 0
    run_clip!(16, |p: Vec3, w: f32| p.z + w);

    // 6. Far: z <= w -> w - z >= 0
    run_clip!(32, |p: Vec3, w: f32| w - p.z);

    // Result is in the buffer indicated by input_is_buf1
    let final_buf = if input_is_buf1 { &buf1 } else { &buf2 };

    // Triangulate (Fan)
    let mut result = ClippedTriangles::new_uninit();

    if count >= 3 {
        // Pivot vertex
        // SAFETY: count >= 3, so final_buf[0] is initialized
        let pivot = unsafe { final_buf[0].assume_init() };
        // Generate triangles: (0, 1, 2), (0, 2, 3), (0, 3, 4), ...
        // Number of triangles = count - 2

        for i in 1..count - 1 {
            if result.count < 8 {
                let idx = result.count * 3;
                // SAFETY: i < count-1, so i and i+1 are within bounds and initialized
                let v1 = unsafe { final_buf[i].assume_init() };
                let v2 = unsafe { final_buf[i + 1].assume_init() };
                result.tris[idx].write(pivot);
                result.tris[idx + 1].write(v1);
                result.tris[idx + 2].write(v2);
                result.count += 1;
            }
        }
    }

    result
}

/// Clip a line segment against the view frustum (6 planes) in Homogeneous Clip Space.
///
/// Returns `Some((v0, v1))` if the line is partially or fully visible, `None` if fully culled.
///
/// # Examples
///
/// ```
/// use abrash_core::clipping::clip_line_to_frustum;
/// use abrash_core::math::Vec3;
///
/// let w = 10.0;
/// // Start point is inside
/// let v0 = (Vec3::new(5.0, 0.0, 0.0), w);
/// // End point is outside the Right Plane (x > w)
/// let v1 = (Vec3::new(15.0, 0.0, 0.0), w);
///
/// let get_pos = |v: &(Vec3, f32)| *v;
/// let clipped = clip_line_to_frustum(v0, v1, get_pos, |a, b, t| (a.0.lerp(b.0, t), a.1 + (b.1 - a.1) * t)).unwrap();
///
/// // The start point remains the same
/// assert_eq!(clipped.0, v0);
///
/// // The end point is clipped exactly to the Right Plane (x = w = 10.0)
/// assert_eq!((clipped.1).0.x, 10.0);
/// ```
pub fn clip_line_to_frustum<V: Copy>(
    v0: V,
    v1: V,
    get_pos: impl Fn(&V) -> (Vec3, f32),
    lerp: impl Fn(V, V, f32) -> V,
) -> Option<(V, V)> {
    let mut curr_v0 = v0;
    let mut curr_v1 = v1;

    // Helper macro to clip against a single plane
    // If a point is outside (dist < 0), we find the intersection.
    macro_rules! clip_plane {
        ($dist_fn:expr) => {
            let (p0, w0) = get_pos(&curr_v0);
            let (p1, w1) = get_pos(&curr_v1);
            let d0 = $dist_fn(p0, w0);
            let d1 = $dist_fn(p1, w1);

            if d0 >= 0.0 && d1 >= 0.0 {
                // Both inside, do nothing
            } else if d0 < 0.0 && d1 < 0.0 {
                // Both outside, cull entire line
                return None;
            } else {
                // One in, one out. Clip.
                let t = d0 / (d0 - d1);
                let intersection = lerp(curr_v0, curr_v1, t);

                if d0 < 0.0 {
                    // v0 is outside, replace v0
                    curr_v0 = intersection;
                } else {
                    // v1 is outside, replace v1
                    curr_v1 = intersection;
                }
            }
        };
    }

    // 1. Left: x >= -w -> x + w >= 0
    clip_plane!(|p: Vec3, w: f32| p.x + w);

    // 2. Right: x <= w -> w - x >= 0
    clip_plane!(|p: Vec3, w: f32| w - p.x);

    // 3. Bottom: y >= -w -> y + w >= 0
    clip_plane!(|p: Vec3, w: f32| p.y + w);

    // 4. Top: y <= w -> w - y >= 0
    clip_plane!(|p: Vec3, w: f32| w - p.y);

    // 5. Near: z >= -w -> z + w >= 0
    clip_plane!(|p: Vec3, w: f32| p.z + w);

    // 6. Far: z <= w -> w - z >= 0
    clip_plane!(|p: Vec3, w: f32| w - p.z);

    Some((curr_v0, curr_v1))
}

/// Clips a single triangle against the camera's near plane (w = 0.001).
///
/// Ensures vertices behind the camera do not project to invalid screen coordinates.
/// Returns either 0, 1, or 2 new triangles that make up the visible portion.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec4;
/// use abrash_core::clipping::clip_triangle_against_near_plane;
///
/// let v0 = Vec4::new(0.0, 0.0, -1.0, -1.0); // Behind camera
/// let v1 = Vec4::new(1.0, 0.0, 1.0, 1.0);   // In front
/// let v2 = Vec4::new(-1.0, 0.0, 1.0, 1.0);  // In front
///
/// let clipped = clip_triangle_against_near_plane(
///     v0, v1, v2,
///     |v| v.w,
///     |a, b, t| a.lerp(b, t)
/// );
/// ```
pub fn clip_triangle_against_near_plane<V: Copy>(
    v0: V,
    v1: V,
    v2: V,
    get_w: impl Fn(&V) -> f32,
    lerp: impl Fn(V, V, f32) -> V,
) -> ClippedTriangles<V> {
    // Check which vertices are inside (w >= NEAR)
    let inside0 = get_w(&v0) >= NEAR;
    let inside1 = get_w(&v1) >= NEAR;
    let inside2 = get_w(&v2) >= NEAR;

    let inside_count = usize::from(inside0) + usize::from(inside1) + usize::from(inside2);

    let mut result = ClippedTriangles::new_uninit();

    if inside_count == 3 {
        // All inside - return original
        result.tris[0].write(v0);
        result.tris[1].write(v1);
        result.tris[2].write(v2);
        result.count = 1;
        return result;
    }

    if inside_count == 0 {
        // All outside - cull
        return result;
    }

    // Clipping needed
    // Helper to intersect edge against w=NEAR
    let intersect = |start: V, end: V| -> V {
        let w_start = get_w(&start);
        let w_end = get_w(&end);
        let t = (NEAR - w_start) / (w_end - w_start);
        lerp(start, end, t)
    };

    // Logic depends on which are inside.
    let vertices = [v0, v1, v2];
    let inside = [inside0, inside1, inside2];

    // Max 4 vertices for a clipped triangle (quad)
    let mut out_verts: [MaybeUninit<V>; 4] = [const { MaybeUninit::uninit() }; 4];
    let mut out_count = 0;

    for i in 0..3 {
        let curr = vertices[i];
        let next = vertices[(i + 1) % 3];
        let curr_in = inside[i];
        let next_in = inside[(i + 1) % 3];

        if curr_in {
            out_verts[out_count].write(curr);
            out_count += 1;
        }

        if curr_in != next_in {
            out_verts[out_count].write(intersect(curr, next));
            out_count += 1;
        }
    }

    // Now assemble triangles
    if out_count == 3 {
        result.tris[0].write(unsafe { out_verts[0].assume_init() });
        result.tris[1].write(unsafe { out_verts[1].assume_init() });
        result.tris[2].write(unsafe { out_verts[2].assume_init() });
        result.count = 1;
    } else if out_count == 4 {
        // Quad (0,1,2,3) -> Tri1(0,1,2), Tri2(0,2,3)
        // Accessing indices 0,1,2,3 is safe because out_count == 4
        let v0 = unsafe { out_verts[0].assume_init() };
        let v1 = unsafe { out_verts[1].assume_init() };
        let v2 = unsafe { out_verts[2].assume_init() };
        let v3 = unsafe { out_verts[3].assume_init() };

        result.tris[0].write(v0);
        result.tris[1].write(v1);
        result.tris[2].write(v2);

        result.tris[3].write(v0);
        result.tris[4].write(v2);
        result.tris[5].write(v3);
        result.count = 2;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec3;

    // Use (Vec3, f32) as vertex type where f32 is w
    type Vertex = (Vec3, f32);

    fn get_w(v: &Vertex) -> f32 {
        v.1
    }

    fn get_pos(v: &Vertex) -> (Vec3, f32) {
        *v
    }

    #[test]
    fn test_frustum_clipping_right_plane() {
        // Triangle crossing the right plane (x = w)
        // v0: (-5, 0, 0, 10)  -> x/w = -0.5 (Inside)
        // v1: ( 5, 0, 0, 10)  -> x/w = 0.5 (Inside)
        // v2: (15, 0, 0, 10)  -> x/w = 1.5 (Outside)

        let v0: Vertex = (Vec3::new(-5.0, 0.0, 0.0), 10.0);
        let v1: Vertex = (Vec3::new(5.0, 0.0, 0.0), 10.0);
        let v2: Vertex = (Vec3::new(15.0, 0.0, 0.0), 10.0);

        let result = clip_triangle_to_frustum(v0, v1, v2, get_pos, |a, b, t| {
            (a.0.lerp(b.0, t), a.1 + (b.1 - a.1) * t)
        });

        // Should be clipped
        assert!(result.count > 0, "Should output at least one triangle");

        for i in 0..result.count {
            let base = i * 3;
            for k in 0..3 {
                let v = result[base + k];
                let (pos, w) = v;
                // Check Right Plane: x <= w
                assert!(
                    pos.x <= w + 0.001,
                    "Point x={} w={} violates Right Plane",
                    pos.x,
                    w
                );
                // Check Left Plane: x >= -w
                assert!(
                    pos.x >= -w - 0.001,
                    "Point x={} w={} violates Left Plane",
                    pos.x,
                    w
                );
            }
        }
    }

    #[test]
    fn test_frustum_all_inside() {
        let v0: Vertex = (Vec3::new(0.0, 0.0, 0.0), 10.0);
        let v1: Vertex = (Vec3::new(1.0, 0.0, 0.0), 10.0);
        let v2: Vertex = (Vec3::new(0.0, 1.0, 0.0), 10.0);

        let result = clip_triangle_to_frustum(v0, v1, v2, get_pos, |a, b, t| {
            (a.0.lerp(b.0, t), a.1 + (b.1 - a.1) * t)
        });

        assert_eq!(result.count(), 1);
        assert_eq!(result[0], v0);
        assert_eq!(result[1], v1);
        assert_eq!(result[2], v2);
    }

    #[test]
    fn test_frustum_all_outside() {
        // All far to the right
        let v0: Vertex = (Vec3::new(20.0, 0.0, 0.0), 10.0);
        let v1: Vertex = (Vec3::new(21.0, 0.0, 0.0), 10.0);
        let v2: Vertex = (Vec3::new(20.0, 1.0, 0.0), 10.0);

        let result = clip_triangle_to_frustum(v0, v1, v2, get_pos, |a, b, t| {
            (a.0.lerp(b.0, t), a.1 + (b.1 - a.1) * t)
        });

        assert_eq!(result.count(), 0);
    }

    #[test]
    fn test_all_inside() {
        let v0: Vertex = (Vec3::new(0.0, 0.0, 0.0), 1.0);
        let v1: Vertex = (Vec3::new(1.0, 0.0, 0.0), 1.0);
        let v2: Vertex = (Vec3::new(0.0, 1.0, 0.0), 1.0);

        let result = clip_triangle_against_near_plane(v0, v1, v2, get_w, |a, b, t| {
            (a.0.lerp(b.0, t), a.1 + (b.1 - a.1) * t)
        });

        assert_eq!(result.count(), 1);
        assert_eq!(result[0], v0);
        assert_eq!(result[1], v1);
        assert_eq!(result[2], v2);
    }

    #[test]
    fn test_all_outside() {
        let v0: Vertex = (Vec3::new(0.0, 0.0, 0.0), -1.0);
        let v1: Vertex = (Vec3::new(1.0, 0.0, 0.0), -1.0);
        let v2: Vertex = (Vec3::new(0.0, 1.0, 0.0), -1.0);

        let result = clip_triangle_against_near_plane(v0, v1, v2, get_w, |a, b, t| {
            (a.0.lerp(b.0, t), a.1 + (b.1 - a.1) * t)
        });

        assert_eq!(result.count(), 0);
    }

    #[test]
    fn test_one_inside() {
        // v0 inside (w=1.0), v1, v2 outside (w=-1.0)
        let v0: Vertex = (Vec3::new(0.0, 0.0, 1.0), 1.0);
        let v1: Vertex = (Vec3::new(0.0, 2.0, -1.0), -1.0);
        let v2: Vertex = (Vec3::new(2.0, 0.0, -1.0), -1.0);

        let result = clip_triangle_against_near_plane(v0, v1, v2, get_w, |a, b, t| {
            (a.0.lerp(b.0, t), a.1 + (b.1 - a.1) * t)
        });

        // Should return 1 triangle
        assert_eq!(result.count(), 1);

        // Check vertices
        // The first vertex should be v0
        assert_eq!(result[0], v0);

        // The other two should have w = NEAR (0.001)
        assert!((result[1].1 - NEAR).abs() < 1e-6);
        assert!((result[2].1 - NEAR).abs() < 1e-6);
    }

    #[test]
    fn test_two_inside() {
        // v0, v1 inside (w=1.0), v2 outside (w=-1.0)
        let v0: Vertex = (Vec3::new(0.0, 0.0, 0.0), 1.0);
        let v1: Vertex = (Vec3::new(1.0, 0.0, 0.0), 1.0);
        let v2: Vertex = (Vec3::new(0.0, 2.0, -1.0), -1.0); // Outside

        let result = clip_triangle_against_near_plane(v0, v1, v2, get_w, |a, b, t| {
            (a.0.lerp(b.0, t), a.1 + (b.1 - a.1) * t)
        });

        // Should return 2 triangles (quad)
        assert_eq!(result.count(), 2);

        // Verify structure of returned triangles
        // First triangle: v0, v1, intersect(v1, v2)
        // Second triangle: v0, intersect(v1, v2), intersect(v2, v0)

        // Check that we have vertices with w=NEAR
        // Count how many have w=NEAR
        // Total vertices in result = 3 * count = 6

        // Tri1: v0, v1, intersect(v1, v2) -> w: 1.0, 1.0, NEAR
        // Tri2: v0, intersect(v1, v2), intersect(v2, v0) -> w: 1.0, NEAR, NEAR

        // Total w values: 1.0, 1.0, NEAR, 1.0, NEAR, NEAR.
        // So 3 vertices with w=1.0, 3 vertices with w=NEAR.

        let mut near_count = 0;
        let mut inner_count = 0;

        for i in 0..6 {
            if (result[i].1 - NEAR).abs() < 1e-6 {
                near_count += 1;
            } else if (result[i].1 - 1.0).abs() < 1e-6 {
                inner_count += 1;
            }
        }

        assert_eq!(near_count, 3);
        assert_eq!(inner_count, 3);
    }

    #[test]
    fn test_exact_boundary() {
        // All vertices exactly on NEAR
        let v0: Vertex = (Vec3::new(0.0, 0.0, 0.0), NEAR);
        let v1: Vertex = (Vec3::new(1.0, 0.0, 0.0), NEAR);
        let v2: Vertex = (Vec3::new(0.0, 1.0, 0.0), NEAR);

        let result = clip_triangle_against_near_plane(v0, v1, v2, get_w, |a, b, t| {
            (a.0.lerp(b.0, t), a.1 + (b.1 - a.1) * t)
        });

        // Should be considered inside
        assert_eq!(result.count(), 1);
    }

    #[test]
    fn test_epsilon_boundary() {
        // v0 just below NEAR, v1 just above
        let eps = 1e-7;
        let v0: Vertex = (Vec3::new(0.0, 0.0, 0.0), NEAR - eps); // Outside
        let v1: Vertex = (Vec3::new(1.0, 0.0, 0.0), NEAR + eps); // Inside
        let v2: Vertex = (Vec3::new(0.0, 1.0, 0.0), 1.0); // Inside

        let result = clip_triangle_against_near_plane(v0, v1, v2, get_w, |a, b, t| {
            (a.0.lerp(b.0, t), a.1 + (b.1 - a.1) * t)
        });

        // Should clip to 2 triangles (quad) since 2 are inside
        assert_eq!(result.count(), 2);
    }

    #[test]
    fn test_line_fully_inside() {
        // Line fully inside the frustum
        // -w <= x,y,z <= w
        let w = 10.0;
        let v0: Vertex = (Vec3::new(0.0, 0.0, 0.0), w);
        let v1: Vertex = (Vec3::new(1.0, 1.0, 5.0), w);

        let result = clip_line_to_frustum(v0, v1, get_pos, |a, b, t| {
            (a.0.lerp(b.0, t), a.1 + (b.1 - a.1) * t)
        });

        assert!(result.is_some());
        let (r0, r1) = result.unwrap();
        assert_eq!(r0, v0);
        assert_eq!(r1, v1);
    }

    #[test]
    fn test_line_fully_outside() {
        // Line fully outside the frustum (to the right)
        // x > w
        let w = 10.0;
        let v0: Vertex = (Vec3::new(20.0, 0.0, 0.0), w);
        let v1: Vertex = (Vec3::new(25.0, 0.0, 0.0), w);

        let result = clip_line_to_frustum(v0, v1, get_pos, |a, b, t| {
            (a.0.lerp(b.0, t), a.1 + (b.1 - a.1) * t)
        });

        assert!(result.is_none());
    }

    #[test]
    fn test_line_intersects_one_plane() {
        // Line crossing the Right plane (x = w)
        // v0 inside: (5, 0, 0, 10) -> 5 <= 10
        // v1 outside: (15, 0, 0, 10) -> 15 > 10
        // Intersection should be at x=10
        let w = 10.0;
        let v0: Vertex = (Vec3::new(5.0, 0.0, 0.0), w);
        let v1: Vertex = (Vec3::new(15.0, 0.0, 0.0), w);

        let result = clip_line_to_frustum(v0, v1, get_pos, |a, b, t| {
            (a.0.lerp(b.0, t), a.1 + (b.1 - a.1) * t)
        });

        assert!(result.is_some());
        let (r0, r1) = result.unwrap();

        // v0 should be unchanged
        assert_eq!(r0, v0);

        // r1 should be on the boundary x=w=10
        // Interpolation: (15-5) range. 5 is 5 units from 10. 15 is 5 units from 10. Midpoint.
        // t = 0.5.
        // r1 pos = lerp(5, 15, 0.5) = 10.
        assert!((r1.0.x - 10.0).abs() < 1e-6);
        assert!((r1.0.y - 0.0).abs() < 1e-6);
        assert!((r1.0.z - 0.0).abs() < 1e-6);
        assert!((r1.1 - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_line_spanning_frustum() {
        // Line crossing Left and Right planes
        // v0: (-20, 0, 0, 10) -> Outside Left
        // v1: ( 20, 0, 0, 10) -> Outside Right
        let w = 10.0;
        let v0: Vertex = (Vec3::new(-20.0, 0.0, 0.0), w);
        let v1: Vertex = (Vec3::new(20.0, 0.0, 0.0), w);

        let result = clip_line_to_frustum(v0, v1, get_pos, |a, b, t| {
            (a.0.lerp(b.0, t), a.1 + (b.1 - a.1) * t)
        });

        assert!(result.is_some());
        let (r0, r1) = result.unwrap();

        // r0 should be at x = -10
        assert!((r0.0.x - (-10.0)).abs() < 1e-6);
        // r1 should be at x = 10
        assert!((r1.0.x - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_line_on_boundary() {
        // Line lying exactly on the Near plane boundary (z = -w)
        let w = 10.0;
        let v0: Vertex = (Vec3::new(0.0, 0.0, -10.0), w);
        let v1: Vertex = (Vec3::new(0.0, 5.0, -10.0), w);

        let result = clip_line_to_frustum(v0, v1, get_pos, |a, b, t| {
            (a.0.lerp(b.0, t), a.1 + (b.1 - a.1) * t)
        });

        assert!(result.is_some());
        let (r0, r1) = result.unwrap();

        assert_eq!(r0, v0);
        assert_eq!(r1, v1);
    }
}
