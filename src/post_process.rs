//! Post-processing effects.

use crate::framebuffer::Framebuffer;

/// Applies a grayscale effect to the framebuffer.
///
/// Converts each pixel to grayscale using the standard luminance formula:
/// `Y = 0.299*R + 0.587*G + 0.114*B`
pub fn apply_grayscale(framebuffer: &mut Framebuffer) {
    for pixel in framebuffer.as_mut_slice() {
        let color = *pixel;
        let alpha = color & 0xFF000000;
        let r = (color >> 16) & 0xFF;
        let g = (color >> 8) & 0xFF;
        let b = color & 0xFF;

        // Y = 0.299*R + 0.587*G + 0.114*B
        // Using fixed point approximation: Y = (77*R + 150*G + 29*B) >> 8
        // 77/256 ≈ 0.300, 150/256 ≈ 0.586, 29/256 ≈ 0.113
        let y = (77 * r + 150 * g + 29 * b) >> 8;

        *pixel = alpha | (y << 16) | (y << 8) | y;
    }
}

/// Applies a scanline effect to the framebuffer.
///
/// Darkens every second row to simulate a CRT scanline effect.
pub fn apply_scanlines(framebuffer: &mut Framebuffer) {
    let width = framebuffer.width() as usize;
    let height = framebuffer.height() as usize;
    let pixels = framebuffer.as_mut_slice();

    for y in (1..height).step_by(2) {
        let row_start = y * width;
        let row_end = row_start + width;
        for pixel in &mut pixels[row_start..row_end] {
            let color = *pixel;
            let alpha = color & 0xFF000000;
            let mut r = (color >> 16) & 0xFF;
            let mut g = (color >> 8) & 0xFF;
            let mut b = color & 0xFF;

            r /= 2;
            g /= 2;
            b /= 2;

            *pixel = alpha | (r << 16) | (g << 8) | b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_apply_grayscale() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        // Set pixel (0, 0) to Red (0xFFFF0000)
        fb.set_pixel(0, 0, 0xFFFF0000);
        // Set pixel (1, 0) to Green (0xFF00FF00)
        fb.set_pixel(1, 0, 0xFF00FF00);
        // Set pixel (0, 1) to Blue (0xFF0000FF)
        fb.set_pixel(0, 1, 0xFF0000FF);
        // Set pixel (1, 1) to White (0xFFFFFFFF)
        fb.set_pixel(1, 1, 0xFFFFFFFF);

        apply_grayscale(&mut fb);

        // Expected grayscale values (approximate):
        // Red: 0.299 * 255 = 76.245 -> 76 (0x4C)
        // Green: 0.587 * 255 = 149.685 -> 149 (0x95)
        // Blue: 0.114 * 255 = 29.07 -> 29 (0x1D). With integer approx: 29*255>>8 = 28 (0x1C)
        // White: 255 -> 255 (0xFF)

        // Alpha should remain 0xFF

        assert_eq!(fb.get_pixel(0, 0), Some(0xFF4C4C4C));
        assert_eq!(fb.get_pixel(1, 0), Some(0xFF959595));
        assert_eq!(fb.get_pixel(0, 1), Some(0xFF1C1C1C));
        assert_eq!(fb.get_pixel(1, 1), Some(0xFFFFFFFF));
    }

    #[test]
    fn test_apply_scanlines() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        // Fill with white
        fb.clear(0xFFFFFFFF);

        apply_scanlines(&mut fb);

        // Row 0: Even row, should be unchanged (or maybe darkened? The spec said "every second row")
        // Let's assume even rows (0, 2, ...) are unchanged, and odd rows (1, 3, ...) are darkened.
        assert_eq!(fb.get_pixel(0, 0), Some(0xFFFFFFFF));
        assert_eq!(fb.get_pixel(1, 0), Some(0xFFFFFFFF));

        // Row 1: Odd row, should be darkened.
        // If we multiply by 0.5, then 255 * 0.5 = 127.5 -> 127 (0x7F)
        assert_eq!(fb.get_pixel(0, 1), Some(0xFF7F7F7F));
        assert_eq!(fb.get_pixel(1, 1), Some(0xFF7F7F7F));
    }
}
