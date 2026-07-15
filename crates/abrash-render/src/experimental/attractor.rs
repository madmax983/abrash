use abrash_core::math::Vec3;

pub struct LorenzAttractor {
    pub position: Vec3,
    pub sigma: f32,
    pub rho: f32,
    pub beta: f32,
}

impl LorenzAttractor {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            sigma: 10.0,
            rho: 28.0,
            beta: 8.0 / 3.0,
        }
    }

    pub fn step(&mut self, dt: f32) {
        let dx = self.sigma * (self.position.y - self.position.x);
        let dy = self.position.x * (self.rho - self.position.z) - self.position.y;
        let dz = self.position.x * self.position.y - self.beta * self.position.z;

        self.position.x += dx * dt;
        self.position.y += dy * dt;
        self.position.z += dz * dt;
    }

    /// ⚡ Bolt Optimization: Batch iteration avoiding loop overhead.
    pub fn run_steps(&mut self, dt: f32, steps: usize) {
        for _ in 0..steps {
            let dx = self.sigma * (self.position.y - self.position.x);
            let dy = self.position.x * (self.rho - self.position.z) - self.position.y;
            let dz = self.position.x * self.position.y - self.beta * self.position.z;

            self.position.x += dx * dt;
            self.position.y += dy * dt;
            self.position.z += dz * dt;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::math::Vec3;

    #[test]
    fn test_lorenz_step() {
        let mut attractor = LorenzAttractor::new(Vec3::new(1.0, 1.0, 1.0));
        attractor.step(0.01);
        assert_eq!(attractor.position.x, 1.0);
        assert_ne!(attractor.position.y, 1.0);
        assert_ne!(attractor.position.z, 1.0);
    }

    #[test]
    fn test_lorenz_run_steps() {
        let mut a1 = LorenzAttractor::new(Vec3::new(1.0, 1.0, 1.0));
        let mut a2 = LorenzAttractor::new(Vec3::new(1.0, 1.0, 1.0));
        for _ in 0..10 { a1.step(0.01); }
        a2.run_steps(0.01, 10);
        assert_eq!(a1.position.x, a2.position.x);
        assert_eq!(a1.position.y, a2.position.y);
        assert_eq!(a1.position.z, a2.position.z);
    }
}
