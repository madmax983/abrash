//! Strange Attractor Generator.
//!
//! Renders strange attractors like the Peter de Jong attractor using iterative mathematical equations
//! and density accumulation to create beautiful, complex fractal-like patterns.

use abrash_core::framebuffer::Framebuffer;

/// Configuration for rendering a Peter de Jong strange attractor.
#[derive(Debug, Clone)]
pub struct AttractorConfig {
    /// Parameter 'a' for the Peter de Jong equation.
    pub a: f32,
    /// Parameter 'b' for the Peter de Jong equation.
    pub b: f32,
    /// Parameter 'c' for the Peter de Jong equation.
    pub c: f32,
    /// Parameter 'd' for the Peter de Jong equation.
    pub d: f32,
    /// The number of iterations (points) to plot.
    pub iterations: u32,
    /// The base color used for the highest density areas (0xAARRGGBB).
    pub color: u32,
    /// The scale factor to control zoom.
    pub scale: f32,
}

impl Default for AttractorConfig {
    fn default() -> Self {
        Self {
            a: 1.4,
            b: -2.3,
            c: 2.4,
            d: -2.1,
            iterations: 1_000_000,
            color: 0xFF_FF_AA_44, // Warm orange/gold
            scale: 0.25,
        }
    }
}

/// Renders a Peter de Jong strange attractor into the given framebuffer.
pub fn render_strange_attractor(fb: &mut Framebuffer, config: &AttractorConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    // Step 1: Accumulate point densities using a flat 1D histogram.
    // Using a flat Vec avoids heavy nested allocation and keeps memory contiguous.
    let mut density = vec![0u32; width * height];
    let mut max_density = 0u32;

    let mut x = 0.0f32;
    let mut y = 0.0f32;

    let w_f32 = width as f32;
    let h_f32 = height as f32;
    let cx = w_f32 * 0.5;
    let cy = h_f32 * 0.5;

    // Scale factor to map the points [-2, 2] to the screen
    let scale_x = w_f32 * config.scale;
    let scale_y = h_f32 * config.scale;

    for _ in 0..config.iterations {
        // Peter de Jong attractor equations:
        // x_{n+1} = sin(a * y_n) - cos(b * x_n)
        // y_{n+1} = sin(c * x_n) - cos(d * y_n)
        let next_x = (config.a * y).sin() - (config.b * x).cos();
        let next_y = (config.c * x).sin() - (config.d * y).cos();

        x = next_x;
        y = next_y;

        // Map to screen coordinates
        let px = (cx + x * scale_x) as i32;
        let py = (cy + y * scale_y) as i32;

        if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
            let idx = (py as usize) * width + (px as usize);
            density[idx] += 1;
            if density[idx] > max_density {
                max_density = density[idx];
            }
        }
    }

    // Step 2: Render to framebuffer, mapping density logarithmically to color intensity.
    if max_density == 0 {
        return;
    }

    // Precalculate logarithmic max density
    let log_max = (max_density as f32).ln_1p();

    // Extract base color components
    let base_a = (config.color >> 24) & 0xFF;
    let base_r = (config.color >> 16) & 0xFF;
    let base_g = (config.color >> 8) & 0xFF;
    let base_b = config.color & 0xFF;

    let pixels = fb.as_mut_slice();

    for (i, &count) in density.iter().enumerate() {
        if count > 0 {
            // Apply logarithmic scaling to highlight sparse areas
            let intensity = (count as f32).ln_1p() / log_max;

            let r = ((base_r as f32) * intensity) as u32;
            let g = ((base_g as f32) * intensity) as u32;
            let b = ((base_b as f32) * intensity) as u32;

            // Additive blending could be done here if blending over existing image,
            // but for now we write directly.
            pixels[i] = (base_a << 24) | (r << 16) | (g << 8) | b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_attractor_0x0() {
        let mut fb = Framebuffer::new(0, 0).unwrap();
        let config = AttractorConfig::default();
        render_strange_attractor(&mut fb, &config);
        assert_eq!(fb.width(), 0);
        assert_eq!(fb.height(), 0);
    }

    #[test]
    fn test_render_attractor_standard() {
        let mut fb = Framebuffer::new(64, 64).unwrap();
        fb.clear(0xFF_00_00_00);
        let config = AttractorConfig {
            iterations: 10_000,
            ..Default::default()
        };

        render_strange_attractor(&mut fb, &config);

        let mut has_non_black = false;
        for &p in fb.as_slice() {
            if p != 0xFF_00_00_00 {
                has_non_black = true;
                break;
            }
        }
        assert!(
            has_non_black,
            "Framebuffer should have some non-black pixels drawn by the attractor"
        );
    }
}
