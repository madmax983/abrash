use crate::clipping::clip_triangle_to_frustum;
use crate::framebuffer::Framebuffer;
use crate::math::{project_to_screen_optimized, ScreenPoint, Vec2, Vec3, Vec4};
use crate::texture::Texture;
use crate::zbuffer::ZBuffer;

use super::common::{assert_same_dimensions, color_to_u32, is_backface, sort_by_y, FIXED_SCALE};

#[derive(Clone, Copy)]
pub struct NormalMapGradients {
    pub dz_dx: f32,
    pub dq_dx: f32, // 1/w
    pub du_dx: f32, // u/w
    pub dv_dx: f32, // v/w
    pub dnx_dx: f32, // nx/w
    pub dny_dx: f32,
    pub dnz_dx: f32,
    pub dtx_dx: f32, // tx/w
    pub dty_dx: f32,
    pub dtz_dx: f32,
    pub dtw_dx: f32, // tw/w
}

impl NormalMapGradients {
    #[allow(clippy::too_many_arguments)]
    fn new(
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
        n0: Vec3,
        n1: Vec3,
        n2: Vec3,
        t0: Vec4,
        t1: Vec4,
        t2: Vec4,
    ) -> Self {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;
        let uq = q1 - q0;
        let uu = u1 - u0;
        let uv = v1 - v0;
        let unx = n1.x - n0.x;
        let uny = n1.y - n0.y;
        let unz = n1.z - n0.z;
        let utx = t1.x - t0.x;
        let uty = t1.y - t0.y;
        let utz = t1.z - t0.z;
        let utw = t1.w - t0.w;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        let vz = p2.z - p0.z;
        let vq = q2 - q0;
        let vu = u2 - u0;
        let vv = v2 - v0;
        let vnx = n2.x - n0.x;
        let vny = n2.y - n0.y;
        let vnz = n2.z - n0.z;
        let vtx = t2.x - t0.x;
        let vty = t2.y - t0.y;
        let vtz = t2.z - t0.z;
        let vtw = t2.w - t0.w;

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

        let nx_nx = uy * vnx - unx * vy;
        let dnx_dx = nx_nx * inv_nz;

        let nx_ny = uy * vny - uny * vy;
        let dny_dx = nx_ny * inv_nz;

        let nx_nz = uy * vnz - unz * vy;
        let dnz_dx = nx_nz * inv_nz;

        let nx_tx = uy * vtx - utx * vy;
        let dtx_dx = nx_tx * inv_nz;

        let nx_ty = uy * vty - uty * vy;
        let dty_dx = nx_ty * inv_nz;

        let nx_tz = uy * vtz - utz * vy;
        let dtz_dx = nx_tz * inv_nz;

        let nx_tw = uy * vtw - utw * vy;
        let dtw_dx = nx_tw * inv_nz;

        Self {
            dz_dx,
            dq_dx,
            du_dx,
            dv_dx,
            dnx_dx,
            dny_dx,
            dnz_dx,
            dtx_dx,
            dty_dx,
            dtz_dx,
            dtw_dx,
        }
    }
}

pub struct NormalMapEdgeWalker {
    pub x: i64,
    pub z: f32,
    pub q: f32, // 1/w
    pub u: f32, // u/w
    pub v: f32, // v/w
    pub nx: f32,
    pub ny: f32,
    pub nz: f32,
    pub tx: f32,
    pub ty: f32,
    pub tz: f32,
    pub tw: f32,
    dx_dy: i64,
    dz_dy: f32,
    dq_dy: f32,
    du_dy: f32,
    dv_dy: f32,
    dnx_dy: f32,
    dny_dy: f32,
    dnz_dy: f32,
    dtx_dy: f32,
    dty_dy: f32,
    dtz_dy: f32,
    dtw_dy: f32,
}

impl NormalMapEdgeWalker {
    #[allow(clippy::too_many_arguments)]
    fn new(
        p_start: ScreenPoint,
        p_end: ScreenPoint,
        q_start: f32,
        q_end: f32,
        u_start: f32,
        u_end: f32,
        v_start: f32,
        v_end: f32,
        n_start: Vec3,
        n_end: Vec3,
        t_start: Vec4,
        t_end: Vec4,
    ) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let inv_h = if height == 0.0 { 0.0 } else { 1.0 / height };

        let dx_dy =
            ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64;
        let dz_dy = (p_end.z - p_start.z) * inv_h;
        let dq_dy = (q_end - q_start) * inv_h;
        let du_dy = (u_end - u_start) * inv_h;
        let dv_dy = (v_end - v_start) * inv_h;
        let dnx_dy = (n_end.x - n_start.x) * inv_h;
        let dny_dy = (n_end.y - n_start.y) * inv_h;
        let dnz_dy = (n_end.z - n_start.z) * inv_h;
        let dtx_dy = (t_end.x - t_start.x) * inv_h;
        let dty_dy = (t_end.y - t_start.y) * inv_h;
        let dtz_dy = (t_end.z - t_start.z) * inv_h;
        let dtw_dy = (t_end.w - t_start.w) * inv_h;

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            q: q_start,
            u: u_start,
            v: v_start,
            nx: n_start.x,
            ny: n_start.y,
            nz: n_start.z,
            tx: t_start.x,
            ty: t_start.y,
            tz: t_start.z,
            tw: t_start.w,
            dx_dy,
            dz_dy,
            dq_dy,
            du_dy,
            dv_dy,
            dnx_dy,
            dny_dy,
            dnz_dy,
            dtx_dy,
            dty_dy,
            dtz_dy,
            dtw_dy,
        }
    }

    fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.q += self.dq_dy;
        self.u += self.du_dy;
        self.v += self.dv_dy;
        self.nx += self.dnx_dy;
        self.ny += self.dny_dy;
        self.nz += self.dnz_dy;
        self.tx += self.dtx_dy;
        self.ty += self.dty_dy;
        self.tz += self.dtz_dy;
        self.tw += self.dtw_dy;
    }

    fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * n_f;
        self.q += self.dq_dy * n_f;
        self.u += self.du_dy * n_f;
        self.v += self.dv_dy * n_f;
        self.nx += self.dnx_dy * n_f;
        self.ny += self.dny_dy * n_f;
        self.nz += self.dnz_dy * n_f;
        self.tx += self.dtx_dy * n_f;
        self.ty += self.dty_dy * n_f;
        self.tz += self.dtz_dy * n_f;
        self.tw += self.dtw_dy * n_f;
    }
}

#[derive(Clone, Copy)]
pub struct NormalMapSpanStart {
    pub z: f32,
    pub q: f32,
    pub u: f32,
    pub v: f32,
    pub nx: f32,
    pub ny: f32,
    pub nz: f32,
    pub tx: f32,
    pub ty: f32,
    pub tz: f32,
    pub tw: f32,
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_scanline_normal_mapped(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    start: NormalMapSpanStart,
    gradients: &NormalMapGradients,
    texture: &Texture,
    normal_map: &Texture,
    neg_light_dir: Vec3,
    pre_diffuse_color: Vec3, // base_color * light_color
    ambient: Vec3,
) {
    let width = fb.width() as i32;
    let mut xs = x_start;
    let mut xe = x_end;

    // Local accumulators
    let mut z = start.z;
    let mut q = start.q;
    let mut u = start.u;
    let mut v = start.v;
    let mut nx = start.nx;
    let mut ny = start.ny;
    let mut nz = start.nz;
    let mut tx = start.tx;
    let mut ty = start.ty;
    let mut tz = start.tz;
    let mut tw = start.tw;

    if xs < 0 {
        let diff = -i64::from(xs);
        let diff_f = diff as f32;
        z += diff_f * gradients.dz_dx;
        q += diff_f * gradients.dq_dx;
        u += diff_f * gradients.du_dx;
        v += diff_f * gradients.dv_dx;
        nx += diff_f * gradients.dnx_dx;
        ny += diff_f * gradients.dny_dx;
        nz += diff_f * gradients.dnz_dx;
        tx += diff_f * gradients.dtx_dx;
        ty += diff_f * gradients.dty_dx;
        tz += diff_f * gradients.dtz_dx;
        tw += diff_f * gradients.dtw_dx;
        xs = 0;
    }

    if xe >= width {
        xe = width - 1;
    }

    if xs > xe {
        return;
    }

    let width_usize = fb.width() as usize;
    let y_offset = (y as usize) * width_usize;
    let start_idx = y_offset + (xs as usize);
    let end_idx = y_offset + (xe as usize);

    // SAFETY: Clamped above.
    let fb_slice = unsafe { fb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };
    let zb_slice = unsafe { zb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };

    for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
        if z < *depth_val {
            *depth_val = z;

            // Perspective recover
            let w_recip = if q.abs() > 0.000_001 { 1.0 / q } else { 1.0 };
            let u_tex = u * w_recip;
            let v_tex = v * w_recip;

            // Sample diffuse
            // Using Nearest for speed in this complex shader, or duplicate logic for Bilinear
            let diffuse_color_u32 = texture.get_pixel_texel(u_tex as i32, v_tex as i32);
            // Unpack diffuse to Vec3 (0-1)
            let diff_r = ((diffuse_color_u32 >> 16) & 0xFF) as f32 / 255.0;
            let diff_g = ((diffuse_color_u32 >> 8) & 0xFF) as f32 / 255.0;
            let diff_b = (diffuse_color_u32 & 0xFF) as f32 / 255.0;
            let diffuse_sample = Vec3::new(diff_r, diff_g, diff_b);

            // Sample normal map
            let nm_color_u32 = normal_map.get_pixel_texel(u_tex as i32, v_tex as i32);
            // Unpack to [-1, 1]
            let nm_r = (((nm_color_u32 >> 16) & 0xFF) as f32 / 255.0) * 2.0 - 1.0;
            let nm_g = (((nm_color_u32 >> 8) & 0xFF) as f32 / 255.0) * 2.0 - 1.0;
            let nm_b = ((nm_color_u32 & 0xFF) as f32 / 255.0) * 2.0 - 1.0;
            let tangent_normal = Vec3::new(nm_r, nm_g, nm_b); // Usually Z is up in tangent space

            // TBN Construction
            // Normalize interpolated N and T (they are n/w and t/w, but direction is same)
            // Need to recover true direction
            let n_interp = Vec3::new(nx, ny, nz).normalize(); // Assuming non-zero
            let t_interp = Vec3::new(tx, ty, tz).normalize();

            // Gram-Schmidt re-orthogonalize T with respect to N
            let t_ortho = (t_interp - n_interp * n_interp.dot(t_interp)).normalize();

            // Calculate Bitangent
            // tw holds handedness * w. But we want just handedness.
            // w is 1/q. So tw/q = handedness * w / (1/w) = handedness * w^2? No.
            // tw is (handedness * 1.0) / w.
            // q is 1/w.
            // tw / q = handedness.
            let handedness = if (tw * w_recip) > 0.0 { 1.0 } else { -1.0 };
            let b_ortho = n_interp.cross(t_ortho) * handedness;

            // Transform normal from tangent space to world space
            // N_world = T * nm.x + B * nm.y + N * nm.z
            let final_normal = (t_ortho * tangent_normal.x + b_ortho * tangent_normal.y + n_interp * tangent_normal.z).normalize();

            // Lighting
            let intensity = final_normal.dot(neg_light_dir).max(0.0);

            // Combine
            let diffuse_total = pre_diffuse_color * diffuse_sample * intensity;
            let final_color_vec = ambient + diffuse_total;
            *pixel = color_to_u32(final_color_vec);
        }

        z += gradients.dz_dx;
        q += gradients.dq_dx;
        u += gradients.du_dx;
        v += gradients.dv_dx;
        nx += gradients.dnx_dx;
        ny += gradients.dny_dx;
        nz += gradients.dnz_dx;
        tx += gradients.dtx_dx;
        ty += gradients.dty_dx;
        tz += gradients.dtz_dx;
        tw += gradients.dtw_dx;
    }
}

/// Fill a 3D triangle with Normal Mapping.
///
/// This function performs per-pixel tangent-space normal mapping.
///
/// # Arguments
///
/// * `v0`, `v1`, `v2` - Vertices defined as `((Position, W), UV, Normal, Tangent)`.
///   - `Tangent`: `Vec4` where xyz is the tangent vector and w is the handedness (+1/-1).
#[allow(clippy::too_many_arguments)]
pub fn fill_triangle_normal_mapped(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec2, Vec3, Vec4),
    v1: ((Vec3, f32), Vec2, Vec3, Vec4),
    v2: ((Vec3, f32), Vec2, Vec3, Vec4),
    texture: &Texture,
    normal_map: &Texture,
    light_dir: Vec3,
    light_color: Vec3,
    ambient: Vec3,
) {
    assert_same_dimensions(fb, zb);

    let clipped = clip_triangle_to_frustum(v0, v1, v2, |v| v.0);

    let width = fb.width();
    let height = fb.height();
    let half_width = width as f32 * 0.5;
    let half_height = height as f32 * 0.5;

    for i in 0..clipped.count {
        let base = i * 3;
        let v0 = clipped.tris[base];
        let v1 = clipped.tris[base + 1];
        let v2 = clipped.tris[base + 2];

        // Project to screen
        let p0_orig = project_to_screen_optimized(v0.0.0, v0.0.1, half_width, half_height);
        let p1_orig = project_to_screen_optimized(v1.0.0, v1.0.1, half_width, half_height);
        let p2_orig = project_to_screen_optimized(v2.0.0, v2.0.1, half_width, half_height);

        // Backface Culling
        if is_backface(p0_orig, p1_orig, p2_orig) {
            continue;
        }

        // Prepare attributes
        let inv_w0 = p0_orig.inv_w;
        let inv_w1 = p1_orig.inv_w;
        let inv_w2 = p2_orig.inv_w;

        // Scale UV by texture size (assuming both maps match size or using one size for ratio)
        // Usually, UVs are 0..1, we multiply by size to get texel coords
        let w = texture.width as f32;
        let h = texture.height as f32;

        let u0 = v0.1.x * w * inv_w0;
        let v0_val = v0.1.y * h * inv_w0;
        let u1 = v1.1.x * w * inv_w1;
        let v1_val = v1.1.y * h * inv_w1;
        let u2 = v2.1.x * w * inv_w2;
        let v2_val = v2.1.y * h * inv_w2;

        let n0 = v0.2 * inv_w0;
        let n1 = v1.2 * inv_w1;
        let n2 = v2.2 * inv_w2;

        let t0 = v0.3 * inv_w0;
        let t1 = v1.3 * inv_w1;
        let t2 = v2.3 * inv_w2;

        let mut verts = [
            (p0_orig, u0, v0_val, n0, t0),
            (p1_orig, u1, v1_val, n1, t1),
            (p2_orig, u2, v2_val, n2, t2),
        ];
        sort_by_y(&mut verts, |(p, ..)| p.y);
        let [(p0, u0, v0_v, n0, t0), (p1, u1, v1_v, n1, t1), (p2, u2, v2_v, n2, t2)] = verts;

        let q0 = p0.inv_w;
        let q1 = p1.inv_w;
        let q2 = p2.inv_w;

        let total_height = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        if total_height == 0.0 {
            continue;
        }

        let y_min = 0;
        let y_max = height as i32 - 1;
        let y_start = p0.y.max(y_min);
        let y_end = p2.y.min(y_max);

        if y_start > y_end {
            continue;
        }

        // Gradients and Edge Walking
        let (gradients, long_edge_is_left) = {
            let g = NormalMapGradients::new(
                p0, p1, p2, q0, q1, q2, u0, u1, u2, v0_v, v1_v, v2_v, n0, n1, n2, t0, t1, t2,
            );
            let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
            let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
            let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
            let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
            let left = ux * vy - uy * vx > 0.0;
            (g, left)
        };

        let mut edge_a =
            NormalMapEdgeWalker::new(p0, p2, q0, q2, u0, u2, v0_v, v2_v, n0, n2, t0, t2);
        if y_start > p0.y {
            edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
        }

        let mut edge_b = if y_start < p1.y {
            let mut e =
                NormalMapEdgeWalker::new(p0, p1, q0, q1, u0, u1, v0_v, v1_v, n0, n1, t0, t1);
            if y_start > p0.y {
                e.step_n(i64::from(y_start) - i64::from(p0.y));
            }
            e
        } else {
            let mut e =
                NormalMapEdgeWalker::new(p1, p2, q1, q2, u1, u2, v1_v, v2_v, n1, n2, t1, t2);
            if y_start > p1.y {
                e.step_n(i64::from(y_start) - i64::from(p1.y));
            }
            e
        };

        let neg_light_dir = light_dir * -1.0;
        let pre_diffuse_color = light_color; // Base color comes from texture

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b = NormalMapEdgeWalker::new(p1, p2, q1, q2, u1, u2, v1_v, v2_v, n1, n2, t1, t2);
            }

            // Unpack walker state
            let (x_start, x_end, z_left, q_left, u_left, v_left, nx_left, ny_left, nz_left, tx_left, ty_left, tz_left, tw_left) =
            if long_edge_is_left {
                ((edge_a.x >> 16) as i32, (edge_b.x >> 16) as i32, edge_a.z, edge_a.q, edge_a.u, edge_a.v, edge_a.nx, edge_a.ny, edge_a.nz, edge_a.tx, edge_a.ty, edge_a.tz, edge_a.tw)
            } else {
                ((edge_b.x >> 16) as i32, (edge_a.x >> 16) as i32, edge_b.z, edge_b.q, edge_b.u, edge_b.v, edge_b.nx, edge_b.ny, edge_b.nz, edge_b.tx, edge_b.ty, edge_b.tz, edge_b.tw)
            };

            let dx = i64::from(x_end) - i64::from(x_start);

            if dx > 0 {
                draw_scanline_normal_mapped(
                    fb, zb, y, x_start, x_end,
                    NormalMapSpanStart {
                        z: z_left, q: q_left, u: u_left, v: v_left,
                        nx: nx_left, ny: ny_left, nz: nz_left,
                        tx: tx_left, ty: ty_left, tz: tz_left, tw: tw_left,
                    },
                    &gradients,
                    texture, normal_map,
                    neg_light_dir, pre_diffuse_color, ambient,
                );
            }

            edge_a.step();
            edge_b.step();
        }
    }
}
