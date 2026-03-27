//! Screen-Space Ambient Occlusion (SSAO).
//!
//! Approximates ambient lighting attenuation based on depth buffer geometry.

use super::blur::box_blur_f32;
use crate::framebuffer::Framebuffer;
use crate::math::{Mat4, Vec3};
use crate::zbuffer::ZBuffer;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

thread_local! {
    static SSAO_CONTEXT: RefCell<SsaoContext> = RefCell::new(SsaoContext::default());
}

const KERNEL_SIZE: usize = 16;
const NOISE_SIZE: usize = 4;

struct SsaoContext {
    occlusion_buffer: Vec<f32>,
    scratch_buffer: Vec<f32>,
    acc_buffer: Vec<f32>,
    precomputed_kernel_buffer: Vec<f32>,
    kernel: [Vec3; KERNEL_SIZE],
    noise: [Vec3; NOISE_SIZE * NOISE_SIZE],
    initialized: bool,
}

impl Default for SsaoContext {
    fn default() -> Self {
        Self {
            occlusion_buffer: Vec::new(),
            scratch_buffer: Vec::new(),
            acc_buffer: Vec::new(),
            precomputed_kernel_buffer: Vec::new(),
            kernel: [Vec3::default(); KERNEL_SIZE],
            noise: [Vec3::default(); NOISE_SIZE * NOISE_SIZE],
            initialized: false,
        }
    }
}

/// Configuration for the SSAO effect.
#[derive(Clone, Copy, Debug)]
pub struct SsaoConfig {
    /// Sampling radius in view space (e.g., 0.5).
    pub radius: f32,
    /// Bias to prevent self-occlusion (e.g., 0.025).
    pub bias: f32,
    /// Strength of the effect (e.g., 1.0 - 3.0).
    pub intensity: f32,
}

impl Default for SsaoConfig {
    fn default() -> Self {
        Self {
            radius: 0.5,
            bias: 0.025,
            intensity: 1.0,
        }
    }
}

/// Applies Screen-Space Ambient Occlusion to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify (darkened by occlusion).
/// * `zb` - The depth buffer (source of geometry).
/// * `proj` - The projection matrix used to render the scene.
/// * `config` - Configuration for the SSAO effect.
pub fn apply_ssao(fb: &mut Framebuffer, zb: &ZBuffer, proj: &Mat4, config: &SsaoConfig) {
    if fb.width() != zb.width() || fb.height() != zb.height() {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let needed_size = width * height;

    SSAO_CONTEXT.with(|ctx_ref| {
        let mut ctx_guard = ctx_ref.borrow_mut();
        let ctx = &mut *ctx_guard;

        if !ctx.initialized {
            ctx.kernel = generate_kernel();
            ctx.noise = generate_noise();
            ctx.precomputed_kernel_buffer = generate_precomputed_kernels(&ctx.kernel, &ctx.noise);
            ctx.initialized = true;
        }

        if ctx.occlusion_buffer.len() < needed_size {
            ctx.occlusion_buffer.resize(needed_size, 0.0);
        }
        if ctx.scratch_buffer.len() < needed_size {
            ctx.scratch_buffer.resize(needed_size, 0.0);
        }
        if ctx.acc_buffer.len() < width {
            ctx.acc_buffer.resize(width, 0.0);
        }

        let occlusion_buffer = &mut ctx.occlusion_buffer[..needed_size];
        let scratch_buffer = &mut ctx.scratch_buffer[..needed_size];
        let acc_buffer = &mut ctx.acc_buffer[..width];
        let kernel = &ctx.kernel;
        let noise = &ctx.noise;
        // Projection parameters
        // Flatten matrix for SIMD
        let mut proj_flat = [0.0; 16];
        for i in 0..4 {
            for j in 0..4 {
                proj_flat[i * 4 + j] = proj.m[i][j];
            }
        }

        #[cfg(all(target_arch = "x86_64", feature = "simd"))]
        {
            if std::is_x86_feature_detected!("avx2") {
                let half_width = width as f32 * 0.5;
                let half_height = height as f32 * 0.5;
                unsafe {
                    apply_ssao_avx2(
                        occlusion_buffer,
                        zb,
                        width,
                        height,
                        &proj_flat,
                        kernel,
                        noise,
                        &ctx.precomputed_kernel_buffer,
                        config.radius,
                        config.bias,
                        half_width,
                        half_height,
                    );
                }
            } else {
                apply_ssao_scalar(
                    occlusion_buffer,
                    zb,
                    proj,
                    kernel,
                    noise,
                    config.radius,
                    config.bias,
                );
            }
        }
        #[cfg(not(all(target_arch = "x86_64", feature = "simd")))]
        apply_ssao_scalar(
            occlusion_buffer,
            zb,
            proj,
            kernel,
            noise,
            config.radius,
            config.bias,
        );

        box_blur_f32(occlusion_buffer, scratch_buffer, acc_buffer, width, height);

        let pixels = fb.as_mut_slice();

        // /// Bolt Performance Optimization:
        // /// Precompute intensity multipliers to avoid float-division and scaling in the inner loop
        // /// Reduces floating-point operations.
        let inv_kernel_size = 1.0 / KERNEL_SIZE as f32;
        let intensity_factor = inv_kernel_size * config.intensity;

        for (p, &occ) in pixels.iter_mut().zip(occlusion_buffer.iter()) {
            let factor = 1.0 - occ * intensity_factor;
            let factor = factor.clamp(0.0, 1.0);

            // /// Bolt Performance Optimization:
            // /// Use integer fixed-point math for color blending (8.8 precision)
            // /// Removes floating point multiplications for R, G, and B.
            let factor_fixed = (factor * 256.0) as u32;

            // ⚡ Bolt: SWAR (SIMD Within A Register) for per-pixel color scaling.
            // Process Red and Blue channels simultaneously to eliminate intermediate shifts.
            let orig = *p;
            let rb = orig & 0x00FF_00FF;
            let g = orig & 0x0000_FF00;

            let rb_new = ((rb * factor_fixed) >> 8) & 0x00FF_00FF;
            let g_new = ((g * factor_fixed) >> 8) & 0x0000_FF00;

            *p = (orig & 0xFF00_0000) | rb_new | g_new;
        }
    });
}

fn apply_ssao_scalar(
    occlusion_buffer: &mut [f32],
    zb: &ZBuffer,
    proj: &Mat4,
    kernel: &[Vec3],
    noise: &[Vec3],
    radius: f32,
    bias: f32,
) {
    let width = zb.width() as usize;
    let height = zb.height() as usize;

    // Projection parameters
    let p00 = proj.m[0][0];
    let p11 = proj.m[1][1];
    let p22 = proj.m[2][2];
    let p32 = proj.m[3][2];

    if width == 0 {
        return;
    }

    let half_width = width as f32 * 0.5;
    let half_height = height as f32 * 0.5;

    #[cfg(feature = "parallel")]
    let iter = occlusion_buffer.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let iter = occlusion_buffer.chunks_exact_mut(width).enumerate();

    iter.for_each(|(y, row)| {
        let noise_y = y % NOISE_SIZE;
        for (x, row_x) in row.iter_mut().enumerate().take(width) {
            let noise_x = x % NOISE_SIZE;
            let noise_idx = noise_y * NOISE_SIZE + noise_x;
            let random_vec = noise[noise_idx];

            let depth_val = zb.get_depth(x as i32, y as i32).unwrap_or(1.0);

            if depth_val >= 1.0 {
                *row_x = 0.0;
                continue;
            }

            // Reconstruct View Position
            let z_view = -p32 / (depth_val + p22);
            let x_ndc = (x as f32 / half_width) - 1.0;
            let y_ndc = 1.0 - (y as f32 / half_height);
            let x_view = x_ndc * (-z_view) / p00;
            let y_view = y_ndc * (-z_view) / p11;
            let pos_view = Vec3::new(x_view, y_view, z_view);

            let rx = random_vec.x;
            let ry = random_vec.y;

            let mut occlusion = 0.0;

            for s in kernel.iter().take(KERNEL_SIZE) {
                let rotated_sample = Vec3::new(s.x * rx - s.y * ry, s.x * ry + s.y * rx, s.z);

                let sample_pos = pos_view + rotated_sample * radius;
                let (sample_clip, sample_w) = proj.transform_point(sample_pos);

                if sample_w > 0.0 {
                    let inv_w = 1.0 / sample_w;
                    let s_ndc_x = sample_clip.x * inv_w;
                    let s_ndc_y = sample_clip.y * inv_w;

                    let s_screen_x = ((s_ndc_x + 1.0) * half_width) as i32;
                    let s_screen_y = ((1.0 - s_ndc_y) * half_height) as i32;

                    if s_screen_x >= 0
                        && s_screen_x < width as i32
                        && s_screen_y >= 0
                        && s_screen_y < height as i32
                    {
                        let existing_depth = zb.get_depth(s_screen_x, s_screen_y).unwrap_or(1.0);
                        let existing_view_z = -p32 / (existing_depth + p22);
                        let sample_view_z = sample_pos.z;
                        let range_check = (existing_view_z - sample_view_z).abs() < radius;

                        if existing_view_z >= sample_view_z + bias && range_check {
                            occlusion += 1.0;
                        }
                    }
                }
            }

            *row_x = occlusion;
        }
    });
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[target_feature(enable = "avx2")]
unsafe fn apply_ssao_avx2(
    occlusion_buffer: &mut [f32],
    zb: &ZBuffer,
    width: usize,
    height: usize,
    proj_m: &[f32; 16],
    kernel: &[Vec3],
    noise: &[Vec3],
    precomputed_kernels: &[f32],
    radius: f32,
    bias: f32,
    half_width: f32,
    half_height: f32,
) {
    use std::arch::x86_64::*;

    unsafe {
        let p00 = _mm256_set1_ps(proj_m[0]);
        let p11 = _mm256_set1_ps(proj_m[5]);
        let p22 = _mm256_set1_ps(proj_m[10]);
        let p32 = _mm256_set1_ps(proj_m[14]);

        // Precompute inverse constants to replace division with multiplication
        let inv_p00 = _mm256_set1_ps(1.0 / proj_m[0]);
        let inv_p11 = _mm256_set1_ps(1.0 / proj_m[5]);
        let inv_half_w = _mm256_set1_ps(1.0 / half_width);
        let inv_half_h = _mm256_set1_ps(1.0 / half_height);

        let radius_vec = _mm256_set1_ps(radius);
        let bias_vec = _mm256_set1_ps(bias);
        let half_w_vec = _mm256_set1_ps(half_width);
        let half_h_vec = _mm256_set1_ps(half_height);
        let one = _mm256_set1_ps(1.0);
        let zero = _mm256_setzero_ps();
        let minus_zero = _mm256_set1_ps(-0.0);

        let width_i = _mm256_set1_epi32(width as i32);
        let height_i = _mm256_set1_epi32(height as i32);
        let minus_one_i = _mm256_set1_epi32(-1);

        if width == 0 {
            return;
        }

        let zb_data = zb.as_slice();

        #[cfg(feature = "parallel")]
        let iter = occlusion_buffer.par_chunks_exact_mut(width).enumerate();
        #[cfg(not(feature = "parallel"))]
        let iter = occlusion_buffer.chunks_exact_mut(width).enumerate();

        iter.for_each(|(y, row)| {
            let y_idx = y * width;
            let mut x = 0;

            let noise_y = y % NOISE_SIZE;

            while x + 8 <= width {
                let depth_ptr = zb_data.as_ptr().add(y_idx + x);
                let depth_val = _mm256_loadu_ps(depth_ptr);

                // Check valid depth (< 1.0)
                let mask_valid = _mm256_cmp_ps(depth_val, one, _CMP_LT_OQ);

                if _mm256_movemask_ps(mask_valid) == 0 {
                    _mm256_storeu_ps(row.as_mut_ptr().add(x), zero);
                    x += 8;
                    continue;
                }

                // Reconstruct View Z
                // z_view = -p32 / (depth_val + p22)
                // Use rcp for division: z_view ~= -p32 * rcp(depth_val + p22)
                let denom = _mm256_add_ps(depth_val, p22);
                let rcp_denom = _mm256_rcp_ps(denom);
                // Optional Newton-Raphson step for better precision: x1 = x0 * (2 - d * x0)
                // let rcp_denom = _mm256_mul_ps(rcp_denom, _mm256_sub_ps(_mm256_set1_ps(2.0), _mm256_mul_ps(denom, rcp_denom)));

                let z_view = _mm256_mul_ps(_mm256_sub_ps(zero, p32), rcp_denom);

                // Reconstruct X, Y View
                let x_offsets = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);
                let x_base = _mm256_set1_ps(x as f32);
                let x_vals = _mm256_add_ps(x_base, x_offsets);

                // x_ndc = (x / half_width) - 1.0 = x * inv_half_w - 1.0
                let x_ndc = _mm256_sub_ps(_mm256_mul_ps(x_vals, inv_half_w), one);
                // y_ndc = 1.0 - (y / half_height) = 1.0 - y * inv_half_h
                let y_ndc = _mm256_sub_ps(one, _mm256_mul_ps(_mm256_set1_ps(y as f32), inv_half_h));

                let neg_z_view = _mm256_sub_ps(zero, z_view);

                // x_view = x_ndc * (-z_view) / p00 = x_ndc * (-z_view) * inv_p00
                let x_view = _mm256_mul_ps(_mm256_mul_ps(x_ndc, neg_z_view), inv_p00);
                // y_view = y_ndc * (-z_view) / p11 = y_ndc * (-z_view) * inv_p11
                let y_view = _mm256_mul_ps(_mm256_mul_ps(y_ndc, neg_z_view), inv_p11);

                let mut occlusion = _mm256_setzero_ps();

                for (k, &_s) in kernel.iter().enumerate().take(KERNEL_SIZE) {
                    let s = kernel[k];
                    let sz = _mm256_set1_ps(s.z);

                    // Load precomputed rotated X/Y
                    // index = (noise_y * KERNEL_SIZE + k) * 16
                    let kernel_offset = (noise_y * KERNEL_SIZE + k) * 16;
                    let ptr = precomputed_kernels.as_ptr().add(kernel_offset);

                    let rot_x = _mm256_loadu_ps(ptr);
                    let rot_y = _mm256_loadu_ps(ptr.add(8));
                    let rot_z = sz;

                    let samp_x = _mm256_fmadd_ps(rot_x, radius_vec, x_view);
                    let samp_y = _mm256_fmadd_ps(rot_y, radius_vec, y_view);
                    let samp_z = _mm256_fmadd_ps(rot_z, radius_vec, z_view);

                    // Project
                    let clip_x = _mm256_mul_ps(samp_x, p00);
                    let clip_y = _mm256_mul_ps(samp_y, p11);
                    let clip_w = _mm256_sub_ps(zero, samp_z);

                    let mask_w = _mm256_cmp_ps(clip_w, zero, _CMP_GT_OQ);

                    // inv_w = 1.0 / clip_w. Use approximate reciprocal.
                    let inv_w = _mm256_rcp_ps(clip_w);

                    let ndc_x = _mm256_mul_ps(clip_x, inv_w);
                    let ndc_y = _mm256_mul_ps(clip_y, inv_w);

                    let s_x_f = _mm256_mul_ps(_mm256_add_ps(ndc_x, one), half_w_vec);
                    let s_y_f = _mm256_mul_ps(_mm256_sub_ps(one, ndc_y), half_h_vec);

                    let s_x = _mm256_cvttps_epi32(s_x_f);
                    let s_y = _mm256_cvttps_epi32(s_y_f);

                    // Bounds check
                    let mask_x = _mm256_and_si256(
                        _mm256_cmpgt_epi32(s_x, minus_one_i),
                        _mm256_cmpgt_epi32(width_i, s_x),
                    );
                    let mask_y = _mm256_and_si256(
                        _mm256_cmpgt_epi32(s_y, minus_one_i),
                        _mm256_cmpgt_epi32(height_i, s_y),
                    );
                    let mask_bounds = _mm256_and_si256(mask_x, mask_y);

                    let idx = _mm256_add_epi32(_mm256_mullo_epi32(s_y, width_i), s_x);

                    // Gather depths with mask
                    let existing_depth = _mm256_mask_i32gather_ps(
                        one,
                        zb_data.as_ptr(),
                        idx,
                        _mm256_castsi256_ps(mask_bounds),
                        4,
                    );

                    let existing_z_denom = _mm256_add_ps(existing_depth, p22);
                    // Use rcp for existing_view_z calculation as well
                    let existing_view_z =
                        _mm256_mul_ps(_mm256_sub_ps(zero, p32), _mm256_rcp_ps(existing_z_denom));

                    // Range check
                    let dist = _mm256_andnot_ps(minus_zero, _mm256_sub_ps(existing_view_z, samp_z));
                    let mask_range = _mm256_cmp_ps(dist, radius_vec, _CMP_LT_OQ);

                    // Bias check
                    let mask_bias =
                        _mm256_cmp_ps(existing_view_z, _mm256_add_ps(samp_z, bias_vec), _CMP_GE_OQ);

                    let mask_total = _mm256_and_ps(mask_w, _mm256_castsi256_ps(mask_bounds));
                    let mask_total = _mm256_and_ps(mask_total, mask_range);
                    let mask_total = _mm256_and_ps(mask_total, mask_bias);

                    let contribution = _mm256_and_ps(mask_total, one);
                    occlusion = _mm256_add_ps(occlusion, contribution);
                }

                let final_occ = _mm256_and_ps(occlusion, mask_valid);
                _mm256_storeu_ps(row.as_mut_ptr().add(x), final_occ);

                x += 8;
            }

            // Tail
            while x < width {
                let noise_x = x % NOISE_SIZE;
                let noise_idx = noise_y * NOISE_SIZE + noise_x;
                let random_vec = noise[noise_idx];

                let depth_val = zb.get_depth(x as i32, y as i32).unwrap_or(1.0);

                if depth_val >= 1.0 {
                    row[x] = 0.0;
                    x += 1;
                    continue;
                }

                let z_view = -proj_m[14] / (depth_val + proj_m[10]);

                let x_ndc = (x as f32 / half_width) - 1.0;
                let y_ndc = 1.0 - (y as f32 / half_height);
                let x_view = x_ndc * (-z_view) / proj_m[0];
                let y_view = y_ndc * (-z_view) / proj_m[5];
                let pos_view = Vec3::new(x_view, y_view, z_view);

                let rx = random_vec.x;
                let ry = random_vec.y;

                let mut occlusion = 0.0;

                for (k, &_s) in kernel.iter().enumerate().take(KERNEL_SIZE) {
                    let s = kernel[k];
                    let rotated_sample = Vec3::new(s.x * rx - s.y * ry, s.x * ry + s.y * rx, s.z);
                    let sample_pos = pos_view + rotated_sample * radius;

                    // Scalar projection
                    let (clip, w) = {
                        let x = sample_pos.x * proj_m[0]
                            + sample_pos.y * proj_m[4]
                            + sample_pos.z * proj_m[8]
                            + proj_m[12];
                        let y = sample_pos.x * proj_m[1]
                            + sample_pos.y * proj_m[5]
                            + sample_pos.z * proj_m[9]
                            + proj_m[13];
                        let z = sample_pos.x * proj_m[2]
                            + sample_pos.y * proj_m[6]
                            + sample_pos.z * proj_m[10]
                            + proj_m[14];
                        let w = sample_pos.x * proj_m[3]
                            + sample_pos.y * proj_m[7]
                            + sample_pos.z * proj_m[11]
                            + proj_m[15];
                        (Vec3::new(x, y, z), w)
                    };

                    if w > 0.0 {
                        let inv_w = 1.0 / w;
                        let s_ndc_x = clip.x * inv_w;
                        let s_ndc_y = clip.y * inv_w;
                        let s_screen_x = ((s_ndc_x + 1.0) * half_width) as i32;
                        let s_screen_y = ((1.0 - s_ndc_y) * half_height) as i32;

                        if s_screen_x >= 0
                            && s_screen_x < width as i32
                            && s_screen_y >= 0
                            && s_screen_y < height as i32
                        {
                            let idx = s_screen_y as usize * width + s_screen_x as usize;
                            let existing_depth = zb_data[idx];
                            let existing_view_z = -proj_m[14] / (existing_depth + proj_m[10]);
                            let sample_view_z = sample_pos.z;
                            if existing_view_z >= sample_view_z + bias
                                && (existing_view_z - sample_view_z).abs() < radius
                            {
                                occlusion += 1.0;
                            }
                        }
                    }
                }
                row[x] = occlusion;
                x += 1;
            }
        });
    }
}

/// Generates a deterministic pseudo-random kernel for SSAO sampling.
fn generate_kernel() -> [Vec3; KERNEL_SIZE] {
    let mut kernel = [Vec3::default(); KERNEL_SIZE];
    let mut seed = 123_456_789;

    for (i, v) in kernel.iter_mut().enumerate() {
        let r1 = rand_f32(&mut seed) * 2.0 - 1.0; // x: -1..1
        let r2 = rand_f32(&mut seed) * 2.0 - 1.0; // y: -1..1
        let r3 = rand_f32(&mut seed); // z: 0..1 (hemisphere)

        let mut sample = Vec3::new(r1, r2, r3).normalize();

        // Scale samples to distribute them within the hemisphere
        let scale = i as f32 / KERNEL_SIZE as f32;
        let scale = lerp(0.1, 1.0, scale * scale);
        sample = sample * scale;

        *v = sample;
    }

    kernel
}

/// Generates a noise texture for kernel rotation.
fn generate_noise() -> [Vec3; NOISE_SIZE * NOISE_SIZE] {
    let mut noise = [Vec3::default(); NOISE_SIZE * NOISE_SIZE];
    let mut seed = 987_654_321;

    for v in &mut noise {
        let x = rand_f32(&mut seed) * 2.0 - 1.0;
        let y = rand_f32(&mut seed) * 2.0 - 1.0;
        *v = Vec3::new(x, y, 0.0).normalize();
    }

    noise
}

fn generate_precomputed_kernels(kernel: &[Vec3], noise: &[Vec3]) -> Vec<f32> {
    // Layout: [noise_y (0..NOISE_SIZE)][kernel_idx (0..KERNEL_SIZE)][component (x, y)][simd_lane (0..8)]
    // Flat: NOISE_SIZE * KERNEL_SIZE * 2 * 8
    let mut buffer = vec![0.0; NOISE_SIZE * KERNEL_SIZE * 2 * 8];

    for ny in 0..NOISE_SIZE {
        for (k, &s) in kernel.iter().enumerate().take(KERNEL_SIZE) {
            // For each of the 8 SIMD lanes, we have a different x => different noise_x
            // Lane i corresponds to pixel x_base + i.
            // noise_x = (x_base + i) % NOISE_SIZE.
            // Since we process aligned to 8, the pattern of (x % 4) is:
            // 0, 1, 2, 3, 0, 1, 2, 3.

            let mut rot_xs = [0.0; 8];
            let mut rot_ys = [0.0; 8];

            for i in 0..8 {
                let nx = i % NOISE_SIZE;
                let noise_idx = ny * NOISE_SIZE + nx;
                let random_vec = noise[noise_idx];

                let rx = random_vec.x;
                let ry = random_vec.y;

                // Rotate sample around Z axis
                // x' = x*rx - y*ry
                // y' = x*ry + y*rx
                rot_xs[i] = s.x * rx - s.y * ry;
                rot_ys[i] = s.x * ry + s.y * rx;
            }

            // Store in buffer
            // 16 floats per kernel (8 for X, 8 for Y)
            let base_idx = (ny * KERNEL_SIZE + k) * 16;
            for i in 0..8 {
                buffer[base_idx + i] = rot_xs[i]; // X component
                buffer[base_idx + 8 + i] = rot_ys[i]; // Y component
            }
        }
    }
    buffer
}

/// Simple Linear Congruential Generator for deterministic randomness.
fn rand_f32(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*seed >> 9) as f32 / 8_388_607.0
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use crate::framebuffer::Framebuffer;
    #[allow(unused_imports)]
    use crate::math::Mat4;
    #[allow(unused_imports)]
    use crate::zbuffer::ZBuffer;
    #[allow(unused_imports)]
    use std::f32::consts::PI;

    #[test]
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    fn test_ssao_simd_vs_scalar() {
        if !std::is_x86_feature_detected!("avx2") {
            return;
        }

        let width = 64;
        let height = 64;
        let mut zb = ZBuffer::new(width, height).unwrap();
        let proj = Mat4::perspective(PI / 2.0, 1.0, 0.1, 100.0);

        // Fill Z buffer with some data (gradient)
        for y in 0..height {
            for x in 0..width {
                let depth = 0.5 + (x as f32 / width as f32) * 0.4;
                zb.test_and_set(x as i32, y as i32, depth);
            }
        }

        let mut occ_scalar = vec![0.0; (width * height) as usize];
        let mut occ_simd = vec![0.0; (width * height) as usize];

        let kernel = generate_kernel();
        let noise = generate_noise();
        let precomputed = generate_precomputed_kernels(&kernel, &noise);

        // Projection parameters
        let mut proj_flat = [0.0; 16];
        for i in 0..4 {
            for j in 0..4 {
                proj_flat[i * 4 + j] = proj.m[i][j];
            }
        }

        apply_ssao_scalar(&mut occ_scalar, &zb, &proj, &kernel, &noise, 1.0, 0.001);

        unsafe {
            apply_ssao_avx2(
                &mut occ_simd,
                &zb,
                width as usize,
                height as usize,
                &proj_flat,
                &kernel,
                &noise,
                &precomputed,
                1.0,
                0.001,
                width as f32 * 0.5,
                height as f32 * 0.5,
            );
        }

        // Compare
        let mut max_diff = 0.0f32;
        for i in 0..occ_scalar.len() {
            let diff = (occ_scalar[i] - occ_simd[i]).abs();
            if diff > max_diff {
                max_diff = diff;
            }
        }

        println!("Max difference between scalar and SIMD SSAO: {max_diff}");
        // Allow some difference due to floating point precision and rcp approximation
        assert!(
            max_diff <= 1.0,
            "SSAO output mismatch too large: {max_diff}",
        );
    }
}
