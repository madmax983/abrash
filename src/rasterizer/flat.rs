use crate::clipping::clip_triangle_to_frustum;
use crate::framebuffer::Framebuffer;
use crate::math::{project_to_screen_optimized, Vec3};
use crate::texture::blend_swar;
use crate::zbuffer::ZBuffer;

use super::common::{
    assert_same_dimensions, color_to_u32, is_backface, prepare_scanline, sort_by_y, EdgeWalker,
};

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
    if let Some((fb_slice, zb_slice, mut z)) =
        prepare_scanline(fb, zb, y, x_start, x_end, z_start, dz_dx)
    {
        for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
            if z < *depth_val {
                *depth_val = z;
                *pixel = color;
            }
            z += dz_dx;
        }
    }
}

/// Draw a single scanline for flat shading with Z-buffering and Alpha Blending
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_scanline_flat_blended(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    z_start: f32,
    dz_dx: f32,
    color: u32,
) {
    if let Some((fb_slice, zb_slice, mut z)) =
        prepare_scanline(fb, zb, y, x_start, x_end, z_start, dz_dx)
    {
        // Alpha blending parameters
        let alpha = (color >> 24) & 0xFF;
        let inv_alpha = 255 - alpha;

        for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
            // Test Z but do not write Z for transparent pixels
            if z < *depth_val {
                let dest_color = *pixel;
                *pixel = blend_swar(color, dest_color, alpha, inv_alpha);
            }
            z += dz_dx;
        }
    }
}

/// Fill a 3D triangle with z-buffer test.
///
/// This function takes vertices in **Homogeneous Clip Space**.
/// It handles:
/// 1.  Frustum Clipping
/// 2.  Perspective Division (converting to Normalized Device Coordinates)
/// 3.  Viewport Mapping (converting to Screen Space)
/// 4.  Rasterization & Depth Testing
///
/// # Arguments
///
/// *   `fb` - Target framebuffer.
/// *   `zb` - Target z-buffer.
/// *   `v0`, `v1`, `v2` - Vertices as `(Position, W)`. `Position` is `Vec3` (x,y,z).
/// *   `color` - 0xAARRGGBB color value.
///
/// # Examples
///
/// ```
/// use abrash::rasterizer::fill_triangle_3d;
/// use abrash::framebuffer::Framebuffer;
/// use abrash::zbuffer::ZBuffer;
/// use abrash::math::Vec3;
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// let mut zb = ZBuffer::new(100, 100).unwrap();
///
/// // Define vertices in Clip Space (x, y, z, w)
/// // Visible range: -w <= x,y,z <= w
/// let v0 = (Vec3::new(0.0, 0.5, 5.0), 5.0);
/// let v1 = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
/// let v2 = (Vec3::new(0.5, -0.5, 5.0), 5.0);
/// let color = 0xFFFF0000; // Red
///
/// fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, color);
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

    let clipped = clip_triangle_to_frustum(v0, v1, v2, |v| (v.0, v.1));

    let width = fb.width();
    let height = fb.height();
    let half_width = width as f32 * 0.5;
    let half_height = height as f32 * 0.5;

    for i in 0..clipped.count {
        let base = i * 3;
        let v0 = clipped.tris[base];
        let v1 = clipped.tris[base + 1];
        let v2 = clipped.tris[base + 2];

        // Project to screen
        let p0_orig = project_to_screen_optimized(v0.0, v0.1, half_width, half_height);
        let p1_orig = project_to_screen_optimized(v1.0, v1.1, half_width, half_height);
        let p2_orig = project_to_screen_optimized(v2.0, v2.1, half_width, half_height);

        // Backface Culling (on original unsorted vertices)
        if is_backface(p0_orig, p1_orig, p2_orig) {
            continue;
        }

        // Sort by y
        let mut verts = [p0_orig, p1_orig, p2_orig];
        sort_by_y(&mut verts, |p| p.y);
        let [p0, p1, p2] = verts;

        // Prevent overflow when p2.y is i32::MAX and p0.y is i32::MIN
        let total_height = (i64::from(p2.y) - i64::from(p0.y)) as f32;
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
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
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
            edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
        }

        let mut edge_b = if y_start < p1.y {
            let mut e = EdgeWalker::new(p0, p1);
            if y_start > p0.y {
                e.step_n(i64::from(y_start) - i64::from(p0.y));
            }
            e
        } else {
            let mut e = EdgeWalker::new(p1, p2);
            if y_start > p1.y {
                e.step_n(i64::from(y_start) - i64::from(p1.y));
            }
            e
        };

        let width_i32 = width as i32;

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b = EdgeWalker::new(p1, p2);
            }

            let (x_start, x_end, z_left) = if long_edge_is_left {
                ((edge_a.x >> 16) as i32, (edge_b.x >> 16) as i32, edge_a.z)
            } else {
                ((edge_b.x >> 16) as i32, (edge_a.x >> 16) as i32, edge_b.z)
            };

            let dx = i64::from(x_end) - i64::from(x_start);

            if dx <= 0 {
                if x_start >= 0 && x_start < width_i32 {
                    // SAFETY: Safe due to clamps on x_start and y.
                    unsafe {
                        let alpha = (color >> 24) & 0xFF;
                        if alpha == 0xFF {
                            if zb.test_and_set_unchecked(x_start as usize, y as usize, z_left) {
                                fb.set_pixel_unchecked(x_start as usize, y as usize, color);
                            }
                        } else {
                            // Transparent single pixel
                            let z_current = zb.get_depth_unchecked(x_start as usize, y as usize);
                            if z_left < z_current {
                                let dest = fb.get_pixel_unchecked(x_start as usize, y as usize);
                                let blended = blend_swar(color, dest, alpha, 255 - alpha);
                                fb.set_pixel_unchecked(x_start as usize, y as usize, blended);
                            }
                        }
                    }
                }
            } else {
                let alpha = (color >> 24) & 0xFF;
                if alpha == 0xFF {
                    draw_scanline_flat(fb, zb, y, x_start, x_end, z_left, dz_dx, color);
                } else {
                    draw_scanline_flat_blended(fb, zb, y, x_start, x_end, z_left, dz_dx, color);
                }
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
    // Ambient shade
    let ambient = base_color * ambient_color;

    // Diffuse shade: base * light * max(0, normal . -dir)
    let intensity = normal.dot(light_dir * -1.0).max(0.0);
    let diffuse = base_color * light_color * intensity;

    // Combine and clamp (clamping handled by color_to_u32)
    let final_color = ambient + diffuse;

    let color_u32 = color_to_u32(final_color);
    fill_triangle_3d(fb, zb, v0, v1, v2, color_u32);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;
    use crate::math::Vec3;
    use crate::zbuffer::ZBuffer;

    #[test]
    fn fill_triangle_3d_with_fixed_point_matches_reference() {
        // This is a regression test to ensure fixed-point conversion
        // does not change rendering output
        let width = 200;
        let height = 200;

        let v0 = (Vec3::new(0.0, 0.5, 5.0), 5.0);
        let v1 = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
        let v2 = (Vec3::new(0.5, -0.5, 5.0), 5.0);
        let color = 0xFFFF_0000;

        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, color);

        // Verify the triangle was rendered (at least some pixels changed)
        let rendered_pixels = fb.as_slice().iter().filter(|&&p| p != 0xFF00_0000).count();

        assert!(
            rendered_pixels > 100,
            "Expected at least 100 pixels rendered, got {rendered_pixels}",
        );

        // Verify center pixel is red (triangle is centered)
        let center_pixel = fb.get_pixel((width / 2) as i32, (height / 2) as i32);
        assert_eq!(
            center_pixel,
            Some(color),
            "Center pixel should be red (triangle color)"
        );
    }

    #[test]
    fn draw_scanline_flat_interpolation() {
        // Test that draw_scanline_flat correctly interpolates z
        let width = 100;
        let height = 1;

        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        // Draw a scanline with float z
        let z_start = 5.0;
        let dz_dx = 0.01; // Slight gradient
        let color = 0xFFFF_0000;

        draw_scanline_flat(&mut fb, &mut zb, 0, 0, 99, z_start, dz_dx, color);

        // Verify all pixels were drawn
        for x in 0..width {
            assert_eq!(
                fb.get_pixel(x as i32, 0),
                Some(color),
                "Pixel at x={x} should be colored",
            );
        }

        // Verify zbuffer was updated correctly
        let zb_slice = zb.as_slice();
        assert!(
            (zb_slice[0] - 5.0).abs() < 0.0001,
            "First pixel: got {}, expected 5.0",
            zb_slice[0]
        );
        assert!(
            zb_slice[50] > 5.0 && zb_slice[50] < 6.0,
            "Middle pixel: got {}, expected between 5.0 and 6.0",
            zb_slice[50]
        );
        assert!(
            zb_slice[99] > 5.0 && zb_slice[99] < 7.0,
            "Last pixel: got {}, expected between 5.0 and 7.0",
            zb_slice[99]
        );
    }
}
