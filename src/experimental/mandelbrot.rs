//! Mandelbrot Set Explorer
//!
//! A procedural texture generator that renders the Mandelbrot set directly into
//! a framebuffer. This effect calculates the escape time for complex numbers
//! and maps the result to a continuous color gradient.

use crate::framebuffer::Framebuffer;

/// Configuration for the Mandelbrot generator.
#[derive(Debug, Clone, Copy)]
pub struct MandelbrotConfig {
    /// The X coordinate of the center of the viewport (real part).
    pub center_x: f64,
    /// The Y coordinate of the center of the viewport (imaginary part).
    pub center_y: f64,
    /// The zoom level. Higher means closer.
    pub zoom: f64,
    /// Maximum number of iterations before bailing out. Higher = more detail but slower.
    pub max_iterations: u32,
    /// Color multiplier for the gradient.
    pub color_shift: f32,
}

impl Default for MandelbrotConfig {
    fn default() -> Self {
        Self {
            center_x: -0.5,
            center_y: 0.0,
            zoom: 1.0,
            max_iterations: 100,
            color_shift: 5.0,
        }
    }
}

/// Generates a Mandelbrot set rendering into the given framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - Parameters controlling the viewport and visual style.
pub fn generate_mandelbrot(fb: &mut Framebuffer, config: &MandelbrotConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let aspect_ratio = width as f64 / height as f64;

    // The default view of the Mandelbrot set is roughly x: [-2.5, 1.0], y: [-1.0, 1.0]
    // A zoom of 1.0 means the vertical range is 2.0
    let scale = 2.0 / config.zoom;
    let x_min = config.center_x - (scale * aspect_ratio) / 2.0;
    let y_min = config.center_y - scale / 2.0;
    let dx = (scale * aspect_ratio) / width as f64;
    let dy = scale / height as f64;
    let max_iter = config.max_iterations;
    let color_shift = config.color_shift;

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        pixels
            .par_chunks_exact_mut(width)
            .enumerate()
            .for_each(|(py, row)| {
                // Map Y pixel to imaginary coordinate
                let y0 = y_min + (py as f64) * dy;

                for (px, pixel) in row.iter_mut().enumerate() {
                    // Map X pixel to real coordinate
                    let x0 = x_min + (px as f64) * dx;

                    let mut x = 0.0;
                    let mut y = 0.0;
                    let mut iteration = 0;

                    // Optimized inner loop: x^2 + y^2 <= 2^2
                    let mut x2 = 0.0;
                    let mut y2 = 0.0;

                    while x2 + y2 <= 4.0 && iteration < max_iter {
                        y = (x + x) * y + y0;
                        x = x2 - y2 + x0;
                        x2 = x * x;
                        y2 = y * y;
                        iteration += 1;
                    }

                    *pixel = colorize(iteration, max_iter, x2 + y2, color_shift);
                }
            });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for py in 0..height {
            let y0 = y_min + (py as f64) * dy;
            let offset = py * width;

            for px in 0..width {
                let x0 = x_min + (px as f64) * dx;

                let mut x = 0.0;
                let mut y = 0.0;
                let mut iteration = 0;

                let mut x2 = 0.0;
                let mut y2 = 0.0;

                while x2 + y2 <= 4.0 && iteration < max_iter {
                    y = (x + x) * y + y0;
                    x = x2 - y2 + x0;
                    x2 = x * x;
                    y2 = y * y;
                    iteration += 1;
                }

                pixels[offset + px] = colorize(iteration, max_iter, x2 + y2, color_shift);
            }
        }
    }
}

/// Maps iteration count to a continuous RGB color gradient.
#[inline(always)]
fn colorize(iteration: u32, max_iter: u32, z_sqr: f64, shift: f32) -> u32 {
    if iteration == max_iter {
        // Interior of the set is black
        return 0xFF00_0000;
    }

    // Continuous smooth coloring
    // nu = log_2(log_2(|z|) / 2)
    // we use base e, so: log(log(|z|) / log(2)) / log(2)
    let log_z = z_sqr.ln() / 2.0; // log(|z|)
    let nu = (log_z / 2.0f64.ln()).ln() / 2.0f64.ln();

    // Smooth iteration count
    let i = (f64::from(iteration) + 1.0 - nu) as f32;

    // Generate an oscillating color gradient based on smoothed iteration count
    let t = i * shift * 0.01;

    // Basic sine wave palette
    let r = ((t.sin() * 0.5 + 0.5) * 255.0) as u32;
    let g = (((t + 1.0).sin() * 0.5 + 0.5) * 255.0) as u32;
    let b = (((t + 2.0).sin() * 0.5 + 0.5) * 255.0) as u32;

    0xFF00_0000 | (r << 16) | (g << 8) | b
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_mandelbrot_generation() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        let config = MandelbrotConfig::default();
        generate_mandelbrot(&mut fb, &config);

        // Basic check: center pixel should probably be black (in the set)
        // At 10x10, the center pixel is near (0,0)
        let pixels = fb.as_slice();
        assert_eq!(pixels[55], 0xFF00_0000);
    }

    #[test]
    fn test_mandelbrot_zoom() {
        let mut fb = Framebuffer::new(20, 20).unwrap();
        let mut config = MandelbrotConfig::default();
        config.zoom = 1000.0; // Zoom in heavily
        config.center_x = 0.2869; // Real part of a seahorse valley
        config.center_y = 0.0142; // Imaginary part
        generate_mandelbrot(&mut fb, &config);

        // Just verify it doesn't crash and generates non-empty data
        let mut non_black = false;
        for &p in fb.as_slice() {
            if p != 0xFF00_0000 {
                non_black = true;
                break;
            }
        }
        assert!(non_black);
    }
}
