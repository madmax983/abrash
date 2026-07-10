//! # Strange Attractor Generator
//!
//! An experimental module that renders chaotic mathematical strange attractors
//! (e.g., Clifford attractors) directly to the framebuffer.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec2;

/// A Clifford Attractor generator.
/// Evaluates:
/// x_{n+1} = sin(a * y_n) + c * cos(a * x_n)
/// y_{n+1} = sin(b * x_n) + d * cos(b * y_n)
#[derive(Debug, Clone, Copy)]
pub struct CliffordAttractor {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub point: Vec2,
    pub color: u32,
    pub iterations_per_frame: usize,
}

impl Default for CliffordAttractor {
    fn default() -> Self {
        Self {
            a: -1.4,
            b: 1.6,
            c: 1.0,
            d: 0.7,
            point: Vec2::new(0.0, 0.0),
            color: 0x05FF_FFFF, // low alpha white for blending
            iterations_per_frame: 100_000,
        }
    }
}

impl CliffordAttractor {
    /// Renders the next N iterations of the attractor into the framebuffer.
    pub fn render(&mut self, fb: &mut Framebuffer) {
        let w = fb.width() as f32;
        let h = fb.height() as f32;

        // Typical clifford bounds are roughly [-3, 3] depending on params.
        // We will scale to fit inside a [-4, 4] window.
        let scale = w.min(h) / 8.0;
        let offset_x = w / 2.0;
        let offset_y = h / 2.0;

        for _ in 0..self.iterations_per_frame {
            let next_x = (self.a * self.point.y).sin() + self.c * (self.a * self.point.x).cos();
            let next_y = (self.b * self.point.x).sin() + self.d * (self.b * self.point.y).cos();

            self.point = Vec2::new(next_x, next_y);

            let screen_x = (self.point.x * scale + offset_x) as i32;
            let screen_y = (self.point.y * scale + offset_y) as i32;

            if screen_x >= 0
                && screen_x < fb.width() as i32
                && screen_y >= 0
                && screen_y < fb.height() as i32
            {
                let idx = (screen_y as u32 * fb.width() + screen_x as u32) as usize;

                // Extremely simple additive blending for speed.
                let current = unsafe { *fb.as_slice().get_unchecked(idx) };

                let mut r = ((current >> 16) & 0xFF) + ((self.color >> 16) & 0xFF);
                let mut g = ((current >> 8) & 0xFF) + ((self.color >> 8) & 0xFF);
                let mut b = (current & 0xFF) + (self.color & 0xFF);

                if r > 255 {
                    r = 255;
                }
                if g > 255 {
                    g = 255;
                }
                if b > 255 {
                    b = 255;
                }

                let new_color = 0xFF00_0000 | (r << 16) | (g << 8) | b;
                // Avoid using set_pixel_unchecked directly unless available on fb.
                // `pixels` slice is exposed via `as_mut_slice`.
                // Actually `Framebuffer` has `set_pixel(x, y, color)`.
                fb.set_pixel(screen_x as u32, screen_y as u32, new_color);
            }
        }
    }
}

/// Fades the framebuffer by subtracting a small amount from RGB channels, leaving trails.
pub fn fade_framebuffer(fb: &mut Framebuffer, amount: u32) {
    let amount_r = (amount >> 16) & 0xFF;
    let amount_g = (amount >> 8) & 0xFF;
    let amount_b = amount & 0xFF;

    for pixel in fb.as_mut_slice().iter_mut() {
        let mut r = (*pixel >> 16) & 0xFF;
        let mut g = (*pixel >> 8) & 0xFF;
        let mut b = *pixel & 0xFF;

        r = r.saturating_sub(amount_r);
        g = g.saturating_sub(amount_g);
        b = b.saturating_sub(amount_b);

        *pixel = 0xFF00_0000 | (r << 16) | (g << 8) | b;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clifford_attractor_draws() {
        let mut fb = Framebuffer::new(200, 200).unwrap();
        fb.clear(0xFF00_0000);

        let mut attractor = CliffordAttractor::default();
        attractor.iterations_per_frame = 1000;
        attractor.render(&mut fb);

        let has_pixels = fb.as_slice().iter().any(|&p| p != 0xFF00_0000);
        assert!(has_pixels);
    }

    #[test]
    fn test_fade_framebuffer() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFFFF_FFFF); // White

        fade_framebuffer(&mut fb, 0x000A_0A0A);

        let center = fb.get_pixel(5, 5).unwrap();
        let r = (center >> 16) & 0xFF;
        let g = (center >> 8) & 0xFF;
        let b = center & 0xFF;

        assert_eq!(r, 255 - 10);
        assert_eq!(g, 255 - 10);
        assert_eq!(b, 255 - 10);
    }
}
