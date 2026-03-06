//! Boids Simulation (Flocking Algorithm)
//!
//! An experimental implementation of Craig Reynolds' Boids algorithm
//! for simulating the flocking behavior of birds, fish, or other entities.

use crate::math::Vec3;
use crate::utils::XorShift32;

#[derive(Clone, Copy, Debug)]
pub struct Boid {
    pub position: Vec3,
    pub velocity: Vec3,
    pub acceleration: Vec3,
}

impl Boid {
    #[must_use]
    pub fn new(position: Vec3, velocity: Vec3) -> Self {
        Self {
            position,
            velocity,
            acceleration: Vec3::default(),
        }
    }
}

pub struct BoidSystem {
    pub boids: Vec<Boid>,
    pub perception_radius: f32,
    pub max_speed: f32,
    pub max_force: f32,
    pub separation_weight: f32,
    pub alignment_weight: f32,
    pub cohesion_weight: f32,
    pub bounds_size: f32,
}

impl BoidSystem {
    #[must_use]
    pub fn new(count: usize, spawn_radius: f32) -> Self {
        let mut rng = XorShift32::new(12345);
        let mut boids = Vec::with_capacity(count);

        for _ in 0..count {
            let pos = Vec3::new(
                rng.next_f32_signed() * spawn_radius,
                rng.next_f32_signed() * spawn_radius,
                rng.next_f32_signed() * spawn_radius,
            );
            let vel = Vec3::new(
                rng.next_f32_signed(),
                rng.next_f32_signed(),
                rng.next_f32_signed(),
            )
            .normalize();

            boids.push(Boid::new(pos, vel));
        }

        Self {
            boids,
            perception_radius: 5.0,
            max_speed: 4.0,
            max_force: 0.1,
            separation_weight: 1.5,
            alignment_weight: 1.0,
            cohesion_weight: 1.0,
            bounds_size: 20.0,
        }
    }

    pub fn update(&mut self, dt: f32) {
        let boid_count = self.boids.len();

        for i in 0..boid_count {
            let mut separation = Vec3::default();
            let mut alignment = Vec3::default();
            let mut cohesion = Vec3::default();
            let mut total_neighbors = 0;

            let boid_i = self.boids[i];

            for j in 0..boid_count {
                if i == j {
                    continue;
                }

                let boid_j = self.boids[j];
                let d = boid_i.position - boid_j.position;
                let dist_sq = d.dot(d);

                if dist_sq > 0.0 && dist_sq < self.perception_radius * self.perception_radius {
                    let dist = dist_sq.sqrt();

                    // Separation
                    separation = separation + (d / dist) / dist;

                    // Alignment
                    alignment = alignment + boid_j.velocity;

                    // Cohesion
                    cohesion = cohesion + boid_j.position;

                    total_neighbors += 1;
                }
            }

            if total_neighbors > 0 {
                let f_total_neighbors = total_neighbors as f32;

                separation = separation / f_total_neighbors;
                if separation.length_sq() > 0.0 {
                    separation = separation.normalize() * self.max_speed;
                    separation = separation - boid_i.velocity;
                    if separation.length_sq() > self.max_force * self.max_force {
                        separation = separation.normalize() * self.max_force;
                    }
                }

                alignment = alignment / f_total_neighbors;
                if alignment.length_sq() > 0.0 {
                    alignment = alignment.normalize() * self.max_speed;
                    alignment = alignment - boid_i.velocity;
                    if alignment.length_sq() > self.max_force * self.max_force {
                        alignment = alignment.normalize() * self.max_force;
                    }
                }

                cohesion = cohesion / f_total_neighbors;
                cohesion = cohesion - boid_i.position;
                if cohesion.length_sq() > 0.0 {
                    cohesion = cohesion.normalize() * self.max_speed;
                    cohesion = cohesion - boid_i.velocity;
                    if cohesion.length_sq() > self.max_force * self.max_force {
                        cohesion = cohesion.normalize() * self.max_force;
                    }
                }
            }

            self.boids[i].acceleration = self.boids[i].acceleration
                + separation * self.separation_weight
                + alignment * self.alignment_weight
                + cohesion * self.cohesion_weight;

            // Box Bounds
            let margin = 5.0;
            let turn_factor = 0.5;
            let bounds = self.bounds_size;

            if self.boids[i].position.x > bounds - margin {
                self.boids[i].acceleration.x -= turn_factor;
            }
            if self.boids[i].position.x < -bounds + margin {
                self.boids[i].acceleration.x += turn_factor;
            }

            if self.boids[i].position.y > bounds - margin {
                self.boids[i].acceleration.y -= turn_factor;
            }
            if self.boids[i].position.y < -bounds + margin {
                self.boids[i].acceleration.y += turn_factor;
            }

            if self.boids[i].position.z > bounds - margin {
                self.boids[i].acceleration.z -= turn_factor;
            }
            if self.boids[i].position.z < -bounds + margin {
                self.boids[i].acceleration.z += turn_factor;
            }
        }

        for boid in &mut self.boids {
            boid.position = boid.position + boid.velocity * dt;
            boid.velocity = boid.velocity + boid.acceleration * dt;
            if boid.velocity.length_sq() > 0.0 {
                boid.velocity = boid.velocity.normalize() * self.max_speed;
            }
            boid.acceleration = Vec3::default();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boid_system_init() {
        let sys = BoidSystem::new(10, 5.0);
        assert_eq!(sys.boids.len(), 10);
    }

    #[test]
    fn test_boid_cohesion() {
        let mut sys = BoidSystem::new(2, 5.0);
        // Force them close enough to perceive each other, but not on top of each other
        sys.boids[0].position = Vec3::new(0.0, 0.0, 0.0);
        sys.boids[0].velocity = Vec3::new(0.0, 1.0, 0.0);

        sys.boids[1].position = Vec3::new(2.0, 0.0, 0.0);
        sys.boids[1].velocity = Vec3::new(0.0, 1.0, 0.0);

        sys.separation_weight = 0.0;
        sys.alignment_weight = 0.0;
        sys.cohesion_weight = 1.0;

        sys.update(1.0);

        // Cohesion should pull boid 0 towards boid 1 (+x)
        assert!(sys.boids[0].velocity.x > 0.0);
        // Cohesion should pull boid 1 towards boid 0 (-x)
        assert!(sys.boids[1].velocity.x < 0.0);
    }

    #[test]
    fn test_boid_separation() {
        let mut sys = BoidSystem::new(2, 5.0);
        sys.boids[0].position = Vec3::new(0.0, 0.0, 0.0);
        sys.boids[0].velocity = Vec3::new(0.0, 1.0, 0.0);

        sys.boids[1].position = Vec3::new(1.0, 0.0, 0.0);
        sys.boids[1].velocity = Vec3::new(0.0, 1.0, 0.0);

        sys.separation_weight = 1.0;
        sys.alignment_weight = 0.0;
        sys.cohesion_weight = 0.0;

        sys.update(1.0);

        // Separation should push boid 0 away from boid 1 (-x)
        assert!(sys.boids[0].velocity.x < 0.0);
        // Separation should push boid 1 away from boid 0 (+x)
        assert!(sys.boids[1].velocity.x > 0.0);
    }
}
