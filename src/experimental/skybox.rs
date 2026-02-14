//! Skybox rendering module.
//!
//! Implements a Skybox using a Cubemap texture.
//! The skybox is rendered as a unit cube centered on the camera,
//! with "infinite" depth (z=1.0) to serve as a background.

use crate::clipping::clip_triangle_to_frustum;
use crate::framebuffer::Framebuffer;
use crate::math::{Mat4, ScreenPoint, Vec3, project_triangle_to_screen};
use crate::rasterizer::sort_by_y;
use crate::texture::Texture;
use crate::zbuffer::ZBuffer;

/// A Cubemap texture consisting of 6 faces.
///
/// Faces are ordered: +X, -X, +Y, -Y, +Z, -Z.
/// (Right, Left, Top, Bottom, Front, Back).
pub struct Cubemap {
    pub faces: [Texture; 6],
}

impl Cubemap {
    /// Create a new Cubemap from 6 textures.
    #[must_use]
    pub const fn new(faces: [Texture; 6]) -> Self {
        Self { faces }
    }

    /// Sample the cubemap using a direction vector.
    #[must_use]
    pub fn sample(&self, dir: Vec3) -> u32 {
        let abs_x = dir.x.abs();
        let abs_y = dir.y.abs();
        let abs_z = dir.z.abs();

        let (face_idx, ma, sc, tc) = if abs_x >= abs_y && abs_x >= abs_z {
            let ma = abs_x;
            if dir.x > 0.0 {
                (0, ma, -dir.z, -dir.y) // +X (Right)
            } else {
                (1, ma, dir.z, -dir.y) // -X (Left)
            }
        } else if abs_y >= abs_x && abs_y >= abs_z {
            let ma = abs_y;
            if dir.y > 0.0 {
                (2, ma, dir.x, dir.z) // +Y (Top)
            } else {
                (3, ma, dir.x, -dir.z) // -Y (Bottom)
            }
        } else {
            let ma = abs_z;
            if dir.z > 0.0 {
                (4, ma, dir.x, -dir.y) // +Z (Front)
            } else {
                (5, ma, -dir.x, -dir.y) // -Z (Back)
            }
        };

        // Avoid division by zero
        if ma == 0.0 {
            return 0xFF00_0000;
        }

        // Map to [0, 1]
        let u = (sc / ma + 1.0) * 0.5;
        let v = (tc / ma + 1.0) * 0.5;

        self.faces[face_idx].get_pixel_bilinear(u, v)
    }
}

// --- Rasterization Helpers ---

const FIXED_SCALE: f32 = 65536.0;

#[derive(Clone, Copy)]
struct SkyboxGradients {
    dq_dx: f32,  // 1/w
    dvx_dx: f32, // dir.x/w
    dvy_dx: f32,
    dvz_dx: f32,
}

impl SkyboxGradients {
    #[allow(clippy::too_many_arguments)]
    fn new(
        p0: ScreenPoint,
        p1: ScreenPoint,
        p2: ScreenPoint,
        q0: f32,
        q1: f32,
        q2: f32,
        v0: Vec3,
        v1: Vec3,
        v2: Vec3,
    ) -> Self {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uq = q1 - q0;
        let uvx = v1.x - v0.x;
        let uvy = v1.y - v0.y;
        let uvz = v1.z - v0.z;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        let vq = q2 - q0;
        let vvx = v2.x - v0.x;
        let vvy = v2.y - v0.y;
        let vvz = v2.z - v0.z;

        let nz = ux * vy - uy * vx;
        let inv_nz = if nz.abs() > 0.0001 { -1.0 / nz } else { 0.0 };

        let nx_q = uy * vq - uq * vy;
        let dq_dx = nx_q * inv_nz;

        let nx_vx = uy * vvx - uvx * vy;
        let dvx_dx = nx_vx * inv_nz;

        let nx_vy = uy * vvy - uvy * vy;
        let dvy_dx = nx_vy * inv_nz;

        let nx_vz = uy * vvz - uvz * vy;
        let dvz_dx = nx_vz * inv_nz;

        Self {
            dq_dx,
            dvx_dx,
            dvy_dx,
            dvz_dx,
        }
    }
}

struct SkyboxEdgeWalker {
    x: i64,
    q: f32,
    vx: f32,
    vy: f32,
    vz: f32,
    dx_dy: i64,
    dq_dy: f32,
    dvx_dy: f32,
    dvy_dy: f32,
    dvz_dy: f32,
}

impl SkyboxEdgeWalker {
    fn new(
        p_start: ScreenPoint,
        p_end: ScreenPoint,
        q_start: f32,
        q_end: f32,
        v_start: Vec3,
        v_end: Vec3,
    ) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let inv_h = if height == 0.0 { 0.0 } else { 1.0 / height };

        let dx_dy =
            ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64;
        let dq_dy = (q_end - q_start) * inv_h;
        let dvx_dy = (v_end.x - v_start.x) * inv_h;
        let dvy_dy = (v_end.y - v_start.y) * inv_h;
        let dvz_dy = (v_end.z - v_start.z) * inv_h;

        Self {
            x: i64::from(p_start.x) << 16,
            q: q_start,
            vx: v_start.x,
            vy: v_start.y,
            vz: v_start.z,
            dx_dy,
            dq_dy,
            dvx_dy,
            dvy_dy,
            dvz_dy,
        }
    }

    fn step(&mut self) {
        self.x += self.dx_dy;
        self.q += self.dq_dy;
        self.vx += self.dvx_dy;
        self.vy += self.dvy_dy;
        self.vz += self.dvz_dy;
    }

    fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.q += self.dq_dy * n_f;
        self.vx += self.dvx_dy * n_f;
        self.vy += self.dvy_dy * n_f;
        self.vz += self.dvz_dy * n_f;
    }
}

struct SkyboxSpanStart {
    q: f32,
    vx: f32,
    vy: f32,
    vz: f32,
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_scanline_skybox(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    start: &SkyboxSpanStart,
    gradients: &SkyboxGradients,
    cubemap: &Cubemap,
) {
    let width = fb.width() as i32;
    let mut xs = x_start;
    let mut xe = x_end;

    let mut q = start.q;
    let mut vx = start.vx;
    let mut vy = start.vy;
    let mut vz = start.vz;

    if xs < 0 {
        let diff = -i64::from(xs);
        let diff_f = diff as f32;
        q += diff_f * gradients.dq_dx;
        vx += diff_f * gradients.dvx_dx;
        vy += diff_f * gradients.dvy_dx;
        vz += diff_f * gradients.dvz_dx;
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

    let z_far = 1.0; // Max depth

    for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
        // Skybox is strictly background, so we write if z_far < current_depth
        // But usually current_depth is INFINITY (if cleared), so 1.0 < INFINITY passes.
        // If scene is drawn, scene depth <= 1.0.
        // If scene depth == 1.0 (also far), we might want to overwrite or not.
        // Let's use LEQUAL behavior: z_far <= *depth_val.
        if z_far <= *depth_val {
            *depth_val = z_far;

            // Perspective recover
            let w_recip = if q.abs() > 0.000_001 { 1.0 / q } else { 1.0 };
            let dir = Vec3::new(vx * w_recip, vy * w_recip, vz * w_recip);

            // Sample cubemap
            *pixel = cubemap.sample(dir);
        }

        q += gradients.dq_dx;
        vx += gradients.dvx_dx;
        vy += gradients.dvy_dx;
        vz += gradients.dvz_dx;
    }
}

/// Fill a 3D triangle for Skybox rendering.
///
/// This interpolates the vertex position (as a direction vector) and samples the cubemap.
/// It writes depth = 1.0 (Far Plane).
///
/// # Panics
/// Panics if framebuffer and zbuffer dimensions do not match.
pub fn fill_triangle_skybox(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec3),
    v1: ((Vec3, f32), Vec3),
    v2: ((Vec3, f32), Vec3),
    cubemap: &Cubemap,
) {
    assert!(
        !(fb.width() != zb.width() || fb.height() != zb.height()),
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

        // Note: For Skybox (viewed from inside), we typically want to see "Backfaces".
        // Standard culling removes backfaces (nz >= 0).
        // Since we are inside the cube with normals pointing OUT, the faces we see are "Backfaces".
        // So we should NOT cull them, or we should cull Frontfaces.
        // Actually, let's just disable culling for Skybox to be safe.
        // If we strictly define winding, we can optimize.
        // But simply skipping the `is_backface` check works for "Double Sided".

        // Prepare attributes: q=1/w, dir/w
        let inv_w0 = p0_orig.inv_w;
        let inv_w1 = p1_orig.inv_w;
        let inv_w2 = p2_orig.inv_w;

        let d0 = v0.1 * inv_w0;
        let d1 = v1.1 * inv_w1;
        let d2 = v2.1 * inv_w2;

        let mut verts = [(p0_orig, d0), (p1_orig, d1), (p2_orig, d2)];
        sort_by_y(&mut verts, |(p, _)| p.y);
        let [(p0, d0), (p1, d1), (p2, d2)] = verts;

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

        let gradients = SkyboxGradients::new(p0, p1, p2, q0, q1, q2, d0, d1, d2);

        // Winding order check for edge walking (left/right)
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        let long_edge_is_left = ux * vy - uy * vx > 0.0;

        let mut edge_a = SkyboxEdgeWalker::new(p0, p2, q0, q2, d0, d2);
        if y_start > p0.y {
            edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
        }

        let mut edge_b = if y_start < p1.y {
            let mut e = SkyboxEdgeWalker::new(p0, p1, q0, q1, d0, d1);
            if y_start > p0.y {
                e.step_n(i64::from(y_start) - i64::from(p0.y));
            }
            e
        } else {
            let mut e = SkyboxEdgeWalker::new(p1, p2, q1, q2, d1, d2);
            if y_start > p1.y {
                e.step_n(i64::from(y_start) - i64::from(p1.y));
            }
            e
        };

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b = SkyboxEdgeWalker::new(p1, p2, q1, q2, d1, d2);
            }

            let (x_start, x_end, q_left, vx_left, vy_left, vz_left) = if long_edge_is_left {
                (
                    (edge_a.x >> 16) as i32,
                    (edge_b.x >> 16) as i32,
                    edge_a.q,
                    edge_a.vx,
                    edge_a.vy,
                    edge_a.vz,
                )
            } else {
                (
                    (edge_b.x >> 16) as i32,
                    (edge_a.x >> 16) as i32,
                    edge_b.q,
                    edge_b.vx,
                    edge_b.vy,
                    edge_b.vz,
                )
            };

            let dx = i64::from(x_end) - i64::from(x_start);

            if dx > 0 {
                draw_scanline_skybox(
                    fb,
                    zb,
                    y,
                    x_start,
                    x_end,
                    &SkyboxSpanStart {
                        q: q_left,
                        vx: vx_left,
                        vy: vy_left,
                        vz: vz_left,
                    },
                    &gradients,
                    cubemap,
                );
            }

            edge_a.step();
            edge_b.step();
        }
    }
}

/// Helper to render a Skybox.
///
/// This constructs a unit cube and rasterizes it.
/// It modifies the view matrix to remove translation, ensuring the skybox stays centered.
pub fn draw_skybox(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    view: Mat4,
    proj: Mat4,
    cubemap: &Cubemap,
) {
    // 1. Remove translation from View Matrix
    let mut view_centered = view;
    view_centered.m[3][0] = 0.0;
    view_centered.m[3][1] = 0.0;
    view_centered.m[3][2] = 0.0;

    let view_proj = view_centered * proj;

    // 2. Define Unit Cube Vertices (centered at 0,0,0)
    // 8 vertices
    let verts = [
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(1.0, -1.0, -1.0),
        Vec3::new(1.0, 1.0, -1.0),
        Vec3::new(-1.0, 1.0, -1.0),
        Vec3::new(-1.0, -1.0, 1.0),
        Vec3::new(1.0, -1.0, 1.0),
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(-1.0, 1.0, 1.0),
    ];

    // 3. Define Indices for 12 triangles (CW winding to be visible from inside?)
    // Or just double-sided (since we disabled backface culling).
    // Let's use standard cube indices.
    let indices = [
        // Front
        0, 1, 2, 0, 2, 3, // Back
        5, 4, 7, 5, 7, 6, // Left
        4, 0, 3, 4, 3, 7, // Right
        1, 5, 6, 1, 6, 2, // Top
        3, 2, 6, 3, 6, 7, // Bottom
        4, 5, 1, 4, 1, 0,
    ];

    for chunk in indices.chunks(3) {
        let i0 = chunk[0];
        let i1 = chunk[1];
        let i2 = chunk[2];

        let v0_local = verts[i0];
        let v1_local = verts[i1];
        let v2_local = verts[i2];

        // Transform
        // We pass the local position as the "Direction" vector for sampling
        // (normalized automatically by sampling logic, so magnitude doesn't matter much)
        // Clip Space position
        let (p0_clip, w0) = view_proj.transform_point(v0_local);
        let (p1_clip, w1) = view_proj.transform_point(v1_local);
        let (p2_clip, w2) = view_proj.transform_point(v2_local);

        fill_triangle_skybox(
            fb,
            zb,
            ((p0_clip, w0), v0_local),
            ((p1_clip, w1), v1_local),
            ((p2_clip, w2), v2_local),
            cubemap,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cubemap_sampling_directions() {
        // Create dummy texture with 1 pixel
        let white_tex = || {
            let mut t = Texture::new(1, 1).unwrap();
            t.pixels[0] = 0xFFFFFFFF;
            t
        };
        let black_tex = || Texture::new(1, 1).unwrap();

        // Map +X (Right) to White, others Black
        let faces = [
            white_tex(), // +X
            black_tex(),
            black_tex(),
            black_tex(),
            black_tex(),
            black_tex(),
        ];
        let cubemap = Cubemap::new(faces);

        // Sample +X direction
        let color = cubemap.sample(Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(color, 0xFFFFFFFF);

        // Sample -X direction
        let color = cubemap.sample(Vec3::new(-1.0, 0.0, 0.0));
        assert_eq!(color, 0xFF000000);
    }

    #[test]
    fn test_draw_skybox_integration() {
        let width = 10;
        let height = 10;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        let faces = [
            Texture::new(1, 1).unwrap(),
            Texture::new(1, 1).unwrap(),
            Texture::new(1, 1).unwrap(),
            Texture::new(1, 1).unwrap(),
            Texture::new(1, 1).unwrap(),
            Texture::new(1, 1).unwrap(),
        ];
        let cubemap = Cubemap::new(faces);

        let view = Mat4::identity();
        let proj = Mat4::perspective(1.57, 1.0, 0.1, 100.0);

        // Should not panic
        draw_skybox(&mut fb, &mut zb, view, proj, &cubemap);
    }
}
