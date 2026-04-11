#![cfg(feature = "nova")]

use abrash_core::framebuffer::Framebuffer;

const FIRE_WIDTH: usize = 320;
const FIRE_HEIGHT: usize = 168;

/// Classic DOOM Fire palette (37 colors)
const PALETTE: [u32; 37] = [
    0xFF070707, 0xFF1F0707, 0xFF2F0F07, 0xFF470F07, 0xFF571707, 0xFF671F07, 0xFF771F07, 0xFF8F2707,
    0xFF9F2F07, 0xFFAF3F07, 0xFFBF4707, 0xFFC74707, 0xFFDF4F07, 0xFFDF5707, 0xFFDF5707, 0xFFD75F07,
    0xFFD75F07, 0xFFD7670F, 0xFFCF6F0F, 0xFFCF770F, 0xFFCF7F0F, 0xFFCF8717, 0xFFC78717, 0xFFC78F17,
    0xFFC7971F, 0xFFBF9F1F, 0xFFBF9F1F, 0xFFBFA727, 0xFFBFA727, 0xFFBFAF2F, 0xFFB7AF2F, 0xFFB7B72F,
    0xFFB7B737, 0xFFCFCF6F, 0xFFDFDF9F, 0xFFEFEFC7, 0xFFFFFFFF,
];

pub struct DoomFire {
    /// 1D array of fire temperatures (0-36)
    pub fire_pixels: Vec<usize>,
    /// Framebuffer for output
    pub buffer: Framebuffer,
    /// Fast PRNG state
    seed: u32,
    /// Whether the fire should be seeded at the bottom row (ignition)
    pub active: bool,
}

impl Default for DoomFire {
    fn default() -> Self {
        Self::new()
    }
}

impl DoomFire {
    pub fn new() -> Self {
        let mut fire_pixels = vec![0; FIRE_WIDTH * FIRE_HEIGHT];

        let mut sim = Self {
            fire_pixels,
            buffer: Framebuffer::new(FIRE_WIDTH as u32, FIRE_HEIGHT as u32).unwrap(),
            seed: 12345,
            active: true,
        };

        sim.ignite();
        sim
    }

    /// Set the bottom row to the maximum heat
    pub fn ignite(&mut self) {
        let start = (FIRE_HEIGHT - 1) * FIRE_WIDTH;
        for i in 0..FIRE_WIDTH {
            self.fire_pixels[start + i] = 36;
        }
    }

    /// Stop feeding the fire at the bottom
    pub fn extinguish(&mut self) {
        let start = (FIRE_HEIGHT - 1) * FIRE_WIDTH;
        for i in 0..FIRE_WIDTH {
            self.fire_pixels[start + i] = 0;
        }
        self.active = false;
    }

    /// PCG-like fast inline PRNG
    #[inline]
    fn fast_rand(&mut self) -> u32 {
        self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
        self.seed >> 16
    }

    pub fn update(&mut self) {
        // Ensure bottom row remains hot if active
        if self.active {
            self.ignite();
        } else {
            self.extinguish();
        }

        // We update from bottom to top (excluding the very bottom row which is our source)
        for x in 0..FIRE_WIDTH {
            for y in 1..FIRE_HEIGHT {
                let src = y * FIRE_WIDTH + x;
                let pixel = self.fire_pixels[src];

                if pixel == 0 {
                    let dst = src - FIRE_WIDTH;
                    self.fire_pixels[dst] = 0;
                } else {
                    let rand_val = (self.fast_rand() & 3) as usize; // 0..3
                    let dst = src.saturating_sub(rand_val).saturating_sub(FIRE_WIDTH);
                    // The trick: decrement temperature occasionally
                    let new_heat = pixel.saturating_sub(rand_val & 1);
                    if dst < self.fire_pixels.len() {
                        self.fire_pixels[dst] = new_heat;
                    }
                }
            }
        }

        // Render to framebuffer
        self.render();
    }

    fn render(&mut self) {
        let fb_slice = self.buffer.as_mut_slice();
        for (i, &heat) in self.fire_pixels.iter().enumerate() {
            let heat = heat.min(36);
            fb_slice[i] = PALETTE[heat];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialization() {
        let fire = DoomFire::new();
        assert_eq!(fire.fire_pixels.len(), FIRE_WIDTH * FIRE_HEIGHT);
        assert!(fire.active);

        // Check bottom row is ignited
        let start = (FIRE_HEIGHT - 1) * FIRE_WIDTH;
        for i in 0..FIRE_WIDTH {
            assert_eq!(fire.fire_pixels[start + i], 36);
        }
    }

    #[test]
    fn test_extinguish() {
        let mut fire = DoomFire::new();
        fire.extinguish();
        assert!(!fire.active);

        let start = (FIRE_HEIGHT - 1) * FIRE_WIDTH;
        for i in 0..FIRE_WIDTH {
            assert_eq!(fire.fire_pixels[start + i], 0);
        }
    }

    #[test]
    fn test_update_propagates_fire() {
        let mut fire = DoomFire::new();
        // Clear all but bottom row
        for i in 0..((FIRE_HEIGHT - 1) * FIRE_WIDTH) {
            fire.fire_pixels[i] = 0;
        }

        fire.update();

        // Check row above bottom has some non-zero values due to propagation
        let row_above = (FIRE_HEIGHT - 2) * FIRE_WIDTH;
        let mut has_fire = false;
        for i in 0..FIRE_WIDTH {
            if fire.fire_pixels[row_above + i] > 0 {
                has_fire = true;
                break;
            }
        }
        assert!(has_fire, "Fire did not propagate upwards");
    }
}
