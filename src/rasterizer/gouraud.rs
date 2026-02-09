//! Gouraud shading rasterization.

use crate::clipping::clip_triangle_to_frustum;
use crate::framebuffer::Framebuffer;
use crate::math::{ScreenPoint, Vec3, project_to_screen};
use crate::zbuffer::ZBuffer;

use super::common::{FIXED_SCALE, assert_same_dimensions, is_backface, pack_color_fixed, sort_by_y};

/// Draw a single scanline for Gouraud shading
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_scanline_gouraud(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    z_start: f32,
    c_start: (i64, i64, i64), // Fixed point color
    dz_dx: f32,
    dc_dx: (i32, i32, i32),
) {
    let width = fb.width() as i32;
    let mut xs = x_start;
    let mut xe = x_end;
    let mut z = z_start;

    // Use i64 for accumulators to prevent overflow when x_start is far off-screen
    let mut r_i = c_start.0;
    let mut g_i = c_start.1;
    let mut b_i = c_start.2;
    let (dr, dg, db) = (i64::from(dc_dx.0), i64::from(dc_dx.1), i64::from(dc_dx.2));

    // Clamp to screen bounds
    if xs < 0 {
        let diff = -i64::from(xs);
        z += (diff as f32) * dz_dx;
        let diff_i64 = diff;
        r_i += diff_i64 * dr;
        g_i += diff_i64 * dg;
        b_i += diff_i64 * db;
        xs = 0;
    }

    if xe >= width {
        xe = width - 1;
    }

    // Optimization: Demote to i32 for the hot loop to reduce register pressure.
    // We used i64 above to handle large off-screen jumps safely without overflow.
    // Once on-screen, 16.16 fixed point color fits comfortably in i32.
    // (Max value ~255 * 65536 = 1.6e7 << i32::MAX)
    let mut r_i = r_i as i32;
    let mut g_i = g_i as i32;
    let mut b_i = b_i as i32;
    let dr = dr as i32;
    let dg = dg as i32;
    let db = db as i32;

    if xs <= xe {
        // Optimization: Use slice iterators to avoid index recalculation and bounds checks in the loop
        let width_usize = fb.width() as usize;
        let y_offset = (y as usize) * width_usize;
        let start_idx = y_offset + (xs as usize);
        let end_idx = y_offset + (xe as usize);

        // SAFETY:
        // 1. xs and xe are clamped to [0, width-1] by the logic above.
        // 2. y is clamped to [0, height-1] by the caller (fill_triangle_gouraud).
        // 3. We checked `xs <= xe` immediately above, so `start_idx <= end_idx`.
        // Therefore, the range is valid and within bounds.
        let fb_slice = unsafe { fb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };
        let zb_slice = unsafe { zb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };

        for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
            // Check depth buffer
            if z < *depth_val {
                *depth_val = z;
                // Unpack fixed point color
                // Optimization: Combine clamp and mask to avoid shifts and intermediate u8 casts
                // 16.16 fixed point means 255.0 is 0x00FF0000
                let r = r_i.clamp(0, 0x00FF_0000);
                let g = g_i.clamp(0, 0x00FF_0000);
                let b = b_i.clamp(0, 0x00FF_0000);

                *pixel = 0xFF00_0000
                    | ((r as u32) & 0x00FF_0000)
                    | (((g as u32) & 0x00FF_0000) >> 8)
                    | (((b as u32) & 0x00FF_0000) >> 16);
            }
            z += dz_dx;
            r_i += dr;
            g_i += dg;
            b_i += db;
        }
    }
}

struct GouraudGradients {
    dz_dx: f32,
    dc_dx: (i32, i32, i32),
}

impl GouraudGradients {
    fn new(
        p0: ScreenPoint,
        p1: ScreenPoint,
        p2: ScreenPoint,
        c0: Vec3,
        c1: Vec3,
        c2: Vec3,
    ) -> Self {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;
        let uc = c1 - c0;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        let vz = p2.z - p0.z;
        let vc = c2 - c0;

        let nz = ux * vy - uy * vx;
        let inv_nz = if nz.abs() > 0.0001 { -1.0 / nz } else { 0.0 };

        let nx_z = uy * vz - uz * vy;
        let dz_dx = nx_z * inv_nz;

        let nx_r = uy * vc.x - uc.x * vy;
        let nx_g = uy * vc.y - uc.y * vy;
        let nx_b = uy * vc.z - uc.z * vy;

        let dr = nx_r * inv_nz;
        let dg = nx_g * inv_nz;
        let db = nx_b * inv_nz;

        let dr_i = (dr * FIXED_SCALE) as i32;
        let dg_i = (dg * FIXED_SCALE) as i32;
        let db_i = (db * FIXED_SCALE) as i32;

        Self {
            dz_dx,
            dc_dx: (dr_i, dg_i, db_i),
        }
    }

    fn is_long_edge_left(p0: ScreenPoint, p1: ScreenPoint, p2: ScreenPoint) -> bool {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        ux * vy - uy * vx > 0.0
    }
}

struct GouraudEdgeWalker {
    x: i64,
    z: f32,
    c: (i64, i64, i64),
    dx_dy: i64,
    dz_dy: f32,
    dc_dy: (i64, i64, i64),
}

impl GouraudEdgeWalker {
    fn new(p_start: ScreenPoint, p_end: ScreenPoint, c_start: Vec3, c_end: Vec3) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let (dx_dy, dz_dy, dc_dy) = if height == 0.0 {
            (0, 0.0, (0, 0, 0))
        } else {
            let inv_h = 1.0 / height;
            let dc = (c_end - c_start) * inv_h;
            (
                ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64,
                (p_end.z - p_start.z) * inv_h,
                (
                    (dc.x * FIXED_SCALE) as i64,
                    (dc.y * FIXED_SCALE) as i64,
                    (dc.z * FIXED_SCALE) as i64,
                ),
            )
        };

        let c_fixed = (
            (c_start.x * FIXED_SCALE) as i64,
            (c_start.y * FIXED_SCALE) as i64,
            (c_start.z * FIXED_SCALE) as i64,
        );

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            c: c_fixed,
            dx_dy,
            dz_dy,
            dc_dy,
        }
    }

    fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.c.0 += self.dc_dy.0;
        self.c.1 += self.dc_dy.1;
        self.c.2 += self.dc_dy.2;
    }

    fn step_n(&mut self, n: i32) {
        let n_f = n as f32;
        let n_i64 = i64::from(n);
        self.x += self.dx_dy * n_i64;
        self.z += self.dz_dy * n_f;
        self.c.0 += self.dc_dy.0 * n_i64;
        self.c.1 += self.dc_dy.1 * n_i64;
        self.c.2 += self.dc_dy.2 * n_i64;
    }
}

/// Fill a 3D triangle with Gouraud (per-vertex) shading
/// Each vertex has a position (clip space + w) and color
pub fn fill_triangle_gouraud(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec3), // ((position, w), color)
    v1: ((Vec3, f32), Vec3),
    v2: ((Vec3, f32), Vec3),
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
        if is_backface(p0_orig, p1_orig, p2_orig) {
            continue;
        }

        // Optimization: Pre-scale colors to 0..255 for faster interpolation and packing
        // allowing us to skip clamp/mul per pixel
        let c0 = v0.1 * 255.0;
        let c1 = v1.1 * 255.0;
        let c2 = v2.1 * 255.0;

        // Sort by y
        let mut verts = [(p0_orig, c0), (p1_orig, c1), (p2_orig, c2)];
        sort_by_y(&mut verts, |(p, _)| p.y);
        let [(p0, c0), (p1, c1), (p2, c2)] = verts;

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
            let g = GouraudGradients::new(p0, p1, p2, c0, c1, c2);
            let left = GouraudGradients::is_long_edge_left(p0, p1, p2);
            (g, left)
        };

        let mut edge_a = GouraudEdgeWalker::new(p0, p2, c0, c2);
        if y_start > p0.y {
            edge_a.step_n(y_start - p0.y);
        }

        let mut edge_b = if y_start < p1.y {
            let mut e = GouraudEdgeWalker::new(p0, p1, c0, c1);
            if y_start > p0.y {
                e.step_n(y_start - p0.y);
            }
            e
        } else {
            let mut e = GouraudEdgeWalker::new(p1, p2, c1, c2);
            if y_start > p1.y {
                e.step_n(y_start - p1.y);
            }
            e
        };

        let width_i32 = width as i32;

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b = GouraudEdgeWalker::new(p1, p2, c1, c2);
            }

            let (x_start, x_end, z_left, c_left) = if long_edge_is_left {
                (
                    (edge_a.x >> 16) as i32,
                    (edge_b.x >> 16) as i32,
                    edge_a.z,
                    edge_a.c,
                )
            } else {
                (
                    (edge_b.x >> 16) as i32,
                    (edge_a.x >> 16) as i32,
                    edge_b.z,
                    edge_b.c,
                )
            };

            let dx = i64::from(x_end) - i64::from(x_start);

            if dx <= 0 {
                if x_start >= 0 && x_start < width_i32 && zb.test_and_set(x_start, y, z_left) {
                    fb.set_pixel(x_start, y, pack_color_fixed(c_left));
                }
            } else {
                draw_scanline_gouraud(
                    fb,
                    zb,
                    y,
                    x_start,
                    x_end,
                    z_left,
                    c_left,
                    gradients.dz_dx,
                    gradients.dc_dx,
                );
            }

            edge_a.step();
            edge_b.step();
        }
    }
}
