//! Physarum (Slime Mold) Simulation.
//!
//! Simulates thousands of agents (particles) following a diffusion/decay map.

#![cfg(feature = "nova")]

use crate::framebuffer::Framebuffer;
use abrash_core::color::Color;
use abrash_core::utils::XorShift32;

/// Configuration parameters for the Physarum simulation.
#[derive(Debug, Clone, Copy)]
pub struct PhysarumConfig {
    /// Sensor offset distance.
    pub sensor_offset_dist: f32,
    /// Sensor angle spacing (in radians).
    pub sensor_angle_spacing: f32,
    /// Rotation angle when turning (in radians).
    pub turn_speed: f32,
    /// Movement speed (pixels per frame).
    pub speed: f32,
    /// Trail diffusion rate.
    pub diffuse_rate: f32,
    /// Trail decay rate.
    pub decay_rate: f32,
    /// The number of agents to simulate.
    pub num_agents: usize,
}

impl Default for PhysarumConfig {
    fn default() -> Self {
        Self {
            sensor_offset_dist: 9.0,
            sensor_angle_spacing: 0.392, // 22.5 degrees
            turn_speed: 0.3,
            speed: 1.0,
            diffuse_rate: 0.5,
            decay_rate: 0.05,
            num_agents: 100_000,
        }
    }
}

/// A single Physarum agent.
#[derive(Debug, Clone, Copy, Default)]
struct Agent {
    x: f32,
    y: f32,
    angle: f32,
}

/// A 2D grid for the Physarum simulation.
pub struct Physarum {
    pub width: usize,
    pub height: usize,
    trail_map: Vec<f32>,
    next_trail_map: Vec<f32>,
    agents: Vec<Agent>,
    pub config: PhysarumConfig,
    rng: XorShift32,
}

impl Physarum {
    /// Creates a new Physarum simulation.
    #[must_use]
    pub fn new(width: usize, height: usize, config: PhysarumConfig) -> Self {
        let size = width * height;
        let mut rng = XorShift32::new(12345);

        let mut agents = Vec::with_capacity(config.num_agents);
        let center_x = width as f32 / 2.0;
        let center_y = height as f32 / 2.0;
        let radius = (width.min(height) as f32 / 2.0) * 0.8;

        for _ in 0..config.num_agents {
            // Distribute uniformly in a circle
            let r = radius * rng.next_f32().sqrt();
            let theta = rng.next_f32() * std::f32::consts::TAU;

            let x = center_x + r * theta.cos();
            let y = center_y + r * theta.sin();

            // Point inwards or outwards? Let's do random.
            let angle = rng.next_f32() * std::f32::consts::TAU;

            agents.push(Agent { x, y, angle });
        }

        Self {
            width,
            height,
            trail_map: vec![0.0; size],
            next_trail_map: vec![0.0; size],
            agents,
            config,
            rng,
        }
    }

    /// Sense the trail map at a given offset and angle.
    fn sense(
        trail_map: &[f32],
        width: usize,
        height: usize,
        config: &PhysarumConfig,
        agent: &Agent,
        angle_offset: f32,
    ) -> f32 {
        let sensor_angle = agent.angle + angle_offset;
        let sensor_dir_x = sensor_angle.cos();
        let sensor_dir_y = sensor_angle.sin();
        let sensor_x = agent.x + sensor_dir_x * config.sensor_offset_dist;
        let sensor_y = agent.y + sensor_dir_y * config.sensor_offset_dist;

        // Sample a 3x3 region around the sensor
        let mut sum = 0.0;
        let cx = sensor_x as i32;
        let cy = sensor_y as i32;

        let w = width as i32;
        let h = height as i32;

        for dy in -1..=1 {
            for dx in -1..=1 {
                let x = cx + dx;
                let y = cy + dy;

                if x >= 0 && x < w && y >= 0 && y < h {
                    sum += trail_map[(y * w + x) as usize];
                }
            }
        }

        sum
    }

    /// Advances the simulation by one time step.
    pub fn step(&mut self) {
        let w = self.width;
        let h = self.height;

        if w == 0 || h == 0 {
            return;
        }

        let config = self.config;

        // 1. Move Agents
        // We need a random value per agent to break symmetry
        for agent in &mut self.agents {
            let weight_forward = Self::sense(&self.trail_map, w, h, &config, agent, 0.0);
            let weight_left = Self::sense(
                &self.trail_map,
                w,
                h,
                &config,
                agent,
                config.sensor_angle_spacing,
            );
            let weight_right = Self::sense(
                &self.trail_map,
                w,
                h,
                &config,
                agent,
                -config.sensor_angle_spacing,
            );

            let random_steer_strength = self.rng.next_f32();

            // Steer based on sensor weights
            if weight_forward > weight_left && weight_forward > weight_right {
                // Keep going forward
            } else if weight_forward < weight_left && weight_forward < weight_right {
                // Randomly turn left or right
                if random_steer_strength > 0.5 {
                    agent.angle += config.turn_speed * random_steer_strength;
                } else {
                    agent.angle -= config.turn_speed * random_steer_strength;
                }
            } else if weight_right > weight_left {
                // Turn right
                agent.angle -= config.turn_speed * random_steer_strength;
            } else if weight_left > weight_right {
                // Turn left
                agent.angle += config.turn_speed * random_steer_strength;
            }

            // Move
            let mut next_x = agent.x + agent.angle.cos() * config.speed;
            let mut next_y = agent.y + agent.angle.sin() * config.speed;

            // Bounce off walls
            if next_x < 0.0 || next_x >= w as f32 || next_y < 0.0 || next_y >= h as f32 {
                next_x = next_x.clamp(0.0, w.saturating_sub(1) as f32);
                next_y = next_y.clamp(0.0, h.saturating_sub(1) as f32);
                agent.angle = self.rng.next_f32() * std::f32::consts::TAU;
            }

            agent.x = next_x;
            agent.y = next_y;

            // Deposit trail
            let x_int = agent.x as usize;
            let y_int = agent.y as usize;

            if x_int < w && y_int < h {
                self.trail_map[y_int * w + x_int] = 1.0;
            }
        }

        // 2. Diffuse and Decay
        let mut next_map = std::mem::take(&mut self.next_trail_map);
        let src_map = &self.trail_map;

        let diffuse_rate = config.diffuse_rate;
        let decay_rate = config.decay_rate;

        // Handle edges carefully or just ignore 1-pixel border for speed
        for y in 0..h {
            for x in 0..w {
                let mut sum = 0.0;

                // 3x3 blur
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let nx = (x as i32 + dx).clamp(0, w.saturating_sub(1) as i32) as usize;
                        let ny = (y as i32 + dy).clamp(0, h.saturating_sub(1) as i32) as usize;
                        sum += src_map[ny * w + nx];
                    }
                }

                let blurred_val = sum / 9.0;
                let original_val = src_map[y * w + x];
                let diffused_val = original_val * (1.0 - diffuse_rate) + blurred_val * diffuse_rate;

                next_map[y * w + x] = (diffused_val * (1.0 - decay_rate)).max(0.0);
            }
        }

        self.next_trail_map = std::mem::take(&mut self.trail_map);
        self.trail_map = next_map;
    }

    /// Renders the current trail map to the framebuffer.
    pub fn render(&self, fb: &mut Framebuffer, color1: u32, color2: u32) {
        let fb_w = fb.width() as usize;
        let w = self.width.min(fb_w);
        let h = self.height.min(fb.height() as usize);

        let pixels = fb.as_mut_slice();

        for y in 0..h {
            let row_start = y * fb_w;
            let grid_start = y * self.width;

            for x in 0..w {
                let val = self.trail_map[grid_start + x];

                let t = val.clamp(0.0, 1.0);

                let c1 = Color::from_argb_u32(color1);
                let c2 = Color::from_argb_u32(color2);
                let c = c1.lerp(c2, t).to_argb_u32();

                pixels[row_start + x] = c;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_physarum_init() {
        let mut sim = Physarum::new(100, 100, PhysarumConfig::default());
        assert_eq!(sim.agents.len(), 100_000);
        sim.step();
        assert!(sim.agents[0].x >= 0.0 && sim.agents[0].x < 100.0);
    }
}
