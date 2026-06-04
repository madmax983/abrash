//! Procedural Lava Lamp Filter.
//!
//! A post-processing effect that generates a procedural "lava lamp" or fluid-like
//! pattern using layered sine waves and distance functions. It colors the
//! framebuffer based on these continuous smooth math fields.

use crate::framebuffer::Framebuffer;
#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Lava Lamp effect.
#[derive(Debug, Clone, Copy)]
pub struct LavaLampConfig {
    /// The current time or phase of the animation.
    pub time: f32,
    /// The scale of the blobs.
    pub scale: f32,
    /// Color of the "lava" blobs.
    pub color_a: u32,
    /// Background color.
    pub color_b: u32,
    /// Threshold for blob boundaries.
    pub threshold: f32,
}

impl Default for LavaLampConfig {
    fn default() -> Self {
        Self {
            time: 0.0,
            scale: 15.0,
            color_a: 0xFFFF_0055, // Hot pink/red
            color_b: 0xFF22_0022, // Dark purple
            threshold: 0.6,
        }
    }
}

/// Applies a procedural lava lamp effect to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify.
/// * `config` - The configuration for the effect.
pub fn apply_lava_lamp(fb: &mut Framebuffer, config: &LavaLampConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let dest = fb.as_mut_slice();

    let w_f32 = width as f32;
    let h_f32 = height as f32;

    #[cfg(feature = "parallel")]
    let row_iter = dest.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = dest.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        let y_norm = (y as f32) / h_f32;
        let cy = y_norm * config.scale;

        for (x, pixel) in row.iter_mut().enumerate() {
            let x_norm = (x as f32) / w_f32;
            let cx = x_norm * config.scale;

            // Generate some procedural blobs using layered sine waves and distances.
            let v1 = (cx + config.time).sin() + (cy + config.time).cos();
            let v2 = (cx * 0.5 - config.time * 0.5).cos() + (cy * 0.5 + config.time * 0.5).sin();

            // Distance from some moving centers
            let dx = cx - (config.time * 0.3).cos() * 5.0 - (config.scale * 0.5);
            let dy = cy - (config.time * 0.4).sin() * 5.0 - (config.scale * 0.5);
            let d = dx.hypot(dy);
            let v3 = (d - config.time).sin();

            // Combine and normalize roughly to 0.0 - 1.0 range
            let combined = (v1 + v2 + v3) / 6.0 + 0.5;

            if combined > config.threshold {
                *pixel = config.color_a;
            } else {
                // Smooth blending based on value could be added here,
                // but a hard threshold gives a classic blobby lava lamp look.
                *pixel = config.color_b;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_apply_lava_lamp_does_not_crash() {
        let mut fb = Framebuffer::new(32, 32).unwrap();
        let mut config = LavaLampConfig::default();
        config.time = 1.0;

        apply_lava_lamp(&mut fb, &config);

        // Just verify it doesn't crash and did *something*
        // It should at least be filled with color_a or color_b
        let p = fb.get_pixel(0, 0).unwrap();
        assert!(p == config.color_a || p == config.color_b);
    }
}
