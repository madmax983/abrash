//! Screen-space reflections.
//!
//! Provides basic reflection mapping based on surface normals and view vectors.

use crate::clipping::clip_triangle_to_frustum;
use crate::framebuffer::Framebuffer;
use crate::math::{ScreenPoint, Vec3, project_triangle_to_screen};
use crate::skybox::Cubemap;
use crate::zbuffer::ZBuffer;

use super::core::{FIXED_SCALE, assert_same_dimensions, is_backface, sort_by_y};

#[derive(Clone, Copy)]
struct ReflectionSpanStart {
    z: f32,
    q: f32,
    nx: f32,
    ny: f32,
    nz: f32,
    wx: f32,
    wy: f32,
    wz: f32,
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2")]
#[allow(clippy::too_many_arguments)]
unsafe fn draw_scanline_reflection_simd(
    fb_slice: &mut [u32],
    zb_slice: &mut [f32],
    start: ReflectionSpanStart,
    gradients: &ReflectionGradients,
    camera_pos: Vec3,
    cubemap: &Cubemap,
) {
    use std::arch::x86_64::{
        _CMP_GT_OQ, _CMP_LT_OQ, _mm256_add_ps, _mm256_andnot_ps, _mm256_blendv_ps, _mm256_cmp_ps,
        _mm256_div_ps, _mm256_loadu_ps, _mm256_movemask_ps, _mm256_mul_ps, _mm256_rsqrt_ps,
        _mm256_set_ps, _mm256_set1_ps, _mm256_storeu_ps, _mm256_sub_ps,
    };

    let len = fb_slice.len();
    let mut i = 0;

    let dz_dx = _mm256_set1_ps(gradients.dz_dx);
    let dq_dx = _mm256_set1_ps(gradients.dq_dx);
    let dnx_dx = _mm256_set1_ps(gradients.dnx_dx);
    let dny_dx = _mm256_set1_ps(gradients.dny_dx);
    let dnz_dx = _mm256_set1_ps(gradients.dnz_dx);
    let dwx_dx = _mm256_set1_ps(gradients.dwx_dx);
    let dwy_dx = _mm256_set1_ps(gradients.dwy_dx);
    let dwz_dx = _mm256_set1_ps(gradients.dwz_dx);

    let offsets = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);

    let mut z_vec = _mm256_add_ps(_mm256_set1_ps(start.z), _mm256_mul_ps(dz_dx, offsets));
    let mut q_vec = _mm256_add_ps(_mm256_set1_ps(start.q), _mm256_mul_ps(dq_dx, offsets));
    let mut nx_vec = _mm256_add_ps(_mm256_set1_ps(start.nx), _mm256_mul_ps(dnx_dx, offsets));
    let mut ny_vec = _mm256_add_ps(_mm256_set1_ps(start.ny), _mm256_mul_ps(dny_dx, offsets));
    let mut nz_vec = _mm256_add_ps(_mm256_set1_ps(start.nz), _mm256_mul_ps(dnz_dx, offsets));
    let mut wx_vec = _mm256_add_ps(_mm256_set1_ps(start.wx), _mm256_mul_ps(dwx_dx, offsets));
    let mut wy_vec = _mm256_add_ps(_mm256_set1_ps(start.wy), _mm256_mul_ps(dwy_dx, offsets));
    let mut wz_vec = _mm256_add_ps(_mm256_set1_ps(start.wz), _mm256_mul_ps(dwz_dx, offsets));

    let step_8 = _mm256_set1_ps(8.0);
    let dz_step = _mm256_mul_ps(dz_dx, step_8);
    let dq_step = _mm256_mul_ps(dq_dx, step_8);
    let dnx_step = _mm256_mul_ps(dnx_dx, step_8);
    let dny_step = _mm256_mul_ps(dny_dx, step_8);
    let dnz_step = _mm256_mul_ps(dnz_dx, step_8);
    let dwx_step = _mm256_mul_ps(dwx_dx, step_8);
    let dwy_step = _mm256_mul_ps(dwy_dx, step_8);
    let dwz_step = _mm256_mul_ps(dwz_dx, step_8);

    let cam_x = _mm256_set1_ps(camera_pos.x);
    let cam_y = _mm256_set1_ps(camera_pos.y);
    let cam_z = _mm256_set1_ps(camera_pos.z);

    let one = _mm256_set1_ps(1.0);
    let two = _mm256_set1_ps(2.0);
    let epsilon = _mm256_set1_ps(0.0001);

    while i + 8 <= len {
        unsafe {
            let depth_ptr = zb_slice.as_mut_ptr().add(i);
            let depth_val = _mm256_loadu_ps(depth_ptr);
            let mask = _mm256_cmp_ps(z_vec, depth_val, _CMP_LT_OQ);

            if _mm256_movemask_ps(mask) != 0 {
                let old_z = _mm256_loadu_ps(depth_ptr);
                let new_z = _mm256_blendv_ps(old_z, z_vec, mask);
                _mm256_storeu_ps(depth_ptr, new_z);

                // Perspective recover
                let q_abs = _mm256_andnot_ps(_mm256_set1_ps(-0.0), q_vec);
                let q_valid = _mm256_cmp_ps(q_abs, epsilon, _CMP_GT_OQ);
                let safe_q = _mm256_blendv_ps(one, q_vec, q_valid);
                let w_recip = _mm256_div_ps(one, safe_q);

                let wx_real = _mm256_mul_ps(wx_vec, w_recip);
                let wy_real = _mm256_mul_ps(wy_vec, w_recip);
                let wz_real = _mm256_mul_ps(wz_vec, w_recip);

                // Incident Vector I = WorldPos - CameraPos
                let mut ix = _mm256_sub_ps(wx_real, cam_x);
                let mut iy = _mm256_sub_ps(wy_real, cam_y);
                let mut iz = _mm256_sub_ps(wz_real, cam_z);

                // Normalize I
                let i_len_sq = _mm256_add_ps(
                    _mm256_mul_ps(ix, ix),
                    _mm256_add_ps(_mm256_mul_ps(iy, iy), _mm256_mul_ps(iz, iz)),
                );
                let i_valid = _mm256_cmp_ps(i_len_sq, epsilon, _CMP_GT_OQ);
                let safe_i_len_sq = _mm256_blendv_ps(one, i_len_sq, i_valid);
                let inv_i_len = _mm256_rsqrt_ps(safe_i_len_sq);
                ix = _mm256_mul_ps(ix, inv_i_len);
                iy = _mm256_mul_ps(iy, inv_i_len);
                iz = _mm256_mul_ps(iz, inv_i_len);

                // Normalize N
                let mut n_x = nx_vec;
                let mut n_y = ny_vec;
                let mut n_z = nz_vec;
                let n_len_sq = _mm256_add_ps(
                    _mm256_mul_ps(n_x, n_x),
                    _mm256_add_ps(_mm256_mul_ps(n_y, n_y), _mm256_mul_ps(n_z, n_z)),
                );
                let n_valid = _mm256_cmp_ps(n_len_sq, epsilon, _CMP_GT_OQ);
                let safe_n_len_sq = _mm256_blendv_ps(one, n_len_sq, n_valid);
                let inv_n_len = _mm256_rsqrt_ps(safe_n_len_sq);
                n_x = _mm256_mul_ps(n_x, inv_n_len);
                n_y = _mm256_mul_ps(n_y, inv_n_len);
                n_z = _mm256_mul_ps(n_z, inv_n_len);

                // Reflect R = I - 2 * dot(N, I) * N
                let dot = _mm256_add_ps(
                    _mm256_mul_ps(n_x, ix),
                    _mm256_add_ps(_mm256_mul_ps(n_y, iy), _mm256_mul_ps(n_z, iz)),
                );
                let factor = _mm256_mul_ps(two, dot);
                let rx = _mm256_sub_ps(ix, _mm256_mul_ps(factor, n_x));
                let ry = _mm256_sub_ps(iy, _mm256_mul_ps(factor, n_y));
                let rz = _mm256_sub_ps(iz, _mm256_mul_ps(factor, n_z));

                // Extract and sample (Scalar fallback for sampling)
                // We extract values to a temporary array
                let mut rx_arr = [0.0; 8];
                let mut ry_arr = [0.0; 8];
                let mut rz_arr = [0.0; 8];
                let mut mask_arr = [0.0; 8];

                _mm256_storeu_ps(rx_arr.as_mut_ptr(), rx);
                _mm256_storeu_ps(ry_arr.as_mut_ptr(), ry);
                _mm256_storeu_ps(rz_arr.as_mut_ptr(), rz);
                _mm256_storeu_ps(mask_arr.as_mut_ptr(), mask); // Store mask as float representation

                let fb_ptr = fb_slice.as_mut_ptr().add(i);

                for k in 0..8 {
                    // Check if mask bit is set (simplistic check on float representation or use _mm256_movemask_ps)
                    // _mm256_movemask_ps returns an int with bits.
                    let bit = 1 << k;
                    if (_mm256_movemask_ps(mask) & bit) != 0 {
                        let dir = Vec3::new(rx_arr[k], ry_arr[k], rz_arr[k]);
                        let color = cubemap.sample(dir);
                        *fb_ptr.add(k) = color;
                    }
                }
            }
        }

        z_vec = _mm256_add_ps(z_vec, dz_step);
        q_vec = _mm256_add_ps(q_vec, dq_step);
        nx_vec = _mm256_add_ps(nx_vec, dnx_step);
        ny_vec = _mm256_add_ps(ny_vec, dny_step);
        nz_vec = _mm256_add_ps(nz_vec, dnz_step);
        wx_vec = _mm256_add_ps(wx_vec, dwx_step);
        wy_vec = _mm256_add_ps(wy_vec, dwy_step);
        wz_vec = _mm256_add_ps(wz_vec, dwz_step);

        i += 8;
    }

    // Scalar Tail
    while i < len {
        let i_f = i as f32;
        let z = start.z + i_f * gradients.dz_dx;
        let depth_val = &mut zb_slice[i];

        if z < *depth_val {
            *depth_val = z;
            let q = start.q + i_f * gradients.dq_dx;
            let nx = start.nx + i_f * gradients.dnx_dx;
            let ny = start.ny + i_f * gradients.dny_dx;
            let nz = start.nz + i_f * gradients.dnz_dx;
            let wx = start.wx + i_f * gradients.dwx_dx;
            let wy = start.wy + i_f * gradients.dwy_dx;
            let wz = start.wz + i_f * gradients.dwz_dx;

            let w_recip = if q.abs() > 0.000_001 { 1.0 / q } else { 1.0 };
            let world_pos = Vec3::new(wx * w_recip, wy * w_recip, wz * w_recip);

            let mut i_x = world_pos.x - camera_pos.x;
            let mut i_y = world_pos.y - camera_pos.y;
            let mut i_z = world_pos.z - camera_pos.z;

            let i_len_sq = i_x * i_x + i_y * i_y + i_z * i_z;
            if i_len_sq > 0.0001 {
                let inv_len = crate::math::fast_inv_sqrt(i_len_sq);
                i_x *= inv_len;
                i_y *= inv_len;
                i_z *= inv_len;
            }

            let mut n_x = nx;
            let mut n_y = ny;
            let mut n_z = nz;
            let n_len_sq = n_x * n_x + n_y * n_y + n_z * n_z;
            if n_len_sq > 0.0001 {
                let inv_len = crate::math::fast_inv_sqrt(n_len_sq);
                n_x *= inv_len;
                n_y *= inv_len;
                n_z *= inv_len;
            }

            let dot = n_x * i_x + n_y * i_y + n_z * i_z;
            let r_x = i_x - 2.0 * dot * n_x;
            let r_y = i_y - 2.0 * dot * n_y;
            let r_z = i_z - 2.0 * dot * n_z;

            fb_slice[i] = cubemap.sample(Vec3::new(r_x, r_y, r_z));
        }
        i += 1;
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_scanline_reflection(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    y: i32,
    x_start: i32,
    x_end: i32,
    start: ReflectionSpanStart,
    gradients: &ReflectionGradients,
    camera_pos: Vec3,
    cubemap: &Cubemap,
) {
    if y < 0 || y >= fb.height() as i32 {
        return;
    }

    let width = fb.width() as i32;
    let mut xs = x_start;
    let mut xe = x_end;

    let mut z = start.z;
    let mut q = start.q;
    let mut nx = start.nx;
    let mut ny = start.ny;
    let mut nz = start.nz;
    let mut wx = start.wx;
    let mut wy = start.wy;
    let mut wz = start.wz;

    if xs < 0 {
        let diff = -i64::from(xs);
        let diff_f = diff as f32;
        z += diff_f * gradients.dz_dx;
        q += diff_f * gradients.dq_dx;
        nx += diff_f * gradients.dnx_dx;
        ny += diff_f * gradients.dny_dx;
        nz += diff_f * gradients.dnz_dx;
        wx += diff_f * gradients.dwx_dx;
        wy += diff_f * gradients.dwy_dx;
        wz += diff_f * gradients.dwz_dx;
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
    let fb_slice = &mut fb.as_mut_slice()[start_idx..=end_idx];
    let zb_slice = &mut zb.as_mut_slice()[start_idx..=end_idx];

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    if fb_slice.len() >= 32 && is_x86_feature_detected!("avx2") {
        unsafe {
            draw_scanline_reflection_simd(
                fb_slice,
                zb_slice,
                ReflectionSpanStart {
                    z,
                    q,
                    nx,
                    ny,
                    nz,
                    wx,
                    wy,
                    wz,
                },
                gradients,
                camera_pos,
                cubemap,
            );
        }
        return;
    }

    for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
        if z < *depth_val {
            *depth_val = z;

            let w_recip = if q.abs() > 0.000_001 { 1.0 / q } else { 1.0 };

            // Recover world position
            let world_pos = Vec3::new(wx * w_recip, wy * w_recip, wz * w_recip);

            // View Vector (Camera to Surface)
            // Note: Usually View is Surface to Camera, but reflect() expects Incident vector.
            // Incident = WorldPos - CameraPos.
            let mut i_x = world_pos.x - camera_pos.x;
            let mut i_y = world_pos.y - camera_pos.y;
            let mut i_z = world_pos.z - camera_pos.z;

            // Normalize Incident
            let i_len_sq = i_x * i_x + i_y * i_y + i_z * i_z;
            if i_len_sq > 0.0001 {
                let inv_len = crate::math::fast_inv_sqrt(i_len_sq);
                i_x *= inv_len;
                i_y *= inv_len;
                i_z *= inv_len;
            }

            // Normal (interpolated, needs normalization)
            let mut n_x = nx;
            let mut n_y = ny;
            let mut n_z = nz;
            let n_len_sq = n_x * n_x + n_y * n_y + n_z * n_z;
            if n_len_sq > 0.0001 {
                let inv_len = crate::math::fast_inv_sqrt(n_len_sq);
                n_x *= inv_len;
                n_y *= inv_len;
                n_z *= inv_len;
            }

            // Reflect: R = I - 2.0 * dot(N, I) * N
            let dot = n_x * i_x + n_y * i_y + n_z * i_z;
            let r_x = i_x - 2.0 * dot * n_x;
            let r_y = i_y - 2.0 * dot * n_y;
            let r_z = i_z - 2.0 * dot * n_z;

            // Sample Cubemap
            *pixel = cubemap.sample(Vec3::new(r_x, r_y, r_z));
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

/// Fill a 3D triangle with Reflection Mapping.
///
/// Renders a triangle that reflects the environment (skybox).
///
/// # Arguments
///
/// *   `fb` - Target framebuffer.
/// *   `zb` - Target z-buffer.
/// *   `v0`, `v1`, `v2` - Vertices defined as `((ClipPos, W), Normal, WorldPos)`.
/// *   `camera_pos` - World position of the camera.
/// *   `cubemap` - Environment map.
#[allow(clippy::too_many_arguments)]
pub fn fill_triangle_reflection(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec3, Vec3), // ((ClipPos, W), Normal, WorldPos)
    v1: ((Vec3, f32), Vec3, Vec3),
    v2: ((Vec3, f32), Vec3, Vec3),
    camera_pos: Vec3,
    cubemap: &Cubemap,
) {
    assert_same_dimensions(fb, zb);

    let clipped = clip_triangle_to_frustum(
        v0,
        v1,
        v2,
        |v| v.0,
        |a, b, t| {
            (
                (a.0.0.lerp(b.0.0, t), a.0.1 + (b.0.1 - a.0.1) * t),
                a.1.lerp(b.1, t),
                a.2.lerp(b.2, t),
            )
        },
    );

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
        let (p0_orig, p1_orig, p2_orig) = project_triangle_to_screen(
            v0.0.0,
            v0.0.1,
            v1.0.0,
            v1.0.1,
            v2.0.0,
            v2.0.1,
            half_width,
            half_height,
        );

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
        let (gradients, long_edge_is_left) =
            ReflectionGradients::new(p0, p1, p2, q0, q1, q2, n0, n1, n2, w0, w1, w2);

        let mut edge_a = ReflectionEdgeWalker::new(p0, p2, q0, q2, n0, n2, w0, w2);
        if y_start > p0.y {
            edge_a.step_n(i64::from(y_start) - i64::from(p0.y));
        }

        let mut edge_b = if y_start < p1.y {
            let mut e = ReflectionEdgeWalker::new(p0, p1, q0, q1, n0, n1, w0, w1);
            if y_start > p0.y {
                e.step_n(i64::from(y_start) - i64::from(p0.y));
            }
            e
        } else {
            let mut e = ReflectionEdgeWalker::new(p1, p2, q1, q2, n1, n2, w1, w2);
            if y_start > p1.y {
                e.step_n(i64::from(y_start) - i64::from(p1.y));
            }
            e
        };

        for y in y_start..=y_end {
            if y == p1.y && y != p0.y {
                edge_b = ReflectionEdgeWalker::new(p1, p2, q1, q2, n1, n2, w1, w2);
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
                draw_scanline_reflection(
                    fb,
                    zb,
                    y,
                    x_start,
                    x_end,
                    ReflectionSpanStart {
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
                    camera_pos,
                    cubemap,
                );
            }

            edge_a.step();
            edge_b.step();
        }
    }
}

struct ReflectionGradients {
    dz_dx: f32,
    dq_dx: f32,
    dnx_dx: f32,
    dny_dx: f32,
    dnz_dx: f32,
    dwx_dx: f32,
    dwy_dx: f32,
    dwz_dx: f32,
}

impl ReflectionGradients {
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
    ) -> (Self, bool) {
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

        (
            Self {
                dz_dx,
                dq_dx,
                dnx_dx,
                dny_dx,
                dnz_dx,
                dwx_dx,
                dwy_dx,
                dwz_dx,
            },
            nz > 0.0,
        )
    }
}

struct ReflectionEdgeWalker {
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

impl ReflectionEdgeWalker {
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
