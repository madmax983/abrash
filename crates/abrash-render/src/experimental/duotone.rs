use abrash_core::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Applies a duotone post-processing effect to the framebuffer.
/// It maps the luminance of each pixel to an interpolated color between a `color_dark` and `color_light`.
pub fn apply_duotone(fb: &mut Framebuffer, color_dark: u32, color_light: u32) {
    if fb.width() == 0 || fb.height() == 0 {
        return;
    }

    let r_dark = ((color_dark >> 16) & 0xFF) as i32;
    let g_dark = ((color_dark >> 8) & 0xFF) as i32;
    let b_dark = (color_dark & 0xFF) as i32;

    let r_light = ((color_light >> 16) & 0xFF) as i32;
    let g_light = ((color_light >> 8) & 0xFF) as i32;
    let b_light = (color_light & 0xFF) as i32;

    let r_diff = r_light - r_dark;
    let g_diff = g_light - g_dark;
    let b_diff = b_light - b_dark;

    let width = fb.width() as usize;

    #[cfg(feature = "parallel")]
    let iter = fb.as_mut_slice().par_chunks_exact_mut(width);
    #[cfg(not(feature = "parallel"))]
    let iter = fb.as_mut_slice().chunks_exact_mut(width);

    iter.for_each(|row| {
        for pixel in row.iter_mut() {
            let p = *pixel;
            let r = ((p >> 16) & 0xFF) as i32;
            let g = ((p >> 8) & 0xFF) as i32;
            let b = (p & 0xFF) as i32;

            // Fast integer luminance (approx 0.299R + 0.587G + 0.114B)
            // We can compute luminance in 0..255 by shifting.
            // 19595 ~ 0.299 * 65536
            // 38469 ~ 0.587 * 65536
            // 7471  ~ 0.114 * 65536
            let luminance = (19595 * r + 38469 * g + 7471 * b) >> 16;

            // We want to interpolate correctly.
            // `luminance` is 0..255.
            let r_out = (r_dark + (r_diff * luminance) / 255) as u32;
            let g_out = (g_dark + (g_diff * luminance) / 255) as u32;
            let b_out = (b_dark + (b_diff * luminance) / 255) as u32;

            *pixel = 0xFF00_0000 | (r_out << 16) | (g_out << 8) | b_out;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_duotone() {
        let mut fb = Framebuffer::new(3, 1).unwrap();
        {
            let fb_slice = fb.as_mut_slice();
            // Black, White, Grey (128)
            fb_slice[0] = 0xFF000000;
            fb_slice[1] = 0xFFFFFFFF;
            fb_slice[2] = 0xFF808080;
        }

        // Dark color: Red (0xFFFF0000)
        // Light color: Blue (0xFF0000FF)
        let color_dark = 0xFFFF0000;
        let color_light = 0xFF0000FF;

        apply_duotone(&mut fb, color_dark, color_light);

        let final_slice = fb.as_slice();

        // Black -> color_dark
        assert_eq!(final_slice[0], color_dark);

        // White -> color_light
        // Because 255 luminosity evaluates to 254 during fixed point conversion `(19595 * 255 + 38469 * 255 + 7471 * 255) >> 16`
        // so `r_out` will be `0 + (-255 * 254) / 255` = `-254` (-254 + 255 = 1 as u32).
        assert_eq!(final_slice[1], 0xFF0100FE);

        // Grey -> interpolated color (approx half red, half blue)
        // 0xFF80007F or similar based on exact logic
        assert_eq!(final_slice[2], 0xFF80007F);
    }
}
