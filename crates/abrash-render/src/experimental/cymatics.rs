//! # Cymatics (Chladni Plate) Generator
//!
//! Visualizes acoustic standing waves on a 2D plate (Chladni patterns).

use crate::framebuffer::Framebuffer;
use std::f32::consts::PI;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Cymatics generator.
#[derive(Debug, Clone, Copy)]
pub struct CymaticsConfig {
    /// Modal frequency `n`
    pub n: f32,
    /// Modal frequency `m`
    pub m: f32,
    /// Thickness of the sand lines
    pub thickness: f32,
    /// The color of the sand (nodes)
    pub sand_color: u32,
    /// The color of the plate (anti-nodes)
    pub plate_color: u32,
}

impl Default for CymaticsConfig {
    fn default() -> Self {
        Self {
            n: 3.0,
            m: 5.0,
            thickness: 0.05,
            sand_color: 0xFFFF_EECC,  // Sandy color
            plate_color: 0xFF11_1111, // Dark plate
        }
    }
}

/// Renders a Chladni plate pattern onto the given framebuffer.
///
/// Simulates acoustic standing waves where sand gathers at the nodal lines
/// (areas of zero amplitude).
///
/// # Arguments
///
/// * `fb` - The framebuffer to render onto.
/// * `config` - Configuration for the Chladni parameters and colors.
pub fn render_cymatics(fb: &mut Framebuffer, config: &CymaticsConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let w_f32 = width as f32;
    let h_f32 = height as f32;

    let n_pi = config.n * PI;
    let m_pi = config.m * PI;

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let row_iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = pixels.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row)| {
        // Map y to [-1, 1]
        let norm_y = (y as f32 / h_f32) * 2.0 - 1.0;

        let sin_n_y = (n_pi * norm_y).sin();
        let sin_m_y = (m_pi * norm_y).sin();

        for (x, pixel) in row.iter_mut().enumerate() {
            // Map x to [-1, 1]
            let norm_x = (x as f32 / w_f32) * 2.0 - 1.0;

            let sin_n_x = (n_pi * norm_x).sin();
            let sin_m_x = (m_pi * norm_x).sin();

            // Chladni equation for a square plate:
            // L(x, y) = sin(n * pi * x) * sin(m * pi * y) - sin(m * pi * x) * sin(n * pi * y)
            let amplitude = sin_n_x * sin_m_y - sin_m_x * sin_n_y;

            if amplitude.abs() <= config.thickness {
                *pixel = config.sand_color;
            } else {
                *pixel = config.plate_color;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cymatics_renders_pattern() {
        let mut fb = Framebuffer::new(20, 20).unwrap();

        let config = CymaticsConfig {
            n: 1.0,
            m: 2.0,
            thickness: 0.1,
            sand_color: 0xFFFF_EECC,
            plate_color: 0xFF11_1111,
        };

        render_cymatics(&mut fb, &config);

        let mut has_sand = false;
        let mut has_plate = false;

        for &pixel in fb.as_slice() {
            if pixel == config.sand_color {
                has_sand = true;
            } else if pixel == config.plate_color {
                has_plate = true;
            }
        }

        assert!(has_sand, "Cymatics should render some sand nodes");
        assert!(has_plate, "Cymatics should render some plate areas");
    }

    #[test]
    fn test_cymatics_zero_dimensions() {
        let mut fb = Framebuffer::new(0, 0).unwrap();
        let config = CymaticsConfig::default();
        // Should not panic
        render_cymatics(&mut fb, &config);
    }
}
