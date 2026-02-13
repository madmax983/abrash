//! Particle System Module
//!
//! A simple CPU-based particle system with billboard rendering.
//!
//! # Features
//! * Point-sprite rendering (Billboards)
//! * Simple physics (Gravity, Velocity)
//! * Texture support
//! * Emitter configuration

use crate::framebuffer::Framebuffer;
use crate::math::{Mat4, Vec2, Vec3};
use crate::rasterizer::fill_triangle_textured;
use crate::texture::Texture;
use crate::zbuffer::ZBuffer;

/// A single particle in the system.
#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
    pub color: u32,
}

impl Particle {
    pub fn new(position: Vec3, velocity: Vec3, life: f32, size: f32, color: u32) -> Self {
        Self {
            position,
            velocity,
            life,
            max_life: life,
            size,
            color,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_emission() {
        let texture = Texture::new(2, 2).unwrap();
        let mut sys = ParticleSystem::new(10, texture);
        sys.emission_rate = 1.0;

        // Update 0.5s -> accumulator 0.5 -> 0 particles
        sys.update(0.5);
        assert_eq!(sys.particles.len(), 0);

        // Update 0.6s -> accumulator 1.1 -> 1 particle emitted -> acc 0.1
        sys.update(0.6);
        assert_eq!(sys.particles.len(), 1);
    }

    #[test]
    fn test_particle_life() {
        let texture = Texture::new(2, 2).unwrap();
        let mut sys = ParticleSystem::new(10, texture);
        sys.emission_rate = 0.0; // Manual emission

        // Manually add a particle
        sys.particles.push(Particle::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
            1.0, // Life 1.0
            0.1,
            0xFFFFFFFF
        ));

        // Update 0.5s -> Life 0.5
        sys.update(0.5);
        assert_eq!(sys.particles.len(), 1);
        assert!((sys.particles[0].life - 0.5).abs() < 0.001);

        // Update 0.6s -> Life -0.1 -> Dead
        sys.update(0.6);
        assert_eq!(sys.particles.len(), 0);
    }

    #[test]
    fn test_physics() {
        let texture = Texture::new(2, 2).unwrap();
        let mut sys = ParticleSystem::new(10, texture);
        sys.gravity = Vec3::new(0.0, -10.0, 0.0);

        sys.particles.push(Particle::new(
            Vec3::new(0.0, 10.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
            2.0, // Life 2.0 so it survives 1.0s update
            0.1,
            0xFFFFFFFF
        ));

        // Update 1.0s
        // Vel = 0 + (-10 * 1) = -10
        // Pos = 10 + (-10 * 1) = 0?
        // Note: Simple Euler integration v += a*dt; p += v*dt;
        // In code: v += g*dt; p += v*dt; (Symplectic Euler)
        // v = -10. p = 10 + (-10) = 0.
        sys.update(1.0);

        let p = &sys.particles[0];
        assert!((p.velocity.y - -10.0).abs() < 0.001);
        assert!((p.position.y - 0.0).abs() < 0.001);
    }
}

/// A particle emitter and manager.
pub struct ParticleSystem {
    pub particles: Vec<Particle>,
    pub position: Vec3,
    pub emission_rate: f32, // Particles per second
    pub gravity: Vec3,
    pub texture: Texture,

    // Emitter properties
    pub start_speed: f32,
    pub start_life: f32,
    pub start_size: f32,
    pub spread: f32,

    // Internal state
    emission_accumulator: f32,
    rng_state: u32,
}

impl ParticleSystem {
    /// Creates a new particle system.
    ///
    /// # Arguments
    /// * `max_particles` - Initial capacity.
    /// * `texture` - The texture to use for particles.
    pub fn new(max_particles: usize, texture: Texture) -> Self {
        Self {
            particles: Vec::with_capacity(max_particles),
            position: Vec3::new(0.0, 0.0, 0.0),
            emission_rate: 10.0,
            gravity: Vec3::new(0.0, -9.8, 0.0),
            texture,
            start_speed: 1.0,
            start_life: 1.0,
            start_size: 0.1,
            spread: 0.5,
            emission_accumulator: 0.0,
            rng_state: 12345,
        }
    }

    /// Simple XorShift RNG
    fn rand_float(&mut self) -> f32 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng_state = x;
        (x as f32) / (u32::MAX as f32)
    }

    /// Returns a random float between -1.0 and 1.0
    fn rand_signed(&mut self) -> f32 {
        self.rand_float() * 2.0 - 1.0
    }

    /// Updates the particle system.
    ///
    /// * `dt` - Delta time in seconds.
    pub fn update(&mut self, dt: f32) {
        // Emit new particles
        self.emission_accumulator += dt * self.emission_rate;
        while self.emission_accumulator >= 1.0 {
            self.emit();
            self.emission_accumulator -= 1.0;
        }

        // Update existing particles
        let mut i = 0;
        while i < self.particles.len() {
            let p = &mut self.particles[i];

            p.life -= dt;
            if p.life <= 0.0 {
                // Remove dead particle (swap remove is O(1))
                self.particles.swap_remove(i);
                // Don't increment i, as the swapped element needs to be checked
                continue;
            }

            // Physics
            p.velocity = p.velocity + self.gravity * dt;
            p.position = p.position + p.velocity * dt;

            i += 1;
        }
    }

    fn emit(&mut self) {
        let vel = Vec3::new(
            self.rand_signed() * self.spread,
            1.0 + self.rand_signed() * self.spread, // Generally upwards
            self.rand_signed() * self.spread,
        ).normalize() * self.start_speed;

        let p = Particle::new(
            self.position,
            vel,
            self.start_life,
            self.start_size,
            0xFFFFFFFF,
        );
        self.particles.push(p);
    }

    /// Renders the particles as billboards.
    pub fn render(
        &self,
        fb: &mut Framebuffer,
        zb: &mut ZBuffer,
        view: Mat4,
        proj: Mat4,
    ) {
        // Extract camera Right and Up vectors from View Matrix.
        // The View Matrix transforms World to Camera space.
        // Row 0 is the Right vector (Side)
        // Row 1 is the Up vector
        // (Assuming standard LookAt construction without scaling)
        let right = Vec3::new(view.m[0][0], view.m[1][0], view.m[2][0]);
        let up = Vec3::new(view.m[0][1], view.m[1][1], view.m[2][1]);

        let mvp = proj * view;

        for p in &self.particles {
            let half_size = p.size * 0.5;

            // Billboard corners in World Space
            // v0: Bottom-Left
            let v0_pos = p.position + (right * -half_size) + (up * -half_size);
            // v1: Top-Left
            let v1_pos = p.position + (right * -half_size) + (up * half_size);
            // v2: Top-Right
            let v2_pos = p.position + (right * half_size) + (up * half_size);
            // v3: Bottom-Right
            let v3_pos = p.position + (right * half_size) + (up * -half_size);

            // Transform to Clip Space
            let (c0, w0) = mvp.transform_point(v0_pos);
            let (c1, w1) = mvp.transform_point(v1_pos);
            let (c2, w2) = mvp.transform_point(v2_pos);
            let (c3, w3) = mvp.transform_point(v3_pos);

            // UVs
            let uv0 = Vec2::new(0.0, 1.0); // BL
            let uv1 = Vec2::new(0.0, 0.0); // TL
            let uv2 = Vec2::new(1.0, 0.0); // TR
            let uv3 = Vec2::new(1.0, 1.0); // BR

            // Render 2 Triangles
            // Tri 1: 0-1-2
            fill_triangle_textured(
                fb, zb,
                ((c0, w0), uv0),
                ((c1, w1), uv1),
                ((c2, w2), uv2),
                &self.texture
            );

            // Tri 2: 0-2-3
            fill_triangle_textured(
                fb, zb,
                ((c0, w0), uv0),
                ((c2, w2), uv2),
                ((c3, w3), uv3),
                &self.texture
            );
        }
    }
}
