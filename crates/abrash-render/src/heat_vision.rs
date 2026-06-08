//! Heat Vision Effect
//!
//! Maps the depth buffer (Z-buffer) to a color gradient, simulating a thermal camera.
//! Uses auto-ranging to adapt to the scene depth.

use crate::framebuffer::Framebuffer;
use crate::zbuffer::ZBuffer;

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
            apply_heat_vision_simd(pixels, depths, min_z, scale);
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

        let (r, g, b) = if t < 256 {
            (255, t, 0)
        } else if t < 512 {
            (255 - (t - 256), 255, 0)
        } else if t < 768 {
            (0, 255, t - 512)
        } else {
            (0, 255 - (t - 768), 255)
        };

        *pixel = 0xFF00_0000 | (r << 16) | (g << 8) | b;
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2")]
unsafe fn apply_heat_vision_simd(pixels: &mut [u32], depths: &[f32], min_z: f32, scale: f32) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{
        __m256i, _CMP_EQ_OQ, _mm256_and_si256, _mm256_andnot_si256, _mm256_blendv_epi8,
        _mm256_castps_si256, _mm256_cmp_ps, _mm256_cmpgt_epi32, _mm256_cvttps_epi32,
        _mm256_loadu_ps, _mm256_max_epi32, _mm256_min_epi32, _mm256_mul_ps, _mm256_or_si256,
        _mm256_set1_epi32, _mm256_set1_ps, _mm256_setzero_si256, _mm256_slli_epi32,
        _mm256_storeu_si256, _mm256_sub_epi32, _mm256_sub_ps,
    };

    let len = pixels.len().min(depths.len());
    let mut i = 0;

    let min_z_vec = _mm256_set1_ps(min_z);
    let scale_vec = _mm256_set1_ps(scale);
    let inf_vec = _mm256_set1_ps(f32::INFINITY);
    let max_t_vec = _mm256_set1_epi32(1023);
    let bg_color = _mm256_set1_epi32(0xFF00_0010_u32 as i32);

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

        // Calculate R, G, B channels using integer math instead of LUT
        let t = t_clamped;

        let c255 = _mm256_set1_epi32(255);
        let c256 = _mm256_set1_epi32(256);
        let c512 = _mm256_set1_epi32(512);
        let c768 = _mm256_set1_epi32(768);

        // Condition masks
        let mask1 = _mm256_cmpgt_epi32(c256, t); // t < 256
        let mask2 = _mm256_andnot_si256(mask1, _mm256_cmpgt_epi32(c512, t)); // 256 <= t < 512
        let mask3 = _mm256_andnot_si256(_mm256_or_si256(mask1, mask2), _mm256_cmpgt_epi32(c768, t)); // 512 <= t < 768
        let mask4 = _mm256_andnot_si256(
            _mm256_or_si256(_mm256_or_si256(mask1, mask2), mask3),
            _mm256_set1_epi32(-1),
        ); // t >= 768

        // --- Calculate Red ---
        // if t < 256 => 255
        // if t < 512 => 255 - (t - 256)
        // else => 0
        let r_p1 = _mm256_and_si256(mask1, c255);
        let r_p2 = _mm256_and_si256(mask2, _mm256_sub_epi32(c255, _mm256_sub_epi32(t, c256)));
        let r = _mm256_or_si256(r_p1, r_p2);

        // --- Calculate Green ---
        // if t < 256 => t
        // if t < 512 => 255
        // if t < 768 => 255
        // else => 255 - (t - 768)
        let g_p1 = _mm256_and_si256(mask1, t);
        let g_p23 = _mm256_and_si256(_mm256_or_si256(mask2, mask3), c255);
        let g_p4 = _mm256_and_si256(mask4, _mm256_sub_epi32(c255, _mm256_sub_epi32(t, c768)));
        let g = _mm256_or_si256(_mm256_or_si256(g_p1, g_p23), g_p4);

        // --- Calculate Blue ---
        // if t < 512 => 0
        // if t < 768 => t - 512
        // else => 255
        let b_p3 = _mm256_and_si256(mask3, _mm256_sub_epi32(t, c512));
        let b_p4 = _mm256_and_si256(mask4, c255);
        let b = _mm256_or_si256(b_p3, b_p4);

        // Pack into ARGB: 0xFF000000 | (R << 16) | (G << 8) | B
        let a_shifted = _mm256_set1_epi32(0xFF00_0000_u32 as i32);
        let r_shifted = _mm256_slli_epi32::<16>(r);
        let g_shifted = _mm256_slli_epi32::<8>(g);
        let argb = _mm256_or_si256(
            _mm256_or_si256(a_shifted, r_shifted),
            _mm256_or_si256(g_shifted, b),
        );

        // Blend: if is_inf, use bg_color, else use calculated argb color
        let final_color = _mm256_blendv_epi8(argb, bg_color, is_inf_int);

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

        let (r, g, b) = if t < 256 {
            (255, t, 0)
        } else if t < 512 {
            (255 - (t - 256), 255, 0)
        } else if t < 768 {
            (0, 255, t - 512)
        } else {
            (0, 255 - (t - 768), 255)
        };

        *pixel = 0xFF00_0000 | (r << 16) | (g << 8) | b;
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
