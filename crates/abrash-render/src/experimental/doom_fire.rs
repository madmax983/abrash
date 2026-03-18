//! Doom Fire Effect
//!
//! A procedural fire effect that simulates the classic Doom fire cellular automata algorithm.
//! Heat propagates upwards with random cooling and horizontal drift, creating a fiery appearance.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::XorShift32;

/// The classic Doom fire color palette mapped from heat indices 0 to 36.
const FIRE_PALETTE: [u32; 37] = [
    0xFF_070707, 0xFF_1F0707, 0xFF_2F0F07, 0xFF_470F07, 0xFF_571707, 0xFF_671F07, 0xFF_771F07,
    0xFF_8F2707, 0xFF_9F2F07, 0xFF_AF3F07, 0xFF_BF4707, 0xFF_C74707, 0xFF_DF4F07, 0xFF_DF5707,
    0xFF_DF5707, 0xFF_D75F07, 0xFF_D7670F, 0xFF_CF6F0F, 0xFF_CF770F, 0xFF_CF7F0F, 0xFF_CF8717,
    0xFF_C78717, 0xFF_C78F17, 0xFF_C7971F, 0xFF_BF9F1F, 0xFF_BF9F1F, 0xFF_BFA727, 0xFF_BFAF2F,
    0xFF_B7AF2F, 0xFF_B7B72F, 0xFF_B7B737, 0xFF_CFCF6F, 0xFF_DFDF9F, 0xFF_EFEFC7, 0xFF_FFFFFF,
    0xFF_FFFFFF, 0xFF_FFFFFF,
];

/// A procedural fire effect simulator.
#[derive(Debug, Clone)]
pub struct DoomFire {
    /// The heat buffer, stored row-by-row (width * height).
    pub heat_buffer: Vec<u8>,
    pub width: usize,
    pub height: usize,
    rng: XorShift32,
}

impl DoomFire {
    /// Creates a new DoomFire instance with the given dimensions.
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        let size = width * height;
        let mut heat_buffer = vec![0; size];

        // Ignite the bottom row (heat index 36)
        if height > 0 {
            let start = (height - 1) * width;
            for i in 0..width {
                heat_buffer[start + i] = 36;
            }
        }

        // Use a simple seeded PRNG. A thread_local or static could be used
        // to preserve state across frames, but re-seeding with something simple
        // or passing it in is better. For this retro effect, just instantiate one
        // with a generic seed or store it in the struct.
        // We will store it in the struct for proper stateful randomness.
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        let rng = XorShift32::new(seed);

        Self { heat_buffer, width, height, rng }
    }

    /// Updates the fire effect for one frame.
    ///
    /// This propagates heat from the bottom upwards. Heat cools randomly
    /// and drifts horizontally simulating wind.
    pub fn update(&mut self) {
        if self.height < 2 {
            return;
        }

        for x in 0..self.width {
            for y in 1..self.height {
                let src_idx = y * self.width + x;
                let heat = self.heat_buffer[src_idx];

                if heat == 0 {
                    let dst_idx = (y - 1) * self.width + x;
                    self.heat_buffer[dst_idx] = 0;
                } else {
                    // Random decay and horizontal drift
                    let r: usize = (self.rng.next_u32() % 4) as usize;

                    // Destination is one row up, drifted left randomly by r
                    let dst_x = x.saturating_sub(r) % self.width;
                    let dst_idx = (y - 1) * self.width + dst_x;

                    // Heat decays depending on random value
                    // The bitwise AND handles the case of subtraction cleanly without panic
                    self.heat_buffer[dst_idx] = heat.saturating_sub((r & 1) as u8);
                }
            }
        }
    }

    /// Renders the current heat state onto the given framebuffer.
    ///
    /// The fire is drawn starting at the top-left coordinate `(offset_x, offset_y)`.
    pub fn draw(&self, fb: &mut Framebuffer, offset_x: i32, offset_y: i32) {
        let fb_width = fb.width() as usize;
        let fb_height = fb.height() as usize;
        let pixels = fb.as_mut_slice();

        // Use par_chunks_mut if parallel feature enabled later, but keep it simple for now
        // since we just map array -> array
        for y in 0..self.height {
            let screen_y = offset_y + y as i32;
            if screen_y < 0 || screen_y >= fb_height as i32 {
                continue;
            }

            for x in 0..self.width {
                let screen_x = offset_x + x as i32;
                if screen_x < 0 || screen_x >= fb_width as i32 {
                    continue;
                }

                let heat_idx = self.heat_buffer[y * self.width + x] as usize;
                // Get color from palette, ensuring bounds
                let color = FIRE_PALETTE[heat_idx.min(36)];

                // We don't want to draw the background 0xFF_070707 over everything
                // if we are overlaying it. But for the true Doom effect, it paints black.
                // If it's pure black in our palette (0xFF_070707), we could skip or blend.
                // For this retro effect, we just overwrite.

                // If you want to overlay it transparently:
                if color != 0xFF_070707 {
                     let idx = (screen_y as usize) * fb_width + (screen_x as usize);
                     pixels[idx] = color;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doom_fire_init() {
        let fire = DoomFire::new(10, 10);
        assert_eq!(fire.width, 10);
        assert_eq!(fire.height, 10);
        assert_eq!(fire.heat_buffer.len(), 100);

        // Check top row is cold
        assert_eq!(fire.heat_buffer[0], 0);

        // Check bottom row is hot
        for i in 0..10 {
            assert_eq!(fire.heat_buffer[90 + i], 36);
        }
    }

    #[test]
    fn test_doom_fire_update_propagates_heat() {
        let mut fire = DoomFire::new(5, 5);

        // Initially, row 3 (second from bottom) should be cold
        assert_eq!(fire.heat_buffer[3 * 5 + 2], 0);

        // One update propagates heat from row 4 to row 3
        fire.update();

        // Check if some heat reached row 3
        let mut heat_propagated = false;
        for i in 0..5 {
            if fire.heat_buffer[3 * 5 + i] > 0 {
                heat_propagated = true;
                break;
            }
        }

        assert!(heat_propagated, "Heat did not propagate upwards");

        // The bottom row must stay hot
        for i in 0..5 {
             assert_eq!(fire.heat_buffer[4 * 5 + i], 36, "Bottom row should remain ignited");
        }
    }

    #[test]
    fn test_doom_fire_draw_bounds() {
        let mut fire = DoomFire::new(10, 10);
        // Force a specific pixel to be hot
        fire.heat_buffer[55] = 36; // At (5, 5)

        // Tiny framebuffer
        let mut fb = Framebuffer::new(5, 5).unwrap();

        // Draw with large offset, should not panic
        fire.draw(&mut fb, -20, -20);
        fire.draw(&mut fb, 100, 100);
    }
}
