//! Procedural Mandelbrot Renderer
//!
//! A procedural renderer that computes and visualizes the Mandelbrot set fractal.
//! Uses Rayon to massively parallelize the per-pixel calculation.

use crate::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Mandelbrot fractal renderer.
#[derive(Debug, Clone, Copy)]
pub struct MandelbrotConfig {
    /// The real component of the center point.
    pub center_x: f64,
    /// The imaginary component of the center point.
    pub center_y: f64,
    /// The zoom level (higher is closer).
    pub zoom: f64,
    /// The maximum number of iterations.
    pub max_iterations: u32,
    /// Color multiplier for creating cool palettes.
    pub color_mult: f32,
}

impl Default for MandelbrotConfig {
    fn default() -> Self {
        Self {
            center_x: -0.75, // Standard view center
            center_y: 0.0,
            zoom: 1.0,
            max_iterations: 100,
            color_mult: 0.05,
        }
    }
}

/// Renders the Mandelbrot set into the framebuffer using the given configuration.
pub fn render_mandelbrot(fb: &mut Framebuffer, config: &MandelbrotConfig) {
    let width = fb.width();
    let height = fb.height();
    let aspect = f64::from(width) / f64::from(height);

    // Viewport calculation.
    // Base width is roughly 3.5 (from -2.5 to 1.0)
    let view_w = 3.5 / config.zoom;
    let view_h = view_w / aspect;

    let x_min = config.center_x - view_w / 2.0;
    let y_min = config.center_y - view_h / 2.0;

    let dx = view_w / f64::from(width);
    let dy = view_h / f64::from(height);

    // We can use rayon to render rows in parallel
    #[cfg(feature = "parallel")]
    let chunks = fb.as_mut_slice().par_chunks_exact_mut(width as usize);

    #[cfg(not(feature = "parallel"))]
    let chunks = fb.as_mut_slice().chunks_exact_mut(width as usize);

    chunks.enumerate().for_each(|(y, row)| {
        let cy = y_min + (y as f64) * dy;

        for (x, pixel) in row.iter_mut().enumerate() {
            let cx = x_min + (x as f64) * dx;

            // Z = Z^2 + C
            let mut zx = 0.0;
            let mut zy = 0.0;
            let mut iter = 0;

            // Fast bail out
            let max_iters = config.max_iterations;

            while zx * zx + zy * zy <= 4.0 && iter < max_iters {
                let tmp = zx * zx - zy * zy + cx;
                zy = 2.0 * zx * zy + cy;
                zx = tmp;
                iter += 1;
            }

            // Map iteration count to a color palette
            if iter == max_iters {
                *pixel = 0xFF_000000; // Black for inside the set
            } else {
                // Smooth coloring trick (optional, but let's keep it simple for now)
                // Let's use a simple procedural palette based on iter count
                let t = (iter as f32) * config.color_mult;
                let r = ((t * std::f32::consts::PI).sin() * 127.0 + 128.0) as u32;
                let g = ((t * std::f32::consts::PI * 1.5).sin() * 127.0 + 128.0) as u32;
                let b = ((t * std::f32::consts::PI * 2.0).sin() * 127.0 + 128.0) as u32;

                // Pack ARGB manually or use color struct
                *pixel = (0xFF << 24) | (r << 16) | (g << 8) | b;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mandelbrot_inside_set() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        // Set center deep inside the set where everything should be black
        let config = MandelbrotConfig {
            center_x: 0.0,
            center_y: 0.0,
            zoom: 100.0,
            max_iterations: 100,
            ..Default::default()
        };

        render_mandelbrot(&mut fb, &config);

        // Everything should be black
        for &pixel in fb.as_slice().iter() {
            assert_eq!(pixel, 0xFF_000000);
        }
    }

    #[test]
    fn test_mandelbrot_outside_set() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        // Set center way outside the set
        let config = MandelbrotConfig {
            center_x: 10.0,
            center_y: 10.0,
            zoom: 1.0,
            max_iterations: 100,
            ..Default::default()
        };

        render_mandelbrot(&mut fb, &config);

        // At least some pixels should not be black
        let has_color = fb.as_slice().iter().any(|&pixel| pixel != 0xFF_000000);
        assert!(has_color);
    }
}
