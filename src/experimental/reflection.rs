//! Reflection mapping using a Skybox.
//!
//! This module implements environment mapping where objects reflect the skybox.

use crate::clipping::clip_triangle_to_frustum;
use crate::experimental::skybox::Cubemap;
use crate::framebuffer::Framebuffer;
use crate::math::{ScreenPoint, Vec3, project_triangle_to_screen};
use crate::rasterizer::{FIXED_SCALE, sort_by_y};
#[cfg(test)]
use crate::math::Mat4;
use crate::zbuffer::ZBuffer;

/// Helper to prepare scanline slices.
/// Copied from rasterizer.rs since it is private.
#[inline(always)]
fn prepare_scanline<'a>(
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

#[derive(Clone, Copy)]
struct ReflectionGradients {
    dz_dx: f32,
    dq_dx: f32,
    dnx_dx: f32,
    dny_dx: f32,
    dnz_dx: f32,
    dwx_dx: f32,
    dwy_dx: f32,
    dwz_dx: f32,
}

impl ReflectionGradients {
    #[allow(clippy::too_many_arguments)]
    fn new(
        p0: ScreenPoint,
        p1: ScreenPoint,
        p2: ScreenPoint,
        q0: f32,
        q1: f32,
        q2: f32,
        n0: Vec3,
        n1: Vec3,
        n2: Vec3,
        w0: Vec3,
        w1: Vec3,
        w2: Vec3,
    ) -> (Self, bool) {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;
        let uq = q1 - q0;
        let unx = n1.x - n0.x;
        let uny = n1.y - n0.y;
        let unz = n1.z - n0.z;
        let uwx = w1.x - w0.x;
        let uwy = w1.y - w0.y;
        let uwz = w1.z - w0.z;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        let vz = p2.z - p0.z;
        let vq = q2 - q0;
        let vnx = n2.x - n0.x;
        let vny = n2.y - n0.y;
        let vnz = n2.z - n0.z;
        let vwx = w2.x - w0.x;
        let vwy = w2.y - w0.y;
        let vwz = w2.z - w0.z;

        let nz = ux * vy - uy * vx;
        let inv_nz = if nz.abs() > 0.0001 { -1.0 / nz } else { 0.0 };

        let nx_z = uy * vz - uz * vy;
        let dz_dx = nx_z * inv_nz;

        let nx_q = uy * vq - uq * vy;
        let dq_dx = nx_q * inv_nz;

        let nx_nx = uy * vnx - unx * vy;
        let dnx_dx = nx_nx * inv_nz;

        let nx_ny = uy * vny - uny * vy;
        let dny_dx = nx_ny * inv_nz;

        let nx_nz = uy * vnz - unz * vy;
        let dnz_dx = nx_nz * inv_nz;

        let nx_wx = uy * vwx - uwx * vy;
        let dwx_dx = nx_wx * inv_nz;

        let nx_wy = uy * vwy - uwy * vy;
        let dwy_dx = nx_wy * inv_nz;

        let nx_wz = uy * vwz - uwz * vy;
        let dwz_dx = nx_wz * inv_nz;

        (
            Self {
                dz_dx,
                dq_dx,
                dnx_dx,
                dny_dx,
                dnz_dx,
                dwx_dx,
                dwy_dx,
                dwz_dx,
            },
            nz > 0.0,
        )
    }
}

struct ReflectionEdgeWalker {
    x: i64,
    z: f32,
    q: f32,
    nx: f32,
    ny: f32,
    nz: f32,
    wx: f32,
    wy: f32,
    wz: f32,
    dx_dy: i64,
    dz_dy: f32,
    dq_dy: f32,
    dnx_dy: f32,
    dny_dy: f32,
    dnz_dy: f32,
    dwx_dy: f32,
    dwy_dy: f32,
    dwz_dy: f32,
}

impl ReflectionEdgeWalker {
    #[allow(clippy::too_many_arguments)]
    fn new(
        p_start: ScreenPoint,
        p_end: ScreenPoint,
        q_start: f32,
        q_end: f32,
        n_start: Vec3,
        n_end: Vec3,
        w_start: Vec3,
        w_end: Vec3,
    ) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let inv_h = if height == 0.0 { 0.0 } else { 1.0 / height };

        let dx_dy =
            ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64;
        let dz_dy = (p_end.z - p_start.z) * inv_h;
        let dq_dy = (q_end - q_start) * inv_h;
        let dnx_dy = (n_end.x - n_start.x) * inv_h;
        let dny_dy = (n_end.y - n_start.y) * inv_h;
        let dnz_dy = (n_end.z - n_start.z) * inv_h;
        let dwx_dy = (w_end.x - w_start.x) * inv_h;
        let dwy_dy = (w_end.y - w_start.y) * inv_h;
        let dwz_dy = (w_end.z - w_start.z) * inv_h;

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            q: q_start,
            nx: n_start.x,
            ny: n_start.y,
            nz: n_start.z,
            wx: w_start.x,
            wy: w_start.y,
            wz: w_start.z,
            dx_dy,
            dz_dy,
            dq_dy,
            dnx_dy,
            dny_dy,
            dnz_dy,
            dwx_dy,
            dwy_dy,
            dwz_dy,
        }
    }

    fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.q += self.dq_dy;
        self.nx += self.dnx_dy;
        self.ny += self.dny_dy;
        self.nz += self.dnz_dy;
        self.wx += self.dwx_dy;
        self.wy += self.dwy_dy;
        self.wz += self.dwz_dy;
    }

    fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * n_f;
        self.q += self.dq_dy * n_f;
        self.nx += self.dnx_dy * n_f;
        self.ny += self.dny_dy * n_f;
        self.nz += self.dnz_dy * n_f;
        self.wx += self.dwx_dy * n_f;
        self.wy += self.dwy_dy * n_f;
        self.wz += self.dwz_dy * n_f;
    }
}

struct ReflectionSpanStart {
    z: f32,
    q: f32,
    nx: f32,
    ny: f32,
    nz: f32,
    wx: f32,
    wy: f32,
    wz: f32,
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::needless_pass_by_value)]
fn draw_scanline_reflection(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    start: ReflectionSpanStart,
    gradients: &ReflectionGradients,
    cubemap: &Cubemap,
    camera_pos: Vec3,
) {
    if let Some((fb_slice, zb_slice, mut z)) =
        prepare_scanline(fb, zb, y, x_start, x_end, start.z, gradients.dz_dx)
    {
        // Calculate offsets to fast-forward state to x_start (handled by prepare_scanline's loop implicitly? No.)
        // prepare_scanline handles z adjustment if x_start < 0.
        // We need to adjust other accumulators if x_start < 0.

        let xs = x_start;
        // let mut xe = x_end; // Unused after slice creation

        let mut q = start.q;
        let mut nx = start.nx;
        let mut ny = start.ny;
        let mut nz = start.nz;
        let mut wx = start.wx;
        let mut wy = start.wy;
        let mut wz = start.wz;

        if xs < 0 {
            let diff = -xs as f32;
            // z already adjusted in prepare_scanline
            q += diff * gradients.dq_dx;
            nx += diff * gradients.dnx_dx;
            ny += diff * gradients.dny_dx;
            nz += diff * gradients.dnz_dx;
            wx += diff * gradients.dwx_dx;
            wy += diff * gradients.dwy_dx;
            wz += diff * gradients.dwz_dx;
            // xs = 0; // handled
        }

        // prepare_scanline returns slice starting at clamped xs.

        for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
            if z < *depth_val {
                *depth_val = z;

                let w_recip = if q.abs() > 0.000_001 { 1.0 / q } else { 1.0 };

                // Recover world position
                // wx is wx/w.
                let world_pos = Vec3::new(wx * w_recip, wy * w_recip, wz * w_recip);

                // Normal
                // We normalize (nx, ny, nz) which removes the 1/w scaling factor naturally
                let normal = Vec3::new(nx, ny, nz).normalize();

                // View Vector (Camera to Surface)
                let view_dir = (world_pos - camera_pos).normalize();

                // Reflection: I - 2.0 * dot(N, I) * N
                // I = view_dir
                let reflect_dir = view_dir - normal * 2.0 * view_dir.dot(normal);

                // Sample
                *pixel = cubemap.sample(reflect_dir);
            }

            z += gradients.dz_dx;
            q += gradients.dq_dx;
            nx += gradients.dnx_dx;
            ny += gradients.dny_dx;
            nz += gradients.dnz_dx;
            wx += gradients.dwx_dx;
            wy += gradients.dwy_dx;
            wz += gradients.dwz_dx;
        }
    }
}

/// Fill a 3D triangle with Environment Reflection Mapping.
///
/// This function calculates the reflection vector per-pixel and samples the provided Cubemap.
///
/// # Arguments
///
/// * `v0`, `v1`, `v2` - Vertices defined as `((ClipPos, W), Normal, WorldPos)`.
/// * `cubemap` - The skybox/environment map to reflect.
/// * `camera_pos` - The world-space position of the camera (used to calculate view vector).
///
/// # Panics
///
/// Panics if the framebuffer and z-buffer dimensions do not match.
pub fn fill_triangle_reflection(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec3, Vec3), // ((ClipPos, W), Normal, WorldPos)
    v1: ((Vec3, f32), Vec3, Vec3),
    v2: ((Vec3, f32), Vec3, Vec3),
    cubemap: &Cubemap,
    camera_pos: Vec3,
) {
    assert!(
        fb.width() == zb.width() && fb.height() == zb.height(),
        "Framebuffer and ZBuffer dimensions mismatch"
    );

    // Clip
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

        // Project
        let (p0_orig, p1_orig, p2_orig) = project_triangle_to_screen(
            v0.0.0,
            v0.0.1,
            v1.0.0,
            v1.0.1,
            v2.0.0,
            v2.0.1,
            half_width,
            half_height,
        );

        // Backface Culling
        let ux_orig = i64::from(p1_orig.x) - i64::from(p0_orig.x);
        let uy_orig = i64::from(p1_orig.y) - i64::from(p0_orig.y);
        let vx_orig = i64::from(p2_orig.x) - i64::from(p0_orig.x);
        let vy_orig = i64::from(p2_orig.y) - i64::from(p0_orig.y);
        let nz_orig = ux_orig * vy_orig - uy_orig * vx_orig;

        if nz_orig >= 0 {
            continue;
        }

        // Prepare attributes
        let inv_w0 = p0_orig.inv_w;
        let inv_w1 = p1_orig.inv_w;
        let inv_w2 = p2_orig.inv_w;

        // Normal * inv_w
        let n0 = v0.1 * inv_w0;
        let n1 = v1.1 * inv_w1;
        let n2 = v2.1 * inv_w2;

        // WorldPos * inv_w
        let w0 = v0.2 * inv_w0;
        let w1 = v1.2 * inv_w1;
        let w2 = v2.2 * inv_w2;

        let mut verts = [(p0_orig, n0, w0), (p1_orig, n1, w1), (p2_orig, n2, w2)];
        let parity = sort_by_y(&mut verts, |(p, ..)| p.y);
        let [(p0, n0, w0), (p1, n1, w1), (p2, n2, w2)] = verts;

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
        let (gradients, _) =
            ReflectionGradients::new(p0, p1, p2, q0, q1, q2, n0, n1, n2, w0, w1, w2);

        let long_edge_is_left = parity;

        let mut edge_a = ReflectionEdgeWalker::new(p0, p2, q0, q2, n0, n2, w0, w2);
        if y_start > p0.y {
            edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
        }

        let mut edge_b = if y_start < p1.y {
            let mut e = ReflectionEdgeWalker::new(p0, p1, q0, q1, n0, n1, w0, w1);
            if y_start > p0.y {
                e.step_n(i64::from(y_start) - i64::from(p0.y));
            }
            e
        } else {
            let mut e = ReflectionEdgeWalker::new(p1, p2, q1, q2, n1, n2, w1, w2);
            if y_start > p1.y {
                e.step_n(i64::from(y_start) - i64::from(p1.y));
            }
            e
        };

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b = ReflectionEdgeWalker::new(p1, p2, q1, q2, n1, n2, w1, w2);
            }

            let (
                x_start,
                x_end,
                z_left,
                nx_left,
                ny_left,
                nz_left,
                wx_left,
                wy_left,
                wz_left,
                q_left,
            ) = if long_edge_is_left {
                (
                    (edge_a.x >> 16) as i32,
                    (edge_b.x >> 16) as i32,
                    edge_a.z,
                    edge_a.nx,
                    edge_a.ny,
                    edge_a.nz,
                    edge_a.wx,
                    edge_a.wy,
                    edge_a.wz,
                    edge_a.q,
                )
            } else {
                (
                    (edge_b.x >> 16) as i32,
                    (edge_a.x >> 16) as i32,
                    edge_b.z,
                    edge_b.nx,
                    edge_b.ny,
                    edge_b.nz,
                    edge_b.wx,
                    edge_b.wy,
                    edge_b.wz,
                    edge_b.q,
                )
            };

            let dx = i64::from(x_end) - i64::from(x_start);

            if dx > 0 {
                draw_scanline_reflection(
                    fb,
                    zb,
                    y,
                    x_start,
                    x_end,
                    ReflectionSpanStart {
                        z: z_left,
                        q: q_left,
                        nx: nx_left,
                        ny: ny_left,
                        nz: nz_left,
                        wx: wx_left,
                        wy: wy_left,
                        wz: wz_left,
                    },
                    &gradients,
                    cubemap,
                    camera_pos,
                );
            }

            edge_a.step();
            edge_b.step();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::texture::Texture;

    #[test]
    fn test_reflection_renders() {
        let width = 100;
        let height = 100;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        // Create a dummy cubemap with distinct colors per face
        let make_tex = |color: u32| {
            let mut t = Texture::new(1, 1).unwrap();
            t.pixels[0] = color;
            t
        };

        // Right(+X): Red, Left(-X): Green, Top(+Y): Blue, Bottom(-Y): Yellow, Front(+Z): Cyan, Back(-Z): Magenta
        let faces = [
            make_tex(0xFFFF0000), // +X Red
            make_tex(0xFF00FF00), // -X Green
            make_tex(0xFF0000FF), // +Y Blue
            make_tex(0xFFFFFF00), // -Y Yellow
            make_tex(0xFF00FFFF), // +Z Cyan
            make_tex(0xFFFF00FF), // -Z Magenta
        ];
        let cubemap = Cubemap::new(faces);

        // Define a triangle centered at 0,0,0, facing +Z (Normal = 0,0,1)
        // Camera at 0,0,10.
        // View Vector (Cam->Pos) = (0,0,0) - (0,0,10) = (0,0,-10) -> (0,0,-1)
        // Normal = (0,0,1)
        // Reflect = V - 2(V.N)N = (0,0,-1) - 2(-1 * 1) * (0,0,1) = (0,0,-1) + 2(0,0,1) = (0,0,1)
        // Reflect Dir = +Z. Should see Front Face (Cyan).

        // Mock Projection: Standard Perspective
        let proj = Mat4::perspective(1.57, 1.0, 0.1, 100.0);

        // Camera at Origin (0,0,0). Triangle at Z=-5 (In front of camera).
        let cam_pos = Vec3::new(0.0, 0.0, 0.0);
        let w0 = Vec3::new(0.0, 1.0, -5.0);
        let w1 = Vec3::new(-1.0, -1.0, -5.0);
        let w2 = Vec3::new(1.0, -1.0, -5.0);
        let n = Vec3::new(0.0, 0.0, 1.0); // Normal facing camera (+Z)

        // Clip Space
        let (c0, cw0) = proj.transform_point(w0);
        let (c1, cw1) = proj.transform_point(w1);
        let (c2, cw2) = proj.transform_point(w2);

        let v0 = ((c0, cw0), n, w0);
        let v1 = ((c1, cw1), n, w1);
        let v2 = ((c2, cw2), n, w2);

        // V = Pos - Cam = (0,0,-5) - 0 = (0,0,-1)
        // N = (0,0,1)
        // R = V - 2(V.N)N = (0,0,-1) - 2(-1)(0,0,1) = (0,0,-1) + 2(0,0,1) = (0,0,1)
        // Reflect Dir = +Z. Should see Front Face (Cyan).

        fill_triangle_reflection(
            &mut fb,
            &mut zb,
            v0, v1, v2,
            &cubemap,
            cam_pos
        );

        // Check center pixel
        let center = fb.get_pixel(50, 50).unwrap();
        // Should be Cyan
        assert_eq!(center, 0xFF00FFFF);
    }
}
