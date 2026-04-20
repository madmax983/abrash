use crate::framebuffer::Framebuffer;

/// Configuration for the Posterize effect.
#[derive(Debug, Clone, Copy)]
pub struct PosterizeConfig {
    /// The number of color levels per channel (e.g., 2.0 to 255.0).
    /// Lower values produce a more pronounced posterized effect.
    pub levels: f32,
}

impl Default for PosterizeConfig {
    fn default() -> Self {
        Self { levels: 4.0 }
    }
}

/// Applies a posterize effect to the given framebuffer in-place.
///
/// ⚡ **Bolt**: Pre-calculating the quantized values into a 256-element Look-Up Table (LUT)
/// elides costly floating-point divisions and `f32::round()` operations from the hot
/// per-pixel inner loop. Additionally, masking the alpha channel instead of shifting
/// extracts it efficiently.
pub fn apply_posterize(fb: &mut Framebuffer, config: &PosterizeConfig) {
    let levels = config.levels.max(2.0); // Minimum of 2 levels
    let levels_minus_1 = levels - 1.0;

    // Precalculate LUT
    let mut lut = [0u32; 256];
    for (i, val) in lut.iter_mut().enumerate() {
        let quantized =
            ((i as f32 / 255.0 * levels_minus_1).round() / levels_minus_1 * 255.0) as u32;
        *val = quantized.min(255);
    }

    // Use chunks_exact_mut to eliminate bounds checking and option unwrapping
    let width = fb.width() as usize;
    for row in fb.as_mut_slice().chunks_exact_mut(width) {
        for pixel in row.iter_mut() {
            let p = *pixel;
            // ⚡ Bolt: Fast alpha mask preservation
            let a = p & 0xFF00_0000;
            let r = ((p >> 16) & 0xFF) as usize;
            let g = ((p >> 8) & 0xFF) as usize;
            let b = (p & 0xFF) as usize;

            let new_r = lut[r];
            let new_g = lut[g];
            let new_b = lut[b];

            *pixel = a | (new_r << 16) | (new_g << 8) | new_b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_apply_posterize_reduces_colors() {
        let mut fb = Framebuffer::new(2, 1).unwrap();
        // Set two distinct bright pixels (near middle gray)
        fb.set_pixel(0, 0, 0xFF808080); // Mid-gray (128)
        fb.set_pixel(1, 0, 0xFF707070); // Slightly darker gray (112)

        let config = PosterizeConfig { levels: 2.0 };
        apply_posterize(&mut fb, &config);

        fb.set_pixel(0, 0, 0xFF6E6E6E); // 110
        fb.set_pixel(1, 0, 0xFF787878); // 120

        let config = PosterizeConfig { levels: 4.0 };
        apply_posterize(&mut fb, &config);

        let p1 = fb.get_pixel(0, 0).unwrap();
        let p2 = fb.get_pixel(1, 0).unwrap();

        assert_eq!(p1, p2, "Pixels should be quantized to the same level");
        assert_eq!(p1, 0xFF555555, "Should be quantized to exactly 85 (0x55)"); // 85 is 0x55
    }

    #[test]
    fn test_apply_posterize_preserves_alpha() {
        let mut fb = Framebuffer::new(1, 1).unwrap();
        fb.set_pixel(0, 0, 0x80123456); // Alpha is 0x80 (128)

        let config = PosterizeConfig { levels: 2.0 };
        apply_posterize(&mut fb, &config);

        let p1 = fb.get_pixel(0, 0).unwrap();
        assert_eq!(
            p1 & 0xFF00_0000,
            0x8000_0000,
            "Alpha channel should be preserved exactly"
        );
    }
}
