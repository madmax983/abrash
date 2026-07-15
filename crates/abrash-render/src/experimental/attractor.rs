//! # Strange Attractor Simulator
//!
//! Generates strange attractors (like Clifford Attractors) which are chaotic math systems.

use crate::framebuffer::Framebuffer;

/// Configuration for a Clifford Attractor
#[derive(Debug, Clone, Copy)]
pub struct CliffordAttractor {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
}

impl CliffordAttractor {
    /// Creates a new Clifford Attractor with the given parameters.
    #[must_use]
    pub const fn new(a: f32, b: f32, c: f32, d: f32) -> Self {
        Self { a, b, c, d }
    }

    /// Computes the next point in the sequence.
    #[must_use]
    pub fn step(&self, x: f32, y: f32) -> (f32, f32) {
        let nx = (self.a * y).sin() + self.c * (self.a * x).cos();
        let ny = (self.b * x).sin() + self.d * (self.b * y).cos();
        (nx, ny)
    }

    /// Runs multiple iterations of the attractor, filling the provided points buffer.
    /// Batching reduces hot-loop jump overhead.
    pub fn run_steps(&self, mut x: f32, mut y: f32, steps: usize, out_points: &mut Vec<(f32, f32)>) {
        out_points.reserve_exact(steps);
        for _ in 0..steps {
            let (nx, ny) = self.step(x, y);
            out_points.push((nx, ny));
            x = nx;
            y = ny;
        }
    }
}

/// Renders a set of attractor points into the framebuffer.
pub fn render_attractor(fb: &mut Framebuffer, points: &[(f32, f32)], scale: f32, offset_x: f32, offset_y: f32, color: u32) {
    let w = fb.width() as f32;
    let h = fb.height() as f32;

    let fb_slice = fb.as_mut_slice();
    let width = w as i32;
    let height = h as i32;

    for &(x, y) in points {
        let px = (x * scale + offset_x).floor() as i32;
        let py = (y * scale + offset_y).floor() as i32;

        if px >= 0 && px < width && py >= 0 && py < height {
            // Additive blending
            let idx = (py * width + px) as usize;
            let bg = fb_slice[idx];
            let r = ((bg >> 16) & 0xFF).saturating_add((color >> 16) & 0xFF);
            let g = ((bg >> 8) & 0xFF).saturating_add((color >> 8) & 0xFF);
            let b = (bg & 0xFF).saturating_add(color & 0xFF);
            fb_slice[idx] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_attractor_step() {
        let a = CliffordAttractor::new(1.5, -1.8, 1.6, 0.9);
        let pt = a.step(0.1, 0.1);
        assert_ne!(pt, (0.1, 0.1));
    }

    #[test]
    fn test_attractor_batch() {
        let a = CliffordAttractor::new(1.5, -1.8, 1.6, 0.9);
        let mut pts = Vec::new();
        a.run_steps(0.1, 0.1, 10, &mut pts);
        assert_eq!(pts.len(), 10);
    }

    #[test]
    fn test_render_attractor() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        let pts = vec![(0.0, 0.0), (1.0, 1.0)];
        render_attractor(&mut fb, &pts, 10.0, 50.0, 50.0, 0x00FF0000);
        let center = fb.as_slice()[50 * 100 + 50];
        assert_eq!(center & 0x00FF0000, 0x00FF0000);
    }
}
