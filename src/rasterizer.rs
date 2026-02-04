//! 2D Rasterization primitives.
//!
//! Software rendering functions for 2D shapes (lines, circles, triangles).

use crate::framebuffer::Framebuffer;
use crate::light::{AmbientLight, DirectionalLight, color_to_u32};
use crate::math::ScreenPoint;
use crate::shapes::{Polygon, Triangle};

pub fn plot_pixel(fb: &mut Framebuffer, x: i32, y: i32, color: u32) {
    fb.set_pixel(x, y, color);
}

/// Draw a horizontal line (optimized - uses slice fill)
pub fn draw_hline(fb: &mut Framebuffer, x0: i32, x1: i32, y: i32, color: u32) {
    if y < 0 || y >= fb.height() as i32 {
        return;
    }

    let x_start = x0.min(x1).max(0).min(fb.width() as i32 - 1);
    let x_end = x0.max(x1).max(0).min(fb.width() as i32 - 1);

    let y = y as usize;
    let width = fb.width() as usize;
    let start = y * width + x_start as usize;
    let end = y * width + x_end as usize + 1;

    let pixels = fb.as_mut_slice();
    pixels[start..end].fill(color);
}

/// Draw a vertical line (optimized - stride access)
pub fn draw_vline(fb: &mut Framebuffer, x: i32, y0: i32, y1: i32, color: u32) {
    if x < 0 || x >= fb.width() as i32 {
        return;
    }

    let y_start = y0.min(y1).max(0).min(fb.height() as i32 - 1);
    let y_end = y0.max(y1).max(0).min(fb.height() as i32 - 1);

    let x = x as usize;
    let width = fb.width() as usize;
    let pixels = fb.as_mut_slice();

    for y in y_start..=y_end {
        let idx = y as usize * width + x;
        pixels[idx] = color;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct OutCode(u8);

impl OutCode {
    const INSIDE: u8 = 0;
    const LEFT: u8 = 1;
    const RIGHT: u8 = 2;
    const BOTTOM: u8 = 4;
    const TOP: u8 = 8;

    fn compute(x: i32, y: i32, width: i32, height: i32) -> Self {
        let mut code = 0;
        if x < 0 {
            code |= Self::LEFT;
        } else if x >= width {
            code |= Self::RIGHT;
        }
        if y < 0 {
            code |= Self::BOTTOM;
        } else if y >= height {
            code |= Self::TOP;
        }
        OutCode(code)
    }

    fn is_inside(self) -> bool {
        self.0 == Self::INSIDE
    }

    fn shares_outside(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    fn contains(self, mask: u8) -> bool {
        (self.0 & mask) != 0
    }
}

/// Clip a line to the screen rectangle using Cohen-Sutherland algorithm.
/// Returns true if the line is at least partially visible.
fn clip_line(
    width: i32,
    height: i32,
    x0: &mut i32,
    y0: &mut i32,
    x1: &mut i32,
    y1: &mut i32,
) -> bool {
    let mut outcode0 = OutCode::compute(*x0, *y0, width, height);
    let mut outcode1 = OutCode::compute(*x1, *y1, width, height);
    let mut accept = false;

    loop {
        if outcode0.is_inside() && outcode1.is_inside() {
            // Both inside
            accept = true;
            break;
        } else if outcode0.shares_outside(outcode1) {
            // Both share an outside zone (trivial reject)
            break;
        } else {
            // Calculate intersection point
            let x: i32;
            let y: i32;

            // Pick at least one point outside
            let outcode_out = if !outcode0.is_inside() {
                outcode0
            } else {
                outcode1
            };

            // Using floating point for precision in intersection
            let x0_f = *x0 as f32;
            let y0_f = *y0 as f32;
            let x1_f = *x1 as f32;
            let y1_f = *y1 as f32;

            if outcode_out.contains(OutCode::TOP) {
                // Point is above clip window (y >= height)
                x = (x0_f + (x1_f - x0_f) * (height as f32 - 1.0 - y0_f) / (y1_f - y0_f)) as i32;
                y = height - 1;
            } else if outcode_out.contains(OutCode::BOTTOM) {
                // Point is below clip window (y < 0)
                x = (x0_f + (x1_f - x0_f) * (0.0 - y0_f) / (y1_f - y0_f)) as i32;
                y = 0;
            } else if outcode_out.contains(OutCode::RIGHT) {
                // Point is to the right of clip window (x >= width)
                y = (y0_f + (y1_f - y0_f) * (width as f32 - 1.0 - x0_f) / (x1_f - x0_f)) as i32;
                x = width - 1;
            } else {
                // LEFT
                // Point is to the left of clip window (x < 0)
                y = (y0_f + (y1_f - y0_f) * (0.0 - x0_f) / (x1_f - x0_f)) as i32;
                x = 0;
            }

            if outcode_out == outcode0 {
                *x0 = x;
                *y0 = y;
                outcode0 = OutCode::compute(*x0, *y0, width, height);
            } else {
                *x1 = x;
                *y1 = y;
                outcode1 = OutCode::compute(*x1, *y1, width, height);
            }
        }
    }
    accept
}

/// Draw a line using Bresenham's algorithm (internal, unchecked)
/// # Safety
/// Caller must ensure coordinates are within bounds.
unsafe fn draw_line_bresenham_unchecked(
    fb: &mut Framebuffer,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: u32,
) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut x = x0;
    let mut y = y0;

    loop {
        // SAFETY: Caller guarantees bounds.
        unsafe {
            fb.set_pixel_unchecked(x as usize, y as usize, color);
        }

        if x == x1 && y == y1 {
            break;
        }

        let e2 = 2 * err;

        if e2 >= dy {
            if x == x1 {
                break;
            }
            err += dy;
            x += sx;
        }

        if e2 <= dx {
            if y == y1 {
                break;
            }
            err += dx;
            y += sy;
        }
    }
}

/// Draw a line using the best available method
pub fn draw_line(fb: &mut Framebuffer, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
    // Dispatch to optimized versions for axis-aligned lines
    if y0 == y1 {
        draw_hline(fb, x0, x1, y0, color);
        return;
    }
    if x0 == x1 {
        draw_vline(fb, x0, y0, y1, color);
        return;
    }

    // Fall back to Bresenham for diagonal lines
    let mut x0 = x0;
    let mut y0 = y0;
    let mut x1 = x1;
    let mut y1 = y1;

    if clip_line(
        fb.width() as i32,
        fb.height() as i32,
        &mut x0,
        &mut y0,
        &mut x1,
        &mut y1,
    ) {
        unsafe {
            draw_line_bresenham_unchecked(fb, x0, y0, x1, y1, color);
        }
    }
}

/// Draw a wireframe polygon
pub fn draw_polygon(fb: &mut Framebuffer, polygon: &Polygon, color: u32) {
    let verts = polygon.vertices();
    if verts.is_empty() {
        return;
    }

    // Draw lines between consecutive vertices
    for i in 0..verts.len() {
        let v0 = verts[i];
        let v1 = verts[(i + 1) % verts.len()];

        draw_line(
            fb,
            v0.x as i32,
            v0.y as i32,
            v1.x as i32,
            v1.y as i32,
            color,
        );
    }
}

/// Draw a circle outline using midpoint algorithm
pub fn draw_circle(fb: &mut Framebuffer, cx: i32, cy: i32, radius: i32, color: u32) {
    if radius <= 0 {
        if radius == 0 {
            fb.set_pixel(cx, cy, color);
        }
        return;
    }

    let mut x = radius;
    let mut y = 0;
    let mut err = 1 - radius;

    while x >= y {
        // Draw 8 octants
        fb.set_pixel(cx + x, cy + y, color);
        fb.set_pixel(cx - x, cy + y, color);
        fb.set_pixel(cx + x, cy - y, color);
        fb.set_pixel(cx - x, cy - y, color);
        fb.set_pixel(cx + y, cy + x, color);
        fb.set_pixel(cx - y, cy + x, color);
        fb.set_pixel(cx + y, cy - x, color);
        fb.set_pixel(cx - y, cy - x, color);

        y += 1;
        if err < 0 {
            err += 2 * y + 1;
        } else {
            x -= 1;
            err += 2 * (y - x) + 1;
        }
    }
}

/// Fill a circle using midpoint algorithm with horizontal lines
pub fn fill_circle(fb: &mut Framebuffer, cx: i32, cy: i32, radius: i32, color: u32) {
    if radius <= 0 {
        if radius == 0 {
            fb.set_pixel(cx, cy, color);
        }
        return;
    }

    let mut x = radius;
    let mut y = 0;
    let mut err = 1 - radius;

    while x >= y {
        // Draw horizontal lines for each y level (fills the circle)
        draw_hline(fb, cx - x, cx + x, cy + y, color);
        draw_hline(fb, cx - x, cx + x, cy - y, color);
        draw_hline(fb, cx - y, cx + y, cy + x, color);
        draw_hline(fb, cx - y, cx + y, cy - x, color);

        y += 1;
        if err < 0 {
            err += 2 * y + 1;
        } else {
            x -= 1;
            err += 2 * (y - x) + 1;
        }
    }
}

/// Fill a triangle using scanline rasterization
pub fn fill_triangle(fb: &mut Framebuffer, tri: &Triangle, color: u32) {
    // Validate inputs
    if !tri.v0.x.is_finite()
        || !tri.v0.y.is_finite()
        || !tri.v1.x.is_finite()
        || !tri.v1.y.is_finite()
        || !tri.v2.x.is_finite()
        || !tri.v2.y.is_finite()
    {
        return;
    }

    // Sort vertices by y coordinate (v0.y <= v1.y <= v2.y)
    let mut v0 = tri.v0;
    let mut v1 = tri.v1;
    let mut v2 = tri.v2;

    if v0.y > v1.y {
        std::mem::swap(&mut v0, &mut v1);
    }
    if v0.y > v2.y {
        std::mem::swap(&mut v0, &mut v2);
    }
    if v1.y > v2.y {
        std::mem::swap(&mut v1, &mut v2);
    }

    let total_height = v2.y - v0.y;
    if total_height < 0.001 {
        return; // Degenerate triangle
    }

    // Clamp vertical range to framebuffer to prevent DoS (huge loops)
    let y_min = (v0.y as i32).max(0);
    let y_max = (v2.y as i32).min(fb.height() as i32 - 1);

    // Rasterize the triangle in two halves
    for y in y_min..=y_max {
        let y_f = y as f32;

        let second_half = y_f > v1.y || (v1.y - v0.y).abs() < 0.001;
        let segment_height = if second_half {
            v2.y - v1.y
        } else {
            v1.y - v0.y
        };

        let alpha = (y_f - v0.y) / total_height;
        let beta = if second_half {
            if segment_height.abs() < 0.001 {
                0.0
            } else {
                (y_f - v1.y) / segment_height
            }
        } else if segment_height.abs() < 0.001 {
            0.0
        } else {
            (y_f - v0.y) / segment_height
        };

        // Interpolate x coordinates along edges
        let mut x_a = v0.x + (v2.x - v0.x) * alpha;
        let mut x_b = if second_half {
            v1.x + (v2.x - v1.x) * beta
        } else {
            v0.x + (v1.x - v0.x) * beta
        };

        if x_a > x_b {
            std::mem::swap(&mut x_a, &mut x_b);
        }

        draw_hline(fb, x_a as i32, x_b as i32, y, color);
    }
}

use crate::math::{Vec3, project_to_screen};
use crate::zbuffer::ZBuffer;

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
        z += (-(xs as i64)) as f32 * dz_dx;
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
    assert_same_dimensions(fb, zb);

    let width = fb.width();
    let height = fb.height();

    // Project to screen
    let p0 = project_to_screen(v0.0, v0.1, width, height);
    let p1 = project_to_screen(v1.0, v1.1, width, height);
    let p2 = project_to_screen(v2.0, v2.1, width, height);

    // Sort by y
    let mut verts = [p0, p1, p2];
    sort_by_y(&mut verts, |p| p.y);
    let [p0, p1, p2] = verts;

    // Prevent overflow when p2.y is i32::MAX and p0.y is i32::MIN
    let total_height = (p2.y as i64 - p0.y as i64) as f32;
    if total_height == 0.0 {
        return;
    }

    // Optimization: Clamp Y range to screen bounds
    let y_min = 0;
    let y_max = height as i32 - 1;
    let y_start = p0.y.max(y_min);
    let y_end = p2.y.min(y_max);

    if y_start > y_end {
        return;
    }

    // Optimization: Pre-calculate dz/dx constant for the whole triangle
    // Plane equation: Ax + By + Cz + D = 0
    // vectors p0->p1 and p0->p2
    // Use i64 for coordinate differences to prevent overflow with extreme coordinates
    let ux = (p1.x as i64 - p0.x as i64) as f32;
    let uy = (p1.y as i64 - p0.y as i64) as f32;
    let uz = p1.z - p0.z;

    let vx = (p2.x as i64 - p0.x as i64) as f32;
    let vy = (p2.y as i64 - p0.y as i64) as f32;
    let vz = p2.z - p0.z;

    // Cross product to get normal (A, B, C)
    let nx = uy * vz - uz * vy;
    // let ny = uz * vx - ux * vz;
    let nz = ux * vy - uy * vx; // This is actually 2D cross product of XY (area)

    // dz/dx = -A/C = -nx/nz
    let dz_dx = if nz.abs() > 0.0001 { -nx / nz } else { 0.0 };

    // Determine if long edge is on the left or right
    // Optimization: Use the sign of the cross product (nz) to determine winding
    // If nz > 0, p1 is to the right of p0->p2, so long edge (p0->p2) is Left.
    let long_edge_is_left = nz > 0.0;

    let mut edge_a = EdgeWalker::new(p0, p2);
    if y_start > p0.y {
        edge_a.step_n(y_start - p0.y);
    }

    let mut edge_b = if y_start < p1.y {
        let mut e = EdgeWalker::new(p0, p1);
        if y_start > p0.y {
            e.step_n(y_start - p0.y);
        }
        e
    } else {
        let mut e = EdgeWalker::new(p1, p2);
        if y_start > p1.y {
            e.step_n(y_start - p1.y);
        }
        e
    };

    let width_i32 = width as i32;

    for y in y_start..=y_end {
        if y == p1.y && y != p0.y {
            edge_b = EdgeWalker::new(p1, p2);
        }

        let x_start;
        let x_end;
        let z_left;

        if long_edge_is_left {
            x_start = (edge_a.x >> 16) as i32;
            x_end = (edge_b.x >> 16) as i32;
            z_left = edge_a.z;
        } else {
            x_start = (edge_b.x >> 16) as i32;
            x_end = (edge_a.x >> 16) as i32;
            z_left = edge_b.z;
        }

        let dx = (x_end as i64) - (x_start as i64);

        if dx <= 0 {
            if x_start >= 0 && x_start < width_i32 && zb.test_and_set(x_start, y, z_left) {
                fb.set_pixel(x_start, y, color);
            }
        } else {
            draw_scanline_flat(fb, zb, y, x_start, x_end, z_left, dz_dx, color);
        }

        edge_a.step();
        edge_b.step();
    }
}

/// Helper for fast color packing from fixed point.
#[inline(always)]
fn pack_color_fixed(c: (i64, i64, i64)) -> u32 {
    let r = (c.0 >> 16).clamp(0, 255) as u8;
    let g = (c.1 >> 16).clamp(0, 255) as u8;
    let b = (c.2 >> 16).clamp(0, 255) as u8;
    0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

// Fixed point scale factor (16.16)
const FIXED_SCALE: f32 = 65536.0;

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
    let (dr, dg, db) = (dc_dx.0 as i64, dc_dx.1 as i64, dc_dx.2 as i64);

    // Clamp to screen bounds
    if xs < 0 {
        let diff = -(xs as i64);
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
                let r = (r_i >> 16).clamp(0, 255) as u8;
                let g = (g_i >> 16).clamp(0, 255) as u8;
                let b = (b_i >> 16).clamp(0, 255) as u8;
                *pixel = 0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
            }
            z += dz_dx;
            r_i += dr;
            g_i += dg;
            b_i += db;
        }
    }
}

struct EdgeWalker {
    x: i64,
    z: f32,
    dx_dy: i64,
    dz_dy: f32,
}

impl EdgeWalker {
    fn new(p_start: ScreenPoint, p_end: ScreenPoint) -> Self {
        let height = (p_end.y as i64 - p_start.y as i64) as f32;
        let (dx_dy, dz_dy) = if height != 0.0 {
            let inv_h = 1.0 / height;
            (
                ((p_end.x as i64 - p_start.x as i64) as f32 * inv_h * 65536.0) as i64,
                (p_end.z - p_start.z) * inv_h,
            )
        } else {
            (0, 0.0)
        };

        Self {
            x: (p_start.x as i64) << 16,
            z: p_start.z,
            dx_dy,
            dz_dy,
        }
    }

    fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
    }

    fn step_n(&mut self, n: i32) {
        let n_i64 = n as i64;
        let n_f = n as f32;
        self.x += self.dx_dy * n_i64;
        self.z += self.dz_dy * n_f;
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
        let ux = (p1.x as i64 - p0.x as i64) as f32;
        let uy = (p1.y as i64 - p0.y as i64) as f32;
        let uz = p1.z - p0.z;
        let uc = c1 - c0;

        let vx = (p2.x as i64 - p0.x as i64) as f32;
        let vy = (p2.y as i64 - p0.y as i64) as f32;
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
        let ux = (p1.x as i64 - p0.x as i64) as f32;
        let uy = (p1.y as i64 - p0.y as i64) as f32;
        let vx = (p2.x as i64 - p0.x as i64) as f32;
        let vy = (p2.y as i64 - p0.y as i64) as f32;
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
        let height = (p_end.y as i64 - p_start.y as i64) as f32;
        let (dx_dy, dz_dy, dc_dy) = if height != 0.0 {
            let inv_h = 1.0 / height;
            let dc = (c_end - c_start) * inv_h;
            (
                ((p_end.x as i64 - p_start.x as i64) as f32 * inv_h * 65536.0) as i64,
                (p_end.z - p_start.z) * inv_h,
                (
                    (dc.x * FIXED_SCALE) as i64,
                    (dc.y * FIXED_SCALE) as i64,
                    (dc.z * FIXED_SCALE) as i64,
                ),
            )
        } else {
            (0, 0.0, (0, 0, 0))
        };

        let c_fixed = (
            (c_start.x * FIXED_SCALE) as i64,
            (c_start.y * FIXED_SCALE) as i64,
            (c_start.z * FIXED_SCALE) as i64,
        );

        Self {
            x: (p_start.x as i64) << 16,
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
        let n_i64 = n as i64;
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

    let width = fb.width();
    let height = fb.height();

    // Project to screen
    let p0 = project_to_screen(v0.0.0, v0.0.1, width, height);
    let p1 = project_to_screen(v1.0.0, v1.0.1, width, height);
    let p2 = project_to_screen(v2.0.0, v2.0.1, width, height);

    // Optimization: Pre-scale colors to 0..255 for faster interpolation and packing
    // allowing us to skip clamp/mul per pixel
    let c0 = v0.1 * 255.0;
    let c1 = v1.1 * 255.0;
    let c2 = v2.1 * 255.0;

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

        let x_start;
        let x_end;
        let z_left;
        let c_left;

        if long_edge_is_left {
            x_start = (edge_a.x >> 16) as i32;
            x_end = (edge_b.x >> 16) as i32;
            z_left = edge_a.z;
            c_left = edge_a.c;
        } else {
            x_start = (edge_b.x >> 16) as i32;
            x_end = (edge_a.x >> 16) as i32;
            z_left = edge_b.z;
            c_left = edge_b.c;
        }

        let dx = (x_end as i64) - (x_start as i64);

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
