//! # Spirograph Generator
//!
//! An experimental module that generates roulette curves like hypotrochoids and epitrochoids
//! simulating a classic Spirograph toy.

use abrash_core::framebuffer::Framebuffer;
use std::f32::consts::PI;

/// A Spirograph generator.
#[derive(Debug, Clone, Copy)]
pub struct Spirograph {
    /// Radius of the fixed circle.
    pub r_fixed: f32,
    /// Radius of the moving circle.
    pub r_moving: f32,
    /// Distance of the pen point from the center of the moving circle.
    pub pen_offset: f32,
    /// Whether the moving circle is inside the fixed circle (hypotrochoid) or outside (epitrochoid).
    pub inside: bool,
    /// The color to draw the curve.
    pub color: u32,
    /// Number of segments to draw per full rotation.
    pub resolution: usize,
    /// How many rotations to perform.
    pub rotations: usize,
}

impl Spirograph {
    /// Generates the (x, y) coordinate at angle `t` (in radians).
    #[must_use]
    pub fn position_at(&self, t: f32) -> (f32, f32) {
        let r = self.r_fixed;
        let r_m = self.r_moving;
        let d = self.pen_offset;

        if self.inside {
            // Hypotrochoid
            let diff = r - r_m;
            let x = diff * t.cos() + d * ((diff / r_m) * t).cos();
            let y = diff * t.sin() - d * ((diff / r_m) * t).sin();
            (x, y)
        } else {
            // Epitrochoid
            let sum = r + r_m;
            let x = sum * t.cos() - d * ((sum / r_m) * t).cos();
            let y = sum * t.sin() - d * ((sum / r_m) * t).sin();
            (x, y)
        }
    }

    /// Draws the spirograph onto the given framebuffer at the specified center.
    pub fn draw(&self, fb: &mut Framebuffer, center_x: i32, center_y: i32) {
        let steps = self.resolution * self.rotations;
        let dt = (2.0 * PI) / (self.resolution as f32);

        let mut prev_point = None;

        for i in 0..=steps {
            let t = (i as f32) * dt;
            let (x, y) = self.position_at(t);
            let px = center_x + (x as i32);
            let py = center_y + (y as i32);

            if let Some((px0, py0)) = prev_point {
                draw_line_2d(fb, px0, py0, px, py, self.color);
            }
            prev_point = Some((px, py));
        }
    }
}

/// Bresenham's line algorithm.
fn draw_line_2d(fb: &mut Framebuffer, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: u32) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        fb.set_pixel(x0, y0, color);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hypotrochoid_points() {
        let s = Spirograph {
            r_fixed: 100.0,
            r_moving: 25.0,
            pen_offset: 10.0,
            inside: true,
            color: 0xFFFFFFFF,
            resolution: 100,
            rotations: 1,
        };

        // t = 0
        let (x0, y0) = s.position_at(0.0);
        assert!((x0 - 85.0).abs() < 0.01);
        assert!((y0 - 0.0).abs() < 0.01);
    }
}
