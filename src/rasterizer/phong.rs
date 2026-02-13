use crate::clipping::clip_triangle_to_frustum;
use crate::framebuffer::Framebuffer;
use crate::math::{fast_inv_sqrt, project_to_screen_optimized, ScreenPoint, Vec3};
use crate::zbuffer::ZBuffer;

use super::common::{
    assert_same_dimensions, color_to_u32_scaled, is_backface, sort_by_y, FIXED_SCALE,
};

#[derive(Clone, Copy)]
pub struct PhongGradients {
    pub dz_dx: f32,
    pub dnx_dx: f32, // d(nx/w)/dx
    pub dny_dx: f32,
    pub dnz_dx: f32,
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

pub struct PhongEdgeWalker {
    pub x: i64,
    pub z: f32,
    pub nx: f32,
    pub ny: f32,
    pub nz: f32,
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

#[derive(Clone, Copy)]
pub struct PhongSpanStart {
    pub z: f32,
    // q unused in optimization
    pub nx: f32,
    pub ny: f32,
    pub nz: f32,
}

#[cfg(all(
    any(target_arch = "x86_64", target_arch = "x86"),
    target_feature = "avx2"
))]
#[target_feature(enable = "avx2")]
#[allow(clippy::too_many_arguments)]
unsafe fn draw_scanline_phong_simd(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    mut z: f32,
    mut nx: f32,
    mut ny: f32,
    mut nz: f32,
    gradients: &PhongGradients,
    pre_diffuse_255: Vec3, // Pre-scaled by 255.0
    neg_light_dir: Vec3,
    ambient_255: Vec3,     // Pre-scaled by 255.0
) {
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
            let fb_ptr = fb_slice.as_mut_ptr().add(i) as *mut __m256i;
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
        let mut z = z + i_f * gradients.dz_dx;
        let mut nx = nx + i_f * gradients.dnx_dx;
        let mut ny = ny + i_f * gradients.dny_dx;
        let mut nz = nz + i_f * gradients.dnz_dx;

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

#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub fn draw_scanline_phong(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    start: PhongSpanStart,
    gradients: &PhongGradients,
    pre_diffuse_255: Vec3, // Pre-scaled
    neg_light_dir: Vec3,
    ambient_255: Vec3,     // Pre-scaled
) {
    let width = fb.width() as i32;
    let mut xs = x_start;
    let mut xe = x_end;

    let mut z = start.z;
    let mut nx = start.nx;
    let mut ny = start.ny;
    let mut nz = start.nz;

    if xs < 0 {
        let diff = -i64::from(xs);
        let diff_f = diff as f32;
        z += diff_f * gradients.dz_dx;
        nx += diff_f * gradients.dnx_dx;
        ny += diff_f * gradients.dny_dx;
        nz += diff_f * gradients.dnz_dx;
        xs = 0;
    }

    if xe >= width {
        xe = width - 1;
    }

    if xs > xe {
        return;
    }

    let width_usize = fb.width() as usize;
    let y_offset = (y as usize) * width_usize;
    let start_idx = y_offset + (xs as usize);
    let end_idx = y_offset + (xe as usize);

    // SAFETY: Clamped above.
    let fb_slice = unsafe { fb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };
    let zb_slice = unsafe { zb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };

    #[cfg(all(
        any(target_arch = "x86_64", target_arch = "x86"),
        target_feature = "avx2"
    ))]
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
/// and computes the lighting equation at every pixel.
///
/// # Arguments
///
/// * `v0`, `v1`, `v2` - Vertices defined as `((Position, W), Normal)`.
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
