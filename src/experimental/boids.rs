use crate::math::{Vec3};

/// Represents a single Boid in the simulation.
#[derive(Debug, Clone, Copy)]
pub struct Boid {
    pub position: Vec3,
    pub velocity: Vec3,
}

impl Boid {
    /// Creates a new Boid.
    #[must_use]
    pub const fn new(position: Vec3, velocity: Vec3) -> Self {
        Self { position, velocity }
    }
}

/// Simulation of a flock of Boids.
pub struct BoidsSimulation {
    pub boids: Vec<Boid>,
    pub separation_radius: f32,
    pub alignment_radius: f32,
    pub cohesion_radius: f32,
    pub separation_weight: f32,
    pub alignment_weight: f32,
    pub cohesion_weight: f32,
    pub max_speed: f32,
    pub min_speed: f32,
    pub bounds: f32,
}

impl BoidsSimulation {
    /// Create a new BoidsSimulation.
    #[must_use]
    pub fn new(boids: Vec<Boid>) -> Self {
        Self {
            boids,
            separation_radius: 2.0,
            alignment_radius: 5.0,
            cohesion_radius: 5.0,
            separation_weight: 1.5,
            alignment_weight: 1.0,
            cohesion_weight: 1.0,
            max_speed: 10.0,
            min_speed: 2.0,
            bounds: 50.0,
        }
    }

    /// Step the simulation forward by `dt` seconds.
    pub fn step(&mut self, dt: f32) {
        let boids_len = self.boids.len();
        if boids_len == 0 {
            return;
        }

        // We need a snapshot of the current state to compute the next state
        let old_boids = self.boids.clone();

        #[cfg(feature = "parallel")]
        use rayon::prelude::*;

        let update_boid = |(i, boid_out): (usize, &mut Boid)| {
            let boid = &old_boids[i];

            let mut separation = Vec3::ZERO;
            let mut alignment = Vec3::ZERO;
            let mut cohesion = Vec3::ZERO;

            let mut separation_count = 0;
            let mut alignment_count = 0;
            let mut cohesion_count = 0;

            for j in 0..boids_len {
                if i == j {
                    continue;
                }

                let other = &old_boids[j];
                let diff = boid.position - other.position;
                let dist_sq = diff.length_sq();

                if dist_sq > 0.0 {
                    // Separation
                    if dist_sq < self.separation_radius * self.separation_radius {
                        separation = separation + (diff.normalize() / dist_sq.sqrt());
                        separation_count += 1;
                    }

                    // Alignment
                    if dist_sq < self.alignment_radius * self.alignment_radius {
                        alignment = alignment + other.velocity;
                        alignment_count += 1;
                    }

                    // Cohesion
                    if dist_sq < self.cohesion_radius * self.cohesion_radius {
                        cohesion = cohesion + other.position;
                        cohesion_count += 1;
                    }
                }
            }

            let mut acceleration = Vec3::ZERO;

            if separation_count > 0 {
                separation = separation / (separation_count as f32);
                if separation.length_sq() > 0.0 {
                    separation = separation.normalize() * self.max_speed - boid.velocity;
                }
                acceleration = acceleration + separation * self.separation_weight;
            }

            if alignment_count > 0 {
                alignment = alignment / (alignment_count as f32);
                if alignment.length_sq() > 0.0 {
                    alignment = alignment.normalize() * self.max_speed - boid.velocity;
                }
                acceleration = acceleration + alignment * self.alignment_weight;
            }

            if cohesion_count > 0 {
                cohesion = cohesion / (cohesion_count as f32);
                let desired = cohesion - boid.position;
                if desired.length_sq() > 0.0 {
                    cohesion = desired.normalize() * self.max_speed - boid.velocity;
                } else {
                    cohesion = Vec3::ZERO;
                }
                acceleration = acceleration + cohesion * self.cohesion_weight;
            }

            // Boundary avoidance (soft boundary)
            let margin = 5.0;
            let turn_factor = 20.0;
            let mut bounds_accel = Vec3::ZERO;

            if boid.position.x < -self.bounds + margin {
                bounds_accel.x += turn_factor;
            } else if boid.position.x > self.bounds - margin {
                bounds_accel.x -= turn_factor;
            }

            if boid.position.y < -self.bounds + margin {
                bounds_accel.y += turn_factor;
            } else if boid.position.y > self.bounds - margin {
                bounds_accel.y -= turn_factor;
            }

            if boid.position.z < -self.bounds + margin {
                bounds_accel.z += turn_factor;
            } else if boid.position.z > self.bounds - margin {
                bounds_accel.z -= turn_factor;
            }

            acceleration = acceleration + bounds_accel;

            // Apply acceleration
            let mut new_velocity = boid.velocity + acceleration * dt;

            // Limit speed
            let speed_sq = new_velocity.length_sq();
            if speed_sq > self.max_speed * self.max_speed {
                new_velocity = new_velocity.normalize() * self.max_speed;
            } else if speed_sq < self.min_speed * self.min_speed && speed_sq > 0.0001 {
                new_velocity = new_velocity.normalize() * self.min_speed;
            } else if speed_sq <= 0.0001 {
                 new_velocity = Vec3::new(self.min_speed, 0.0, 0.0);
            }

            boid_out.velocity = new_velocity;
            boid_out.position = boid.position + new_velocity * dt;
        };

        #[cfg(feature = "parallel")]
        {
            self.boids.par_iter_mut().enumerate().for_each(update_boid);
        }

        #[cfg(not(feature = "parallel"))]
        {
            self.boids.iter_mut().enumerate().for_each(update_boid);
        }
    }

    /// Compute dynamic forward, up, and right vectors for rendering a boid.
    /// Returns (forward, up, right).
    #[must_use]
    pub fn compute_basis(velocity: Vec3) -> (Vec3, Vec3, Vec3) {
        let forward = if velocity.length_sq() > 0.000001 {
            velocity.normalize()
        } else {
            Vec3::new(0.0, 0.0, -1.0)
        };

        // Assume global up is Y
        let global_up = Vec3::new(0.0, 1.0, 0.0);

        let mut right = forward.cross(global_up);
        if right.length_sq() < 0.000001 {
            // Forward is nearly parallel to global up, use X as right
            right = Vec3::new(1.0, 0.0, 0.0);
        } else {
            right = right.normalize();
        }

        let up = right.cross(forward).normalize();

        (forward, up, right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boids_separation() {
        let b1 = Boid::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
        let b2 = Boid::new(Vec3::new(0.5, 0.0, 0.0), Vec3::new(-1.0, 0.0, 0.0)); // Too close!
        let mut sim = BoidsSimulation::new(vec![b1, b2]);
        sim.step(0.1);

        // B1 should accelerate left (negative X) to avoid B2
        assert!(sim.boids[0].velocity.x < 1.0);
    }

    #[test]
    fn test_boids_alignment() {
        let b1 = Boid::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
        let b2 = Boid::new(Vec3::new(2.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.0));
        let mut sim = BoidsSimulation::new(vec![b1, b2]);
        sim.step(0.1);

        // B1 should start aligning its X velocity with B2
        assert!(sim.boids[0].velocity.x > 0.0);
    }

    #[test]
    fn test_boids_cohesion() {
        let b1 = Boid::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
        let b2 = Boid::new(Vec3::new(4.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
        let mut sim = BoidsSimulation::new(vec![b1, b2]);
        sim.step(0.1);

        // B1 should accelerate right (positive X) towards the center of mass (B2)
        assert!(sim.boids[0].velocity.x > 0.0);
    }

    #[test]
    fn test_boids_bounds() {
        let b1 = Boid::new(Vec3::new(49.0, 0.0, 0.0), Vec3::new(5.0, 0.0, 0.0));
        let mut sim = BoidsSimulation::new(vec![b1]);
        sim.step(1.0); // Should move out of bounds (49 + 5 > 50) and get pushed back

        assert!(sim.boids[0].velocity.x < 5.0); // Velocity should be reduced or inverted
    }

    #[test]
    fn test_compute_basis() {
        let velocity = Vec3::new(1.0, 0.0, 0.0); // Moving along +X
        let (forward, up, right) = BoidsSimulation::compute_basis(velocity);

        // Forward must match velocity direction
        assert!((forward.x - 1.0).abs() < f32::EPSILON);
        assert!((forward.y - 0.0).abs() < f32::EPSILON);
        assert!((forward.z - 0.0).abs() < f32::EPSILON);

        // Orthogonality checks
        assert!(forward.dot(up).abs() < f32::EPSILON);
        assert!(forward.dot(right).abs() < f32::EPSILON);
        assert!(up.dot(right).abs() < f32::EPSILON);
    }
}