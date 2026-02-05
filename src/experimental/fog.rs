use crate::framebuffer::Framebuffer;
use crate::zbuffer::ZBuffer;

/// Applies depth-based fog to the framebuffer using the z-buffer.
///
/// Pixels with depth <= start_depth are unaffected.
/// Pixels with depth >= end_depth are fully fogged (replaced by fog_color).
/// Pixels in between are linearly interpolated.
pub fn apply_depth_fog(
    fb: &mut Framebuffer,
    zb: &ZBuffer,
    fog_color: u32,
    start_depth: f32,
    end_depth: f32,
) {
    if fb.width() != zb.width() || fb.height() != zb.height() {
        return;
    }

    let pixels = fb.as_mut_slice();
    let depths = zb.as_slice();

    let fog_r = ((fog_color >> 16) & 0xFF) as f32;
    let fog_g = ((fog_color >> 8) & 0xFF) as f32;
    let fog_b = (fog_color & 0xFF) as f32;

    let range = end_depth - start_depth;
    let inv_range = if range.abs() > 0.00001 {
        1.0 / range
    } else {
        0.0
    };

    for (pixel, &depth) in pixels.iter_mut().zip(depths.iter()) {
        let factor = if depth.is_infinite() {
            1.0
        } else if depth <= start_depth {
            0.0
        } else if depth >= end_depth {
            1.0
        } else {
            (depth - start_depth) * inv_range
        };

        if factor <= 0.0 {
            continue;
        }

        if factor >= 1.0 {
            *pixel = fog_color;
            continue;
        }

        let p = *pixel;
        let r = ((p >> 16) & 0xFF) as f32;
        let g = ((p >> 8) & 0xFF) as f32;
        let b = (p & 0xFF) as f32;

        let inv_f = 1.0 - factor;

        let new_r = (r * inv_f + fog_r * factor) as u32;
        let new_g = (g * inv_f + fog_g * factor) as u32;
        let new_b = (b * inv_f + fog_b * factor) as u32;

        *pixel = 0xFF000000 | (new_r << 16) | (new_g << 8) | new_b;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fog_application() {
        let width = 2;
        let height = 2;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        // Fill with red
        fb.clear(0xFFFF0000);
        // Clear Z-buffer (infinity)
        zb.clear();

        // Set specific depths
        // (0,0): Close (0.0) -> Should be red (unaffected)
        zb.test_and_set(0, 0, 0.0);

        // (1,0): Far (1.0) -> Should be blue (fully fogged)
        zb.test_and_set(1, 0, 1.0);

        // (0,1): Middle (0.5) -> Should be purple (blend)
        zb.test_and_set(0, 1, 0.5);

        // (1,1): Infinity (background) -> Should be blue (fully fogged)
        // (already set by clear)

        let fog_color = 0xFF0000FF; // Blue
        apply_depth_fog(&mut fb, &zb, fog_color, 0.0, 1.0);

        // Check (0,0) - Red
        let p00 = fb.get_pixel(0, 0).unwrap();
        assert_eq!(p00, 0xFFFF0000, "Pixel at depth 0 should be original color");

        // Check (1,0) - Blue
        let p10 = fb.get_pixel(1, 0).unwrap();
        assert_eq!(p10, 0xFF0000FF, "Pixel at depth 1 should be fog color");

        // Check (0,1) - Blend (Red + Blue) / 2 = 0xFF 7F 00 7F roughly
        let p01 = fb.get_pixel(0, 1).unwrap();
        let r = (p01 >> 16) & 0xFF;
        let b = p01 & 0xFF;

        // Allow for some rounding error
        assert!(
            r > 100 && r < 155,
            "Red component {} should be blended (expected ~127)",
            r
        );
        assert!(
            b > 100 && b < 155,
            "Blue component {} should be blended (expected ~127)",
            b
        );

        // Check (1,1) - Infinity -> Blue
        let p11 = fb.get_pixel(1, 1).unwrap();
        assert_eq!(
            p11, 0xFF0000FF,
            "Background (infinite depth) should be fully fogged"
        );
    }

    #[test]
    fn test_fog_range() {
        let width = 1;
        let height = 1;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        fb.clear(0xFFFFFFFF); // White
        zb.test_and_set(0, 0, 5.0); // Depth 5.0

        // Fog starts at 10.0, ends at 20.0
        // Depth 5.0 is < 10.0, so no fog.
        apply_depth_fog(&mut fb, &zb, 0xFF000000, 10.0, 20.0);
        assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFFFFFFFF);

        // Fog starts at 0.0, ends at 4.0
        // Depth 5.0 is > 4.0, so full fog.
        apply_depth_fog(&mut fb, &zb, 0xFF000000, 0.0, 4.0);
        assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF000000);
    }
}
