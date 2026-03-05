//! Boids Flocking Simulation
//!
//! A 3D flocking simulation implementing Craig Reynolds' classic Boids rules:
//! 1. Separation: steer to avoid crowding local flockmates.
//! 2. Alignment: steer towards the average heading of local flockmates.
//! 3. Cohesion: steer to move towards the average position of local flockmates.

use crate::math::{Mat4, Vec3};
use crate::utils::XorShift32;

/// Represents a single boid in the flock.
#[derive(Clone, Copy, Debug)]
pub struct Boid {
    /// Current 3D position.
    pub position: Vec3,
    /// Current 3D velocity.
    pub velocity: Vec3,
    /// Color of the boid.
    pub color: u32,
}

impl Boid {
    /// Creates a new boid with given position, velocity, and color.
    #[must_use]
    pub const fn new(position: Vec3, velocity: Vec3, color: u32) -> Self {
        Self {
            position,
            velocity,
            color,
        }
    }

    /// Computes the orientation matrix for rendering the boid so it faces its velocity direction.
    /// The local 'forward' is assumed to be along the +Z axis.
    #[must_use]
    pub fn transform_matrix(&self, scale: f32) -> Mat4 {
        // Forward vector (Z)
        let forward = if self.velocity.length_sq() > 0.0001 {
            self.velocity.normalize()
        } else {
            Vec3::new(0.0, 0.0, 1.0)
        };

        // Assume an arbitrary UP vector
        let mut up_temp = Vec3::new(0.0, 1.0, 0.0);

        // If forward is nearly parallel to UP, choose a different UP
        if forward.dot(up_temp).abs() > 0.99 {
            up_temp = Vec3::new(1.0, 0.0, 0.0);
        }

        // Right vector (X) = UP x Forward
        let right = up_temp.cross(forward).normalize();

        // True UP vector (Y) = Forward x Right
        let up = forward.cross(right).normalize();

        // Create Rotation Matrix directly
        let rot = Mat4 {
            m: [
                [right.x, right.y, right.z, 0.0],
                [up.x, up.y, up.z, 0.0],
                [forward.x, forward.y, forward.z, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        };

        let scale_mat = Mat4::scale(scale, scale, scale);
        let trans_mat = Mat4::translation(self.position.x, self.position.y, self.position.z);

        // Transform = Scale * Rotation * Translation
        scale_mat * rot * trans_mat
    }
}

/// The system managing the flock of boids.
pub struct BoidSystem {
    /// List of boids.
    pub boids: Vec<Boid>,

    /// Distance within which boids try to separate.
    pub separation_radius: f32,
    /// Distance within which boids align and cohere.
    pub visual_radius: f32,

    /// Weight for separation force.
    pub separation_weight: f32,
    /// Weight for alignment force.
    pub alignment_weight: f32,
    /// Weight for cohesion force.
    pub cohesion_weight: f32,
    /// Weight for avoiding the boundary.
    pub boundary_weight: f32,

    /// Maximum speed a boid can travel.
    pub max_speed: f32,
    /// Maximum steering force applied per frame.
    pub max_force: f32,

    /// Boundary size (box from -size to +size on all axes).
    pub boundary_size: f32,
}

impl BoidSystem {
    /// Creates a new Boid System with default parameters.
    #[must_use]
    pub fn new(num_boids: usize, boundary_size: f32) -> Self {
        let mut rng = XorShift32::new(1337);
        let mut boids = Vec::with_capacity(num_boids);

        for _ in 0..num_boids {
            let position = Vec3::new(
                rng.next_f32_signed() * boundary_size * 0.5,
                rng.next_f32_signed() * boundary_size * 0.5,
                rng.next_f32_signed() * boundary_size * 0.5,
            );

            let velocity = Vec3::new(
                rng.next_f32_signed(),
                rng.next_f32_signed(),
                rng.next_f32_signed(),
            )
            .normalize()
                * 2.0;

            let r = (rng.next_u32() % 155 + 100) as u32;
            let g = (rng.next_u32() % 155 + 100) as u32;
            let b = (rng.next_u32() % 155 + 100) as u32;
            let color = 0xFF000000 | (r << 16) | (g << 8) | b;

            boids.push(Boid::new(position, velocity, color));
        }

        Self {
            boids,
            separation_radius: 1.5,
            visual_radius: 4.0,
            separation_weight: 1.5,
            alignment_weight: 1.0,
            cohesion_weight: 1.0,
            boundary_weight: 2.0,
            max_speed: 5.0,
            max_force: 0.1,
            boundary_size,
        }
    }

    /// Updates the boids simulation by `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        // Compute forces based on current state to avoid order-dependency issues
        let mut new_velocities = Vec::with_capacity(self.boids.len());

        for i in 0..self.boids.len() {
            let boid = self.boids[i];

            let mut separation_steer = Vec3::ZERO;
            let mut alignment_steer = Vec3::ZERO;
            let mut cohesion_center = Vec3::ZERO;

            let mut total_neighbors = 0;
            let mut sep_neighbors = 0;

            for j in 0..self.boids.len() {
                if i == j {
                    continue;
                }

                let other = self.boids[j];
                let diff = boid.position - other.position;
                let dist_sq = diff.length_sq();

                if dist_sq < self.visual_radius * self.visual_radius {
                    let dist = dist_sq.sqrt();

                    // Alignment
                    alignment_steer = alignment_steer + other.velocity;
                    // Cohesion
                    cohesion_center = cohesion_center + other.position;
                    total_neighbors += 1;

                    // Separation
                    if dist > 0.0 && dist < self.separation_radius {
                        separation_steer = separation_steer + (diff.normalize() / dist);
                        sep_neighbors += 1;
                    }
                }
            }

            let mut acceleration = Vec3::ZERO;

            if sep_neighbors > 0 {
                separation_steer = separation_steer * (1.0 / sep_neighbors as f32);
                if separation_steer.length_sq() > 0.0 {
                    separation_steer =
                        separation_steer.normalize() * self.max_speed - boid.velocity;
                    separation_steer = Self::limit_vec(separation_steer, self.max_force);
                    acceleration = acceleration + separation_steer * self.separation_weight;
                }
            }

            if total_neighbors > 0 {
                // Alignment
                alignment_steer = alignment_steer * (1.0 / total_neighbors as f32);
                if alignment_steer.length_sq() > 0.0 {
                    alignment_steer = alignment_steer.normalize() * self.max_speed - boid.velocity;
                    alignment_steer = Self::limit_vec(alignment_steer, self.max_force);
                    acceleration = acceleration + alignment_steer * self.alignment_weight;
                }

                // Cohesion
                cohesion_center = cohesion_center * (1.0 / total_neighbors as f32);
                let desired = cohesion_center - boid.position;
                if desired.length_sq() > 0.0 {
                    let steer = desired.normalize() * self.max_speed - boid.velocity;
                    let steer = Self::limit_vec(steer, self.max_force);
                    acceleration = acceleration + steer * self.cohesion_weight;
                }
            }

            // Boundary avoidance (soft bounding box)
            let mut boundary_steer = Vec3::ZERO;
            let margin = self.boundary_size * 0.8; // Start turning around at 80% to edge

            if boid.position.x < -margin {
                boundary_steer.x = 1.0;
            }
            if boid.position.x > margin {
                boundary_steer.x = -1.0;
            }
            if boid.position.y < -margin {
                boundary_steer.y = 1.0;
            }
            if boid.position.y > margin {
                boundary_steer.y = -1.0;
            }
            if boid.position.z < -margin {
                boundary_steer.z = 1.0;
            }
            if boid.position.z > margin {
                boundary_steer.z = -1.0;
            }

            if boundary_steer.length_sq() > 0.0 {
                let steer = boundary_steer.normalize() * self.max_speed - boid.velocity;
                let steer = Self::limit_vec(steer, self.max_force * 2.0);
                acceleration = acceleration + steer * self.boundary_weight;
            }

            // Simple integration
            let mut vel = boid.velocity + acceleration;
            vel = Self::limit_vec(vel, self.max_speed);

            new_velocities.push(vel);
        }

        // Apply new states
        for (i, vel) in new_velocities.iter().enumerate() {
            let vel = *vel;
            self.boids[i].velocity = vel;
            self.boids[i].position = self.boids[i].position + vel * dt;
        }
    }

    fn limit_vec(v: Vec3, max: f32) -> Vec3 {
        let len_sq = v.length_sq();
        if len_sq > max * max {
            v.normalize() * max
        } else {
            v
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boid_transform_matrix() {
        let boid = Boid::new(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(0.0, 0.0, 1.0),
            0xFFFFFFFF,
        );
        let mat = boid.transform_matrix(1.0);

        let local_forward = Vec3::new(0.0, 0.0, 1.0);
        let (world_forward, _) = mat.transform_point(local_forward);

        // Origin of boid is at (1,2,3), local forward (0,0,1) should point to (1,2,4)
        assert!((world_forward.z - 4.0).abs() < 0.001);
        assert!((world_forward.x - 1.0).abs() < 0.001);
        assert!((world_forward.y - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_boid_system_update() {
        let mut sys = BoidSystem::new(10, 10.0);
        sys.update(0.1);
        // Ensure they moved
        assert!(sys.boids[0].velocity.length() > 0.0);
    }
}
