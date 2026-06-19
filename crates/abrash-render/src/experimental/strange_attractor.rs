//! # Strange Attractor Generator
//!
//! An experimental procedural renderer that simulates chaotic mathematical systems
//! (like the Lorenz attractor) using numerical integration.
//!
//! The points of the system are tracked via an intermediate 2D hit density map,
//! and logarithmic tonemapping is applied to write colors to the framebuffer,
//! highlighting fine details and preventing oversaturation.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec3;

/// Types of strange attractors supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttractorType {
    /// Lorenz strange attractor.
    Lorenz,
}

/// A generator for 3D chaotic strange attractors.
pub struct StrangeAttractor {
    pub attractor_type: AttractorType,
    pub iterations: usize,
    pub dt: f32,
    pub scale: f32,
    pub base_color: u32,
}

impl StrangeAttractor {
    /// Create a new strange attractor generator.
    #[must_use]
    pub const fn new(attractor_type: AttractorType) -> Self {
        Self {
            attractor_type,
            iterations: 10_000_000,
            dt: 0.001,
            scale: 10.0,
            base_color: 0xFF_00_FF_00, // Green
        }
    }

    /// Renders the attractor into the framebuffer.
    pub fn render(&self, fb: &mut Framebuffer) {
        let width = fb.width() as usize;
        let height = fb.height() as usize;

        if width == 0 || height == 0 {
            return;
        }

        // 1. Build Hit Density Map
        // We use an intermediate grid to track how many times a path hits each pixel.
        // This avoids oversaturating bright spots immediately and allows us to tonemap later.
        let mut density_grid = vec![0.0f32; width * height];
        let mut max_density = 0.0f32;

        let center_x = width as f32 * 0.5;
        let center_y = height as f32 * 0.5;

        // Lorenz Attractor standard parameters
        let sigma = 10.0;
        let rho = 28.0;
        let beta = 8.0 / 3.0;

        // Initial conditions (must not be exactly 0,0,0)
        let mut p = Vec3::new(0.1, 0.0, 0.0);

        for _ in 0..self.iterations {
            let dx = sigma * (p.y - p.x);
            let dy = p.x * (rho - p.z) - p.y;
            let dz = p.x * p.y - beta * p.z;

            p.x += dx * self.dt;
            p.y += dy * self.dt;
            p.z += dz * self.dt;

            // Map 3D point to 2D screen coordinates.
            // For Lorenz, the typical view looks good projected down the Y axis,
            // mapping X to screen X, and Z to screen Y (inverted to match screen coords).
            // We subtract an offset (like 28.0) from Z so it centers nicely.
            let screen_x = center_x + p.x * self.scale;
            let screen_y = center_y + (p.z - 28.0) * self.scale;

            let px = screen_x as i32;
            let py = screen_y as i32;

            if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                let idx = py as usize * width + px as usize;
                // Add a small amount of density per hit.
                density_grid[idx] += 1.0;
                if density_grid[idx] > max_density {
                    max_density = density_grid[idx];
                }
            }
        }

        // 2. Resolve and Tonemap to Framebuffer
        // Prevent divide-by-zero if max_density is very small
        let log_max = max_density.ln_1p().max(0.0001);

        let (base_r, base_g, base_b) = (
            ((self.base_color >> 16) & 0xFF) as f32,
            ((self.base_color >> 8) & 0xFF) as f32,
            (self.base_color & 0xFF) as f32,
        );

        let pixels = fb.as_mut_slice();
        for (i, density) in density_grid.into_iter().enumerate() {
            if density > 0.0 {
                // Logarithmic tonemapping: highly dense areas approach 1.0 slowly.
                let normalized = density.ln_1p() / log_max;

                // Enhance visibility of weak hits by applying a simple curve
                let mapped = normalized.powf(0.5).clamp(0.0, 1.0);

                let r = (base_r * mapped) as u32;
                let g = (base_g * mapped) as u32;
                let b = (base_b * mapped) as u32;

                // Additive blend over the existing framebuffer color.
                let existing = pixels[i];
                let ex_r = (existing >> 16) & 0xFF;
                let ex_g = (existing >> 8) & 0xFF;
                let ex_b = existing & 0xFF;

                let out_r = (ex_r + r).min(255);
                let out_g = (ex_g + g).min(255);
                let out_b = (ex_b + b).min(255);

                pixels[i] = 0xFF_00_00_00 | (out_r << 16) | (out_g << 8) | out_b;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::framebuffer::Framebuffer;

    #[test]
    fn test_render_modifies_framebuffer() {
        let mut fb = Framebuffer::new(200, 200).unwrap();
        fb.clear(0xFF_00_00_00); // Clear to black

        let mut attractor = StrangeAttractor::new(AttractorType::Lorenz);
        attractor.iterations = 1000; // Small iterations for fast test
        attractor.render(&mut fb);

        // Verify that *some* pixel has been changed from black.
        let has_drawn_pixel = fb.as_slice().iter().any(|&p| p != 0xFF_00_00_00);
        assert!(
            has_drawn_pixel,
            "Strange attractor should have drawn on the framebuffer"
        );
    }
}
