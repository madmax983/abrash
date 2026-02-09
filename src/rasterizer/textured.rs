//! Textured rasterization (Perspective Correct).

use crate::clipping::clip_triangle_to_frustum;
use crate::framebuffer::Framebuffer;
use crate::math::{ScreenPoint, Vec2, Vec3, project_to_screen};
use crate::texture::{FilterMode, Texture};
use crate::zbuffer::ZBuffer;

use super::common::{FIXED_SCALE, assert_same_dimensions, sort_by_y};

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

        Self {
            dz_dx,
            dq_dx,
            du_dx,
            dv_dx,
            dq_dy,
            du_dy,
            dv_dy,
        }
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

    pub(crate) fn step_n(&mut self, n: i32) {
        let n_i64 = i64::from(n);
        let n_f = n as f32;
        self.x += self.dx_dy * n_i64;
        self.z += self.dz_dy * n_f;
        self.q += self.dq_dy * n_f;
        self.u += self.du_dy * n_f;
        self.v += self.dv_dy * n_f;
    }
}

#[derive(Clone, Copy)]
pub(crate) struct PerspectiveSpanStart {
    pub(crate) z: f32,
    pub(crate) q: f32,
    pub(crate) u: f32,
    pub(crate) v: f32,
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

/// Draw a single scanline with perspective-correct texture mapping
/// Optimized using span-based interpolation (every 16 pixels)
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_scanline_textured_perspective(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    texture: &Texture,
    y: i32,
    x_start: i32,
    x_end: i32,
    start: PerspectiveSpanStart,
    gradients: &PerspectiveTextureGradients,
) {
    let width = fb.width() as i32;
    let mut xs = x_start;
    let mut xe = x_end;
    let mut z = start.z;
    let mut q = start.q;
    let mut u = start.u;
    let mut v = start.v;

    if xs < 0 {
        let diff = -i64::from(xs);
        let diff_f = diff as f32;
        z += diff_f * gradients.dz_dx;
        q += diff_f * gradients.dq_dx;
        u += diff_f * gradients.du_dx;
        v += diff_f * gradients.dv_dx;
        xs = 0;
    }

    if xe >= width {
        xe = width - 1;
    }

    if xs > xe {
        return;
    }

    let span_size = 16;
    let mut x = xs;

    // Calculate initial start values
    let w_start = if q.abs() > 0.000_001 { 1.0 / q } else { 1.0 };
    let mut u_tex_start = u * w_start;
    let mut v_tex_start = v * w_start;

    while x <= xe {
        let remaining = xe - x + 1;
        let count = remaining.min(span_size);

        // End values at 'x + count'
        let q_end = q + gradients.dq_dx * count as f32;
        let u_end = u + gradients.du_dx * count as f32;
        let v_end = v + gradients.dv_dx * count as f32;

        // Perform perspective divide at span endpoints
        let w_end = if q_end.abs() > 0.000_001 {
            1.0 / q_end
        } else {
            1.0
        };
        let u_tex_end = u_end * w_end;
        let v_tex_end = v_end * w_end;

        // Interpolate texel coordinates linearly over the span
        // Optimization: Use reciprocal table to replace division with multiplication
        // count is guaranteed to be in [1, 16]
        let inv_count = RECIPROCAL_TABLE[count as usize];
        let du_tex_step = (u_tex_end - u_tex_start) * inv_count;
        let dv_tex_step = (v_tex_end - v_tex_start) * inv_count;

        let width_usize = fb.width() as usize;
        let y_offset = (y as usize) * width_usize;
        let start_idx = y_offset + (x as usize);
        let end_idx = y_offset + ((x + count - 1) as usize);

        // SAFETY: Bounds checked by xs, xe clamping and loop logic
        let fb_slice = unsafe { fb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };
        let zb_slice = unsafe { zb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };

        match texture.filter_mode {
            FilterMode::Nearest => {
                // Fixed point optimization for Nearest Neighbor
                let mut u_fix = (u_tex_start * 65536.0) as i32;
                let mut v_fix = (v_tex_start * 65536.0) as i32;
                let du_fix = (du_tex_step * 65536.0) as i32;
                let dv_fix = (dv_tex_step * 65536.0) as i32;

                for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
                    if z < *depth_val {
                        *depth_val = z;
                        *pixel = texture.get_pixel_texel(u_fix >> 16, v_fix >> 16);
                    }
                    z += gradients.dz_dx;
                    u_fix = u_fix.wrapping_add(du_fix);
                    v_fix = v_fix.wrapping_add(dv_fix);
                }
            }
            FilterMode::Bilinear => {
                // Fixed point optimization for Bilinear
                // Use 16.16 for accumulation to maintain precision, then downshift to 24.8 for sampling
                let mut u_fix = (u_tex_start * 65536.0) as i32;
                let mut v_fix = (v_tex_start * 65536.0) as i32;
                let du_fix = (du_tex_step * 65536.0) as i32;
                let dv_fix = (dv_tex_step * 65536.0) as i32;

                for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
                    if z < *depth_val {
                        *depth_val = z;
                        // Convert 16.16 to 24.8 (x >> 8)
                        *pixel = texture.get_pixel_bilinear_fixed(u_fix >> 8, v_fix >> 8);
                    }
                    z += gradients.dz_dx;
                    u_fix = u_fix.wrapping_add(du_fix);
                    v_fix = v_fix.wrapping_add(dv_fix);
                }
            }
            FilterMode::Trilinear => {
                // For Trilinear, we need LOD
                // Calculate LOD at span center (approx)
                // We use span start values for calculation to avoid extra per-pixel work
                // u_tex_start, v_tex_start are u/q * w = u * w^2 ? No.
                // u_tex = u * w. u in span is u/w.
                // u_tex_start is the actual texture coordinate at start of span.

                // Derivatives at start of span:
                // q_start is q at start of span.
                let w = w_start; // 1/q
                let w_sq = w * w;

                // Derivatives of u_tex w.r.t screen X
                // du_tex/dx = (du/dx * q - u * dq/dx) / q^2
                // gradients.du_dx is du/dx for the variable u (which is U/W).
                // Wait.
                // In setup: u is U/W. q is 1/W.
                // Texture coord U_tex = u / q.
                // d(u/q)/dx = (u'q - uq')/q^2
                // u' = gradients.du_dx. q' = gradients.dq_dx.

                // Calculate X derivatives
                let du_tex_dx = (gradients.du_dx * q - u * gradients.dq_dx) * w_sq;
                let dv_tex_dx = (gradients.dv_dx * q - v * gradients.dq_dx) * w_sq;

                // Calculate Y derivatives
                let du_tex_dy = (gradients.du_dy * q - u * gradients.dq_dy) * w_sq;
                let dv_tex_dy = (gradients.dv_dy * q - v * gradients.dq_dy) * w_sq;

                let max_rho_sq = (du_tex_dx*du_tex_dx + dv_tex_dx*dv_tex_dx).max(
                                 du_tex_dy*du_tex_dy + dv_tex_dy*dv_tex_dy);

                let lod = 0.5 * max_rho_sq.log2();

                // Interpolate
                let mut u_fix = (u_tex_start * 65536.0) as i32;
                let mut v_fix = (v_tex_start * 65536.0) as i32;
                let du_fix = (du_tex_step * 65536.0) as i32;
                let dv_fix = (dv_tex_step * 65536.0) as i32;

                for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
                    if z < *depth_val {
                        *depth_val = z;
                        let u_float = (u_fix as f32) / 65536.0;
                        let v_float = (v_fix as f32) / 65536.0;
                        *pixel = texture.get_pixel_trilinear(u_float, v_float, lod);
                    }
                    z += gradients.dz_dx;
                    u_fix = u_fix.wrapping_add(du_fix);
                    v_fix = v_fix.wrapping_add(dv_fix);
                }
            }
        }

        // Advance state
        q = q_end;
        u = u_end;
        v = v_end;

        // Reuse end values for next start
        u_tex_start = u_tex_end;
        v_tex_start = v_tex_end;

        x += count;
    }
}

/// Fill a 3D triangle with perspective-correct texture mapping
pub fn fill_triangle_textured(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec2), // (Position, W), UV
    v1: ((Vec3, f32), Vec2),
    v2: ((Vec3, f32), Vec2),
    texture: &Texture,
) {
    assert_same_dimensions(fb, zb);

    let clipped = clip_triangle_to_frustum(v0, v1, v2, |v| v.0);

    for i in 0..clipped.count {
        let base = i * 3;
        let v0 = clipped.tris[base];
        let v1 = clipped.tris[base + 1];
        let v2 = clipped.tris[base + 2];

        let width = fb.width();
        let height = fb.height();

        // Project to screen
        let p0_orig = project_to_screen(v0.0.0, v0.0.1, width, height);
        let p1_orig = project_to_screen(v1.0.0, v1.0.1, width, height);
        let p2_orig = project_to_screen(v2.0.0, v2.0.1, width, height);

        // Backface Culling
        let ux_orig = (i64::from(p1_orig.x) - i64::from(p0_orig.x)) as f32;
        let uy_orig = (i64::from(p1_orig.y) - i64::from(p0_orig.y)) as f32;
        let vx_orig = (i64::from(p2_orig.x) - i64::from(p0_orig.x)) as f32;
        let vy_orig = (i64::from(p2_orig.y) - i64::from(p0_orig.y)) as f32;
        let nz_orig = ux_orig * vy_orig - uy_orig * vx_orig;

        if nz_orig >= 0.0 {
            continue;
        }

        // Prepare perspective attributes: q=1/w, u/w, v/w
        // Note: We multiply UV by texture dimensions here so interpolation happens in texel space
        let w0 = v0.0.1;
        let w1 = v1.0.1;
        let w2 = v2.0.1;

        // Avoid division by zero
        let inv_w0 = if w0.abs() > 0.0001 { 1.0 / w0 } else { 1.0 };
        let inv_w1 = if w1.abs() > 0.0001 { 1.0 / w1 } else { 1.0 };
        let inv_w2 = if w2.abs() > 0.0001 { 1.0 / w2 } else { 1.0 };

        let u0 = v0.1.x * texture.width as f32 * inv_w0;
        let v0_val = v0.1.y * texture.height as f32 * inv_w0;

        let u1 = v1.1.x * texture.width as f32 * inv_w1;
        let v1_val = v1.1.y * texture.height as f32 * inv_w1;

        let u2 = v2.1.x * texture.width as f32 * inv_w2;
        let v2_val = v2.1.y * texture.height as f32 * inv_w2;

        // Sort by y
        // We need to keep track of all attributes (p, q, u, v)
        let mut verts = [
            (p0_orig, inv_w0, u0, v0_val),
            (p1_orig, inv_w1, u1, v1_val),
            (p2_orig, inv_w2, u2, v2_val),
        ];
        sort_by_y(&mut verts, |(p, _, _, _)| p.y);
        let [(p0, q0, u0, v0), (p1, q1, u1, v1), (p2, q2, u2, v2)] = verts;

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
            let g =
                PerspectiveTextureGradients::new(p0, p1, p2, q0, q1, q2, u0, u1, u2, v0, v1, v2);

            let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
            let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
            let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
            let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
            let left = ux * vy - uy * vx > 0.0;

            (g, left)
        };

        let mut edge_a = PerspectiveTextureEdgeWalker::new(p0, p2, q0, q2, u0, u2, v0, v2);
        if y_start > p0.y {
            edge_a.step_n(y_start - p0.y);
        }

        let mut edge_b = if y_start < p1.y {
            let mut e = PerspectiveTextureEdgeWalker::new(p0, p1, q0, q1, u0, u1, v0, v1);
            if y_start > p0.y {
                e.step_n(y_start - p0.y);
            }
            e
        } else {
            let mut e = PerspectiveTextureEdgeWalker::new(p1, p2, q1, q2, u1, u2, v1, v2);
            if y_start > p1.y {
                e.step_n(y_start - p1.y);
            }
            e
        };

        let width_i32 = width as i32;

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b = PerspectiveTextureEdgeWalker::new(p1, p2, q1, q2, u1, u2, v1, v2);
            }

            let (x_start, x_end, z_left, q_left, u_left, v_left) = if long_edge_is_left {
                (
                    (edge_a.x >> 16) as i32,
                    (edge_b.x >> 16) as i32,
                    edge_a.z,
                    edge_a.q,
                    edge_a.u,
                    edge_a.v,
                )
            } else {
                (
                    (edge_b.x >> 16) as i32,
                    (edge_a.x >> 16) as i32,
                    edge_b.z,
                    edge_b.q,
                    edge_b.u,
                    edge_b.v,
                )
            };

            let dx = i64::from(x_end) - i64::from(x_start);

            if dx <= 0 {
                if x_start >= 0
                    && x_start < width_i32
                    && zb.test_and_set(x_start, y, z_left)
                    && q_left.abs() > 0.000_001
                {
                    let w = 1.0 / q_left;
                    let u_tex = u_left * w;
                    let v_tex = v_left * w;
                    let color = match texture.filter_mode {
                        FilterMode::Nearest => texture.get_pixel_texel(u_tex as i32, v_tex as i32),
                        FilterMode::Bilinear => texture.get_pixel_bilinear_texel(u_tex, v_tex),
                        FilterMode::Trilinear => {
                            // Calculate LOD for single pixel
                            // q = 1/w.
                            // u_tex = u/q.
                            // du_tex/dx = (du/dx * q - u * dq/dx) / q^2
                            let w = 1.0 / q_left;
                            let w_sq = w * w;

                            let du_tex_dx = (gradients.du_dx * q_left - u_left * gradients.dq_dx) * w_sq;
                            let dv_tex_dx = (gradients.dv_dx * q_left - v_left * gradients.dq_dx) * w_sq;
                            let du_tex_dy = (gradients.du_dy * q_left - u_left * gradients.dq_dy) * w_sq;
                            let dv_tex_dy = (gradients.dv_dy * q_left - v_left * gradients.dq_dy) * w_sq;

                            let max_rho_sq = (du_tex_dx*du_tex_dx + dv_tex_dx*dv_tex_dx).max(
                                             du_tex_dy*du_tex_dy + dv_tex_dy*dv_tex_dy);

                            let lod = 0.5 * max_rho_sq.log2();
                            texture.get_pixel_trilinear(u_tex, v_tex, lod)
                        }
                    };
                    fb.set_pixel(x_start, y, color);
                }
            } else {
                draw_scanline_textured_perspective(
                    fb,
                    zb,
                    texture,
                    y,
                    x_start,
                    x_end,
                    PerspectiveSpanStart {
                        z: z_left,
                        q: q_left,
                        u: u_left,
                        v: v_left,
                    },
                    &gradients,
                );
            }

            edge_a.step();
            edge_b.step();
        }
    }
}
