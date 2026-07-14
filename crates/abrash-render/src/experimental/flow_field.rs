//! Flow Field Post-Processing Filter
//!
//! A procedural post-processing effect that simulates particles flowing through a vector field.
//! This creates smooth, swirling patterns resembling fluid dynamics or magnetic fields.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::XorShift32;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Flow Field effect.
#[derive(Debug, Clone, Copy)]
pub struct FlowFieldConfig {
    /// The number of particles to simulate.
    pub particle_count: usize,
    /// The length of the particle trails (fade factor).
    pub trail_length: f32,
    /// The scale of the noise field (higher = smaller swirls).
    pub noise_scale: f32,
    /// The speed at which particles move.
    pub speed: f32,
    /// The color of the particles.
    pub color: u32,
    /// Current time for animating the noise field.
    pub time: f32,
}

impl Default for FlowFieldConfig {
    fn default() -> Self {
        Self {
            particle_count: 5000,
            trail_length: 0.9,
            noise_scale: 0.05,
            speed: 2.0,
            color: 0xFF_00_FF_FF, // Cyan
            time: 0.0,
        }
    }
}

#[derive(Clone, Copy)]
struct Particle {
    x: f32,
    y: f32,
}

thread_local! {
    static PARTICLES: RefCell<Vec<Particle>> = const { RefCell::new(Vec::new()) };
    static PREV_FRAMEBUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Applies a Flow Field effect to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to apply the effect to.
/// * `config` - The configuration parameters for the effect.
pub fn apply_flow_field(fb: &mut Framebuffer, config: &FlowFieldConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || config.particle_count == 0 {
        return;
    }

    let dest_pixels = fb.as_mut_slice();

    // 1. Fade previous frame to create trails
    PREV_FRAMEBUFFER.with(|prev_fb_cell| {
        let mut prev_fb = prev_fb_cell.borrow_mut();
        if prev_fb.len() != dest_pixels.len() {
            prev_fb.resize(dest_pixels.len(), 0xFF_00_00_00);
        }

        // Apply fade
        let fade = (config.trail_length.clamp(0.0, 1.0) * 255.0) as u32;

        #[cfg(feature = "parallel")]
        let iter = dest_pixels.par_iter_mut().zip(prev_fb.par_iter_mut());
        #[cfg(not(feature = "parallel"))]
        let iter = dest_pixels.iter_mut().zip(prev_fb.iter_mut());

        iter.for_each(|(dst, src)| {
            let r = (((*src >> 16) & 0xFF) * fade) / 255;
            let g = (((*src >> 8) & 0xFF) * fade) / 255;
            let b = ((*src & 0xFF) * fade) / 255;

            let faded = 0xFF00_0000 | (r << 16) | (g << 8) | b;
            *src = faded;
            *dst = faded; // Write faded background to current frame
        });
    });

    // 2. Update and draw particles
    PARTICLES.with(|particles_cell| {
        let mut particles = particles_cell.borrow_mut();

        // Initialize if empty or size changed
        if particles.len() != config.particle_count {
            particles.clear();
            let mut rng = XorShift32::new(0x1337_BEEF);
            for _ in 0..config.particle_count {
                particles.push(Particle {
                    x: (rng.next_u32() % width as u32) as f32,
                    y: (rng.next_u32() % height as u32) as f32,
                });
            }
        }

        // We process particles sequentially since they write to arbitrary locations
        for p in particles.iter_mut() {
            // Simple pseudo-random vector field based on coordinates and time
            // Using sine/cosine of coordinates creates repeating swirl patterns
            let angle =
                ((p.x * config.noise_scale).sin() + (p.y * config.noise_scale).cos() + config.time)
                    * std::f32::consts::TAU;

            p.x += angle.cos() * config.speed;
            p.y += angle.sin() * config.speed;

            // Wrap around screen edges
            if p.x < 0.0 {
                p.x += width as f32;
            }
            if p.x >= width as f32 {
                p.x -= width as f32;
            }
            if p.y < 0.0 {
                p.y += height as f32;
            }
            if p.y >= height as f32 {
                p.y -= height as f32;
            }

            // Draw particle
            let px = p.x as usize;
            let py = p.y as usize;
            if px < width && py < height {
                let idx = py * width + px;
                dest_pixels[idx] = config.color;
            }
        }

        // Save state for next frame's trails
        PREV_FRAMEBUFFER.with(|prev_fb_cell| {
            let mut prev_fb = prev_fb_cell.borrow_mut();
            prev_fb.copy_from_slice(dest_pixels);
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_flow_field_updates_buffer() {
        let width = 100;
        let height = 100;
        let mut fb = Framebuffer::new(width, height).unwrap();
        fb.clear(0xFF_00_00_00);

        let mut fb_clone = Framebuffer::new(width, height).unwrap();
        fb_clone.clear(0xFF_00_00_00);

        let config = FlowFieldConfig::default();
        apply_flow_field(&mut fb, &config);

        let mut different = false;
        for i in 0..(width * height) as usize {
            if fb.as_slice()[i] != fb_clone.as_slice()[i] {
                different = true;
                break;
            }
        }
        assert!(different, "Flow field should draw particles to the buffer");
    }
}
