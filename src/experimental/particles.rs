//! Simple particle system for rendering points/sprites.

use crate::framebuffer::Framebuffer;
use crate::light::color_to_u32;
use crate::math::{Mat4, Vec3};
use crate::zbuffer::ZBuffer;

#[derive(Debug, Clone, Copy)]
pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub color: Vec3,
    pub life: f32,
    pub max_life: f32,
}

impl Particle {
    pub fn new(position: Vec3, velocity: Vec3, color: Vec3, life: f32) -> Self {
        Self {
            position,
            velocity,
            color,
            life,
            max_life: life,
        }
    }
}

pub struct ParticleSystem {
    particles: Vec<Particle>,
    max_particles: usize,
}

impl ParticleSystem {
    pub fn new(max_particles: usize) -> Self {
        Self {
            particles: Vec::with_capacity(max_particles),
            max_particles,
        }
    }

    /// Spawn a new particle if under the limit
    pub fn emit(&mut self, position: Vec3, velocity: Vec3, color: Vec3, life: f32) {
        if self.particles.len() < self.max_particles {
            self.particles
                .push(Particle::new(position, velocity, color, life));
        }
    }

    /// Update particles (move and age)
    pub fn update(&mut self, dt: f32) {
        // Move particles
        for p in &mut self.particles {
            p.position = p.position + p.velocity * dt;
            p.life -= dt;
        }

        // Remove dead particles
        // Use swap_remove for O(1) removal, order doesn't matter
        let mut i = 0;
        while i < self.particles.len() {
            if self.particles[i].life <= 0.0 {
                self.particles.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// Render particles to framebuffer
    pub fn render(&self, fb: &mut Framebuffer, zb: &mut ZBuffer, view_proj: &Mat4) {
        let width = fb.width() as f32;
        let height = fb.height() as f32;

        for p in &self.particles {
            // Project to clip space
            let (clip_pos, w) = view_proj.transform_point(p.position);

            // Clip check (simple w check for behind camera)
            if w <= 0.0001 {
                continue;
            }

            // Perspective divide
            let inv_w = 1.0 / w;
            let ndc_x = clip_pos.x * inv_w;
            let ndc_y = clip_pos.y * inv_w;
            let depth = clip_pos.z * inv_w;

            // Frustum cull (rough)
            if !(-1.1..=1.1).contains(&ndc_x)
                || !(-1.1..=1.1).contains(&ndc_y)
                || !(0.0..=1.0).contains(&depth)
            {
                continue;
            }

            // Viewport transform
            let screen_x = ((ndc_x + 1.0) * 0.5 * width) as i32;
            let screen_y = ((1.0 - ndc_y) * 0.5 * height) as i32; // Flip Y

            // Fade alpha based on life
            let alpha = (p.life / p.max_life).clamp(0.0, 1.0);
            let faded_color = p.color * alpha;
            let color_u32 = color_to_u32(faded_color);

            // Draw 2x2 block
            for dy in 0..2 {
                for dx in 0..2 {
                    let sx = screen_x + dx;
                    let sy = screen_y + dy;
                    if zb.test_and_set(sx, sy, depth) {
                        fb.set_pixel(sx, sy, color_u32);
                    }
                }
            }
        }
    }

    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_lifecycle() {
        let mut sys = ParticleSystem::new(10);

        sys.emit(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
            1.0,
        );
        assert_eq!(sys.particle_count(), 1);

        // Update 0.5s - should move 0.5 units X
        sys.update(0.5);
        assert_eq!(sys.particle_count(), 1);
        assert!((sys.particles[0].position.x - 0.5).abs() < 0.0001);
        assert!((sys.particles[0].life - 0.5).abs() < 0.0001);

        // Update 0.6s - life < 0, should die
        sys.update(0.6);
        assert_eq!(sys.particle_count(), 0);
    }
}
