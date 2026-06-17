#![cfg(feature = "nova")]

use crate::math::Vec3;

/// Types of strange attractors that can be generated.
#[derive(Debug, Clone, Copy)]
pub enum AttractorType {
    /// The classic Lorenz attractor. Parameters typically are: sigma (10.0), rho (28.0), beta (8.0/3.0).
    Lorenz { sigma: f32, rho: f32, beta: f32 },
    /// The Roessler attractor. Parameters typically are: a (0.2), b (0.2), c (5.7).
    Roessler { a: f32, b: f32, c: f32 },
}

/// A generator for creating paths through strange attractors using numerical integration.
#[derive(Debug, Clone)]
pub struct StrangeAttractor {
    pub attractor_type: AttractorType,
}

impl StrangeAttractor {
    /// Creates a new Lorenz attractor.
    #[must_use]
    pub const fn new_lorenz(sigma: f32, rho: f32, beta: f32) -> Self {
        Self {
            attractor_type: AttractorType::Lorenz { sigma, rho, beta },
        }
    }

    /// Creates a new Roessler attractor.
    #[must_use]
    pub const fn new_roessler(a: f32, b: f32, c: f32) -> Self {
        Self {
            attractor_type: AttractorType::Roessler { a, b, c },
        }
    }

    /// Evaluates the derivative at a given point based on the attractor type.
    fn evaluate_derivative(&self, p: Vec3) -> Vec3 {
        match self.attractor_type {
            AttractorType::Lorenz { sigma, rho, beta } => {
                let dx = sigma * (p.y - p.x);
                let dy = p.x * (rho - p.z) - p.y;
                let dz = p.x * p.y - beta * p.z;
                Vec3::new(dx, dy, dz)
            }
            AttractorType::Roessler { a, b, c } => {
                let dx = -p.y - p.z;
                let dy = p.x + a * p.y;
                let dz = b + p.z * (p.x - c);
                Vec3::new(dx, dy, dz)
            }
        }
    }

    /// Generates a path of points through the attractor using Euler integration.
    ///
    /// # Arguments
    /// * `start_point` - The initial state (position).
    /// * `dt` - The time step for numerical integration.
    /// * `steps` - The number of points to generate.
    #[must_use]
    pub fn generate(&self, start_point: Vec3, dt: f32, steps: usize) -> Vec<Vec3> {
        let mut path = Vec::with_capacity(steps);
        let mut current_pos = start_point;

        for _ in 0..steps {
            path.push(current_pos);
            let derivative = self.evaluate_derivative(current_pos);

            // Simple Euler integration
            current_pos.x += derivative.x * dt;
            current_pos.y += derivative.y * dt;
            current_pos.z += derivative.z * dt;
        }

        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lorenz_attractor() {
        let attractor = StrangeAttractor::new_lorenz(10.0, 28.0, 8.0 / 3.0);
        let start_point = Vec3::new(1.0, 1.0, 1.0);
        let path = attractor.generate(start_point, 0.01, 100);

        assert_eq!(path.len(), 100);
        assert_ne!(path[0], path[99]); // Ensure it's not static
    }

    #[test]
    fn test_roessler_attractor() {
        let attractor = StrangeAttractor::new_roessler(0.2, 0.2, 5.7);
        let start_point = Vec3::new(1.0, 1.0, 1.0);
        let path = attractor.generate(start_point, 0.01, 100);

        assert_eq!(path.len(), 100);
        assert_ne!(path[0], path[99]);
    }
}
