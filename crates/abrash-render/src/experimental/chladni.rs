//! Chladni Plate Simulator Filter
//!
//! Simulates acoustic resonance patterns (cymatics) on a 2D plate.

use crate::framebuffer::Framebuffer;
use std::f32::consts::PI;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration parameters for the Chladni filter.
#[derive(Debug, Clone, Copy)]
pub struct ChladniConfig {
    /// Harmonic frequency 'm'.
    pub m: f32,
    /// Harmonic frequency 'n'.
    pub n: f32,
    /// Thickness of the visual nodes (0.0 to 1.0).
    pub thickness: f32,
    /// Color of the node lines.
    pub color: u32,
    /// Background color.
    pub bg_color: u32,
}

impl Default for ChladniConfig {
    fn default() -> Self {
        Self {
            m: 1.0,
            n: 2.0,
            thickness: 0.05,
            color: 0xFF_FFFFFF,    // White
            bg_color: 0xFF_000000, // Black
        }
    }
}

/// Applies a Chladni plate resonance visualization to the framebuffer.
///
/// This filter evaluates the equation: `cos(n*pi*x)*cos(m*pi*y) - cos(m*pi*x)*cos(n*pi*y) = 0`
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - Configuration for the Chladni display.
pub fn apply_chladni(fb: &mut Framebuffer, config: &ChladniConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    // Pre-calculate the independent X components to avoid recalculating cosines for every pixel.
    // This turns an O(W*H) math bottleneck into an O(W+H) setup for the separable functions.
    let mut cos_n_x = vec![0.0; width];
    let mut cos_m_x = vec![0.0; width];

    for x in 0..width {
        let nx = (x as f32 / width as f32) * 2.0 - 1.0;
        cos_n_x[x] = (config.n * PI * nx).cos();
        cos_m_x[x] = (config.m * PI * nx).cos();
    }

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let row_iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = pixels.chunks_exact_mut(width).enumerate();

    let thickness = config.thickness;
    let color = config.color;
    let bg_color = config.bg_color;
    let n = config.n;
    let m = config.m;

    row_iter.for_each(|(y, row)| {
        let ny = (y as f32 / height as f32) * 2.0 - 1.0;
        let cos_m_y = (m * PI * ny).cos();
        let cos_n_y = (n * PI * ny).cos();

        for (x, pixel) in row.iter_mut().enumerate() {
            let term1 = cos_n_x[x] * cos_m_y;
            let term2 = cos_m_x[x] * cos_n_y;
            let val = term1 - term2;

            if val.abs() < thickness {
                *pixel = color;
            } else {
                *pixel = bg_color;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chladni_node_patterns() {
        let mut fb = Framebuffer::new(100, 100).unwrap();

        let mut config = ChladniConfig::default();
        config.m = 1.0;
        config.n = 2.0;
        config.thickness = 0.1;
        apply_chladni(&mut fb, &config);
        let count_1_2 = fb.as_slice().iter().filter(|&&p| p == config.color).count();

        let mut fb2 = Framebuffer::new(100, 100).unwrap();
        let mut config2 = ChladniConfig::default();
        config2.m = 3.0;
        config2.n = 5.0;
        config2.thickness = 0.1;
        apply_chladni(&mut fb2, &config2);
        let count_3_5 = fb2
            .as_slice()
            .iter()
            .filter(|&&p| p == config2.color)
            .count();

        assert_ne!(
            count_1_2, count_3_5,
            "Different frequencies should produce different patterns"
        );
        assert!(count_1_2 > 0, "Should draw some nodes");
        assert!(count_3_5 > 0, "Should draw some nodes");
    }

    #[test]
    fn test_chladni_non_square_bounds() {
        let mut fb = Framebuffer::new(200, 50).unwrap();
        let config = ChladniConfig::default();
        apply_chladni(&mut fb, &config);

        let has_nodes = fb.as_slice().iter().any(|&p| p == config.color);
        let has_bg = fb.as_slice().iter().any(|&p| p == config.bg_color);

        assert!(has_nodes, "Should draw some nodes on non-square buffer");
        assert!(has_bg, "Should draw some background on non-square buffer");
    }
}
