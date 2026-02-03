//! 2D Rasterization primitives.
//!
//! Software rendering functions for 2D shapes (lines, circles, triangles).

use crate::framebuffer::Framebuffer;
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

/// Draw a line using Bresenham's algorithm (internal)
fn draw_line_bresenham(fb: &mut Framebuffer, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut x = x0;
    let mut y = y0;

    loop {
        fb.set_pixel(x, y, color);

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
    draw_line_bresenham(fb, x0, y0, x1, y1, color);
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

    // Rasterize the triangle in two halves
    for y in (v0.y as i32)..=(v2.y as i32) {
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

    // Calculate gradients for the long edge (p0 -> p2)
    let inv_total_height = 1.0 / total_height;
    let dx_dy_a = (p2.x as i64 - p0.x as i64) as f32 * inv_total_height;
    let dz_dy_a = (p2.z - p0.z) * inv_total_height;

    // Determine if long edge is on the left or right
    // Optimization: Use the sign of the cross product (nz) to determine winding
    // If nz > 0, p1 is to the right of p0->p2, so long edge (p0->p2) is Left.
    let long_edge_is_left = nz > 0.0;

    // Initialize walkers
    // A (long edge)
    let mut ax = p0.x as f32;
    let mut az = p0.z;

    if y_start > p0.y {
        let dy = (y_start as i64 - p0.y as i64) as f32;
        ax += dx_dy_a * dy;
        az += dz_dy_a * dy;
    }

    // B (short edges)
    // Pre-calculate b1 slopes
    let h1 = (p1.y as i64 - p0.y as i64) as f32;
    let (dx_dy_b1, dz_dy_b1) = if h1 != 0.0 {
        let inv_h1 = 1.0 / h1;
        (
            (p1.x as i64 - p0.x as i64) as f32 * inv_h1,
            (p1.z - p0.z) * inv_h1,
        )
    } else {
        (0.0, 0.0)
    };

    // Pre-calculate b2 slopes
    let h2 = (p2.y as i64 - p1.y as i64) as f32;
    let (dx_dy_b2, dz_dy_b2) = if h2 != 0.0 {
        let inv_h2 = 1.0 / h2;
        (
            (p2.x as i64 - p1.x as i64) as f32 * inv_h2,
            (p2.z - p1.z) * inv_h2,
        )
    } else {
        (0.0, 0.0)
    };

    let mut bx;
    let mut bz;
    let mut dx_dy_b;
    let mut dz_dy_b;

    if y_start < p1.y {
        // Start on first segment
        bx = p0.x as f32;
        bz = p0.z;
        dx_dy_b = dx_dy_b1;
        dz_dy_b = dz_dy_b1;

        if y_start > p0.y {
            let dy = (y_start as i64 - p0.y as i64) as f32;
            bx += dx_dy_b * dy;
            bz += dz_dy_b * dy;
        }
    } else {
        // Start on second segment (includes flat top case where y_start == p0.y == p1.y)
        bx = p1.x as f32;
        bz = p1.z;
        dx_dy_b = dx_dy_b2;
        dz_dy_b = dz_dy_b2;

        if y_start > p1.y {
            let dy = (y_start as i64 - p1.y as i64) as f32;
            bx += dx_dy_b * dy;
            bz += dz_dy_b * dy;
        }
    }

    for y in y_start..=y_end {
        if y == p1.y && y != p0.y {
            bx = p1.x as f32;
            bz = p1.z;
            let h2 = (p2.y - p1.y) as f32;
            if h2 != 0.0 {
                let inv_h2 = 1.0 / h2;
                dx_dy_b = (p2.x - p1.x) as f32 * inv_h2;
                dz_dy_b = (p2.z - p1.z) * inv_h2;
            }
        }

        let (x_left, z_left, x_right) = if long_edge_is_left {
            (ax, az, bx)
        } else {
            (bx, bz, ax)
        };

        let x_start = x_left as i32;
        let x_end = x_right as i32;
        let dx = x_end - x_start;

        if dx <= 0 {
            if x_start >= 0 && x_start < width as i32 && zb.test_and_set(x_start, y, z_left) {
                fb.set_pixel(x_start, y, color);
            }
        } else {
            draw_scanline_flat(fb, zb, y, x_start, x_end, z_left, dz_dx, color);
        }

        ax += dx_dy_a;
        az += dz_dy_a;
        bx += dx_dy_b;
        bz += dz_dy_b;
    }
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

    // Optimization: Pre-calculate gradients (dz/dx, dc/dx) using plane equation
    // This avoids per-scanline division and subtraction.
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

    // Optimization: Use the sign of the cross product (nz) to determine winding
    let long_edge_is_left = nz > 0.0;

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

    // Calculate gradients for the long edge (p0 -> p2)
    let inv_total_height = 1.0 / total_height;
    let dx_dy_a = (p2.x - p0.x) as f32 * inv_total_height;
    let dz_dy_a = (p2.z - p0.z) * inv_total_height;
    let dc_dy_a = (c2 - c0) * inv_total_height;

    // Initialize walkers
    // A is always the long edge
    let mut ax = p0.x as f32;
    let mut az = p0.z;
    let mut ac = c0;

    // B is the split edge
    let mut bx = p0.x as f32;
    let mut bz = p0.z;
    let mut bc = c0;

    // Gradient for the first segment (p0 -> p1)
    let h1 = (p1.y - p0.y) as f32;
    let (dx_dy_b1, dz_dy_b1, dc_dy_b1) = if h1 != 0.0 {
        let inv_h1 = 1.0 / h1;
        (
            (p1.x - p0.x) as f32 * inv_h1,
            (p1.z - p0.z) * inv_h1,
            (c1 - c0) * inv_h1,
        )
    } else {
        (0.0, 0.0, Vec3::default())
    };

    // Pre-advance to y_start if needed (clipping)
    if y_start > p0.y {
        let dy = (y_start - p0.y) as f32;
        ax += dx_dy_a * dy;
        az += dz_dy_a * dy;
        ac = ac + dc_dy_a * dy;

        if y_start < p1.y {
            bx += dx_dy_b1 * dy;
            bz += dz_dy_b1 * dy;
            bc = bc + dc_dy_b1 * dy;
        } else {
            // We are starting in the second segment (or exactly at p1)
            // Initialize B at p1 and advance from there
            bx = p1.x as f32;
            bz = p1.z;
            bc = c1;

            let h2 = (p2.y - p1.y) as f32;
            if h2 != 0.0 {
                let inv_h2 = 1.0 / h2;
                let dx_dy_b2 = (p2.x - p1.x) as f32 * inv_h2;
                let dz_dy_b2 = (p2.z - p1.z) * inv_h2;
                let dc_dy_b2 = (c2 - c1) * inv_h2;

                let dy2 = (y_start - p1.y) as f32;
                bx += dx_dy_b2 * dy2;
                bz += dz_dy_b2 * dy2;
                bc = bc + dc_dy_b2 * dy2;
            }
        }
    }

    // Gradients for B (current)
    let mut dx_dy_b = dx_dy_b1;
    let mut dz_dy_b = dz_dy_b1;
    let mut dc_dy_b = dc_dy_b1;

    // If we start past p1.y, we need to set the slopes to b2 slopes
    if y_start >= p1.y {
        let h2 = (p2.y - p1.y) as f32;
        if h2 != 0.0 {
            let inv_h2 = 1.0 / h2;
            dx_dy_b = (p2.x - p1.x) as f32 * inv_h2;
            dz_dy_b = (p2.z - p1.z) * inv_h2;
            dc_dy_b = (c2 - c1) * inv_h2;
        }
    }

    for y in y_start..=y_end {
        // Handle slope switch at p1.y
        if y == p1.y && y != p0.y {
            bx = p1.x as f32;
            bz = p1.z;
            bc = c1;

            let h2 = (p2.y - p1.y) as f32;
            if h2 != 0.0 {
                let inv_h2 = 1.0 / h2;
                dx_dy_b = (p2.x - p1.x) as f32 * inv_h2;
                dz_dy_b = (p2.z - p1.z) * inv_h2;
                dc_dy_b = (c2 - c1) * inv_h2;
            }
        }

        // Determine left/right edges
        let (x_left, z_left, c_left, x_right, _z_right, _c_right) = if long_edge_is_left {
            (ax, az, ac, bx, bz, bc)
        } else {
            (bx, bz, bc, ax, az, ac)
        };

        let x_start = x_left as i32;
        let x_end = x_right as i32;
        let dx = x_end - x_start;

        if dx <= 0 {
            if x_start >= 0 && x_start < width as i32 && zb.test_and_set(x_start, y, z_left) {
                fb.set_pixel(x_start, y, pack_color_fast(c_left));
            }
            // Increment for next iteration
            ax += dx_dy_a;
            az += dz_dy_a;
            ac = ac + dc_dy_a;

            bx += dx_dy_b;
            bz += dz_dy_b;
            bc = bc + dc_dy_b;
            continue;
        }

        // Optimization: dz_dx and dc_dx are pre-calculated outside the loop
        draw_scanline_gouraud(fb, zb, y, x_start, x_end, z_left, c_left, dz_dx, dc_dx);

        // Increment for next iteration
        ax += dx_dy_a;
        az += dz_dy_a;
        ac = ac + dc_dy_a;

        bx += dx_dy_b;
        bz += dz_dy_b;
        bc = bc + dc_dy_b;
    }
}
