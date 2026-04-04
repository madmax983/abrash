use crate::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Duotone effect.
#[derive(Debug, Clone, Copy)]
pub struct DuotoneConfig {
    /// The color to map pure black to (0xAARRGGBB format).
    pub color_dark: u32,
    /// The color to map pure white to (0xAARRGGBB format).
    pub color_light: u32,
}

impl Default for DuotoneConfig {
    fn default() -> Self {
        Self {
            color_dark: 0xFF000080,  // Dark Blue
            color_light: 0xFFFFA500, // Orange
        }
    }
}

/// Applies a duotone effect to the given framebuffer in-place.
pub fn apply_duotone(fb: &mut Framebuffer, config: &DuotoneConfig) {
    let dark = config.color_dark;
    let light = config.color_light;

    let d_r = ((dark >> 16) & 0xFF) as f32;
    let d_g = ((dark >> 8) & 0xFF) as f32;
    let d_b = (dark & 0xFF) as f32;

    let l_r = ((light >> 16) & 0xFF) as f32;
    let l_g = ((light >> 8) & 0xFF) as f32;
    let l_b = (light & 0xFF) as f32;

    let width = fb.width() as usize;
    if width == 0 {
        return;
    }

    // Process using parallel iterator if 'parallel' feature is enabled.
    // Use chunks_exact_mut to eliminate bounds checking.
    #[cfg(feature = "parallel")]
    let iter = fb.as_mut_slice().par_chunks_exact_mut(width);

    #[cfg(not(feature = "parallel"))]
    let iter = fb.as_mut_slice().chunks_exact_mut(width);

    iter.for_each(|row| {
        for pixel in row.iter_mut() {
            let p = *pixel;
            let a = p & 0xFF000000;
            let r = (p >> 16) & 0xFF;
            let g = (p >> 8) & 0xFF;
            let b = p & 0xFF;

            // Using fast Rec. 709 luminance
            let r_i = r as i32;
            let g_i = g as i32;
            let b_i = b as i32;
            // 13933 + 46871 + 4732 = 65536
            let luminance = (13933 * r_i + 46871 * g_i + 4732 * b_i) >> 16;

            // Normalize luminance to [0.0, 1.0] for interpolation
            let t = (luminance as f32 / 255.0).clamp(0.0, 1.0);

            // Interpolate between dark and light colors
            let new_r = (d_r + (l_r - d_r) * t).round() as u32;
            let new_g = (d_g + (l_g - d_g) * t).round() as u32;
            let new_b = (d_b + (l_b - d_b) * t).round() as u32;

            let new_r = new_r.clamp(0, 255);
            let new_g = new_g.clamp(0, 255);
            let new_b = new_b.clamp(0, 255);

            *pixel = a | (new_r << 16) | (new_g << 8) | new_b;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_apply_duotone() {
        let mut fb = Framebuffer::new(3, 1).unwrap();
        // Set pure black, pure white, and a mid-gray color
        fb.set_pixel(0, 0, 0xFF000000); // Black
        fb.set_pixel(1, 0, 0xFFFFFFFF); // White
        fb.set_pixel(2, 0, 0xFF808080); // Mid-gray

        let config = DuotoneConfig {
            color_dark: 0xFF000000,
            color_light: 0xFFFFFFFF,
        };

        // If dark is black and light is white, duotone of grays should be basically the same gray
        apply_duotone(&mut fb, &config);

        let p1 = fb.get_pixel(0, 0).unwrap();
        let p2 = fb.get_pixel(1, 0).unwrap();
        let p3 = fb.get_pixel(2, 0).unwrap();

        assert_eq!(p1, 0xFF000000, "Black should map to dark color");
        assert_eq!(p2, 0xFFFFFFFF, "White should map to light color");

        // Luminance of 0x808080 is roughly 128
        // Result should be approximately 0xFF808080
        assert_eq!(p3, 0xFF808080, "Mid-gray should map to interpolated color");
    }

    #[test]
    fn test_apply_duotone_custom_colors() {
        let mut fb = Framebuffer::new(3, 1).unwrap();
        fb.set_pixel(0, 0, 0xFF000000); // Black
        fb.set_pixel(1, 0, 0xFFFFFFFF); // White
        fb.set_pixel(2, 0, 0xFF808080); // Mid-gray

        let config = DuotoneConfig {
            color_dark: 0xFF0000FF, // Blue
            color_light: 0xFFFF0000, // Red
        };

        apply_duotone(&mut fb, &config);

        let p1 = fb.get_pixel(0, 0).unwrap();
        let p2 = fb.get_pixel(1, 0).unwrap();
        let p3 = fb.get_pixel(2, 0).unwrap();

        assert_eq!(p1, 0xFF0000FF, "Black should map to dark color");
        assert_eq!(p2, 0xFFFF0000, "White should map to light color");

        // Mid-gray (128) should map to blend of Blue and Red -> Purple
        let a = (p3 >> 24) & 0xFF;
        let r = (p3 >> 16) & 0xFF;
        let g = (p3 >> 8) & 0xFF;
        let b = p3 & 0xFF;

        assert_eq!(a, 0xFF);
        // r should be roughly 128, g 0, b roughly 127/128
        assert!(r > 120 && r < 135, "Red channel should be ~128, got {}", r);
        assert_eq!(g, 0, "Green channel should be 0");
        assert!(b > 120 && b < 135, "Blue channel should be ~128, got {}", b);
    }
}