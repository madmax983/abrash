//! Hologram Post-Processing Filter
//!
//! A retro sci-fi post-processing effect that turns the framebuffer into a flickering,
//! scanline-heavy holographic projection.

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

thread_local! {
    static HOLOGRAM_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the Hologram effect.
#[derive(Debug, Clone, Copy)]
pub struct HologramConfig {
    /// The base color of the hologram (e.g., 0xFF00FFFF for cyan).
    pub color: u32,
    /// A continuously increasing time value used to animate flicker and scanlines.
    pub time: f32,
    /// Intensity of the scanlines (0.0 to 1.0).
    pub scanline_intensity: f32,
    /// Speed of the flicker effect.
    pub flicker_speed: f32,
    /// Intensity of the flicker (0.0 to 1.0).
    pub flicker_intensity: f32,
    /// Horizontal interference/glitch displacement strength (0.0 to 1.0).
    pub interference_strength: f32,
}

impl Default for HologramConfig {
    fn default() -> Self {
        Self {
            color: 0xFF00_FFFF, // Cyan
            time: 0.0,
            scanline_intensity: 0.5,
            flicker_speed: 10.0,
            flicker_intensity: 0.2,
            interference_strength: 0.02,
        }
    }
}

/// Applies a Hologram effect to the framebuffer.
///
/// This simulates a sci-fi holographic projection by:
/// 1. Converting the image to a monochrome tint based on the `color`.
/// 2. Adding horizontal rolling scanlines.
/// 3. Applying a global flicker.
/// 4. Displacing rows horizontally for interference.
pub fn apply_hologram(fb: &mut Framebuffer, config: &HologramConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    // Extract target color components
    let tc_r = ((config.color >> 16) & 0xFF) as f32;
    let tc_g = ((config.color >> 8) & 0xFF) as f32;
    let tc_b = (config.color & 0xFF) as f32;

    // Global flicker
    let flicker = 1.0 - config.flicker_intensity * (config.time * config.flicker_speed).sin().abs();

    // We need a source buffer for horizontal shifts
    HOLOGRAM_BUFFER.with(|buf| {
        let mut src_fb_vec = buf.borrow_mut();
        let size = width * height;
        src_fb_vec.clear();
        src_fb_vec.extend_from_slice(fb.as_slice());
        let src_pixels = src_fb_vec.as_slice();

        let dest_pixels = fb.as_mut_slice();

        #[cfg(feature = "parallel")]
        let row_iter = dest_pixels.par_chunks_exact_mut(width).enumerate();
        #[cfg(not(feature = "parallel"))]
        let row_iter = dest_pixels.chunks_exact_mut(width).enumerate();

        row_iter.for_each(|(y, row)| {
            let y_f32 = y as f32;

            // Rolling scanlines
            let scanline_phase = y_f32 * 0.1 - config.time * 5.0;
            let scanline = 1.0 - config.scanline_intensity * (0.5 + 0.5 * scanline_phase.sin());

            // Horizontal interference
            let interference_phase = y_f32 * 0.05 + config.time * 20.0;
            let mut x_shift = 0;

            if config.interference_strength > 0.0 {
                let shift_amt =
                    interference_phase.sin() * (width as f32 * config.interference_strength);
                // Occasional harsh glitch
                if (interference_phase * 0.5).cos() > 0.95 {
                    x_shift = (shift_amt * 2.0) as i32;
                } else {
                    x_shift = shift_amt as i32;
                }
            }

            let combined_intensity = flicker * scanline;

            for (x, pixel) in row.iter_mut().enumerate() {
                // Apply horizontal shift safely
                let mut src_x = x as i32 + x_shift;
                src_x = src_x.clamp(0, width as i32 - 1);

                let src_idx = y * width + (src_x as usize);
                let p = src_pixels[src_idx];

                let r = ((p >> 16) & 0xFF) as f32;
                let g = ((p >> 8) & 0xFF) as f32;
                let b = (p & 0xFF) as f32;

                // Grayscale luminance
                let lum = 0.299 * r + 0.587 * g + 0.114 * b;

                // Map luminance to target color, scaling by intensity
                let final_r = (lum * tc_r / 255.0 * combined_intensity).clamp(0.0, 255.0) as u32;
                let final_g = (lum * tc_g / 255.0 * combined_intensity).clamp(0.0, 255.0) as u32;
                let final_b = (lum * tc_b / 255.0 * combined_intensity).clamp(0.0, 255.0) as u32;

                *pixel = 0xFF00_0000 | (final_r << 16) | (final_g << 8) | final_b;
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_apply_hologram_changes_buffer() {
        let width = 100;
        let height = 100;
        let mut fb = Framebuffer::new(width, height).unwrap();

        fb.clear(0xFF_FF_FF_FF); // White

        let mut fb_clone = Framebuffer::new(width, height).unwrap();
        fb_clone.as_mut_slice().copy_from_slice(fb.as_slice());

        let config = HologramConfig {
            color: 0xFF00_FF00, // Green
            ..Default::default()
        };

        apply_hologram(&mut fb, &config);

        let mut different = false;
        for i in 0..(width * height) as usize {
            if fb.as_slice()[i] != fb_clone.as_slice()[i] {
                different = true;
                break;
            }
        }
        assert!(different, "Hologram filter did not modify the framebuffer");

        // Ensure green channel is dominant
        let p = fb.get_pixel(50, 50).unwrap();
        let r = (p >> 16) & 0xFF;
        let g = (p >> 8) & 0xFF;
        let b = p & 0xFF;
        assert!(g > r);
        assert!(g > b);
    }
}
