//! Heat Vision Effect
//!
//! Maps the depth buffer (Z-buffer) to a color gradient, simulating a thermal camera.
//! Uses auto-ranging to adapt to the scene depth.

use crate::framebuffer::Framebuffer;
use crate::zbuffer::ZBuffer;

const fn generate_lut() -> [u32; 1024] {
    let mut lut = [0u32; 1024];
    let mut t = 0;
    while t < 1024 {
        let (r, g, b) = if t < 256 {
            // Red -> Yellow
            (255, t, 0)
        } else if t < 512 {
            // Yellow -> Green
            let local_t = t - 256;
            (255 - local_t, 255, 0)
        } else if t < 768 {
            // Green -> Cyan
            let local_t = t - 512;
            (0, 255, local_t)
        } else {
            // Cyan -> Blue
            let local_t = t - 768;
            (0, 255 - local_t, 255)
        };

        // Combine into ARGB
        lut[t as usize] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
        t += 1;
    }
    lut
}

/// Applies a heat vision effect to the framebuffer based on the depth buffer.
///
/// *   **Close objects** (small Z) are rendered as "Hot" (Red/Yellow).
/// *   **Far objects** (large Z) are rendered as "Cold" (Blue/Purple).
/// *   **Background** (Infinity) is rendered as black/dark blue.
///
/// # Examples
///
/// ```
/// use abrash_core::framebuffer::Framebuffer;
/// use abrash_core::zbuffer::ZBuffer;
/// use abrash_render::heat_vision::apply_heat_vision;
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// let mut zb = ZBuffer::new(100, 100).unwrap();
///
/// // Render something...
///
/// apply_heat_vision(&mut fb, &zb);
/// ```
pub fn apply_heat_vision(fb: &mut Framebuffer, zb: &ZBuffer) {
    const LUT: [u32; 1024] = generate_lut();

    if fb.width() != zb.width() || fb.height() != zb.height() {
        return;
    }

    let pixels = fb.as_mut_slice();
    let depths = zb.as_slice();

    // 1. Find min and max depth (excluding Infinity)
    let mut min_z = f32::MAX;
    let mut max_z = f32::MIN;
    let mut has_content = false;

    for &z in depths {
        if z != f32::INFINITY {
            if z < min_z {
                min_z = z;
            }
            if z > max_z {
                max_z = z;
            }
            has_content = true;
        }
    }

    if !has_content {
        // Nothing drawn, just clear to cold background
        for p in pixels.iter_mut() {
            *p = 0xFF00_0020; // Dark Blue
        }
        return;
    }

    // Add a small epsilon to avoid division by zero if flat plane
    let range = (max_z - min_z).max(0.0001);
    // Map [0.0, range] to [0, 1023] (4 segments of 256)
    // Adding a slight bias to prevent floating point inaccuracy at the absolute top end
    // from truncating 1024 to 1023 when scaling.
    let scale = 1024.0 / range;

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    if std::is_x86_feature_detected!("avx2") {
        unsafe {
            apply_heat_vision_simd(pixels, depths, min_z, scale, &LUT);
        }
        return;
    }

    for (pixel, &depth) in pixels.iter_mut().zip(depths.iter()) {
        if depth == f32::INFINITY {
            *pixel = 0xFF00_0010; // Very Dark Blue Background
            continue;
        }

        let t = ((depth - min_z) * scale) as u32;
        let t = t.min(1023); // Clamp strictly to 1023

        // SAFETY: t is strictly clamped to 1023 above, which is within the bounds of the 1024-element LUT.
        *pixel = unsafe { *LUT.get_unchecked(t as usize) };
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2")]
unsafe fn apply_heat_vision_simd(
    pixels: &mut [u32],
    depths: &[f32],
    min_z: f32,
    scale: f32,
    lut: &[u32; 1024],
) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{
        __m256i, _CMP_EQ_OQ, _mm256_blendv_epi8, _mm256_castps_si256, _mm256_cmp_ps,
        _mm256_cvttps_epi32, _mm256_loadu_ps, _mm256_max_epi32, _mm256_sub_epi32, _mm256_slli_epi32, _mm256_or_si256,
        _mm256_min_epi32, _mm256_mul_ps, _mm256_set1_epi32, _mm256_set1_ps, _mm256_setzero_si256,
        _mm256_storeu_si256, _mm256_sub_ps,
    };

    let len = pixels.len().min(depths.len());
    let mut i = 0;

    let min_z_vec = _mm256_set1_ps(min_z);
    let scale_vec = _mm256_set1_ps(scale);
    let inf_vec = _mm256_set1_ps(f32::INFINITY);
    let max_t_vec = _mm256_set1_epi32(1023);
    let bg_color = _mm256_set1_epi32(0xFF00_0010_u32 as i32);
    let lut_ptr = lut.as_ptr().cast::<i32>();

    while i + 8 <= len {
        let depth_ptr = depths.as_ptr().add(i);
        let depth_val = _mm256_loadu_ps(depth_ptr);

        // depth == f32::INFINITY
        let is_inf = _mm256_cmp_ps(depth_val, inf_vec, _CMP_EQ_OQ);
        let is_inf_int = _mm256_castps_si256(is_inf);

        // t = (depth - min_z) * scale
        let t_f32 = _mm256_mul_ps(_mm256_sub_ps(depth_val, min_z_vec), scale_vec);

        // t_u32 = t_f32 as i32
        let t_i32 = _mm256_cvttps_epi32(t_f32);

        // Ensure not negative
        let zero_vec = _mm256_setzero_si256();
        let t_clamped_low = _mm256_max_epi32(t_i32, zero_vec);

        // Clamp to 1023
        let t_clamped = _mm256_min_epi32(t_clamped_low, max_t_vec);

        // Calculate RGB components from t_clamped (0..1023)
        let t = t_clamped;

        // Constants for vector math
        let c255 = _mm256_set1_epi32(255);
        let c256 = _mm256_set1_epi32(256);
        let c511 = _mm256_set1_epi32(511);
        let c512 = _mm256_set1_epi32(512);
        let c768 = _mm256_set1_epi32(768);
        let c1023 = _mm256_set1_epi32(1023);
        let c0 = _mm256_setzero_si256();

        // 0 <= t < 256: R=255, G=t, B=0
        // 256 <= t < 512: R=511-t, G=255, B=0
        // 512 <= t < 768: R=0, G=255, B=t-512
        // 768 <= t < 1024: R=0, G=1023-t, B=255

        // R channel logic
        let r_t1 = _mm256_sub_epi32(c511, t); // 511 - t
        let r_t2 = _mm256_max_epi32(r_t1, c0); // max(511-t, 0)
        let r_clamped = _mm256_min_epi32(r_t2, c255); // clamp R to 255

        // G channel logic
        let g_t1 = _mm256_sub_epi32(c1023, t); // 1023 - t
        let g_t2 = _mm256_min_epi32(t, g_t1); // min(t, 1023-t)
        let g_clamped = _mm256_min_epi32(g_t2, c255); // clamp G to 255

        // B channel logic
        let b_t1 = _mm256_sub_epi32(t, c512); // t - 512
        let b_t2 = _mm256_max_epi32(b_t1, c0); // max(t-512, 0)
        let b_clamped = _mm256_min_epi32(b_t2, c255); // clamp B to 255

        // Shift into place
        let r_shifted = _mm256_slli_epi32(r_clamped, 16);
        let g_shifted = _mm256_slli_epi32(g_clamped, 8);

        // Alpha component 0xFF000000
        let a_shifted = _mm256_set1_epi32(0xFF00_0000_u32 as i32);

        // OR everything together
        let color_rgb = _mm256_or_si256(_mm256_or_si256(r_shifted, g_shifted), b_clamped);
        let gathered = _mm256_or_si256(a_shifted, color_rgb);

        // Blend: if is_inf, use bg_color, else use gathered color
        let final_color = _mm256_blendv_epi8(gathered, bg_color, is_inf_int);

        // Store to framebuffer
        #[allow(clippy::cast_ptr_alignment)]
        let fb_ptr = pixels.as_mut_ptr().add(i).cast::<__m256i>();
        _mm256_storeu_si256(fb_ptr, final_color);

        i += 8;
    }

    // Scalar tail
    for (pixel, &depth) in pixels[i..len].iter_mut().zip(depths[i..len].iter()) {
        if depth == f32::INFINITY {
            *pixel = 0xFF00_0010;
            continue;
        }

        let t = ((depth - min_z) * scale) as u32;
        let t = t.min(1023);

        *pixel = unsafe { *lut.get_unchecked(t as usize) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heat_vision_gradient() {
        let width = 5;
        let height = 1;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        // Set up a gradient of depths
        // 0: Close (1.0) -> Red
        // 1: Mid-Close (2.0) -> Yellow/Greenish
        // 2: Mid (3.0) -> Green
        // 3: Mid-Far (4.0) -> Cyan/Blueish
        // 4: Far (5.0) -> Blue

        zb.test_and_set(0, 0, 1.0);
        zb.test_and_set(1, 0, 2.0);
        zb.test_and_set(2, 0, 3.0);
        zb.test_and_set(3, 0, 4.0);
        zb.test_and_set(4, 0, 5.0);

        apply_heat_vision(&mut fb, &zb);

        // Check 0 (Closest/Red)
        let p0 = fb.get_pixel(0, 0).unwrap();
        assert_eq!(p0, 0xFFFF_0000, "Closest pixel should be Red");

        // Check 4 (Furthest/Blue)
        let p4 = fb.get_pixel(4, 0).unwrap();
        assert_eq!(p4, 0xFF00_00FF, "Furthest pixel should be Blue");

        // Check 2 (Middle/Green)
        let p2 = fb.get_pixel(2, 0).unwrap();
        // Middle of 1.0..5.0 is 3.0.
        // normalized = (3.0 - 1.0) / (5.0 - 1.0) = 0.5
        // At 0.5 -> Green (0, 255, 0)
        assert_eq!(p2, 0xFF00_FF00, "Middle pixel should be Green");
    }

    #[test]
    fn test_heat_vision_empty() {
        let mut fb = Framebuffer::new(1, 1).unwrap();
        let zb = ZBuffer::new(1, 1).unwrap(); // Infinity

        fb.set_pixel(0, 0, 0xFFFF_FFFF); // White

        apply_heat_vision(&mut fb, &zb);

        let p = fb.get_pixel(0, 0).unwrap();
        assert_eq!(p, 0xFF00_0020, "Empty buffer should be background color");
    }

    #[test]
    fn test_heat_vision_out_of_bounds_depths() {
        let mut fb = Framebuffer::new(2, 1).unwrap();
        let mut zb = ZBuffer::new(2, 1).unwrap();

        // Set depths that are extremely large and extremely small
        zb.test_and_set(0, 0, f32::MIN);
        zb.test_and_set(1, 0, f32::MAX);

        // Should not panic
        apply_heat_vision(&mut fb, &zb);

        // We aren't asserting specific colors here because float math at extremes
        // might underflow/overflow to inf/NaN, but we ensure it doesn't crash the renderer.
        let _p0 = fb.get_pixel(0, 0).unwrap();
        let _p1 = fb.get_pixel(1, 0).unwrap();
    }

    #[test]
    fn test_heat_vision_same_depth() {
        let mut fb = Framebuffer::new(2, 1).unwrap();
        let mut zb = ZBuffer::new(2, 1).unwrap();

        // Both pixels have the exact same depth
        zb.test_and_set(0, 0, 10.0);
        zb.test_and_set(1, 0, 10.0);

        apply_heat_vision(&mut fb, &zb);

        let p0 = fb.get_pixel(0, 0).unwrap();
        let p1 = fb.get_pixel(1, 0).unwrap();

        // With a fallback range of 0.0001, normalized will be 0.0 / 0.0001 = 0.0
        // So both should be mapped to the start of the gradient (Hot/Red)
        assert_eq!(p0, p1);
        assert_eq!(
            p0, 0xFFFF_0000,
            "Should map to Red when only one depth is present"
        );
    }

    #[test]
    fn test_heat_vision_mismatched_dimensions() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        let zb = ZBuffer::new(5, 5).unwrap();

        // Mismatched dimensions should early-return without doing anything or panicking
        apply_heat_vision(&mut fb, &zb);

        // First pixel should still be the default unchanged Framebuffer color (Solid Black/Transparent)
        let p = fb.get_pixel(0, 0).unwrap();
        assert_eq!(
            p, 0x00FF_000000,
            "Should remain unchanged default Framebuffer color (Solid Black)"
        );
    }
}
