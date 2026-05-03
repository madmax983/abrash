//! Light Leak Post-Processing Filter
//!
//! Simulates an analog film light leak by mapping colored gradients and flares
//! onto the screen based on screen coordinates and procedural time-based animation.

use crate::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Light Leak effect.
#[derive(Debug, Clone, Copy)]
pub struct LightLeakConfig {
    /// Overall intensity of the light leak effect (0.0 to 1.0).
    pub intensity: f32,
    /// Time variable used for animating the light flares.
    pub time: f32,
    /// Base color of the primary leak (e.g., orange/red 0xFFFFAA00).
    pub color_primary: u32,
    /// Secondary color of the leak (e.g., magenta 0xFFFF00AA).
    pub color_secondary: u32,
}

impl Default for LightLeakConfig {
    fn default() -> Self {
        Self {
            intensity: 0.8,
            time: 0.0,
            color_primary: 0xFF_FF4400,   // Bright Orange-Red
            color_secondary: 0xFF_FF0055, // Deep Pink
        }
    }
}

/// Applies a light leak effect to the framebuffer.
///
/// Combines moving circular/elliptical gradients overlapping the edge of the frame
/// using additive/screen blending logic to simulate exposed film edges.
///
/// # Arguments
///
/// * `fb` - The framebuffer to apply the effect to.
/// * `config` - The configuration parameters for the effect.
pub fn apply_light_leak(fb: &mut Framebuffer, config: &LightLeakConfig) {
    if config.intensity <= 0.0 {
        return; // Nothing to do
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let w_f32 = width as f32;
    let h_f32 = height as f32;

    // Destructure colors into components
    let r1 = ((config.color_primary >> 16) & 0xFF) as f32 / 255.0;
    let g1 = ((config.color_primary >> 8) & 0xFF) as f32 / 255.0;
    let b1 = (config.color_primary & 0xFF) as f32 / 255.0;

    let r2 = ((config.color_secondary >> 16) & 0xFF) as f32 / 255.0;
    let g2 = ((config.color_secondary >> 8) & 0xFF) as f32 / 255.0;
    let b2 = (config.color_secondary & 0xFF) as f32 / 255.0;

    // Flare positions based on time
    let flare1_x = (config.time * 0.5).sin() * 0.4 + 0.5; // Oscillate mostly near center-right
    let flare1_y = (config.time * 0.3).cos() * 0.2 + 0.1; // Mostly near top

    let flare2_x = (config.time * 0.4 + 2.0).sin() * 0.3 + 0.2;
    let flare2_y = (config.time * 0.6 + 1.0).cos() * 0.4 + 0.8; // Mostly near bottom left

    let dest_pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let row_iter = dest_pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = dest_pixels.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        let ny = y as f32 / h_f32;

        let dy1 = (ny - flare1_y) * 1.5; // stretch vertically
        let dy1_sq = dy1 * dy1;

        let dy2 = (ny - flare2_y) * 2.0;
        let dy2_sq = dy2 * dy2;

        for (x, pixel) in row.iter_mut().enumerate() {
            let nx = x as f32 / w_f32;

            let dx1 = (nx - flare1_x) * 0.8; // wide
            let dist1 = (dx1 * dx1 + dy1_sq).sqrt();
            let falloff1 = (1.0 - dist1 * 1.5).max(0.0).powi(2); // Smooth dropoff

            let dx2 = (nx - flare2_x) * 0.5;
            let dist2 = (dx2 * dx2 + dy2_sq).sqrt();
            let falloff2 = (1.0 - dist2 * 2.0).max(0.0).powi(2);

            let mut leak_r = falloff1 * r1 + falloff2 * r2;
            let mut leak_g = falloff1 * g1 + falloff2 * g2;
            let mut leak_b = falloff1 * b1 + falloff2 * b2;

            leak_r *= config.intensity;
            leak_g *= config.intensity;
            leak_b *= config.intensity;

            if leak_r > 0.001 || leak_g > 0.001 || leak_b > 0.001 {
                let p = *pixel;
                let orig_r = ((p >> 16) & 0xFF) as f32 / 255.0;
                let orig_g = ((p >> 8) & 0xFF) as f32 / 255.0;
                let orig_b = (p & 0xFF) as f32 / 255.0;

                // Screen blending: 1 - (1 - a)(1 - b)
                let final_r = 1.0 - (1.0 - orig_r) * (1.0 - leak_r.min(1.0));
                let final_g = 1.0 - (1.0 - orig_g) * (1.0 - leak_g.min(1.0));
                let final_b = 1.0 - (1.0 - orig_b) * (1.0 - leak_b.min(1.0));

                let out_r = (final_r.clamp(0.0, 1.0) * 255.0) as u32;
                let out_g = (final_g.clamp(0.0, 1.0) * 255.0) as u32;
                let out_b = (final_b.clamp(0.0, 1.0) * 255.0) as u32;

                *pixel = 0xFF00_0000 | (out_r << 16) | (out_g << 8) | out_b;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_light_leak_adds_color() {
        let width = 10;
        let height = 10;
        let mut fb = Framebuffer::new(width, height).unwrap();
        fb.clear(0xFF00_0000); // Black

        let config = LightLeakConfig {
            intensity: 1.0,
            time: 0.0,
            color_primary: 0xFFFF_0000,   // Red
            color_secondary: 0xFF00_0000, // None
        };

        apply_light_leak(&mut fb, &config);

        let mut has_red = false;
        for &pixel in fb.as_slice() {
            let r = (pixel >> 16) & 0xFF;
            if r > 0 {
                has_red = true;
                break;
            }
        }

        assert!(has_red, "Light leak should add color to black screen");
    }

    #[test]
    fn test_light_leak_zero_intensity() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFF12_3456);

        let config = LightLeakConfig {
            intensity: 0.0,
            time: 0.0,
            color_primary: 0xFFFF_0000,
            color_secondary: 0xFFFF_0000,
        };

        apply_light_leak(&mut fb, &config);

        // Untouched
        for &pixel in fb.as_slice() {
            assert_eq!(pixel, 0xFF12_3456);
        }
    }
}
