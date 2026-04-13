//! Physarum (Slime Mold) Simulation
//!
//! Simulates a network of agents that deposit chemical trails and follow them,
//! mimicking the behavior of Physarum polycephalum finding optimal paths.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::XorShift32;

/// Configuration for the Physarum simulation.
#[derive(Debug, Clone, Copy)]
pub struct PhysarumConfig {
    pub sensor_angle: f32,
    pub sensor_distance: f32,
    pub rotation_angle: f32,
    pub move_speed: f32,
    pub decay_rate: f32,
    pub diffuse_rate: f32,
    pub deposit_amount: f32,
}

impl Default for PhysarumConfig {
    fn default() -> Self {
        Self {
            sensor_angle: 0.392, // roughly 22.5 degrees
            sensor_distance: 9.0,
            rotation_angle: 0.785, // roughly 45 degrees
            move_speed: 1.0,
            decay_rate: 0.05,
            diffuse_rate: 0.1,
            deposit_amount: 1.0,
        }
    }
}

/// A single particle/agent in the simulation.
#[derive(Debug, Clone)]
pub struct Agent {
    pub x: f32,
    pub y: f32,
    pub heading: f32, // angle in radians
}

pub struct PhysarumSimulation {
    pub width: usize,
    pub height: usize,
    pub agents: Vec<Agent>,
    pub trail_map: Vec<f32>,
    pub diffuse_map: Vec<f32>,
    pub config: PhysarumConfig,
}

impl PhysarumSimulation {
    #[must_use]
    pub fn new(width: usize, height: usize, num_agents: usize, rng: &mut XorShift32) -> Self {
        let mut agents = Vec::with_capacity(num_agents);
        for _ in 0..num_agents {
            agents.push(Agent {
                x: rng.next_f32() * width as f32,
                y: rng.next_f32() * height as f32,
                heading: rng.next_f32() * std::f32::consts::TAU,
            });
        }

        Self {
            width,
            height,
            agents,
            trail_map: vec![0.0; width * height],
            diffuse_map: vec![0.0; width * height],
            config: PhysarumConfig::default(),
        }
    }

    pub fn step(&mut self, rng: &mut XorShift32) {
        // 1. Move Agents & Deposit
        // We iterate using indices so we don't hold a mutable reference to self while calling self methods
        let num_agents = self.agents.len();
        for i in 0..num_agents {
            // Read sensors using the trail map directly
            let agent = &self.agents[i];
            let heading = agent.heading;
            let ax = agent.x;
            let ay = agent.y;

            let mut sense = |angle_offset: f32| -> f32 {
                let sensor_angle = heading + angle_offset;
                let sensor_x = ax + sensor_angle.cos() * self.config.sensor_distance;
                let sensor_y = ay + sensor_angle.sin() * self.config.sensor_distance;

                let sx = sensor_x as i32;
                let sy = sensor_y as i32;

                if sx < 0 || sx >= self.width as i32 || sy < 0 || sy >= self.height as i32 {
                    return 0.0;
                }

                self.trail_map[(sy * self.width as i32 + sx) as usize]
            };

            let weight_forward = sense(0.0);
            let weight_left = sense(-self.config.sensor_angle);
            let weight_right = sense(self.config.sensor_angle);

            let random_steer = rng.next_f32();

            let mut agent_mut = &mut self.agents[i];

            if weight_forward > weight_left && weight_forward > weight_right {
                // Keep going forward
            } else if weight_forward < weight_left && weight_forward < weight_right {
                if random_steer > 0.5 {
                    agent_mut.heading += self.config.rotation_angle;
                } else {
                    agent_mut.heading -= self.config.rotation_angle;
                }
            } else if weight_left > weight_right {
                agent_mut.heading -= self.config.rotation_angle;
            } else if weight_right > weight_left {
                agent_mut.heading += self.config.rotation_angle;
            }

            let dx = agent_mut.heading.cos() * self.config.move_speed;
            let dy = agent_mut.heading.sin() * self.config.move_speed;

            agent_mut.x += dx;
            agent_mut.y += dy;

            // Handle boundaries (bounce)
            if agent_mut.x < 0.0 || agent_mut.x >= self.width as f32 {
                agent_mut.x = agent_mut.x.clamp(0.0, self.width as f32 - 1.0);
                agent_mut.heading = std::f32::consts::PI - agent_mut.heading;
            }
            if agent_mut.y < 0.0 || agent_mut.y >= self.height as f32 {
                agent_mut.y = agent_mut.y.clamp(0.0, self.height as f32 - 1.0);
                agent_mut.heading = -agent_mut.heading;
            }

            let cx = agent_mut.x as usize;
            let cy = agent_mut.y as usize;
            let idx = cy * self.width + cx;

            self.trail_map[idx] += self.config.deposit_amount;
            self.trail_map[idx] = self.trail_map[idx].min(1.0); // Clamp deposit
        }

        // 2. Diffuse & Decay
        let w = self.width;
        let h = self.height;
        let decay = self.config.decay_rate;
        let diffuse = self.config.diffuse_rate;

        // Use diffuse_map as the next state
        for y in 0..h {
            for x in 0..w {
                let mut sum = 0.0;

                // 3x3 box blur for diffusion
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;

                        if nx >= 0 && nx < w as i32 && ny >= 0 && ny < h as i32 {
                            sum += self.trail_map[(ny * w as i32 + nx) as usize];
                        }
                    }
                }

                let blurred = sum / 9.0;
                let original = self.trail_map[y * w + x];

                let next_val = original * (1.0 - diffuse) + blurred * diffuse;
                let decayed = next_val * (1.0 - decay);

                self.diffuse_map[y * w + x] = decayed.clamp(0.0, 1.0);
            }
        }

        // Swap maps
        std::mem::swap(&mut self.trail_map, &mut self.diffuse_map);
    }

    pub fn render(&self, fb: &mut Framebuffer) {
        let pixels = fb.as_mut_slice();
        for (pixel, &val) in pixels.iter_mut().zip(self.trail_map.iter()) {
            let intensity = (val * 255.0) as u32;
            let color = 0xFF00_0000 | (intensity << 16) | (intensity << 8) | intensity;
            *pixel = color;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_physarum_step() {
        let mut rng = XorShift32::new(12345);
        let mut sim = PhysarumSimulation::new(100, 100, 100, &mut rng);

        // Step the simulation
        sim.step(&mut rng);

        // Trails should be > 0.0 somewhere since agents moved and deposited
        let max_trail = sim.trail_map.iter().copied().fold(0.0, f32::max);
        assert!(max_trail > 0.0);
    }
}
