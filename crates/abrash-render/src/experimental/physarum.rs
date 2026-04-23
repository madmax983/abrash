//! Physarum (Slime Mold) Simulation
//!
//! A generative particle-based simulation of Physarum polycephalum, where independent
//! agents move around depositing pheromones into a shared trail map. They use sensors
//! to steer towards higher concentrations of the trail map.

#![cfg(feature = "nova")]

use abrash_core::framebuffer::Framebuffer;
use abrash_core::random::Rng;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Physarum simulation.
#[derive(Clone, Debug)]
pub struct PhysarumConfig {
    /// Sensor offset distance.
    pub sensor_distance: f32,
    /// Sensor angle in radians.
    pub sensor_angle: f32,
    /// Turn speed in radians.
    pub turn_speed: f32,
    /// Movement speed.
    pub move_speed: f32,
    /// How fast the trail evaporates (0.0 to 1.0).
    pub evaporation_rate: f32,
    /// How fast the trail diffuses (0.0 to 1.0).
    pub diffuse_rate: f32,
    /// Size of the sensor.
    pub sensor_size: i32,
}

impl Default for PhysarumConfig {
    fn default() -> Self {
        Self {
            sensor_distance: 9.0,
            sensor_angle: 0.392, // ~22.5 degrees
            turn_speed: 0.785,   // ~45 degrees
            move_speed: 1.0,
            evaporation_rate: 0.01,
            diffuse_rate: 0.1,
            sensor_size: 1,
        }
    }
}

/// A single Physarum agent.
#[derive(Clone, Debug, PartialEq)]
pub struct PhysarumAgent {
    /// Position (X coordinate)
    pub x: f32,
    /// Position (Y coordinate)
    pub y: f32,
    /// Heading angle in radians
    pub angle: f32,
}

impl PhysarumAgent {
    /// Creates a new `PhysarumAgent`.
    #[must_use]
    pub const fn new(x: f32, y: f32, angle: f32) -> Self {
        Self { x, y, angle }
    }
}

/// A fast, localized RNG for updating agents without locks.
#[derive(Clone)]
struct AgentRng {
    state: u32,
}

impl AgentRng {
    fn next_f32(&mut self) -> f32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        (self.state as f32) / (std::u32::MAX as f32)
    }
}

/// Reads a value from the trail map with a specified sensor size and position, wrapping edges.
fn sense(
    trail_map: &[f32],
    width: usize,
    height: usize,
    agent: &PhysarumAgent,
    config: &PhysarumConfig,
    angle_offset: f32,
) -> f32 {
    let sensor_angle = agent.angle + angle_offset;
    let sensor_dir_x = sensor_angle.cos();
    let sensor_dir_y = sensor_angle.sin();

    let center_x = agent.x + sensor_dir_x * config.sensor_distance;
    let center_y = agent.y + sensor_dir_y * config.sensor_distance;

    let mut sum = 0.0;

    // Bounds check optimization (Nova)
    // Avoid expensive modulo if we're well within the map.
    // Otherwise use modulo to wrap around.

    for offset_y in -config.sensor_size..=config.sensor_size {
        for offset_x in -config.sensor_size..=config.sensor_size {
            let sample_x = (center_x as i32 + offset_x).rem_euclid(width as i32) as usize;
            let sample_y = (center_y as i32 + offset_y).rem_euclid(height as i32) as usize;

            let index = sample_y * width + sample_x;
            sum += trail_map[index];
        }
    }

    sum
}

/// Updates the agents and writes their positions to the trail map.
/// Trail Map is passed in decoupled from `agents` to satisfy the borrow checker.
pub fn update_agents(
    agents: &mut [PhysarumAgent],
    trail_map: &mut [f32],
    width: usize,
    height: usize,
    config: &PhysarumConfig,
    base_seed: u32,
) {
    if width == 0 || height == 0 || trail_map.is_empty() || agents.is_empty() {
        return;
    }

    // Agent update is tricky to parallelize with write-conflicts to the trail_map,
    // so we evaluate their movement (which reads trail_map) and position updates.
    // In a fully parallelized version we would atomic ADD to the trail map or use dual buffers.
    // For CPU rendering we'll run this sequentially for now to avoid locks or atomics on f32.

    for (i, agent) in agents.iter_mut().enumerate() {
        // Fast local RNG for randomness in wandering
        let mut rng = AgentRng {
            state: base_seed.wrapping_add(i as u32).max(1),
        };
        let random_steer_strength = rng.next_f32();

        let weight_forward = sense(trail_map, width, height, agent, config, 0.0);
        let weight_left = sense(trail_map, width, height, agent, config, config.sensor_angle);
        let weight_right = sense(
            trail_map,
            width,
            height,
            agent,
            config,
            -config.sensor_angle,
        );

        // Steer based on sensor readings
        if weight_forward > weight_left && weight_forward > weight_right {
            // Continue straight
        } else if weight_forward < weight_left && weight_forward < weight_right {
            // Turn randomly left or right
            if random_steer_strength > 0.5 {
                agent.angle += config.turn_speed * random_steer_strength;
            } else {
                agent.angle -= config.turn_speed * random_steer_strength;
            }
        } else if weight_left > weight_right {
            // Turn left
            agent.angle += config.turn_speed * random_steer_strength;
        } else if weight_right > weight_left {
            // Turn right
            agent.angle -= config.turn_speed * random_steer_strength;
        }

        // Move
        let direction_x = agent.angle.cos();
        let direction_y = agent.angle.sin();
        agent.x += direction_x * config.move_speed;
        agent.y += direction_y * config.move_speed;

        // Clamp to map boundaries
        if agent.x < 0.0 || agent.x >= width as f32 || agent.y < 0.0 || agent.y >= height as f32 {
            // Bounce off wall and pick a random new direction inward
            agent.x = agent.x.clamp(0.01, width as f32 - 1.01);
            agent.y = agent.y.clamp(0.01, height as f32 - 1.01);
            agent.angle = rng.next_f32() * std::f32::consts::TAU;
        }

        // Deposit pheromone
        let px = agent.x as usize;
        let py = agent.y as usize;
        let index = py * width + px;
        if index < trail_map.len() {
            trail_map[index] += 1.0;
        }
    }
}

/// Applies a Box Blur and Evaporation pass to the trail map.
pub fn diffuse_and_evaporate(
    trail_map: &[f32],
    next_trail_map: &mut [f32],
    width: usize,
    height: usize,
    config: &PhysarumConfig,
) {
    if width == 0 || height == 0 {
        return;
    }

    let diffuse_rate = config.diffuse_rate;
    let evaporation_rate = config.evaporation_rate;
    let blur_factor = 1.0 / 9.0;

    #[cfg(feature = "parallel")]
    let iter = next_trail_map.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let iter = next_trail_map.chunks_exact_mut(width).enumerate();

    iter.for_each(|(y, row)| {
        for (x, cell) in row.iter_mut().enumerate() {
            let mut sum = 0.0;

            // 3x3 Box blur
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let sample_x = (x as i32 + dx).clamp(0, width as i32 - 1) as usize;
                    let sample_y = (y as i32 + dy).clamp(0, height as i32 - 1) as usize;
                    let index = sample_y * width + sample_x;
                    sum += trail_map[index];
                }
            }

            let blurred_val = sum * blur_factor;
            let current_val = trail_map[y * width + x];

            // Lerp between current value and blurred value based on diffuse rate
            let diffused_val = current_val + (blurred_val - current_val) * diffuse_rate;

            // Evaporate
            *cell = (diffused_val - evaporation_rate).max(0.0);
        }
    });
}

/// Renders the trail map to the framebuffer using a color gradient.
pub fn render_physarum(trail_map: &[f32], fb: &mut Framebuffer, color_a: u32, color_b: u32) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || trail_map.len() != width * height {
        return;
    }

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let iter = pixels.par_iter_mut().zip(trail_map.par_iter());
    #[cfg(not(feature = "parallel"))]
    let iter = pixels.iter_mut().zip(trail_map.iter());

    let r_a = ((color_a >> 16) & 0xFF) as f32;
    let g_a = ((color_a >> 8) & 0xFF) as f32;
    let b_a = (color_a & 0xFF) as f32;

    let r_b = ((color_b >> 16) & 0xFF) as f32;
    let g_b = ((color_b >> 8) & 0xFF) as f32;
    let b_b = (color_b & 0xFF) as f32;

    iter.for_each(|(pixel, &trail_val)| {
        let t = trail_val.clamp(0.0, 1.0);

        let r = (r_a + (r_b - r_a) * t) as u32;
        let g = (g_a + (g_b - g_a) * t) as u32;
        let b = (b_a + (b_b - b_a) * t) as u32;

        *pixel = 0xFF00_0000 | (r << 16) | (g << 8) | b;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_physarum_agent_init() {
        let agent = PhysarumAgent::new(10.0, 20.0, std::f32::consts::PI);
        assert_eq!(agent.x, 10.0);
        assert_eq!(agent.y, 20.0);
        assert_eq!(agent.angle, std::f32::consts::PI);
    }

    #[test]
    fn test_agent_update_deposits_pheromone() {
        let mut agents = vec![PhysarumAgent::new(5.0, 5.0, 0.0)];
        let mut trail_map = vec![0.0; 100];
        let config = PhysarumConfig::default();

        update_agents(&mut agents, &mut trail_map, 10, 10, &config, 42);

        // Should have moved and deposited
        let mut sum = 0.0;
        for val in trail_map.iter() {
            sum += *val;
        }
        assert!(sum > 0.0);
    }

    #[test]
    fn test_diffuse_evaporate() {
        let mut trail_map = vec![0.0; 100];
        let mut next_map = vec![0.0; 100];

        // Put a strong signal in the middle
        trail_map[55] = 10.0;

        let config = PhysarumConfig {
            diffuse_rate: 1.0, // max diffusion
            evaporation_rate: 0.0,
            ..Default::default()
        };

        diffuse_and_evaporate(&trail_map, &mut next_map, 10, 10, &config);

        // Ensure it spread
        assert!(next_map[55] < 10.0);
        assert!(next_map[56] > 0.0);
    }
}
