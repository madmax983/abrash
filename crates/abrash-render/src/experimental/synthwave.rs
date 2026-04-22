//! Synthwave Retro Perspective Grid Filter
//!
//! A procedural post-processing effect that renders an animated, pseudo-3D
//! perspective grid characteristic of the 80s synthwave/retrowave aesthetic.
//! It works purely in screen space by mapping Y-coordinates to depth (Z).

use crate::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Synthwave effect.
#[derive(Debug, Clone, Copy)]
pub struct SynthwaveConfig {
    /// An continuously increasing time value used to animate the grid moving forward.
    pub time: f32,
    /// The normalized vertical position of the horizon line (e.g., 0.5 for the middle).
    pub horizon: f32,
    /// The color of the glowing grid lines (ARGB).
    pub grid_color: u32,
    /// The base background color for the ground (ARGB).
    pub ground_color: u32,
    /// The speed at which the grid moves towards the viewer.
    pub speed: f32,
    /// The number of horizontal lines visible before the horizon.
    pub grid_density: f32,
    /// The thickness of the grid lines.
    pub line_thickness: f32,
}

impl Default for SynthwaveConfig {
    fn default() -> Self {
        Self {
            time: 0.0,
            horizon: 0.5,
            grid_color: 0xFFFF_00FF,   // Magenta glow
            ground_color: 0xFF10_0020, // Dark purple/black
            speed: 5.0,
            grid_density: 20.0,
            line_thickness: 0.05,
        }
    }
}

/// Applies a retro synthwave perspective grid effect to the lower half of the framebuffer.
///
/// Only pixels below the horizon line are modified. The effect works by projecting
/// 2D screen coordinates into a pseudo-3D space, calculating a procedural grid
/// using modulo arithmetic, and mapping it back.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - The configuration parameters for the effect.
pub fn apply_synthwave(fb: &mut Framebuffer, config: &SynthwaveConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let horizon_y = (config.horizon * height as f32) as usize;

    // Safety clamp in case horizon is out of bounds
    let horizon_y = horizon_y.min(height);

    if horizon_y >= height {
        return; // Nothing to draw below the bottom edge
    }

    let pixels = fb.as_mut_slice();

    // Split the buffer to only process the lower half
    let (_, lower_half) = pixels.split_at_mut(horizon_y * width);

    let half_w = width as f32 / 2.0;

    // Precalculate time offset
    let time_offset = config.time * config.speed;

    #[cfg(feature = "parallel")]
    let row_iter = lower_half.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = lower_half.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(local_y, row)| {
        let y = (horizon_y + local_y) as f32;

        // Pseudo-depth based on distance from horizon.
        // We add a tiny epsilon to prevent division by zero exactly at the horizon.
        let dy = y - (horizon_y as f32) + 0.001;

        // Z distance gets smaller as we go down the screen.
        let z = 1.0 / (dy / height as f32);

        // Calculate horizontal moving lines using modulo arithmetic
        let z_moved = z * config.grid_density - time_offset;
        let mut h_line = z_moved.fract();
        if h_line < 0.0 {
            h_line += 1.0;
        }

        // The thickness must be proportional to depth so lines get thinner in the distance
        let scaled_thickness = config.line_thickness * (1.0 + z * 0.1);

        let is_horizontal = h_line < scaled_thickness;

        for (x, pixel) in row.iter_mut().enumerate() {
            let dx = x as f32 - half_w;

            // Calculate perspective vertical lines
            // Map X based on depth to create converging lines
            let perspective_x = dx * z * (config.grid_density / width as f32);

            let mut v_line = perspective_x.fract();
            if v_line < 0.0 {
                v_line += 1.0; // Handle negative fract
            }

            let is_vertical = v_line < scaled_thickness;

            if is_horizontal || is_vertical {
                *pixel = config.grid_color;
            } else {
                *pixel = config.ground_color;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthwave_modifies_ground_only() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFFFF_FFFF); // All white

        let config = SynthwaveConfig {
            horizon: 0.5,
            ..Default::default()
        };

        apply_synthwave(&mut fb, &config);

        let pixels = fb.as_slice();

        // Top half should remain untouched (white)
        for y in 0..50 {
            for x in 0..100 {
                assert_eq!(pixels[y * 100 + x], 0xFFFF_FFFF, "Sky pixel changed");
            }
        }

        // Bottom half should have changed to grid or ground colors
        let mut changed = false;
        for y in 50..100 {
            for x in 0..100 {
                let p = pixels[y * 100 + x];
                if p == config.grid_color || p == config.ground_color {
                    changed = true;
                    break;
                }
            }
        }
        assert!(changed, "Ground was not modified");
    }

    #[test]
    fn test_synthwave_zero_height() {
        let mut fb = Framebuffer::new(0, 0).unwrap();
        let config = SynthwaveConfig::default();
        // Should not panic
        apply_synthwave(&mut fb, &config);
    }
}
