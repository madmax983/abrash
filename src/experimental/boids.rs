//! Boids Flocking Simulation
//!
//! A procedural crowd simulation based on Craig Reynolds' Boids algorithm.

use crate::math::Vec3;

#[derive(Clone, Debug, PartialEq)]
pub struct Boid {
    pub position: Vec3,
    pub velocity: Vec3,
}

impl Boid {
    /// Compute an orthogonal basis (forward, up, right) for orienting meshes
    /// along the boid's velocity vector.
    pub fn basis_vectors(&self) -> (Vec3, Vec3, Vec3) {
        let forward = if self.velocity.length_sq() > 1e-6 {
            self.velocity.fast_normalize()
        } else {
            Vec3::new(0.0, 0.0, 1.0)
        };

        // Pick a roughly up vector. If forward is exactly up, pick another one.
        let mut temp_up = Vec3::new(0.0, 1.0, 0.0);
        if forward.dot(temp_up).abs() > 0.99 {
            temp_up = Vec3::new(0.0, 0.0, 1.0);
        }

        let right = temp_up.cross(forward).fast_normalize();
        let up = forward.cross(right).fast_normalize();

        (forward, up, right)
    }
}

pub struct Flock {
    pub boids: Vec<Boid>,
    pub perception_radius: f32,
    pub max_speed: f32,
    pub max_force: f32,

    pub separation_weight: f32,
    pub alignment_weight: f32,
    pub cohesion_weight: f32,
}

impl Flock {
    pub fn new(boids: Vec<Boid>) -> Self {
        Self {
            boids,
            perception_radius: 5.0,
            max_speed: 2.0,
            max_force: 0.05,

            separation_weight: 1.5,
            alignment_weight: 1.0,
            cohesion_weight: 1.0,
        }
    }

    pub fn update(&mut self, dt: f32) {
        let mut new_velocities = Vec::with_capacity(self.boids.len());

        for i in 0..self.boids.len() {
            let mut separation = Vec3::new(0.0, 0.0, 0.0);
            let mut alignment = Vec3::new(0.0, 0.0, 0.0);
            let mut cohesion = Vec3::new(0.0, 0.0, 0.0);
            let mut total_neighbors = 0;

            let pos_i = self.boids[i].position;
            let vel_i = self.boids[i].velocity;

            for j in 0..self.boids.len() {
                if i == j {
                    continue;
                }

                let pos_j = self.boids[j].position;
                let diff = pos_i - pos_j; // Vector from j to i
                let dist_sq = diff.length_sq();

                if dist_sq > 0.0 && dist_sq < self.perception_radius * self.perception_radius {
                    total_neighbors += 1;

                    // Separation (inversely proportional to distance)
                    let dist = dist_sq.sqrt();
                    let push = diff * (1.0 / dist);
                    separation = separation + push;

                    // Alignment (sum velocities)
                    alignment = alignment + self.boids[j].velocity;

                    // Cohesion (sum positions)
                    cohesion = cohesion + pos_j;
                }
            }

            let mut acceleration = Vec3::new(0.0, 0.0, 0.0);

            if total_neighbors > 0 {
                let neighbor_count = total_neighbors as f32;

                // Separation
                separation = separation * (1.0 / neighbor_count);
                if separation.length_sq() > 0.0 {
                    separation = separation.fast_normalize() * self.max_speed;
                    separation = separation - vel_i;
                    separation = limit(separation, self.max_force);
                }

                // Alignment
                alignment = alignment * (1.0 / neighbor_count);
                if alignment.length_sq() > 0.0 {
                    alignment = alignment.fast_normalize() * self.max_speed;
                    alignment = alignment - vel_i;
                    alignment = limit(alignment, self.max_force);
                }

                // Cohesion
                cohesion = cohesion * (1.0 / neighbor_count);
                let mut steer = cohesion - pos_i;
                if steer.length_sq() > 0.0 {
                    steer = steer.fast_normalize() * self.max_speed;
                    steer = steer - vel_i;
                    steer = limit(steer, self.max_force);
                }

                acceleration = acceleration + (separation * self.separation_weight);
                acceleration = acceleration + (alignment * self.alignment_weight);
                acceleration = acceleration + (steer * self.cohesion_weight);
            }

            // Limit speed
            let mut new_vel = vel_i + acceleration * dt * 60.0;
            let speed_sq = new_vel.length_sq();
            if speed_sq > self.max_speed * self.max_speed {
                new_vel = new_vel.fast_normalize() * self.max_speed;
            }

            new_velocities.push(new_vel);
        }

        // Apply
        for i in 0..self.boids.len() {
            self.boids[i].velocity = new_velocities[i];
            self.boids[i].position = self.boids[i].position + (self.boids[i].velocity * dt);
        }
    }
}

fn limit(v: Vec3, max: f32) -> Vec3 {
    let len_sq = v.length_sq();
    if len_sq > max * max {
        v.fast_normalize() * max
    } else {
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec3;

    #[test]
    fn test_flock_cohesion() {
        let mut flock = Flock::new(vec![
            Boid { position: Vec3::new(-1.0, 0.0, 0.0), velocity: Vec3::new(0.0, 0.0, 0.0) },
            Boid { position: Vec3::new(1.0, 0.0, 0.0), velocity: Vec3::new(0.0, 0.0, 0.0) },
        ]);

        flock.cohesion_weight = 1.0;
        flock.separation_weight = 0.0;
        flock.alignment_weight = 0.0;

        flock.update(1.0);

        // They should move towards each other
        assert!(flock.boids[0].velocity.x > 0.0);
        assert!(flock.boids[1].velocity.x < 0.0);
    }

    #[test]
    fn test_boid_basis_vectors() {
        let boid = Boid {
            position: Vec3::new(0.0, 0.0, 0.0),
            velocity: Vec3::new(1.0, 0.0, 0.0),
        };
        let (forward, up, right) = boid.basis_vectors();
        assert_eq!(forward, Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(up, Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(right, Vec3::new(0.0, 0.0, -1.0));
    }
}
