use abrash_core::framebuffer::Framebuffer;
#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Synthwave retro filter.
#[derive(Clone, Debug)]
pub struct SynthwaveConfig {
    /// Time value used to animate the grid.
    pub time: f32,
    /// Color of the grid lines (ARGB).
    pub grid_color: u32,
    /// Color of the retro sun (ARGB).
    pub sun_color: u32,
    /// Color of the sky (ARGB).
    pub sky_color: u32,
    /// Color of the ground (ARGB).
    pub ground_color: u32,
}

impl Default for SynthwaveConfig {
    fn default() -> Self {
        Self {
            time: 0.0,
            grid_color: 0xFFFF00FF,   // Magenta
            sun_color: 0xFFFF8800,    // Orange/Yellow
            sky_color: 0xFF100020,    // Deep purple
            ground_color: 0xFF050010, // Dark ground
        }
    }
}

/// Applies a retro 80s Synthwave aesthetic to the framebuffer.
/// This completely overwrites the framebuffer with a procedurally generated
/// retro sun and a perspective grid that moves over time.
pub fn apply_synthwave(fb: &mut Framebuffer, config: &SynthwaveConfig) {
    let width = fb.width() as i32;
    let height = fb.height() as i32;
    let half_height = height / 2;
    let half_width = width / 2;

    let time = config.time;

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let chunk_iter = pixels.par_chunks_exact_mut(width as usize);

    #[cfg(not(feature = "parallel"))]
    let chunk_iter = pixels.chunks_exact_mut(width as usize);

    chunk_iter.enumerate().for_each(|(y, row)| {
        let y_i32 = y as i32;

        if y_i32 < half_height {
            // Top half: Sky and Retro Sun
            for x in 0..width {
                // Sun logic
                let sun_radius = height as f32 * 0.3;
                let dx = (x - half_width) as f32;
                let dy = (y_i32 - half_height) as f32;
                // Slightly lower the sun's center
                let dist_sq = dx * dx + (dy + sun_radius * 0.2) * (dy + sun_radius * 0.2);

                let mut color = config.sky_color;

                if dist_sq < sun_radius * sun_radius {
                    // We are inside the sun. Apply cutouts based on height
                    let sun_y = y_i32 as f32;
                    // Frequency of the cutouts gets higher towards the bottom
                    let normalized_y = (sun_y / half_height as f32).clamp(0.0, 1.0);

                    // Modulo magic for cutouts
                    // The cutouts should be horizontal lines
                    // We create thicker lines near the bottom of the sun
                    let cutout_freq = 20.0 + normalized_y * 30.0;
                    let line_phase = (normalized_y * cutout_freq) % 1.0;

                    // If line_phase is large enough, it's a cutout (show sky), else show sun
                    let threshold = 0.5 + normalized_y * 0.4; // Cutouts get thicker at the bottom

                    if line_phase < threshold {
                        color = config.sun_color;
                    }
                }

                row[x as usize] = color;
            }
        } else {
            // Bottom half: Ground and Perspective Grid
            // Calculate perspective projection for the ground
            // y_i32 goes from half_height to height
            let horizon_y = y_i32 - half_height;
            // Avoid division by zero
            let z = if horizon_y == 0 {
                10000.0
            } else {
                half_height as f32 / horizon_y as f32
            };

            // Check horizontal grid lines
            // We map z to a repeating grid
            let grid_size_z = 2.0;
            // Move the grid towards the viewer over time
            let shifted_z = z - time * 5.0;

            let line_thickness_z = 0.1 * z; // Lines get thicker closer to camera

            let is_horizontal_line = (shifted_z % grid_size_z).abs() < line_thickness_z;

            for x in 0..width {
                let mut color = config.ground_color;

                // Calculate x in world space
                let dx = (x - half_width) as f32;
                let world_x = dx * z * 0.01;

                let grid_size_x = 1.0;
                let line_thickness_x = 0.05;

                let is_vertical_line = (world_x % grid_size_x).abs() < line_thickness_x;

                if is_horizontal_line || is_vertical_line {
                    // Distance fade for the grid
                    let fade = (1.0 - (z / 20.0)).clamp(0.0, 1.0);

                    // Extract RGB components to apply fade
                    let r = ((config.grid_color >> 16) & 0xFF) as f32 * fade;
                    let g = ((config.grid_color >> 8) & 0xFF) as f32 * fade;
                    let b = (config.grid_color & 0xFF) as f32 * fade;

                    let faded_grid_color =
                        0xFF00_0000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);

                    // Further fade the color to black as it approaches horizon
                    color = faded_grid_color;
                }

                row[x as usize] = color;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_synthwave_modifies_buffer() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFFFFFFFF); // White background

        let config = SynthwaveConfig::default();
        apply_synthwave(&mut fb, &config);

        // Check that the sky was applied in the top left
        assert_eq!(fb.get_pixel(0, 0).unwrap(), config.sky_color);

        // Ground is applied at the bottom left
        assert_eq!(fb.get_pixel(0, 99).unwrap(), config.ground_color);
    }
}
