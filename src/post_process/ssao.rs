use super::blur::box_blur_f32;
use crate::framebuffer::Framebuffer;
use crate::math::{Mat4, Vec3};
use crate::zbuffer::ZBuffer;
use std::cell::RefCell;

thread_local! {
    static SSAO_CONTEXT: RefCell<SsaoContext> = RefCell::new(SsaoContext::default());
}

const KERNEL_SIZE: usize = 16;
const NOISE_SIZE: usize = 4;

struct SsaoContext {
    occlusion_buffer: Vec<f32>,
    scratch_buffer: Vec<f32>,
    acc_buffer: Vec<f32>,
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
            kernel: [Vec3::default(); KERNEL_SIZE],
            noise: [Vec3::default(); NOISE_SIZE * NOISE_SIZE],
            initialized: false,
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
/// * `radius` - Sampling radius in view space (e.g., 0.5).
/// * `bias` - Bias to prevent self-occlusion (e.g., 0.025).
/// * `intensity` - Strength of the effect (e.g., 1.0 - 3.0).
pub fn apply_ssao(
    fb: &mut Framebuffer,
    zb: &ZBuffer,
    proj: &Mat4,
    radius: f32,
    bias: f32,
    intensity: f32,
) {
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
                        radius,
                        bias,
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
                    width,
                    height,
                    radius,
                    bias,
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
            width,
            height,
            radius,
            bias,
        );

        box_blur_f32(occlusion_buffer, scratch_buffer, acc_buffer, width, height);

        let pixels = fb.as_mut_slice();
        for (i, p) in pixels.iter_mut().enumerate() {
            let occ = occlusion_buffer[i];
            let factor = 1.0 - (occ / KERNEL_SIZE as f32) * intensity;
            let factor = factor.clamp(0.0, 1.0);

            let r = ((*p >> 16) & 0xFF) as f32;
            let g = ((*p >> 8) & 0xFF) as f32;
            let b = (*p & 0xFF) as f32;

            let new_r = (r * factor) as u32;
            let new_g = (g * factor) as u32;
            let new_b = (b * factor) as u32;

            *p = (*p & 0xFF00_0000) | (new_r << 16) | (new_g << 8) | new_b;
        }
    });
}

fn apply_ssao_scalar(
    occlusion_buffer: &mut [f32],
    zb: &ZBuffer,
    proj: &Mat4,
    kernel: &[Vec3],
    noise: &[Vec3],
    width: usize,
    height: usize,
    radius: f32,
    bias: f32,
) {
    // Projection parameters
    let p00 = proj.m[0][0];
    let p11 = proj.m[1][1];
    let p22 = proj.m[2][2];
    let p32 = proj.m[3][2];

    let half_width = width as f32 * 0.5;
    let half_height = height as f32 * 0.5;

    for y in 0..height {
        let noise_y = y % NOISE_SIZE;
        for x in 0..width {
            let noise_x = x % NOISE_SIZE;
            let noise_idx = noise_y * NOISE_SIZE + noise_x;
            let random_vec = noise[noise_idx];

            let depth_val = zb.get_depth(x as i32, y as i32).unwrap_or(1.0);

            if depth_val >= 1.0 {
                occlusion_buffer[y * width + x] = 0.0;
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

            for k in 0..KERNEL_SIZE {
                let s = kernel[k];
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

            occlusion_buffer[y * width + x] = occlusion;
        }
    }
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

        let zb_data = zb.as_slice();

        for y in 0..height {
            let y_idx = y * width;
            let mut x = 0;

            let noise_y = y % NOISE_SIZE;
            let noise_y_vec = _mm256_set1_epi32((noise_y * NOISE_SIZE) as i32);

            while x + 8 <= width {
                let depth_ptr = zb_data.as_ptr().add(y_idx + x);
                let depth_val = _mm256_loadu_ps(depth_ptr);

                // Check valid depth (< 1.0)
                let mask_valid = _mm256_cmp_ps(depth_val, one, _CMP_LT_OQ);

                if _mm256_movemask_ps(mask_valid) == 0 {
                    _mm256_storeu_ps(occlusion_buffer.as_mut_ptr().add(y_idx + x), zero);
                    x += 8;
                    continue;
                }

                // Reconstruct View Z
                let denom = _mm256_add_ps(depth_val, p22);
                let z_view = _mm256_div_ps(_mm256_sub_ps(zero, p32), denom);

                // Reconstruct X, Y View
                let x_offsets = _mm256_set_ps(7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0);
                let x_base = _mm256_set1_ps(x as f32);
                let x_vals = _mm256_add_ps(x_base, x_offsets);

                let x_ndc = _mm256_sub_ps(_mm256_div_ps(x_vals, half_w_vec), one);
                let y_ndc = _mm256_sub_ps(one, _mm256_div_ps(_mm256_set1_ps(y as f32), half_h_vec));

                let neg_z_view = _mm256_sub_ps(zero, z_view);
                let x_view = _mm256_div_ps(_mm256_mul_ps(x_ndc, neg_z_view), p00);
                let y_view = _mm256_div_ps(_mm256_mul_ps(y_ndc, neg_z_view), p11);

                let mut occlusion = _mm256_setzero_ps();

                // Gather Noise
                let x_i = _mm256_set_epi32(
                    x as i32 + 7,
                    x as i32 + 6,
                    x as i32 + 5,
                    x as i32 + 4,
                    x as i32 + 3,
                    x as i32 + 2,
                    x as i32 + 1,
                    x as i32,
                );
                let noise_mask = _mm256_set1_epi32(3);
                let noise_x = _mm256_and_si256(x_i, noise_mask);
                let noise_idx = _mm256_add_epi32(noise_y_vec, noise_x);

                let idx_3 = _mm256_mullo_epi32(noise_idx, _mm256_set1_epi32(3)); // stride 3 floats (12 bytes)
                let noise_ptr = noise.as_ptr() as *const f32;
                // Gather X and Y components of noise
                let rx = _mm256_i32gather_ps(noise_ptr, idx_3, 4);
                let ry = _mm256_i32gather_ps(
                    noise_ptr,
                    _mm256_add_epi32(idx_3, _mm256_set1_epi32(1)),
                    4,
                );

                for k in 0..KERNEL_SIZE {
                    let s = kernel[k];
                    let sx = _mm256_set1_ps(s.x);
                    let sy = _mm256_set1_ps(s.y);
                    let sz = _mm256_set1_ps(s.z);

                    // Rotate sample
                    let rot_x = _mm256_sub_ps(_mm256_mul_ps(sx, rx), _mm256_mul_ps(sy, ry));
                    let rot_y = _mm256_add_ps(_mm256_mul_ps(sx, ry), _mm256_mul_ps(sy, rx));
                    let rot_z = sz;

                    let samp_x = _mm256_fmadd_ps(rot_x, radius_vec, x_view);
                    let samp_y = _mm256_fmadd_ps(rot_y, radius_vec, y_view);
                    let samp_z = _mm256_fmadd_ps(rot_z, radius_vec, z_view);

                    // Project
                    let clip_x = _mm256_mul_ps(samp_x, p00);
                    let clip_y = _mm256_mul_ps(samp_y, p11);
                    let clip_w = _mm256_sub_ps(zero, samp_z);

                    let mask_w = _mm256_cmp_ps(clip_w, zero, _CMP_GT_OQ);
                    let inv_w = _mm256_div_ps(one, clip_w);

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
                    let existing_view_z = _mm256_div_ps(_mm256_sub_ps(zero, p32), existing_z_denom);

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
                _mm256_storeu_ps(occlusion_buffer.as_mut_ptr().add(y_idx + x), final_occ);

                x += 8;
            }

            // Tail
            while x < width {
                let noise_x = x % NOISE_SIZE;
                let noise_idx = noise_y * NOISE_SIZE + noise_x;
                let random_vec = noise[noise_idx];

                let depth_val = zb.get_depth(x as i32, y as i32).unwrap_or(1.0);

                if depth_val >= 1.0 {
                    occlusion_buffer[y * width + x] = 0.0;
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

                for k in 0..KERNEL_SIZE {
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
                occlusion_buffer[y * width + x] = occlusion;
                x += 1;
            }
        }
    }
}

/// Generates a deterministic pseudo-random kernel for SSAO sampling.
fn generate_kernel() -> [Vec3; KERNEL_SIZE] {
    let mut kernel = [Vec3::default(); KERNEL_SIZE];
    let mut seed = 123456789;

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
    let mut seed = 987654321;

    for v in &mut noise {
        let x = rand_f32(&mut seed) * 2.0 - 1.0;
        let y = rand_f32(&mut seed) * 2.0 - 1.0;
        *v = Vec3::new(x, y, 0.0).normalize();
    }

    noise
}

/// Simple Linear Congruential Generator for deterministic randomness.
fn rand_f32(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    (*seed >> 9) as f32 / 8388607.0
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
