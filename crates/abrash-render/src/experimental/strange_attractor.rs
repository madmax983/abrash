//! Strange Attractor generator.
//!
//! Evaluates procedural mathematical chaotic systems like the Clifford Attractor,
//! rendering density-mapped procedural shapes into the framebuffer.

use abrash_core::framebuffer::Framebuffer;

#[derive(Debug, Clone)]
pub struct AttractorConfig {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub iterations: usize,
    pub scale: f64,
    pub color_start: u32,
    pub color_end: u32,
}

impl Default for AttractorConfig {
    fn default() -> Self {
        Self {
            // Default parameters for an interesting Clifford Attractor
            a: -1.4,
            b: 1.6,
            c: 1.0,
            d: 0.7,
            iterations: 1_000_000,
            scale: 0.2,               // ~ 1/5th screen relative
            color_start: 0xFF_000044, // dark blue
            color_end: 0xFF_FFFFFF,   // white core
        }
    }
}

pub struct StrangeAttractor {
    pub config: AttractorConfig,
}

impl StrangeAttractor {
    #[must_use]
    pub const fn new(config: AttractorConfig) -> Self {
        Self { config }
    }

    pub fn render(&self, fb: &mut Framebuffer) {
        let width = fb.width() as usize;
        let height = fb.height() as usize;

        if width == 0 || height == 0 {
            return;
        }

        let mut density_grid = vec![0u32; width * height];
        let mut max_density = 0u32;

        let mut x = 0.0;
        let mut y = 0.0;

        let a = self.config.a;
        let b = self.config.b;
        let c = self.config.c;
        let d = self.config.d;

        let center_x = width as f64 / 2.0;
        let center_y = height as f64 / 2.0;
        let scale = width.min(height) as f64 * self.config.scale;

        // Iterate to generate the attractor
        for _ in 0..self.config.iterations {
            // Clifford Attractor formula
            let x_new = (a * y).sin() + c * (a * x).cos();
            let y_new = (b * x).sin() + d * (b * y).cos();

            x = x_new;
            y = y_new;

            // Map to screen coordinates
            let px = (center_x + x * scale) as i32;
            let py = (center_y + y * scale) as i32;

            if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                let idx = py as usize * width + px as usize;
                density_grid[idx] += 1;
                if density_grid[idx] > max_density {
                    max_density = density_grid[idx];
                }
            }
        }

        if max_density == 0 {
            return;
        }

        // Apply logarithmic tonemapping and colorize
        let log_max = f64::from(max_density).ln_1p();

        let pixels = fb.as_mut_slice();
        for (i, &density) in density_grid.iter().enumerate() {
            if density > 0 {
                let t = f64::from(density).ln_1p() / log_max;

                // Extract RGB from color_start and color_end
                let r_start = f64::from((self.config.color_start >> 16) & 0xFF);
                let g_start = f64::from((self.config.color_start >> 8) & 0xFF);
                let b_start = f64::from(self.config.color_start & 0xFF);

                let r_end = f64::from((self.config.color_end >> 16) & 0xFF);
                let g_end = f64::from((self.config.color_end >> 8) & 0xFF);
                let b_end = f64::from(self.config.color_end & 0xFF);

                // Interpolate colors
                let r = (r_start + (r_end - r_start) * t) as u32;
                let g = (g_start + (g_end - g_start) * t) as u32;
                let b = (b_start + (b_end - b_start) * t) as u32;

                // Additive blend with current framebuffer pixel
                let current_color = pixels[i];
                let cur_r = (current_color >> 16) & 0xFF;
                let cur_g = (current_color >> 8) & 0xFF;
                let cur_b = current_color & 0xFF;

                let new_r = (cur_r + r).min(255);
                let new_g = (cur_g + g).min(255);
                let new_b = (cur_b + b).min(255);

                pixels[i] = 0xFF_000000 | (new_r << 16) | (new_g << 8) | new_b;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attractor_renders_pixels() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF_000000);
        let config = AttractorConfig {
            iterations: 100,
            ..Default::default()
        };
        let attractor = StrangeAttractor::new(config);
        attractor.render(&mut fb);

        let mut has_color = false;
        for &pixel in fb.as_slice() {
            if pixel != 0xFF_000000 {
                has_color = true;
                break;
            }
        }
        assert!(has_color, "Attractor should modify framebuffer");
    }
}
