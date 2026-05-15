#![cfg(feature = "nova")]

use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec3;

/// A trait defining a chaotic dynamical system or strange attractor.
pub trait Attractor {
    /// Advances the simulation by `dt` and returns the new position.
    fn step(&mut self, dt: f32) -> Vec3;

    /// Returns the current position.
    fn position(&self) -> Vec3;
}

/// A simple Lorenz attractor simulator.
pub struct LorenzAttractor {
    pub position: Vec3,
    pub sigma: f32,
    pub rho: f32,
    pub beta: f32,
}

impl LorenzAttractor {
    #[must_use]
    pub const fn new(start_pos: Vec3) -> Self {
        Self {
            position: start_pos,
            sigma: 10.0,
            rho: 28.0,
            beta: 8.0 / 3.0,
        }
    }
}

impl Attractor for LorenzAttractor {
    fn step(&mut self, dt: f32) -> Vec3 {
        let x = self.position.x;
        let y = self.position.y;
        let z = self.position.z;

        let dx = self.sigma * (y - x);
        let dy = x * (self.rho - z) - y;
        let dz = x * y - self.beta * z;

        self.position.x += dx * dt;
        self.position.y += dy * dt;
        self.position.z += dz * dt;

        self.position
    }

    fn position(&self) -> Vec3 {
        self.position
    }
}

/// A simple Rössler attractor simulator.
pub struct RoesslerAttractor {
    pub position: Vec3,
    pub a: f32,
    pub b: f32,
    pub c: f32,
}

impl RoesslerAttractor {
    #[must_use]
    pub const fn new(start_pos: Vec3) -> Self {
        Self {
            position: start_pos,
            a: 0.2,
            b: 0.2,
            c: 5.7,
        }
    }
}

impl Attractor for RoesslerAttractor {
    fn step(&mut self, dt: f32) -> Vec3 {
        let x = self.position.x;
        let y = self.position.y;
        let z = self.position.z;

        let dx = -y - z;
        let dy = x + self.a * y;
        let dz = self.b + z * (x - self.c);

        self.position.x += dx * dt;
        self.position.y += dy * dt;
        self.position.z += dz * dt;

        self.position
    }

    fn position(&self) -> Vec3 {
        self.position
    }
}

/// A simple Aizawa attractor simulator.
pub struct AizawaAttractor {
    pub position: Vec3,
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl AizawaAttractor {
    #[must_use]
    pub const fn new(start_pos: Vec3) -> Self {
        Self {
            position: start_pos,
            a: 0.95,
            b: 0.7,
            c: 0.6,
            d: 3.5,
            e: 0.25,
            f: 0.1,
        }
    }
}

impl Attractor for AizawaAttractor {
    fn step(&mut self, dt: f32) -> Vec3 {
        let x = self.position.x;
        let y = self.position.y;
        let z = self.position.z;

        let dx = (z - self.b) * x - self.d * y;
        let dy = self.d * x + (z - self.b) * y;
        let dz = self.c + self.a * z - z * z * z / 3.0 - (x * x + y * y) * (1.0 + self.e * z)
            + self.f * z * x * x * x;

        self.position.x += dx * dt;
        self.position.y += dy * dt;
        self.position.z += dz * dt;

        self.position
    }

    fn position(&self) -> Vec3 {
        self.position
    }
}

pub fn render_attractor<T: Attractor>(
    fb: &mut Framebuffer,
    attractor: &mut T,
    iterations: usize,
    dt: f32,
    scale: f32,
    pitch: f32,
    yaw: f32,
) {
    let hw = fb.width() as f32 / 2.0;
    let hh = fb.height() as f32 / 2.0;

    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    let (sin_pitch, cos_pitch) = pitch.sin_cos();

    for _ in 0..iterations {
        let p = attractor.step(dt);

        // Apply simple 3D rotations for visualization
        // Yaw (Y-axis rotation)
        let x_yaw = p.x * cos_yaw - p.z * sin_yaw;
        let z_yaw = p.x * sin_yaw + p.z * cos_yaw;

        // Pitch (X-axis rotation)
        let y_pitch = p.y * cos_pitch - z_yaw * sin_pitch;
        let z_pitch = p.y * sin_pitch + z_yaw * cos_pitch;

        // Color mapping based on Z-depth (from near to far)
        // Normalize roughly assuming typical chaotic bounds
        let z_norm = ((z_pitch + 20.0) / 40.0).clamp(0.0, 1.0);
        let r = (255.0 * z_norm) as u32;
        let g = (255.0 * (1.0 - z_norm)) as u32;
        let b = 150_u32;

        let color = 0xFF_00_00_00 | (r << 16) | (g << 8) | b;

        // Simple orthographic projection mapping x/y to screen
        let sx = (x_yaw * scale + hw) as i32;
        let sy = (y_pitch * scale + hh) as i32;

        fb.set_pixel(sx, sy, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lorenz_attractor_steps() {
        let mut lorenz = LorenzAttractor::new(Vec3::new(1.0, 2.0, 3.0));
        let next_pos = lorenz.step(0.01);

        assert_ne!(next_pos.x, 1.0);
        assert_ne!(next_pos.y, 2.0);
        assert_ne!(next_pos.z, 3.0);
    }

    #[test]
    fn test_render_attractor_draws_pixels() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF_00_00_00);

        let mut lorenz = LorenzAttractor::new(Vec3::new(0.1, 0.0, 0.0));
        render_attractor(&mut fb, &mut lorenz, 100, 0.01, 1.0, 0.0, 0.0);

        let mut has_color = false;
        for &pixel in fb.as_slice() {
            if pixel != 0xFF_00_00_00 {
                has_color = true;
                break;
            }
        }

        assert!(has_color, "Attractor should draw at least one pixel.");
    }
}
