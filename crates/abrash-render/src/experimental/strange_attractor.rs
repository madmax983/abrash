//! Strange Attractor (Peter de Jong) Renderer
//!
//! Renders beautiful 2D strange attractors by iteratively plotting
//! points in a density map and applying logarithmic color mapping.

use abrash_core::color::Color;
use abrash_core::framebuffer::Framebuffer;

/// Configuration for the Peter de Jong strange attractor.
#[derive(Debug, Clone, Copy)]
pub struct AttractorConfig {
    /// Parameter A
    pub a: f32,
    /// Parameter B
    pub b: f32,
    /// Parameter C
    pub c: f32,
    /// Parameter D
    pub d: f32,
    /// Number of points to iterate
    pub iterations: usize,
    /// Base color for mapping density
    pub base_color: Color,
}

impl Default for AttractorConfig {
    fn default() -> Self {
        Self {
            // Some nice default parameters
            a: -2.24,
            b: 0.43,
            c: -0.65,
            d: -2.43,
            iterations: 2_000_000,
            base_color: Color::new(0.3, 0.6, 1.0, 1.0), // Cool blue
        }
    }
}

/// Renders a Peter de Jong strange attractor onto the framebuffer.
///
/// It uses a density estimation approach: points are generated and binned into a 2D array,
/// and then mapped logarithmically to color.
pub fn render_strange_attractor(fb: &mut Framebuffer, config: &AttractorConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let mut density_map = vec![0u32; width * height];
    let mut x = 0.0f32;
    let mut y = 0.0f32;
    let mut max_density = 0u32;

    for _ in 0..config.iterations {
        let next_x = (config.a * y).sin() - (config.b * x).cos();
        let next_y = (config.c * x).sin() - (config.d * y).cos();
        x = next_x;
        y = next_y;

        // Peter de Jong attractors typically fall in the range [-2.0, 2.0]
        let norm_x = (x + 2.0) / 4.0;
        let norm_y = (y + 2.0) / 4.0;

        let px = (norm_x * width as f32) as isize;
        let py = (norm_y * height as f32) as isize;

        if px >= 0 && px < width as isize && py >= 0 && py < height as isize {
            let idx = (py as usize) * width + (px as usize);
            density_map[idx] += 1;
            if density_map[idx] > max_density {
                max_density = density_map[idx];
            }
        }
    }

    let max_density_f32 = max_density as f32;
    let log_max = if max_density_f32 > 0.0 {
        max_density_f32.ln_1p()
    } else {
        1.0
    };

    let fb_slice = fb.as_mut_slice();

    for i in 0..(width * height) {
        let density = density_map[i];
        if density > 0 {
            let log_density = (density as f32).ln_1p();
            let intensity = log_density / log_max;

            // Map intensity to color
            let r = config.base_color.r * intensity;
            let g = config.base_color.g * intensity;
            let b = config.base_color.b * intensity;

            let final_color = Color::new(r, g, b, 1.0);
            fb_slice[i] = final_color.to_argb_u32();
        } else {
            fb_slice[i] = 0xFF00_0000; // Black background
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_strange_attractor() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        let config = AttractorConfig {
            a: 1.4,
            b: -2.3,
            c: 2.4,
            d: -2.1,
            iterations: 10_000,
            base_color: Color::new(1.0, 0.0, 0.0, 1.0),
        };

        render_strange_attractor(&mut fb, &config);

        // Verify that some pixels are drawn (not completely black)
        let mut has_color = false;
        for &pixel in fb.as_slice() {
            if pixel != 0xFF00_0000 {
                has_color = true;
                break;
            }
        }
        assert!(has_color, "Attractor failed to draw any pixels");
    }
}
