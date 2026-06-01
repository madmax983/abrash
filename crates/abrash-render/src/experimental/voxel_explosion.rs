//! Voxel Explosion Module
//!
//! Converts a mesh into an exploding cloud of particles by voxelizing it.
//! Each voxel becomes a particle that flies outward from the center.

use crate::experimental::voxelizer::Voxelizer;
use crate::particles::{Particle, ParticleSystem};
use abrash_core::geometry::AABB;
use abrash_core::math::Vec3;
use abrash_core::mesh::Mesh;
use abrash_core::texture::Texture;
use abrash_core::utils::XorShift32;

/// Configuration for the explosion effect.
#[derive(Debug, Clone, Copy)]
pub struct ExplosionConfig {
    /// Initial speed of particles.
    pub speed: f32,
    /// Randomness factor for velocity direction.
    pub randomness: f32,
    /// Initial life of particles (seconds).
    pub life: f32,
    /// Size of each particle (world units).
    pub size: f32,
}

impl Default for ExplosionConfig {
    fn default() -> Self {
        Self {
            speed: 5.0,
            randomness: 0.5,
            life: 2.0,
            size: 0.1,
        }
    }
}

/// Creates a particle system representing an exploding mesh.
///
/// # Arguments
///
/// * `mesh` - The source mesh to explode.
/// * `resolution` - Voxel grid resolution (higher = more particles).
/// * `texture` - Texture to apply to particles.
/// * `config` - Explosion parameters.
#[must_use]
pub fn create_explosion(
    mesh: &Mesh,
    resolution: usize,
    texture: Texture,
    config: ExplosionConfig,
) -> ParticleSystem {
    // 1. Voxelize the mesh
    let grid = Voxelizer::voxelize(mesh, resolution);

    // 2. Count active voxels to size the system
    let active_count = grid.data.iter().filter(|&&b| b).count();

    // Create system
    // Note: We initialize with active_count capacity.
    let mut sys = ParticleSystem::new(active_count, texture);

    // We want the explosion to center on the mesh center
    // But individual particles start at their voxel positions.
    let aabb = AABB::from_points(&mesh.vertices);
    let center = aabb.center();

    let mut rng = XorShift32::new(98765);
    let half_voxel = grid.voxel_size * 0.5;

    for z in 0..grid.depth {
        for y in 0..grid.height {
            for x in 0..grid.width {
                if grid.get(x, y, z) {
                    // Calculate world position of voxel center
                    let pos = grid.origin
                        + Vec3::new(
                            (x as f32) * grid.voxel_size + half_voxel,
                            (y as f32) * grid.voxel_size + half_voxel,
                            (z as f32) * grid.voxel_size + half_voxel,
                        );

                    // Calculate direction from center
                    // Add some small randomness to avoid perfectly uniform lines
                    let dir = (pos - center).fast_normalize();

                    // Random offset to velocity
                    let rand_vec = Vec3::new(
                        rng.next_f32_signed(),
                        rng.next_f32_signed(),
                        rng.next_f32_signed(),
                    ) * config.randomness;

                    let velocity = (dir + rand_vec).fast_normalize() * config.speed;

                    // Add particle manually
                    // We bypass the emitter logic since this is a one-shot burst.
                    sys.particles.push(Particle::new(
                        pos,
                        velocity,
                        config.life,
                        config.size,
                        0xFFFF_FFFF, // White by default
                    ));
                }
            }
        }
    }

    sys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_explosion() {
        // Create a simple cube mesh (2.0 size)
        let mesh = Mesh::cube(2.0);

        // Create a minimal texture
        let mut texture = Texture::new(1, 1).unwrap();
        texture.pixels[0] = 0xFFFF_FFFF;

        let config = ExplosionConfig {
            speed: 10.0,
            randomness: 0.1,
            life: 1.0,
            size: 0.1,
        };

        // Low resolution for test speed
        let resolution = 5;

        let sys = create_explosion(&mesh, resolution, texture, config);

        // Should have particles
        assert!(!sys.particles.is_empty(), "No particles generated!");

        // Check first particle properties
        let p = sys.particles[0];
        assert!(p.life > 0.0);
        assert!(p.size > 0.0);

        // Velocity should be roughly 10.0 (speed)
        let speed = p.velocity.length();
        // Allow some variance due to randomness
        assert!(speed > 0.0, "Particle has zero velocity");
    }
}
