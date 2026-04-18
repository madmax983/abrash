//! Spirograph Generator
//!
//! A procedural generator for hypotrochoid and epitrochoid curves (Spirograph).

use abrash_core::framebuffer::Framebuffer;
use std::f32::consts::PI;

/// Configuration for the Spirograph generation.
#[derive(Debug, Clone, Copy)]
pub struct SpirographConfig {
    /// Radius of the fixed circle (R)
    pub fixed_radius: f32,
    /// Radius of the moving circle (r). Negative for epitrochoids.
    pub moving_radius: f32,
    /// Distance of the pen point from the center of the moving circle (rho)
    pub pen_offset: f32,
    /// Color of the spirograph line
    pub color: u32,
    /// Resolution (number of steps per full rotation)
    pub resolution: usize,
    /// Number of total rotations
    pub rotations: usize,
    /// Center of the drawing on the framebuffer
    pub center_x: i32,
    pub center_y: i32,
}

impl Default for SpirographConfig {
    fn default() -> Self {
        Self {
            fixed_radius: 100.0,
            moving_radius: 20.0,
            pen_offset: 40.0,
            color: 0xFF_FFFFFF,
            resolution: 100,
            rotations: 10,
            center_x: 0,
            center_y: 0,
        }
    }
}

/// Simple Bresenham's line algorithm for 2D framebuffer lines
fn draw_line_2d(fb: &mut Framebuffer, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = x0;
    let mut y = y0;

    let w = fb.width() as i32;
    let h = fb.height() as i32;

    loop {
        if x >= 0 && x < w && y >= 0 && y < h {
            fb.set_pixel(x, y, color);
        }

        if x == x1 && y == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// Draws a spirograph on the framebuffer based on the provided configuration.
pub fn draw_spirograph(fb: &mut Framebuffer, config: &SpirographConfig) {
    let r_fixed = config.fixed_radius;
    let r_moving = config.moving_radius;
    let rho = config.pen_offset;

    let dt = (2.0 * PI) / config.resolution as f32;
    let total_steps = config.resolution * config.rotations;

    let mut prev_x = 0;
    let mut prev_y = 0;

    for step in 0..=total_steps {
        let t = step as f32 * dt;

        let x = (r_fixed - r_moving) * t.cos() + rho * ((r_fixed - r_moving) / r_moving * t).cos();
        let y = (r_fixed - r_moving) * t.sin() - rho * ((r_fixed - r_moving) / r_moving * t).sin();

        let px = config.center_x + x as i32;
        let py = config.center_y + y as i32;

        if step > 0 {
            draw_line_2d(fb, prev_x, prev_y, px, py, config.color);
        }

        prev_x = px;
        prev_y = py;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spirograph_default() {
        let config = SpirographConfig::default();
        assert_eq!(config.fixed_radius, 100.0);
    }

    #[test]
    fn test_draw_spirograph() {
        let mut fb = Framebuffer::new(200, 200).unwrap();
        let config = SpirographConfig {
            fixed_radius: 50.0,
            moving_radius: 10.0,
            pen_offset: 15.0,
            center_x: 100,
            center_y: 100,
            ..Default::default()
        };
        draw_spirograph(&mut fb, &config);

        // Ensure at least some pixels were drawn with the config color.
        let mut drawn = false;
        for y in 0..200 {
            for x in 0..200 {
                if fb.get_pixel(x, y).unwrap() == config.color {
                    drawn = true;
                    break;
                }
            }
        }
        assert!(drawn, "Spirograph did not draw any pixels");
    }
}
