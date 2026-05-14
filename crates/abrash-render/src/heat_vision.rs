//! Heat Vision Effect
//!
//! Maps the depth buffer (Z-buffer) to a color gradient, simulating a thermal camera.
//! Uses auto-ranging to adapt to the scene depth.

use crate::framebuffer::Framebuffer;
use crate::zbuffer::ZBuffer;

const fn generate_heat_lut() -> [u32; 1024] {
    let mut lut = [0; 1024];
    let mut i = 0;
    while i < 1024 {
        let (r, g, b) = if i < 256 {
            (255, i as u32, 0)
        } else if i < 512 {
            (255 - (i as u32 - 256), 255, 0)
        } else if i < 768 {
            (0, 255, i as u32 - 512)
        } else {
            (0, 255 - (i as u32 - 768), 255)
        };
        lut[i] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
        i += 1;
    }
    lut
}

const HEAT_LUT: [u32; 1024] = generate_heat_lut();

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

    for (pixel, &depth) in pixels.iter_mut().zip(depths.iter()) {
        if depth == f32::INFINITY {
            *pixel = 0xFF00_0010; // Very Dark Blue Background
            continue;
        }

        let t = ((depth - min_z) * scale) as usize;
        let t = t.min(1023); // Clamp strictly to 1023

        *pixel = HEAT_LUT[t];
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
