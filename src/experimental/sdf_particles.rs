//! SDF Particle Interaction Module.
//!
//! This module enables interaction between the particle system and Signed Distance Fields (SDF).
//! It allows particles to collide with SDF geometry and simulate fluid-like behavior using SPH.

use super::sdf::SdfScene;
use crate::math::Vec3;
use crate::particles::ParticleSystem;

/// Resolves collisions between particles and an SDF scene.
///
/// Particles that penetrate the SDF surface are pushed out and their velocity is reflected.
///
/// # Arguments
/// * `system` - The particle system to update.
/// * `scene` - The SDF scene to collide with.
/// * `restitution` - Bounciness factor (0.0 = no bounce, 1.0 = perfect elastic).
/// * `friction` - Friction factor (0.0 = no friction, 1.0 = sticky).
pub fn resolve_sdf_collisions(
    system: &mut ParticleSystem,
    scene: &SdfScene,
    restitution: f32,
    friction: f32,
) {
    for p in &mut system.particles {
        let (dist, _) = scene.map(p.position);

        // Check if inside (dist < particle_radius)
        // Ideally we use p.size/2 as radius.
        let radius = p.size * 0.5;

        if dist < radius {
            // Collision!
            let normal = scene.normal(p.position);
            let penetration = radius - dist;

            // Push out
            p.position = p.position + normal * penetration;

            // Reflect velocity
            let v = p.velocity;
            let v_n = v.dot(normal);

            if v_n < 0.0 {
                // Bounce
                let j = -(1.0 + restitution) * v_n;
                p.velocity = v + normal * j;

                // Friction (tangential velocity)
                let v_t = v - normal * v_n;
                p.velocity = p.velocity - v_t * friction;
            }
        }
    }
}

/// Applies Smoothed Particle Hydrodynamics (SPH) forces to the particle system.
///
/// Simulates pressure and viscosity forces to make particles behave like a fluid.
/// NOTE: This is a simplified implementation (O(N^2)). Use with < 1000 particles.
///
/// # Arguments
/// * `system` - The particle system.
/// * `smoothing_radius` - Interaction radius (h).
/// * `target_density` - Rest density of the fluid (rho0).
/// * `pressure_multiplier` - Stiffness of the pressure force (k).
/// * `viscosity` - Viscosity coefficient.
/// * `dt` - Time step (seconds).
pub fn apply_sph_forces(
    system: &mut ParticleSystem,
    smoothing_radius: f32,
    target_density: f32,
    pressure_multiplier: f32,
    viscosity: f32,
    dt: f32,
) {
    let n = system.particles.len();
    if n == 0 {
        return;
    }

    // 1. Calculate Density for each particle
    // density = sum(mass * W(r, h))
    // We assume mass = 1.0 for now.
    let mut densities = vec![0.0; n];
    let h2 = smoothing_radius * smoothing_radius;
    // Poly6 Kernel constant: 315 / (64 * pi * h^9)
    // Simplified: Just use (h^2 - r^2)^3

    for i in 0..n {
        let p_i = system.particles[i].position;
        let mut density = 0.0;

        for j in 0..n {
            let p_j = system.particles[j].position;
            // Vec3 doesn't have length_squared, use dot product
            let diff = p_i - p_j;
            let r2 = diff.dot(diff);

            if r2 < h2 {
                let diff_val = h2 - r2;
                density += diff_val * diff_val * diff_val;
            }
        }
        // Avoid division by zero
        densities[i] = density.max(0.0001);
    }

    // 2. Calculate Pressure Forces
    // F_pressure = -sum(m * (pi + pj) / (2 * rho_j) * grad(W))
    // Pressure P = k * (rho - rho0)

    let pressures: Vec<f32> = densities
        .iter()
        .map(|&rho| pressure_multiplier * (rho - target_density))
        .collect();

    // Spiky Kernel Gradient constant: -45 / (pi * h^6)
    // Simplified: (h - r)^2 * normalize(r)

    // Store forces to apply later
    let mut forces = vec![Vec3::default(); n];

    for i in 0..n {
        let p_i = system.particles[i].position;
        let mut force_pressure = Vec3::default();
        let mut force_viscosity = Vec3::default();

        for j in 0..n {
            if i == j {
                continue;
            }
            let p_j = system.particles[j].position;
            let r_vec = p_i - p_j;
            let r = r_vec.length();

            if r < smoothing_radius && r > 0.0001 {
                // Pressure Force
                // Push apart if pressure is positive
                let direction = r_vec.normalize();

                // Symmetric pressure term
                let p_term = (pressures[i] + pressures[j]) / (2.0 * densities[j]);

                // Kernel Gradient magnitude (Spiky): (h - r)^2
                let grad_mag = (smoothing_radius - r).powi(2);

                force_pressure = force_pressure + direction * (p_term * grad_mag);

                // Viscosity Force
                // F_visc = mu * sum((v_j - v_i) / rho_j * laplacian(W))
                // Laplacian (Viscosity Kernel): (h - r)
                let v_rel = system.particles[j].velocity - system.particles[i].velocity;
                force_viscosity = force_viscosity + v_rel * ((smoothing_radius - r) / densities[j]);
            }
        }

        forces[i] = force_pressure + force_viscosity * viscosity;
    }

    // Apply forces
    for i in 0..n {
        // F = ma => a = F/m (m=1)
        let accel = forces[i];
        system.particles[i].velocity = system.particles[i].velocity + accel * dt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experimental::sdf::{SdfObject, SdfPrimitive};
    use crate::math::Vec3;
    use crate::particles::ParticleSystem;
    use crate::texture::Texture;

    #[test]
    fn test_collision() {
        // Setup simple particle system
        let texture = Texture::new(1, 1).unwrap();
        let mut sys = ParticleSystem::new(1, texture);
        sys.emission_rate = 0.0;

        // Add particle falling towards origin
        sys.particles.push(crate::particles::Particle::new(
            Vec3::new(0.0, 2.0, 0.0),   // Above origin
            Vec3::new(0.0, -10.0, 0.0), // Moving down
            1.0,
            0.1,
            0xFFFFFFFF,
        ));

        // Sphere at origin, radius 1.0
        let mut scene = SdfScene::new();
        scene.add(SdfObject {
            primitive: SdfPrimitive::Sphere {
                radius: 1.0,
                center: Vec3::new(0.0, 0.0, 0.0),
            },
            color: 0xFFFFFFFF,
        });

        // Move particle to inside sphere manually to test resolution
        sys.particles[0].position = Vec3::new(0.0, 0.9, 0.0); // Inside radius 1.0

        resolve_sdf_collisions(&mut sys, &scene, 0.5, 0.0);

        // Should be pushed out to radius + particle_radius (0.05) -> 1.05
        let p = sys.particles[0];
        assert!(p.position.y >= 1.05 - 0.001);

        // Velocity should be flipped (positive Y)
        assert!(p.velocity.y > 0.0);
    }

    #[test]
    fn test_sph_repulsion() {
        let texture = Texture::new(1, 1).unwrap();
        let mut sys = ParticleSystem::new(2, texture);
        sys.emission_rate = 0.0;

        // Two particles close to each other
        sys.particles.push(crate::particles::Particle::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
            1.0,
            1.0,
            0xFFFFFFFF,
        ));
        sys.particles.push(crate::particles::Particle::new(
            Vec3::new(0.1, 0.0, 0.0), // Very close
            Vec3::new(0.0, 0.0, 0.0),
            1.0,
            1.0,
            0xFFFFFFFF,
        ));

        // Apply SPH
        apply_sph_forces(&mut sys, 1.0, 0.0, 100.0, 0.0, 0.1);

        // Particles should move apart
        let p0 = sys.particles[0];
        let p1 = sys.particles[1];

        // p0 should move left (negative x), p1 right (positive x)
        assert!(p0.velocity.x < 0.0);
        assert!(p1.velocity.x > 0.0);
    }
}
