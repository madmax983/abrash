//! Holographic Projection System 🌟
//!
//! A specialized rasterizer for rendering meshes with a sci-fi hologram effect.
//!
//! Features:
//! *   **Wireframe-ish Look**: Semi-transparent, additive blending.
//! *   **Scanlines**: Horizontal bands of varying intensity based on Y coordinate and time.
//! *   **Glitch/Jitter**: Random vertex displacement for a "bad connection" vibe.
//! *   **Ghosting**: Optional double rendering (not implemented in v1).

use crate::clipping::clip_triangle_to_frustum;
use crate::framebuffer::Framebuffer;
use crate::math::{ScreenPoint, Vec3, project_triangle_to_screen};
use crate::rasterizer::core::{FIXED_SCALE, is_backface, sort_by_y};
use crate::utils::XorShift32;
use crate::zbuffer::ZBuffer;

/// Parameters controlling the hologram effect.
#[derive(Clone, Copy, Debug)]
pub struct HologramParams {
    /// Base color of the hologram (RGB). Alpha is ignored (additive blend).
    pub color: u32,
    /// Base intensity (alpha equivalent) 0.0 - 1.0.
    pub intensity: f32,
    /// Speed of the scanline movement.
    pub scan_speed: f32,
    /// Density of scanlines.
    pub scan_density: f32,
    /// Amount of random vertex jitter (in clip space).
    pub jitter_amount: f32,
    /// Current simulation time.
    pub time: f32,
}

impl Default for HologramParams {
    fn default() -> Self {
        Self {
            color: 0x0000FFFF, // Cyan
            intensity: 0.8,
            scan_speed: 2.0,
            scan_density: 10.0,
            jitter_amount: 0.05,
            time: 0.0,
        }
    }
}

/// Renders a mesh as a hologram.
pub fn draw_holographic_mesh(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    mesh: &crate::mesh::Mesh,
    mvp: crate::math::Mat4,
    params: HologramParams,
) {
    let half_width = fb.width() as f32 * 0.5;
    let half_height = fb.height() as f32 * 0.5;

    // Use a deterministic RNG for jitter consistency per frame if needed,
    // but here we want jitter to change every frame based on time?
    // Actually, usually jitter is random per frame.
    // We can use the params.time as a seed offset if we want meaningful jitter.
    let mut rng = XorShift32::new((params.time * 1000.0) as u32);

    for tri in &mesh.indices {
        let v0_local = mesh.vertices[tri[0]];
        let v1_local = mesh.vertices[tri[1]];
        let v2_local = mesh.vertices[tri[2]];

        // Transform to Clip Space
        let (mut c0, w0) = mvp.transform_point(v0_local);
        let (mut c1, w1) = mvp.transform_point(v1_local);
        let (mut c2, w2) = mvp.transform_point(v2_local);

        // Apply Jitter (in Clip Space)
        if params.jitter_amount > 0.0 {
            // Random glitch threshold (e.g., only jitter 10% of frames/vertices)
            // But here we jitter everything slightly or glitch occasionally.
            // Let's jitter everything slightly.
            let j = params.jitter_amount;
            c0.x += rng.next_f32_signed() * j;
            c0.y += rng.next_f32_signed() * j;
            c1.x += rng.next_f32_signed() * j;
            c1.y += rng.next_f32_signed() * j;
            c2.x += rng.next_f32_signed() * j;
            c2.y += rng.next_f32_signed() * j;
        }

        // Clip
        let clipped = clip_triangle_to_frustum(
            (c0, w0),
            (c1, w1),
            (c2, w2),
            |v| *v, // Interpolator (just the vertex itself)
        );

        for i in 0..clipped.count {
            let base = i * 3;
            let v0 = clipped.tris[base];
            let v1 = clipped.tris[base + 1];
            let v2 = clipped.tris[base + 2];

            // Project to Screen
            let (p0, p1, p2) = project_triangle_to_screen(
                v0.0, v0.1,
                v1.0, v1.1,
                v2.0, v2.1,
                half_width, half_height,
            );

            // Backface Culling (Optional for holograms, they might be double-sided)
            // But for now, let's cull to save fill rate.
            if is_backface(p0, p1, p2) {
                continue;
            }

            fill_triangle_hologram(fb, zb, p0, p1, p2, &params);
        }
    }
}

fn fill_triangle_hologram(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    p0: ScreenPoint,
    p1: ScreenPoint,
    p2: ScreenPoint,
    params: &HologramParams,
) {
    // Sort vertices by Y
    let mut verts = [p0, p1, p2];
    sort_by_y(&mut verts, |p| p.y);
    let [p0, p1, p2] = verts;

    let height = fb.height();
    let total_height = (i64::from(p2.y) - i64::from(p0.y)) as f32;
    if total_height == 0.0 {
        return;
    }

    let y_min = 0;
    let y_max = height as i32 - 1;
    let y_start = p0.y.max(y_min);
    let y_end = p2.y.min(y_max);

    if y_start > y_end {
        return;
    }

    // Edge Walkers
    // We only need X and Z.
    // Actually, we don't even need Z for the Z-Buffer check if we want to be "always on top"
    // or if we want to respect depth.
    // Let's respect depth (Z-Test) but not Write Depth (transparent).

    // We can reuse EdgeWalker from core, but it interpolates 1/w (q), u/w, v/w, etc.
    // We just need X and Z.
    // Core EdgeWalker is generic? No, it's specific to what it carries.
    // Let's look at `rasterizer::core::EdgeWalker` struct definition.
    // It's not generic. It usually carries just X (and maybe attributes in specific versions).
    // `rasterizer::core::EdgeWalker` is likely just X?
    // Let's double check via memory or file.
    // In `gouraud.rs`, `GouraudEdgeWalker` was used.
    // In `texture.rs`, `PerspectiveTextureEdgeWalker`.
    // In `core.rs`, `EdgeWalker` struct definition?

    // Let's assume I need to make a simple one or use `core::EdgeWalker` if it exists and fits.
    // I'll make a local one to be safe and dependency-free.
    struct SimpleEdgeWalker {
        x: i64,
        z: f32,
        dx_dy: i64,
        dz_dy: f32,
    }

    impl SimpleEdgeWalker {
        fn new(p_start: ScreenPoint, p_end: ScreenPoint) -> Self {
            let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
            let inv_h = if height == 0.0 { 0.0 } else { 1.0 / height };

            let dx_dy = ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64;
            let dz_dy = (p_end.z - p_start.z) * inv_h;

            Self {
                x: i64::from(p_start.x) << 16,
                z: p_start.z,
                dx_dy,
                dz_dy,
            }
        }

        fn step(&mut self) {
            self.x += self.dx_dy;
            self.z += self.dz_dy;
        }

        fn step_n(&mut self, n: i64) {
            self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
            self.z += self.dz_dy * (n as f32);
        }
    }

    // Determine long edge
    // Check handedness to determine which side is the long edge
    let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
    let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
    let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
    let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
    let cross_product = ux * vy - uy * vx;
    let long_edge_is_left = cross_product > 0.0;

    // Z gradient
    // Plane equation: z = A*x + B*y + C
    // We need dz/dx to interpolate Z across the scanline.
    // (Similar to gradients in other rasterizers)
    let nz = cross_product; // This is actually the Z component of the normal vector in screen space (x, y, 1) cross (x, y, 1)? No.
    // It's the 2D cross product.

    let inv_nz = if nz.abs() > 0.0001 { -1.0 / nz } else { 0.0 };
    let uz = p1.z - p0.z;
    let vz = p2.z - p0.z;
    // nx_z = uy * vz - uz * vy
    let nx_z = uy * vz - uz * vy;
    let dz_dx = nx_z * inv_nz;

    let mut edge_a = SimpleEdgeWalker::new(p0, p2);
    if y_start > p0.y {
        edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
    }

    let mut edge_b = if y_start < p1.y {
        let mut e = SimpleEdgeWalker::new(p0, p1);
        if y_start > p0.y {
            e.step_n(i64::from(y_start) - i64::from(p0.y));
        }
        e
    } else {
        let mut e = SimpleEdgeWalker::new(p1, p2);
        if y_start > p1.y {
            e.step_n(i64::from(y_start) - i64::from(p1.y));
        }
        e
    };

    let width = fb.width() as i32;

    for y in y_start..=y_end {
        if y == p1.y && y != p0.y {
            edge_b = SimpleEdgeWalker::new(p1, p2);
        }

        let (x_start, x_end, z_start) = if long_edge_is_left {
            ((edge_a.x >> 16) as i32, (edge_b.x >> 16) as i32, edge_a.z)
        } else {
            ((edge_b.x >> 16) as i32, (edge_a.x >> 16) as i32, edge_b.z)
        };

        // Clamping
        let xs = x_start.max(0);
        let xe = x_end.min(width - 1);

        if xs <= xe {
            // Adjust Z start for clamping
            let z_curr = z_start + (xs - x_start) as f32 * dz_dx;

            draw_scanline_hologram(fb, zb, y, xs, xe, z_curr, dz_dx, params);
        }

        edge_a.step();
        edge_b.step();
    }
}

fn draw_scanline_hologram(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    z_start: f32,
    dz_dx: f32,
    params: &HologramParams,
) {
    let mut z = z_start;
    let width_usize = fb.width() as usize;
    let y_offset = (y as usize) * width_usize;

    // Scanline Effect
    // Calculate a factor 0.0 - 1.0 based on Y and Time.
    // Sin wave moving down.
    let scan_y = y as f32 * params.scan_density + params.time * params.scan_speed * 10.0;
    let scan_factor = (scan_y.sin() + 1.0) * 0.5; // 0..1

    // Mix intensity
    // Base intensity + scanline boost
    let alpha_float = params.intensity * (0.8 + 0.4 * scan_factor);
    let alpha = (alpha_float * 255.0).clamp(0.0, 255.0) as u32;

    let r_src = (params.color >> 16) & 0xFF;
    let g_src = (params.color >> 8) & 0xFF;
    let b_src = params.color & 0xFF;

    // Pre-calculate weighted color
    let r_add = (r_src * alpha) >> 8;
    let g_add = (g_src * alpha) >> 8;
    let b_add = (b_src * alpha) >> 8;

    for x in x_start..=x_end {
        let idx = y_offset + x as usize;

        // Z-Test (ReadOnly)
        // If current Z < ZBuffer, we are visible (or strictly less?)
        // Standard is <.
        // Holograms are transparent, so they should be occluded by solid objects.
        // But they should NOT write to ZBuffer.

        let depth = zb.as_slice()[idx];
        if z < depth {
            // Additive Blend
            let dest = fb.as_slice()[idx];
            let r_dst = (dest >> 16) & 0xFF;
            let g_dst = (dest >> 8) & 0xFF;
            let b_dst = dest & 0xFF;

            // Additive: dst + src
            let r_out = (r_dst + r_add).min(255);
            let g_out = (g_dst + g_add).min(255);
            let b_out = (b_dst + b_add).min(255);

            // Preserve destination alpha? Or set to 255?
            // Usually screen is 255 alpha.
            let a_out = 255;

            fb.as_mut_slice()[idx] = (a_out << 24) | (r_out << 16) | (g_out << 8) | b_out;
        }

        z += dz_dx;
    }
}
