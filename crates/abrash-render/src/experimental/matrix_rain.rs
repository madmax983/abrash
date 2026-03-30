//! Matrix Digital Rain Post-Processing Effect
//!
//! A retro post-processing effect that overlays the classic "Digital Rain"
//! (falling green code) on top of the framebuffer.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::XorShift32;

#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::cell::RefCell;

thread_local! {
    // We store the state of each column (head Y position, speed, and character offset)
    // To support varying resolutions, we store a large enough buffer.
    static MATRIX_STATE: RefCell<Vec<ColumnState>> = const { RefCell::new(Vec::new()) };
}

#[derive(Clone, Copy)]
struct ColumnState {
    head_y: f32,
    speed: f32,
    char_offset: u32,
}

/// Configuration for the Matrix Digital Rain effect.
#[derive(Debug, Clone, Copy)]
pub struct MatrixRainConfig {
    /// Width of each falling column cell in pixels.
    pub cell_width: usize,
    /// Height of each falling column cell in pixels.
    pub cell_height: usize,
    /// Overall speed multiplier for the falling rain.
    pub speed: f32,
    /// The primary color of the text (typically bright green).
    pub color: u32,
    /// How fast the trail fades out (higher = shorter tails).
    pub fade_speed: f32,
    /// A continuously increasing time value used to animate the rain.
    pub time: f32,
}

impl Default for MatrixRainConfig {
    fn default() -> Self {
        Self {
            cell_width: 8,
            cell_height: 12,
            speed: 30.0,
            color: 0xFF_00FF41, // Classic Matrix Green
            fade_speed: 0.05,
            time: 0.0,
        }
    }
}

/// A minimal 8x12 font for a few distinct "digital" characters.
/// Each character is represented by 12 bytes, where the lower 8 bits of each byte
/// represent the pixels of a row.
const FONT: [[u8; 12]; 8] = [
    // Random Katakana/Digital shapes
    [
        0x00, 0x3E, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x3E, 0x00,
    ], // 'I'ish
    [
        0x00, 0x7E, 0x02, 0x02, 0x3E, 0x20, 0x20, 0x20, 0x20, 0x20, 0x7E, 0x00,
    ], // 'Z'ish
    [
        0x00, 0x42, 0x42, 0x42, 0x42, 0x7E, 0x42, 0x42, 0x42, 0x42, 0x42, 0x00,
    ], // 'H'ish
    [
        0x00, 0x3C, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x3C, 0x00,
    ], // 'O'ish
    [
        0x00, 0x7E, 0x42, 0x42, 0x42, 0x7E, 0x40, 0x40, 0x40, 0x40, 0x40, 0x00,
    ], // 'P'ish
    [
        0x00, 0x42, 0x42, 0x42, 0x42, 0x7E, 0x02, 0x02, 0x02, 0x02, 0x7E, 0x00,
    ], // 'S'ish
    [
        0x00, 0x7E, 0x40, 0x40, 0x40, 0x7E, 0x02, 0x02, 0x02, 0x02, 0x7E, 0x00,
    ], // '5'ish
    [
        0x00, 0x42, 0x42, 0x42, 0x42, 0x3C, 0x08, 0x08, 0x08, 0x08, 0x3E, 0x00,
    ], // 'Y'ish
];

/// Applies a Matrix Digital Rain effect to the framebuffer.
///
/// This effect overlays falling characters in discrete columns.
/// The characters leave a trailing fade effect, and the head of the column
/// is drawn brightest (often white in the actual movie, but we'll stick to bright green/white mix here).
///
/// # Arguments
///
/// * `fb` - The framebuffer to apply the effect to.
/// * `config` - The configuration parameters for the effect.
pub fn apply_matrix_rain(fb: &mut Framebuffer, config: &MatrixRainConfig) {
    if config.cell_width == 0 || config.cell_height == 0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let cw = config.cell_width;
    let ch = config.cell_height;
    let cols = (width + cw - 1) / cw;
    let rows = (height + ch - 1) / ch;

    // First, dim the existing framebuffer slightly to create the fade effect
    // We do this manually to simulate the phosphor trail fading
    let pixels = fb.as_mut_slice();

    // Convert fade_speed (0.0 to 1.0) to an integer multiplier for the channels
    // 1.0 means instant fade (multiply by 0), 0.0 means no fade (multiply by 255)
    let fade_mult = ((1.0 - config.fade_speed).clamp(0.0, 1.0) * 255.0) as u32;

    #[cfg(feature = "parallel")]
    let pixel_iter = pixels.par_iter_mut();
    #[cfg(not(feature = "parallel"))]
    let pixel_iter = pixels.iter_mut();

    pixel_iter.for_each(|pixel| {
        let p = *pixel;
        let a = (p >> 24) & 0xFF;
        let r = (((p >> 16) & 0xFF) * fade_mult) >> 8;
        let g = (((p >> 8) & 0xFF) * fade_mult) >> 8;
        let b = ((p & 0xFF) * fade_mult) >> 8;
        *pixel = (a << 24) | (r << 16) | (g << 8) | b;
    });

    // Now update and draw the falling columns
    MATRIX_STATE.with(|state_ref| {
        let mut state = state_ref.borrow_mut();

        // Ensure we have enough columns for the current resolution
        if state.len() < cols {
            let mut prng = XorShift32::new(0x1337_C0DE);
            let needed = cols - state.len();
            for _ in 0..needed {
                state.push(ColumnState {
                    // Randomize initial positions so they don't all fall together
                    head_y: -((prng.next_u32() % (rows as u32 * 2)) as f32),
                    // Randomize speeds
                    speed: 0.5 + (prng.next_u32() % 100) as f32 / 100.0,
                    // Randomize initial char offset
                    char_offset: prng.next_u32(),
                });
            }
        }

        // We process each column independently. Since columns don't overlap horizontally,
        // we could parallelize this by chunking the framebuffer vertically or by columns.
        // For simplicity and safety with mutable slices, we'll iterate columns sequentially
        // but draw them fast.

        // Time delta simulation (assuming config.time advances per frame)
        // We'll just use a small constant delta for simplicity, scaled by speed
        let dt = 0.016 * config.speed;

        for cx in 0..cols {
            let col = &mut state[cx];

            // Move the head down
            col.head_y += col.speed * dt;

            // If the head falls off the bottom (plus some margin), reset it to the top
            if col.head_y > (rows + 10) as f32 {
                let mut prng =
                    XorShift32::new((config.time * 1000.0) as u32 ^ cx as u32 ^ 0xDEAD_BEEF);
                col.head_y = -((prng.next_u32() % 20) as f32);
                col.speed = 0.5 + (prng.next_u32() % 100) as f32 / 100.0;
                col.char_offset = prng.next_u32();
            }

            let head_row = col.head_y as i32;

            // Draw the head character if it's on screen
            if head_row >= 0 && head_row < rows as i32 {
                let char_idx =
                    ((head_row as u32).wrapping_add(col.char_offset) / 4) as usize % FONT.len();
                let char_bitmap = &FONT[char_idx];

                let start_x = cx * cw;
                let start_y = head_row as usize * ch;

                // Head character is often white or very bright
                let head_color = 0xFF_FFFFFF;

                for dy in 0..ch {
                    let py = start_y + dy;
                    if py >= height {
                        break;
                    }

                    let bitmap_y = (dy * 12) / ch; // Scale Y to 0-11
                    let row_bits = char_bitmap[bitmap_y];

                    for dx in 0..cw {
                        let px = start_x + dx;
                        if px >= width {
                            break;
                        }

                        let bitmap_x = (dx * 8) / cw; // Scale X to 0-7
                        let mask = 1 << (7 - bitmap_x);

                        if (row_bits & mask) != 0 {
                            pixels[py * width + px] = head_color;
                        }
                    }
                }
            }

            // Draw a few trailing characters to "refresh" the trail behind the head
            for i in 1..4 {
                let trail_row = head_row - i;
                if trail_row >= 0 && trail_row < rows as i32 {
                    let char_idx = ((trail_row as u32).wrapping_add(col.char_offset) / 4) as usize
                        % FONT.len();
                    let char_bitmap = &FONT[char_idx];

                    let start_x = cx * cw;
                    let start_y = trail_row as usize * ch;

                    // Trailing characters are the configured color (green)
                    let trail_color = config.color;

                    for dy in 0..ch {
                        let py = start_y + dy;
                        if py >= height {
                            break;
                        }

                        let bitmap_y = (dy * 12) / ch; // Scale Y to 0-11
                        let row_bits = char_bitmap[bitmap_y];

                        for dx in 0..cw {
                            let px = start_x + dx;
                            if px >= width {
                                break;
                            }

                            let bitmap_x = (dx * 8) / cw; // Scale X to 0-7
                            let mask = 1 << (7 - bitmap_x);

                            if (row_bits & mask) != 0 {
                                // Don't completely overwrite, but max it with the trail color
                                pixels[py * width + px] = trail_color;
                            }
                        }
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_apply_matrix_rain_changes_buffer() {
        let width = 64;
        let height = 64;
        let mut fb = Framebuffer::new(width, height).unwrap();
        fb.clear(0xFF_000000);

        let mut config = MatrixRainConfig::default();
        config.speed = 100.0; // Fast to ensure movement

        let mut fb_clone = Framebuffer::new(width, height).unwrap();
        fb_clone.as_mut_slice().copy_from_slice(fb.as_slice());

        // Apply effect a few times to advance the simulation
        for _ in 0..5 {
            config.time += 0.016;
            apply_matrix_rain(&mut fb, &config);
        }

        // Verify the buffer was modified (some green/white pixels should exist)
        let mut different = false;
        for i in 0..(width * height) as usize {
            if fb.as_slice()[i] != fb_clone.as_slice()[i] {
                different = true;
                break;
            }
        }
        assert!(
            different,
            "Matrix Rain filter did not modify the framebuffer"
        );
    }

    #[test]
    fn test_apply_matrix_rain_zero_size() {
        let width = 10;
        let height = 10;
        let mut fb = Framebuffer::new(width, height).unwrap();
        fb.clear(0xFF12_3456);

        let mut config = MatrixRainConfig::default();
        config.cell_width = 0; // Should early exit
        config.cell_height = 0;

        apply_matrix_rain(&mut fb, &config);

        // Buffer should be untouched
        for &pixel in fb.as_slice() {
            assert_eq!(pixel, 0xFF12_3456);
        }
    }
}
