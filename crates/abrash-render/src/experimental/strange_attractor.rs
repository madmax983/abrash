//! # Strange Attractor Generator
//!
//! An experimental module that generates strange attractors (like the Clifford Attractor)
//! and renders them using histogram-based density accumulation.

use crate::framebuffer::Framebuffer;
use crate::math::Vec2;

/// Configuration for a Clifford Attractor
#[derive(Debug, Clone)]
pub struct CliffordAttractor {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub color_base: u32,
    pub iterations: usize,
}

impl Default for CliffordAttractor {
    fn default() -> Self {
        Self {
            a: -1.4,
            b: 1.6,
            c: 1.0,
            d: 0.7,
            color_base: 0xFF_00_FF_FF,
            iterations: 1_000_000,
        }
    }
}

impl CliffordAttractor {
    /// Evaluates the next position of the attractor given the current position.
    #[must_use]
    pub fn next_pos(&self, pos: Vec2) -> Vec2 {
        let x_new = (self.a * pos.y).sin() + self.c * (self.a * pos.x).cos();
        let y_new = (self.b * pos.x).sin() + self.d * (self.b * pos.y).cos();
        Vec2::new(x_new, y_new)
    }

    /// Renders the attractor to the given framebuffer using a histogram accumulation pass.
    pub fn render(&self, fb: &mut Framebuffer) {
        let width = fb.width() as usize;
        let height = fb.height() as usize;

        if width == 0 || height == 0 {
            return;
        }

        let mut histogram = vec![0u32; width * height];
        let mut max_density = 0u32;

        let mut pos = Vec2::new(0.0, 0.0);

        let center_x = width as f32 / 2.0;
        let center_y = height as f32 / 2.0;
        let scale = (width.min(height) as f32) / 6.0;

        for _ in 0..self.iterations {
            pos = self.next_pos(pos);

            let screen_x = (center_x + pos.x * scale) as i32;
            let screen_y = (center_y + pos.y * scale) as i32;

            if screen_x >= 0 && screen_x < width as i32 && screen_y >= 0 && screen_y < height as i32
            {
                let idx = screen_y as usize * width + screen_x as usize;
                histogram[idx] += 1;
                if histogram[idx] > max_density {
                    max_density = histogram[idx];
                }
            }
        }

        if max_density == 0 {
            return;
        }

        let log_max = (max_density as f32).ln_1p();
        let r_base = ((self.color_base >> 16) & 0xFF) as f32;
        let g_base = ((self.color_base >> 8) & 0xFF) as f32;
        let b_base = (self.color_base & 0xFF) as f32;

        let pixels = fb.as_mut_slice();
        for (idx, &count) in histogram.iter().enumerate() {
            if count > 0 {
                let log_count = (count as f32).ln_1p();
                let intensity = log_count / log_max;

                let r = (r_base * intensity).clamp(0.0, 255.0) as u32;
                let g = (g_base * intensity).clamp(0.0, 255.0) as u32;
                let b = (b_base * intensity).clamp(0.0, 255.0) as u32;

                let current_color = pixels[idx];
                let current_r = (current_color >> 16) & 0xFF;
                let current_g = (current_color >> 8) & 0xFF;
                let current_b = current_color & 0xFF;

                let new_r = (current_r + r).min(255);
                let new_g = (current_g + g).min(255);
                let new_b = (current_b + b).min(255);

                pixels[idx] = 0xFF_00_00_00 | (new_r << 16) | (new_g << 8) | new_b;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_clifford_next_pos() {
        let attr = CliffordAttractor::default();
        let pos = attr.next_pos(Vec2::new(0.0, 0.0));
        assert!((pos.x - 1.0).abs() < 1e-5);
        assert!((pos.y - 0.7).abs() < 1e-5);
    }

    #[test]
    fn test_render_modifies_fb() {
        let mut fb = Framebuffer::new(100, 100).unwrap();

        let mut attr = CliffordAttractor::default();
        attr.iterations = 1000;
        attr.render(&mut fb);

        let has_color = fb.as_mut_slice().iter().any(|&p| p != 0xFF_00_00_00);
        assert!(has_color, "Framebuffer should not be entirely black");
    }
}
