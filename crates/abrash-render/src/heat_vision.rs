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

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    if std::is_x86_feature_detected!("avx2") {
        unsafe {
            let (mz, mz_max, hc) = find_min_max_simd(depths);
            min_z = mz;
            max_z = mz_max;
            has_content = hc;
        }
    } else {
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
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
    {
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
    }


    if !has_content {
        // Nothing drawn, just clear to cold background
        for p in pixels.iter_mut() {
            *p = 0xFF00_0020; // Dark Blue
        }
        return;
    }

    // Add a small epsilon to avoid division by zero if flat plane


    // Add a small epsilon to avoid division by zero if flat plane

    // Avoid float multiplication in the per-pixel loop by using fixed point arithmetic.
    // 16.16 fixed point allows enough precision for depth gradients without float hardware.

    // Scale is 1024.0 / range. We can precalculate a fixed point multiplier.
    // However, if the depths are huge, 16.16 fixed point depth * fixed point scale might overflow.
    // So we just precalculate `t` directly via fixed point if depths are within reason.
    // Wait, the memory instruction specifically says:
    // "Replacing floating-point normalization gradients with fixed-point integer scaling buckets and strict integer bounds checking inside per-pixel loops."



    // Avoid float multiplication in the per-pixel loop by using integer mapping.
    // We must handle extreme floating point values (f32::MAX/MIN) which can overflow fixed-point i64 math.
    // Instead, we will normalize the depth into a 0.0..1.0 float, then scale to 1024,
    // OR we just cap the depths if we want pure fixed point.
    // Wait, the directive specifically says:
    // "Replacing floating-point normalization gradients with fixed-point integer scaling buckets and strict integer bounds checking inside per-pixel loops."

    // To prevent fixed-point overflow with extreme f32 values, we cap the depths used for scaling to a safe fixed-point range.
    let max_safe_depth = 32000.0; // Arbitrary safe far-plane distance for fixed-point math
    let min_z_capped = min_z.clamp(-max_safe_depth, max_safe_depth);
    let max_z_capped = max_z.clamp(-max_safe_depth, max_safe_depth);
    let range = (max_z_capped - min_z_capped).max(0.0001);

    let min_z_fp = (min_z_capped * 65536.0) as i64;
    let scale_fp = ((1024.0 / range) * 65536.0) as i64;

    for (pixel, &depth) in pixels.iter_mut().zip(depths.iter()) {
        if depth == f32::INFINITY {
            *pixel = 0xFF00_0010;
            continue;
        }

        // Fixed point math. Clamp depth to safe range to prevent i64 overflow.
        let depth_capped = depth.clamp(-max_safe_depth, max_safe_depth);
        let depth_fp = (depth_capped * 65536.0) as i64;
        let t_fp = (depth_fp.saturating_sub(min_z_fp)).saturating_mul(scale_fp);

        let mut t = (t_fp >> 32) as u32;
        t = t.min(1023);

        *pixel = unsafe { *LUT.get_unchecked(t as usize) };
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


#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2")]
unsafe fn find_min_max_simd(depths: &[f32]) -> (f32, f32, bool) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    let mut min_vec = _mm256_set1_ps(f32::MAX);
    let mut max_vec = _mm256_set1_ps(f32::MIN);
    let inf_vec = _mm256_set1_ps(f32::INFINITY);
    let max_val_vec = _mm256_set1_ps(f32::MAX);
    let min_val_vec = _mm256_set1_ps(f32::MIN);

    let len = depths.len();
    let mut i = 0;

    while i + 8 <= len {
        let d = _mm256_loadu_ps(depths.as_ptr().add(i));
        let is_inf = _mm256_cmp_ps(d, inf_vec, _CMP_EQ_OQ);

        let min_d = _mm256_blendv_ps(d, max_val_vec, is_inf);
        let max_d = _mm256_blendv_ps(d, min_val_vec, is_inf);

        min_vec = _mm256_min_ps(min_vec, min_d);
        max_vec = _mm256_max_ps(max_vec, max_d);

        i += 8;
    }

    let mut mins = [0.0f32; 8];
    let mut maxs = [0.0f32; 8];
    _mm256_storeu_ps(mins.as_mut_ptr(), min_vec);
    _mm256_storeu_ps(maxs.as_mut_ptr(), max_vec);

    let mut min_z = f32::MAX;
    let mut max_z = f32::MIN;

    for j in 0..8 {
        if mins[j] < min_z {
            min_z = mins[j];
        }
        if maxs[j] > max_z {
            max_z = maxs[j];
        }
    }

    for &z in &depths[i..len] {
        if z != f32::INFINITY {
            if z < min_z {
                min_z = z;
            }
            if z > max_z {
                max_z = z;
            }
        }
    }

    let has_content = min_z != f32::MAX || max_z != f32::MIN;
    (min_z, max_z, has_content)
}


#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2")]
unsafe fn find_min_max_simd(depths: &[f32]) -> (f32, f32, bool) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    let mut min_vec = _mm256_set1_ps(f32::MAX);
    let mut max_vec = _mm256_set1_ps(f32::MIN);
    let inf_vec = _mm256_set1_ps(f32::INFINITY);
    let max_val_vec = _mm256_set1_ps(f32::MAX);
    let min_val_vec = _mm256_set1_ps(f32::MIN);

    let len = depths.len();
    let mut i = 0;

    while i + 8 <= len {
        let d = _mm256_loadu_ps(depths.as_ptr().add(i));
        let is_inf = _mm256_cmp_ps(d, inf_vec, _CMP_EQ_OQ);

        let min_d = _mm256_blendv_ps(d, max_val_vec, is_inf);
        let max_d = _mm256_blendv_ps(d, min_val_vec, is_inf);

        min_vec = _mm256_min_ps(min_vec, min_d);
        max_vec = _mm256_max_ps(max_vec, max_d);

        i += 8;
    }

    let mut mins = [0.0f32; 8];
    let mut maxs = [0.0f32; 8];
    _mm256_storeu_ps(mins.as_mut_ptr(), min_vec);
    _mm256_storeu_ps(maxs.as_mut_ptr(), max_vec);

    let mut min_z = f32::MAX;
    let mut max_z = f32::MIN;

    for j in 0..8 {
        if mins[j] < min_z {
            min_z = mins[j];
        }
        if maxs[j] > max_z {
            max_z = maxs[j];
        }
    }

    for &z in &depths[i..len] {
        if z != f32::INFINITY {
            if z < min_z {
                min_z = z;
            }
            if z > max_z {
                max_z = z;
            }
        }
    }

    let has_content = min_z != f32::MAX || max_z != f32::MIN;
    (min_z, max_z, has_content)
}
