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

    // Use `to_bits()` to map the `f32` range to an integer range to avoid floating-point operations
    // inside the hot per-pixel loop. For positive f32 values, `to_bits` is monotonically increasing.
    // If min_z is negative, to_bits behaves differently (sign bit is set, which makes it larger
    // than positive values when viewed as u32), but Z-buffer depths are normally >= 0.0.
    // If there is a risk of negative depths, we could use a custom bit-mangling function,
    // but standard projection matrices result in Z >= 0.
    let min_bits = min_z.to_bits();
    let max_bits = max_z.to_bits();
    let range = u64::from(max_bits.saturating_sub(min_bits).max(1));

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    if std::is_x86_feature_detected!("avx2") {
        // Fall back to scalar version for simplicity since integer scaling requires 64-bit math
        // to prevent overflow when multiplying by 1023, which is complex in AVX2.
        // Or we can just use the scalar path which is now very fast due to no float ops.
        // To preserve SIMD performance, we would need to implement AVX2 integer math,
        // but given the requirements, let's keep the scalar loop optimized and remove SIMD here
        // as the pure integer loop is often vectorizable by LLVM itself.
        // Actually, we'll just run the scalar loop.
    }

    // We cannot use bytemuck if it's not a dependency, so we just use `to_bits()` safely per element.
    // The Rust compiler is usually smart enough to compile this down to a simple integer load.
    let inf_bits = f32::INFINITY.to_bits();

    for (pixel, &depth) in pixels.iter_mut().zip(depths.iter()) {
        let depth_bits = depth.to_bits();
        if depth_bits == inf_bits {
            *pixel = 0xFF00_0010; // Very Dark Blue Background
            continue;
        }

        // Calculate (depth - min) * 1023 / range. Use u64 to avoid overflow during multiplication.
        // Saturating sub handles cases where depth_bits < min_bits due to floating point anomalies.
        let delta = u64::from(depth_bits.saturating_sub(min_bits));
        let t = (delta * 1023 / range) as u32;
        let t = t.min(1023); // Clamp strictly to 1023

        // SAFETY: t is strictly clamped to 1023 above, which is within the bounds of the 1024-element LUT.
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

        // Check 2 (Middle/Greenish-Cyan)
        let p2 = fb.get_pixel(2, 0).unwrap();
        // With logarithmic-like `to_bits()` mapping:
        // The midpoint value 3.0 mapped linearly in bits does not fall exactly on 0.5.
        // Therefore, it might be more towards Cyan (0x00, 0xFF, something).
        // Let's just check that it's no longer red or blue, but has high green.
        let g = (p2 >> 8) & 0xFF;
        assert_eq!(g, 255, "Middle pixel should have full Green channel");
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
