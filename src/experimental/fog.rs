//! Experimental depth fog effect.

use crate::framebuffer::Framebuffer;
use crate::zbuffer::ZBuffer;

/// Applies a depth-based fog effect to the framebuffer.
///
/// Pixels with depth values between `fog_start` and `fog_end` are blended with `fog_color`.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify.
/// * `zb` - The z-buffer containing depth information.
/// * `fog_color` - The color of the fog (0xAARRGGBB).
/// * `fog_start` - The depth value where fog begins to appear (factor 0.0).
/// * `fog_end` - The depth value where fog becomes fully opaque (factor 1.0).
///
/// # Notes
///
/// The `fog_start` and `fog_end` values should be in the same coordinate space as the Z-buffer values.
/// For standard perspective projection, this is usually Normalized Device Coordinates (NDC).
pub fn apply_depth_fog(
    fb: &mut Framebuffer,
    zb: &ZBuffer,
    fog_color: u32,
    fog_start: f32,
    fog_end: f32,
) {
    if fb.width() != zb.width() || fb.height() != zb.height() {
        return;
    }

    let pixels = fb.as_mut_slice();
    let depths = zb.as_slice();

    // Pre-extract fog components
    // We ignore fog alpha for blending logic, effectively assuming fog is opaque volume,
    // but we preserve pixel alpha.
    let f_r = ((fog_color >> 16) & 0xFF) as f32;
    let f_g = ((fog_color >> 8) & 0xFF) as f32;
    let f_b = (fog_color & 0xFF) as f32;

    let dist = fog_end - fog_start;
    let range_inv = if dist.abs() > 0.00001 {
        1.0 / dist
    } else {
        0.0
    };

    for (pixel, &depth) in pixels.iter_mut().zip(depths.iter()) {
        // Skip invalid depths if necessary? No, logic handles it.

        let factor = if depth <= fog_start {
            0.0
        } else if depth >= fog_end {
            1.0
        } else {
            (depth - fog_start) * range_inv
        };

        if factor <= 0.0 {
            continue;
        }

        if factor >= 1.0 {
            // Fully fogged
            // Preserve original alpha? Or use fog alpha?
            // Usually we want to hide the object completely, so we set it to fog color.
            // But we keep the alpha of the original pixel?
            // If the pixel was transparent (0 alpha), it remains transparent?
            // If we are drawing over a background, the background is cleared to a color.
            // Let's preserve the original alpha to be safe.
            let p_a = *pixel & 0xFF000000;
            *pixel = p_a | (fog_color & 0x00FFFFFF);
            continue;
        }

        // Blend
        let p = *pixel;
        let p_a = p & 0xFF000000;
        let p_r = ((p >> 16) & 0xFF) as f32;
        let p_g = ((p >> 8) & 0xFF) as f32;
        let p_b = (p & 0xFF) as f32;

        let inv_f = 1.0 - factor;

        let r = (p_r * inv_f + f_r * factor) as u32;
        let g = (p_g * inv_f + f_g * factor) as u32;
        let b = (p_b * inv_f + f_b * factor) as u32;

        *pixel = p_a | (r << 16) | (g << 8) | b;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_depth_fog() {
        let width = 2;
        let height = 1;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        // Pixel 0: Close (depth 0.0), RED
        fb.set_pixel(0, 0, 0xFFFF0000); // Red
        zb.test_and_set(0, 0, 0.0);

        // Pixel 1: Far (depth 1.0), RED
        fb.set_pixel(1, 0, 0xFFFF0000); // Red
        zb.test_and_set(1, 0, 1.0);

        // Fog: Blue, start 0.2, end 0.8
        // Pixel 0 (0.0) < 0.2 -> Should stay Red
        // Pixel 1 (1.0) > 0.8 -> Should become Blue

        let fog_color = 0xFF0000FF; // Blue
        apply_depth_fog(&mut fb, &zb, fog_color, 0.2, 0.8);

        // Check Pixel 0
        let p0 = fb.get_pixel(0, 0).unwrap();
        assert_eq!(p0, 0xFFFF0000, "Pixel 0 should remain Red");

        // Check Pixel 1
        let p1 = fb.get_pixel(1, 0).unwrap();
        // Should be Blue
        assert_eq!(p1 & 0xFFFFFF, 0x0000FF, "Pixel 1 should become Blue");
    }

    #[test]
    fn test_fog_gradient() {
        let width = 1;
        let height = 1;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        // Pixel: depth 0.5. Range 0.0 - 1.0. Factor should be 0.5.
        // Color: Black (0xFF000000). Fog: White (0xFFFFFFFF). Result: Gray (0xFF7F7F7F)
        fb.set_pixel(0, 0, 0xFF000000);
        zb.test_and_set(0, 0, 0.5);

        let fog_color = 0xFFFFFFFF;
        apply_depth_fog(&mut fb, &zb, fog_color, 0.0, 1.0);

        let p = fb.get_pixel(0, 0).unwrap();
        let r = (p >> 16) & 0xFF;
        assert!(
            (126..=129).contains(&r),
            "Pixel should be roughly 127/128 (Gray), got {}",
            r
        );
    }
}
