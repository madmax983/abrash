//! CCTV Camera Filter
//!
//! A retro post-processing effect simulating a cheap security camera feed.
//! It desaturates the image, adds scanlines, noise, and a blinking "REC" overlay.

use crate::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the CCTV effect.
#[derive(Debug, Clone, Copy)]
pub struct CctvConfig {
    /// Desaturation amount (0.0 to 1.0).
    pub desaturation: f32,
    /// Intensity of the procedural noise/snow (0.0 to 1.0).
    pub noise_intensity: f32,
    /// Time variable used for animating noise and the REC indicator.
    pub time: f32,
    /// Speed of the blinking REC indicator.
    pub blink_speed: f32,
}

impl Default for CctvConfig {
    fn default() -> Self {
        Self {
            desaturation: 0.8,
            noise_intensity: 0.3,
            time: 0.0,
            blink_speed: 2.0,
        }
    }
}

/// Applies a CCTV post-processing effect to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to apply the effect to.
/// * `config` - The configuration parameters for the effect.
pub fn apply_cctv(fb: &mut Framebuffer, config: &CctvConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let noise_seed = (config.time * 1000.0) as u32 ^ 0x1234_5678;

    let dest_pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let row_iter = dest_pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = dest_pixels.chunks_exact_mut(width).enumerate();

    let desat = config.desaturation.clamp(0.0, 1.0);
    let noise_intensity = config.noise_intensity.clamp(0.0, 1.0) * 255.0;

    row_iter.for_each(|(y, row)| {
        let is_scanline = y % 3 == 0;
        let row_noise_base = noise_seed.wrapping_add((y as u32).wrapping_mul(7919));

        for (x, pixel) in row.iter_mut().enumerate() {
            let p = *pixel;
            let r = ((p >> 16) & 0xFF) as f32;
            let g = ((p >> 8) & 0xFF) as f32;
            let b = (p & 0xFF) as f32;

            // Rec. 601 Luma
            let luma = 0.299 * r + 0.587 * g + 0.114 * b;

            let mut out_r = r * (1.0 - desat) + luma * desat;
            let mut out_g = g * (1.0 - desat) + luma * desat;
            let mut out_b = b * (1.0 - desat) + luma * desat;

            // Apply faint scanlines
            if is_scanline {
                out_r *= 0.8;
                out_g *= 0.8;
                out_b *= 0.8;
            }

            // Procedural noise
            if noise_intensity > 0.0 {
                let n = (x as u32).wrapping_mul(1973).wrapping_add(row_noise_base);
                let noise_val = (n.wrapping_mul(2_654_435_761) >> 16) as f32 / 65535.0;
                let noise_offset = (noise_val - 0.5) * noise_intensity;
                out_r += noise_offset;
                out_g += noise_offset;
                out_b += noise_offset;
            }

            let final_r = out_r.clamp(0.0, 255.0) as u32;
            let final_g = out_g.clamp(0.0, 255.0) as u32;
            let final_b = out_b.clamp(0.0, 255.0) as u32;

            *pixel = 0xFF00_0000 | (final_r << 16) | (final_g << 8) | final_b;
        }
    });

    // Draw REC indicator
    let blink = (config.time * config.blink_speed).sin();
    if blink > 0.0 && width > 30 && height > 30 {
        // Top right corner
        let rec_w = 12;
        let rec_h = 12;
        let padding = 10;
        let start_x = width - rec_w - padding;
        let start_y = padding;

        for y in start_y..(start_y + rec_h) {
            for x in start_x..(start_x + rec_w) {
                // Circle equation for the red dot
                let cx = start_x + rec_w / 2;
                let cy = start_y + rec_h / 2;
                let dx = x as isize - cx as isize;
                let dy = y as isize - cy as isize;
                if dx * dx + dy * dy <= (rec_w as isize / 2) * (rec_w as isize / 2) {
                    fb.set_pixel(x as i32, y as i32, 0xFFFF_0000);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_cctv_desaturation_and_scanlines() {
        let width = 20;
        let height = 20;
        let mut fb = Framebuffer::new(width, height).unwrap();

        // Fill with bright red
        fb.clear(0xFFFF_0000);

        let config = CctvConfig {
            desaturation: 1.0,    // Fully desaturate to grayscale
            noise_intensity: 0.0, // No noise for this test
            time: 0.0,
            blink_speed: 0.0,
        };

        apply_cctv(&mut fb, &config);

        // Center pixel (least affected by edge artifacts if any)
        let pixel = fb.get_pixel(10, 10).unwrap();
        let r = (pixel >> 16) & 0xFF;
        let g = (pixel >> 8) & 0xFF;
        let b = pixel & 0xFF;

        // If desaturated, r, g, and b should be close to each other (grayscale)
        // Red has luminance ~0.299 * 255 = 76
        assert!(
            (r as i32 - g as i32).abs() < 5,
            "Not desaturated: R={r}, G={g}"
        );
        assert!(
            (g as i32 - b as i32).abs() < 5,
            "Not desaturated: G={g}, B={b}"
        );

        // Check for scanlines
        let pixel1 = fb.get_pixel(10, 11).unwrap();
        let pixel2 = fb.get_pixel(10, 12).unwrap();
        assert_ne!(pixel1, pixel2, "Scanlines not applied (rows match)");
    }

    #[test]
    fn test_cctv_rec_indicator() {
        let width = 64;
        let height = 64;
        let mut fb = Framebuffer::new(width, height).unwrap();

        // Clear to black
        fb.clear(0xFF000000);

        let config = CctvConfig {
            desaturation: 0.0,
            noise_intensity: 0.0,
            time: 1.0, // Time to ensure REC is drawn (sin(1.0 * 2.0) > 0)
            blink_speed: 2.0,
        };

        apply_cctv(&mut fb, &config);

        // Check for pure red REC indicator in the top right corner
        // 0xFFFF_0000
        let mut found_red = false;
        for y in 0..16 {
            for x in (width - 32)..width {
                if fb.get_pixel(x as i32, y as i32).unwrap() == 0xFFFF_0000 {
                    found_red = true;
                    break;
                }
            }
        }
        assert!(found_red, "REC indicator not found");

        let config_hidden = CctvConfig {
            desaturation: 0.0,
            noise_intensity: 0.0,
            time: 0.0, // Time to ensure REC is hidden (sin(0) = 0)
            blink_speed: 2.0,
        };

        let mut fb2 = Framebuffer::new(width, height).unwrap();
        fb2.clear(0xFF000000);
        apply_cctv(&mut fb2, &config_hidden);

        let mut found_red_hidden = false;
        for y in 0..16 {
            for x in (width - 32)..width {
                if fb2.get_pixel(x as i32, y as i32).unwrap() == 0xFFFF_0000 {
                    found_red_hidden = true;
                    break;
                }
            }
        }
        assert!(!found_red_hidden, "REC indicator should be hidden");
    }
}
