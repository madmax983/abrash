use crate::framebuffer::Framebuffer;
use crate::geometry::clipping::clip_triangle_to_frustum;
use crate::math::{Vec3, project_triangle_to_screen};
use crate::texture::blend_swar;
use crate::zbuffer::ZBuffer;

use super::core::{EdgeWalker, assert_same_dimensions, is_backface, prepare_scanline, sort_by_y};

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2")]
unsafe fn draw_scanline_flat_simd(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    z_start: f32,
    dz_dx: f32,
    color: u32,
) {
    use std::arch::x86_64::*;

    let len = fb_slice.len();
    let mut i = 0;

    let dz_dx_vec = _mm256_set1_ps(dz_dx);
    let color_vec = _mm256_set1_epi32(color as i32);

    // Initial offsets for 8 pixels
    let offsets = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);
    let mut z_vec = _mm256_add_ps(_mm256_set1_ps(z_start), _mm256_mul_ps(dz_dx_vec, offsets));
    let dz_step = _mm256_mul_ps(dz_dx_vec, _mm256_set1_ps(8.0));

    while i + 8 <= len {
        // SAFETY: i + 8 <= len ensures bounds.
        unsafe {
            let depth_ptr = zb_slice.as_mut_ptr().add(i);
            let depth_val = _mm256_loadu_ps(depth_ptr);

            let mask = _mm256_cmp_ps(z_vec, depth_val, _CMP_LT_OQ);
            let mask_int = _mm256_castps_si256(mask);

            // If any pixel passes Z-test
            if _mm256_movemask_ps(mask) != 0 {
                // Update Z-buffer
                let old_z = _mm256_loadu_ps(depth_ptr);
                let new_z = _mm256_blendv_ps(old_z, z_vec, mask);
                _mm256_storeu_ps(depth_ptr, new_z);

                // Update Framebuffer
                let fb_ptr = fb_slice.as_mut_ptr().add(i) as *mut __m256i;
                let old_color = _mm256_loadu_si256(fb_ptr);
                let new_color = _mm256_blendv_epi8(old_color, color_vec, mask_int);
                _mm256_storeu_si256(fb_ptr, new_color);
            }
        }

        z_vec = _mm256_add_ps(z_vec, dz_step);
        i += 8;
    }

    // Scalar tail
    let mut z = z_start + (i as f32) * dz_dx;
    while i < len {
        // SAFETY: Bounds checked by slice length
        unsafe {
            let depth_val = zb_slice.get_unchecked_mut(i);
            if z < *depth_val {
                *depth_val = z;
                *fb_slice.get_unchecked_mut(i) = color;
            }
        }
        z += dz_dx;
        i += 1;
    }
}

/// Draw a single scanline for flat shading with Z-buffering
#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub fn draw_scanline_flat(
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
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        if is_x86_feature_detected!("avx2") {
            unsafe {
                draw_scanline_flat_simd(fb_slice, zb_slice, z, dz_dx, color);
            }
            return;
        }

        for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
            if z < *depth_val {
                *depth_val = z;
                *pixel = color;
            }
            z += dz_dx;
        }
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2")]
unsafe fn draw_scanline_flat_blended_simd(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    z_start: f32,
    dz_dx: f32,
    color: u32,
) {
    use std::arch::x86_64::*;

    let len = fb_slice.len();
    let mut i = 0;

    // Alpha blend constants
    let alpha = (color >> 24) & 0xFF;
    let inv_alpha = 255 - alpha;

    // SIMD constants
    let dz_dx_vec = _mm256_set1_ps(dz_dx);
    // Pre-scaled source color components
    let rb_src = color & 0x00FF_00FF;
    let ag_src = (color >> 8) & 0x00FF_00FF;
    let rb_src_scaled = rb_src * alpha;
    let ag_src_scaled = ag_src * alpha;

    // Vectorize constants
    let rb_src_vec = _mm256_set1_epi32(rb_src_scaled as i32); // 32-bit broadcast
    let ag_src_vec = _mm256_set1_epi32(ag_src_scaled as i32);
    let inv_alpha_vec = _mm256_set1_epi16(inv_alpha as i16);
    let mask_rb_ag = _mm256_set1_epi32(0x00FF_00FF);

    // Initial offsets for 8 pixels
    let offsets = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);
    let mut z_vec = _mm256_add_ps(_mm256_set1_ps(z_start), _mm256_mul_ps(dz_dx_vec, offsets));
    let dz_step = _mm256_mul_ps(dz_dx_vec, _mm256_set1_ps(8.0));

    while i + 8 <= len {
        // SAFETY: i + 8 <= len
        unsafe {
            let depth_ptr = zb_slice.as_mut_ptr().add(i);
            let depth_val = _mm256_loadu_ps(depth_ptr);

            let mask = _mm256_cmp_ps(z_vec, depth_val, _CMP_LT_OQ);
            let mask_int = _mm256_castps_si256(mask);

            if _mm256_movemask_ps(mask) != 0 {
                // Load Framebuffer
                let fb_ptr = fb_slice.as_mut_ptr().add(i) as *mut __m256i;
                let dest_pixels = _mm256_loadu_si256(fb_ptr);

                // Separate Dest channels
                // 00RR00BB
                let dest_rb = _mm256_and_si256(dest_pixels, mask_rb_ag);
                // 00AA00GG
                let dest_ag = _mm256_and_si256(_mm256_srli_epi32(dest_pixels, 8), mask_rb_ag);

                // Scale Dest (dest * inv_alpha)
                // mullo_epi16 treats inputs as 16x 16-bit integers.
                // Our layout is 00RR 00BB. Both 16-bit parts are <= 255.
                // Multiplying by inv_alpha (<256) yields <65536, fitting in u16.
                let dest_rb_scaled = _mm256_mullo_epi16(dest_rb, inv_alpha_vec);
                let dest_ag_scaled = _mm256_mullo_epi16(dest_ag, inv_alpha_vec);

                // Add scaled Source
                let rb_sum = _mm256_add_epi32(rb_src_vec, dest_rb_scaled);
                let ag_sum = _mm256_add_epi32(ag_src_vec, dest_ag_scaled);

                // Shift right by 8 (divide by 256)
                let rb_res = _mm256_srli_epi32(rb_sum, 8);
                let ag_res = _mm256_srli_epi32(ag_sum, 8);

                // Mask and Combine
                let rb_final = _mm256_and_si256(rb_res, mask_rb_ag);
                let ag_final = _mm256_and_si256(ag_res, mask_rb_ag);

                // Result = rb | (ag << 8)
                let blended = _mm256_or_si256(rb_final, _mm256_slli_epi32(ag_final, 8));

                // Store (masked)
                let new_color = _mm256_blendv_epi8(dest_pixels, blended, mask_int);
                _mm256_storeu_si256(fb_ptr, new_color);
            }
        }

        z_vec = _mm256_add_ps(z_vec, dz_step);
        i += 8;
    }

    // Scalar tail
    let mut z = z_start + (i as f32) * dz_dx;
    let inv_alpha_u32 = inv_alpha;
    let rb_src_scaled_u32 = rb_src_scaled;
    let ag_src_scaled_u32 = ag_src_scaled;

    while i < len {
        // SAFETY: Bounds checked
        unsafe {
            let depth_val = zb_slice.get_unchecked_mut(i);
            if z < *depth_val {
                let pixel = fb_slice.get_unchecked_mut(i);
                let dest = *pixel;

                let rb_dest = dest & 0x00FF_00FF;
                let ag_dest = (dest >> 8) & 0x00FF_00FF;

                let rb = ((rb_src_scaled_u32 + rb_dest * inv_alpha_u32) >> 8) & 0x00FF_00FF;
                let ag = ((ag_src_scaled_u32 + ag_dest * inv_alpha_u32) >> 8) & 0x00FF_00FF;

                *pixel = rb | (ag << 8);
            }
        }
        z += dz_dx;
        i += 1;
    }
}

/// Draw a single scanline for flat shading with Z-buffering and Alpha Blending
#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub fn draw_scanline_flat_blended(
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
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        if is_x86_feature_detected!("avx2") {
            unsafe {
                draw_scanline_flat_blended_simd(fb_slice, zb_slice, z, dz_dx, color);
            }
            return;
        }

        // Alpha blending parameters
        // Correct weights for standard alpha blending (255=Opaque, 0=Transparent)
        // w (dest weight) = 255 - alpha
        // inv_w (src weight) = alpha
        let alpha = (color >> 24) & 0xFF;
        let inv_alpha = 255 - alpha; // This is dest weight

        // Optimization: Hoist source color scaling out of the loop
        // Since color is constant, we can pre-calculate (src * alpha).
        let rb_src = color & 0x00FF_00FF;
        let ag_src = (color >> 8) & 0x00FF_00FF;

        let rb_src_scaled = rb_src * alpha;
        let ag_src_scaled = ag_src * alpha;

        for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
            // Test Z but do not write Z for transparent pixels
            if z < *depth_val {
                let dest = *pixel;

                let rb_dest = dest & 0x00FF_00FF;
                let ag_dest = (dest >> 8) & 0x00FF_00FF;

                // (src * alpha + dest * (255 - alpha)) >> 8
                let rb = ((rb_src_scaled + rb_dest * inv_alpha) >> 8) & 0x00FF_00FF;
                let ag = ((ag_src_scaled + ag_dest * inv_alpha) >> 8) & 0x00FF_00FF;

                *pixel = rb | (ag << 8);
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
/// // Vertices are passed as (Position, w) tuples in Clip Space.
/// // w usually comes from the projection matrix (typically z_view).
/// // Here we simulate a triangle at z=5.0 with w=5.0 (so z_ndc = 1.0, far plane).
///
/// let v0 = (Vec3::new(0.0, 5.0, 5.0), 5.0);   // Top
/// let v1 = (Vec3::new(-5.0, -5.0, 5.0), 5.0); // Bottom Left
/// let v2 = (Vec3::new(5.0, -5.0, 5.0), 5.0);  // Bottom Right
/// let color = 0xFFFF0000; // Red
///
/// fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, color);
///
/// // Verify center pixel
/// assert_eq!(fb.get_pixel(50, 50), Some(0xFFFF0000));
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

    // Optimization: Early exit if fully transparent (alpha == 0).
    // In this engine, alpha=0 is typically treated as transparent by textured pipelines,
    // so flat shading should match this behavior.
    // Also, blend_swar treats alpha=0 as Opaque Source, which is likely unintended for "flat transparency".
    // Culling here fixes the inconsistency and optimizes performance.
    let alpha = (color >> 24) & 0xFF;
    if alpha == 0 {
        return;
    }

    let clipped = clip_triangle_to_frustum(v0, v1, v2, |v| (v.0, v.1));

    let width = fb.width();
    let height = fb.height();
    let half_width = width as f32 * 0.5;
    let half_height = height as f32 * 0.5;

    for i in 0..clipped.count {
        let base = i * 3;
        let v0 = clipped[base];
        let v1 = clipped[base + 1];
        let v2 = clipped[base + 2];

        // Project to screen
        let (p0_orig, p1_orig, p2_orig) =
            project_triangle_to_screen(v0.0, v0.1, v1.0, v1.1, v2.0, v2.1, half_width, half_height);

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
                        if alpha == 0xFF {
                            if zb.test_and_set_unchecked(x_start as usize, y as usize, z_left) {
                                fb.set_pixel_unchecked(x_start as usize, y as usize, color);
                            }
                        } else {
                            // Transparent single pixel
                            let z_current = zb.get_depth_unchecked(x_start as usize, y as usize);
                            if z_left < z_current {
                                let dest = fb.get_pixel_unchecked(x_start as usize, y as usize);
                                let blended = blend_swar(color, dest, 255 - alpha, alpha);
                                fb.set_pixel_unchecked(x_start as usize, y as usize, blended);
                            }
                        }
                    }
                }
            } else if alpha == 0xFF {
                draw_scanline_flat(fb, zb, y, x_start, x_end, z_left, dz_dx, color);
            } else {
                draw_scanline_flat_blended(fb, zb, y, x_start, x_end, z_left, dz_dx, color);
            }

            edge_a.step();
            edge_b.step();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec3;

    #[test]
    fn fill_triangle_3d_with_fixed_point_matches_reference() {
        let width = 200;
        let height = 200;
        let v0 = (Vec3::new(0.0, 0.5, 5.0), 5.0);
        let v1 = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
        let v2 = (Vec3::new(0.5, -0.5, 5.0), 5.0);
        let color = 0xFFFF_0000;

        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, color);

        let rendered_pixels = fb.as_slice().iter().filter(|&&p| p != 0xFF00_0000).count();
        assert!(rendered_pixels > 100);
        let center_pixel = fb.get_pixel((width / 2) as i32, (height / 2) as i32);
        assert_eq!(center_pixel, Some(color));
    }

    #[test]
    fn draw_scanline_flat_interpolation() {
        let width = 100;
        let height = 1;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        let z_start = 5.0;
        let dz_dx = 0.01;
        let color = 0xFFFF_0000;

        draw_scanline_flat(&mut fb, &mut zb, 0, 0, 99, z_start, dz_dx, color);

        for x in 0..width {
            assert_eq!(fb.get_pixel(x as i32, 0), Some(color));
        }

        let zb_slice = zb.as_slice();
        assert!((zb_slice[0] - 5.0).abs() < 0.0001);
        assert!(zb_slice[50] > 5.0 && zb_slice[50] < 6.0);
        assert!(zb_slice[99] > 5.0 && zb_slice[99] < 7.0);
    }
}
