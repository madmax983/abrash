use crate::framebuffer::Framebuffer;
use crate::light::{AmbientLight, DirectionalLight, color_to_u32};
use crate::math::{Vec3, project_to_screen};
use crate::rasterizer::edge::{EdgeWalker, GouraudEdgeWalker, GouraudGradients};
use crate::rasterizer::lines::draw_hline;
use crate::rasterizer::scanline::{
    assert_same_dimensions, draw_scanline_flat, draw_scanline_gouraud, pack_color_fixed,
};
use crate::shapes::Triangle;
use crate::zbuffer::ZBuffer;

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
            x_start = edge_a.x as i32;
            x_end = edge_b.x as i32;
            z_left = edge_a.z;
        } else {
            x_start = edge_b.x as i32;
            x_end = edge_a.x as i32;
            z_left = edge_b.z;
        }

        let dx = x_end - x_start;

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
            x_start = edge_a.x as i32;
            x_end = edge_b.x as i32;
            z_left = edge_a.z;
            c_left = edge_a.c;
        } else {
            x_start = edge_b.x as i32;
            x_end = edge_a.x as i32;
            z_left = edge_b.z;
            c_left = edge_b.c;
        }

        let dx = x_end - x_start;

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
