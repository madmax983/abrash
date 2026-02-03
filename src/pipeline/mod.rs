//! 3D Rendering pipeline.
//!
//! This module handles the transformation and rasterization of 3D primitives (triangles).
//!
//! # The Pipeline Stages
//!
//! 1.  **Vertex Processing**: Vertices are transformed from Model Space to Clip Space.
//! 2.  **Projection**: Vertices are projected to Screen Space ([`project_to_screen`]).
//! 3.  **Rasterization**: Triangles are converted into pixels ([`fill_triangle_3d`], [`fill_triangle_gouraud`]).
//! 4.  **Fragment Processing**: Color and depth are written to the buffers.
//!
//! # Shading Models
//!
//! *   **Flat Shading**: One color per triangle. Fast but faceted look.
//! *   **Gouraud Shading**: Per-vertex color interpolation. Smoother look.

use crate::framebuffer::Framebuffer;
use crate::light::{AmbientLight, DirectionalLight, color_to_u32};
use crate::math::Vec3;
use crate::zbuffer::ZBuffer;

pub mod projection;
pub use projection::{ScreenPoint, project_to_screen};

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
    debug_assert_eq!(fb.width(), zb.width(), "Framebuffer and ZBuffer widths must match");
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

/// Fill a 3D triangle with z-buffer test (Flat Shading).
///
/// This function rasterizes a triangle using a flat color. It handles:
/// *   Screen space projection.
/// *   Y-sorting vertices.
/// *   Scanline conversion.
/// *   Depth testing against the Z-buffer.
///
/// # Examples
///
/// ```
/// use abrash::framebuffer::Framebuffer;
/// use abrash::zbuffer::ZBuffer;
/// use abrash::pipeline::fill_triangle_3d;
/// use abrash::math::Vec3;
///
/// let mut fb = Framebuffer::new(100, 100);
/// let mut zb = ZBuffer::new(100, 100);
///
/// // Define vertices (World Space) and Homogeneous W (usually 1.0 for unprojected)
/// let v0 = (Vec3::new(0.0, 50.0, 0.0), 1.0);
/// let v1 = (Vec3::new(-50.0, -50.0, 0.0), 1.0);
/// let v2 = (Vec3::new(50.0, -50.0, 0.0), 1.0);
///
/// fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, 0xFFFFFFFF);
/// ```
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
    let dz_dx = if nz.abs() > 0.0001 {
        -nx / nz
    } else {
        0.0
    };

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
        ((p1.x as i64 - p0.x as i64) as f32 * inv_h1, (p1.z - p0.z) * inv_h1)
    } else {
        (0.0, 0.0)
    };

    // Pre-calculate b2 slopes
    let h2 = (p2.y as i64 - p1.y as i64) as f32;
    let (dx_dy_b2, dz_dy_b2) = if h2 != 0.0 {
        let inv_h2 = 1.0 / h2;
        ((p2.x as i64 - p1.x as i64) as f32 * inv_h2, (p2.z - p1.z) * inv_h2)
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

/// Fill a 3D triangle with Gouraud (per-vertex) shading.
///
/// # Algorithm
///
/// Gouraud shading interpolates colors across the face of the triangle.
/// 1.  Calculates color gradients (`dc/dx`, `dc/dy`).
/// 2.  Interpolates Red, Green, and Blue channels independently along edges and scanlines.
/// 3.  Produces a smooth gradient between vertices.
///
/// # Examples
///
/// ```
/// use abrash::framebuffer::Framebuffer;
/// use abrash::zbuffer::ZBuffer;
/// use abrash::pipeline::fill_triangle_gouraud;
/// use abrash::math::Vec3;
///
/// let mut fb = Framebuffer::new(100, 100);
/// let mut zb = ZBuffer::new(100, 100);
///
/// // Vertices with Colors: ((Position, W), Color)
/// let v0 = ((Vec3::new(0.0, 50.0, 0.0), 1.0), Vec3::new(1.0, 0.0, 0.0));   // Red Top
/// let v1 = ((Vec3::new(-50.0, -50.0, 0.0), 1.0), Vec3::new(0.0, 1.0, 0.0)); // Green Left
/// let v2 = ((Vec3::new(50.0, -50.0, 0.0), 1.0), Vec3::new(0.0, 0.0, 1.0));  // Blue Right
///
/// fill_triangle_gouraud(&mut fb, &mut zb, v0, v1, v2);
/// ```
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
