//! 3D Rendering pipeline.
//!
//! Functions for projecting and rasterizing 3D primitives (triangles) with shading.

use crate::framebuffer::Framebuffer;
use crate::light::{AmbientLight, DirectionalLight, color_to_u32};
use crate::math::Vec3;
use crate::zbuffer::ZBuffer;

/// Project a 3D point to screen coordinates
fn project_to_screen(v: Vec3, w: f32, width: u32, height: u32) -> (i32, i32, f32) {
    // Perspective divide
    let inv_w = if w.abs() > 0.0001 { 1.0 / w } else { 1.0 };
    let ndc_x = v.x * inv_w;
    let ndc_y = v.y * inv_w;
    let depth = v.z * inv_w;

    // NDC to screen coordinates
    let screen_x = ((ndc_x + 1.0) * 0.5 * width as f32) as i32;
    let screen_y = ((1.0 - ndc_y) * 0.5 * height as f32) as i32; // Flip Y

    (screen_x, screen_y, depth)
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
        let segment_height = if second_half { y2 - y1 } else { y1 - y0 };

        if segment_height == 0 {
            return None;
        }

        let alpha = (y - y0) as f32 / total_height;
        let beta = if second_half {
            (y - y1) as f32 / segment_height as f32
        } else {
            (y - y0) as f32 / segment_height as f32
        };

        Some(Self {
            alpha,
            beta,
            second_half,
        })
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
    let width = fb.width();
    let height = fb.height();

    // Project to screen
    let (x0, y0, z0) = project_to_screen(v0.0, v0.1, width, height);
    let (x1, y1, z1) = project_to_screen(v1.0, v1.1, width, height);
    let (x2, y2, z2) = project_to_screen(v2.0, v2.1, width, height);

    // Sort by y
    let mut verts = [(x0, y0, z0), (x1, y1, z1), (x2, y2, z2)];
    verts.sort_by(|a, b| a.1.cmp(&b.1));
    let [(x0, y0, z0), (x1, y1, z1), (x2, y2, z2)] = verts;

    let total_height = y2 - y0;
    if total_height == 0 {
        return;
    }

    // Optimization: Clamp Y range to screen bounds
    let y_min = 0;
    let y_max = height as i32 - 1;
    let y_start = y0.max(y_min);
    let y_end = y2.min(y_max);

    for y in y_start..=y_end {
        let Some(step) = ScanlineStep::new(y, y0, y1, y2, total_height as f32) else {
            continue;
        };

        let mut ax = x0 as f32 + (x2 - x0) as f32 * step.alpha;
        let mut az = z0 + (z2 - z0) * step.alpha;

        let (mut bx, mut bz) = if step.second_half {
            (
                x1 as f32 + (x2 - x1) as f32 * step.beta,
                z1 + (z2 - z1) * step.beta,
            )
        } else {
            (
                x0 as f32 + (x1 - x0) as f32 * step.beta,
                z0 + (z1 - z0) * step.beta,
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
        let mut z = az;

        // Clamp X range to screen bounds
        let mut xs = x_start;
        let mut xe = x_end;

        if xs < 0 {
            // Advance z if we start off-screen
            z += (-xs) as f32 * dz_dx;
            xs = 0;
        }

        if xe >= width as i32 {
            xe = width as i32 - 1;
        }

        if xs > xe {
            continue;
        }

        // Optimization: Use unchecked access in hot loop since bounds are clamped
        // SAFETY: xs and xe are clamped to [0, width-1]. y is clamped to [0, height-1].
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
    let width = fb.width();
    let height = fb.height();

    // Project to screen
    let (x0, y0, z0) = project_to_screen(v0.0.0, v0.0.1, width, height);
    let (x1, y1, z1) = project_to_screen(v1.0.0, v1.0.1, width, height);
    let (x2, y2, z2) = project_to_screen(v2.0.0, v2.0.1, width, height);

    let c0 = v0.1;
    let c1 = v1.1;
    let c2 = v2.1;

    // Sort by y
    let mut verts = [(x0, y0, z0, c0), (x1, y1, z1, c1), (x2, y2, z2, c2)];
    verts.sort_by(|a, b| a.1.cmp(&b.1));
    let [(x0, y0, z0, c0), (x1, y1, z1, c1), (x2, y2, z2, c2)] = verts;

    let total_height = y2 - y0;
    if total_height == 0 {
        return;
    }

    for y in y0..=y2 {
        let Some(step) = ScanlineStep::new(y, y0, y1, y2, total_height as f32) else {
            continue;
        };

        // Interpolate position and color along edges
        let mut ax = x0 as f32 + (x2 - x0) as f32 * step.alpha;
        let mut az = z0 + (z2 - z0) * step.alpha;
        let mut ac = c0 + (c2 - c0) * step.alpha;

        let (mut bx, mut bz, mut bc) = if step.second_half {
            (
                x1 as f32 + (x2 - x1) as f32 * step.beta,
                z1 + (z2 - z1) * step.beta,
                c1 + (c2 - c1) * step.beta,
            )
        } else {
            (
                x0 as f32 + (x1 - x0) as f32 * step.beta,
                z0 + (z1 - z0) * step.beta,
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

        for x in x_start..=x_end {
            let t = if (x_end - x_start) > 0 {
                (x - x_start) as f32 / (x_end - x_start) as f32
            } else {
                0.0
            };

            let z = az + (bz - az) * t;
            let color = ac + (bc - ac) * t;

            if zb.test_and_set(x, y, z) {
                fb.set_pixel(x, y, color_to_u32(color));
            }
        }
    }
}
