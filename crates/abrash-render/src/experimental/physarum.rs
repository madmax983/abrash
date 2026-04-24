//! Physarum (Slime Mold) Simulation
//!
//! Simulates a transport network forming based on Physarum Polycephalum.
//! Agents drop a trail map and sense it to steer towards high concentrations.

use crate::framebuffer::Framebuffer;
use abrash_core::color;
use abrash_core::utils::XorShift32;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration parameters for the Physarum simulation.
#[derive(Debug, Clone, Copy)]
pub struct PhysarumConfig {
    pub move_speed: f32,
    pub turn_speed: f32,
    pub sensor_angle: f32,
    pub sensor_dist: f32,
    pub sensor_size: i32,
    pub deposit_amount: f32,
    pub decay_rate: f32,
    pub diffuse_rate: f32,
}

impl Default for PhysarumConfig {
    fn default() -> Self {
        Self {
            move_speed: 1.0,
            turn_speed: 0.5,
            sensor_angle: 0.5,
            sensor_dist: 9.0,
            sensor_size: 1,
            deposit_amount: 1.0,
            decay_rate: 0.01,
            diffuse_rate: 0.1,
        }
    }
}

/// A single particle/agent.
#[derive(Debug, Clone, Copy)]
pub struct PhysarumAgent {
    pub x: f32,
    pub y: f32,
    pub angle: f32,
}

/// The main simulation state.
pub struct PhysarumSimulation {
    pub width: usize,
    pub height: usize,
    pub trail_map: Vec<f32>,
    pub next_trail_map: Vec<f32>,
    pub agents: Vec<PhysarumAgent>,
    pub config: PhysarumConfig,
    rng: XorShift32,
}

impl PhysarumSimulation {
    #[must_use]
    pub fn new(width: usize, height: usize, num_agents: usize) -> Self {
        let size = width * height;
        let trail_map = vec![0.0; size];
        let next_trail_map = vec![0.0; size];

        let mut rng = XorShift32::new(12345);
        let mut agents = Vec::with_capacity(num_agents);

        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;
        let radius = (width.min(height) as f32) * 0.4;

        for _ in 0..num_agents {
            let r = rng.next_f32() * radius;
            let theta = rng.next_f32() * std::f32::consts::TAU;

            agents.push(PhysarumAgent {
                x: cx + r * theta.cos(),
                y: cy + r * theta.sin(),
                angle: theta + std::f32::consts::PI, // inward facing
            });
        }

        Self {
            width,
            height,
            trail_map,
            next_trail_map,
            agents,
            config: PhysarumConfig::default(),
            rng,
        }
    }

    fn sense(
        trail_map: &[f32],
        width: usize,
        height: usize,
        agent: &PhysarumAgent,
        angle_offset: f32,
        config: &PhysarumConfig,
    ) -> f32 {
        let sensor_angle = agent.angle + angle_offset;
        let sensor_dir_x = sensor_angle.cos();
        let sensor_dir_y = sensor_angle.sin();
        let sensor_center_x = (agent.x + sensor_dir_x * config.sensor_dist) as i32;
        let sensor_center_y = (agent.y + sensor_dir_y * config.sensor_dist) as i32;

        let mut sum = 0.0;
        for dy in -config.sensor_size..=config.sensor_size {
            for dx in -config.sensor_size..=config.sensor_size {
                let px = sensor_center_x + dx;
                let py = sensor_center_y + dy;

                if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                    sum += trail_map[(py as usize) * width + (px as usize)];
                }
            }
        }
        sum
    }

    pub fn step(&mut self) {
        let w = self.width;
        let h = self.height;
        let w_f32 = w as f32;
        let h_f32 = h as f32;
        let config = self.config;

        // Note: Decoupling borrow to avoid &self vs &mut self.agents issues
        let trail_map_ref = &self.trail_map;

        // 1. Move Agents & Deposit Trail
        for agent in &mut self.agents {
            let weight_forward = Self::sense(trail_map_ref, w, h, agent, 0.0, &config);
            let weight_left = Self::sense(trail_map_ref, w, h, agent, config.sensor_angle, &config);
            let weight_right =
                Self::sense(trail_map_ref, w, h, agent, -config.sensor_angle, &config);

            let random_steer = (self.rng.next_f32() - 0.5) * 0.1;

            if weight_forward > weight_left && weight_forward > weight_right {
                // Keep going straight
                agent.angle += random_steer;
            } else if weight_forward < weight_left && weight_forward < weight_right {
                // Random turn
                if self.rng.next_f32() > 0.5 {
                    agent.angle += config.turn_speed + random_steer;
                } else {
                    agent.angle -= config.turn_speed - random_steer;
                }
            } else if weight_left > weight_right {
                agent.angle += config.turn_speed + random_steer;
            } else if weight_right > weight_left {
                agent.angle -= config.turn_speed - random_steer;
            }

            let mut nx = agent.x + agent.angle.cos() * config.move_speed;
            let mut ny = agent.y + agent.angle.sin() * config.move_speed;

            if nx < 0.0 || nx >= w_f32 || ny < 0.0 || ny >= h_f32 {
                nx = nx.clamp(0.0, w_f32 - 1.0);
                ny = ny.clamp(0.0, h_f32 - 1.0);
                agent.angle = self.rng.next_f32() * std::f32::consts::TAU;
            }

            agent.x = nx;
            agent.y = ny;

            let idx = (ny as usize) * w + (nx as usize);
            self.next_trail_map[idx] += config.deposit_amount;
        }

        // 2. Diffuse and Decay Trail Map
        let mut next_trail_map = std::mem::take(&mut self.next_trail_map);
        let current_trail_map = &self.trail_map;

        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;
                let mut sum = 0.0;
                let mut count = 0.0;

                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx >= 0 && nx < w as i32 && ny >= 0 && ny < h as i32 {
                            sum += current_trail_map[(ny as usize) * w + (nx as usize)];
                            count += 1.0;
                        }
                    }
                }

                let blurred = sum / count;
                let original = current_trail_map[idx];

                // Diffuse
                let mut diffused =
                    original * (1.0 - config.diffuse_rate) + blurred * config.diffuse_rate;

                // Add the deposits we collected this frame
                diffused += next_trail_map[idx];
                next_trail_map[idx] = 0.0; // Clear for next frame

                // Decay
                diffused *= 1.0 - config.decay_rate;
                next_trail_map[idx] = diffused.clamp(0.0, 1.0);
            }
        }

        self.next_trail_map = std::mem::take(&mut self.trail_map);
        self.trail_map = next_trail_map;
    }

    pub fn render(&self, fb: &mut Framebuffer, trail_color: u32) {
        let fb_w = fb.width() as usize;
        let w = self.width.min(fb_w);
        let h = self.height.min(fb.height() as usize);

        let pixels = fb.as_mut_slice();
        let c1 = color::Color::from_argb_u32(0xFF_000000); // Black background
        let c2 = color::Color::from_argb_u32(trail_color);

        for y in 0..h {
            let row_start = y * fb_w;
            let grid_start = y * self.width;

            for x in 0..w {
                let val = self.trail_map[grid_start + x];
                let c = c1.lerp(c2, val.clamp(0.0, 1.0)).to_argb_u32();
                pixels[row_start + x] = c;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_physarum_step() {
        let mut sim = PhysarumSimulation::new(10, 10, 1);
        sim.agents[0].x = 5.0;
        sim.agents[0].y = 5.0;
        sim.agents[0].angle = 0.0;

        sim.step();

        // Agent should move right
        assert!(sim.agents[0].x > 5.0);

        // Trail should be deposited
        let mut trail_sum = 0.0;
        for t in &sim.trail_map {
            trail_sum += *t;
        }
        assert!(trail_sum > 0.0);
    }
}
