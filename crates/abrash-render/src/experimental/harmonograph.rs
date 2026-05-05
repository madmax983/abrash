//! # Harmonograph Generator
//!
//! An experimental module that generates Lissajous curves and complex geometric
//! patterns simulating a mechanical harmonograph (damped pendulums).
//!
//! A harmonograph is a mechanical apparatus that employs pendulums to create a
//! geometric image. The drawings created typically are Lissajous curves, or
//! related drawings of greater complexity.
//!
//! ## Overview
//!
//! The simulation models a number of pendulums swinging, with their movements
//! combined to trace a path.
//!
//! `X(t) = A_1 * sin(f_1 * t + p_1) * e^(-d_1 * t) + A_2 * sin(f_2 * t + p_2) * e^(-d_2 * t)`
//! `Y(t) = A_3 * sin(f_3 * t + p_3) * e^(-d_3 * t) + A_4 * sin(f_4 * t + p_4) * e^(-d_4 * t)`
//!
//! Where:
//! - A is amplitude
//! - f is frequency
//! - p is phase
//! - d is damping

use crate::framebuffer::Framebuffer;
use crate::math::Vec2;
use std::f32::consts::PI;

/// A single damped pendulum component.
#[derive(Debug, Clone, Copy)]
pub struct Pendulum {
    /// Amplitude (size of the swing)
    pub amplitude: f32,
    /// Frequency (speed of the swing)
    pub frequency: f32,
    /// Phase (starting offset, typically in radians)
    pub phase: f32,
    /// Damping (how quickly it slows down over time)
    pub damping: f32,
}

impl Default for Pendulum {
    fn default() -> Self {
        Self {
            amplitude: 100.0,
            frequency: 1.0,
            phase: 0.0,
            damping: 0.001,
        }
    }
}

/// A mechanical harmonograph composed of 4 pendulums (2 for X axis, 2 for Y axis).
#[derive(Debug, Clone)]
pub struct Harmonograph {
    /// Pendulums controlling movement along the X axis
    pub x_pendulums: [Pendulum; 2],
    /// Pendulums controlling movement along the Y axis
    pub y_pendulums: [Pendulum; 2],
    /// Time increment per step
    pub step_size: f32,
    /// Total iterations to simulate
    pub iterations: usize,
    /// The color of the drawn line
    pub color: u32,
}

impl Default for Harmonograph {
    fn default() -> Self {
        Self {
            x_pendulums: [
                Pendulum {
                    amplitude: 200.0,
                    frequency: 2.01,
                    phase: PI / 2.0,
                    damping: 0.0005,
                },
                Pendulum {
                    amplitude: 200.0,
                    frequency: 3.0,
                    phase: 0.0,
                    damping: 0.0005,
                },
            ],
            y_pendulums: [
                Pendulum {
                    amplitude: 200.0,
                    frequency: 3.0,
                    phase: PI / 2.0,
                    damping: 0.0005,
                },
                Pendulum {
                    amplitude: 200.0,
                    frequency: 2.0,
                    phase: 0.0,
                    damping: 0.0005,
                },
            ],
            step_size: 0.01,
            iterations: 10000,
            color: 0xFFFF_FFFF,
        }
    }
}

impl Harmonograph {
    /// Evaluate the position of the harmonograph at time `t`.
    #[must_use]
    pub fn evaluate(&self, t: f32) -> Vec2 {
        let x = self.x_pendulums[0].amplitude
            * (self.x_pendulums[0].frequency * t + self.x_pendulums[0].phase).sin()
            * (-self.x_pendulums[0].damping * t).exp()
            + self.x_pendulums[1].amplitude
                * (self.x_pendulums[1].frequency * t + self.x_pendulums[1].phase).sin()
                * (-self.x_pendulums[1].damping * t).exp();

        let y = self.y_pendulums[0].amplitude
            * (self.y_pendulums[0].frequency * t + self.y_pendulums[0].phase).sin()
            * (-self.y_pendulums[0].damping * t).exp()
            + self.y_pendulums[1].amplitude
                * (self.y_pendulums[1].frequency * t + self.y_pendulums[1].phase).sin()
                * (-self.y_pendulums[1].damping * t).exp();

        Vec2::new(x, y)
    }

    /// Renders the harmonograph to the given framebuffer, centered.
    pub fn render(&self, fb: &mut Framebuffer) {
        let center_x = fb.width() as f32 / 2.0;
        let center_y = fb.height() as f32 / 2.0;

        let mut prev_pos = self.evaluate(0.0);
        let mut prev_screen_x = (center_x + prev_pos.x) as i32;
        let mut prev_screen_y = (center_y + prev_pos.y) as i32;

        let mut t = 0.0;
        for _ in 0..self.iterations {
            t += self.step_size;
            let current_pos = self.evaluate(t);

            let screen_x = (center_x + current_pos.x) as i32;
            let screen_y = (center_y + current_pos.y) as i32;

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
        // Bounds checking is essential as lines might stray offscreen
        if x0 >= 0 && x0 < fb.width() as i32 && y0 >= 0 && y0 < fb.height() as i32 {
            // SAFETY: We just did bounds checks
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
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_harmonograph_evaluate_at_zero() {
        let h = Harmonograph {
            x_pendulums: [
                Pendulum {
                    amplitude: 100.0,
                    frequency: 1.0,
                    phase: 0.0,
                    damping: 0.0,
                },
                Pendulum {
                    amplitude: 0.0,
                    frequency: 0.0,
                    phase: 0.0,
                    damping: 0.0,
                },
            ],
            y_pendulums: [
                Pendulum {
                    amplitude: 100.0,
                    frequency: 1.0,
                    phase: PI / 2.0,
                    damping: 0.0,
                },
                Pendulum {
                    amplitude: 0.0,
                    frequency: 0.0,
                    phase: 0.0,
                    damping: 0.0,
                },
            ],
            ..Default::default()
        };

        let pos = h.evaluate(0.0);
        // x = 100 * sin(0) = 0
        assert_eq!(pos.x, 0.0);
        // y = 100 * sin(pi/2) = 100
        assert!((pos.y - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_harmonograph_render_modifies_framebuffer() {
        let mut fb = Framebuffer::new(200, 200).unwrap();
        fb.clear(0xFF00_0000); // Clear to black

        let mut h = Harmonograph::default();
        h.iterations = 100;
        h.color = 0xFFFF_FFFF; // Draw white

        h.render(&mut fb);

        // Verify that *some* pixel has been changed from black to white.
        let has_white_pixel = fb.as_slice().iter().any(|&p| p == 0xFFFF_FFFF);
        assert!(
            has_white_pixel,
            "Harmonograph should have drawn on the framebuffer"
        );
    }

    #[test]
    fn test_draw_line_2d_oob() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFF00_0000);

        // Drawing a line that's completely out of bounds should not panic or modify the visible area
        draw_line_2d(&mut fb, -20, -20, -10, -10, 0xFFFF_FFFF);

        let has_white = fb.as_slice().iter().any(|&p| p == 0xFFFF_FFFF);
        assert!(!has_white, "OOB line should not modify the framebuffer");
    }
}
