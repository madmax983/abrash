//! Physarum: A Slime Mold Simulation Module.
//!
//! This module implements a biologically inspired multi-agent simulation based on the
//! Physarum polycephalum slime mold. Agents move around a grid, deposit trails, and
//! follow the trails left by others, resulting in complex, emergent networks.

use crate::framebuffer::Framebuffer;
use std::f32::consts::PI;

/// A single agent in the Physarum simulation.
#[derive(Clone, Copy)]
pub struct Agent {
    /// X position
    pub x: f32,
    /// Y position
    pub y: f32,
    /// Heading angle in radians
    pub angle: f32,
}

/// Configuration parameters for the Physarum simulation.
pub struct PhysarumConfig {
    /// Angle offset for left/right sensors
    pub sensor_angle: f32,
    /// Distance of sensors from agent
    pub sensor_dist: f32,
    /// Angle to turn when steering
    pub rotation_angle: f32,
    /// Distance to move per step
    pub step_size: f32,
    /// Amount of trail to deposit per step
    pub deposit_amount: f32,
    /// Trail decay factor (e.g., 0.95)
    pub decay_rate: f32,
}

impl Default for PhysarumConfig {
    fn default() -> Self {
        Self {
            sensor_angle: PI / 4.0,
            sensor_dist: 9.0,
            rotation_angle: PI / 4.0,
            step_size: 1.0,
            deposit_amount: 5.0,
            decay_rate: 0.95,
        }
    }
}

/// The Physarum (Slime Mold) simulator.
pub struct PhysarumSim {
    pub width: u32,
    pub height: u32,
    /// Grid storing the chemical trail values.
    pub trail_map: Vec<f32>,
    /// Secondary buffer for diffusion/decay step.
    pub back_buffer: Vec<f32>,
    /// Active agents.
    pub agents: Vec<Agent>,
    pub config: PhysarumConfig,
}

impl PhysarumSim {
    /// Creates a new Physarum simulation.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            trail_map: vec![0.0; (width * height) as usize],
            back_buffer: vec![0.0; (width * height) as usize],
            agents: Vec::new(),
            config: PhysarumConfig::default(),
        }
    }

    /// Adds a new agent at the specified position and angle.
    pub fn add_agent(&mut self, x: f32, y: f32, angle: f32) {
        self.agents.push(Agent { x, y, angle });
    }

    /// Retrieves the trail value at the specified coordinates.
    #[must_use]
    pub fn get_trail(&self, x: u32, y: u32) -> f32 {
        if x < self.width && y < self.height {
            self.trail_map[(y * self.width + x) as usize]
        } else {
            0.0
        }
    }

    fn sample_trail(map: &[f32], width: u32, height: u32, x: f32, y: f32) -> f32 {
        if x < 0.0 || x >= width as f32 || y < 0.0 || y >= height as f32 {
            return 0.0;
        }
        map[(y as u32 * width + x as u32) as usize]
    }

    /// Advances the simulation by one step.
    pub fn step(&mut self) {
        let width = self.width;
        let height = self.height;
        let config = &self.config;
        let trail_map = &mut self.trail_map;

        // Nova Optimization Note:
        // We decouple `trail_map` reading from `agents` to satisfy the borrow checker.
        for agent in &mut self.agents {
            // Sense
            let sense = |angle_offset: f32| -> f32 {
                let sensor_angle = agent.angle + angle_offset;
                let sx = agent.x + sensor_angle.cos() * config.sensor_dist;
                let sy = agent.y + sensor_angle.sin() * config.sensor_dist;
                Self::sample_trail(trail_map, width, height, sx, sy)
            };

            let weight_forward = sense(0.0);
            let weight_left = sense(-config.sensor_angle);
            let weight_right = sense(config.sensor_angle);

            // Steer
            if weight_forward > weight_left && weight_forward > weight_right {
                // Keep going forward
            } else if weight_forward < weight_left && weight_forward < weight_right {
                // Pseudo-random turn if left and right are equally appealing
                // To keep it deterministic for tests and simple, we alternate based on coords
                if (agent.x as u32 + agent.y as u32) % 2 == 0 {
                    agent.angle -= config.rotation_angle;
                } else {
                    agent.angle += config.rotation_angle;
                }
            } else if weight_left > weight_right {
                agent.angle -= config.rotation_angle;
            } else if weight_right > weight_left {
                agent.angle += config.rotation_angle;
            }

            // Move
            agent.x += agent.angle.cos() * config.step_size;
            agent.y += agent.angle.sin() * config.step_size;

            // Clamp bounds and bounce
            if agent.x < 0.0 {
                agent.x = 0.0;
                agent.angle += PI;
            } else if agent.x >= width as f32 {
                agent.x = width as f32 - 1.0;
                agent.angle += PI;
            }

            if agent.y < 0.0 {
                agent.y = 0.0;
                agent.angle += PI;
            } else if agent.y >= height as f32 {
                agent.y = height as f32 - 1.0;
                agent.angle += PI;
            }

            // Deposit
            let idx = (agent.y as u32 * width + agent.x as u32) as usize;
            if idx < trail_map.len() {
                trail_map[idx] += config.deposit_amount;
            }
        }

        // Diffuse and Decay
        for y in 0..height {
            for x in 0..width {
                let mut sum = 0.0;
                let mut count = 0;

                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                            sum += trail_map[(ny as u32 * width + nx as u32) as usize];
                            count += 1;
                        }
                    }
                }

                let blur = sum / count as f32;
                self.back_buffer[(y * width + x) as usize] = blur * config.decay_rate;
            }
        }

        std::mem::swap(&mut self.trail_map, &mut self.back_buffer);
    }
}

/// Helper function to map the simulation state to a Framebuffer.
pub fn render_physarum(sim: &PhysarumSim, fb: &mut Framebuffer) {
    let width = fb.width().min(sim.width);
    let height = fb.height().min(sim.height);

    for y in 0..height {
        for x in 0..width {
            let trail = sim.get_trail(x, y);
            // Map trail intensity to a color (e.g., neon green)
            // Clamp intensity to 255
            let intensity = (trail * 255.0).min(255.0) as u32;
            let color = 0xFF00_0000 | (0 << 16) | (intensity << 8) | (intensity / 2);
            fb.set_pixel(x as i32, y as i32, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_moves_and_deposits() {
        let mut sim = PhysarumSim::new(10, 10);
        sim.add_agent(5.0, 5.0, 0.0); // angle 0.0 means moving +x
        sim.step();

        let val = sim.get_trail(6, 5);
        assert!(
            val > 0.0,
            "Agent should have moved right and deposited trail"
        );
    }

    #[test]
    fn test_render_physarum() {
        let mut sim = PhysarumSim::new(10, 10);
        sim.add_agent(5.0, 5.0, 0.0);
        sim.step();

        let mut fb = Framebuffer::new(10, 10).unwrap();
        render_physarum(&sim, &mut fb);

        // Check that at least one pixel is non-black
        let mut has_color = false;
        for y in 0..10 {
            for x in 0..10 {
                if fb.get_pixel(x as i32, y as i32) != Some(0xFF00_0000) {
                    has_color = true;
                    break;
                }
            }
        }
        assert!(has_color, "Framebuffer should have rendered trail colors");
    }
}
