//! 4-wide batched SIMD primitive rendering.
//!
//! Process 4 pixels at once to amortize SIMD overhead.

use crate::framebuffer::Framebuffer;
use crate::math::{Vec2, Vec3};
use crate::texture::Texture;
use crate::zbuffer::ZBuffer;

#[cfg(target_feature = "sse")]
use super::rcp_ss;

/// Project a 3D point to screen coordinates (SIMD-optimized perspective divide)
#[inline]
#[cfg(target_feature = "sse")]
fn project_to_screen_simd(v: Vec3, w: f32, width: u32, height: u32) -> (i32, i32, f32) {
    let inv_w = if w.abs() > 0.0001 {
        unsafe { rcp_ss(w) }
    } else {
        1.0
    };

    let ndc_x = v.x * inv_w;
    let ndc_y = v.y * inv_w;
    let depth = v.z * inv_w;

    let screen_x = ((ndc_x + 1.0) * 0.5 * width as f32) as i32;
    let screen_y = ((1.0 - ndc_y) * 0.5 * height as f32) as i32;

    (screen_x, screen_y, depth)
}

/// 4-wide batched textured triangle fill using SSE for parallel pixel processing
///
/// Key optimization: Process 4 pixels at once to amortize SIMD overhead
///
/// Performance wins:
/// 1. RCPPS computes 4 reciprocals in parallel (same latency as 1)
/// 2. Vectorized UV interpolation
/// 3. Amortize loop overhead over 4 pixels
/// 4. Better instruction-level parallelism
#[allow(clippy::too_many_arguments)]
#[cfg(target_feature = "sse")]
pub fn fill_triangle_textured_batched(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec2),
    v1: ((Vec3, f32), Vec2),
    v2: ((Vec3, f32), Vec2),
    texture: &Texture,
) {
    let width = fb.width();
    let height = fb.height();

    // Project to screen using SIMD reciprocal
    let (x0, y0, z0) = project_to_screen_simd(v0.0 .0, v0.0 .1, width, height);
    let (x1, y1, z1) = project_to_screen_simd(v1.0 .0, v1.0 .1, width, height);
    let (x2, y2, z2) = project_to_screen_simd(v2.0 .0, v2.0 .1, width, height);

    // Pre-compute 1/w for all vertices using fast reciprocal
    let inv_w0 = unsafe { rcp_ss(v0.0 .1) };
    let inv_w1 = unsafe { rcp_ss(v1.0 .1) };
    let inv_w2 = unsafe { rcp_ss(v2.0 .1) };

    // Store UV/w instead of UV
    let uv_over_w0 = v0.1 * inv_w0;
    let uv_over_w1 = v1.1 * inv_w1;
    let uv_over_w2 = v2.1 * inv_w2;

    // Sort by y
    let mut verts = [
        (x0, y0, z0, inv_w0, uv_over_w0),
        (x1, y1, z1, inv_w1, uv_over_w1),
        (x2, y2, z2, inv_w2, uv_over_w2),
    ];
    if verts[0].1 > verts[1].1 {
        verts.swap(0, 1);
    }
    if verts[0].1 > verts[2].1 {
        verts.swap(0, 2);
    }
    if verts[1].1 > verts[2].1 {
        verts.swap(1, 2);
    }

    let (x0, y0, z0, inv_w0, uv_over_w0) = verts[0];
    let (x1, y1, z1, inv_w1, uv_over_w1) = verts[1];
    let (x2, y2, z2, inv_w2, uv_over_w2) = verts[2];

    let total_height = y2 - y0;
    if total_height == 0 {
        return;
    }

    // Clamp Y range to screen bounds
    let y_min = 0;
    let y_max = height as i32 - 1;
    let y_start = y0.max(y_min);
    let y_end = y2.min(y_max);

    for y in y_start..=y_end {
        let second_half = y > y1 || y1 == y0;
        let segment_height = if second_half { y2 - y1 } else { y1 - y0 };
        if segment_height == 0 {
            continue;
        }

        let alpha = (y - y0) as f32 / total_height as f32;
        let beta = if second_half {
            (y - y1) as f32 / segment_height as f32
        } else {
            (y - y0) as f32 / segment_height as f32
        };

        // Interpolate along edges
        let mut ax = x0 as f32 + (x2 - x0) as f32 * alpha;
        let mut az = z0 + (z2 - z0) * alpha;
        let mut a_inv_w = inv_w0 + (inv_w2 - inv_w0) * alpha;
        let mut a_uv_over_w = Vec2::new(
            uv_over_w0.x + (uv_over_w2.x - uv_over_w0.x) * alpha,
            uv_over_w0.y + (uv_over_w2.y - uv_over_w0.y) * alpha,
        );

        let (mut bx, mut bz, mut b_inv_w, mut b_uv_over_w) = if second_half {
            (
                x1 as f32 + (x2 - x1) as f32 * beta,
                z1 + (z2 - z1) * beta,
                inv_w1 + (inv_w2 - inv_w1) * beta,
                Vec2::new(
                    uv_over_w1.x + (uv_over_w2.x - uv_over_w1.x) * beta,
                    uv_over_w1.y + (uv_over_w2.y - uv_over_w1.y) * beta,
                ),
            )
        } else {
            (
                x0 as f32 + (x1 - x0) as f32 * beta,
                z0 + (z1 - z0) * beta,
                inv_w0 + (inv_w1 - inv_w0) * beta,
                Vec2::new(
                    uv_over_w0.x + (uv_over_w1.x - uv_over_w0.x) * beta,
                    uv_over_w0.y + (uv_over_w1.y - uv_over_w0.y) * beta,
                ),
            )
        };

        if ax > bx {
            std::mem::swap(&mut ax, &mut bx);
            std::mem::swap(&mut az, &mut bz);
            std::mem::swap(&mut a_inv_w, &mut b_inv_w);
            std::mem::swap(&mut a_uv_over_w, &mut b_uv_over_w);
        }

        let x_start = ax as i32;
        let x_end = bx as i32;
        let dx = x_end - x_start;

        if dx <= 0 {
            if x_start >= 0 && x_start < width as i32 {
                let w = unsafe { rcp_ss(a_inv_w) };
                let uv = a_uv_over_w * w;
                if zb.test_and_set(x_start, y, az) {
                    let color = texture.sample_nearest(uv.x, uv.y);
                    fb.set_pixel(x_start, y, color);
                }
            }
            continue;
        }

        // Pre-calculate increments
        let dz_dx = (bz - az) / dx as f32;
        let d_inv_w_dx = (b_inv_w - a_inv_w) / dx as f32;
        let d_uv_over_w_dx = Vec2::new(
            (b_uv_over_w.x - a_uv_over_w.x) / dx as f32,
            (b_uv_over_w.y - a_uv_over_w.y) / dx as f32,
        );

        let mut z = az;
        let mut inv_w = a_inv_w;
        let mut uv_over_w = a_uv_over_w;

        // Clamp X bounds
        let mut xs = x_start;
        let mut xe = x_end;

        if xs < 0 {
            let skip = -xs as f32;
            z += skip * dz_dx;
            inv_w += skip * d_inv_w_dx;
            uv_over_w = uv_over_w + (d_uv_over_w_dx * skip);
            xs = 0;
        }

        if xe >= width as i32 {
            xe = width as i32 - 1;
        }

        if xs > xe {
            continue;
        }

        // BATCHED 4-WIDE LOOP: The money maker
        let x_range = (xe - xs + 1) as usize;
        let num_batches = x_range / 4;
        let remainder = x_range % 4;

        // SAFETY: xs and xe are clamped to [0, width-1]. y is clamped to [0, height-1].
        unsafe {
            let y_idx = y as usize;
            let mut xi = xs as usize;

            // Process 4 pixels at once
            for _batch in 0..num_batches {
                // Gather 4 inv_w values
                let inv_w_batch = [
                    inv_w,
                    inv_w + d_inv_w_dx,
                    inv_w + d_inv_w_dx * 2.0,
                    inv_w + d_inv_w_dx * 3.0,
                ];

                // Batch reciprocal: 4 divisions for the price of 1!
                let w_batch = batch_reciprocal_sse(inv_w_batch);

                // Process each pixel in the batch
                for i in 0..4 {
                    let pixel_z = z + dz_dx * i as f32;
                    let pixel_uv_over_w = Vec2::new(
                        uv_over_w.x + d_uv_over_w_dx.x * i as f32,
                        uv_over_w.y + d_uv_over_w_dx.y * i as f32,
                    );

                    if zb.test_and_set_unchecked(xi + i, y_idx, pixel_z) {
                        // Perspective-correct UV using pre-computed w
                        let u = pixel_uv_over_w.x * w_batch[i];
                        let v = pixel_uv_over_w.y * w_batch[i];
                        let color = texture.sample_nearest(u, v);
                        fb.set_pixel_unchecked(xi + i, y_idx, color);
                    }
                }

                // Advance by 4 pixels
                z += dz_dx * 4.0;
                inv_w += d_inv_w_dx * 4.0;
                uv_over_w = uv_over_w + (d_uv_over_w_dx * 4.0);
                xi += 4;
            }

            // Handle remainder pixels (0-3) with scalar code
            for _i in 0..remainder {
                if zb.test_and_set_unchecked(xi, y_idx, z) {
                    let w = rcp_ss(inv_w);
                    let u = uv_over_w.x * w;
                    let v = uv_over_w.y * w;
                    let color = texture.sample_nearest(u, v);
                    fb.set_pixel_unchecked(xi, y_idx, color);
                }
                z += dz_dx;
                inv_w += d_inv_w_dx;
                uv_over_w = uv_over_w + d_uv_over_w_dx;
                xi += 1;
            }
        }
    }
}

/// Batch reciprocal using RCPPS: compute 4 reciprocals at once
#[inline]
#[cfg(target_feature = "sse")]
unsafe fn batch_reciprocal_sse(inv_w: [f32; 4]) -> [f32; 4] {
    use std::arch::asm;

    let mut result = [0.0f32; 4];

    // SAFETY: Caller ensures SSE support
    unsafe {
        asm!(
            "movups xmm0, [{inv_w}]",       // Load 4 inv_w values
            "rcpps xmm0, xmm0",              // 4 reciprocals in parallel!
            "movups [{result}], xmm0",       // Store 4 w values
            inv_w = in(reg) &inv_w as *const [f32; 4],
            result = in(reg) &mut result as *mut [f32; 4],
            out("xmm0") _,
            options(nostack),
        );
    }

    result
}
