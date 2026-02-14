//! 2D Rasterization primitives.
//!
//! Software rendering functions for 3D triangles (flat, gouraud, textured, lit).
//!
//! # Rasterization Rules
//!
//! This module implements a standard **Scanline Rasterization** algorithm.
//!
//! 1.  **Triangle Setup**: Vertices are sorted by Y-coordinate.
//! 2.  **Edge Walking**: The left and right edges of the triangle are traced row by row.
//! 3.  **Span Filling**: For each scanline, a horizontal span of pixels is filled between the left and right edges.
//! 4.  **Top-Left Rule**: To prevent double-drawing on shared edges, the rasterizer follows standard fill conventions.
//!
//! # Performance
//!
//! *   **Fixed-Point Math**: Internal interpolation often uses 16.16 fixed-point arithmetic for speed.
//! *   **Z-Buffering**: Depth testing is performed per-pixel.
//! *   **Clipping**: Triangles are clipped to the view frustum before rasterization to ensure safety.

use crate::clipping::{clip_line_to_frustum, clip_triangle_to_frustum};
use crate::framebuffer::Framebuffer;
use crate::math::{
    Mat4, ScreenPoint, Vec2, Vec3, Vec4, fast_inv_sqrt, project_to_screen_optimized,
};
use crate::texture::{FilterMode, Texture, blend_four_way, blend_swar};
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

/// Helper to clip a scanline span to the framebuffer width.
///
/// Returns `Some((new_x_start, new_x_end, skip_count))` if the span is valid (partially or fully on-screen).
/// * `new_x_start`: Clamped start X.
/// * `new_x_end`: Clamped end X.
/// * `skip_count`: Number of pixels skipped from the left (for advancing interpolators).
#[inline(always)]
fn clip_span(width: i32, x_start: i32, x_end: i32) -> Option<(i32, i32, f32)> {
    if x_start > x_end {
        return None;
    }

    let mut xs = x_start;
    let mut xe = x_end;
    let mut skip = 0.0;

    if xs < 0 {
        skip = -xs as f32;
        xs = 0;
    }

    if xe >= width {
        xe = width - 1;
    }

    if xs > xe {
        return None;
    }

    Some((xs, xe, skip))
}

/// Helper to retrieve mutable slices for a scanline.
///
/// # Safety
///
/// Caller must ensure `x_start` and `x_end` are within bounds `[0, width-1]`.
/// This is typically ensured by calling `clip_span` first.
#[inline(always)]
unsafe fn get_scanline_slices<'a>(
    fb: &'a mut Framebuffer,
    zb: &'a mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
) -> (&'a mut [u32], &'a mut [f32]) {
    let width_usize = fb.width() as usize;
    let y_offset = (y as usize) * width_usize;
    let start_idx = y_offset + (x_start as usize);
    let end_idx = y_offset + (x_end as usize);

    // SAFETY: Caller must ensure bounds.
    unsafe {
        (
            fb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx),
            zb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx),
        )
    }
}

/// Helper to prepare scanline slices.
/// Returns (`fb_slice`, `zb_slice`, `adjusted_z_start`) or `None` if off-screen.
#[inline(always)]
fn prepare_scanline<'a>(
    fb: &'a mut Framebuffer,
    zb: &'a mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    mut z: f32,
    dz_dx: f32,
) -> Option<(&'a mut [u32], &'a mut [f32], f32)> {
    let width = fb.width() as i32;
    if let Some((xs, xe, skip)) = clip_span(width, x_start, x_end) {
        z += skip * dz_dx;
        // SAFETY: clip_span ensures xs, xe are within bounds [0, width-1].
        let (fb_slice, zb_slice) = unsafe { get_scanline_slices(fb, zb, y, xs, xe) };
        Some((fb_slice, zb_slice, z))
    } else {
        None
    }
}

/// Helper to sort 3 vertices by Y coordinate
///
/// Optimization: Uses a manual sorting network to avoid the heap allocation
/// incurred by `slice::sort_by_key` for small arrays.
pub(crate) fn sort_by_y<T, F>(verts: &mut [T; 3], get_y: F)
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

/// Checks if a triangle is backfacing (or degenerate)
///
/// Uses the 2D cross product of the screen-space edges.
/// Returns true if the triangle should be culled (ccw winding for front faces).
#[inline(always)]
pub(crate) fn is_backface(p0: ScreenPoint, p1: ScreenPoint, p2: ScreenPoint) -> bool {
    let ux = i64::from(p1.x) - i64::from(p0.x);
    let uy = i64::from(p1.y) - i64::from(p0.y);
    let vx = i64::from(p2.x) - i64::from(p0.x);
    let vy = i64::from(p2.y) - i64::from(p0.y);
    // Use i64 for cross product.
    // i64 is sufficient as long as viewport width < 3e9, which is enforced by Framebuffer::new.
    let nz = ux * vy - uy * vx;
    nz >= 0
}

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
                let fb_ptr = fb_slice.as_mut_ptr().add(i).cast::<__m256i>();
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
                let fb_ptr = fb_slice.as_mut_ptr().add(i).cast::<__m256i>();
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

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_scanline_phong_shadowed(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    start: ShadowPhongSpanStart,
    gradients: &ShadowPhongGradients,
    pre_diffuse_255: Vec3,
    neg_light_dir: Vec3,
    ambient_255: Vec3,
    shadow_map: &ZBuffer,
    light_vp: Mat4,
) {
    let width = fb.width() as i32;
    let Some((xs, xe, skip)) = clip_span(width, x_start, x_end) else {
        return;
    };

    let mut z = start.z + skip * gradients.dz_dx;
    let mut q = start.q + skip * gradients.dq_dx;
    let mut nx = start.nx + skip * gradients.dnx_dx;
    let mut ny = start.ny + skip * gradients.dny_dx;
    let mut nz = start.nz + skip * gradients.dnz_dx;
    let mut wx = start.wx + skip * gradients.dwx_dx;
    let mut wy = start.wy + skip * gradients.dwy_dx;
    let mut wz = start.wz + skip * gradients.dwz_dx;

    // SAFETY: clip_span ensures xs, xe are within bounds.
    let (fb_slice, zb_slice) = unsafe { get_scanline_slices(fb, zb, y, xs, xe) };

    // Shadow Map dimensions
    let sm_w = shadow_map.width() as f32;
    let sm_h = shadow_map.height() as f32;
    let bias = 0.005; // Bias to prevent shadow acne

    for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
        if z < *depth_val {
            *depth_val = z;

            let w_recip = if q.abs() > 0.000_001 { 1.0 / q } else { 1.0 };

            // Recover world position
            let world_pos = Vec3::new(wx * w_recip, wy * w_recip, wz * w_recip);

            // Shadow Test
            let (light_clip, light_w) = light_vp.transform_point(world_pos);
            let mut shadow_factor = 1.0;

            if light_w > 0.0 {
                let inv_light_w = 1.0 / light_w;
                let ndc_x = light_clip.x * inv_light_w;
                let ndc_y = light_clip.y * inv_light_w;
                let ndc_z = light_clip.z * inv_light_w;

                // Check if inside light frustum
                if (-1.0..=1.0).contains(&ndc_x)
                    && (-1.0..=1.0).contains(&ndc_y)
                    && (-1.0..=1.0).contains(&ndc_z)
                {
                    // Map to texture coordinates [0, 1]
                    let u = (ndc_x + 1.0) * 0.5;
                    let v = (1.0 - ndc_y) * 0.5; // Flip Y for texture lookup

                    let sm_x = (u * sm_w) as i32;
                    let sm_y = (v * sm_h) as i32;

                    // PCF (Percentage Closer Filtering) 3x3
                    let mut shadow_sum = 0.0;
                    let mut samples = 0.0;

                    for y_off in -1..=1 {
                        for x_off in -1..=1 {
                            if let Some(closest_depth) =
                                shadow_map.get_depth(sm_x + x_off, sm_y + y_off)
                            {
                                if ndc_z > closest_depth + bias {
                                    // In shadow
                                } else {
                                    // Lit
                                    shadow_sum += 1.0;
                                }
                                samples += 1.0;
                            }
                        }
                    }

                    if samples > 0.0 {
                        shadow_factor = shadow_sum / samples;
                    }
                }
            }

            // Lighting
            // Deferred Normalization
            let len_sq = nx * nx + ny * ny + nz * nz;
            let dot_unorm = nx * neg_light_dir.x + ny * neg_light_dir.y + nz * neg_light_dir.z;

            let intensity = if len_sq > 0.0001 {
                let inv_len = fast_inv_sqrt(len_sq);
                (dot_unorm * inv_len).max(0.0)
            } else {
                0.0
            };

            let diffuse = pre_diffuse_255 * intensity * shadow_factor;
            let final_color_vec = ambient_255 + diffuse;
            *pixel = color_to_u32_scaled(final_color_vec);
        }

        z += gradients.dz_dx;
        q += gradients.dq_dx;
        nx += gradients.dnx_dx;
        ny += gradients.dny_dx;
        nz += gradients.dnz_dx;
        wx += gradients.dwx_dx;
        wy += gradients.dwy_dx;
        wz += gradients.dwz_dx;
    }
}

/// Fill a 3D triangle with Phong Shading and Shadow Mapping.
#[allow(clippy::too_many_arguments)]
pub fn fill_triangle_phong_shadowed(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec3, Vec3), // ((ClipPos, W), Normal, WorldPos)
    v1: ((Vec3, f32), Vec3, Vec3),
    v2: ((Vec3, f32), Vec3, Vec3),
    color: Vec3,
    light_dir: Vec3,
    light_color: Vec3,
    ambient: Vec3,
    shadow_map: &ZBuffer,
    light_vp: Mat4,
) {
    assert_same_dimensions(fb, zb);

    // Note: We use clip_triangle_to_frustum which uses Lerp.
    // Ensure ((Vec3, f32), Vec3, Vec3) implements Lerp in clipping.rs
    let clipped = clip_triangle_to_frustum(v0, v1, v2, |v| v.0);

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
        let p0_orig = project_to_screen_optimized(v0.0.0, v0.0.1, half_width, half_height);
        let p1_orig = project_to_screen_optimized(v1.0.0, v1.0.1, half_width, half_height);
        let p2_orig = project_to_screen_optimized(v2.0.0, v2.0.1, half_width, half_height);

        // Backface Culling
        if is_backface(p0_orig, p1_orig, p2_orig) {
            continue;
        }

        // Prepare attributes
        let inv_w0 = p0_orig.inv_w;
        let inv_w1 = p1_orig.inv_w;
        let inv_w2 = p2_orig.inv_w;

        // Normal * inv_w
        let n0 = v0.1 * inv_w0;
        let n1 = v1.1 * inv_w1;
        let n2 = v2.1 * inv_w2;

        // WorldPos * inv_w
        let w0 = v0.2 * inv_w0;
        let w1 = v1.2 * inv_w1;
        let w2 = v2.2 * inv_w2;

        let mut verts = [(p0_orig, n0, w0), (p1_orig, n1, w1), (p2_orig, n2, w2)];
        sort_by_y(&mut verts, |(p, ..)| p.y);
        let [(p0, n0, w0), (p1, n1, w1), (p2, n2, w2)] = verts;

        let q0 = p0.inv_w;
        let q1 = p1.inv_w;
        let q2 = p2.inv_w;

        let total_height = (i64::from(p2.y) - i64::from(p0.y)) as f32;
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
            let g = ShadowPhongGradients::new(p0, p1, p2, q0, q1, q2, n0, n1, n2, w0, w1, w2);
            let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
            let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
            let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
            let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
            let left = ux * vy - uy * vx > 0.0;
            (g, left)
        };

        let mut edge_a = ShadowPhongEdgeWalker::new(p0, p2, q0, q2, n0, n2, w0, w2);
        if y_start > p0.y {
            edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
        }

        let mut edge_b = if y_start < p1.y {
            let mut e = ShadowPhongEdgeWalker::new(p0, p1, q0, q1, n0, n1, w0, w1);
            if y_start > p0.y {
                e.step_n(i64::from(y_start) - i64::from(p0.y));
            }
            e
        } else {
            let mut e = ShadowPhongEdgeWalker::new(p1, p2, q1, q2, n1, n2, w1, w2);
            if y_start > p1.y {
                e.step_n(i64::from(y_start) - i64::from(p1.y));
            }
            e
        };

        // Lighting constants
        let pre_diffuse_255 = color * light_color * 255.0;
        let neg_light_dir = light_dir * -1.0;
        let ambient_255 = ambient * 255.0;

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b = ShadowPhongEdgeWalker::new(p1, p2, q1, q2, n1, n2, w1, w2);
            }

            let (
                x_start,
                x_end,
                z_left,
                nx_left,
                ny_left,
                nz_left,
                wx_left,
                wy_left,
                wz_left,
                q_left,
            ) = if long_edge_is_left {
                (
                    (edge_a.x >> 16) as i32,
                    (edge_b.x >> 16) as i32,
                    edge_a.z,
                    edge_a.nx,
                    edge_a.ny,
                    edge_a.nz,
                    edge_a.wx,
                    edge_a.wy,
                    edge_a.wz,
                    edge_a.q,
                )
            } else {
                (
                    (edge_b.x >> 16) as i32,
                    (edge_a.x >> 16) as i32,
                    edge_b.z,
                    edge_b.nx,
                    edge_b.ny,
                    edge_b.nz,
                    edge_b.wx,
                    edge_b.wy,
                    edge_b.wz,
                    edge_b.q,
                )
            };

            let dx = i64::from(x_end) - i64::from(x_start);

            if dx > 0 {
                draw_scanline_phong_shadowed(
                    fb,
                    zb,
                    y,
                    x_start,
                    x_end,
                    ShadowPhongSpanStart {
                        z: z_left,
                        q: q_left,
                        nx: nx_left,
                        ny: ny_left,
                        nz: nz_left,
                        wx: wx_left,
                        wy: wy_left,
                        wz: wz_left,
                    },
                    &gradients,
                    pre_diffuse_255,
                    neg_light_dir,
                    ambient_255,
                    shadow_map,
                    light_vp,
                );
            }

            edge_a.step();
            edge_b.step();
        }
    }
}

/// Convert Vec3 color (0.0-1.0 per channel) to u32 ARGB
#[must_use]
pub fn color_to_u32(color: Vec3) -> u32 {
    let r = (color.x.clamp(0.0, 1.0) * 255.0) as u32;
    let g = (color.y.clamp(0.0, 1.0) * 255.0) as u32;
    let b = (color.z.clamp(0.0, 1.0) * 255.0) as u32;
    0xFF00_0000 | (r << 16) | (g << 8) | b
}

/// Helper to convert pre-scaled (0.0-255.0) Vec3 color to u32 ARGB
#[must_use]
#[inline(always)]
#[allow(clippy::missing_const_for_fn)]
fn color_to_u32_scaled(color: Vec3) -> u32 {
    let r = color.x.clamp(0.0, 255.0) as u32;
    let g = color.y.clamp(0.0, 255.0) as u32;
    let b = color.z.clamp(0.0, 255.0) as u32;
    0xFF00_0000 | (r << 16) | (g << 8) | b
}

/// Helper to pack 8-bit color channels into u32 ARGB
#[inline(always)]
const fn pack_color_channels(r: u32, g: u32, b: u32) -> u32 {
    0xFF00_0000 | (r << 16) | (g << 8) | b
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
pub(crate) const FIXED_SCALE: f32 = 65536.0;

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
    let Some((xs, xe, skip)) = clip_span(width, x_start, x_end) else {
        return;
    };

    // Use i64 for accumulators to prevent overflow when x_start is far off-screen
    // Note: c_start is already at x_start.
    // We need to advance by `skip`
    let (dr, dg, db) = (i64::from(dc_dx.0), i64::from(dc_dx.1), i64::from(dc_dx.2));

    let mut z = z_start + skip * dz_dx;
    let skip_i64 = skip as i64;
    let r_i = c_start.0 + skip_i64 * dr;
    let g_i = c_start.1 + skip_i64 * dg;
    let b_i = c_start.2 + skip_i64 * db;

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

    // SAFETY: clip_span ensures xs, xe are within bounds.
    let (fb_slice, zb_slice) = unsafe { get_scanline_slices(fb, zb, y, xs, xe) };

    // Optimization: Check for fast path (no clamping needed)
    // If all color channels are within [0, 255] for the entire span, we can skip clamping.
    // r_i is 16.16 fixed point. Max value is 255.0 = 0x00FF_0000.
    // We calculate end values based on start + delta * count.
    let count = xe - xs;
    let r_end = r_i.wrapping_add(dr.wrapping_mul(count));
    let g_end = g_i.wrapping_add(dg.wrapping_mul(count));
    let b_end = b_i.wrapping_add(db.wrapping_mul(count));

    // Use strict upper bound 0x0100_0000 (256.0) to ensure integer part fits in u8.
    // Cast to u32 handles negative check (becomes large u32).
    let safe_limit: u32 = 0x0100_0000;
    let safe = (r_i as u32) < safe_limit
        && (r_end as u32) < safe_limit
        && (g_i as u32) < safe_limit
        && (g_end as u32) < safe_limit
        && (b_i as u32) < safe_limit
        && (b_end as u32) < safe_limit;

    if safe {
        for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
            if z < *depth_val {
                *depth_val = z;
                // Fast path: direct shift, no clamp/mask
                // r_i as u32 >> 16 extracts the integer part (0..255)
                let r = (r_i as u32) >> 16;
                let g = (g_i as u32) >> 16;
                let b = (b_i as u32) >> 16;

                *pixel = 0xFF00_0000 | (r << 16) | (g << 8) | b;
            }
            z += dz_dx;
            r_i += dr;
            g_i += dg;
            b_i += db;
        }
    } else {
        for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
            // Check depth buffer
            if z < *depth_val {
                *depth_val = z;
                // Unpack fixed point color
                // Optimization: Combine clamp and mask to avoid shifts and intermediate u8 casts
                // 16.16 fixed point means 255.0 is 0x00FF0000
                let r = r_i.clamp(0, 0x00FF_0000);
                let g = g_i.clamp(0, 0x00FF_0000);
                let b = b_i.clamp(0, 0x00FF_0000);

                *pixel = 0xFF00_0000
                    | ((r as u32) & 0x00FF_0000)
                    | (((g as u32) & 0x00FF_0000) >> 8)
                    | (((b as u32) & 0x00FF_0000) >> 16);
            }
            z += dz_dx;
            r_i += dr;
            g_i += dg;
            b_i += db;
        }
    }
}

/// Fill a 3D triangle with Gouraud (per-vertex) shading.
///
/// This function linearly interpolates colors across the face of the triangle.
/// Lighting calculations are performed at vertices, and the resulting colors
/// are passed to this function.
///
/// # Arguments
///
/// *   `v0`, `v1`, `v2` - Vertices defined as `((Position, W), Color)`.
///     *   `Position`: Clip Space position.
///     *   `W`: Homogeneous W coordinate.
///     *   `Color`: RGB color (0.0 - 1.0) for the vertex.
///
/// # Examples
///
/// ```
/// use abrash::rasterizer::fill_triangle_gouraud;
/// use abrash::framebuffer::Framebuffer;
/// use abrash::zbuffer::ZBuffer;
/// use abrash::math::Vec3;
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// let mut zb = ZBuffer::new(100, 100).unwrap();
///
/// // Vertices: ((x, y, z, w), (r, g, b))
/// let v0 = ((Vec3::new(0.0, 0.5, 5.0), 5.0), Vec3::new(1.0, 0.0, 0.0)); // Red
/// let v1 = ((Vec3::new(-0.5, -0.5, 5.0), 5.0), Vec3::new(0.0, 1.0, 0.0)); // Green
/// let v2 = ((Vec3::new(0.5, -0.5, 5.0), 5.0), Vec3::new(0.0, 0.0, 1.0)); // Blue
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

    let clipped = clip_triangle_to_frustum(v0, v1, v2, |v| v.0);

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
        let p0_orig = project_to_screen_optimized(v0.0.0, v0.0.1, half_width, half_height);
        let p1_orig = project_to_screen_optimized(v1.0.0, v1.0.1, half_width, half_height);
        let p2_orig = project_to_screen_optimized(v2.0.0, v2.0.1, half_width, half_height);

        // Backface Culling
        if is_backface(p0_orig, p1_orig, p2_orig) {
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

        let total_height = (i64::from(p2.y) - i64::from(p0.y)) as f32;
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
            edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
        }

        let mut edge_b = if y_start < p1.y {
            let mut e = GouraudEdgeWalker::new(p0, p1, c0, c1);
            if y_start > p0.y {
                e.step_n(i64::from(y_start) - i64::from(p0.y));
            }
            e
        } else {
            let mut e = GouraudEdgeWalker::new(p1, p2, c1, c2);
            if y_start > p1.y {
                e.step_n(i64::from(y_start) - i64::from(p1.y));
            }
            e
        };

        let width_i32 = width as i32;

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b = GouraudEdgeWalker::new(p1, p2, c1, c2);
            }

            let (x_start, x_end, z_left, c_left) = if long_edge_is_left {
                (
                    (edge_a.x >> 16) as i32,
                    (edge_b.x >> 16) as i32,
                    edge_a.z,
                    edge_a.c,
                )
            } else {
                (
                    (edge_b.x >> 16) as i32,
                    (edge_a.x >> 16) as i32,
                    edge_b.z,
                    edge_b.c,
                )
            };

            let dx = i64::from(x_end) - i64::from(x_start);

            if dx <= 0 {
                if x_start >= 0 && x_start < width_i32 {
                    // SAFETY: Safe due to clamps on x_start and y
                    unsafe {
                        if zb.test_and_set_unchecked(x_start as usize, y as usize, z_left) {
                            fb.set_pixel_unchecked(
                                x_start as usize,
                                y as usize,
                                pack_color_fixed(c_left),
                            );
                        }
                    }
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

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_span_nearest(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    texture: &Texture,
    mut z: f32,
    dz_dx: f32,
    mut u_fix: i32,
    mut v_fix: i32,
    du_fix: i32,
    dv_fix: i32,
) {
    // Hoist texture properties
    let tex_pixels = &texture.pixels;
    let tex_w = texture.width;
    let tex_h = texture.height;
    let tex_w_usize = tex_w as usize;

    let shift = texture.width_shift;
    // Optimization: Loop versioning.
    // Duplicate the loop to specialize for power-of-two textures.
    // This hoists the branch `if shift < 32` out of the tight loop and allows
    // the use of bitwise shifting `v << shift` instead of multiplication `v * width`.
    if shift < 32 {
        for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
            if z < *depth_val {
                // Inline sampling
                let u = u_fix >> 16;
                let v = v_fix >> 16;
                let color = if (u as u32) < tex_w && (v as u32) < tex_h {
                    tex_pixels[((v as usize) << shift) + (u as usize)]
                } else {
                    texture.get_pixel_texel(u, v)
                };

                let alpha = (color >> 24) & 0xFF;
                if alpha == 255 {
                    *depth_val = z;
                    *pixel = color;
                } else if alpha > 0 {
                    let dest = *pixel;
                    *pixel = blend_swar(color, dest, 255 - alpha, alpha);
                }
            }
            z += dz_dx;
            u_fix = u_fix.wrapping_add(du_fix);
            v_fix = v_fix.wrapping_add(dv_fix);
        }
    } else {
        for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
            if z < *depth_val {
                // Inline sampling
                let u = u_fix >> 16;
                let v = v_fix >> 16;
                let color = if (u as u32) < tex_w && (v as u32) < tex_h {
                    tex_pixels[(v as usize) * tex_w_usize + (u as usize)]
                } else {
                    texture.get_pixel_texel(u, v)
                };

                let alpha = (color >> 24) & 0xFF;
                if alpha == 255 {
                    *depth_val = z;
                    *pixel = color;
                } else if alpha > 0 {
                    let dest = *pixel;
                    *pixel = blend_swar(color, dest, alpha, 255 - alpha);
                }
            }
            z += dz_dx;
            u_fix = u_fix.wrapping_add(du_fix);
            v_fix = v_fix.wrapping_add(dv_fix);
        }
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_span_bilinear(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    texture: &Texture,
    mut z: f32,
    dz_dx: f32,
    mut u_fix: i32,
    mut v_fix: i32,
    du_fix: i32,
    dv_fix: i32,
) {
    let tex_pixels = &texture.pixels;
    let tex_w = texture.width;
    let tex_h = texture.height;
    let shift = texture.width_shift;

    let w_i32 = (tex_w as i32).wrapping_sub(1);
    let h_i32 = (tex_h as i32).wrapping_sub(1);
    let tex_w_usize = tex_w as usize;

    // Optimization: Use a macro to hoist the `shift < 32` check out of the hot loop.
    // This allows the compiler to generate two specialized versions of the loop:
    // one using bit-shifting (fast) and one using multiplication (slower),
    // without branching inside the loop for every pixel.
    macro_rules! process_span_bilinear {
        ($op:tt, $val:expr) => {
            let mut cached_x0 = i32::MIN;
            let mut cached_y0 = i32::MIN;
            let mut c00 = 0;
            let mut c10 = 0;
            let mut c01 = 0;
            let mut c11 = 0;

            for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
                if z < *depth_val {
                    let u_img_fixed = u_fix >> 8;
                    let v_img_fixed = v_fix >> 8;

                    let x0_raw = u_img_fixed >> 8;
                    let y0_raw = v_img_fixed >> 8;

                    if x0_raw != cached_x0 || y0_raw != cached_y0 {
                        cached_x0 = x0_raw;
                        cached_y0 = y0_raw;

                        let (t00, t10, t01, t11) =
                            if (x0_raw as u32) < (w_i32 as u32) && (y0_raw as u32) < (h_i32 as u32) {
                                let x0 = x0_raw as usize;
                                let y0 = y0_raw as usize;

                                let row0 = y0 $op $val;
                                let row1 = row0 + tex_w_usize;

                                unsafe {
                                    // Optimization: Read 2 pixels at a time as u64.
                                    #[cfg(target_endian = "little")]
                                    {
                                        let ptr = tex_pixels.as_ptr();
                                        let row0_pair = ptr.add(row0 + x0).cast::<u64>().read_unaligned();
                                        let row1_pair = ptr.add(row1 + x0).cast::<u64>().read_unaligned();

                                        (
                                            row0_pair as u32,
                                            (row0_pair >> 32) as u32,
                                            row1_pair as u32,
                                            (row1_pair >> 32) as u32,
                                        )
                                    }
                                    #[cfg(not(target_endian = "little"))]
                                    {
                                        (
                                            *tex_pixels.get_unchecked(row0 + x0),
                                            *tex_pixels.get_unchecked(row0 + x0 + 1),
                                            *tex_pixels.get_unchecked(row1 + x0),
                                            *tex_pixels.get_unchecked(row1 + x0 + 1),
                                        )
                                    }
                                }
                            } else {
                                let x0 = x0_raw.clamp(0, w_i32) as usize;
                                let y0 = y0_raw.clamp(0, h_i32) as usize;
                                let x1 = (x0_raw + 1).clamp(0, w_i32) as usize;
                                let y1 = (y0_raw + 1).clamp(0, h_i32) as usize;

                                let row0 = y0 $op $val;
                                let row1 = y1 $op $val;

                                unsafe {
                                    (
                                        *tex_pixels.get_unchecked(row0 + x0),
                                        *tex_pixels.get_unchecked(row0 + x1),
                                        *tex_pixels.get_unchecked(row1 + x0),
                                        *tex_pixels.get_unchecked(row1 + x1),
                                    )
                                }
                            };
                        c00 = t00;
                        c10 = t10;
                        c01 = t01;
                        c11 = t11;
                    }

                    let wx = (u_img_fixed & 0xFF) as u32;
                    let wy = (v_img_fixed & 0xFF) as u32;

                    let final_color = blend_four_way(c00, c10, c01, c11, wx, wy);

                    let alpha = (final_color >> 24) & 0xFF;
                    if alpha == 255 {
                        *depth_val = z;
                        *pixel = final_color;
                    } else if alpha > 0 {
                        let dest = *pixel;
                        *pixel = blend_swar(final_color, dest, 255 - alpha, alpha);
                    }
                }
                z += dz_dx;
                u_fix = u_fix.wrapping_add(du_fix);
                v_fix = v_fix.wrapping_add(dv_fix);
            }
        };
    }

    if shift < 32 {
        process_span_bilinear!(<<, shift);
    } else {
        process_span_bilinear!(*, tex_w_usize);
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_span_trilinear(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    texture: &Texture,
    mut z: f32,
    dz_dx: f32,
    mut u_fix: i32,
    mut v_fix: i32,
    du_fix: i32,
    dv_fix: i32,
    lod: f32,
) {
    for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
        if z < *depth_val {
            let color = texture.get_pixel_trilinear_fixed(u_fix, v_fix, lod);
            let alpha = (color >> 24) & 0xFF;

            if alpha == 255 {
                *depth_val = z;
                *pixel = color;
            } else if alpha > 0 {
                let dest = *pixel;
                *pixel = blend_swar(color, dest, 255 - alpha, alpha);
            }
        }
        z += dz_dx;
        u_fix = u_fix.wrapping_add(du_fix);
        v_fix = v_fix.wrapping_add(dv_fix);
    }
}

/// Draw a single scanline with perspective-correct texture mapping
/// Optimized using span-based interpolation (every 16 pixels)
#[inline(always)]
#[allow(clippy::too_many_arguments)]
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
    let Some((xs, xe, skip)) = clip_span(width, x_start, x_end) else {
        return;
    };

    let mut z = start.z + skip * gradients.dz_dx;
    let mut q = start.q + skip * gradients.dq_dx;
    let mut u = start.u + skip * gradients.du_dx;
    let mut v = start.v + skip * gradients.dv_dx;

    let span_size = 16;
    let mut x = xs;

    // Calculate initial start values
    let w_start = if q.abs() > 0.000_001 { 1.0 / q } else { 1.0 };
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
        let w_end = if q_end.abs() > 0.000_001 {
            1.0 / q_end
        } else {
            1.0
        };
        let u_tex_end = u_end * w_end;
        let v_tex_end = v_end * w_end;

        // Interpolate texel coordinates linearly over the span
        // Optimization: Use reciprocal table to replace division with multiplication
        // count is guaranteed to be in [1, 16]
        let inv_count = RECIPROCAL_TABLE[count as usize];
        let du_tex_step = (u_tex_end - u_tex_start) * inv_count;
        let dv_tex_step = (v_tex_end - v_tex_start) * inv_count;

        // SAFETY: loop logic ensures x and x+count-1 are <= xe, and xe <= width-1
        let (fb_slice, zb_slice) = unsafe { get_scanline_slices(fb, zb, y, x, x + count - 1) };

        match texture.filter_mode {
            FilterMode::Nearest => {
                // Fixed point optimization for Nearest Neighbor
                let u_fix = (u_tex_start * 65536.0) as i32;
                let v_fix = (v_tex_start * 65536.0) as i32;
                let du_fix = (du_tex_step * 65536.0) as i32;
                let dv_fix = (dv_tex_step * 65536.0) as i32;

                draw_span_nearest(
                    fb_slice,
                    zb_slice,
                    texture,
                    z,
                    gradients.dz_dx,
                    u_fix,
                    v_fix,
                    du_fix,
                    dv_fix,
                );
            }
            FilterMode::Bilinear => {
                // Fixed point optimization for Bilinear
                // Use 16.16 for accumulation to maintain precision, then downshift to 24.8 for sampling
                // Optimization: Subtract 0.5 (128 units in 24.8, 32768 in 16.16) upfront
                // to avoid per-pixel subtraction in get_pixel_bilinear_fixed
                let u_fix = ((u_tex_start * 65536.0) as i32).wrapping_sub(32768);
                let v_fix = ((v_tex_start * 65536.0) as i32).wrapping_sub(32768);
                let du_fix = (du_tex_step * 65536.0) as i32;
                let dv_fix = (dv_tex_step * 65536.0) as i32;

                draw_span_bilinear(
                    fb_slice,
                    zb_slice,
                    texture,
                    z,
                    gradients.dz_dx,
                    u_fix,
                    v_fix,
                    du_fix,
                    dv_fix,
                );
            }
            FilterMode::Trilinear => {
                // For Trilinear, we need LOD.
                // Calculate LOD at span start to avoid extra per-pixel work.
                let w = w_start; // 1/q
                let w_sq = w * w;

                // Derivatives of texture coordinates with respect to screen x/y.
                // u_tex = u / q, v_tex = v / q.
                let du_tex_dx = (gradients.du_dx * q - u * gradients.dq_dx) * w_sq;
                let dv_tex_dx = (gradients.dv_dx * q - v * gradients.dq_dx) * w_sq;
                let du_tex_dy = (gradients.du_dy * q - u * gradients.dq_dy) * w_sq;
                let dv_tex_dy = (gradients.dv_dy * q - v * gradients.dq_dy) * w_sq;

                let max_rho_sq = (du_tex_dx * du_tex_dx + dv_tex_dx * dv_tex_dx)
                    .max(du_tex_dy * du_tex_dy + dv_tex_dy * dv_tex_dy);
                let lod = 0.5 * max_rho_sq.log2();

                let u_fix = (u_tex_start * 65536.0) as i32;
                let v_fix = (v_tex_start * 65536.0) as i32;
                let du_fix = (du_tex_step * 65536.0) as i32;
                let dv_fix = (dv_tex_step * 65536.0) as i32;

                draw_span_trilinear(
                    fb_slice,
                    zb_slice,
                    texture,
                    z,
                    gradients.dz_dx,
                    u_fix,
                    v_fix,
                    du_fix,
                    dv_fix,
                    lod,
                );
            }
        }

        // Advance state
        z += gradients.dz_dx * count as f32;
        q = q_end;
        u = u_end;
        v = v_end;

        // Reuse end values for next start
        u_tex_start = u_tex_end;
        v_tex_start = v_tex_end;

        x += count;
    }
}
/// Fill a 3D triangle with texture mapping.
///
/// This function performs perspective-correct texture mapping using standard scanline rasterization.
/// It interpolates texture coordinates ($u, v$) and perspective term ($1/w$) across the triangle surface.
///
/// # Arguments
///
/// *   `fb` - Target framebuffer.
/// *   `zb` - Target z-buffer.
/// *   `v0`, `v1`, `v2` - Vertices, each defined as `((Position, W), UV)`.
///     *   `Position`: 3D vertex position in Clip Space (before perspective divide).
///     *   `W`: Homogeneous W coordinate (distance from camera plane).
///     *   `UV`: Texture coordinates in range $[0.0, 1.0]$.
/// *   `texture` - The source texture to map onto the triangle.
///
/// # Perspective Correction
///
/// To avoid texture swimming (warping) when viewing triangles at an angle, this rasterizer
/// performs perspective-correct interpolation:
/// 1.  At each vertex, calculate $q = 1/w$, $u' = u/w$, $v' = v/w$.
/// 2.  Linearly interpolate $q, u', v'$ across the screen.
/// 3.  Per-pixel (or per-span), recover $u = u'/q$ and $v = v'/q$.
///
/// # Examples
///
/// ```
/// use abrash::rasterizer::fill_triangle_textured;
/// use abrash::framebuffer::Framebuffer;
/// use abrash::zbuffer::ZBuffer;
/// use abrash::texture::Texture;
/// use abrash::math::{Vec3, Vec2};
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// let mut zb = ZBuffer::new(100, 100).unwrap();
///
/// // Create a simple checkerboard texture
/// let texture = Texture::checkered(32, 32, 0xFFFFFFFF, 0xFF000000).unwrap();
///
/// // Define vertices in Clip Space ((Position, W), UV)
/// // Triangle covering center of screen
/// let v0 = ((Vec3::new(0.0, 0.5, 5.0), 5.0), Vec2::new(0.5, 0.0));
/// let v1 = ((Vec3::new(-0.5, -0.5, 5.0), 5.0), Vec2::new(0.0, 1.0));
/// let v2 = ((Vec3::new(0.5, -0.5, 5.0), 5.0), Vec2::new(1.0, 1.0));
///
/// fill_triangle_textured(&mut fb, &mut zb, v0, v1, v2, &texture);
///
/// // Verify center pixel was drawn
/// assert_ne!(fb.get_pixel(50, 50), Some(0x00000000));
/// ```
pub fn fill_triangle_textured(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec2),
    v1: ((Vec3, f32), Vec2),
    v2: ((Vec3, f32), Vec2),
    texture: &Texture,
) {
    assert_same_dimensions(fb, zb);

    let clipped = clip_triangle_to_frustum(v0, v1, v2, |v| v.0);

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
        let p0_orig = project_to_screen_optimized(v0.0.0, v0.0.1, half_width, half_height);
        let p1_orig = project_to_screen_optimized(v1.0.0, v1.0.1, half_width, half_height);
        let p2_orig = project_to_screen_optimized(v2.0.0, v2.0.1, half_width, half_height);

        // Backface Culling
        let ux_orig = (i64::from(p1_orig.x) - i64::from(p0_orig.x)) as f32;
        let uy_orig = (i64::from(p1_orig.y) - i64::from(p0_orig.y)) as f32;
        let vx_orig = (i64::from(p2_orig.x) - i64::from(p0_orig.x)) as f32;
        let vy_orig = (i64::from(p2_orig.y) - i64::from(p0_orig.y)) as f32;
        let nz_orig = ux_orig * vy_orig - uy_orig * vx_orig;

        if nz_orig >= 0.0 {
            continue;
        }

        // Prepare perspective attributes: q=1/w, u/w, v/w
        // Note: We multiply UV by texture dimensions here so interpolation happens in texel space

        // Optimization: Reuse inv_w calculated during projection to avoid division
        let inv_w0 = p0_orig.inv_w;
        let inv_w1 = p1_orig.inv_w;
        let inv_w2 = p2_orig.inv_w;

        let u0 = v0.1.x * texture.width as f32 * inv_w0;
        let v0_val = v0.1.y * texture.height as f32 * inv_w0;

        let u1 = v1.1.x * texture.width as f32 * inv_w1;
        let v1_val = v1.1.y * texture.height as f32 * inv_w1;

        let u2 = v2.1.x * texture.width as f32 * inv_w2;
        let v2_val = v2.1.y * texture.height as f32 * inv_w2;

        // Sort by y
        // We need to keep track of all attributes (p, u, v) - q is inside p
        let mut verts = [
            (p0_orig, u0, v0_val),
            (p1_orig, u1, v1_val),
            (p2_orig, u2, v2_val),
        ];
        sort_by_y(&mut verts, |(p, _, _)| p.y);
        let [(p0, u0, v0), (p1, u1, v1), (p2, u2, v2)] = verts;

        let q0 = p0.inv_w;
        let q1 = p1.inv_w;
        let q2 = p2.inv_w;

        let total_height = (i64::from(p2.y) - i64::from(p0.y)) as f32;
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

            let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
            let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
            let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
            let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
            let left = ux * vy - uy * vx > 0.0;

            (g, left)
        };

        let mut edge_a = PerspectiveTextureEdgeWalker::new(p0, p2, q0, q2, u0, u2, v0, v2);
        if y_start > p0.y {
            edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
        }

        let mut edge_b = if y_start < p1.y {
            let mut e = PerspectiveTextureEdgeWalker::new(p0, p1, q0, q1, u0, u1, v0, v1);
            if y_start > p0.y {
                e.step_n(i64::from(y_start) - i64::from(p0.y));
            }
            e
        } else {
            let mut e = PerspectiveTextureEdgeWalker::new(p1, p2, q1, q2, u1, u2, v1, v2);
            if y_start > p1.y {
                e.step_n(i64::from(y_start) - i64::from(p1.y));
            }
            e
        };

        let width_i32 = width as i32;

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b = PerspectiveTextureEdgeWalker::new(p1, p2, q1, q2, u1, u2, v1, v2);
            }

            let (x_start, x_end, z_left, q_left, u_left, v_left) = if long_edge_is_left {
                (
                    (edge_a.x >> 16) as i32,
                    (edge_b.x >> 16) as i32,
                    edge_a.z,
                    edge_a.q,
                    edge_a.u,
                    edge_a.v,
                )
            } else {
                (
                    (edge_b.x >> 16) as i32,
                    (edge_a.x >> 16) as i32,
                    edge_b.z,
                    edge_b.q,
                    edge_b.u,
                    edge_b.v,
                )
            };

            let dx = i64::from(x_end) - i64::from(x_start);

            if dx <= 0 {
                if x_start >= 0 && x_start < width_i32 && q_left.abs() > 0.000_001 {
                    // SAFETY: Safe due to clamps on x_start and y
                    unsafe {
                        let z_current = zb.get_depth_unchecked(x_start as usize, y as usize);
                        if z_left < z_current {
                            let w = 1.0 / q_left;
                            let u_tex = u_left * w;
                            let v_tex = v_left * w;
                            let color = match texture.filter_mode {
                                FilterMode::Nearest => {
                                    texture.get_pixel_texel(u_tex as i32, v_tex as i32)
                                }
                                FilterMode::Bilinear => {
                                    texture.get_pixel_bilinear_texel(u_tex, v_tex)
                                }
                                FilterMode::Trilinear => {
                                    // Calculate LOD for single pixel
                                    // q = 1/w.
                                    // u_tex = u/q.
                                    // du_tex/dx = (du/dx * q - u * dq/dx) / q^2
                                    let w = 1.0 / q_left;
                                    let w_sq = w * w;

                                    let du_tex_dx = (gradients.du_dx * q_left
                                        - u_left * gradients.dq_dx)
                                        * w_sq;
                                    let dv_tex_dx = (gradients.dv_dx * q_left
                                        - v_left * gradients.dq_dx)
                                        * w_sq;
                                    let du_tex_dy = (gradients.du_dy * q_left
                                        - u_left * gradients.dq_dy)
                                        * w_sq;
                                    let dv_tex_dy = (gradients.dv_dy * q_left
                                        - v_left * gradients.dq_dy)
                                        * w_sq;

                                    let max_rho_sq = (du_tex_dx * du_tex_dx
                                        + dv_tex_dx * dv_tex_dx)
                                        .max(du_tex_dy * du_tex_dy + dv_tex_dy * dv_tex_dy);

                                    let lod = 0.5 * max_rho_sq.log2();
                                    texture.get_pixel_trilinear(u_tex, v_tex, lod)
                                }
                            };

                            let alpha = (color >> 24) & 0xFF;
                            if alpha == 255 {
                                // Manually update Z
                                let width_usize = fb.width() as usize;
                                let idx = (y as usize) * width_usize + (x_start as usize);
                                *zb.as_mut_slice().get_unchecked_mut(idx) = z_left;
                                fb.set_pixel_unchecked(x_start as usize, y as usize, color);
                            } else if alpha > 0 {
                                let dest = fb.get_pixel_unchecked(x_start as usize, y as usize);
                                let blended = blend_swar(color, dest, 255 - alpha, alpha);
                                fb.set_pixel_unchecked(x_start as usize, y as usize, blended);
                            }
                        }
                    }
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

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2")]
#[allow(clippy::too_many_arguments)]
unsafe fn draw_scanline_phong_simd(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    z: f32,
    nx: f32,
    ny: f32,
    nz: f32,
    gradients: &PhongGradients,
    pre_diffuse_255: Vec3, // Pre-scaled by 255.0
    neg_light_dir: Vec3,
    ambient_255: Vec3, // Pre-scaled by 255.0
) {
    unsafe {
        use std::arch::x86_64::*;

        let len = fb_slice.len();
        let mut i = 0;

        // Load constants
        let dz_dx_vec = _mm256_set1_ps(gradients.dz_dx);
        let dnx_dx_vec = _mm256_set1_ps(gradients.dnx_dx);
        let dny_dx_vec = _mm256_set1_ps(gradients.dny_dx);
        let dnz_dx_vec = _mm256_set1_ps(gradients.dnz_dx);

        let lx = _mm256_set1_ps(neg_light_dir.x);
        let ly = _mm256_set1_ps(neg_light_dir.y);
        let lz = _mm256_set1_ps(neg_light_dir.z);

        let diff_r = _mm256_set1_ps(pre_diffuse_255.x);
        let diff_g = _mm256_set1_ps(pre_diffuse_255.y);
        let diff_b = _mm256_set1_ps(pre_diffuse_255.z);

        let amb_r = _mm256_set1_ps(ambient_255.x);
        let amb_g = _mm256_set1_ps(ambient_255.y);
        let amb_b = _mm256_set1_ps(ambient_255.z);

        let epsilon = _mm256_set1_ps(0.0001);
        let zero = _mm256_setzero_ps();
        let one = _mm256_set1_ps(1.0);
        let one_point_five = _mm256_set1_ps(1.5);
        let zero_point_five = _mm256_set1_ps(0.5);
        let scale_255 = _mm256_set1_ps(255.0);
        let alpha_mask = _mm256_set1_epi32(0xFF00_0000u32 as i32);

        // Initial offsets for 8 pixels
        let offsets = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);
        let mut z_vec = _mm256_add_ps(_mm256_set1_ps(z), _mm256_mul_ps(dz_dx_vec, offsets));
        let mut nx_vec = _mm256_add_ps(_mm256_set1_ps(nx), _mm256_mul_ps(dnx_dx_vec, offsets));
        let mut ny_vec = _mm256_add_ps(_mm256_set1_ps(ny), _mm256_mul_ps(dny_dx_vec, offsets));
        let mut nz_vec = _mm256_add_ps(_mm256_set1_ps(nz), _mm256_mul_ps(dnz_dx_vec, offsets));

        // Steps for 8 pixels
        let dz_step = _mm256_mul_ps(dz_dx_vec, _mm256_set1_ps(8.0));
        let dnx_step = _mm256_mul_ps(dnx_dx_vec, _mm256_set1_ps(8.0));
        let dny_step = _mm256_mul_ps(dny_dx_vec, _mm256_set1_ps(8.0));
        let dnz_step = _mm256_mul_ps(dnz_dx_vec, _mm256_set1_ps(8.0));

        while i + 8 <= len {
            // Load depth buffer
            let depth_ptr = zb_slice.as_mut_ptr().add(i);
            let depth_val = _mm256_loadu_ps(depth_ptr);

            // Z-test
            let mask = _mm256_cmp_ps(z_vec, depth_val, _CMP_LT_OQ);
            let mask_int = _mm256_castps_si256(mask);

            // If any pixel passes Z-test
            if _mm256_movemask_ps(mask) != 0 {
                // Update Z-buffer
                // Optimization: Use load-blend-store instead of maskstore which can be slow
                let old_z = _mm256_loadu_ps(depth_ptr);
                let new_z = _mm256_blendv_ps(old_z, z_vec, mask);
                _mm256_storeu_ps(depth_ptr, new_z);

                // Shading
                let nx_sq = _mm256_mul_ps(nx_vec, nx_vec);
                let ny_sq = _mm256_mul_ps(ny_vec, ny_vec);
                let nz_sq = _mm256_mul_ps(nz_vec, nz_vec);
                let len_sq = _mm256_add_ps(nx_sq, _mm256_add_ps(ny_sq, nz_sq));

                // Check if len_sq > epsilon
                let len_valid = _mm256_cmp_ps(len_sq, epsilon, _CMP_GT_OQ);

                // Calculate rsqrt. Avoid rsqrt(0) by blending with 1.0 (doesn't matter what value, masked out later)
                let safe_len_sq = _mm256_blendv_ps(one, len_sq, len_valid);
                let rsqrt = _mm256_rsqrt_ps(safe_len_sq);

                // Newton-Raphson iteration: y = y * (1.5 - 0.5 * x * y * y)
                let iter1 = _mm256_mul_ps(safe_len_sq, _mm256_mul_ps(rsqrt, rsqrt));
                let iter2 = _mm256_sub_ps(one_point_five, _mm256_mul_ps(zero_point_five, iter1));
                let inv_len = _mm256_mul_ps(rsqrt, iter2);

                // Dot product (unnormalized)
                let dot_x = _mm256_mul_ps(nx_vec, lx);
                let dot_y = _mm256_mul_ps(ny_vec, ly);
                let dot_z = _mm256_mul_ps(nz_vec, lz);
                let dot_unorm = _mm256_add_ps(dot_x, _mm256_add_ps(dot_y, dot_z));

                // Intensity
                let intensity_raw = _mm256_mul_ps(dot_unorm, inv_len);
                let intensity = _mm256_max_ps(zero, intensity_raw);

                // Apply mask for valid length
                let intensity = _mm256_blendv_ps(zero, intensity, len_valid);

                // Calculate Color (Pre-scaled)
                let r = _mm256_add_ps(amb_r, _mm256_mul_ps(diff_r, intensity));
                let g = _mm256_add_ps(amb_g, _mm256_mul_ps(diff_g, intensity));
                let b = _mm256_add_ps(amb_b, _mm256_mul_ps(diff_b, intensity));

                // Clamp and convert to u32
                // Clamp 0.0-255.0
                let r_clamp = _mm256_min_ps(_mm256_max_ps(r, zero), scale_255);
                let g_clamp = _mm256_min_ps(_mm256_max_ps(g, zero), scale_255);
                let b_clamp = _mm256_min_ps(_mm256_max_ps(b, zero), scale_255);

                // Already scaled
                let r_255 = r_clamp;
                let g_255 = g_clamp;
                let b_255 = b_clamp;

                // Convert to i32 (truncation match scalar?) Scalar uses `as u32` which is truncation.
                // cvttps truncates.
                let r_i = _mm256_cvttps_epi32(r_255);
                let g_i = _mm256_cvttps_epi32(g_255);
                let b_i = _mm256_cvttps_epi32(b_255);

                // Pack: 0xFF000000 | (r << 16) | (g << 8) | b
                let pixel_val = _mm256_or_si256(
                    alpha_mask,
                    _mm256_or_si256(
                        _mm256_slli_epi32(r_i, 16),
                        _mm256_or_si256(_mm256_slli_epi32(g_i, 8), b_i),
                    ),
                );

                // Store pixels
            let fb_ptr = fb_slice.as_mut_ptr().add(i).cast::<__m256i>();
                let old_color = _mm256_loadu_si256(fb_ptr);
                // blendv_epi8 blends based on the high bit of each byte.
                // Our mask is 32-bit 0xFFFFFFFF or 0x00000000, so it works for bytes too.
                let new_color = _mm256_blendv_epi8(old_color, pixel_val, mask_int);
                _mm256_storeu_si256(fb_ptr, new_color);
            }

            // Advance
            z_vec = _mm256_add_ps(z_vec, dz_step);
            nx_vec = _mm256_add_ps(nx_vec, dnx_step);
            ny_vec = _mm256_add_ps(ny_vec, dny_step);
            nz_vec = _mm256_add_ps(nz_vec, dnz_step);

            i += 8;
        }

        // Scalar tail loop
        while i < len {
            // We need to extract current scalar values from vector state or recompute?
            // Recomputing is safer/easier than extraction.
            // Or simply maintain scalar counters parallel to vector?
            // But vector state is already advanced.

            // Let's just recompute for the tail from the current `i`.
            // x_current = x_start + i
            // value = start + gradient * i

            let i_f = i as f32;
            let z = z + i_f * gradients.dz_dx;
            let nx = nx + i_f * gradients.dnx_dx;
            let ny = ny + i_f * gradients.dny_dx;
            let nz = nz + i_f * gradients.dnz_dx;

            let pixel = &mut fb_slice[i];
            let depth_val = &mut zb_slice[i];

            if z < *depth_val {
                *depth_val = z;

                let len_sq = nx * nx + ny * ny + nz * nz;
                let dot_unorm = nx * neg_light_dir.x + ny * neg_light_dir.y + nz * neg_light_dir.z;

                let intensity = if len_sq > 0.0001 {
                    let inv_len = fast_inv_sqrt(len_sq);
                    (dot_unorm * inv_len).max(0.0)
                } else {
                    0.0
                };

                let diffuse = pre_diffuse_255 * intensity;
                let final_color_vec = ambient_255 + diffuse;
                *pixel = color_to_u32_scaled(final_color_vec);
            }

            i += 1;
        }
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_scanline_phong(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    start: PhongSpanStart,
    gradients: &PhongGradients,
    pre_diffuse_255: Vec3, // Pre-scaled
    neg_light_dir: Vec3,
    ambient_255: Vec3, // Pre-scaled
) {
    let width = fb.width() as i32;
    let Some((xs, xe, skip)) = clip_span(width, x_start, x_end) else {
        return;
    };

    let mut z = start.z + skip * gradients.dz_dx;
    let mut nx = start.nx + skip * gradients.dnx_dx;
    let mut ny = start.ny + skip * gradients.dny_dx;
    let mut nz = start.nz + skip * gradients.dnz_dx;

    // SAFETY: clip_span ensures xs, xe are within bounds.
    let (fb_slice, zb_slice) = unsafe { get_scanline_slices(fb, zb, y, xs, xe) };

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    if is_x86_feature_detected!("avx2") {
        unsafe {
            draw_scanline_phong_simd(
                fb_slice,
                zb_slice,
                z,
                nx,
                ny,
                nz,
                gradients,
                pre_diffuse_255,
                neg_light_dir,
                ambient_255,
            );
        }
        return;
    }

    for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
        if z < *depth_val {
            *depth_val = z;

            // Optimization: Deferred Normalization.
            // Instead of constructing a Vec3 and calling fast_normalize() (which does len_sq, inv_sqrt, and 3 muls),
            // we compute len_sq and the unnormalized dot product first.
            // intensity = dot(N_norm, L) = dot(N / |N|, L) = dot(N, L) / |N| = dot(N, L) * fast_inv_sqrt(|N|^2)
            // This saves 2 multiplications per pixel and avoids Vec3 construction overhead.
            let len_sq = nx * nx + ny * ny + nz * nz;
            let dot_unorm = nx * neg_light_dir.x + ny * neg_light_dir.y + nz * neg_light_dir.z;

            let intensity = if len_sq > 0.0001 {
                let inv_len = fast_inv_sqrt(len_sq);
                (dot_unorm * inv_len).max(0.0)
            } else {
                0.0
            };

            let diffuse = pre_diffuse_255 * intensity;
            let final_color_vec = ambient_255 + diffuse;
            *pixel = color_to_u32_scaled(final_color_vec);
        }

        z += gradients.dz_dx;
        nx += gradients.dnx_dx;
        ny += gradients.dny_dx;
        nz += gradients.dnz_dx;
    }
}

/// Fill a 3D triangle with Phong Shading (per-pixel lighting).
///
/// This function interpolates the normal vector across the triangle surface
/// and computes the Blinn-Phong lighting equation at every pixel.
/// It produces much smoother highlights than Gouraud shading but is more computationally expensive.
///
/// # Arguments
///
/// *   `v0`, `v1`, `v2` - Vertices defined as `((Position, W), Normal)`.
///     *   `Normal`: Surface normal vector at the vertex.
/// *   `color`: Base diffuse color of the material.
/// *   `light_dir`: Direction *to* the light source (normalized).
/// *   `light_color`: Color/Intensity of the light.
/// *   `ambient`: Ambient light color.
///
/// # Examples
///
/// ```
/// use abrash::rasterizer::fill_triangle_phong;
/// use abrash::framebuffer::Framebuffer;
/// use abrash::zbuffer::ZBuffer;
/// use abrash::math::Vec3;
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// let mut zb = ZBuffer::new(100, 100).unwrap();
///
/// // Vertices with normals
/// let v0 = ((Vec3::new(0.0, 0.5, 5.0), 5.0), Vec3::new(0.0, 0.0, 1.0));
/// let v1 = ((Vec3::new(-0.5, -0.5, 5.0), 5.0), Vec3::new(0.0, 0.0, 1.0));
/// let v2 = ((Vec3::new(0.5, -0.5, 5.0), 5.0), Vec3::new(0.0, 0.0, 1.0));
///
/// let light_dir = Vec3::new(0.0, 0.0, -1.0).normalize();
///
/// fill_triangle_phong(
///     &mut fb, &mut zb,
///     v0, v1, v2,
///     Vec3::new(1.0, 0.0, 0.0), // Red material
///     light_dir,
///     Vec3::new(1.0, 1.0, 1.0), // White light
///     Vec3::new(0.1, 0.1, 0.1), // Dim ambient
/// );
/// ```
#[allow(clippy::too_many_arguments)]
pub fn fill_triangle_phong(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec3),
    v1: ((Vec3, f32), Vec3),
    v2: ((Vec3, f32), Vec3),
    color: Vec3,
    light_dir: Vec3,
    light_color: Vec3,
    ambient: Vec3,
) {
    assert_same_dimensions(fb, zb);

    let clipped = clip_triangle_to_frustum(v0, v1, v2, |v| v.0);

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
        let p0_orig = project_to_screen_optimized(v0.0.0, v0.0.1, half_width, half_height);
        let p1_orig = project_to_screen_optimized(v1.0.0, v1.0.1, half_width, half_height);
        let p2_orig = project_to_screen_optimized(v2.0.0, v2.0.1, half_width, half_height);

        // Backface Culling
        if is_backface(p0_orig, p1_orig, p2_orig) {
            continue;
        }

        // Prepare attributes: q=1/w, n/w
        let inv_w0 = p0_orig.inv_w;
        let inv_w1 = p1_orig.inv_w;
        let inv_w2 = p2_orig.inv_w;

        let n0 = v0.1 * inv_w0;
        let n1 = v1.1 * inv_w1;
        let n2 = v2.1 * inv_w2;

        let mut verts = [(p0_orig, n0), (p1_orig, n1), (p2_orig, n2)];
        sort_by_y(&mut verts, |(p, _)| p.y);
        let [(p0, n0), (p1, n1), (p2, n2)] = verts;

        let total_height = (i64::from(p2.y) - i64::from(p0.y)) as f32;
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
            let g = PhongGradients::new(p0, p1, p2, n0, n1, n2);
            let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
            let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
            let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
            let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
            let left = ux * vy - uy * vx > 0.0;
            (g, left)
        };

        let mut edge_a = PhongEdgeWalker::new(p0, p2, n0, n2);
        if y_start > p0.y {
            edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
        }

        let mut edge_b = if y_start < p1.y {
            let mut e = PhongEdgeWalker::new(p0, p1, n0, n1);
            if y_start > p0.y {
                e.step_n(i64::from(y_start) - i64::from(p0.y));
            }
            e
        } else {
            let mut e = PhongEdgeWalker::new(p1, p2, n1, n2);
            if y_start > p1.y {
                e.step_n(i64::from(y_start) - i64::from(p1.y));
            }
            e
        };

        // Precalculate lighting constants
        // Optimization: Pre-scale by 255.0 to avoid per-pixel multiplication
        let pre_diffuse_255 = color * light_color * 255.0;
        let neg_light_dir = light_dir * -1.0;
        let ambient_255 = ambient * 255.0;

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b = PhongEdgeWalker::new(p1, p2, n1, n2);
            }

            let (x_start, x_end, z_left, nx_left, ny_left, nz_left) = if long_edge_is_left {
                (
                    (edge_a.x >> 16) as i32,
                    (edge_b.x >> 16) as i32,
                    edge_a.z,
                    edge_a.nx,
                    edge_a.ny,
                    edge_a.nz,
                )
            } else {
                (
                    (edge_b.x >> 16) as i32,
                    (edge_a.x >> 16) as i32,
                    edge_b.z,
                    edge_b.nx,
                    edge_b.ny,
                    edge_b.nz,
                )
            };

            let dx = i64::from(x_end) - i64::from(x_start);

            if dx > 0 {
                draw_scanline_phong(
                    fb,
                    zb,
                    y,
                    x_start,
                    x_end,
                    PhongSpanStart {
                        z: z_left,
                        nx: nx_left,
                        ny: ny_left,
                        nz: nz_left,
                    },
                    &gradients,
                    pre_diffuse_255,
                    neg_light_dir,
                    ambient_255,
                );
            }

            edge_a.step();
            edge_b.step();
        }
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2")]
#[allow(clippy::too_many_arguments)]
unsafe fn draw_scanline_normal_mapped_simd(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    z: f32,
    q: f32,
    u: f32,
    v: f32,
    lx: f32,
    ly: f32,
    lz: f32,
    gradients: &NormalMapGradients,
    texture: &Texture,
    normal_map: &Texture,
    pre_diffuse_color: Vec3,
    ambient: Vec3,
) {
    unsafe {
        use std::arch::x86_64::*;

        let len = fb_slice.len();
        let mut i = 0;

        // Load gradients
        let dz_dx = _mm256_set1_ps(gradients.dz_dx);
        let dq_dx = _mm256_set1_ps(gradients.dq_dx);
        let du_dx = _mm256_set1_ps(gradients.du_dx);
        let dv_dx = _mm256_set1_ps(gradients.dv_dx);
        let dlx_dx = _mm256_set1_ps(gradients.dlx_dx);
        let dly_dx = _mm256_set1_ps(gradients.dly_dx);
        let dlz_dx = _mm256_set1_ps(gradients.dlz_dx);

        let offsets = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);

        let mut z_vec = _mm256_add_ps(_mm256_set1_ps(z), _mm256_mul_ps(dz_dx, offsets));
        let mut q_vec = _mm256_add_ps(_mm256_set1_ps(q), _mm256_mul_ps(dq_dx, offsets));
        let mut u_vec = _mm256_add_ps(_mm256_set1_ps(u), _mm256_mul_ps(du_dx, offsets));
        let mut v_vec = _mm256_add_ps(_mm256_set1_ps(v), _mm256_mul_ps(dv_dx, offsets));
        let mut lx_vec = _mm256_add_ps(_mm256_set1_ps(lx), _mm256_mul_ps(dlx_dx, offsets));
        let mut ly_vec = _mm256_add_ps(_mm256_set1_ps(ly), _mm256_mul_ps(dly_dx, offsets));
        let mut lz_vec = _mm256_add_ps(_mm256_set1_ps(lz), _mm256_mul_ps(dlz_dx, offsets));

        let step_8 = _mm256_set1_ps(8.0);
        let dz_step = _mm256_mul_ps(dz_dx, step_8);
        let dq_step = _mm256_mul_ps(dq_dx, step_8);
        let du_step = _mm256_mul_ps(du_dx, step_8);
        let dv_step = _mm256_mul_ps(dv_dx, step_8);
        let dlx_step = _mm256_mul_ps(dlx_dx, step_8);
        let dly_step = _mm256_mul_ps(dly_dx, step_8);
        let dlz_step = _mm256_mul_ps(dlz_dx, step_8);

        let one = _mm256_set1_ps(1.0);
        let epsilon = _mm256_set1_ps(0.0001);
        let scale_255 = _mm256_set1_ps(255.0);
        let inv_255 = _mm256_set1_ps(1.0 / 255.0);
        let two = _mm256_set1_ps(2.0);
        let zero = _mm256_setzero_ps();
        let one_point_five = _mm256_set1_ps(1.5);
        let zero_point_five = _mm256_set1_ps(0.5);
        let _neg_one = _mm256_set1_ps(-1.0);
        let alpha_mask = _mm256_set1_epi32(0xFF00_0000u32 as i32);

        let diff_r_const = _mm256_set1_ps(pre_diffuse_color.x);
        let diff_g_const = _mm256_set1_ps(pre_diffuse_color.y);
        let diff_b_const = _mm256_set1_ps(pre_diffuse_color.z);

        let amb_r = _mm256_set1_ps(ambient.x);
        let amb_g = _mm256_set1_ps(ambient.y);
        let amb_b = _mm256_set1_ps(ambient.z);

        let scale_nm = _mm256_mul_ps(two, inv_255);

        while i + 8 <= len {
            let depth_ptr = zb_slice.as_mut_ptr().add(i);
            let depth_val = _mm256_loadu_ps(depth_ptr);
            let mask = _mm256_cmp_ps(z_vec, depth_val, _CMP_LT_OQ);

            if _mm256_movemask_ps(mask) != 0 {
                // Update Z
                let old_z = _mm256_loadu_ps(depth_ptr);
                let new_z = _mm256_blendv_ps(old_z, z_vec, mask);
                _mm256_storeu_ps(depth_ptr, new_z);

                // Perspective recover
                let q_abs = _mm256_andnot_ps(_mm256_set1_ps(-0.0), q_vec); // abs
                let q_valid = _mm256_cmp_ps(q_abs, _mm256_set1_ps(1e-6), _CMP_GT_OQ);
                let safe_q = _mm256_blendv_ps(one, q_vec, q_valid);
                let w_recip = _mm256_div_ps(one, safe_q);

                let u_tex_f = _mm256_mul_ps(u_vec, w_recip);
                let v_tex_f = _mm256_mul_ps(v_vec, w_recip);

                // Convert to i32 for gathering
                let u_i = _mm256_cvttps_epi32(u_tex_f);
                let v_i = _mm256_cvttps_epi32(v_tex_f);

                // Calculate texture indices (Vectorized)
                // Optimization: Specialized path for Power-of-Two textures using bitwise masking
                let idx = if texture.width_shift < 32 {
                    let mask_x = _mm256_set1_epi32((texture.width - 1) as i32);
                    let mask_y = _mm256_set1_epi32((texture.height - 1) as i32);
                    let shift_vec = _mm256_set1_epi32(texture.width_shift as i32);

                    // wrap: u & (w-1)
                    let u_masked = _mm256_and_si256(u_i, mask_x);
                    let v_masked = _mm256_and_si256(v_i, mask_y);

                    // idx = (v << shift) | u
                    _mm256_or_si256(_mm256_sllv_epi32(v_masked, shift_vec), u_masked)
                } else {
                    // Generic path with clamping
                    let w_vec = _mm256_set1_epi32(texture.width as i32);
                    let max_x = _mm256_set1_epi32((texture.width - 1) as i32);
                    let max_y = _mm256_set1_epi32((texture.height - 1) as i32);
                    let zero_i = _mm256_setzero_si256();

                    // clamp(val, 0, max)
                    let u_clamped = _mm256_min_epi32(_mm256_max_epi32(u_i, zero_i), max_x);
                    let v_clamped = _mm256_min_epi32(_mm256_max_epi32(v_i, zero_i), max_y);

                    // idx = v * w + u
                    _mm256_add_epi32(_mm256_mullo_epi32(v_clamped, w_vec), u_clamped)
                };

                // Gather Diffuse
                let diff_base = texture.pixels.as_ptr() as *const i32;
                let diff_packed = _mm256_i32gather_epi32(diff_base, idx, 4);

                // Gather Normal Map
                // Optimization: If normal map has same dimensions (very common), reuse indices
                let nm_packed =
                    if normal_map.width == texture.width && normal_map.height == texture.height {
                        let nm_base = normal_map.pixels.as_ptr() as *const i32;
                        _mm256_i32gather_epi32(nm_base, idx, 4)
                    } else {
                        // Fallback: Recompute indices for normal map
                        let idx_nm = if normal_map.width_shift < 32 {
                            let mask_x = _mm256_set1_epi32((normal_map.width - 1) as i32);
                            let mask_y = _mm256_set1_epi32((normal_map.height - 1) as i32);
                            let shift_vec = _mm256_set1_epi32(normal_map.width_shift as i32);
                            let u_masked = _mm256_and_si256(u_i, mask_x);
                            let v_masked = _mm256_and_si256(v_i, mask_y);
                            _mm256_or_si256(_mm256_sllv_epi32(v_masked, shift_vec), u_masked)
                        } else {
                            let w_vec = _mm256_set1_epi32(normal_map.width as i32);
                            let max_x = _mm256_set1_epi32((normal_map.width - 1) as i32);
                            let max_y = _mm256_set1_epi32((normal_map.height - 1) as i32);
                            let zero_i = _mm256_setzero_si256();
                            let u_clamped = _mm256_min_epi32(_mm256_max_epi32(u_i, zero_i), max_x);
                            let v_clamped = _mm256_min_epi32(_mm256_max_epi32(v_i, zero_i), max_y);
                            _mm256_add_epi32(_mm256_mullo_epi32(v_clamped, w_vec), u_clamped)
                        };
                        let nm_base = normal_map.pixels.as_ptr() as *const i32;
                        _mm256_i32gather_epi32(nm_base, idx_nm, 4)
                    };

                // Unpack Diffuse (0..255 -> 0.0..1.0)
                let r_mask_i32 = _mm256_set1_epi32(0xFF);

                let diff_r_i = _mm256_and_si256(_mm256_srli_epi32(diff_packed, 16), r_mask_i32);
                let diff_g_i = _mm256_and_si256(_mm256_srli_epi32(diff_packed, 8), r_mask_i32);
                let diff_b_i = _mm256_and_si256(diff_packed, r_mask_i32);

                let diff_r_tex = _mm256_mul_ps(_mm256_cvtepi32_ps(diff_r_i), inv_255);
                let diff_g_tex = _mm256_mul_ps(_mm256_cvtepi32_ps(diff_g_i), inv_255);
                let diff_b_tex = _mm256_mul_ps(_mm256_cvtepi32_ps(diff_b_i), inv_255);

                // Unpack Normal Map (0..255 -> -1.0..1.0)
                // val * (2.0 / 255.0) - 1.0
                let nm_r_i = _mm256_and_si256(_mm256_srli_epi32(nm_packed, 16), r_mask_i32);
                let nm_g_i = _mm256_and_si256(_mm256_srli_epi32(nm_packed, 8), r_mask_i32);
                let nm_b_i = _mm256_and_si256(nm_packed, r_mask_i32);

                // fmsub(a, b, c) = a * b - c
                let nm_x = _mm256_fmsub_ps(_mm256_cvtepi32_ps(nm_r_i), scale_nm, one);
                let nm_y = _mm256_fmsub_ps(_mm256_cvtepi32_ps(nm_g_i), scale_nm, one);
                let nm_z = _mm256_fmsub_ps(_mm256_cvtepi32_ps(nm_b_i), scale_nm, one);

                // Lighting
                // len_sq = lx*lx + ly*ly + lz*lz
                let len_sq = _mm256_add_ps(
                    _mm256_mul_ps(lx_vec, lx_vec),
                    _mm256_add_ps(_mm256_mul_ps(ly_vec, ly_vec), _mm256_mul_ps(lz_vec, lz_vec)),
                );

                // rsqrt
                let len_valid = _mm256_cmp_ps(len_sq, epsilon, _CMP_GT_OQ);
                let safe_len_sq = _mm256_blendv_ps(one, len_sq, len_valid);
                let rsqrt = _mm256_rsqrt_ps(safe_len_sq);
                let iter1 = _mm256_mul_ps(safe_len_sq, _mm256_mul_ps(rsqrt, rsqrt));
                let iter2 = _mm256_sub_ps(one_point_five, _mm256_mul_ps(zero_point_five, iter1));
                let inv_len = _mm256_mul_ps(rsqrt, iter2);

                // Dot product (nm . l)
                let dot = _mm256_add_ps(
                    _mm256_mul_ps(nm_x, lx_vec),
                    _mm256_add_ps(_mm256_mul_ps(nm_y, ly_vec), _mm256_mul_ps(nm_z, lz_vec)),
                );

                let intensity = _mm256_max_ps(zero, _mm256_mul_ps(dot, inv_len));
                let intensity = _mm256_blendv_ps(zero, intensity, len_valid);

                // Combine
                // final = ambient + pre_diffuse * diff_tex * intensity
                // a + b * c * d
                let diffuse_term_r = _mm256_mul_ps(diff_r_const, diff_r_tex);
                let diffuse_term_g = _mm256_mul_ps(diff_g_const, diff_g_tex);
                let diffuse_term_b = _mm256_mul_ps(diff_b_const, diff_b_tex);

                let r_final = _mm256_fmadd_ps(diffuse_term_r, intensity, amb_r);
                let g_final = _mm256_fmadd_ps(diffuse_term_g, intensity, amb_g);
                let b_final = _mm256_fmadd_ps(diffuse_term_b, intensity, amb_b);

                // Clamp and scale to 255
                let r_clamp = _mm256_min_ps(
                    _mm256_max_ps(_mm256_mul_ps(r_final, scale_255), zero),
                    scale_255,
                );
                let g_clamp = _mm256_min_ps(
                    _mm256_max_ps(_mm256_mul_ps(g_final, scale_255), zero),
                    scale_255,
                );
                let b_clamp = _mm256_min_ps(
                    _mm256_max_ps(_mm256_mul_ps(b_final, scale_255), zero),
                    scale_255,
                );

                let r_out = _mm256_cvttps_epi32(r_clamp);
                let g_out = _mm256_cvttps_epi32(g_clamp);
                let b_out = _mm256_cvttps_epi32(b_clamp);

                let pixel_val = _mm256_or_si256(
                    alpha_mask,
                    _mm256_or_si256(
                        _mm256_slli_epi32(r_out, 16),
                        _mm256_or_si256(_mm256_slli_epi32(g_out, 8), b_out),
                    ),
                );

                let fb_ptr = fb_slice.as_mut_ptr().add(i).cast::<__m256i>();
                let old_color = _mm256_loadu_si256(fb_ptr);
                let mask_int = _mm256_castps_si256(mask);
                let new_color = _mm256_blendv_epi8(old_color, pixel_val, mask_int);
                _mm256_storeu_si256(fb_ptr, new_color);
            }

            z_vec = _mm256_add_ps(z_vec, dz_step);
            q_vec = _mm256_add_ps(q_vec, dq_step);
            u_vec = _mm256_add_ps(u_vec, du_step);
            v_vec = _mm256_add_ps(v_vec, dv_step);
            lx_vec = _mm256_add_ps(lx_vec, dlx_step);
            ly_vec = _mm256_add_ps(ly_vec, dly_step);
            lz_vec = _mm256_add_ps(lz_vec, dlz_step);

            i += 8;
        }

        // Scalar tail
        while i < len {
            let i_f = i as f32;
            let z = z + i_f * gradients.dz_dx;
            let q = q + i_f * gradients.dq_dx;
            let u = u + i_f * gradients.du_dx;
            let v = v + i_f * gradients.dv_dx;
            let lx = lx + i_f * gradients.dlx_dx;
            let ly = ly + i_f * gradients.dly_dx;
            let lz = lz + i_f * gradients.dlz_dx;

            let pixel = &mut fb_slice[i];
            let depth_val = &mut zb_slice[i];

            if z < *depth_val {
                *depth_val = z;
                let w_recip = if q.abs() > 0.000_001 { 1.0 / q } else { 1.0 };
                let u_tex = u * w_recip;
                let v_tex = v * w_recip;

                let diffuse_color_u32 = texture.get_pixel_texel(u_tex as i32, v_tex as i32);
                let diff_r = ((diffuse_color_u32 >> 16) & 0xFF) as f32 / 255.0;
                let diff_g = ((diffuse_color_u32 >> 8) & 0xFF) as f32 / 255.0;
                let diff_b = (diffuse_color_u32 & 0xFF) as f32 / 255.0;
                let diffuse_sample = Vec3::new(diff_r, diff_g, diff_b);

                let nm_color_u32 = normal_map.get_pixel_texel(u_tex as i32, v_tex as i32);
                let nm_r = (((nm_color_u32 >> 16) & 0xFF) as f32 / 255.0) * 2.0 - 1.0;
                let nm_g = (((nm_color_u32 >> 8) & 0xFF) as f32 / 255.0) * 2.0 - 1.0;
                let nm_b = ((nm_color_u32 & 0xFF) as f32 / 255.0) * 2.0 - 1.0;

                let len_sq = lx * lx + ly * ly + lz * lz;
                let intensity = if len_sq > 0.0001 {
                    let inv_len = fast_inv_sqrt(len_sq);
                    (nm_r * lx + nm_g * ly + nm_b * lz) * inv_len
                } else {
                    0.0
                }
                .max(0.0);

                let diffuse_total = pre_diffuse_color * diffuse_sample * intensity;
                let final_color_vec = ambient + diffuse_total;
                *pixel = color_to_u32(final_color_vec);
            }
            i += 1;
        }
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_scanline_normal_mapped(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    start: NormalMapSpanStart,
    gradients: &NormalMapGradients,
    texture: &Texture,
    normal_map: &Texture,
    pre_diffuse_color: Vec3, // base_color * light_color
    ambient: Vec3,
) {
    let width = fb.width() as i32;
    let Some((xs, xe, skip)) = clip_span(width, x_start, x_end) else {
        return;
    };

    // Local accumulators
    let mut z = start.z + skip * gradients.dz_dx;
    let mut q = start.q + skip * gradients.dq_dx;
    let mut u = start.u + skip * gradients.du_dx;
    let mut v = start.v + skip * gradients.dv_dx;
    let mut lx = start.lx + skip * gradients.dlx_dx;
    let mut ly = start.ly + skip * gradients.dly_dx;
    let mut lz = start.lz + skip * gradients.dlz_dx;

    // SAFETY: clip_span ensures xs, xe are within bounds.
    let (fb_slice, zb_slice) = unsafe { get_scanline_slices(fb, zb, y, xs, xe) };

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    if is_x86_feature_detected!("avx2") {
        unsafe {
            draw_scanline_normal_mapped_simd(
                fb_slice,
                zb_slice,
                z,
                q,
                u,
                v,
                lx,
                ly,
                lz,
                gradients,
                texture,
                normal_map,
                pre_diffuse_color,
                ambient,
            );
        }
        return;
    }

    for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
        if z < *depth_val {
            *depth_val = z;

            // Perspective recover
            let w_recip = if q.abs() > 0.000_001 { 1.0 / q } else { 1.0 };
            let u_tex = u * w_recip;
            let v_tex = v * w_recip;

            // Sample diffuse
            // Using Nearest for speed in this complex shader, or duplicate logic for Bilinear
            let diffuse_color_u32 = texture.get_pixel_texel(u_tex as i32, v_tex as i32);
            // Unpack diffuse to Vec3 (0-1)
            let diff_r = ((diffuse_color_u32 >> 16) & 0xFF) as f32 / 255.0;
            let diff_g = ((diffuse_color_u32 >> 8) & 0xFF) as f32 / 255.0;
            let diff_b = (diffuse_color_u32 & 0xFF) as f32 / 255.0;
            let diffuse_sample = Vec3::new(diff_r, diff_g, diff_b);

            // Sample normal map (Tangent Space Normal)
            let nm_color_u32 = normal_map.get_pixel_texel(u_tex as i32, v_tex as i32);
            // Unpack to [-1, 1]
            let nm_r = (((nm_color_u32 >> 16) & 0xFF) as f32 / 255.0) * 2.0 - 1.0;
            let nm_g = (((nm_color_u32 >> 8) & 0xFF) as f32 / 255.0) * 2.0 - 1.0;
            let nm_b = ((nm_color_u32 & 0xFF) as f32 / 255.0) * 2.0 - 1.0;
            // let tangent_normal = Vec3::new(nm_r, nm_g, nm_b);

            // Light Vector in Tangent Space
            // Interpolated L is (L_true / w).
            // normalize(L_true / w) points in same direction as L_true if w > 0.
            // So we don't need explicit perspective recovery (division by q) for the direction.
            let len_sq = lx * lx + ly * ly + lz * lz;
            let intensity = if len_sq > 0.0001 {
                let inv_len = fast_inv_sqrt(len_sq);
                // Dot product: normal . light
                (nm_r * lx + nm_g * ly + nm_b * lz) * inv_len
            } else {
                0.0
            }
            .max(0.0);

            // Combine
            let diffuse_total = pre_diffuse_color * diffuse_sample * intensity;
            let final_color_vec = ambient + diffuse_total;
            *pixel = color_to_u32(final_color_vec);
        }

        z += gradients.dz_dx;
        q += gradients.dq_dx;
        u += gradients.du_dx;
        v += gradients.dv_dx;
        lx += gradients.dlx_dx;
        ly += gradients.dly_dx;
        lz += gradients.dlz_dx;
    }
}

/// Fill a 3D triangle with Normal Mapping.
///
/// This function performs per-pixel tangent-space normal mapping.
///
/// # Arguments
///
/// * `v0`, `v1`, `v2` - Vertices defined as `((Position, W), UV, Normal, Tangent)`.
///   - `Tangent`: `Vec4` where xyz is the tangent vector and w is the handedness (+1/-1).
#[allow(clippy::too_many_arguments)]
pub fn fill_triangle_normal_mapped(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec2, Vec3, Vec4),
    v1: ((Vec3, f32), Vec2, Vec3, Vec4),
    v2: ((Vec3, f32), Vec2, Vec3, Vec4),
    texture: &Texture,
    normal_map: &Texture,
    light_dir: Vec3,
    light_color: Vec3,
    ambient: Vec3,
) {
    assert_same_dimensions(fb, zb);

    let clipped = clip_triangle_to_frustum(v0, v1, v2, |v| v.0);

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
        let p0_orig = project_to_screen_optimized(v0.0.0, v0.0.1, half_width, half_height);
        let p1_orig = project_to_screen_optimized(v1.0.0, v1.0.1, half_width, half_height);
        let p2_orig = project_to_screen_optimized(v2.0.0, v2.0.1, half_width, half_height);

        // Backface Culling
        if is_backface(p0_orig, p1_orig, p2_orig) {
            continue;
        }

        // Prepare attributes
        let inv_w0 = p0_orig.inv_w;
        let inv_w1 = p1_orig.inv_w;
        let inv_w2 = p2_orig.inv_w;

        // Scale UV by texture size (assuming both maps match size or using one size for ratio)
        // Usually, UVs are 0..1, we multiply by size to get texel coords
        let w = texture.width as f32;
        let h = texture.height as f32;

        let u0 = v0.1.x * w * inv_w0;
        let v0_val = v0.1.y * h * inv_w0;
        let u1 = v1.1.x * w * inv_w1;
        let v1_val = v1.1.y * h * inv_w1;
        let u2 = v2.1.x * w * inv_w2;
        let v2_val = v2.1.y * h * inv_w2;

        // Compute Tangent Space Light Vectors
        let calculate_ts_light = |n: Vec3, t: Vec4| -> Vec3 {
            let n_norm = n.normalize();
            let t_norm = Vec3::new(t.x, t.y, t.z).normalize();
            // Re-orthogonalize T with respect to N (Gram-Schmidt)
            let t_ortho = (t_norm - n_norm * n_norm.dot(t_norm)).normalize();
            let b_ortho = n_norm.cross(t_ortho) * t.w;

            // Transform LightDir to Tangent Space.
            // LightDir passed in is direction of light (sun).
            // We want vector TO light, so -light_dir.
            let l_world = light_dir * -1.0;

            // TS_L = TBN^T * L_world
            Vec3::new(
                t_ortho.dot(l_world),
                b_ortho.dot(l_world),
                n_norm.dot(l_world),
            )
        };

        // Use true normals/tangents (v0.2, v0.3) not scaled by inv_w
        let l0_ts = calculate_ts_light(v0.2, v0.3);
        let l1_ts = calculate_ts_light(v1.2, v1.3);
        let l2_ts = calculate_ts_light(v2.2, v2.3);

        // Prepare for interpolation
        let l0 = l0_ts * inv_w0;
        let l1 = l1_ts * inv_w1;
        let l2 = l2_ts * inv_w2;

        let mut verts = [
            (p0_orig, u0, v0_val, l0),
            (p1_orig, u1, v1_val, l1),
            (p2_orig, u2, v2_val, l2),
        ];
        sort_by_y(&mut verts, |(p, ..)| p.y);
        let [(p0, u0, v0_v, l0), (p1, u1, v1_v, l1), (p2, u2, v2_v, l2)] = verts;

        let q0 = p0.inv_w;
        let q1 = p1.inv_w;
        let q2 = p2.inv_w;

        let total_height = (i64::from(p2.y) - i64::from(p0.y)) as f32;
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
            let g = NormalMapGradients::new(
                p0, p1, p2, q0, q1, q2, u0, u1, u2, v0_v, v1_v, v2_v, l0, l1, l2,
            );
            let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
            let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
            let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
            let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
            let left = ux * vy - uy * vx > 0.0;
            (g, left)
        };

        let mut edge_a = NormalMapEdgeWalker::new(p0, p2, q0, q2, u0, u2, v0_v, v2_v, l0, l2);
        if y_start > p0.y {
            edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
        }

        let mut edge_b = if y_start < p1.y {
            let mut e = NormalMapEdgeWalker::new(p0, p1, q0, q1, u0, u1, v0_v, v1_v, l0, l1);
            if y_start > p0.y {
                e.step_n(i64::from(y_start) - i64::from(p0.y));
            }
            e
        } else {
            let mut e = NormalMapEdgeWalker::new(p1, p2, q1, q2, u1, u2, v1_v, v2_v, l1, l2);
            if y_start > p1.y {
                e.step_n(i64::from(y_start) - i64::from(p1.y));
            }
            e
        };

        let pre_diffuse_color = light_color; // Base color comes from texture

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b = NormalMapEdgeWalker::new(p1, p2, q1, q2, u1, u2, v1_v, v2_v, l1, l2);
            }

            // Unpack walker state
            let (x_start, x_end, z_left, q_left, u_left, v_left, lx_left, ly_left, lz_left) =
                if long_edge_is_left {
                    (
                        (edge_a.x >> 16) as i32,
                        (edge_b.x >> 16) as i32,
                        edge_a.z,
                        edge_a.q,
                        edge_a.u,
                        edge_a.v,
                        edge_a.lx,
                        edge_a.ly,
                        edge_a.lz,
                    )
                } else {
                    (
                        (edge_b.x >> 16) as i32,
                        (edge_a.x >> 16) as i32,
                        edge_b.z,
                        edge_b.q,
                        edge_b.u,
                        edge_b.v,
                        edge_b.lx,
                        edge_b.ly,
                        edge_b.lz,
                    )
                };

            let dx = i64::from(x_end) - i64::from(x_start);

            if dx > 0 {
                draw_scanline_normal_mapped(
                    fb,
                    zb,
                    y,
                    x_start,
                    x_end,
                    NormalMapSpanStart {
                        z: z_left,
                        q: q_left,
                        u: u_left,
                        v: v_left,
                        lx: lx_left,
                        ly: ly_left,
                        lz: lz_left,
                    },
                    &gradients,
                    texture,
                    normal_map,
                    pre_diffuse_color,
                    ambient,
                );
            }

            edge_a.step();
            edge_b.step();
        }
    }
}

/// Draw a 3D line with Z-buffering.
///
/// Handles frustum clipping and perspective projection.
///
/// # Arguments
///
/// * `v0`, `v1` - Vertices defined as `(Position, W)`.
pub fn draw_line_3d(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: (Vec3, f32),
    v1: (Vec3, f32),
    color: u32,
) {
    // Clip against frustum (returns None if fully culled)
    if let Some((v0_clipped, v1_clipped)) = clip_line_to_frustum(v0, v1, |v| *v) {
        let width = fb.width();
        let height = fb.height();
        let half_width = width as f32 * 0.5;
        let half_height = height as f32 * 0.5;

        // Project to screen
        let p0 = project_to_screen_optimized(v0_clipped.0, v0_clipped.1, half_width, half_height);
        let p1 = project_to_screen_optimized(v1_clipped.0, v1_clipped.1, half_width, half_height);

        // Bresenham's algorithm with Z interpolation
        // Standard integer-based line drawing
        let mut x0 = p0.x;
        let mut y0 = p0.y;
        let x1 = p1.x;
        let y1 = p1.y;

        // Z-interpolation (linear in screen space for simplicity/speed, though technically 1/z is linear)
        // For wireframes, linear Z is usually acceptable.
        let mut z = p0.z;
        let z_end = p1.z;

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        // Calculate step size for Z interpolation
        // Total steps = max(|dx|, |dy|)
        let steps = dx.max(-dy);
        let dz = if steps > 0 {
            (z_end - z) / (steps as f32)
        } else {
            0.0
        };

        loop {
            // Check bounds (clipping should handle most cases, but guard against precision issues)
            if x0 >= 0 && x0 < width as i32 && y0 >= 0 && y0 < height as i32 {
                // Z-test
                // SAFETY: Bounds checked.
                unsafe {
                    let idx = (y0 as usize) * (width as usize) + (x0 as usize);
                    let z_buffer_val = zb.as_mut_slice().get_unchecked_mut(idx);
                    // Use standard depth test (less is closer for negative Z, wait.
                    // Project to screen produces z = v.z / w.
                    // If using standard OpenGL conventions, z is in [-1, 1].
                    // But rasterizer uses z < *depth_val.
                    // Let's assume standard behavior.
                    if z < *z_buffer_val {
                        *z_buffer_val = z;
                        fb.set_pixel_unchecked(x0 as usize, y0 as usize, color);
                    }
                }
            }

            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
            z += dz;
        }
    }
}

/// Fill a 3D triangle in wireframe mode.
///
/// Draws the three edges of the triangle as lines.
pub fn fill_triangle_wireframe(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: (Vec3, f32),
    v1: (Vec3, f32),
    v2: (Vec3, f32),
    color: u32,
) {
    draw_line_3d(fb, zb, v0, v1, color);
    draw_line_3d(fb, zb, v1, v2, color);
    draw_line_3d(fb, zb, v2, v0, color);
}

/// Helper to iterate along the edge of a triangle in screen space.
///
/// This struct manages the state for walking down a triangle edge, interpolating
/// X and Z coordinates. It uses fixed-point arithmetic for X to ensure
/// pixel-perfect rasterization consistency.
pub(crate) struct EdgeWalker {
    /// Current X coordinate in 16.16 fixed-point format.
    ///
    /// The upper 16 bits represent the integer pixel coordinate.
    /// The lower 16 bits represent sub-pixel precision.
    pub(crate) x: i64,
    /// Change in X per scanline (dx/dy) in 16.16 fixed-point format.
    dx_dy: i64,
    /// Current Z depth.
    pub(crate) z: f32,
    /// Change in Z per scanline (dz/dy).
    dz_dy: f32,
}

impl EdgeWalker {
    pub(crate) fn new(p_start: ScreenPoint, p_end: ScreenPoint) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let (dx_dy, dz_dy) = if height == 0.0 {
            (0, 0.0)
        } else {
            let inv_h = 1.0 / height;

            (
                ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64,
                (p_end.z - p_start.z) * inv_h,
            )
        };

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            dx_dy,
            dz_dy,
        }
    }

    pub(crate) fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
    }

    pub(crate) fn step_n(&mut self, n: i64) {
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * (n as f32);
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
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;
        let uc = c1 - c0;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
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
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
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
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let (dx_dy, dz_dy, dc_dy) = if height == 0.0 {
            (0, 0.0, (0, 0, 0))
        } else {
            let inv_h = 1.0 / height;
            let dc = (c_end - c_start) * inv_h;
            (
                ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64,
                (p_end.z - p_start.z) * inv_h,
                (
                    (dc.x * FIXED_SCALE) as i64,
                    (dc.y * FIXED_SCALE) as i64,
                    (dc.z * FIXED_SCALE) as i64,
                ),
            )
        };

        let c_fixed = (
            (c_start.x * FIXED_SCALE) as i64,
            (c_start.y * FIXED_SCALE) as i64,
            (c_start.z * FIXED_SCALE) as i64,
        );

        Self {
            x: i64::from(p_start.x) << 16,
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

    fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * n_f;
        self.c.0 = self.c.0.wrapping_add(self.dc_dy.0.wrapping_mul(n));
        self.c.1 = self.c.1.wrapping_add(self.dc_dy.1.wrapping_mul(n));
        self.c.2 = self.c.2.wrapping_add(self.dc_dy.2.wrapping_mul(n));
    }
}

#[derive(Clone, Copy)]
pub struct PerspectiveTextureGradients {
    pub dz_dx: f32,
    pub dq_dx: f32,
    pub du_dx: f32,
    pub dv_dx: f32,
    pub dq_dy: f32,
    pub du_dy: f32,
    pub dv_dy: f32,
}

impl PerspectiveTextureGradients {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
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
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;
        let uq = q1 - q0;
        let uu = u1 - u0;
        let uv = v1 - v0;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
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

        // Calculate Y gradients
        let ny_q = uq * vx - ux * vq;
        let dq_dy = ny_q * inv_nz;

        let ny_u = uu * vx - ux * vu;
        let du_dy = ny_u * inv_nz;

        let ny_v = uv * vx - ux * vv;
        let dv_dy = ny_v * inv_nz;

        Self {
            dz_dx,
            dq_dx,
            du_dx,
            dv_dx,
            dq_dy,
            du_dy,
            dv_dy,
        }
    }
}

pub(crate) struct PerspectiveTextureEdgeWalker {
    pub(crate) x: i64,
    pub(crate) z: f32,
    pub(crate) q: f32, // 1/w
    pub(crate) u: f32, // u/w
    pub(crate) v: f32, // v/w
    dx_dy: i64,
    dz_dy: f32,
    dq_dy: f32,
    du_dy: f32,
    dv_dy: f32,
}

impl PerspectiveTextureEdgeWalker {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        p_start: ScreenPoint,
        p_end: ScreenPoint,
        q_start: f32,
        q_end: f32,
        u_start: f32,
        u_end: f32,
        v_start: f32,
        v_end: f32,
    ) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let inv_h = if height == 0.0 { 0.0 } else { 1.0 / height };

        let dx_dy =
            ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64;
        let dz_dy = (p_end.z - p_start.z) * inv_h;
        let dq_dy = (q_end - q_start) * inv_h;
        let du_dy = (u_end - u_start) * inv_h;
        let dv_dy = (v_end - v_start) * inv_h;

        Self {
            x: i64::from(p_start.x) << 16,
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

    pub(crate) fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.q += self.dq_dy;
        self.u += self.du_dy;
        self.v += self.dv_dy;
    }

    pub(crate) fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * n_f;
        self.q += self.dq_dy * n_f;
        self.u += self.du_dy * n_f;
        self.v += self.dv_dy * n_f;
    }
}

#[derive(Clone, Copy)]
pub(crate) struct PerspectiveSpanStart {
    pub(crate) z: f32,
    pub(crate) q: f32,
    pub(crate) u: f32,
    pub(crate) v: f32,
}

pub(crate) const RECIPROCAL_TABLE: [f32; 17] = [
    0.0,
    1.0,
    0.5,
    0.333_333_34,
    0.25,
    0.2,
    0.166_666_67,
    0.142_857_15,
    0.125,
    0.111_111_11,
    0.1,
    0.090_909_09,
    0.083_333_336,
    0.076_923_08,
    0.071_428_575,
    0.066_666_67,
    0.062_5,
];

#[derive(Clone, Copy)]
struct PhongGradients {
    dz_dx: f32,
    dnx_dx: f32, // d(nx/w)/dx
    dny_dx: f32,
    dnz_dx: f32,
}

impl PhongGradients {
    #[allow(clippy::too_many_arguments)]
    fn new(
        p0: ScreenPoint,
        p1: ScreenPoint,
        p2: ScreenPoint,
        n0: Vec3,
        n1: Vec3,
        n2: Vec3,
    ) -> Self {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;
        let unx = n1.x - n0.x;
        let uny = n1.y - n0.y;
        let unz = n1.z - n0.z;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        let vz = p2.z - p0.z;
        let vnx = n2.x - n0.x;
        let vny = n2.y - n0.y;
        let vnz = n2.z - n0.z;

        let nz = ux * vy - uy * vx;
        let inv_nz = if nz.abs() > 0.0001 { -1.0 / nz } else { 0.0 };

        let nx_z = uy * vz - uz * vy;
        let dz_dx = nx_z * inv_nz;

        let nx_nx = uy * vnx - unx * vy;
        let dnx_dx = nx_nx * inv_nz;

        let nx_ny = uy * vny - uny * vy;
        let dny_dx = nx_ny * inv_nz;

        let nx_nz = uy * vnz - unz * vy;
        let dnz_dx = nx_nz * inv_nz;

        Self {
            dz_dx,
            dnx_dx,
            dny_dx,
            dnz_dx,
        }
    }
}

struct PhongEdgeWalker {
    x: i64,
    z: f32,
    nx: f32,
    ny: f32,
    nz: f32,
    dx_dy: i64,
    dz_dy: f32,
    dnx_dy: f32,
    dny_dy: f32,
    dnz_dy: f32,
}

impl PhongEdgeWalker {
    #[allow(clippy::too_many_arguments)]
    fn new(p_start: ScreenPoint, p_end: ScreenPoint, n_start: Vec3, n_end: Vec3) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let inv_h = if height == 0.0 { 0.0 } else { 1.0 / height };

        let dx_dy =
            ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64;
        let dz_dy = (p_end.z - p_start.z) * inv_h;
        let dnx_dy = (n_end.x - n_start.x) * inv_h;
        let dny_dy = (n_end.y - n_start.y) * inv_h;
        let dnz_dy = (n_end.z - n_start.z) * inv_h;

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            nx: n_start.x,
            ny: n_start.y,
            nz: n_start.z,
            dx_dy,
            dz_dy,
            dnx_dy,
            dny_dy,
            dnz_dy,
        }
    }

    fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.nx += self.dnx_dy;
        self.ny += self.dny_dy;
        self.nz += self.dnz_dy;
    }

    fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * n_f;
        self.nx += self.dnx_dy * n_f;
        self.ny += self.dny_dy * n_f;
        self.nz += self.dnz_dy * n_f;
    }
}

struct ShadowPhongGradients {
    dz_dx: f32,
    dq_dx: f32,
    dnx_dx: f32,
    dny_dx: f32,
    dnz_dx: f32,
    dwx_dx: f32,
    dwy_dx: f32,
    dwz_dx: f32,
}

impl ShadowPhongGradients {
    #[allow(clippy::too_many_arguments)]
    fn new(
        p0: ScreenPoint,
        p1: ScreenPoint,
        p2: ScreenPoint,
        q0: f32,
        q1: f32,
        q2: f32,
        n0: Vec3,
        n1: Vec3,
        n2: Vec3,
        w0: Vec3,
        w1: Vec3,
        w2: Vec3,
    ) -> Self {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;
        let uq = q1 - q0;
        let unx = n1.x - n0.x;
        let uny = n1.y - n0.y;
        let unz = n1.z - n0.z;
        let uwx = w1.x - w0.x;
        let uwy = w1.y - w0.y;
        let uwz = w1.z - w0.z;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        let vz = p2.z - p0.z;
        let vq = q2 - q0;
        let vnx = n2.x - n0.x;
        let vny = n2.y - n0.y;
        let vnz = n2.z - n0.z;
        let vwx = w2.x - w0.x;
        let vwy = w2.y - w0.y;
        let vwz = w2.z - w0.z;

        let nz = ux * vy - uy * vx;
        let inv_nz = if nz.abs() > 0.0001 { -1.0 / nz } else { 0.0 };

        let nx_z = uy * vz - uz * vy;
        let dz_dx = nx_z * inv_nz;

        let nx_q = uy * vq - uq * vy;
        let dq_dx = nx_q * inv_nz;

        let nx_nx = uy * vnx - unx * vy;
        let dnx_dx = nx_nx * inv_nz;

        let nx_ny = uy * vny - uny * vy;
        let dny_dx = nx_ny * inv_nz;

        let nx_nz = uy * vnz - unz * vy;
        let dnz_dx = nx_nz * inv_nz;

        let nx_wx = uy * vwx - uwx * vy;
        let dwx_dx = nx_wx * inv_nz;

        let nx_wy = uy * vwy - uwy * vy;
        let dwy_dx = nx_wy * inv_nz;

        let nx_wz = uy * vwz - uwz * vy;
        let dwz_dx = nx_wz * inv_nz;

        Self {
            dz_dx,
            dq_dx,
            dnx_dx,
            dny_dx,
            dnz_dx,
            dwx_dx,
            dwy_dx,
            dwz_dx,
        }
    }
}

struct ShadowPhongEdgeWalker {
    x: i64,
    z: f32,
    q: f32,
    nx: f32,
    ny: f32,
    nz: f32,
    wx: f32,
    wy: f32,
    wz: f32,
    dx_dy: i64,
    dz_dy: f32,
    dq_dy: f32,
    dnx_dy: f32,
    dny_dy: f32,
    dnz_dy: f32,
    dwx_dy: f32,
    dwy_dy: f32,
    dwz_dy: f32,
}

impl ShadowPhongEdgeWalker {
    #[allow(clippy::too_many_arguments)]
    fn new(
        p_start: ScreenPoint,
        p_end: ScreenPoint,
        q_start: f32,
        q_end: f32,
        n_start: Vec3,
        n_end: Vec3,
        w_start: Vec3,
        w_end: Vec3,
    ) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let inv_h = if height == 0.0 { 0.0 } else { 1.0 / height };

        let dx_dy =
            ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64;
        let dz_dy = (p_end.z - p_start.z) * inv_h;
        let dq_dy = (q_end - q_start) * inv_h;
        let dnx_dy = (n_end.x - n_start.x) * inv_h;
        let dny_dy = (n_end.y - n_start.y) * inv_h;
        let dnz_dy = (n_end.z - n_start.z) * inv_h;
        let dwx_dy = (w_end.x - w_start.x) * inv_h;
        let dwy_dy = (w_end.y - w_start.y) * inv_h;
        let dwz_dy = (w_end.z - w_start.z) * inv_h;

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            q: q_start,
            nx: n_start.x,
            ny: n_start.y,
            nz: n_start.z,
            wx: w_start.x,
            wy: w_start.y,
            wz: w_start.z,
            dx_dy,
            dz_dy,
            dq_dy,
            dnx_dy,
            dny_dy,
            dnz_dy,
            dwx_dy,
            dwy_dy,
            dwz_dy,
        }
    }

    fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.q += self.dq_dy;
        self.nx += self.dnx_dy;
        self.ny += self.dny_dy;
        self.nz += self.dnz_dy;
        self.wx += self.dwx_dy;
        self.wy += self.dwy_dy;
        self.wz += self.dwz_dy;
    }

    fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * n_f;
        self.q += self.dq_dy * n_f;
        self.nx += self.dnx_dy * n_f;
        self.ny += self.dny_dy * n_f;
        self.nz += self.dnz_dy * n_f;
        self.wx += self.dwx_dy * n_f;
        self.wy += self.dwy_dy * n_f;
        self.wz += self.dwz_dy * n_f;
    }
}

#[derive(Clone, Copy)]
struct ShadowPhongSpanStart {
    z: f32,
    q: f32,
    nx: f32,
    ny: f32,
    nz: f32,
    wx: f32,
    wy: f32,
    wz: f32,
}

#[derive(Clone, Copy)]
struct PhongSpanStart {
    z: f32,
    // q unused in optimization
    nx: f32,
    ny: f32,
    nz: f32,
}

#[derive(Clone, Copy)]
struct NormalMapGradients {
    dz_dx: f32,
    dq_dx: f32,  // 1/w
    du_dx: f32,  // u/w
    dv_dx: f32,  // v/w
    dlx_dx: f32, // lx/w (Tangent Space Light X)
    dly_dx: f32,
    dlz_dx: f32,
}

impl NormalMapGradients {
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
        l0: Vec3, // Tangent Space Light Vectors (pre-scaled by q)
        l1: Vec3,
        l2: Vec3,
    ) -> Self {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;
        let uq = q1 - q0;
        let uu = u1 - u0;
        let uv = v1 - v0;
        let ulx = l1.x - l0.x;
        let uly = l1.y - l0.y;
        let ulz = l1.z - l0.z;

        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        let vz = p2.z - p0.z;
        let vq = q2 - q0;
        let vu = u2 - u0;
        let vv = v2 - v0;
        let vlx = l2.x - l0.x;
        let vly = l2.y - l0.y;
        let vlz = l2.z - l0.z;

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

        let nx_lx = uy * vlx - ulx * vy;
        let dlx_dx = nx_lx * inv_nz;

        let nx_ly = uy * vly - uly * vy;
        let dly_dx = nx_ly * inv_nz;

        let nx_lz = uy * vlz - ulz * vy;
        let dlz_dx = nx_lz * inv_nz;

        Self {
            dz_dx,
            dq_dx,
            du_dx,
            dv_dx,
            dlx_dx,
            dly_dx,
            dlz_dx,
        }
    }
}

struct NormalMapEdgeWalker {
    x: i64,
    z: f32,
    q: f32,
    u: f32,
    v: f32,
    lx: f32,
    ly: f32,
    lz: f32,
    dx_dy: i64,
    dz_dy: f32,
    dq_dy: f32,
    du_dy: f32,
    dv_dy: f32,
    dlx_dy: f32,
    dly_dy: f32,
    dlz_dy: f32,
}

impl NormalMapEdgeWalker {
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
        l_start: Vec3,
        l_end: Vec3,
    ) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let inv_h = if height == 0.0 { 0.0 } else { 1.0 / height };

        let dx_dy =
            ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64;
        let dz_dy = (p_end.z - p_start.z) * inv_h;
        let dq_dy = (q_end - q_start) * inv_h;
        let du_dy = (u_end - u_start) * inv_h;
        let dv_dy = (v_end - v_start) * inv_h;
        let dlx_dy = (l_end.x - l_start.x) * inv_h;
        let dly_dy = (l_end.y - l_start.y) * inv_h;
        let dlz_dy = (l_end.z - l_start.z) * inv_h;

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            q: q_start,
            u: u_start,
            v: v_start,
            lx: l_start.x,
            ly: l_start.y,
            lz: l_start.z,
            dx_dy,
            dz_dy,
            dq_dy,
            du_dy,
            dv_dy,
            dlx_dy,
            dly_dy,
            dlz_dy,
        }
    }

    fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.q += self.dq_dy;
        self.u += self.du_dy;
        self.v += self.dv_dy;
        self.lx += self.dlx_dy;
        self.ly += self.dly_dy;
        self.lz += self.dlz_dy;
    }

    fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * n_f;
        self.q += self.dq_dy * n_f;
        self.u += self.du_dy * n_f;
        self.v += self.dv_dy * n_f;
        self.lx += self.dlx_dy * n_f;
        self.ly += self.dly_dy * n_f;
        self.lz += self.dlz_dy * n_f;
    }
}

#[derive(Clone, Copy)]
struct NormalMapSpanStart {
    z: f32,
    q: f32,
    u: f32,
    v: f32,
    lx: f32,
    ly: f32,
    lz: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;
    use crate::math::Vec3;
    use crate::zbuffer::ZBuffer;

    #[test]
    fn edge_walker_z_interpolation() {
        // Test that EdgeWalker correctly interpolates z
        let p0 = ScreenPoint {
            x: 0,
            y: 0,
            z: 1.0,
            inv_w: 1.0,
        };
        let p1 = ScreenPoint {
            x: 100,
            y: 100,
            z: 2.0,
            inv_w: 1.0,
        };

        let walker = EdgeWalker::new(p0, p1);

        // z should be 1.0
        assert!((walker.z - 1.0).abs() < 0.0001);

        // dz_dy should be (2.0 - 1.0) / 100.0 = 0.01
        let expected_dz_dy = (2.0_f32 - 1.0_f32) / 100.0_f32;
        assert!((walker.dz_dy - expected_dz_dy).abs() < 0.0001);
    }

    #[test]
    fn edge_walker_step_accumulates_correctly() {
        // Test that stepping accumulates z correctly
        let p0 = ScreenPoint {
            x: 0,
            y: 0,
            z: 1.0,
            inv_w: 1.0,
        };
        let p1 = ScreenPoint {
            x: 100,
            y: 100,
            z: 2.0,
            inv_w: 1.0,
        };

        let mut walker = EdgeWalker::new(p0, p1);

        let initial_z = walker.z;
        let dz = walker.dz_dy;

        // Step 50 times
        walker.step_n(50);

        // After 50 steps, z should be initial + 50*dz
        let expected_z = initial_z + dz * 50.0;
        assert!((walker.z - expected_z).abs() < 0.0001);

        assert!(
            walker.z > 1.0 && walker.z < 2.0,
            "z should be interpolated between 1.0 and 2.0, got {}",
            walker.z,
        );
    }

    #[test]
    fn edge_walker_zero_height_no_panic() {
        // Test that EdgeWalker handles degenerate case (zero height)
        let p0 = ScreenPoint {
            x: 0,
            y: 100,
            z: 1.0,
            inv_w: 1.0,
        };
        let p1 = ScreenPoint {
            x: 100,
            y: 100,
            z: 2.0,
            inv_w: 1.0,
        };

        let walker = EdgeWalker::new(p0, p1);

        // Should have zero gradients
        assert_eq!(walker.dx_dy, 0);
        assert!((walker.dz_dy - 0.0).abs() < f32::EPSILON);

        // z should still be correct
        assert!((walker.z - 1.0).abs() < 0.0001);
    }

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

    #[test]
    fn test_is_backface_overflow_safe() {
        // Construct points with large coordinates that fit within i64 product.
        // Framebuffer::new limits width/height to i32::MAX, so max difference is roughly 2e9.
        // 2e9 * 2e9 = 4e18, which is < i64::MAX (9e18).

        // p0 at (0, 0)
        let p0 = ScreenPoint {
            x: 0,
            y: 0,
            z: 0.0,
            inv_w: 1.0,
        };
        // p1 at (2e9, 0)
        let p1 = ScreenPoint {
            x: 2_000_000_000,
            y: 0,
            z: 0.0,
            inv_w: 1.0,
        };
        // p2 at (0, 2e9)
        let p2 = ScreenPoint {
            x: 0,
            y: 2_000_000_000,
            z: 0.0,
            inv_w: 1.0,
        };

        // nz = 2e9 * 2e9 = 4e18. Should not panic and return true.
        let result = is_backface(p0, p1, p2);

        assert!(result);
    }
}
