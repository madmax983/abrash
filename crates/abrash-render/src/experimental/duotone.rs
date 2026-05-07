//! Duotone processing for stylistic color remapping.
//!
//! This module provides the [`apply_duotone`] function, which maps the grayscale
//! luminance of an image into a two-color gradient space, similar to the iconic
//! "Spotify" duotone visual effect.

use abrash_core::framebuffer::Framebuffer;

/// Applies a duotone color remapping effect to the given [`Framebuffer`].
///
/// Converts each pixel to its relative luminance and remaps it using a linear interpolation
/// between `color1` (representing darkest areas) and `color2` (representing lightest areas).
/// The alpha channel of the original pixel is preserved.
///
/// # Examples
///
/// ```
/// use abrash_core::framebuffer::Framebuffer;
/// use abrash_render::experimental::duotone::apply_duotone;
///
/// let mut fb = Framebuffer::new(3, 1).unwrap();
///
/// // Setup three pixels: black, mid-gray, and white
/// fb.set_pixel(0, 0, 0xFF_000000);
/// fb.set_pixel(1, 0, 0xFF_808080);
/// fb.set_pixel(2, 0, 0xFF_FFFFFF);
///
/// // Map the darkest colors to Red and the lightest colors to Blue
/// let dark_color = 0xFF_FF0000;
/// let light_color = 0xFF_0000FF;
///
/// apply_duotone(&mut fb, dark_color, light_color);
///
/// // The black pixel becomes Red
/// assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF_FF0000);
/// // The white pixel becomes Blue
/// assert_eq!(fb.get_pixel(2, 0).unwrap(), 0xFF_0000FF);
/// // The gray pixel becomes a blended purple (Red + Blue)
/// let blended = fb.get_pixel(1, 0).unwrap();
/// assert_eq!(blended, 0xFF_7F0080);
/// ```
pub fn apply_duotone(fb: &mut Framebuffer, color1: u32, color2: u32) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let pixels = fb.as_mut_slice();

    let r1 = (color1 >> 16) & 0xFF;
    let g1 = (color1 >> 8) & 0xFF;
    let b1 = color1 & 0xFF;

    let r2 = (color2 >> 16) & 0xFF;
    let g2 = (color2 >> 8) & 0xFF;
    let b2 = color2 & 0xFF;

    for row in pixels.chunks_exact_mut(width).take(height) {
        for pixel in row.iter_mut() {
            let color = *pixel;

            let r = (color >> 16) & 0xFF;
            let g = (color >> 8) & 0xFF;
            let b = color & 0xFF;

            // Simple relative luminance
            let lum = (r * 299 + g * 587 + b * 114) / 1000;

            let out_r = (r1 as i32 + (r2 as i32 - r1 as i32) * (lum as i32) / 255) as u32;
            let out_g = (g1 as i32 + (g2 as i32 - g1 as i32) * (lum as i32) / 255) as u32;
            let out_b = (b1 as i32 + (b2 as i32 - b1 as i32) * (lum as i32) / 255) as u32;

            *pixel = (color & 0xFF00_0000) | (out_r << 16) | (out_g << 8) | out_b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_duotone() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        // Set pixels to black, gray, white
        fb.set_pixel(0, 0, 0xFF00_0000); // Black
        fb.set_pixel(1, 0, 0xFF80_8080); // Mid Gray
        fb.set_pixel(0, 1, 0xFFFF_FFFF); // White

        let color1 = 0xFFFF_0000; // Red (mapped to darkest)
        let color2 = 0xFF0000FF; // Blue (mapped to lightest)

        apply_duotone(&mut fb, color1, color2);

        assert_eq!(fb.get_pixel(0, 0).unwrap(), color1);
        assert_eq!(fb.get_pixel(0, 1).unwrap(), color2);
        // Gray should be a mix of Red and Blue (approx Purple)
        let mid = fb.get_pixel(1, 0).unwrap();
        assert!(mid != 0xFF80_8080, "Pixel was not modified");
        assert_eq!(mid, 0xFF7F0080); // Expected mix
    }
}
