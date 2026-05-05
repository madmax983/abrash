//! Gradient Map Filter
//!
//! A post-processing effect that maps the grayscale luminance of an image into
//! colors sampled from an arbitrary color gradient.

#![cfg(feature = "nova")]

use abrash_core::framebuffer::Framebuffer;
use abrash_core::gradient::Gradient;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Applies a gradient map color remapping effect to the given [`Framebuffer`].
///
/// Converts each pixel to its relative luminance and maps that value directly
/// into a lookup table pre-sampled from the provided `gradient`.
/// The alpha channel of the original pixel is preserved.
///
/// # Examples
///
/// ```
/// use abrash_core::framebuffer::Framebuffer;
/// use abrash_core::gradient::Gradient;
/// use abrash_render::experimental::gradient_map::apply_gradient_map;
///
/// let mut fb = Framebuffer::new(3, 1).unwrap();
/// fb.set_pixel(0, 0, 0xFF_000000);
/// fb.set_pixel(1, 0, 0xFF_808080);
/// fb.set_pixel(2, 0, 0xFF_FFFFFF);
///
/// let gradient = Gradient::heat();
/// apply_gradient_map(&mut fb, &gradient);
/// ```
pub fn apply_gradient_map(fb: &mut Framebuffer, gradient: &Gradient) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    // Pre-calculate the gradient look-up table. We sample 256 colors
    // to map from the 8-bit luminance [0..255] space.
    let mut lut = [0u32; 256];
    for i in 0..256 {
        let t = i as f32 / 255.0;
        lut[i] = gradient.sample(t).to_argb_u32();
    }

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let iter = pixels.par_chunks_exact_mut(width);

    #[cfg(not(feature = "parallel"))]
    let iter = pixels.chunks_exact_mut(width);

    iter.for_each(|row| {
        for pixel in row.iter_mut() {
            let color = *pixel;

            let r = (color >> 16) & 0xFF;
            let g = (color >> 8) & 0xFF;
            let b = color & 0xFF;

            // Simple relative luminance
            let lum = ((r * 299 + g * 587 + b * 114) / 1000) as usize;

            // Clamp to 255 to avoid out-of-bounds just in case
            let lum_clamped = lum.min(255);

            let mapped_color = lut[lum_clamped];

            // Combine original alpha with mapped color RGB
            *pixel = (color & 0xFF00_0000) | (mapped_color & 0x00FF_FFFF);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::color::Color;

    #[test]
    fn test_apply_gradient_map() {
        let mut fb = Framebuffer::new(3, 1).unwrap();
        // Set pixels to black, gray, white
        fb.set_pixel(0, 0, 0xFF_000000); // Black
        fb.set_pixel(1, 0, 0xFF_808080); // Mid Gray
        fb.set_pixel(2, 0, 0xFF_FFFFFF); // White

        // Map black to Red, white to Blue
        let gradient = Gradient::new([(0.0, Color::RED), (1.0, Color::BLUE)]);

        apply_gradient_map(&mut fb, &gradient);

        // Black maps to Red
        let c0 = fb.get_pixel(0, 0).unwrap();
        assert_eq!(c0, 0xFF_FF0000);

        // White maps to Blue
        let c2 = fb.get_pixel(2, 0).unwrap();
        assert_eq!(c2, 0xFF_0000FF);

        // Mid Gray maps to somewhere in between (approx purple)
        let c1 = fb.get_pixel(1, 0).unwrap();
        let r = (c1 >> 16) & 0xFF;
        let b = c1 & 0xFF;
        assert!(r > 0 && r < 255);
        assert!(b > 0 && b < 255);
    }
}
