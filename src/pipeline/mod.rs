//! 3D Rendering pipeline.
//!
//! Functions for projecting and rasterizing 3D primitives (triangles) with shading.

use crate::framebuffer::Framebuffer;
use crate::light::{AmbientLight, DirectionalLight, color_to_u32};
use crate::math::Vec3;
use crate::zbuffer::ZBuffer;

pub mod projection;
pub use projection::{ScreenPoint, project_to_screen};

/// Vertex with position and homogeneous W component.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipVertex {
    pub position: Vec3,
    pub w: f32,
}

impl ClipVertex {
    pub fn new(position: Vec3, w: f32) -> Self {
        Self { position, w }
    }
}

/// Vertex with position, homogeneous W component, and color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GouraudVertex {
    pub position: Vec3,
    pub w: f32,
    pub color: Vec3,
}

impl GouraudVertex {
    pub fn new(position: Vec3, w: f32, color: Vec3) -> Self {
        Self { position, w, color }
    }
}

impl From<((Vec3, f32), Vec3)> for GouraudVertex {
    fn from(v: ((Vec3, f32), Vec3)) -> Self {
        Self {
            position: v.0.0,
            w: v.0.1,
            color: v.1,
        }
    }
}

struct GouraudGradients {
    dz_dx: f32,
    dc_dx: Vec3,
}

impl GouraudGradients {
    fn new(
        p0: ScreenPoint,
        c0: Vec3,
        p1: ScreenPoint,
        c1: Vec3,
        p2: ScreenPoint,
        c2: Vec3,
    ) -> Self {
        let ux = (p1.x as i64 - p0.x as i64) as f32;
        let uy = (p1.y as i64 - p0.y as i64) as f32;
        let uz = p1.z - p0.z;
        let uc = c1 - c0;

        let vx = (p2.x as i64 - p0.x as i64) as f32;
        let vy = (p2.y as i64 - p0.y as i64) as f32;
        let vz = p2.z - p0.z;
        let vc = c2 - c0;

        // Cross product Z component (signed area)
        let nz = ux * vy - uy * vx;

        let inv_nz = if nz.abs() > 0.0001 { -1.0 / nz } else { 0.0 };

        // dz/dx = -nx / nz
        let nx_z = uy * vz - uz * vy;
        let dz_dx = nx_z * inv_nz;

        // dc/dx = -nx_c / nz
        // We compute this for each component
        let nx_r = uy * vc.x - uc.x * vy;
        let nx_g = uy * vc.y - uc.y * vy;
        let nx_b = uy * vc.z - uc.z * vy;
        let dc_dx = Vec3::new(nx_r * inv_nz, nx_g * inv_nz, nx_b * inv_nz);

        Self { dz_dx, dc_dx }
    }
}

struct GouraudEdge {
    x: f32,
    z: f32,
    color: Vec3,

    dx_dy: f32,
    dz_dy: f32,
    dc_dy: Vec3,
}

impl GouraudEdge {
    fn new(p_start: ScreenPoint, c_start: Vec3, p_end: ScreenPoint, c_end: Vec3) -> Self {
        let height = (p_end.y as i64 - p_start.y as i64) as f32;
        let (dx_dy, dz_dy, dc_dy) = if height != 0.0 {
            let inv_h = 1.0 / height;
            (
                (p_end.x as i64 - p_start.x as i64) as f32 * inv_h,
                (p_end.z - p_start.z) * inv_h,
                (c_end - c_start) * inv_h,
            )
        } else {
            (0.0, 0.0, Vec3::default())
        };

        Self {
            x: p_start.x as f32,
            z: p_start.z,
            color: c_start,
            dx_dy,
            dz_dy,
            dc_dy,
        }
    }

    fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.color = self.color + self.dc_dy;
    }

    fn step_by(&mut self, steps: f32) {
        self.x += self.dx_dy * steps;
        self.z += self.dz_dy * steps;
        self.color = self.color + self.dc_dy * steps;
    }
}

struct FlatGradients {
    dz_dx: f32,
}

impl FlatGradients {
    fn new(p0: ScreenPoint, p1: ScreenPoint, p2: ScreenPoint) -> Self {
        let ux = (p1.x as i64 - p0.x as i64) as f32;
        let uy = (p1.y as i64 - p0.y as i64) as f32;
        let uz = p1.z - p0.z;

        let vx = (p2.x as i64 - p0.x as i64) as f32;
        let vy = (p2.y as i64 - p0.y as i64) as f32;
        let vz = p2.z - p0.z;

        let nx = uy * vz - uz * vy;
        let nz = ux * vy - uy * vx;

        let dz_dx = if nz.abs() > 0.0001 { -nx / nz } else { 0.0 };

        Self { dz_dx }
    }
}

struct FlatEdge {
    x: f32,
    z: f32,
    dx_dy: f32,
    dz_dy: f32,
}

impl FlatEdge {
    fn new(p_start: ScreenPoint, p_end: ScreenPoint) -> Self {
        let height = (p_end.y as i64 - p_start.y as i64) as f32;
        let (dx_dy, dz_dy) = if height != 0.0 {
            let inv_h = 1.0 / height;
            (
                (p_end.x as i64 - p_start.x as i64) as f32 * inv_h,
                (p_end.z - p_start.z) * inv_h,
            )
        } else {
            (0.0, 0.0)
        };

        Self {
            x: p_start.x as f32,
            z: p_start.z,
            dx_dy,
            dz_dy,
        }
    }

    fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
    }

    fn step_by(&mut self, steps: f32) {
        self.x += self.dx_dy * steps;
        self.z += self.dz_dy * steps;
    }
}

/// Helper to ensure buffer dimensions match
#[inline]
fn assert_same_dimensions(fb: &Framebuffer, zb: &ZBuffer) {
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

/// Helper to sort 3 vertices by Y coordinate
///
/// Optimization: Uses a manual sorting network to avoid the heap allocation
/// incurred by `slice::sort_by_key` for small arrays.
fn sort_by_y<T, F>(verts: &mut [T; 3], get_y: F)
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

/// Draw a single scanline for flat shading with Z-buffering
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_scanline_flat(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    z_start: f32,
    dz_dx: f32,
    color: u32,
) {
    let width = fb.width() as i32;
    // Clamp X range to screen bounds
    let mut xs = x_start;
    let mut xe = x_end;
    let mut z = z_start;

    if xs < 0 {
        // Advance z if we start off-screen
        z += (-xs) as f32 * dz_dx;
        xs = 0;
    }

    if xe >= width {
        xe = width - 1;
    }

    if xs > xe {
        return;
    }

    // Optimization: Use slice iterators to avoid index recalculation and bounds checks in the loop
    debug_assert_eq!(
        fb.width(),
        zb.width(),
        "Framebuffer and ZBuffer widths must match"
    );
    let width_usize = fb.width() as usize;
    let y_offset = (y as usize) * width_usize;
    let start_idx = y_offset + (xs as usize);
    let end_idx = y_offset + (xe as usize);

    // SAFETY:
    // 1. xs and xe are clamped to [0, width-1] by the logic above.
    // 2. y is clamped to [0, height-1] by the caller.
    // 3. We checked `xs <= xe` immediately above, so `start_idx <= end_idx`.
    let fb_slice = unsafe { fb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };
    let zb_slice = unsafe { zb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };

    for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
        if z < *depth_val {
            *depth_val = z;
            *pixel = color;
        }
        z += dz_dx;
    }
}

/// Fill a 3D triangle with z-buffer test
pub fn fill_triangle_3d(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: (Vec3, f32),
    v1: (Vec3, f32),
    v2: (Vec3, f32),
    color: u32,
) {
    let v0 = ClipVertex::new(v0.0, v0.1);
    let v1 = ClipVertex::new(v1.0, v1.1);
    let v2 = ClipVertex::new(v2.0, v2.1);
    fill_triangle_3d_impl(fb, zb, v0, v1, v2, color);
}

fn fill_triangle_3d_impl(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ClipVertex,
    v1: ClipVertex,
    v2: ClipVertex,
    color: u32,
) {
    assert_same_dimensions(fb, zb);

    let width = fb.width();
    let height = fb.height();

    // Project to screen
    let p0 = project_to_screen(v0.position, v0.w, width, height);
    let p1 = project_to_screen(v1.position, v1.w, width, height);
    let p2 = project_to_screen(v2.position, v2.w, width, height);

    // Sort by y
    let mut verts = [p0, p1, p2];
    sort_by_y(&mut verts, |p| p.y);
    let [p0, p1, p2] = verts;

    let total_height = (p2.y as i64 - p0.y as i64) as f32;
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

    let gradients = FlatGradients::new(p0, p1, p2);

    // Winding order
    let ux = (p1.x as i64 - p0.x as i64) as f32;
    let uy = (p1.y as i64 - p0.y as i64) as f32;
    let vx = (p2.x as i64 - p0.x as i64) as f32;
    let vy = (p2.y as i64 - p0.y as i64) as f32;
    let nz = ux * vy - uy * vx;
    let long_edge_is_left = nz > 0.0;

    // Edge A (long edge)
    let mut edge_a = FlatEdge::new(p0, p2);
    if y_start > p0.y {
        edge_a.step_by((y_start as i64 - p0.y as i64) as f32);
    }

    // Edge B (short edges)
    let mut edge_b = if y_start < p1.y {
        let mut e = FlatEdge::new(p0, p1);
        if y_start > p0.y {
            e.step_by((y_start as i64 - p0.y as i64) as f32);
        }
        e
    } else {
        let mut e = FlatEdge::new(p1, p2);
        if y_start > p1.y {
            e.step_by((y_start as i64 - p1.y as i64) as f32);
        }
        e
    };

    for y in y_start..=y_end {
        if y == p1.y && y != p0.y {
            edge_b = FlatEdge::new(p1, p2);
        }

        let (left, right) = if long_edge_is_left {
            (&edge_a, &edge_b)
        } else {
            (&edge_b, &edge_a)
        };

        let x_start = left.x as i32;
        let x_end = right.x as i32;
        let dx = x_end - x_start;

        if dx <= 0 {
            if x_start >= 0 && x_start < width as i32 && zb.test_and_set(x_start, y, left.z) {
                fb.set_pixel(x_start, y, color);
            }
        } else {
            draw_scanline_flat(fb, zb, y, x_start, x_end, left.z, gradients.dz_dx, color);
        }

        edge_a.step();
        edge_b.step();
    }
}

/// Fill a 3D triangle with flat shading
pub fn fill_triangle_flat(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: (Vec3, f32),
    v1: (Vec3, f32),
    v2: (Vec3, f32),
    normal: Vec3,
    base_color: Vec3,
) {
    // Default lighting setup
    let ambient = AmbientLight::new(Vec3::new(0.2, 0.2, 0.2));
    let sun = DirectionalLight::new(Vec3::new(-0.5, -1.0, -0.5), Vec3::new(1.0, 1.0, 1.0));

    // Calculate flat shade
    let ambient_color = ambient.shade(base_color);
    let diffuse_color = sun.shade(normal, base_color);

    let final_color = Vec3::new(
        (ambient_color.x + diffuse_color.x).min(1.0),
        (ambient_color.y + diffuse_color.y).min(1.0),
        (ambient_color.z + diffuse_color.z).min(1.0),
    );

    let color_u32 = color_to_u32(final_color);
    fill_triangle_3d(fb, zb, v0, v1, v2, color_u32);
}

/// Fill a 3D triangle with custom lighting
#[allow(clippy::too_many_arguments)] // Rendering API requires all parameters explicitly
pub fn fill_triangle_lit(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: (Vec3, f32),
    v1: (Vec3, f32),
    v2: (Vec3, f32),
    normal: Vec3,
    base_color: Vec3,
    ambient: &AmbientLight,
    light: &DirectionalLight,
) {
    let ambient_color = ambient.shade(base_color);
    let diffuse_color = light.shade(normal, base_color);

    let final_color = Vec3::new(
        (ambient_color.x + diffuse_color.x).min(1.0),
        (ambient_color.y + diffuse_color.y).min(1.0),
        (ambient_color.z + diffuse_color.z).min(1.0),
    );

    let color_u32 = color_to_u32(final_color);
    fill_triangle_3d(fb, zb, v0, v1, v2, color_u32);
}

/// Helper for fast color packing using saturating casts.
#[inline(always)]
fn pack_color_fast(c: Vec3) -> u32 {
    pack_rgb_scalar(c.x, c.y, c.z)
}

/// Helper for fast color packing from scalars.
#[inline(always)]
fn pack_rgb_scalar(r: f32, g: f32, b: f32) -> u32 {
    let r = r as u8;
    let g = g as u8;
    let b = b as u8;
    0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

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
    c_start: Vec3,
    dz_dx: f32,
    dc_dx: Vec3,
) {
    let width = fb.width() as i32;
    let mut xs = x_start;
    let mut xe = x_end;
    let mut z = z_start;
    let mut c = c_start;

    // Clamp to screen bounds
    if xs < 0 {
        let diff = -xs as f32;
        z += diff * dz_dx;
        c = c + dc_dx * diff;
        xs = 0;
    }

    if xe >= width {
        xe = width - 1;
    }

    if xs <= xe {
        // Optimization: Decompose Vec3 to scalars to avoid struct construction overhead in hot loop
        let mut r = c.x;
        let mut g = c.y;
        let mut b = c.z;
        let dr = dc_dx.x;
        let dg = dc_dx.y;
        let db = dc_dx.z;

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
                *pixel = pack_rgb_scalar(r, g, b);
            }
            z += dz_dx;
            r += dr;
            g += dg;
            b += db;
        }
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
    let v0 = GouraudVertex::from(v0);
    let v1 = GouraudVertex::from(v1);
    let v2 = GouraudVertex::from(v2);
    fill_triangle_gouraud_impl(fb, zb, v0, v1, v2);
}

fn fill_triangle_gouraud_impl(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: GouraudVertex,
    v1: GouraudVertex,
    v2: GouraudVertex,
) {
    assert_same_dimensions(fb, zb);

    let width = fb.width();
    let height = fb.height();

    // Project to screen
    let p0 = project_to_screen(v0.position, v0.w, width, height);
    let p1 = project_to_screen(v1.position, v1.w, width, height);
    let p2 = project_to_screen(v2.position, v2.w, width, height);

    // Optimization: Pre-scale colors to 0..255 for faster interpolation and packing
    // allowing us to skip clamp/mul per pixel
    let c0 = v0.color * 255.0;
    let c1 = v1.color * 255.0;
    let c2 = v2.color * 255.0;

    // Sort by y
    let mut verts = [(p0, c0), (p1, c1), (p2, c2)];
    sort_by_y(&mut verts, |(p, _)| p.y);
    let [(p0, c0), (p1, c1), (p2, c2)] = verts;

    let total_height = (p2.y as i64 - p0.y as i64) as f32;
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

    let gradients = GouraudGradients::new(p0, c0, p1, c1, p2, c2);

    // Winding order
    let ux = (p1.x as i64 - p0.x as i64) as f32;
    let uy = (p1.y as i64 - p0.y as i64) as f32;
    let vx = (p2.x as i64 - p0.x as i64) as f32;
    let vy = (p2.y as i64 - p0.y as i64) as f32;
    // Cross product Z component (signed area)
    let nz = ux * vy - uy * vx;

    // Optimization: Use the sign of the cross product (nz) to determine winding
    let long_edge_is_left = nz > 0.0;

    // Initialize walkers
    // A (long edge)
    let mut edge_a = GouraudEdge::new(p0, c0, p2, c2);
    if y_start > p0.y {
        edge_a.step_by((y_start as i64 - p0.y as i64) as f32);
    }

    // B (short edges)
    // We need to handle the split at p1.y
    let mut edge_b = if y_start < p1.y {
        let mut e = GouraudEdge::new(p0, c0, p1, c1);
        if y_start > p0.y {
            e.step_by((y_start as i64 - p0.y as i64) as f32);
        }
        e
    } else {
        let mut e = GouraudEdge::new(p1, c1, p2, c2);
        if y_start > p1.y {
            e.step_by((y_start as i64 - p1.y as i64) as f32);
        }
        e
    };

    for y in y_start..=y_end {
        // Handle slope switch at p1.y
        if y == p1.y && y != p0.y {
            edge_b = GouraudEdge::new(p1, c1, p2, c2);
        }

        // Determine left/right edges
        let (left, right) = if long_edge_is_left {
            (&edge_a, &edge_b)
        } else {
            (&edge_b, &edge_a)
        };

        let x_start = left.x as i32;
        let x_end = right.x as i32;
        let dx = x_end - x_start;

        if dx <= 0 {
            if x_start >= 0 && x_start < width as i32 && zb.test_and_set(x_start, y, left.z) {
                fb.set_pixel(x_start, y, pack_color_fast(left.color));
            }
        } else {
            // Optimization: dz_dx and dc_dx are pre-calculated outside the loop
            draw_scanline_gouraud(
                fb,
                zb,
                y,
                x_start,
                x_end,
                left.z,
                left.color,
                gradients.dz_dx,
                gradients.dc_dx,
            );
        }

        edge_a.step();
        edge_b.step();
    }
}
