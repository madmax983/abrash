//! Strange Attractor Generator
//!
//! An experimental module that generates and renders 2D projections of
//! chaotic 3D strange attractors (like the Lorenz or Thomas' cyclically
//! symmetric attractors).

use crate::framebuffer::Framebuffer;
use abrash_core::math::Vec3;

/// Type of strange attractor to generate
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AttractorType {
    /// Lorenz Attractor (butterfly shape)
    Lorenz { sigma: f32, rho: f32, beta: f32 },
    /// Thomas' cyclically symmetric attractor (fluid, interconnected loops)
    Thomas { b_param: f32 },
    /// Aizawa Attractor (spherical, intertwining)
    Aizawa {
        a: f32,
        b: f32,
        c: f32,
        d: f32,
        e: f32,
        f: f32,
    },
}

impl Default for AttractorType {
    fn default() -> Self {
        AttractorType::Lorenz {
            sigma: 10.0,
            rho: 28.0,
            beta: 8.0 / 3.0,
        }
    }
}

/// Configuration for the strange attractor simulation
#[derive(Debug, Clone)]
pub struct AttractorConfig {
    /// Type of the attractor
    pub attractor_type: AttractorType,
    /// Number of points to simulate
    pub iterations: usize,
    /// Time step per iteration (dt)
    pub dt: f32,
    /// Scale of the rendering (pixels per unit)
    pub scale: f32,
    /// Initial starting position
    pub start_pos: Vec3,
    /// Color of the points
    pub color: u32,
    /// Additive blending instead of overwriting
    pub additive: bool,
}

impl Default for AttractorConfig {
    fn default() -> Self {
        Self {
            attractor_type: AttractorType::default(),
            iterations: 100_000,
            dt: 0.01,
            scale: 10.0,
            start_pos: Vec3::new(0.1, 0.0, 0.0),
            color: 0xFF_00FF00, // Matrix green
            additive: true,
        }
    }
}

/// Renders a strange attractor into the given framebuffer.
pub fn render_strange_attractor(fb: &mut Framebuffer, config: &AttractorConfig) {
    let width = fb.width() as f32;
    let height = fb.height() as f32;
    let center_x = width / 2.0;
    let center_y = height / 2.0;

    let mut pos = config.start_pos;

    // Unpack color
    let alpha = (config.color >> 24) & 0xFF;
    let r = (config.color >> 16) & 0xFF;
    let g = (config.color >> 8) & 0xFF;
    let b = config.color & 0xFF;

    for _ in 0..config.iterations {
        // Calculate derivatives based on the attractor equations
        let (dx, dy, dz) = match config.attractor_type {
            AttractorType::Lorenz { sigma, rho, beta } => {
                let dx = sigma * (pos.y - pos.x);
                let dy = pos.x * (rho - pos.z) - pos.y;
                let dz = pos.x * pos.y - beta * pos.z;
                (dx, dy, dz)
            }
            AttractorType::Thomas { b_param } => {
                let dx = pos.y.sin() - b_param * pos.x;
                let dy = pos.z.sin() - b_param * pos.y;
                let dz = pos.x.sin() - b_param * pos.z;
                (dx, dy, dz)
            }
            AttractorType::Aizawa { a, b, c, d, e, f } => {
                let dx = (pos.z - b) * pos.x - d * pos.y;
                let dy = d * pos.x + (pos.z - b) * pos.y;
                let dz = c + a * pos.z - (pos.z * pos.z * pos.z) / 3.0
                    - (pos.x * pos.x + pos.y * pos.y) * (1.0 + e * pos.z)
                    + f * pos.z * (pos.x * pos.x * pos.x);
                (dx, dy, dz)
            }
        };

        // Euler integration step
        pos.x += dx * config.dt;
        pos.y += dy * config.dt;
        pos.z += dz * config.dt;

        // Project to 2D screen space (Orthographic projection for simplicity)
        // Depending on the attractor, we might want to map different axes.
        // For Lorenz, looking at X/Z or X/Y is common.
        let (screen_x, screen_y) = match config.attractor_type {
            AttractorType::Lorenz { .. } => {
                // Lorenz is tall on Z axis, so we map X to X, Z to Y (flipped)
                let sx = center_x + pos.x * config.scale;
                let sy = center_y - (pos.z - 25.0) * config.scale; // Offset Z to center
                (sx, sy)
            }
            AttractorType::Thomas { .. } => {
                let sx = center_x + pos.x * config.scale;
                let sy = center_y - pos.y * config.scale;
                (sx, sy)
            }
            AttractorType::Aizawa { .. } => {
                let sx = center_x + pos.x * config.scale;
                let sy = center_y - pos.z * config.scale;
                (sx, sy)
            }
        };

        let ix = screen_x as i32;
        let iy = screen_y as i32;

        if ix >= 0 && ix < fb.width() as i32 && iy >= 0 && iy < fb.height() as i32 {
            if config.additive {
                // Simple additive blending
                let current = fb.get_pixel(ix, iy).unwrap_or(0);

                let cr = ((current >> 16) & 0xFF).saturating_add(r);
                let cg = ((current >> 8) & 0xFF).saturating_add(g);
                let cb = (current & 0xFF).saturating_add(b);

                let r_clamped = cr.min(255);
                let g_clamped = cg.min(255);
                let b_clamped = cb.min(255);

                let blended = (alpha << 24) | (r_clamped << 16) | (g_clamped << 8) | b_clamped;

                // SAFETY: bounds checked above
                unsafe {
                    fb.set_pixel_unchecked(ix as usize, iy as usize, blended);
                }
            } else {
                // SAFETY: bounds checked above
                unsafe {
                    fb.set_pixel_unchecked(ix as usize, iy as usize, config.color);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_lorenz() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF_000000);

        let config = AttractorConfig {
            attractor_type: AttractorType::Lorenz {
                sigma: 10.0,
                rho: 28.0,
                beta: 8.0 / 3.0,
            },
            iterations: 1000,
            dt: 0.01,
            scale: 2.0,
            start_pos: Vec3::new(0.1, 0.1, 0.1),
            color: 0xFF_FFFFFF,
            additive: false,
        };

        render_strange_attractor(&mut fb, &config);

        let has_pixels = fb.as_slice().iter().any(|&p| p != 0xFF_000000);
        assert!(has_pixels, "Lorenz attractor should render something");
    }

    #[test]
    fn test_render_thomas() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF_000000);

        let config = AttractorConfig {
            attractor_type: AttractorType::Thomas { b_param: 0.208186 },
            iterations: 1000,
            dt: 0.1,
            scale: 10.0,
            start_pos: Vec3::new(0.1, 0.1, 0.1),
            color: 0xFF_FFFFFF,
            additive: true,
        };

        render_strange_attractor(&mut fb, &config);

        let has_pixels = fb.as_slice().iter().any(|&p| p != 0xFF_000000);
        assert!(has_pixels, "Thomas attractor should render something");
    }
}
