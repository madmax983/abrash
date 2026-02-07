//! Shared rasterization utilities.

use crate::math::ScreenPoint;

/// Fixed point scale factor (16.16)
pub(crate) const FIXED_SCALE: f32 = 65536.0;

/// Helper to sort 3 vertices by Y coordinate
///
/// Optimization: Uses a manual sorting network to avoid the heap allocation
/// incurred by `slice::sort_by_key` for small arrays.
pub(crate) fn sort_by_y<T, F>(verts: &mut [T; 3], get_y: F)
where
    F: Fn(&T) -> i32,
{
    // Manual 3-step sort to avoid allocation
    if get_y(&verts[0]) > get_y(&verts[1]) {
        verts.swap(0, 1);
    }
    if get_y(&verts[1]) > get_y(&verts[2]) {
        verts.swap(1, 2);
    }
    if get_y(&verts[0]) > get_y(&verts[1]) {
        verts.swap(0, 1);
    }
}

/// Checks if a triangle is backfacing (or degenerate)
///
/// Uses the 2D cross product of the screen-space edges.
/// Returns true if the triangle should be culled (ccw winding for front faces).
#[inline(always)]
pub(crate) fn is_backface(p0: ScreenPoint, p1: ScreenPoint, p2: ScreenPoint) -> bool {
    let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
    let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
    let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
    let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
    let nz = ux * vy - uy * vx;
    nz >= 0.0
}

pub(crate) struct EdgeWalker {
    pub(crate) x: i64,
    pub(crate) z: f32,
    dx_dy: i64,
    dz_dy: f32,
}

impl EdgeWalker {
    pub(crate) fn new(p_start: ScreenPoint, p_end: ScreenPoint) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let (dx_dy, dz_dy) = if height == 0.0 {
            (0, 0.0)
        } else {
            let inv_h = 1.0 / height;
            (
                ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64,
                (p_end.z - p_start.z) * inv_h,
            )
        };

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            dx_dy,
            dz_dy,
        }
    }

    pub(crate) fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
    }

    pub(crate) fn step_n(&mut self, n: i32) {
        let n_i64 = i64::from(n);
        let n_f = n as f32;
        self.x += self.dx_dy * n_i64;
        self.z += self.dz_dy * n_f;
    }
}
