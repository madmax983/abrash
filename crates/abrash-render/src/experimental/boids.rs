//! Boids Flocking Simulation
//!
//! A procedural simulation of flocking behavior (like birds or fish) based on Craig Reynolds' Boids algorithm.
//! The algorithm simulates emergent behavior using three simple rules:
//! 1. Separation: steer to avoid crowding local flockmates
//! 2. Alignment: steer towards the average heading of local flockmates
//! 3. Cohesion: steer to move towards the average position (center of mass) of local flockmates

#![cfg(feature = "nova")]

use crate::math::Vec3;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration parameters for the Boids simulation.
#[derive(Clone, Debug)]
pub struct FlockConfig {
    pub separation_weight: f32,
    pub alignment_weight: f32,
    pub cohesion_weight: f32,
    pub bound_weight: f32,
    pub perception_radius: f32,
    pub separation_radius: f32,
    pub max_speed: f32,
    pub min_speed: f32,
    pub bounds: Vec3,
}

impl Default for FlockConfig {
    fn default() -> Self {
        Self {
            separation_weight: 1.5,
            alignment_weight: 1.0,
            cohesion_weight: 1.0,
            bound_weight: 5.0,
            perception_radius: 10.0,
            separation_radius: 2.5,
            max_speed: 10.0,
            min_speed: 3.0,
            bounds: Vec3::new(50.0, 50.0, 50.0),
        }
    }
}

/// A single simulated entity.
#[derive(Clone, Debug, PartialEq)]
pub struct Boid {
    pub position: Vec3,
    pub velocity: Vec3,
}

impl Boid {
    #[must_use]
    pub const fn new(position: Vec3, velocity: Vec3) -> Self {
        Self { position, velocity }
    }
}

/// A manager for a collection of Boids.
pub struct Flock {
    pub boids: Vec<Boid>,
    /// Pre-allocated buffer to store the previous state of the flock without per-frame allocations.
    pub old_boids: Vec<Boid>,
    pub config: FlockConfig,
}

impl Flock {
    #[must_use]
    pub const fn new(config: FlockConfig) -> Self {
        Self {
            boids: Vec::new(),
            old_boids: Vec::new(),
            config,
        }
    }

    pub fn add_boid(&mut self, boid: Boid) {
        self.boids.push(boid);
    }

    /// Updates the flock by one time step.
    pub fn update(&mut self, delta_time: f32) {
        self.old_boids.clear();
        self.old_boids.extend_from_slice(&self.boids);

        let old_boids = &self.old_boids;

        let perception_radius_sq = self.config.perception_radius * self.config.perception_radius;
        let separation_radius_sq = self.config.separation_radius * self.config.separation_radius;

        #[cfg(feature = "parallel")]
        let iter = self.boids.par_iter_mut();
        #[cfg(not(feature = "parallel"))]
        let iter = self.boids.iter_mut();

        iter.enumerate().for_each(|(i, boid)| {
            let mut separation = Vec3::new(0.0, 0.0, 0.0);
            let mut alignment = Vec3::new(0.0, 0.0, 0.0);
            let mut cohesion = Vec3::new(0.0, 0.0, 0.0);

            let mut total_boids_perceived = 0;
            let mut total_boids_separated = 0;

            for (j, other_boid) in old_boids.iter().enumerate() {
                if i == j {
                    continue;
                }

                let dx = boid.position.x - other_boid.position.x;
                let dy = boid.position.y - other_boid.position.y;
                let dz = boid.position.z - other_boid.position.z;

                // Bolt Performance Optimization:
                // Replace `dx.hypot(dy).hypot(dz)` distance calculation with squared distance.
                // This avoids expensive square root operations for entities that fall entirely
                // outside the relevant interaction radiuses.
                let distance_sq = dx * dx + dy * dy + dz * dz;

                if distance_sq > 0.0 && distance_sq < perception_radius_sq {
                    // Alignment
                    alignment.x += other_boid.velocity.x;
                    alignment.y += other_boid.velocity.y;
                    alignment.z += other_boid.velocity.z;

                    // Cohesion
                    cohesion.x += other_boid.position.x;
                    cohesion.y += other_boid.position.y;
                    cohesion.z += other_boid.position.z;

                    total_boids_perceived += 1;
                }

                if distance_sq > 0.0 && distance_sq < separation_radius_sq {
                    let distance = distance_sq.sqrt();
                    // Separation (weighted by inverse distance)
                    let diff_x = boid.position.x - other_boid.position.x;
                    let diff_y = boid.position.y - other_boid.position.y;
                    let diff_z = boid.position.z - other_boid.position.z;

                    let inv_dist = 1.0 / distance;
                    separation.x += diff_x * inv_dist;
                    separation.y += diff_y * inv_dist;
                    separation.z += diff_z * inv_dist;

                    total_boids_separated += 1;
                }
            }

            if total_boids_perceived > 0 {
                let f_perceived = total_boids_perceived as f32;
                alignment.x /= f_perceived;
                alignment.y /= f_perceived;
                alignment.z /= f_perceived;

                // Alignment steers towards the average heading
                alignment.x = (alignment.x - boid.velocity.x) * self.config.alignment_weight;
                alignment.y = (alignment.y - boid.velocity.y) * self.config.alignment_weight;
                alignment.z = (alignment.z - boid.velocity.z) * self.config.alignment_weight;

                cohesion.x /= f_perceived;
                cohesion.y /= f_perceived;
                cohesion.z /= f_perceived;

                // Cohesion steers towards the center of mass
                cohesion.x = (cohesion.x - boid.position.x) * self.config.cohesion_weight;
                cohesion.y = (cohesion.y - boid.position.y) * self.config.cohesion_weight;
                cohesion.z = (cohesion.z - boid.position.z) * self.config.cohesion_weight;
            }

            if total_boids_separated > 0 {
                let f_separated = total_boids_separated as f32;
                separation.x /= f_separated;
                separation.y /= f_separated;
                separation.z /= f_separated;

                separation.x *= self.config.separation_weight;
                separation.y *= self.config.separation_weight;
                separation.z *= self.config.separation_weight;
            }

            // Boundary avoidance
            let mut bounds_steering = Vec3::new(0.0, 0.0, 0.0);
            let margin = 5.0; // Distance from the bounds to start turning

            // X bounds
            if boid.position.x < margin {
                bounds_steering.x += self.config.bound_weight;
            } else if boid.position.x > self.config.bounds.x - margin {
                bounds_steering.x -= self.config.bound_weight;
            }

            // Y bounds
            if boid.position.y < margin {
                bounds_steering.y += self.config.bound_weight;
            } else if boid.position.y > self.config.bounds.y - margin {
                bounds_steering.y -= self.config.bound_weight;
            }

            // Z bounds
            if boid.position.z < margin {
                bounds_steering.z += self.config.bound_weight;
            } else if boid.position.z > self.config.bounds.z - margin {
                bounds_steering.z -= self.config.bound_weight;
            }

            // Apply steering forces
            boid.velocity.x +=
                (separation.x + alignment.x + cohesion.x + bounds_steering.x) * delta_time;
            boid.velocity.y +=
                (separation.y + alignment.y + cohesion.y + bounds_steering.y) * delta_time;
            boid.velocity.z +=
                (separation.z + alignment.z + cohesion.z + bounds_steering.z) * delta_time;

            // Clamp speed
            let speed_sq = boid.velocity.x * boid.velocity.x
                + boid.velocity.y * boid.velocity.y
                + boid.velocity.z * boid.velocity.z;
            let speed = speed_sq.sqrt();

            if speed > self.config.max_speed {
                let f = self.config.max_speed / speed;
                boid.velocity.x *= f;
                boid.velocity.y *= f;
                boid.velocity.z *= f;
            } else if speed < self.config.min_speed && speed > 0.001 {
                let f = self.config.min_speed / speed;
                boid.velocity.x *= f;
                boid.velocity.y *= f;
                boid.velocity.z *= f;
            }

            // Update position
            boid.position.x += boid.velocity.x * delta_time;
            boid.position.y += boid.velocity.y * delta_time;
            boid.position.z += boid.velocity.z * delta_time;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_separation() {
        let mut flock = Flock::new(FlockConfig {
            separation_weight: 1.0,
            alignment_weight: 0.0,
            cohesion_weight: 0.0,
            bound_weight: 0.0,
            perception_radius: 10.0,
            separation_radius: 5.0,
            max_speed: 10.0,
            min_speed: 0.0,
            bounds: Vec3::new(100.0, 100.0, 100.0),
        });

        flock.add_boid(Boid::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
        ));
        flock.add_boid(Boid::new(
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
        ));

        flock.update(1.0);

        // Boid 0 should move away from Boid 1 (to the left)
        assert!(flock.boids[0].velocity.x < 0.0);
        // Boid 1 should move away from Boid 0 (to the right)
        assert!(flock.boids[1].velocity.x > 0.0);
    }

    #[test]
    fn test_alignment() {
        let mut flock = Flock::new(FlockConfig {
            separation_weight: 0.0,
            alignment_weight: 1.0,
            cohesion_weight: 0.0,
            bound_weight: 0.0,
            perception_radius: 10.0,
            separation_radius: 5.0,
            max_speed: 10.0,
            min_speed: 0.0,
            bounds: Vec3::new(100.0, 100.0, 100.0),
        });

        flock.add_boid(Boid::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
        ));
        flock.add_boid(Boid::new(
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(5.0, 5.0, 5.0),
        ));

        flock.update(1.0);

        // Boid 0 should align its velocity with Boid 1
        assert!(flock.boids[0].velocity.x > 0.0);
        assert!(flock.boids[0].velocity.y > 0.0);
        assert!(flock.boids[0].velocity.z > 0.0);
    }

    #[test]
    fn test_cohesion() {
        let mut flock = Flock::new(FlockConfig {
            separation_weight: 0.0,
            alignment_weight: 0.0,
            cohesion_weight: 1.0,
            bound_weight: 0.0,
            perception_radius: 10.0,
            separation_radius: 5.0,
            max_speed: 10.0,
            min_speed: 0.0,
            bounds: Vec3::new(100.0, 100.0, 100.0),
        });

        flock.add_boid(Boid::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
        ));
        flock.add_boid(Boid::new(
            Vec3::new(4.0, 4.0, 4.0),
            Vec3::new(0.0, 0.0, 0.0),
        ));

        flock.update(1.0);

        // Boid 0 should move towards the center of mass (which is at 2,2,2)
        assert!(flock.boids[0].velocity.x > 0.0);
        assert!(flock.boids[0].velocity.y > 0.0);
        assert!(flock.boids[0].velocity.z > 0.0);
    }
}
