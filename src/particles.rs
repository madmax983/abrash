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
use crate::rasterizer::fill_quad_textured;
use crate::texture::Texture;
use crate::utils::XorShift32;
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
    #[must_use]
    pub const fn new(position: Vec3, velocity: Vec3, life: f32, size: f32, color: u32) -> Self {
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
            0xFFFFFFFF,
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
            0xFFFFFFFF,
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
    rng: XorShift32,
}

impl ParticleSystem {
    /// Creates a new particle system.
    ///
    /// # Arguments
    /// * `max_particles` - Initial capacity.
    /// * `texture` - The texture to use for particles.
    #[must_use]
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
            rng: XorShift32::new(12345),
        }
    }

    /// Updates the particle system.
    ///
    /// * `dt` - Delta time in seconds.
    pub fn update(&mut self, dt: f32) {
        // Emit new particles
        self.emission_accumulator += dt * self.emission_rate;
        #[allow(clippy::while_float)]
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
            self.rng.next_f32_signed() * self.spread,
            1.0 + self.rng.next_f32_signed() * self.spread, // Generally upwards
            self.rng.next_f32_signed() * self.spread,
        )
        .normalize()
            * self.start_speed;

        let p = Particle::new(
            self.position,
            vel,
            self.start_life,
            self.start_size,
            0xFFFF_FFFF,
        );
        self.particles.push(p);
    }

    /// Renders the particles as billboards.
    pub fn render(&self, fb: &mut Framebuffer, zb: &mut ZBuffer, view: Mat4, proj: Mat4) {
        // Extract camera Right and Up vectors from View Matrix.
        // The View Matrix transforms World to Camera space.
        // Row 0 is the Right vector (Side)
        // Row 1 is the Up vector
        // (Assuming standard LookAt construction without scaling)
        let right = Vec3::new(view.m[0][0], view.m[1][0], view.m[2][0]);
        let up = Vec3::new(view.m[0][1], view.m[1][1], view.m[2][1]);

        let mvp = proj * view;

        // Optimization: Pre-transform camera basis vectors to Clip Space.
        // This allows us to calculate billboard corners directly in Clip Space,
        // reducing per-particle matrix multiplications from 4 to 1.
        //
        // NOTE: Since these are direction vectors, w=0. We perform manual transform
        // because Mat4::transform_point assumes w=1.
        let transform_vector = |v: Vec3, m: &Mat4| -> (Vec3, f32) {
            let x = m.m[0][0] * v.x + m.m[1][0] * v.y + m.m[2][0] * v.z;
            let y = m.m[0][1] * v.x + m.m[1][1] * v.y + m.m[2][1] * v.z;
            let z = m.m[0][2] * v.x + m.m[1][2] * v.y + m.m[2][2] * v.z;
            let w = m.m[0][3] * v.x + m.m[1][3] * v.y + m.m[2][3] * v.z;
            (Vec3::new(x, y, z), w)
        };

        let (right_clip, right_w) = transform_vector(right, &mvp);
        let (up_clip, up_w) = transform_vector(up, &mvp);

        // UVs are constant for all particles
        let uv0 = Vec2::new(0.0, 1.0); // BL
        let uv1 = Vec2::new(0.0, 0.0); // TL
        let uv2 = Vec2::new(1.0, 0.0); // TR
        let uv3 = Vec2::new(1.0, 1.0); // BR

        for p in &self.particles {
            let half_size = p.size * 0.5;

            // Transform center to Clip Space
            let (center_clip, center_w) = mvp.transform_point(p.position);

            // Scale offsets
            let r_vec = right_clip * half_size;
            let r_w = right_w * half_size;
            let u_vec = up_clip * half_size;
            let u_w = up_w * half_size;

            // Calculate corners in Clip Space
            // v0: Bottom-Left (center - right - up)
            let c0 = center_clip - r_vec - u_vec;
            let w0 = center_w - r_w - u_w;

            // v1: Top-Left (center - right + up)
            let c1 = center_clip - r_vec + u_vec;
            let w1 = center_w - r_w + u_w;

            // v2: Top-Right (center + right + up)
            let c2 = center_clip + r_vec + u_vec;
            let w2 = center_w + r_w + u_w;

            // v3: Bottom-Right (center + right - up)
            let c3 = center_clip + r_vec - u_vec;
            let w3 = center_w + r_w - u_w;

            // Render Quad
            fill_quad_textured(
                fb,
                zb,
                ((c0, w0), uv0),
                ((c1, w1), uv1),
                ((c2, w2), uv2),
                ((c3, w3), uv3),
                &self.texture,
            );
        }
    }
}
