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
pub fn apply_posterize(fb: &mut Framebuffer, config: &PosterizeConfig) {
    let levels = config.levels.max(2.0); // Minimum of 2 levels
    let levels_minus_1 = levels - 1.0;

    // ⚡ Bolt: Precomputing a 256-element Look-Up Table (LUT) eliminates expensive floating-point
    // operations (.round(), division, clamping) from the hot per-pixel loop.
    let mut lut = [0u32; 256];
    for i in 0..=255 {
        let v = i as f32;
        let new_v = ((v / 255.0 * levels_minus_1).round() / levels_minus_1 * 255.0) as u32;
        lut[i] = new_v.min(255);
    }

    // Use chunks_exact_mut to eliminate bounds checking and option unwrapping
    let width = fb.width() as usize;
    for row in fb.as_mut_slice().chunks_exact_mut(width) {
        for pixel in row.iter_mut() {
            let p = *pixel;
            let a = p & 0xFF00_0000;
            let new_r = lut[((p >> 16) & 0xFF) as usize];
            let new_g = lut[((p >> 8) & 0xFF) as usize];
            let new_b = lut[(p & 0xFF) as usize];

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

        // With 2 levels (0 and 255)
        // 128 / 255 * 1.0 = 0.5019... -> round to 1.0 -> 1.0 / 1.0 * 255.0 = 255
        // 112 / 255 * 1.0 = 0.439... -> round to 0.0 -> 0.0 / 1.0 * 255.0 = 0
        // Oh, wait, 128 and 112 might round to different levels with levels=2.0.
        // Let's use colors that will snap to the same level.
        // If levels = 4 (0, 85, 170, 255):
        // 110/255 * 3 = 1.29 -> round = 1 -> 85
        // 120/255 * 3 = 1.41 -> round = 1 -> 85

        fb.set_pixel(0, 0, 0xFF6E6E6E); // 110
        fb.set_pixel(1, 0, 0xFF787878); // 120

        let config = PosterizeConfig { levels: 4.0 };
        apply_posterize(&mut fb, &config);

        let p1 = fb.get_pixel(0, 0).unwrap();
        let p2 = fb.get_pixel(1, 0).unwrap();

        assert_eq!(p1, p2, "Pixels should be quantized to the same level");
        assert_eq!(p1, 0xFF555555, "Should be quantized to exactly 85 (0x55)"); // 85 is 0x55
    }
}

#[test]
fn test_apply_posterize_alpha_preservation() {
    let mut fb = Framebuffer::new(1, 1).unwrap();
    // Set a pixel with 50% alpha (0x80)
    fb.set_pixel(0, 0, 0x80FF8080);

    let config = PosterizeConfig { levels: 4.0 };
    apply_posterize(&mut fb, &config);

    let p1 = fb.get_pixel(0, 0).unwrap();
    // Check that alpha is preserved exactly
    assert_eq!(
        p1 & 0xFF00_0000,
        0x8000_0000,
        "Alpha channel should be preserved exactly without modification"
    );
}
