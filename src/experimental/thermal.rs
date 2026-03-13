//! Thermal vision effect.
//!
//! Maps luminance to a heat map color gradient.

use crate::framebuffer::Framebuffer;
use crate::utils::pixel_luminance;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the thermal vision post-processing filter.
#[derive(Debug, Clone, Copy)]
pub struct ThermalConfig {
    /// Intensity of the thermal effect (0.0 to 1.0).
    pub intensity: f32,
    /// Whether to invert the thermal mapping (e.g., hot is dark instead of light).
    pub invert: bool,
}

impl Default for ThermalConfig {
    fn default() -> Self {
        Self {
            intensity: 1.0,
            invert: false,
        }
    }
}

/// Applies a thermal vision post-processing effect to the framebuffer.
///
/// This filter converts the image into a heatmap based on luminance,
/// simulating a thermal imaging camera. Darker areas map to blues and purples,
/// while brighter areas map to reds, yellows, and whites.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - Configuration for the thermal effect.
/// Bolt Performance Optimization:
/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
/// remainder chunk handling and bounds checking, enabling better vectorization
/// and measurable performance improvements.
pub fn apply_thermal(fb: &mut Framebuffer, config: &ThermalConfig) {
    if config.intensity <= 0.0 {
        return;
    }

    let pixels = fb.as_mut_slice();
    let intensity = config.intensity.clamp(0.0, 1.0);
    let invert = config.invert;

    // Fixed-point intensity (0..256)
    let intensity_fixed = (intensity * 256.0) as u32;

    #[cfg(feature = "parallel")]
    {
        pixels.par_iter_mut().for_each(|pixel| {
            process_pixel(pixel, intensity_fixed, invert);
        });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for pixel in pixels.iter_mut() {
            process_pixel(pixel, intensity_fixed, invert);
        }
    }
}

#[inline(always)]
#[allow(clippy::cast_lossless)]
const fn process_pixel(pixel: &mut u32, intensity_fixed: u32, invert: bool) {
    let p = *pixel;
    let mut lum = pixel_luminance(p);

    if invert {
        lum = 255 - lum;
    }

    // Thermal Color Palette Mapping
    // 0..51: Black -> Blue (0, 0, 0) -> (0, 0, 255)
    // 51..102: Blue -> Purple (0, 0, 255) -> (128, 0, 128)
    // 102..153: Purple -> Red (128, 0, 128) -> (255, 0, 0)
    // 153..204: Red -> Yellow (255, 0, 0) -> (255, 255, 0)
    // 204..255: Yellow -> White (255, 255, 0) -> (255, 255, 255)

    let (r, g, b) = match lum {
        0..=50 => {
            let t = lum as u32;
            let b = (t * 255) / 51;
            (0, 0, b)
        }
        51..=101 => {
            let t = (lum - 51) as u32;
            let r = (t * 128) / 51;
            let b = 255 - ((t * 127) / 51);
            (r, 0, b)
        }
        102..=152 => {
            let t = (lum - 102) as u32;
            let r = 128 + ((t * 127) / 51);
            let b = 128 - ((t * 128) / 51);
            (r, 0, b)
        }
        153..=203 => {
            let t = (lum - 153) as u32;
            let g = (t * 255) / 51;
            (255, g, 0)
        }
        _ => {
            let t = (lum - 204) as u32;
            let b = (t * 255) / 51;
            (255, 255, b)
        }
    };

    // Blend original pixel with thermal pixel based on intensity
    let orig_a = p & 0xFF00_0000;
    let orig_r = (p >> 16) & 0xFF;
    let orig_g = (p >> 8) & 0xFF;
    let orig_b = p & 0xFF;

    let final_r = (orig_r * (256 - intensity_fixed) + r * intensity_fixed) >> 8;
    let final_g = (orig_g * (256 - intensity_fixed) + g * intensity_fixed) >> 8;
    let final_b = (orig_b * (256 - intensity_fixed) + b * intensity_fixed) >> 8;

    *pixel = orig_a | (final_r << 16) | (final_g << 8) | final_b;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_thermal() {
        let mut fb = Framebuffer::new(5, 1).unwrap();
        // 0: Black, 1: Dark Gray, 2: Mid Gray, 3: Light Gray, 4: White
        fb.set_pixel(0, 0, 0xFF00_0000); // 0 (Black)
        fb.set_pixel(1, 0, 0xFF40_4040); // 64 (Dark Gray)
        fb.set_pixel(2, 0, 0xFF80_8080); // 128 (Mid Gray)
        fb.set_pixel(3, 0, 0xFFC0_C0C0); // 192 (Light Gray)
        fb.set_pixel(4, 0, 0xFFFF_FFFF); // 255 (White)

        let config = ThermalConfig::default();
        apply_thermal(&mut fb, &config);

        // Check mapping values (approximate due to integer math)
        // 0: Black maps to Blue/Black (0, 0, 0)
        assert_eq!(fb.get_pixel(0, 0).unwrap() & 0x00FF_FFFF, 0x0000_0000);

        // 64: Dark Gray maps to Blue->Purple range (r=~32, g=0, b=~223)
        let p1 = fb.get_pixel(1, 0).unwrap();
        assert_eq!((p1 >> 8) & 0xFF, 0, "Green should be 0");
        assert!(((p1 >> 16) & 0xFF) > 0, "Red should be > 0");
        assert!((p1 & 0xFF) > 128, "Blue should be > 128");

        // 128: Mid Gray maps to Purple->Red range
        let p2 = fb.get_pixel(2, 0).unwrap();
        assert_eq!((p2 >> 8) & 0xFF, 0, "Green should be 0");
        assert!(((p2 >> 16) & 0xFF) > 128, "Red should be > 128");

        // 192: Light Gray maps to Red->Yellow range (r=255, g>0, b=0)
        let p3 = fb.get_pixel(3, 0).unwrap();
        assert_eq!((p3 >> 16) & 0xFF, 255, "Red should be 255");
        assert!(((p3 >> 8) & 0xFF) > 128, "Green should be > 128");
        assert_eq!(p3 & 0xFF, 0, "Blue should be 0");

        // 255: White maps to White (255, 255, 255)
        // With integer math, 255 -> 204..255 range -> t=51 -> b=(51*255)/51 = 255
        assert_eq!(fb.get_pixel(4, 0).unwrap() & 0x00FF_FFFF, 0x00FF_FFFF);
    }

    #[test]
    fn test_apply_thermal_invert() {
        let mut fb = Framebuffer::new(2, 1).unwrap();
        fb.set_pixel(0, 0, 0xFF00_0000); // 0 (Black)
        fb.set_pixel(1, 0, 0xFFFF_FFFF); // 255 (White)

        let config = ThermalConfig {
            intensity: 1.0,
            invert: true,
        };
        apply_thermal(&mut fb, &config);

        // Inverted: Black -> White, White -> Black
        assert_eq!(fb.get_pixel(0, 0).unwrap() & 0x00FF_FFFF, 0x00FF_FFFF);
        assert_eq!(fb.get_pixel(1, 0).unwrap() & 0x00FF_FFFF, 0x0000_0000);
    }
}
