//! Rasterization primitives.
//!
//! Software rendering functions that operate on framebuffers.
//! All primitives perform bounds checking.

use crate::framebuffer::Framebuffer;
use crate::light::{AmbientLight, DirectionalLight, color_to_u32};
use crate::math::Vec3;
use crate::shapes::{Polygon, Triangle};
use crate::zbuffer::ZBuffer;

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
    let [v0, v1, v2] = sort_triangle_by_y(tri.v0, tri.v1, tri.v2, |v| v.y);

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
    let [(x0, y0, z0), (x1, y1, z1), (x2, y2, z2)] =
        sort_triangle_by_y((x0, y0, z0), (x1, y1, z1), (x2, y2, z2), |v| v.1 as f32);

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
        let Some(factors) = compute_scanline_factors(y, y0, y1, y2, total_height) else {
            continue;
        };
        let ScanlineFactors {
            alpha,
            beta,
            second_half,
        } = factors;

        let mut ax = x0 as f32 + (x2 - x0) as f32 * alpha;
        let mut az = z0 + (z2 - z0) * alpha;

        let (mut bx, mut bz) = if second_half {
            (x1 as f32 + (x2 - x1) as f32 * beta, z1 + (z2 - z1) * beta)
        } else {
            (x0 as f32 + (x1 - x0) as f32 * beta, z0 + (z1 - z0) * beta)
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
    let [(x0, y0, z0, c0), (x1, y1, z1, c1), (x2, y2, z2, c2)] =
        sort_triangle_by_y((x0, y0, z0, c0), (x1, y1, z1, c1), (x2, y2, z2, c2), |v| {
            v.1 as f32
        });

    let total_height = y2 - y0;
    if total_height == 0 {
        return;
    }

    for y in y0..=y2 {
        let Some(factors) = compute_scanline_factors(y, y0, y1, y2, total_height) else {
            continue;
        };
        let ScanlineFactors {
            alpha,
            beta,
            second_half,
        } = factors;

        // Interpolate position and color along edges
        let mut ax = x0 as f32 + (x2 - x0) as f32 * alpha;
        let mut az = z0 + (z2 - z0) * alpha;
        let mut ac = Vec3::new(
            c0.x + (c2.x - c0.x) * alpha,
            c0.y + (c2.y - c0.y) * alpha,
            c0.z + (c2.z - c0.z) * alpha,
        );

        let (mut bx, mut bz, mut bc) = if second_half {
            (
                x1 as f32 + (x2 - x1) as f32 * beta,
                z1 + (z2 - z1) * beta,
                Vec3::new(
                    c1.x + (c2.x - c1.x) * beta,
                    c1.y + (c2.y - c1.y) * beta,
                    c1.z + (c2.z - c1.z) * beta,
                ),
            )
        } else {
            (
                x0 as f32 + (x1 - x0) as f32 * beta,
                z0 + (z1 - z0) * beta,
                Vec3::new(
                    c0.x + (c1.x - c0.x) * beta,
                    c0.y + (c1.y - c0.y) * beta,
                    c0.z + (c1.z - c0.z) * beta,
                ),
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
            let color = Vec3::new(
                ac.x + (bc.x - ac.x) * t,
                ac.y + (bc.y - ac.y) * t,
                ac.z + (bc.z - ac.z) * t,
            );

            if zb.test_and_set(x, y, z) {
                fb.set_pixel(x, y, color_to_u32(color));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec2;

    #[test]
    fn test_sort_triangle_vertices() {
        let v0 = Vec2::new(0.0, 10.0);
        let v1 = Vec2::new(1.0, 5.0);
        let v2 = Vec2::new(2.0, 20.0);

        let [p0, p1, p2] = sort_triangle_by_y(v0, v1, v2, |v| v.y);

        assert_eq!(p0.y, 5.0, "p0 should be the one with smallest y (5.0)");
        assert_eq!(p1.y, 10.0, "p1 should be the one with middle y (10.0)");
        assert_eq!(p2.y, 20.0, "p2 should be the one with largest y (20.0)");
    }

    #[test]
    fn test_compute_scanline_factors() {
        // Triangle with height 100, split at 50
        let y0 = 0;
        let y1 = 50;
        let y2 = 100;
        let total_height = 100;

        // Test at y=25 (first half)
        let factors = compute_scanline_factors(25, y0, y1, y2, total_height).unwrap();
        assert!(!factors.second_half);
        assert!((factors.alpha - 0.25).abs() < 0.001);
        assert!((factors.beta - 0.5).abs() < 0.001); // 25 / 50 = 0.5

        // Test at y=75 (second half)
        let factors = compute_scanline_factors(75, y0, y1, y2, total_height).unwrap();
        assert!(factors.second_half);
        assert!((factors.alpha - 0.75).abs() < 0.001);
        assert!((factors.beta - 0.5).abs() < 0.001); // (75-50)/(100-50) = 25/50 = 0.5
    }
}

#[derive(Debug, PartialEq)]
struct ScanlineFactors {
    alpha: f32,
    beta: f32,
    second_half: bool,
}

fn compute_scanline_factors(
    y: i32,
    y0: i32,
    y1: i32,
    y2: i32,
    total_height: i32,
) -> Option<ScanlineFactors> {
    if total_height == 0 {
        return None;
    }

    let second_half = y > y1 || y1 == y0;
    let segment_height = if second_half { y2 - y1 } else { y1 - y0 };

    if segment_height == 0 {
        return None;
    }

    let alpha = (y - y0) as f32 / total_height as f32;
    let beta = if second_half {
        (y - y1) as f32 / segment_height as f32
    } else {
        (y - y0) as f32 / segment_height as f32
    };

    Some(ScanlineFactors {
        alpha,
        beta,
        second_half,
    })
}

/// Sort 3 vertices by Y coordinate (ascending)
fn sort_triangle_by_y<T, F>(mut v0: T, mut v1: T, mut v2: T, get_y: F) -> [T; 3]
where
    F: Fn(&T) -> f32,
{
    if get_y(&v0) > get_y(&v1) {
        std::mem::swap(&mut v0, &mut v1);
    }
    if get_y(&v0) > get_y(&v2) {
        std::mem::swap(&mut v0, &mut v2);
    }
    if get_y(&v1) > get_y(&v2) {
        std::mem::swap(&mut v1, &mut v2);
    }
    [v0, v1, v2]
}
