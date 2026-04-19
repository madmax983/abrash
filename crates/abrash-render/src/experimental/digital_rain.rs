//! Digital Rain Filter (Matrix style).
//!
//! A retro effect that simulates the classic scrolling digital rain.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::XorShift32;

/// State for the Digital Rain effect
pub struct DigitalRain {
    /// Tracks the Y position of the "head" of the drop for each column
    drops: Vec<f32>,
    /// RNG for varying speeds and lengths
    rng: XorShift32,
    /// Width of the target framebuffer (to know when to resize)
    cached_width: u32,
    cached_height: u32,
}

impl Default for DigitalRain {
    fn default() -> Self {
        Self {
            drops: Vec::new(),
            rng: XorShift32::new(0xDEAD_BEEF),
            cached_width: 0,
            cached_height: 0,
        }
    }
}

impl DigitalRain {
    /// Creates a new instance of the digital rain effect
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Renders the digital rain onto the given framebuffer
    ///
    /// * `fb`: Framebuffer to render to
    /// * `time`: Delta time or elapsed time to move the rain
    pub fn apply(&mut self, fb: &mut Framebuffer, dt: f32) {
        let width = fb.width();
        let height = fb.height();

        if width == 0 || height == 0 {
            return;
        }

        // Initialize or resize the drops array if framebuffer size changes
        // Using a "cell" size of roughly 8 pixels wide
        let cell_width = 8;
        let columns = (width as f32 / cell_width as f32).ceil() as usize;

        if self.cached_width != width || self.cached_height != height {
            self.drops.clear();
            self.drops.resize(columns, 0.0);
            for d in &mut self.drops {
                // Randomize initial starting positions so they don't all fall at once
                *d = self.rng.next_f32() * -(height as f32);
            }
            self.cached_width = width;
            self.cached_height = height;
        }

        // Apply a dark fade to the entire framebuffer to create the "trails"
        // This is a fast path to dimming all pixels slightly
        let pixels = fb.as_mut_slice();
        for p in pixels.iter_mut() {
            let r = ((*p >> 16) & 0xFF) as u32;
            let g = ((*p >> 8) & 0xFF) as u32;
            let b = (*p & 0xFF) as u32;

            // Fade out by ~10% each frame
            let fade = 20;
            let new_r = r.saturating_sub(fade);
            let new_g = g.saturating_sub(fade);
            let new_b = b.saturating_sub(fade);

            *p = 0xFF00_0000 | (new_r << 16) | (new_g << 8) | new_b;
        }

        // Update drop positions and draw the "heads"
        for i in 0..columns {
            // Speed of this column (randomized slightly based on index so it's consistent)
            // Just use a pseudo-random value based on column index
            let speed_factor = 1.0 + ((i * 12345) % 10) as f32 * 0.2;
            let drop_speed = 300.0 * speed_factor;

            self.drops[i] += drop_speed * dt;

            // If a drop goes past the bottom, reset it to the top
            if self.drops[i] > height as f32 {
                self.drops[i] = self.rng.next_f32() * -50.0; // Start slightly above
            }

            let head_y = self.drops[i] as i32;
            let x_start = (i as u32 * cell_width) as i32;

            // Draw a block for the head (pure white/bright green)
            if head_y >= 0 && head_y < height as i32 {
                for y in head_y..(head_y + 8) {
                    if y >= height as i32 {
                        continue;
                    }
                    for x in x_start..(x_start + cell_width as i32) {
                        if x >= width as i32 {
                            continue;
                        }

                        // Head is white, trailing slightly to green
                        let color = if y == head_y + 7 {
                            0xFF_FF_FF_FF // White tip
                        } else {
                            0xFF_00_FF_00 // Bright green body
                        };

                        let idx = (y as u32 * width + x as u32) as usize;
                        pixels[idx] = color;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_digital_rain() {
        let mut fb = Framebuffer::new(32, 32).unwrap();
        fb.clear(0xFF_00_00_00);

        let mut rain = DigitalRain::new();
        // First frame
        rain.apply(&mut fb, 0.1);

        let mut has_non_black = false;
        for &p in fb.as_slice() {
            if p != 0xFF_00_00_00 {
                has_non_black = true;
                break;
            }
        }
        assert!(has_non_black, "Framebuffer should be modified");

        // Test fading
        rain.apply(&mut fb, 0.1);
    }
}
