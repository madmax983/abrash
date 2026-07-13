//! Procedural Lightning Generator.
//!
//! A retro effect that generates a fractal branching lightning bolt
//! using L-system like branching and `XorShift32` RNG.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::XorShift32;

/// Configuration for the Lightning effect.
#[derive(Debug, Clone, Copy)]
pub struct LightningConfig {
    /// The color of the lightning bolt.
    pub color: u32,
    /// The starting point X coordinate.
    pub start_x: f32,
    /// The starting point Y coordinate.
    pub start_y: f32,
    /// The ending point X coordinate.
    pub end_x: f32,
    /// The ending point Y coordinate.
    pub end_y: f32,
    /// The amount of branching and jaggedness.
    pub jaggedness: f32,
}

impl Default for LightningConfig {
    fn default() -> Self {
        Self {
            color: 0xFF_AA_CC_FF,
            start_x: 0.5,
            start_y: 0.0,
            end_x: 0.5,
            end_y: 1.0,
            jaggedness: 0.2,
        }
    }
}

/// State for the Lightning effect.
pub struct LightningFilter {
    pub config: LightningConfig,
    rng: XorShift32,
}

impl Default for LightningFilter {
    fn default() -> Self {
        Self {
            config: LightningConfig::default(),
            rng: XorShift32::new(0xDEAD_BEEF),
        }
    }
}

impl LightningFilter {
    #[must_use]
    pub const fn new(config: LightningConfig) -> Self {
        Self {
            config,
            rng: XorShift32::new(0xDEAD_BEEF),
        }
    }

    /// Generates a fractal lightning bolt on the framebuffer.
    pub fn apply(&mut self, fb: &mut Framebuffer) {
        let w = fb.width() as f32;
        let h = fb.height() as f32;

        let sx = self.config.start_x * w;
        let sy = self.config.start_y * h;
        let ex = self.config.end_x * w;
        let ey = self.config.end_y * h;

        self.draw_lightning(fb, sx, sy, ex, ey, 5);
    }

    fn draw_lightning(
        &mut self,
        fb: &mut Framebuffer,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        depth: u32,
    ) {
        if depth == 0 {
            Self::draw_line_2d(
                fb,
                x0 as i32,
                y0 as i32,
                x1 as i32,
                y1 as i32,
                self.config.color,
            );
            return;
        }

        let mid_x = (x0 + x1) * 0.5;
        let mid_y = (y0 + y1) * 0.5;

        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = dx.hypot(dy);

        // Perpendicular offset
        let nx = -dy / len;
        let ny = dx / len;

        // Random displacement based on jaggedness
        let offset = (self.rng.next_f32() - 0.5) * len * self.config.jaggedness;

        let jx = mid_x + nx * offset;
        let jy = mid_y + ny * offset;

        self.draw_lightning(fb, x0, y0, jx, jy, depth - 1);
        self.draw_lightning(fb, jx, jy, x1, y1, depth - 1);

        // Randomly branch off
        if self.rng.next_f32() < 0.3 {
            let bx = jx + nx * offset * 2.0;
            let by = jy + ny * offset * 2.0;
            self.draw_lightning(fb, jx, jy, bx, by, depth - 1);
        }
    }

    fn draw_line_2d(fb: &mut Framebuffer, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: u32) {
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if x0 >= 0 && x0 < fb.width() as i32 && y0 >= 0 && y0 < fb.height() as i32 {
                unsafe { fb.set_pixel_unchecked(x0 as usize, y0 as usize, color) };
            }
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lightning_filter() {
        let mut fb = Framebuffer::new(32, 32).unwrap();
        fb.clear(0xFF_00_00_00);

        let mut lightning = LightningFilter::default();
        lightning.apply(&mut fb);

        let has_pixels = fb.as_slice().iter().any(|&p| p != 0xFF_00_00_00);
        assert!(
            has_pixels,
            "Lightning filter should draw to the framebuffer"
        );
    }
}
