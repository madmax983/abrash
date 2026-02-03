//! Post-processing effects for the framebuffer.

use crate::framebuffer::Framebuffer;

/// Apply grayscale effect to the framebuffer.
/// Uses the standard luma coefficients: 0.2126 R + 0.7152 G + 0.0722 B
pub fn apply_grayscale(fb: &mut Framebuffer) {
    let pixels = fb.as_mut_slice();
    for pixel in pixels.iter_mut() {
        let p = *pixel;
        let a = (p >> 24) & 0xFF;
        let r = (p >> 16) & 0xFF;
        let g = (p >> 8) & 0xFF;
        let b = p & 0xFF;

        let luma = (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32) as u32;

        *pixel = (a << 24) | (luma << 16) | (luma << 8) | luma;
    }
}

/// Apply scanline effect to the framebuffer.
/// Darkens every other row by the given intensity factor (0.0 to 1.0).
pub fn apply_scanlines(fb: &mut Framebuffer, intensity: f32) {
    let width = fb.width();
    let height = fb.height();
    let pixels = fb.as_mut_slice();
    let factor = 1.0 - intensity.clamp(0.0, 1.0);

    for y in 0..height {
        if y % 2 != 0 {
            let start = (y * width) as usize;
            let end = start + width as usize;
            // Bounds check just in case, though it should be safe
            if start < pixels.len() && end <= pixels.len() {
                for pixel in &mut pixels[start..end] {
                    let p = *pixel;
                    let a = (p >> 24) & 0xFF;
                    let r = (p >> 16) & 0xFF;
                    let g = (p >> 8) & 0xFF;
                    let b = p & 0xFF;

                    let r_new = (r as f32 * factor) as u32;
                    let g_new = (g as f32 * factor) as u32;
                    let b_new = (b as f32 * factor) as u32;

                    *pixel = (a << 24) | (r_new << 16) | (g_new << 8) | b_new;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grayscale() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        // Set pixel (0,0) to Red (AA RR GG BB) -> 0xFF FF 00 00
        fb.set_pixel(0, 0, 0xFFFF0000);

        apply_grayscale(&mut fb);

        let pixel = fb.get_pixel(0, 0).unwrap();
        // Red component is 255. Grayscale should be roughly 0.2126 * 255 = 54.
        // So R=54, G=54, B=54.
        let r = (pixel >> 16) & 0xFF;
        let g = (pixel >> 8) & 0xFF;
        let b = pixel & 0xFF;

        assert_eq!(r, g, "Red and Green should be equal");
        assert_eq!(g, b, "Green and Blue should be equal");
        assert!(r > 50 && r < 60, "Value should be around 54");
    }

    #[test]
    fn test_scanlines() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        fb.clear(0xFFFFFFFF); // White

        apply_scanlines(&mut fb, 0.5);

        // Row 0 should be untouched (or darkened depending on logic)
        // We darken odd lines (index 1).
        let p0 = fb.get_pixel(0, 0).unwrap() & 0xFF; // Blue component
        let p1 = fb.get_pixel(0, 1).unwrap() & 0xFF; // Blue component

        // p0 should be 255. p1 should be 255 * 0.5 = 127.
        assert_eq!(p0, 255);
        assert!(p1 < p0, "Row 1 should be darker than Row 0");
        assert!(p1 > 120 && p1 < 135, "Row 1 should be around 127");
    }
}
