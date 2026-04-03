use crate::framebuffer::Framebuffer;

pub struct Duotone {
    pub color_dark: u32,
    pub color_light: u32,
}

impl Duotone {
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(color_dark: u32, color_light: u32) -> Self {
        Self {
            color_dark,
            color_light,
        }
    }

    pub fn apply(&self, fb: &mut Framebuffer) {
        #[cfg(feature = "parallel")]
        use rayon::prelude::*;

        let width = fb.width() as usize;
        let height = fb.height() as usize;
        if width == 0 || height == 0 {
            return;
        }

        let dr = (self.color_dark >> 16) & 0xFF;
        let dg = (self.color_dark >> 8) & 0xFF;
        let db = self.color_dark & 0xFF;
        let da = (self.color_dark >> 24) & 0xFF;

        let lr = (self.color_light >> 16) & 0xFF;
        let lg = (self.color_light >> 8) & 0xFF;
        let lb = self.color_light & 0xFF;
        let la = (self.color_light >> 24) & 0xFF;

        #[cfg(not(feature = "parallel"))]
        let iter = fb.as_mut_slice().chunks_exact_mut(width).take(height);

        #[cfg(feature = "parallel")]
        let iter = fb.as_mut_slice().par_chunks_exact_mut(width).take(height);

        iter.for_each(|row| {
            for pixel in row.iter_mut() {
                let p = *pixel;
                let pr = (p >> 16) & 0xFF;
                let pg = (p >> 8) & 0xFF;
                let pb = p & 0xFF;

                // Rec. 709 luminance
                let lum = (pr * 2126 + pg * 7152 + pb * 722) / 10000;

                // Mix dark and light based on luminance (0-255)
                let r = (dr as i32 + ((lr as i32 - dr as i32) * lum as i32 / 255)) as u32;
                let g = (dg as i32 + ((lg as i32 - dg as i32) * lum as i32 / 255)) as u32;
                let b = (db as i32 + ((lb as i32 - db as i32) * lum as i32 / 255)) as u32;
                let a = (da as i32 + ((la as i32 - da as i32) * lum as i32 / 255)) as u32;

                *pixel = (a << 24) | (r << 16) | (g << 8) | b;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duotone_applies_correct_colors() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        // Set pixels with different luminance values
        // Black (Luminance ~0)
        fb.set_pixel(0, 0, 0xFF00_0000);
        // White (Luminance ~255)
        fb.set_pixel(1, 0, 0xFFFF_FFFF);
        // Gray (Luminance ~127)
        fb.set_pixel(0, 1, 0xFF80_8080);

        let filter = Duotone::new(0xFF00_00FF, 0xFFFF_0000); // Dark -> Blue, Light -> Red
        filter.apply(&mut fb);

        // Verify colors are mapped
        assert_eq!(
            fb.get_pixel(0, 0).unwrap(),
            0xFF00_00FF,
            "Black should map to dark color"
        );
        assert_eq!(
            fb.get_pixel(1, 0).unwrap(),
            0xFFFF_0000,
            "White should map to light color"
        );

        let gray_pixel = fb.get_pixel(0, 1).unwrap();
        assert_ne!(
            gray_pixel, 0xFF80_8080,
            "Gray should be mapped to a mixed color"
        );
    }
}
