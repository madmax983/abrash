use crate::framebuffer::Framebuffer;
use crate::math::{ScreenPoint, Vec3};
use crate::zbuffer::ZBuffer;

/// Fixed point scale factor (16.16)
pub const FIXED_SCALE: f32 = 65536.0;

/// Helper to ensure buffer dimensions match
#[inline]
pub(crate) fn assert_same_dimensions(fb: &Framebuffer, zb: &ZBuffer) {
    assert_eq!(
        fb.width(),
        zb.width(),
        "Framebuffer and ZBuffer widths must match"
    );
    assert_eq!(
        fb.height(),
        zb.height(),
        "Framebuffer and ZBuffer heights must match"
    );
}

/// Helper to prepare scanline slices.
/// Returns (`fb_slice`, `zb_slice`, `adjusted_z_start`) or `None` if off-screen.
#[inline(always)]
pub(crate) fn prepare_scanline<'a>(
    fb: &'a mut Framebuffer,
    zb: &'a mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    mut z: f32,
    dz_dx: f32,
) -> Option<(&'a mut [u32], &'a mut [f32], f32)> {
    if y < 0 || y >= fb.height() as i32 {
        return None;
    }

    let width = fb.width() as i32;
    let mut xs = x_start;
    let mut xe = x_end;

    if xs < 0 {
        let diff = -xs as f32;
        z += diff * dz_dx;
        xs = 0;
    }

    if xe >= width {
        xe = width - 1;
    }

    if xs > xe {
        return None;
    }

    let width_usize = fb.width() as usize;
    let y_offset = (y as usize) * width_usize;
    let start_idx = y_offset + (xs as usize);
    let end_idx = y_offset + (xe as usize);

    // SAFETY:
    // 1. xs and xe are clamped to [0, width-1].
    // 2. y is assumed to be within bounds by caller (clamped in fill_triangle).
    // 3. start_idx <= end_idx because xs <= xe.
    let fb_slice = unsafe { fb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };
    let zb_slice = unsafe { zb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };

    Some((fb_slice, zb_slice, z))
}

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
    let ux = i64::from(p1.x) - i64::from(p0.x);
    let uy = i64::from(p1.y) - i64::from(p0.y);
    let vx = i64::from(p2.x) - i64::from(p0.x);
    let vy = i64::from(p2.y) - i64::from(p0.y);
    // Use i64 for cross product.
    // i64 is sufficient as long as viewport width < 3e9, which is enforced by Framebuffer::new.
    let nz = ux * vy - uy * vx;
    nz >= 0
}

/// Convert Vec3 color (0.0-1.0 per channel) to u32 ARGB
#[must_use]
pub fn color_to_u32(color: Vec3) -> u32 {
    let r = (color.x.clamp(0.0, 1.0) * 255.0) as u32;
    let g = (color.y.clamp(0.0, 1.0) * 255.0) as u32;
    let b = (color.z.clamp(0.0, 1.0) * 255.0) as u32;
    0xFF00_0000 | (r << 16) | (g << 8) | b
}

/// Helper to convert pre-scaled (0.0-255.0) Vec3 color to u32 ARGB
#[must_use]
#[inline(always)]
#[allow(clippy::missing_const_for_fn)]
pub fn color_to_u32_scaled(color: Vec3) -> u32 {
    let r = color.x.clamp(0.0, 255.0) as u32;
    let g = color.y.clamp(0.0, 255.0) as u32;
    let b = color.z.clamp(0.0, 255.0) as u32;
    0xFF00_0000 | (r << 16) | (g << 8) | b
}

/// Helper to pack 8-bit color channels into u32 ARGB
#[inline(always)]
pub const fn pack_color_channels(r: u32, g: u32, b: u32) -> u32 {
    0xFF00_0000 | (r << 16) | (g << 8) | b
}

/// Helper for fast color packing from fixed point.
#[inline(always)]
pub fn pack_color_fixed(c: (i64, i64, i64)) -> u32 {
    let r = (c.0 >> 16).clamp(0, 255) as u32;
    let g = (c.1 >> 16).clamp(0, 255) as u32;
    let b = (c.2 >> 16).clamp(0, 255) as u32;
    pack_color_channels(r, g, b)
}

/// Helper to iterate along the edge of a triangle in screen space.
///
/// This struct manages the state for walking down a triangle edge, interpolating
/// X and Z coordinates. It uses fixed-point arithmetic for X to ensure
/// pixel-perfect rasterization consistency.
pub(crate) struct EdgeWalker {
    /// Current X coordinate in 16.16 fixed-point format.
    ///
    /// The upper 16 bits represent the integer pixel coordinate.
    /// The lower 16 bits represent sub-pixel precision.
    pub(crate) x: i64,
    /// Change in X per scanline (dx/dy) in 16.16 fixed-point format.
    dx_dy: i64,
    /// Current Z depth.
    pub(crate) z: f32,
    /// Change in Z per scanline (dz/dy).
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

    pub(crate) fn step_n(&mut self, n: i64) {
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * (n as f32);
    }
}

pub(crate) const RECIPROCAL_TABLE: [f32; 17] = [
    0.0,
    1.0,
    0.5,
    0.333_333_34,
    0.25,
    0.2,
    0.166_666_67,
    0.142_857_15,
    0.125,
    0.111_111_11,
    0.1,
    0.090_909_09,
    0.083_333_336,
    0.076_923_08,
    0.071_428_575,
    0.066_666_67,
    0.062_5,
];

#[derive(Clone, Copy)]
pub struct PerspectiveTextureGradients {
    pub dz_dx: f32,
    pub dq_dx: f32,
    pub du_dx: f32,
    pub dv_dx: f32,
    pub dq_dy: f32,
    pub du_dy: f32,
    pub dv_dy: f32,
}

impl PerspectiveTextureGradients {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        p0: ScreenPoint,
        p1: ScreenPoint,
        p2: ScreenPoint,
        q0: f32,
        q1: f32,
        q2: f32,
        u0: f32,
        u1: f32,
        u2: f32,
        v0: f32,
        v1: f32,
        v2: f32,
    ) -> Self {
        Self::new_with_winding(p0, p1, p2, q0, q1, q2, u0, u1, u2, v0, v1, v2).0
    }

    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new_with_winding(
        p0: ScreenPoint,
        p1: ScreenPoint,
        p2: ScreenPoint,
        q0: f32,
        q1: f32,
        q2: f32,
        u0: f32,
        u1: f32,
        u2: f32,
        v0: f32,
        v1: f32,
        v2: f32,
    ) -> (Self, bool) {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;
        let uq = q1 - q0;
        let uu = u1 - u0;
        let uv = v1 - v0;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        let vz = p2.z - p0.z;
        let vq = q2 - q0;
        let vu = u2 - u0;
        let vv = v2 - v0;

        let nz = ux * vy - uy * vx;
        let inv_nz = if nz.abs() > 0.0001 { -1.0 / nz } else { 0.0 };

        let nx_z = uy * vz - uz * vy;
        let dz_dx = nx_z * inv_nz;

        let nx_q = uy * vq - uq * vy;
        let dq_dx = nx_q * inv_nz;

        let nx_u = uy * vu - uu * vy;
        let du_dx = nx_u * inv_nz;

        let nx_v = uy * vv - uv * vy;
        let dv_dx = nx_v * inv_nz;

        // Calculate Y gradients
        let ny_q = uq * vx - ux * vq;
        let dq_dy = ny_q * inv_nz;

        let ny_u = uu * vx - ux * vu;
        let du_dy = ny_u * inv_nz;

        let ny_v = uv * vx - ux * vv;
        let dv_dy = ny_v * inv_nz;

        (
            Self {
                dz_dx,
                dq_dx,
                du_dx,
                dv_dx,
                dq_dy,
                du_dy,
                dv_dy,
            },
            nz > 0.0,
        )
    }
}

pub(crate) struct PerspectiveTextureEdgeWalker {
    pub(crate) x: i64,
    pub(crate) z: f32,
    pub(crate) q: f32, // 1/w
    pub(crate) u: f32, // u/w
    pub(crate) v: f32, // v/w
    dx_dy: i64,
    dz_dy: f32,
    dq_dy: f32,
    du_dy: f32,
    dv_dy: f32,
}

impl PerspectiveTextureEdgeWalker {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        p_start: ScreenPoint,
        p_end: ScreenPoint,
        q_start: f32,
        q_end: f32,
        u_start: f32,
        u_end: f32,
        v_start: f32,
        v_end: f32,
    ) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let inv_h = if height == 0.0 { 0.0 } else { 1.0 / height };

        let dx_dy =
            ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64;
        let dz_dy = (p_end.z - p_start.z) * inv_h;
        let dq_dy = (q_end - q_start) * inv_h;
        let du_dy = (u_end - u_start) * inv_h;
        let dv_dy = (v_end - v_start) * inv_h;

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            q: q_start,
            u: u_start,
            v: v_start,
            dx_dy,
            dz_dy,
            dq_dy,
            du_dy,
            dv_dy,
        }
    }

    pub(crate) fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.q += self.dq_dy;
        self.u += self.du_dy;
        self.v += self.dv_dy;
    }

    pub(crate) fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * n_f;
        self.q += self.dq_dy * n_f;
        self.u += self.du_dy * n_f;
        self.v += self.dv_dy * n_f;
    }
}

#[derive(Clone, Copy)]
pub struct PerspectiveSpanStart {
    pub z: f32,
    pub q: f32,
    pub u: f32,
    pub v: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct NormalMapGradients {
    pub(crate) dz_dx: f32,
    pub(crate) dq_dx: f32,  // 1/w
    pub(crate) du_dx: f32,  // u/w
    pub(crate) dv_dx: f32,  // v/w
    pub(crate) dlx_dx: f32, // lx/w (Tangent Space Light X)
    pub(crate) dly_dx: f32,
    pub(crate) dlz_dx: f32,
}

impl NormalMapGradients {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        p0: ScreenPoint,
        p1: ScreenPoint,
        p2: ScreenPoint,
        q0: f32,
        q1: f32,
        q2: f32,
        u0: f32,
        u1: f32,
        u2: f32,
        v0: f32,
        v1: f32,
        v2: f32,
        l0: Vec3, // Tangent Space Light Vectors (pre-scaled by q)
        l1: Vec3,
        l2: Vec3,
    ) -> (Self, bool) {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;
        let uq = q1 - q0;
        let uu = u1 - u0;
        let uv = v1 - v0;
        let ulx = l1.x - l0.x;
        let uly = l1.y - l0.y;
        let ulz = l1.z - l0.z;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        let vz = p2.z - p0.z;
        let vq = q2 - q0;
        let vu = u2 - u0;
        let vv = v2 - v0;
        let vlx = l2.x - l0.x;
        let vly = l2.y - l0.y;
        let vlz = l2.z - l0.z;

        let nz = ux * vy - uy * vx;
        let inv_nz = if nz.abs() > 0.0001 { -1.0 / nz } else { 0.0 };

        let nx_z = uy * vz - uz * vy;
        let dz_dx = nx_z * inv_nz;

        let nx_q = uy * vq - uq * vy;
        let dq_dx = nx_q * inv_nz;

        let nx_u = uy * vu - uu * vy;
        let du_dx = nx_u * inv_nz;

        let nx_v = uy * vv - uv * vy;
        let dv_dx = nx_v * inv_nz;

        let nx_lx = uy * vlx - ulx * vy;
        let dlx_dx = nx_lx * inv_nz;

        let nx_ly = uy * vly - uly * vy;
        let dly_dx = nx_ly * inv_nz;

        let nx_lz = uy * vlz - ulz * vy;
        let dlz_dx = nx_lz * inv_nz;

        (
            Self {
                dz_dx,
                dq_dx,
                du_dx,
                dv_dx,
                dlx_dx,
                dly_dx,
                dlz_dx,
            },
            nz > 0.0,
        )
    }
}

pub(crate) struct NormalMapEdgeWalker {
    pub(crate) x: i64,
    pub(crate) z: f32,
    pub(crate) q: f32,
    pub(crate) u: f32,
    pub(crate) v: f32,
    pub(crate) lx: f32,
    pub(crate) ly: f32,
    pub(crate) lz: f32,
    dx_dy: i64,
    dz_dy: f32,
    dq_dy: f32,
    du_dy: f32,
    dv_dy: f32,
    dlx_dy: f32,
    dly_dy: f32,
    dlz_dy: f32,
}

impl NormalMapEdgeWalker {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        p_start: ScreenPoint,
        p_end: ScreenPoint,
        q_start: f32,
        q_end: f32,
        u_start: f32,
        u_end: f32,
        v_start: f32,
        v_end: f32,
        l_start: Vec3,
        l_end: Vec3,
    ) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let inv_h = if height == 0.0 { 0.0 } else { 1.0 / height };

        let dx_dy =
            ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64;
        let dz_dy = (p_end.z - p_start.z) * inv_h;
        let dq_dy = (q_end - q_start) * inv_h;
        let du_dy = (u_end - u_start) * inv_h;
        let dv_dy = (v_end - v_start) * inv_h;
        let dlx_dy = (l_end.x - l_start.x) * inv_h;
        let dly_dy = (l_end.y - l_start.y) * inv_h;
        let dlz_dy = (l_end.z - l_start.z) * inv_h;

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            q: q_start,
            u: u_start,
            v: v_start,
            lx: l_start.x,
            ly: l_start.y,
            lz: l_start.z,
            dx_dy,
            dz_dy,
            dq_dy,
            du_dy,
            dv_dy,
            dlx_dy,
            dly_dy,
            dlz_dy,
        }
    }

    pub(crate) fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.q += self.dq_dy;
        self.u += self.du_dy;
        self.v += self.dv_dy;
        self.lx += self.dlx_dy;
        self.ly += self.dly_dy;
        self.lz += self.dlz_dy;
    }

    pub(crate) fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * n_f;
        self.q += self.dq_dy * n_f;
        self.u += self.du_dy * n_f;
        self.v += self.dv_dy * n_f;
        self.lx += self.dlx_dy * n_f;
        self.ly += self.dly_dy * n_f;
        self.lz += self.dlz_dy * n_f;
    }
}

#[derive(Clone, Copy)]
pub(crate) struct NormalMapSpanStart {
    pub(crate) z: f32,
    pub(crate) q: f32,
    pub(crate) u: f32,
    pub(crate) v: f32,
    pub(crate) lx: f32,
    pub(crate) ly: f32,
    pub(crate) lz: f32,
}

#[derive(Clone, Copy)]
pub struct TexturedGouraudGradients {
    pub dz_dx: f32,
    pub dq_dx: f32,
    pub du_dx: f32,
    pub dv_dx: f32,
    pub dr_dx: f32,
    pub dg_dx: f32,
    pub db_dx: f32,
    // Y gradients for steps
    dq_dy: f32,
    du_dy: f32,
    dv_dy: f32,
    dr_dy: f32,
    dg_dy: f32,
    db_dy: f32,
}

impl TexturedGouraudGradients {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        p0: ScreenPoint,
        p1: ScreenPoint,
        p2: ScreenPoint,
        q0: f32,
        q1: f32,
        q2: f32,
        u0: f32,
        u1: f32,
        u2: f32,
        v0: f32,
        v1: f32,
        v2: f32,
        c0: Vec3,
        c1: Vec3,
        c2: Vec3,
    ) -> (Self, bool) {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;
        let uq = q1 - q0;
        let uu = u1 - u0;
        let uv = v1 - v0;
        let ur = c1.x - c0.x;
        let ug = c1.y - c0.y;
        let ub = c1.z - c0.z;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        let vz = p2.z - p0.z;
        let vq = q2 - q0;
        let vu = u2 - u0;
        let vv = v2 - v0;
        let vr = c2.x - c0.x;
        let vg = c2.y - c0.y;
        let vb = c2.z - c0.z;

        let nz = ux * vy - uy * vx;
        let inv_nz = if nz.abs() > 0.0001 { -1.0 / nz } else { 0.0 };

        let nx_z = uy * vz - uz * vy;
        let dz_dx = nx_z * inv_nz;

        let nx_q = uy * vq - uq * vy;
        let dq_dx = nx_q * inv_nz;

        let nx_u = uy * vu - uu * vy;
        let du_dx = nx_u * inv_nz;

        let nx_v = uy * vv - uv * vy;
        let dv_dx = nx_v * inv_nz;

        let nx_r = uy * vr - ur * vy;
        let dr_dx = nx_r * inv_nz;

        let nx_g = uy * vg - ug * vy;
        let dg_dx = nx_g * inv_nz;

        let nx_b = uy * vb - ub * vy;
        let db_dx = nx_b * inv_nz;

        // Y gradients
        let ny_q = uq * vx - ux * vq;
        let dq_dy = ny_q * inv_nz;

        let ny_u = uu * vx - ux * vu;
        let du_dy = ny_u * inv_nz;

        let ny_v = uv * vx - ux * vv;
        let dv_dy = ny_v * inv_nz;

        let ny_r = ur * vx - ux * vr;
        let dr_dy = ny_r * inv_nz;

        let ny_g = ug * vx - ux * vg;
        let dg_dy = ny_g * inv_nz;

        let ny_b = ub * vx - ux * vb;
        let db_dy = ny_b * inv_nz;

        (
            Self {
                dz_dx,
                dq_dx,
                du_dx,
                dv_dx,
                dr_dx,
                dg_dx,
                db_dx,
                dq_dy,
                du_dy,
                dv_dy,
                dr_dy,
                dg_dy,
                db_dy,
            },
            nz > 0.0,
        )
    }
}

pub(crate) struct TexturedGouraudEdgeWalker {
    pub(crate) x: i64,
    pub(crate) z: f32,
    pub(crate) q: f32, // 1/w
    pub(crate) u: f32, // u/w
    pub(crate) v: f32, // v/w
    pub(crate) r: f32, // r
    pub(crate) g: f32, // g
    pub(crate) b: f32, // b
    dx_dy: i64,
    dz_dy: f32,
    dq_dy: f32,
    du_dy: f32,
    dv_dy: f32,
    dr_dy: f32,
    dg_dy: f32,
    db_dy: f32,
}

impl TexturedGouraudEdgeWalker {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        p_start: ScreenPoint,
        p_end: ScreenPoint,
        q_start: f32,
        q_end: f32,
        u_start: f32,
        u_end: f32,
        v_start: f32,
        v_end: f32,
        c_start: Vec3,
        c_end: Vec3,
    ) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let inv_h = if height == 0.0 { 0.0 } else { 1.0 / height };

        let dx_dy =
            ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64;
        let dz_dy = (p_end.z - p_start.z) * inv_h;
        let dq_dy = (q_end - q_start) * inv_h;
        let du_dy = (u_end - u_start) * inv_h;
        let dv_dy = (v_end - v_start) * inv_h;
        let dr_dy = (c_end.x - c_start.x) * inv_h;
        let dg_dy = (c_end.y - c_start.y) * inv_h;
        let db_dy = (c_end.z - c_start.z) * inv_h;

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            q: q_start,
            u: u_start,
            v: v_start,
            r: c_start.x,
            g: c_start.y,
            b: c_start.z,
            dx_dy,
            dz_dy,
            dq_dy,
            du_dy,
            dv_dy,
            dr_dy,
            dg_dy,
            db_dy,
        }
    }

    pub(crate) fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.q += self.dq_dy;
        self.u += self.du_dy;
        self.v += self.dv_dy;
        self.r += self.dr_dy;
        self.g += self.dg_dy;
        self.b += self.db_dy;
    }

    pub(crate) fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * n_f;
        self.q += self.dq_dy * n_f;
        self.u += self.du_dy * n_f;
        self.v += self.dv_dy * n_f;
        self.r += self.dr_dy * n_f;
        self.g += self.dg_dy * n_f;
        self.b += self.db_dy * n_f;
    }
}

#[derive(Clone, Copy)]
pub struct TexturedGouraudSpanStart {
    pub z: f32,
    pub q: f32,
    pub u: f32,
    pub v: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::ScreenPoint;

    #[test]
    fn edge_walker_z_interpolation() {
        let p0 = ScreenPoint {
            x: 0,
            y: 0,
            z: 1.0,
            inv_w: 1.0,
        };
        let p1 = ScreenPoint {
            x: 100,
            y: 100,
            z: 2.0,
            inv_w: 1.0,
        };

        let walker = EdgeWalker::new(p0, p1);

        assert!((walker.z - 1.0).abs() < 0.0001);
        let expected_dz_dy = (2.0_f32 - 1.0_f32) / 100.0_f32;
        assert!((walker.dz_dy - expected_dz_dy).abs() < 0.0001);
    }

    #[test]
    fn edge_walker_step_accumulates_correctly() {
        let p0 = ScreenPoint {
            x: 0,
            y: 0,
            z: 1.0,
            inv_w: 1.0,
        };
        let p1 = ScreenPoint {
            x: 100,
            y: 100,
            z: 2.0,
            inv_w: 1.0,
        };

        let mut walker = EdgeWalker::new(p0, p1);
        let initial_z = walker.z;
        let dz = walker.dz_dy;

        walker.step_n(50);
        let expected_z = initial_z + dz * 50.0;
        assert!((walker.z - expected_z).abs() < 0.0001);
        assert!(
            walker.z > 1.0 && walker.z < 2.0,
            "z should be interpolated between 1.0 and 2.0, got {}",
            walker.z,
        );
    }

    #[test]
    fn edge_walker_zero_height_no_panic() {
        let p0 = ScreenPoint {
            x: 0,
            y: 100,
            z: 1.0,
            inv_w: 1.0,
        };
        let p1 = ScreenPoint {
            x: 100,
            y: 100,
            z: 2.0,
            inv_w: 1.0,
        };

        let walker = EdgeWalker::new(p0, p1);
        assert_eq!(walker.dx_dy, 0);
        assert!((walker.dz_dy - 0.0).abs() < f32::EPSILON);
        assert!((walker.z - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_is_backface_overflow_safe() {
        let p0 = ScreenPoint {
            x: 0,
            y: 0,
            z: 0.0,
            inv_w: 1.0,
        };
        let p1 = ScreenPoint {
            x: 2_000_000_000,
            y: 0,
            z: 0.0,
            inv_w: 1.0,
        };
        let p2 = ScreenPoint {
            x: 0,
            y: 2_000_000_000,
            z: 0.0,
            inv_w: 1.0,
        };

        let result = is_backface(p0, p1, p2);
        assert!(result);
    }
}
