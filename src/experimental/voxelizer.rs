use crate::math::{Vec2, Vec3};
use crate::mesh::Mesh;
use crate::particles::{Particle, ParticleSystem};
use crate::texture::Texture;
use crate::utils::XorShift32;

/// Converts a Mesh into a ParticleSystem by sampling points on the surface.
///
/// # Arguments
///
/// * `mesh` - The mesh to voxelize.
/// * `count` - The target number of particles to generate.
/// * `texture` - The texture to sample colors from.
pub fn voxelize_mesh(mesh: &Mesh, count: usize, texture: Texture) -> ParticleSystem {
    let mut system = ParticleSystem::new(count, texture);
    // Important: Disable auto-emission, we want a static point cloud that we can explode manually.
    system.emission_rate = 0.0;

    if mesh.indices.is_empty() {
        return system;
    }

    let mut rng = XorShift32::new(12345);

    // 1. Calculate total area to distribute particles uniformly
    let mut total_area = 0.0;
    let mut triangle_areas = Vec::with_capacity(mesh.indices.len());

    for &[i0, i1, i2] in &mesh.indices {
        // Safe access check
        if i0 >= mesh.vertices.len() || i1 >= mesh.vertices.len() || i2 >= mesh.vertices.len() {
            triangle_areas.push(0.0);
            continue;
        }

        let v0 = mesh.vertices[i0];
        let v1 = mesh.vertices[i1];
        let v2 = mesh.vertices[i2];
        let area = (v1 - v0).cross(v2 - v0).length() * 0.5;
        total_area += area;
        triangle_areas.push(area);
    }

    if total_area <= 0.0 {
        return system;
    }

    // 2. Spawn particles
    let mut created_count = 0;

    for (tri_idx, &[i0, i1, i2]) in mesh.indices.iter().enumerate() {
        if created_count >= count {
            break;
        }

        if i0 >= mesh.vertices.len() || i1 >= mesh.vertices.len() || i2 >= mesh.vertices.len() {
            continue;
        }

        let area = triangle_areas[tri_idx];

        // Calculate number of particles for this triangle
        // Use a probabilistic approach?
        // Or deterministic: (area / total) * count.
        // We use ceil to ensure we have enough, and break when full.
        let target_count_f = (area / total_area) * count as f32;
        // Use ceil but also accumulate fractional parts?
        // Simple ceil is fine if we clamp total.
        let num_particles = target_count_f.ceil() as usize;

        let v0 = mesh.vertices[i0];
        let v1 = mesh.vertices[i1];
        let v2 = mesh.vertices[i2];

        // UVs (Barycentric interpolation requires UVs)
        // If UVs are missing, default to 0,0
        let uv0 = if i0 < mesh.uvs.len() {
            mesh.uvs[i0]
        } else {
            Vec2::default()
        };
        let uv1 = if i1 < mesh.uvs.len() {
            mesh.uvs[i1]
        } else {
            Vec2::default()
        };
        let uv2 = if i2 < mesh.uvs.len() {
            mesh.uvs[i2]
        } else {
            Vec2::default()
        };

        for _ in 0..num_particles {
            if created_count >= count {
                break;
            }

            // Random Barycentric Coordinates for Uniform Distribution
            let r1 = rng.next_f32();
            let r2 = rng.next_f32();
            let sqrt_r1 = r1.sqrt();
            let u = 1.0 - sqrt_r1;
            let v = sqrt_r1 * (1.0 - r2);
            let w = sqrt_r1 * r2;

            let pos = v0 * u + v1 * v + v2 * w;

            let uv = uv0 * u + uv1 * v + uv2 * w;

            // Sample color from texture
            // system.texture is moved into struct, so we access it via system.texture
            let color = system.texture.get_pixel_bilinear(uv.x, uv.y);

            // Create particle
            // Initialize velocity to 0. Explosion logic will be external.
            let p = Particle {
                position: pos,
                velocity: Vec3::new(0.0, 0.0, 0.0),
                life: 10000.0, // Very long life
                max_life: 10000.0,
                size: 0.05,
                color,
            };

            system.particles.push(p);
            created_count += 1;
        }
    }

    system
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::Mesh;
    use crate::texture::Texture;

    #[test]
    fn test_voxelize_cube() {
        let mesh = Mesh::cube(2.0);
        let texture = Texture::new(2, 2).unwrap();
        let particles = voxelize_mesh(&mesh, 100, texture);

        assert_eq!(particles.particles.len(), 100);
        assert_eq!(particles.emission_rate, 0.0);

        // Verify positions are within cube bounds [-1, 1]
        for p in &particles.particles {
            assert!(p.position.x >= -1.01 && p.position.x <= 1.01);
            assert!(p.position.y >= -1.01 && p.position.y <= 1.01);
            assert!(p.position.z >= -1.01 && p.position.z <= 1.01);
        }
    }
}
