//! # Strange Attractor Generators
//!
//! An experimental module that renders Strange Attractors (like Lorenz or Roessler)
//! by evaluating chaotic differential equations using simple numerical integration
//! (Euler method) and projecting the resulting 3D geometry onto a 2D framebuffer.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec3;

/// Types of Strange Attractors supported.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AttractorType {
    /// Lorenz Attractor: A classic chaotic system.
    Lorenz { sigma: f32, rho: f32, beta: f32 },
    /// Roessler Attractor: Known for its continuous single-band chaos.
    Roessler { a: f32, b: f32, c: f32 },
}

impl Default for AttractorType {
    fn default() -> Self {
        Self::Lorenz {
            sigma: 10.0,
            rho: 28.0,
            beta: 8.0 / 3.0,
        }
    }
}

/// A Strange Attractor generator.
#[derive(Debug, Clone)]
pub struct StrangeAttractor {
    pub attractor_type: AttractorType,
    pub step_size: f32,
    pub iterations: usize,
    pub color: u32,
    pub scale: f32,
    pub start_pos: Vec3,
}

impl Default for StrangeAttractor {
    fn default() -> Self {
        Self {
            attractor_type: AttractorType::default(),
            step_size: 0.005,
            iterations: 50000,
            color: 0xFF_00_FF_00,
            scale: 10.0,
            start_pos: Vec3::new(0.1, 0.0, 0.0),
        }
    }
}

impl StrangeAttractor {
    /// Evaluates the derivative for the given attractor at point `p`.
    #[must_use]
    pub fn derivative(&self, p: Vec3) -> Vec3 {
        match self.attractor_type {
            AttractorType::Lorenz { sigma, rho, beta } => {
                let dx = sigma * (p.y - p.x);
                let dy = p.x * (rho - p.z) - p.y;
                let dz = p.x * p.y - beta * p.z;
                Vec3::new(dx, dy, dz)
            }
            AttractorType::Roessler { a, b, c } => {
                let dx = -p.y - p.z;
                let dy = p.x + a * p.y;
                let dz = b + p.z * (p.x - c);
                Vec3::new(dx, dy, dz)
            }
        }
    }

    /// Renders the attractor to the given framebuffer, projecting 3D to 2D.
    pub fn render(&self, fb: &mut Framebuffer) {
        let center_x = fb.width() as f32 / 2.0;
        let center_y = fb.height() as f32 / 2.0;

        let mut p = self.start_pos;

        // Initial projection (orthographic on XY for Lorenz, XY for Roessler)
        let mut prev_screen_x = (center_x + p.x * self.scale) as i32;
        let mut prev_screen_y = match self.attractor_type {
            AttractorType::Lorenz { .. } => {
                (center_y - p.z * self.scale + 25.0 * self.scale) as i32
            }
            AttractorType::Roessler { .. } => (center_y - p.y * self.scale) as i32,
        };

        for _ in 0..self.iterations {
            let dp = self.derivative(p);
            p = p + dp * self.step_size;

            let screen_x = (center_x + p.x * self.scale) as i32;
            let screen_y = match self.attractor_type {
                // Shift Lorenz down a bit since it's mostly positive Z
                AttractorType::Lorenz { .. } => {
                    (center_y - p.z * self.scale + 25.0 * self.scale) as i32
                }
                AttractorType::Roessler { .. } => (center_y - p.y * self.scale) as i32,
            };

            draw_line_2d(
                fb,
                prev_screen_x,
                prev_screen_y,
                screen_x,
                screen_y,
                self.color,
            );

            prev_screen_x = screen_x;
            prev_screen_y = screen_y;
        }
    }
}

/// A fast integer-based 2D line drawing function using Bresenham's algorithm.
fn draw_line_2d(fb: &mut Framebuffer, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: u32) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if x0 >= 0 && x0 < fb.width() as i32 && y0 >= 0 && y0 < fb.height() as i32 {
            unsafe {
                fb.set_pixel_unchecked(x0 as usize, y0 as usize, color);
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lorenz_derivative() {
        let attractor = StrangeAttractor {
            attractor_type: AttractorType::Lorenz {
                sigma: 10.0,
                rho: 28.0,
                beta: 8.0 / 3.0,
            },
            ..Default::default()
        };
        let p = Vec3::new(1.0, 2.0, 3.0);
        let dp = attractor.derivative(p);

        // dx = 10 * (2 - 1) = 10
        // dy = 1 * (28 - 3) - 2 = 23
        // dz = 1*2 - 8/3 * 3 = 2 - 8 = -6
        assert!((dp.x - 10.0).abs() < 1e-5);
        assert!((dp.y - 23.0).abs() < 1e-5);
        assert!((dp.z + 6.0).abs() < 1e-5);
    }

    #[test]
    fn test_roessler_derivative() {
        let attractor = StrangeAttractor {
            attractor_type: AttractorType::Roessler {
                a: 0.2,
                b: 0.2,
                c: 5.7,
            },
            ..Default::default()
        };
        let p = Vec3::new(1.0, 2.0, 3.0);
        let dp = attractor.derivative(p);

        // dx = -2 - 3 = -5
        // dy = 1 + 0.2 * 2 = 1.4
        // dz = 0.2 + 3 * (1 - 5.7) = 0.2 - 14.1 = -13.9
        assert!((dp.x + 5.0).abs() < 1e-5);
        assert!((dp.y - 1.4).abs() < 1e-5);
        assert!((dp.z + 13.9).abs() < 1e-5);
    }

    #[test]
    fn test_attractor_render() {
        let mut fb = Framebuffer::new(600, 600).unwrap();
        fb.clear(0xFF_00_00_00);

        let mut attr = StrangeAttractor::default();
        attr.iterations = 100;
        attr.color = 0xFF_FF_FF_FF;
        attr.render(&mut fb);

        let has_pixels = fb.as_slice().iter().any(|&p| p == 0xFF_FF_FF_FF);
        assert!(
            has_pixels,
            "Attractor should have drawn something on the screen"
        );
    }
}
