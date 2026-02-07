//! Post-processing effects for the framebuffer.
//!
//! This module provides screen-space effects like fog and CRT filters.

use crate::framebuffer::Framebuffer;
use crate::zbuffer::ZBuffer;

/// Applies depth-based fog to the framebuffer.
///
/// Pixels with depth < `start_depth` are unaffected.
/// Pixels with depth > `end_depth` are fully fogged.
/// Intermediate depths are linearly interpolated.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify.
/// * `zb` - The z-buffer containing depth values.
/// * `fog_color` - The color of the fog (ARGB).
/// * `start_depth` - The depth at which fog starts (0.0 = near, 1.0 = far).
/// * `end_depth` - The depth at which fog is fully opaque.
pub fn apply_depth_fog(
    fb: &mut Framebuffer,
    zb: &ZBuffer,
    fog_color: u32,
    start_depth: f32,
    end_depth: f32,
) {
    if fb.width() != zb.width() || fb.height() != zb.height() {
        return; // Mismatched dimensions
    }

    let fog_r = ((fog_color >> 16) & 0xFF) as f32;
    let fog_g = ((fog_color >> 8) & 0xFF) as f32;
    let fog_b = (fog_color & 0xFF) as f32;

    let pixels = fb.as_mut_slice();
    let depths = zb.as_slice();

    let inv_range = if (end_depth - start_depth).abs() > 0.0001 {
        1.0 / (end_depth - start_depth)
    } else {
        0.0
    };

    for (pixel, &depth) in pixels.iter_mut().zip(depths.iter()) {
        if depth == f32::INFINITY {
            // Background is infinite depth.
            // If we want fog to affect background, we treat it as max depth.
            // Let's assume background color is handled by clear_color,
            // but fog should blend it towards fog_color if end_depth < infinity.
            // However, typical fog usage: clear color IS fog color.
            // If clear color != fog color, we should blend.
            // Let's treat infinity as > end_depth, so full fog.
            *pixel = fog_color;
            continue;
        }

        if depth <= start_depth {
            continue;
        }

        let factor = if depth >= end_depth {
            1.0
        } else {
            (depth - start_depth) * inv_range
        };

        let r = ((*pixel >> 16) & 0xFF) as f32;
        let g = ((*pixel >> 8) & 0xFF) as f32;
        let b = (*pixel & 0xFF) as f32;

        let out_r = r + (fog_r - r) * factor;
        let out_g = g + (fog_g - g) * factor;
        let out_b = b + (fog_b - b) * factor;

        *pixel = 0xFF00_0000
            | ((out_r as u32) << 16)
            | ((out_g as u32) << 8)
            | (out_b as u32);
    }
}

/// Applies a CRT-like scanline filter.
///
/// Darkens every other horizontal line to simulate a retro display.
pub fn apply_crt_filter(fb: &mut Framebuffer) {
    let width = fb.width();
    let height = fb.height();
    let pixels = fb.as_mut_slice();

    for y in 0..height {
        if y % 2 == 0 {
            continue;
        }

        let row_start = (y * width) as usize;
        let row_end = row_start + width as usize;

        for pixel in &mut pixels[row_start..row_end] {
            // Darken by 50%
            let r = ((*pixel >> 16) & 0xFF) >> 1;
            let g = ((*pixel >> 8) & 0xFF) >> 1;
            let b = (*pixel & 0xFF) >> 1;

            *pixel = 0xFF00_0000 | (r << 16) | (g << 8) | b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;
    use crate::zbuffer::ZBuffer;

    #[test]
    fn test_fog_application() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        let mut zb = ZBuffer::new(2, 2).unwrap();

        // Pixel 0: Near (no fog)
        fb.set_pixel(0, 0, 0xFFFFFFFF); // White
        zb.test_and_set(0, 0, 0.0);

        // Pixel 1: Mid (50% fog)
        fb.set_pixel(1, 0, 0xFFFFFFFF); // White
        zb.test_and_set(1, 0, 0.5);

        // Pixel 2: Far (100% fog)
        fb.set_pixel(0, 1, 0xFFFFFFFF); // White
        zb.test_and_set(0, 1, 1.0);

        // Pixel 3: Infinite (Background) -> Full Fog
        fb.set_pixel(1, 1, 0xFF000000); // Black background
        // Depth remains INFINITY

        let fog_color = 0xFF0000FF; // Blue

        apply_depth_fog(&mut fb, &zb, fog_color, 0.0, 1.0);

        // Check Pixel 0: Should be White (start_depth=0.0, depth=0.0 -> factor 0)
        // Wait, depth <= start_depth (0.0 <= 0.0) -> continue.
        assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFFFFFFFF);

        // Check Pixel 1: Depth 0.5 -> factor 0.5. White (255) -> Blue (0,0,255)
        // R: 255 + (0-255)*0.5 = 127.5 -> 127
        // G: 255 + (0-255)*0.5 = 127.5 -> 127
        // B: 255 + (255-255)*0.5 = 255
        // Expect 0xFF7F7FFF
        let p1 = fb.get_pixel(1, 0).unwrap();
        assert_eq!(p1 & 0xFF000000, 0xFF000000);
        assert!(((p1 >> 16) & 0xFF).abs_diff(127) <= 1);
        assert!(((p1 >> 8) & 0xFF).abs_diff(127) <= 1);
        assert_eq!(p1 & 0xFF, 255);

        // Check Pixel 2: Depth 1.0 -> factor 1.0. Full Blue.
        assert_eq!(fb.get_pixel(0, 1).unwrap(), 0xFF0000FF);

        // Check Pixel 3: Infinity -> Full Blue
        assert_eq!(fb.get_pixel(1, 1).unwrap(), 0xFF0000FF);
    }

    #[test]
    fn test_crt_filter() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        fb.clear(0xFFFFFFFF); // All White

        apply_crt_filter(&mut fb);

        // Row 0 (y=0): Unchanged
        assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFFFFFFFF);
        assert_eq!(fb.get_pixel(1, 0).unwrap(), 0xFFFFFFFF);

        // Row 1 (y=1): Darkened (50%)
        // 255 >> 1 = 127
        // Expect 0xFF7F7F7F
        assert_eq!(fb.get_pixel(0, 1).unwrap(), 0xFF7F7F7F);
        assert_eq!(fb.get_pixel(1, 1).unwrap(), 0xFF7F7F7F);
    }
}
