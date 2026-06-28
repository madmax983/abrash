//! CCTV Security Camera Filter
//!
//! A post-processing effect that simulates a low-fidelity security camera feed.
//! It combines grayscale conversion with a green tint, scanlines, heavy vignette,
//! noise, and a blinking "REC" text overlay.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

/// Configuration for the CCTV filter.
#[derive(Debug, Clone, Copy)]
pub struct CctvConfig {
    /// The RGB color to tint the image (e.g., `0x00_00FF00` for green).
    pub tint_color: u32,
    /// Intensity of the scanlines (0.0 to 1.0).
    pub scanline_intensity: f32,
    /// Intensity of the random noise (0.0 to 1.0).
    pub noise_intensity: f32,
    /// Intensity of the vignette effect (0.0 to 1.0).
    pub vignette_intensity: f32,
    /// Current time, used to animate the blinking "REC" text and noise.
    pub time: f32,
    /// Whether to display the blinking "REC" text.
    pub show_rec: bool,
}

impl Default for CctvConfig {
    fn default() -> Self {
        Self {
            tint_color: 0xFF_44FF44, // Bright green tint
            scanline_intensity: 0.3,
            noise_intensity: 0.2,
            vignette_intensity: 0.8,
            time: 0.0,
            show_rec: true,
        }
    }
}

// Minimal 5x7 font for "REC " (and colon if needed)
// R, E, C, O, :
const FONT: [[u8; 7]; 5] = [
    // R
    [0xF0, 0x88, 0x88, 0xF0, 0xA0, 0x90, 0x88],
    // E
    [0xF8, 0x80, 0x80, 0xF0, 0x80, 0x80, 0xF8],
    // C
    [0x70, 0x88, 0x80, 0x80, 0x80, 0x88, 0x70],
    // O
    [0x70, 0x88, 0x88, 0x88, 0x88, 0x88, 0x70],
    // :
    [0x00, 0x60, 0x60, 0x00, 0x60, 0x60, 0x00],
];

// Map a character to our minimal font index
const fn char_to_font_idx(c: char) -> Option<usize> {
    match c {
        'R' => Some(0),
        'E' => Some(1),
        'C' => Some(2),
        'O' => Some(3),
        ':' => Some(4),
        _ => None,
    }
}

/// Applies a CCTV / Security Camera effect to the framebuffer.
pub fn apply_cctv(fb: &mut Framebuffer, config: &CctvConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let pixels = fb.as_mut_slice();
    // A simple LCG PRNG for the noise
    let mut prng_state = (config.time * 1000.0) as u32 ^ 0xDEAD_BEEF;

    let tint_r = ((config.tint_color >> 16) & 0xFF) as f32;
    let tint_g = ((config.tint_color >> 8) & 0xFF) as f32;
    let tint_b = (config.tint_color & 0xFF) as f32;

    let center_x = width as f32 * 0.5;
    let center_y = height as f32 * 0.5;
    let max_dist_sq = center_x * center_x + center_y * center_y;

    for y in 0..height {
        let y_f = y as f32;
        let dy = y_f - center_y;
        let dy_sq = dy * dy;
        let scanline_darken = if y % 2 == 0 {
            1.0 - config.scanline_intensity
        } else {
            1.0
        };

        for x in 0..width {
            let x_f = x as f32;
            let dx = x_f - center_x;
            let dist_sq = dx * dx + dy_sq;

            let vignette = 1.0 - (dist_sq / max_dist_sq) * config.vignette_intensity;
            let vignette = vignette.clamp(0.0, 1.0);

            let idx = y * width + x;
            let pixel = pixels[idx];
            let lum = f32::from(pixel_luminance(pixel));

            prng_state = prng_state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let noise = ((prng_state & 0xFF) as f32 / 255.0) * config.noise_intensity * 255.0;

            let mut out_r = ((lum / 255.0) * tint_r + noise) * vignette * scanline_darken;
            let mut out_g = ((lum / 255.0) * tint_g + noise) * vignette * scanline_darken;
            let mut out_b = ((lum / 255.0) * tint_b + noise) * vignette * scanline_darken;

            out_r = out_r.clamp(0.0, 255.0);
            out_g = out_g.clamp(0.0, 255.0);
            out_b = out_b.clamp(0.0, 255.0);

            pixels[idx] =
                0xFF00_0000 | ((out_r as u32) << 16) | ((out_g as u32) << 8) | (out_b as u32);
        }
    }

    if config.show_rec && (config.time % 1.0) < 0.5 {
        let text = "REC";
        let start_x = width.saturating_sub(100);
        let start_y = 20;
        let scale = 4; // Scale up the 5x7 font

        for (i, c) in text.chars().enumerate() {
            if let Some(font_idx) = char_to_font_idx(c) {
                let char_data = &FONT[font_idx];
                let char_offset_x = start_x + i * (6 * scale); // 5 cols + 1 space

                for cy in 0..7 {
                    let row = char_data[cy];
                    for cx in 0..5 {
                        if (row & (0x80 >> cx)) != 0 {
                            // Draw scaled pixel
                            for sy in 0..scale {
                                for sx in 0..scale {
                                    let px = char_offset_x + cx * scale + sx;
                                    let py = start_y + cy * scale + sy;
                                    if px < width && py < height {
                                        // Draw a red pixel for "REC"
                                        pixels[py * width + px] = 0xFF_FF0000;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_cctv() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF_FFFFFF); // White background

        let config = CctvConfig {
            tint_color: 0xFF_44FF44,
            scanline_intensity: 0.5,
            noise_intensity: 0.0, // No noise for deterministic test
            vignette_intensity: 0.0,
            time: 0.0,
            show_rec: true,
        };

        apply_cctv(&mut fb, &config);

        // Center pixel should be tinted green
        // Since floating point math might slightly vary, we just check it is not white
        let center_pixel = fb.as_slice()[50 * 100 + 50];
        assert_ne!(center_pixel, 0xFF_FFFFFF);
    }
}
