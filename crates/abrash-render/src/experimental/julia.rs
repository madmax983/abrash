//! Julia Set rendering

use abrash_core::framebuffer::Framebuffer;

/// Configuration for rendering the Julia set.
pub struct JuliaConfig {
    pub center_x: f64,
    pub center_y: f64,
    pub zoom: f64,
    pub max_iterations: u32,
    pub c_re: f64,
    pub c_im: f64,
}

impl Default for JuliaConfig {
    fn default() -> Self {
        Self {
            center_x: 0.0,
            center_y: 0.0,
            zoom: 1.0,
            max_iterations: 100,
            c_re: -0.7,
            c_im: 0.27015,
        }
    }
}

#[inline(always)]
fn calculate_julia_iterations(
    z_re: f64,
    z_im: f64,
    c_re: f64,
    c_im: f64,
    max_iterations: u32,
) -> u32 {
    let mut z_r = z_re;
    let mut z_i = z_im;
    let mut iteration = 0;

    while z_r * z_r + z_i * z_i <= 4.0 && iteration < max_iterations {
        let z_r_new = z_r * z_r - z_i * z_i + c_re;
        z_i = 2.0 * z_r * z_i + c_im;
        z_r = z_r_new;
        iteration += 1;
    }

    iteration
}

#[inline(always)]
fn map_iterations_to_color(iteration: u32, max_iterations: u32) -> u32 {
    if iteration == max_iterations {
        0xFF_000000 // Black for inside the set
    } else {
        // Smooth coloring based on iterations
        let t = f64::from(iteration) / f64::from(max_iterations);

        let r = (9.0 * (1.0 - t) * t * t * t * 255.0) as u32;
        let g = (15.0 * (1.0 - t) * (1.0 - t) * t * t * 255.0) as u32;
        let b = (8.5 * (1.0 - t) * (1.0 - t) * (1.0 - t) * t * 255.0) as u32;

        0xFF_000000 | (r << 16) | (g << 8) | b
    }
}

/// Renders the Julia set to the given framebuffer.
#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Renders the Julia set to the given framebuffer.
pub fn render_julia(fb: &mut Framebuffer, config: &JuliaConfig) {
    let width = f64::from(fb.width());
    let height = f64::from(fb.height());

    let aspect_ratio = width / height;

    // Convert screen pixel (x, y) into a complex point (z_re, z_im)
    let scale_x = 3.5 / config.zoom * aspect_ratio / width;
    let scale_y = 3.5 / config.zoom / height;

    let offset_x = config.center_x - (width / 2.0) * scale_x;
    let offset_y = config.center_y - (height / 2.0) * scale_y;

    #[cfg(feature = "parallel")]
    let row_iter = fb.as_mut_slice().par_chunks_exact_mut(width as usize);
    #[cfg(not(feature = "parallel"))]
    let row_iter = fb.as_mut_slice().chunks_exact_mut(width as usize);

    row_iter.enumerate().for_each(|(y, row)| {
        let z_im = offset_y + (y as f64) * scale_y;

        for (x, pixel) in row.iter_mut().enumerate() {
            let z_re = offset_x + (x as f64) * scale_x;

            let iteration = calculate_julia_iterations(
                z_re,
                z_im,
                config.c_re,
                config.c_im,
                config.max_iterations,
            );

            *pixel = map_iterations_to_color(iteration, config.max_iterations);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_julia_basic() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0);
        let config = JuliaConfig::default();
        render_julia(&mut fb, &config);

        let mut has_non_zero = false;
        for y in 0..10 {
            for x in 0..10 {
                if fb.get_pixel(x, y).unwrap() != 0 {
                    has_non_zero = true;
                }
            }
        }
        assert!(
            has_non_zero,
            "Framebuffer should be modified by the fractal renderer"
        );
    }
}
