//! Physarum (Slime Mold) Simulation
//!
//! A procedural simulation of Physarum polycephalum (slime mold) behavior.
//! Agents move around the screen, depositing a trail. They sense the trail
//! ahead of them and steer towards stronger trails, creating complex
//! self-organizing networks and structures.

#![cfg(feature = "nova")]

use crate::framebuffer::Framebuffer;
use abrash_core::color::Color;
use abrash_core::math::Vec2;
use abrash_core::random::Rng;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration parameters for the Physarum simulation.
#[derive(Clone, Debug)]
pub struct PhysarumConfig {
    /// Number of agents in the simulation.
    pub agent_count: usize,
    /// Agent movement speed per step.
    pub move_speed: f32,
    /// Agent turn speed (in radians) when steering.
    pub turn_speed: f32,
    /// Distance ahead the agent looks to sense trails.
    pub sensor_offset_dist: f32,
    /// Angle offset (in radians) for the left/right sensors.
    pub sensor_angle: f32,
    /// Size of the sensor box (0 means just 1 pixel).
    pub sensor_size: i32,
    /// Pheromone decay factor (amount subtracted per step).
    pub decay_rate: f32,
    /// Pheromone diffuse matrix scale (amount spread to neighbors).
    pub diffuse_rate: f32,
    /// Trail color deposited by agents (ARGB).
    pub trail_color: u32,
}

impl Default for PhysarumConfig {
    fn default() -> Self {
        Self {
            agent_count: 50000,
            move_speed: 1.0,
            turn_speed: 0.2,
            sensor_offset_dist: 9.0,
            sensor_angle: 0.4,
            sensor_size: 1,
            decay_rate: 0.05,
            diffuse_rate: 0.2,
            trail_color: 0xFF_00FF00, // Green slime
        }
    }
}

/// A single slime mold agent.
#[derive(Clone, Debug)]
pub struct Agent {
    /// Current position (x, y).
    pub position: Vec2,
    /// Current heading angle in radians.
    pub angle: f32,
}

thread_local! {
    /// Agents in the simulation.
    static AGENTS: RefCell<Vec<Agent>> = const { RefCell::new(Vec::new()) };
    /// Pheromone trail map (stores strength 0.0 to 1.0).
    static TRAIL_MAP: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
    /// Double buffer for the diffusion step.
    static TRAIL_MAP_NEXT: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
}

/// Applies one step of the Physarum simulation and renders it to the framebuffer.
///
/// # Panics
///
/// Panics if the system time is set before the UNIX EPOCH.
pub fn apply_physarum(fb: &mut Framebuffer, config: &PhysarumConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let grid_size = width * height;

    if width == 0 || height == 0 {
        return;
    }

    AGENTS.with(|agents| {
        TRAIL_MAP.with(|trail_map| {
            TRAIL_MAP_NEXT.with(|trail_map_next| {
                let mut agents_borrow = agents.borrow_mut();
                let mut trail_borrow = trail_map.borrow_mut();
                let mut trail_next_borrow = trail_map_next.borrow_mut();

                // Initialize if needed
                if trail_borrow.len() != grid_size {
                    trail_borrow.clear();
                    trail_borrow.resize(grid_size, 0.0);
                    trail_next_borrow.clear();
                    trail_next_borrow.resize(grid_size, 0.0);
                }

                if agents_borrow.len() != config.agent_count {
                    agents_borrow.clear();
                    let mut rng = Rng::seeded(12345);
                    let center_x = width as f32 / 2.0;
                    let center_y = height as f32 / 2.0;

                    for _ in 0..config.agent_count {
                        // Start agents randomly in a circle
                        let radius = rng.f32() * (width.min(height) as f32) * 0.4;
                        let angle = rng.f32() * std::f32::consts::TAU;

                        agents_borrow.push(Agent {
                            position: Vec2::new(
                                center_x + angle.cos() * radius,
                                center_y + angle.sin() * radius,
                            ),
                            // Point inwards or randomly
                            angle: angle + std::f32::consts::PI,
                        });
                    }
                }

                // Step 1: Update agents and deposit pheromones
                let mut rng = Rng::seeded(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as u64,
                );

                let width_i32 = width as i32;
                let height_i32 = height as i32;
                let sensor_dist = config.sensor_offset_dist;
                let sensor_size = config.sensor_size;

                for agent in agents_borrow.iter_mut() {
                    let sense = |angle_offset: f32, trail_slice: &[f32]| -> f32 {
                        let sensor_angle = agent.angle + angle_offset;
                        let (sin_a, cos_a) = sensor_angle.sin_cos();
                        let sensor_pos_x = agent.position.x + cos_a * sensor_dist;
                        let sensor_pos_y = agent.position.y + sin_a * sensor_dist;

                        let sx = sensor_pos_x as i32;
                        let sy = sensor_pos_y as i32;

                        let mut sum = 0.0;
                        for dy in -sensor_size..=sensor_size {
                            let mut ny = sy + dy;

                            // Optimize wrapping logic
                            if (ny as u32) >= height as u32 {
                                if ny < 0 {
                                    ny += height_i32;
                                } else {
                                    ny -= height_i32;
                                }
                            }

                            let row_idx = ny as usize * width;

                            for dx in -sensor_size..=sensor_size {
                                let mut nx = sx + dx;

                                if (nx as u32) >= width as u32 {
                                    if nx < 0 {
                                        nx += width_i32;
                                    } else {
                                        nx -= width_i32;
                                    }
                                }

                                sum += trail_slice[row_idx + nx as usize];
                            }
                        }
                        sum
                    };

                    let weight_forward = sense(0.0, &trail_borrow);
                    let weight_left = sense(config.sensor_angle, &trail_borrow);
                    let weight_right = sense(-config.sensor_angle, &trail_borrow);

                    let random_steer_strength = rng.f32();

                    // Steer based on sensory data
                    if weight_forward > weight_left && weight_forward > weight_right {
                        // Keep going straight
                    } else if weight_forward < weight_left && weight_forward < weight_right {
                        // Both sides are strong, turn randomly
                        if random_steer_strength > 0.5 {
                            agent.angle += config.turn_speed;
                        } else {
                            agent.angle -= config.turn_speed;
                        }
                    } else if weight_right > weight_left {
                        // Turn right
                        agent.angle -= config.turn_speed * random_steer_strength;
                    } else if weight_left > weight_right {
                        // Turn left
                        agent.angle += config.turn_speed * random_steer_strength;
                    }

                    // Move agent
                    let direction = Vec2::new(agent.angle.cos(), agent.angle.sin());
                    agent.position.x += direction.x * config.move_speed;
                    agent.position.y += direction.y * config.move_speed;

                    // Wrap position around screen (using if/else since range is known)
                    if agent.position.x < 0.0 {
                        agent.position.x += width as f32;
                    } else if agent.position.x >= width as f32 {
                        agent.position.x -= width as f32;
                    }

                    if agent.position.y < 0.0 {
                        agent.position.y += height as f32;
                    } else if agent.position.y >= height as f32 {
                        agent.position.y -= height as f32;
                    }

                    // Deposit pheromone
                    let px = agent.position.x as usize;
                    let py = agent.position.y as usize;

                    if px < width && py < height {
                        trail_borrow[py * width + px] = 1.0;
                    }
                }

                // Step 2: Diffuse and decay
                let diffuse_center = 1.0 - config.diffuse_rate;
                let diffuse_side = config.diffuse_rate / 8.0;

                for y in 0..height {
                    let y_prev = if y == 0 { height - 1 } else { y - 1 };
                    let y_next = if y == height - 1 { 0 } else { y + 1 };

                    let row_idx = y * width;
                    let row_prev_idx = y_prev * width;
                    let row_next_idx = y_next * width;

                    for x in 0..width {
                        let x_prev = if x == 0 { width - 1 } else { x - 1 };
                        let x_next = if x == width - 1 { 0 } else { x + 1 };

                        let sum =
                            // Top row
                            trail_borrow[row_prev_idx + x_prev] * diffuse_side +
                            trail_borrow[row_prev_idx + x] * diffuse_side +
                            trail_borrow[row_prev_idx + x_next] * diffuse_side +
                            // Middle row
                            trail_borrow[row_idx + x_prev] * diffuse_side +
                            trail_borrow[row_idx + x] * diffuse_center +
                            trail_borrow[row_idx + x_next] * diffuse_side +
                            // Bottom row
                            trail_borrow[row_next_idx + x_prev] * diffuse_side +
                            trail_borrow[row_next_idx + x] * diffuse_side +
                            trail_borrow[row_next_idx + x_next] * diffuse_side;

                        // Decay
                        let decayed = (sum - config.decay_rate).max(0.0);
                        trail_next_borrow[row_idx + x] = decayed;
                    }
                }

                // Swap buffers
                std::mem::swap(&mut *trail_borrow, &mut *trail_next_borrow);

                // Step 3: Render trail map to framebuffer
                let fb_slice = fb.as_mut_slice();
                for i in 0..grid_size {
                    let intensity = trail_borrow[i];
                    if intensity > 0.01 {
                        // Map intensity to alpha 0-255
                        let alpha = (intensity * 255.0).clamp(0.0, 255.0) as u32;
                        let color_with_alpha = (alpha << 24) | (config.trail_color & 0x00_FF_FF_FF);

                        let bg = Color::from_argb_u32(fb_slice[i]);
                        let fg = Color::from_argb_u32(color_with_alpha);

                        let blend = Color::blend_over(fg, bg);

                        fb_slice[i] = blend.to_argb_u32();
                    }
                }
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_apply_physarum_no_crash() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        let config = PhysarumConfig {
            agent_count: 1000,
            ..Default::default()
        };
        apply_physarum(&mut fb, &config);
    }
}
