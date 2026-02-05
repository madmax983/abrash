//! 2D Rasterization primitives.
//!
//! Software rendering functions for 2D shapes (lines, circles, triangles).

use crate::clipping::clip_triangle_against_near_plane;
use crate::framebuffer::Framebuffer;
use crate::light::color_to_u32;
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

/// Helper to iterate over circle points using Midpoint Algorithm
fn for_each_circle_point(radius: i32, mut op: impl FnMut(i32, i32)) {
    let mut x = radius;
    let mut y = 0;
    let mut err = 1 - radius;

    while x >= y {
        op(x, y);

        y += 1;
        if err < 0 {
            err += 2 * y + 1;
        } else {
            x -= 1;
            err += 2 * (y - x) + 1;
        }
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

    for_each_circle_point(radius, |x, y| {
        fb.set_pixel(cx + x, cy + y, color);
        fb.set_pixel(cx - x, cy + y, color);
        fb.set_pixel(cx + x, cy - y, color);
        fb.set_pixel(cx - x, cy - y, color);
        fb.set_pixel(cx + y, cy + x, color);
        fb.set_pixel(cx - y, cy + x, color);
        fb.set_pixel(cx + y, cy - x, color);
        fb.set_pixel(cx - y, cy - x, color);
    });
}

/// Fill a circle using midpoint algorithm with horizontal lines
pub fn fill_circle(fb: &mut Framebuffer, cx: i32, cy: i32, radius: i32, color: u32) {
    if radius <= 0 {
        if radius == 0 {
            fb.set_pixel(cx, cy, color);
        }
        return;
    }

    for_each_circle_point(radius, |x, y| {
        draw_hline(fb, cx - x, cx + x, cy + y, color);
        draw_hline(fb, cx - x, cx + x, cy - y, color);
        draw_hline(fb, cx - y, cx + y, cy + x, color);
        draw_hline(fb, cx - y, cx + y, cy - x, color);
    });
}

/// Fill a triangle using scanline rasterization.
///
/// This function implements a standard triangle rasterizer:
/// 1.  **Sorting:** Vertices are sorted by Y-coordinate (`v0.y <= v1.y <= v2.y`).
/// 2.  **Splitting:** The triangle is split into two parts:
///     *   Top flat-bottom triangle (v0 to v1/v_split).
///     *   Bottom flat-top triangle (v1/v_split to v2).
/// 3.  **Scanning:** Iterates Y from top to bottom, interpolating X coordinates along the edges
///     and drawing horizontal spans (`draw_hline`).
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

use crate::math::{Vec2, Vec3, project_to_screen};
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

    let clipped = clip_triangle_against_near_plane(v0, v1, v2, |v| v.1);

    for i in 0..clipped.count {
        let base = i * 3;
        let v0 = clipped.tris[base];
        let v1 = clipped.tris[base + 1];
        let v2 = clipped.tris[base + 2];

        let width = fb.width();
        let height = fb.height();

        // Project to screen
        let p0_orig = project_to_screen(v0.0, v0.1, width, height);
        let p1_orig = project_to_screen(v1.0, v1.1, width, height);
        let p2_orig = project_to_screen(v2.0, v2.1, width, height);

        // Backface Culling (on original unsorted vertices)
        let ux_orig = (p1_orig.x as i64 - p0_orig.x as i64) as f32;
        let uy_orig = (p1_orig.y as i64 - p0_orig.y as i64) as f32;
        let vx_orig = (p2_orig.x as i64 - p0_orig.x as i64) as f32;
        let vy_orig = (p2_orig.y as i64 - p0_orig.y as i64) as f32;
        let nz_orig = ux_orig * vy_orig - uy_orig * vx_orig;

        if nz_orig >= 0.0 {
            continue;
        }

        // Sort by y
        let mut verts = [p0_orig, p1_orig, p2_orig];
        sort_by_y(&mut verts, |p| p.y);
        let [p0, p1, p2] = verts;

        // Prevent overflow when p2.y is i32::MAX and p0.y is i32::MIN
        let total_height = (p2.y as i64 - p0.y as i64) as f32;
        if total_height == 0.0 {
            continue;
        }

        // Optimization: Clamp Y range to screen bounds
        let y_min = 0;
        let y_max = height as i32 - 1;
        let y_start = p0.y.max(y_min);
        let y_end = p2.y.min(y_max);

        if y_start > y_end {
            continue;
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
        let nz = ux * vy - uy * vx; // This is actually 2D cross product of XY (area) of SORTED triangle

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
}

/// Helper to pack 8-bit color channels into u32 ARGB
#[inline(always)]
fn pack_color_channels(r: u32, g: u32, b: u32) -> u32 {
    0xFF000000 | (r << 16) | (g << 8) | b
}

/// Helper for fast color packing from fixed point.
#[inline(always)]
fn pack_color_fixed(c: (i64, i64, i64)) -> u32 {
    let r = (c.0 >> 16).clamp(0, 255) as u32;
    let g = (c.1 >> 16).clamp(0, 255) as u32;
    let b = (c.2 >> 16).clamp(0, 255) as u32;
    pack_color_channels(r, g, b)
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
                // Optimization: Avoid intermediate casts to u8 by clamping to u32 directly
                let r = (r_i >> 16).clamp(0, 255) as u32;
                let g = (g_i >> 16).clamp(0, 255) as u32;
                let b = (b_i >> 16).clamp(0, 255) as u32;
                *pixel = pack_color_channels(r, g, b);
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
                ((p_end.x as i64 - p_start.x as i64) as f32 * inv_h * FIXED_SCALE) as i64,
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
                ((p_end.x as i64 - p_start.x as i64) as f32 * inv_h * FIXED_SCALE) as i64,
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

    let clipped = clip_triangle_against_near_plane(v0, v1, v2, |v| v.0.1);

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
        let ux_orig = (p1_orig.x as i64 - p0_orig.x as i64) as f32;
        let uy_orig = (p1_orig.y as i64 - p0_orig.y as i64) as f32;
        let vx_orig = (p2_orig.x as i64 - p0_orig.x as i64) as f32;
        let vy_orig = (p2_orig.y as i64 - p0_orig.y as i64) as f32;
        let nz_orig = ux_orig * vy_orig - uy_orig * vx_orig;

        if nz_orig >= 0.0 {
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

        let total_height = (p2.y as i64 - p0.y as i64) as f32;
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
    ambient_color: Vec3,
    light_dir: Vec3, // Direction the light travels
    light_color: Vec3,
) {
    // Ambient shade: base * ambient
    let a_r = base_color.x * ambient_color.x;
    let a_g = base_color.y * ambient_color.y;
    let a_b = base_color.z * ambient_color.z;

    // Diffuse shade: base * light * max(0, normal . -dir)
    let intensity = normal.dot(light_dir * -1.0).max(0.0);
    let d_r = base_color.x * light_color.x * intensity;
    let d_g = base_color.y * light_color.y * intensity;
    let d_b = base_color.z * light_color.z * intensity;

    let final_color = Vec3::new(
        (a_r + d_r).min(1.0),
        (a_g + d_g).min(1.0),
        (a_b + d_b).min(1.0),
    );

    let color_u32 = color_to_u32(final_color);
    fill_triangle_3d(fb, zb, v0, v1, v2, color_u32);
}

/// Texture filtering mode.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FilterMode {
    /// Nearest neighbor interpolation (pixelated look).
    Nearest,
    /// Bilinear interpolation (smooth look).
    Bilinear,
}

/// A simple 2D texture map.
///
/// Stores pixel data as 32-bit ARGB values.
/// Supports both Nearest and Bilinear filtering.
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>,
    pub filter_mode: FilterMode,
}

impl Texture {
    pub fn new(width: u32, height: u32) -> Self {
        assert!(
            width > 0 && height > 0,
            "Texture dimensions must be positive"
        );
        Self {
            width,
            height,
            pixels: vec![0xFF000000; (width * height) as usize],
            filter_mode: FilterMode::Nearest,
        }
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, color: u32) {
        if x < self.width && y < self.height {
            self.pixels[(y * self.width + x) as usize] = color;
        }
    }

    /// Sample texture using interpolation mode
    /// u, v are in range [0.0, 1.0]
    #[inline]
    pub fn get_pixel(&self, u: f32, v: f32) -> u32 {
        match self.filter_mode {
            FilterMode::Nearest => {
                let x = (u * self.width as f32) as i32;
                let y = (v * self.height as f32) as i32;
                self.get_pixel_texel(x, y)
            }
            FilterMode::Bilinear => self.get_pixel_bilinear(u, v),
        }
    }

    /// Sample texture using bilinear interpolation
    pub fn get_pixel_bilinear(&self, u: f32, v: f32) -> u32 {
        let w = self.width as f32;
        let h = self.height as f32;
        self.get_pixel_bilinear_texel(u * w, v * h)
    }

    /// Sample texture using bilinear interpolation with texel coordinates.
    ///
    /// # SWAR Optimization
    ///
    /// This function uses **SIMD Within A Register (SWAR)** to perform bilinear blending
    /// of four color channels (ARGB) in parallel without using architecture-specific SIMD instructions.
    ///
    /// The Red/Blue channels and Alpha/Green channels are masked and blended separately to
    /// prevent overflow from one channel affecting its neighbor during the integer multiplication.
    #[inline]
    pub fn get_pixel_bilinear_texel(&self, u_tex: f32, v_tex: f32) -> u32 {
        // Convert to 24.8 fixed point
        // 0.5 in 24.8 is 128
        let u_fixed = (u_tex * 256.0) as i32;
        let v_fixed = (v_tex * 256.0) as i32;

        let u_img_fixed = u_fixed - 128;
        let v_img_fixed = v_fixed - 128;

        // Weights (0..256)
        let wx = (u_img_fixed & 0xFF) as u32;
        let wy = (v_img_fixed & 0xFF) as u32;
        let inv_wx = 256 - wx;
        let inv_wy = 256 - wy;

        // Coordinates
        let w_i32 = self.width as i32 - 1;
        let h_i32 = self.height as i32 - 1;

        // Arithmetic shift preserves sign (floor behavior for negative numbers)
        let x0_raw = u_img_fixed >> 8;
        let y0_raw = v_img_fixed >> 8;

        let x0 = x0_raw.clamp(0, w_i32) as usize;
        let y0 = y0_raw.clamp(0, h_i32) as usize;
        let x1 = (x0_raw + 1).clamp(0, w_i32) as usize;
        let y1 = (y0_raw + 1).clamp(0, h_i32) as usize;

        let width_usize = self.width as usize;
        let row0 = y0 * width_usize;
        let row1 = y1 * width_usize;

        // SAFETY: We clamped coordinates to valid ranges [0, width-1] / [0, height-1]
        let (c00, c10, c01, c11) = unsafe {
            (
                *self.pixels.get_unchecked(row0 + x0),
                *self.pixels.get_unchecked(row0 + x1),
                *self.pixels.get_unchecked(row1 + x0),
                *self.pixels.get_unchecked(row1 + x1),
            )
        };

        // Function to blend two colors with weight w using SWAR (SIMD Within A Register)
        // Blends R/B and A/G in parallel
        let blend = |c0: u32, c1: u32, w: u32, inv_w: u32| -> u32 {
            let rb0 = c0 & 0x00FF00FF;
            let ag0 = (c0 >> 8) & 0x00FF00FF;
            let rb1 = c1 & 0x00FF00FF;
            let ag1 = (c1 >> 8) & 0x00FF00FF;

            let rb = ((rb0 * inv_w + rb1 * w) >> 8) & 0x00FF00FF;
            let ag = ((ag0 * inv_w + ag1 * w) >> 8) & 0x00FF00FF;

            rb | (ag << 8)
        };

        let top = blend(c00, c10, wx, inv_wx);
        let bottom = blend(c01, c11, wx, inv_wx);
        let final_color = blend(top, bottom, wy, inv_wy);

        // Ensure alpha is 0xFF
        final_color | 0xFF000000
    }

    /// Sample texture using texel coordinates
    #[inline]
    pub fn get_pixel_texel(&self, x: i32, y: i32) -> u32 {
        let x = x.clamp(0, self.width as i32 - 1) as usize;
        let y = y.clamp(0, self.height as i32 - 1) as usize;
        unsafe { *self.pixels.get_unchecked(y * self.width as usize + x) }
    }

    /// Create a checkerboard texture
    pub fn checkered(width: u32, height: u32, c1: u32, c2: u32) -> Self {
        let mut tex = Self::new(width, height);
        // Scale checks based on size, defaulting to 8x8 blocks
        let block_w = (width / 8).max(1);
        let block_h = (height / 8).max(1);

        for y in 0..height {
            for x in 0..width {
                let check = ((x / block_w) + (y / block_h)) & 1 == 0;
                tex.set_pixel(x, y, if check { c1 } else { c2 });
            }
        }
        tex
    }
}

struct PerspectiveTextureGradients {
    dz_dx: f32,
    dq_dx: f32,
    du_dx: f32,
    dv_dx: f32,
}

impl PerspectiveTextureGradients {
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
    ) -> Self {
        let ux = (p1.x as i64 - p0.x as i64) as f32;
        let uy = (p1.y as i64 - p0.y as i64) as f32;
        let uz = p1.z - p0.z;
        let uq = q1 - q0;
        let uu = u1 - u0;
        let uv = v1 - v0;

        let vx = (p2.x as i64 - p0.x as i64) as f32;
        let vy = (p2.y as i64 - p0.y as i64) as f32;
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

        Self {
            dz_dx,
            dq_dx,
            du_dx,
            dv_dx,
        }
    }
}

struct PerspectiveTextureEdgeWalker {
    x: i64,
    z: f32,
    q: f32, // 1/w
    u: f32, // u/w
    v: f32, // v/w
    dx_dy: i64,
    dz_dy: f32,
    dq_dy: f32,
    du_dy: f32,
    dv_dy: f32,
}

impl PerspectiveTextureEdgeWalker {
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
    ) -> Self {
        let height = (p_end.y as i64 - p_start.y as i64) as f32;
        let inv_h = if height != 0.0 { 1.0 / height } else { 0.0 };

        let dx_dy = ((p_end.x as i64 - p_start.x as i64) as f32 * inv_h * FIXED_SCALE) as i64;
        let dz_dy = (p_end.z - p_start.z) * inv_h;
        let dq_dy = (q_end - q_start) * inv_h;
        let du_dy = (u_end - u_start) * inv_h;
        let dv_dy = (v_end - v_start) * inv_h;

        Self {
            x: (p_start.x as i64) << 16,
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

    fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.q += self.dq_dy;
        self.u += self.du_dy;
        self.v += self.dv_dy;
    }

    fn step_n(&mut self, n: i32) {
        let n_i64 = n as i64;
        let n_f = n as f32;
        self.x += self.dx_dy * n_i64;
        self.z += self.dz_dy * n_f;
        self.q += self.dq_dy * n_f;
        self.u += self.du_dy * n_f;
        self.v += self.dv_dy * n_f;
    }
}

struct PerspectiveSpanStart {
    z: f32,
    q: f32,
    u: f32,
    v: f32,
}

/// Draw a single scanline with perspective-correct texture mapping.
///
/// # Perspective Correction
///
/// Linearly interpolating `u` and `v` in screen space causes texture distortion ("wobble").
/// Correct mapping requires interpolating `1/w`, `u/w`, and `v/w`, then performing a perspective divide
/// at every pixel: `u = (u/w) / (1/w)`.
///
/// # Optimization: Span-Based Interpolation
///
/// To avoid the costly division at every pixel, this function subdivides the scanline into
/// small spans (16 pixels). It calculates exact perspective-correct coordinates at the start and end
/// of each span, and then linearly interpolates between them (which is a close enough approximation
/// over short distances).
#[inline(always)]
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
        let diff = -(xs as i64);
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
    let w_start = if q.abs() > 0.000001 { 1.0 / q } else { 1.0 };
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
        let w_end = if q_end.abs() > 0.000001 {
            1.0 / q_end
        } else {
            1.0
        };
        let u_tex_end = u_end * w_end;
        let v_tex_end = v_end * w_end;

        // Interpolate texel coordinates linearly over the span
        let du_tex_step = (u_tex_end - u_tex_start) / count as f32;
        let dv_tex_step = (v_tex_end - v_tex_start) / count as f32;

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
                let mut u_tex = u_tex_start;
                let mut v_tex = v_tex_start;
                for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
                    if z < *depth_val {
                        *depth_val = z;
                        *pixel = texture.get_pixel_bilinear_texel(u_tex, v_tex);
                    }
                    z += gradients.dz_dx;
                    u_tex += du_tex_step;
                    v_tex += dv_tex_step;
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

    let clipped = clip_triangle_against_near_plane(v0, v1, v2, |v| v.0.1);

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
        let ux_orig = (p1_orig.x as i64 - p0_orig.x as i64) as f32;
        let uy_orig = (p1_orig.y as i64 - p0_orig.y as i64) as f32;
        let vx_orig = (p2_orig.x as i64 - p0_orig.x as i64) as f32;
        let vy_orig = (p2_orig.y as i64 - p0_orig.y as i64) as f32;
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

        let total_height = (p2.y as i64 - p0.y as i64) as f32;
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

            let ux = (p1.x as i64 - p0.x as i64) as f32;
            let uy = (p1.y as i64 - p0.y as i64) as f32;
            let vx = (p2.x as i64 - p0.x as i64) as f32;
            let vy = (p2.y as i64 - p0.y as i64) as f32;
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

            let x_start;
            let x_end;
            let z_left;
            let q_left;
            let u_left;
            let v_left;

            if long_edge_is_left {
                x_start = (edge_a.x >> 16) as i32;
                x_end = (edge_b.x >> 16) as i32;
                z_left = edge_a.z;
                q_left = edge_a.q;
                u_left = edge_a.u;
                v_left = edge_a.v;
            } else {
                x_start = (edge_b.x >> 16) as i32;
                x_end = (edge_a.x >> 16) as i32;
                z_left = edge_b.z;
                q_left = edge_b.q;
                u_left = edge_b.u;
                v_left = edge_b.v;
            }

            let dx = (x_end as i64) - (x_start as i64);

            if dx <= 0 {
                if x_start >= 0
                    && x_start < width_i32
                    && zb.test_and_set(x_start, y, z_left)
                    && q_left.abs() > 0.000001
                {
                    let w = 1.0 / q_left;
                    let u_tex = u_left * w;
                    let v_tex = v_left * w;
                    let color = match texture.filter_mode {
                        FilterMode::Nearest => texture.get_pixel_texel(u_tex as i32, v_tex as i32),
                        FilterMode::Bilinear => texture.get_pixel_bilinear_texel(u_tex, v_tex),
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
