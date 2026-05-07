//! Experimental Falling Sand Simulation
//!
//! Simulates falling sand physics directly on the Framebuffer.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::random::Rng;

/// Simulates falling sand physics on a framebuffer.
///
/// * `fb`: The Framebuffer where particles are drawn.
/// * `sand_color`: The ARGB color representing sand (e.g. `0xFFEE_DD88`).
/// * `empty_color`: The ARGB color representing empty space (e.g. `0xFF00_0000`).
/// * `seed`: A random seed to randomize horizontal processing direction.
pub fn update_falling_sand(fb: &mut Framebuffer, sand_color: u32, empty_color: u32, seed: u64) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height < 2 {
        return;
    }

    let mut rng = Rng::seeded(seed);

    // We process from bottom to top (excluding the very bottom row, which has nowhere to fall)
    // to prevent particles from teleporting multiple rows in a single frame.
    for y in (0..(height - 1)).rev() {
        // Randomize the horizontal iteration direction to prevent directional bias
        // when sand grains fall diagonally.
        let dir = if rng.bool() { 1 } else { -1 };

        let start_x = if dir == 1 { 0 } else { width as isize - 1 };
        let end_x = if dir == 1 { width as isize } else { -1 };

        let mut x = start_x;
        while x != end_x {
            let ux = x as usize;
            let current_color = fb.get_pixel(ux as i32, y as i32).unwrap_or(empty_color);

            // Only process sand particles
            if current_color == sand_color {
                // Check cell directly below
                if fb
                    .get_pixel(ux as i32, (y + 1) as i32)
                    .unwrap_or(sand_color)
                    == empty_color
                {
                    fb.set_pixel(ux as i32, y as i32, empty_color);
                    fb.set_pixel(ux as i32, (y + 1) as i32, sand_color);
                } else {
                    // Try diagonally left and right
                    // Randomize which diagonal to try first to prevent piling bias
                    let try_left_first = rng.bool();

                    if try_left_first {
                        if x > 0
                            && fb
                                .get_pixel((ux - 1) as i32, (y + 1) as i32)
                                .unwrap_or(sand_color)
                                == empty_color
                        {
                            fb.set_pixel(ux as i32, y as i32, empty_color);
                            fb.set_pixel((ux - 1) as i32, (y + 1) as i32, sand_color);
                        } else if ux + 1 < width
                            && fb
                                .get_pixel((ux + 1) as i32, (y + 1) as i32)
                                .unwrap_or(sand_color)
                                == empty_color
                        {
                            fb.set_pixel(ux as i32, y as i32, empty_color);
                            fb.set_pixel((ux + 1) as i32, (y + 1) as i32, sand_color);
                        }
                    } else {
                        if ux + 1 < width
                            && fb
                                .get_pixel((ux + 1) as i32, (y + 1) as i32)
                                .unwrap_or(sand_color)
                                == empty_color
                        {
                            fb.set_pixel(ux as i32, y as i32, empty_color);
                            fb.set_pixel((ux + 1) as i32, (y + 1) as i32, sand_color);
                        } else if x > 0
                            && fb
                                .get_pixel((ux - 1) as i32, (y + 1) as i32)
                                .unwrap_or(sand_color)
                                == empty_color
                        {
                            fb.set_pixel(ux as i32, y as i32, empty_color);
                            fb.set_pixel((ux - 1) as i32, (y + 1) as i32, sand_color);
                        }
                    }
                }
            }

            x += dir;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAND: u32 = 0xFFEE_DD88;
    const EMPTY: u32 = 0xFF00_0000;
    const ROCK: u32 = 0xFF88_8888;

    #[test]
    fn test_sand_falls_down() {
        let mut fb = Framebuffer::new(3, 3).unwrap();
        fb.clear(EMPTY);
        fb.set_pixel(1, 0, SAND);

        update_falling_sand(&mut fb, SAND, EMPTY, 42);

        assert_eq!(fb.get_pixel(1, 0).unwrap(), EMPTY);
        assert_eq!(fb.get_pixel(1, 1).unwrap(), SAND);
    }

    #[test]
    fn test_sand_stops_at_bottom() {
        let mut fb = Framebuffer::new(3, 3).unwrap();
        fb.clear(EMPTY);
        fb.set_pixel(1, 2, SAND);

        update_falling_sand(&mut fb, SAND, EMPTY, 42);

        assert_eq!(fb.get_pixel(1, 2).unwrap(), SAND);
    }

    #[test]
    fn test_sand_falls_diagonally_over_obstacle() {
        let mut fb = Framebuffer::new(3, 3).unwrap();
        fb.clear(EMPTY);

        // Put rock in middle
        fb.set_pixel(1, 1, ROCK);
        // Put sand directly above rock
        fb.set_pixel(1, 0, SAND);

        update_falling_sand(&mut fb, SAND, EMPTY, 42);

        // Sand should move diagonally since below is blocked
        assert_eq!(fb.get_pixel(1, 0).unwrap(), EMPTY);

        let moved_left = fb.get_pixel(0, 1).unwrap() == SAND;
        let moved_right = fb.get_pixel(2, 1).unwrap() == SAND;

        assert!(moved_left || moved_right, "Sand did not fall diagonally");
    }

    #[test]
    fn test_sand_respects_boundaries() {
        let mut fb = Framebuffer::new(3, 3).unwrap();
        fb.clear(EMPTY);

        // Put rock below
        fb.set_pixel(0, 1, ROCK);
        fb.set_pixel(1, 1, ROCK);
        fb.set_pixel(2, 1, ROCK);

        // Put sand on left and right edge
        fb.set_pixel(0, 0, SAND);
        fb.set_pixel(2, 0, SAND);

        update_falling_sand(&mut fb, SAND, EMPTY, 42);

        // Sand should not be able to move since boundaries are hit and rocks are below
        assert_eq!(fb.get_pixel(0, 0).unwrap(), SAND);
        assert_eq!(fb.get_pixel(2, 0).unwrap(), SAND);
    }
}
