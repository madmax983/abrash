//! 3D Rendering pipeline.
//!
//! Functions for projecting and rasterizing 3D primitives (triangles) with shading.

use crate::framebuffer::Framebuffer;
use crate::light::{AmbientLight, DirectionalLight, color_to_u32};
use crate::math::Vec3;
use crate::zbuffer::ZBuffer;

#[derive(Debug, Clone, Copy, PartialEq)]
struct ScreenPoint {
    x: i32,
    y: i32,
    z: f32,
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

/// Project a 3D point to screen coordinates
fn project_to_screen(v: Vec3, w: f32, width: u32, height: u32) -> ScreenPoint {
    // Perspective divide
    let inv_w = if w.abs() > 0.0001 { 1.0 / w } else { 1.0 };
    let ndc_x = v.x * inv_w;
    let ndc_y = v.y * inv_w;
    let depth = v.z * inv_w;

    // NDC to screen coordinates
    let screen_x = ((ndc_x + 1.0) * 0.5 * width as f32) as i32;
    let screen_y = ((1.0 - ndc_y) * 0.5 * height as f32) as i32; // Flip Y

    ScreenPoint {
        x: screen_x,
        y: screen_y,
        z: depth,
    }
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

struct ScanlineStep {
    alpha: f32,
    beta: f32,
    second_half: bool,
}

impl ScanlineStep {
    #[inline(always)]
    fn new(y: i32, y0: i32, y1: i32, y2: i32, total_height: f32) -> Option<Self> {
        let second_half = y > y1 || y1 == y0;
        // Use i64 to prevent overflow during subtraction
        let segment_height = if second_half {
            (y2 as i64) - (y1 as i64)
        } else {
            (y1 as i64) - (y0 as i64)
        };

        if segment_height == 0 {
            return None;
        }

        let alpha = ((y as i64) - (y0 as i64)) as f32 / total_height;
        let beta = if second_half {
            ((y as i64) - (y1 as i64)) as f32 / segment_height as f32
        } else {
            ((y as i64) - (y0 as i64)) as f32 / segment_height as f32
        };

        Some(Self {
            alpha,
            beta,
            second_half,
        })
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
        z += -(xs as f32) * dz_dx;
        xs = 0;
    }

    if xe >= width {
        xe = width - 1;
    }

    if xs > xe {
        return;
    }

    // Optimization: Use unchecked access in hot loop since bounds are clamped
    // SAFETY: xs and xe are clamped to [0, width-1]. y must be valid (caller responsibility).
    unsafe {
        let y_idx = y as usize;
        for xi in xs..=xe {
            if zb.test_and_set_unchecked(xi as usize, y_idx, z) {
                fb.set_pixel_unchecked(xi as usize, y_idx, color);
            }
            z += dz_dx;
        }
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

    for y in y_start..=y_end {
        let Some(step) = ScanlineStep::new(y, p0.y, p1.y, p2.y, total_height) else {
            continue;
        };

        let mut ax = p0.x as f32 + (p2.x as i64 - p0.x as i64) as f32 * step.alpha;
        let mut az = p0.z + (p2.z - p0.z) * step.alpha;

        let (mut bx, mut bz) = if step.second_half {
            (
                p1.x as f32 + (p2.x as i64 - p1.x as i64) as f32 * step.beta,
                p1.z + (p2.z - p1.z) * step.beta,
            )
        } else {
            (
                p0.x as f32 + (p1.x as i64 - p0.x as i64) as f32 * step.beta,
                p0.z + (p1.z - p0.z) * step.beta,
            )
        };

        if ax > bx {
            std::mem::swap(&mut ax, &mut bx);
            std::mem::swap(&mut az, &mut bz);
        }

        let x_start = ax as i32;
        let x_end = bx as i32;

        let dx = x_end as i64 - x_start as i64;
        let dz = bz - az;

        // Handle single pixel or invalid width
        if dx <= 0 {
            if x_start >= 0 && x_start < width as i32 && zb.test_and_set(x_start, y, az) {
                fb.set_pixel(x_start, y, color);
            }
            continue;
        }

        // Optimization: Pre-calculate Z increment per pixel
        let dz_dx = dz / dx as f32;

        draw_scanline_flat(fb, zb, y, x_start, x_end, az, dz_dx, color);
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

    // Calculate gradients for the long edge (p0 -> p2)
    let inv_total_height = 1.0 / total_height;
    let dx_dy_a = (p2.x as i64 - p0.x as i64) as f32 * inv_total_height;
    let dz_dy_a = (p2.z - p0.z) * inv_total_height;
    let dc_dy_a = (c2 - c0) * inv_total_height;

    // Determine if long edge is on the left or right
    // We check the X coordinate of the long edge at y = p1.y
    // x_long = p0.x + (p2.x - p0.x) * (p1.y - p0.y) / (p2.y - p0.y)
    let dy_total = p2.y - p0.y;
    let x_long_at_p1 = if dy_total != 0 {
        p0.x as f32 + (p2.x as i64 - p0.x as i64) as f32 * ((p1.y - p0.y) as f32 / dy_total as f32)
    } else {
        p0.x as f32
    };
    let long_edge_is_left = x_long_at_p1 < p1.x as f32;

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
            (p1.x as i64 - p0.x as i64) as f32 * inv_h1,
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
                let dx_dy_b2 = (p2.x as i64 - p1.x as i64) as f32 * inv_h2;
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
            dx_dy_b = (p2.x as i64 - p1.x as i64) as f32 * inv_h2;
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
                dx_dy_b = (p2.x as i64 - p1.x as i64) as f32 * inv_h2;
                dz_dy_b = (p2.z - p1.z) * inv_h2;
                dc_dy_b = (c2 - c1) * inv_h2;
            }
        }

        // Determine left/right edges
        let (x_left, z_left, c_left, x_right, z_right, c_right) = if long_edge_is_left {
            (ax, az, ac, bx, bz, bc)
        } else {
            (bx, bz, bc, ax, az, ac)
        };

        let x_start = x_left as i32;
        let x_end = x_right as i32;
        let dx = x_end as i64 - x_start as i64;

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

        let dz = z_right - z_left;
        let dc = c_right - c_left;
        let inv_dx = 1.0 / dx as f32;
        let dz_dx = dz * inv_dx;
        let dc_dx = dc * inv_dx;

        let mut xs = x_start;
        let mut xe = x_end;
        let mut z = z_left;
        let mut c = c_left;

        // Clamp to screen bounds
        if xs < 0 {
            let diff = -(xs as f32);
            z += diff * dz_dx;
            c = c + dc_dx * diff;
            xs = 0;
        }

        if xe >= width as i32 {
            xe = width as i32 - 1;
        }

        if xs <= xe {
            // Optimization: Decompose Vec3 to scalars to avoid struct construction overhead in hot loop
            let mut r = c.x;
            let mut g = c.y;
            let mut b = c.z;
            let dr = dc_dx.x;
            let dg = dc_dx.y;
            let db = dc_dx.z;

            // Optimization: Use unchecked access in hot loop since bounds are clamped
            // SAFETY: xs and xe are clamped to [0, width-1]. y is clamped to [0, height-1].
            unsafe {
                let y_idx = y as usize;
                for x in xs..=xe {
                    if zb.test_and_set_unchecked(x as usize, y_idx, z) {
                        fb.set_pixel_unchecked(x as usize, y_idx, pack_rgb_scalar(r, g, b));
                    }
                    z += dz_dx;
                    r += dr;
                    g += dg;
                    b += db;
                }
            }
        }

        // Increment for next iteration
        ax += dx_dy_a;
        az += dz_dy_a;
        ac = ac + dc_dy_a;

        bx += dx_dy_b;
        bz += dz_dy_b;
        bc = bc + dc_dy_b;
    }
}
