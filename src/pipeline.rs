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
fn sort_by_y<T, F>(verts: &mut [T; 3], get_y: F)
where
    F: Fn(&T) -> i32,
{
    verts.sort_by_key(|a| get_y(a));
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
        z += (-xs) as f32 * dz_dx;
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

        let mut ax = p0.x as f32 + (p2.x - p0.x) as f32 * step.alpha;
        let mut az = p0.z + (p2.z - p0.z) * step.alpha;

        let (mut bx, mut bz) = if step.second_half {
            (
                p1.x as f32 + (p2.x - p1.x) as f32 * step.beta,
                p1.z + (p2.z - p1.z) * step.beta,
            )
        } else {
            (
                p0.x as f32 + (p1.x - p0.x) as f32 * step.beta,
                p0.z + (p1.z - p0.z) * step.beta,
            )
        };

        if ax > bx {
            std::mem::swap(&mut ax, &mut bx);
            std::mem::swap(&mut az, &mut bz);
        }

        let x_start = ax as i32;
        let x_end = bx as i32;

        let dx = x_end - x_start;
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

    let c0 = v0.1;
    let c1 = v1.1;
    let c2 = v2.1;

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

    for y in y_start..=y_end {
        let Some(step) = ScanlineStep::new(y, p0.y, p1.y, p2.y, total_height) else {
            continue;
        };

        // Interpolate position and color along edges
        let mut ax = p0.x as f32 + (p2.x - p0.x) as f32 * step.alpha;
        let mut az = p0.z + (p2.z - p0.z) * step.alpha;
        let mut ac = c0 + (c2 - c0) * step.alpha;

        let (mut bx, mut bz, mut bc) = if step.second_half {
            (
                p1.x as f32 + (p2.x - p1.x) as f32 * step.beta,
                p1.z + (p2.z - p1.z) * step.beta,
                c1 + (c2 - c1) * step.beta,
            )
        } else {
            (
                p0.x as f32 + (p1.x - p0.x) as f32 * step.beta,
                p0.z + (p1.z - p0.z) * step.beta,
                c0 + (c1 - c0) * step.beta,
            )
        };

        if ax > bx {
            std::mem::swap(&mut ax, &mut bx);
            std::mem::swap(&mut az, &mut bz);
            std::mem::swap(&mut ac, &mut bc);
        }

        let x_start = ax as i32;
        let x_end = bx as i32;

        let dx = x_end - x_start;
        if dx <= 0 {
            if x_start >= 0 && x_start < width as i32 && zb.test_and_set(x_start, y, az) {
                fb.set_pixel(x_start, y, color_to_u32(ac));
            }
            continue;
        }

        let dz = bz - az;
        let dc = bc - ac;

        let inv_dx = 1.0 / dx as f32;
        let dz_dx = dz * inv_dx;
        let dc_dx = dc * inv_dx;

        let mut xs = x_start;
        let mut xe = x_end;
        let mut z = az;
        let mut c = ac;

        // Clamp to screen bounds
        if xs < 0 {
            let diff = -xs as f32;
            z += diff * dz_dx;
            c = c + dc_dx * diff;
            xs = 0;
        }

        if xe >= width as i32 {
            xe = width as i32 - 1;
        }

        if xs > xe {
            continue;
        }

        // Use safe access (bounds checked, but loop invariant optimization still applies)
        for x in xs..=xe {
            if zb.test_and_set(x, y, z) {
                fb.set_pixel(x, y, color_to_u32(c));
            }
            z += dz_dx;
            c = c + dc_dx;
        }
    }
}
