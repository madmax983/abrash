//! 3D Particle System
//!
//! A simple system for managing and rendering 3D particles.

use crate::framebuffer::Framebuffer;
use crate::light::color_to_u32;
use crate::math::{Mat4, Vec3};
use crate::zbuffer::ZBuffer;

#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub color: Vec3,
    pub life: f32,     // Remaining life in seconds
    pub max_life: f32, // Total life for fading
    pub size: f32,
}

impl Particle {
    pub fn new(position: Vec3, velocity: Vec3, color: Vec3, life: f32) -> Self {
        Self {
            position,
            velocity,
            color,
            life,
            max_life: life,
            size: 1.0,
        }
    }
}

pub struct ParticleSystem {
    pub particles: Vec<Particle>,
    pub gravity: Vec3,
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self {
            particles: Vec::new(),
            gravity: Vec3::new(0.0, -9.8, 0.0),
        }
    }

    pub fn add(&mut self, particle: Particle) {
        self.particles.push(particle);
    }

    pub fn update(&mut self, dt: f32) {
        // Update particles
        for p in &mut self.particles {
            p.velocity = p.velocity + self.gravity * dt;
            p.position = p.position + p.velocity * dt;
            p.life -= dt;
        }

        // Remove dead particles
        self.particles.retain(|p| p.life > 0.0);
    }

    pub fn render(&self, fb: &mut Framebuffer, zb: &mut ZBuffer, mvp: &Mat4) {
        let width = fb.width();
        let height = fb.height();

        for p in &self.particles {
            // Transform position
            let (clip_pos, w) = mvp.transform_point(p.position);

            // Clip check (simple w check for now)
            if w <= 0.0001 {
                continue;
            }

            // Project
            let sp = crate::math::project_to_screen(clip_pos, w, width, height);

            // Frustum culling (simple check on z/depth)
            // project_to_screen returns depth in sp.z
            // We want 0.0 <= depth <= 1.0
            if !(0.0..=1.0).contains(&sp.z) {
                continue;
            }

            // Screen bounds check
            // project_to_screen output can be outside screen
            if sp.x >= 0 && sp.x < width as i32 && sp.y >= 0 && sp.y < height as i32 {
                // Simple point rendering
                if zb.test_and_set(sp.x, sp.y, sp.z) {
                    // Fade alpha based on life
                    let life_ratio = (p.life / p.max_life).clamp(0.0, 1.0);
                    // Fade to black as it dies
                    let faded_color = p.color * life_ratio;
                    fb.set_pixel(sp.x, sp.y, color_to_u32(faded_color));
                }
            }
        }
    }
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_update() {
        let mut sys = ParticleSystem::new();
        // Disable gravity for simple test
        sys.gravity = Vec3::default();

        let p = Particle::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
            1.0,
        );
        sys.add(p);

        sys.update(0.5);

        assert_eq!(sys.particles.len(), 1);
        assert!((sys.particles[0].position.x - 0.5).abs() < 0.001);
        assert!((sys.particles[0].life - 0.5).abs() < 0.001);

        sys.update(0.6); // Total 1.1s, should die
        assert_eq!(sys.particles.len(), 0);
    }
}
