//! Strange Attractor (Peter de Jong) rendering

use abrash_core::framebuffer::Framebuffer;
use std::f32::consts::PI;

/// Configuration for rendering the Peter de Jong attractor.
pub struct AttractorConfig {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub iterations: u32,
}

impl Default for AttractorConfig {
    fn default() -> Self {
        Self {
            a: 1.4,
            b: -2.3,
            c: 2.4,
            d: -2.1,
            iterations: 1_000_000,
        }
    }
}

/// Renders a Peter de Jong strange attractor to the given framebuffer.
///
/// Iterates the equations:
/// `x_new = sin(a * y) - cos(b * x)`
/// `y_new = sin(c * x) - cos(d * y)`
///
/// It builds a 2D density histogram, mapping the calculated coordinates (which fall
/// roughly in the `[-2.0, 2.0]` range) to the framebuffer dimensions.
/// Finally, the histogram is mapped logarithmically to pixel luminance to produce a glowing effect.
pub fn render_attractor(fb: &mut Framebuffer, config: &AttractorConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    // Using a 1D vector to avoid 2D vec allocations for the histogram.
    let mut histogram = vec![0u32; width * height];
    let mut max_density = 0u32;

    let mut x: f32 = 0.0;
    let mut y: f32 = 0.0;

    // Peter de Jong attractor values are theoretically bounded by [-2.0, 2.0]
    // Since max val of sin/cos is 1, sin(ay)-cos(bx) is in [-2.0, 2.0]
    let range_min = -2.0;
    let range_max = 2.0;
    let range = range_max - range_min;

    for _ in 0..config.iterations {
        let x_new = (config.a * y).sin() - (config.b * x).cos();
        let y_new = (config.c * x).sin() - (config.d * y).cos();

        x = x_new;
        y = y_new;

        // Map to screen coordinates
        // Using normalize: (val - min) / range -> [0.0, 1.0]
        let norm_x = (x - range_min) / range;
        let norm_y = (y - range_min) / range;

        // Protect against edge case float inaccuracies, though mathematically
        // bounded, floating point can slightly exceed bounds.
        if norm_x >= 0.0 && norm_x < 1.0 && norm_y >= 0.0 && norm_y < 1.0 {
            let px = (norm_x * width as f32) as usize;
            let py = (norm_y * height as f32) as usize;

            let idx = py * width + px;
            histogram[idx] += 1;

            if histogram[idx] > max_density {
                max_density = histogram[idx];
            }
        }
    }

    if max_density == 0 {
        return; // Nothing to render
    }

    // Map histogram to colors
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let density = histogram[idx];

            if density > 0 {
                // Logarithmic mapping for glowing effect, common for fractal/attractor density plots
                let intensity = (density as f32).ln_1p() / (max_density as f32).ln_1p();

                // Map intensity [0.0, 1.0] to an ethereal cyan/blue gradient
                let r = (intensity * 50.0).min(255.0) as u32;
                let g = (intensity * 150.0).min(255.0) as u32;
                let b = (intensity * 255.0).min(255.0) as u32;

                let color = 0xFF_00_00_00 | (r << 16) | (g << 8) | b;

                // Using unchecked since we are strictly inside the buffer dimension loops
                unsafe {
                    fb.set_pixel_unchecked(x, y, color);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strange_attractor_modifies_framebuffer() {
        let mut fb = Framebuffer::new(64, 64).unwrap();
        fb.clear(0);

        let config = AttractorConfig {
            iterations: 10_000, // Small iteration count for test
            ..Default::default()
        };

        render_attractor(&mut fb, &config);

        let mut has_non_zero = false;
        for y in 0..64 {
            for x in 0..64 {
                if fb.get_pixel(x, y).unwrap() != 0 {
                    has_non_zero = true;
                    break;
                }
            }
            if has_non_zero {
                break;
            }
        }

        assert!(
            has_non_zero,
            "Framebuffer should be modified by the strange attractor renderer"
        );
    }
}
