use crate::framebuffer::Framebuffer;

pub fn apply_frosted_glass(fb: &mut Framebuffer, radius: u32, seed: u64) {
    if radius == 0 {
        return;
    }

    let width = fb.width() as i32;
    let height = fb.height() as i32;

    // We must clone the buffer since we are reading from random locations while writing
    let source = fb.as_slice().to_vec();
    let dest = fb.as_mut_slice();

    let r_i32 = radius as i32;
    let range = (radius * 2 + 1) as u32;
    let uwidth = width as usize;

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        dest.par_chunks_exact_mut(uwidth)
            .enumerate()
            .for_each(|(y, row)| {
                // Initialize a thread-local PRNG seeded deterministically by the row index and base seed
                // Ensure seed is never 0 for xor-shift
                let mut current_seed = seed.wrapping_add(y as u64).wrapping_mul(11400714819323198485);
                if current_seed == 0 {
                    current_seed = 1;
                }

                let y = y as i32;
                for (x, pixel) in row.iter_mut().enumerate() {
                    let x = x as i32;

                    // Generate two random numbers per pixel using xor-shift
                    current_seed ^= current_seed << 13;
                    current_seed ^= current_seed >> 17;
                    current_seed ^= current_seed << 5;

                    let rand_x = (current_seed >> 32) as u32;
                    let rand_y = current_seed as u32;

                    let dx = (rand_x % range) as i32 - r_i32;
                    let dy = (rand_y % range) as i32 - r_i32;

                    // Fast max/min clamp logic without function calls
                    let mut nx = x + dx;
                    if nx < 0 { nx = 0; } else if nx >= width { nx = width - 1; }

                    let mut ny = y + dy;
                    if ny < 0 { ny = 0; } else if ny >= height { ny = height - 1; }

                    *pixel = source[(ny * width + nx) as usize];
                }
            });
    }

    #[cfg(not(feature = "parallel"))]
    {
        let mut current_seed = seed;
        if current_seed == 0 {
            current_seed = 1;
        }
        dest.chunks_exact_mut(uwidth)
            .enumerate()
            .for_each(|(y, row)| {
                let y = y as i32;
                for (x, pixel) in row.iter_mut().enumerate() {
                    let x = x as i32;

                    // Generate two random numbers per pixel using xor-shift
                    current_seed ^= current_seed << 13;
                    current_seed ^= current_seed >> 17;
                    current_seed ^= current_seed << 5;

                    let rand_x = (current_seed >> 32) as u32;
                    let rand_y = current_seed as u32;

                    let dx = (rand_x % range) as i32 - r_i32;
                    let dy = (rand_y % range) as i32 - r_i32;

                    // Fast max/min clamp logic without function calls
                    let mut nx = x + dx;
                    if nx < 0 { nx = 0; } else if nx >= width { nx = width - 1; }

                    let mut ny = y + dy;
                    if ny < 0 { ny = 0; } else if ny >= height { ny = height - 1; }

                    *pixel = source[(ny * width + nx) as usize];
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frosted_glass_displacement() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        // Fill the framebuffer with a known pattern:
        // Left half is Red, Right half is Blue
        for y in 0..10 {
            for x in 0..10 {
                let color = if x < 5 { 0xFFFF_0000 } else { 0xFF00_00FF };
                fb.set_pixel(x, y, color);
            }
        }

        // Apply frosted glass with a radius of 2
        apply_frosted_glass(&mut fb, 2, 12345);

        // Check that some pixels near the boundary (x=4, x=5) have been displaced.
        // There should be at least one red pixel on the right half, or one blue pixel on the left half.
        let mut displaced = false;
        for y in 0..10 {
            for x in 0..5 {
                if fb.get_pixel(x, y).unwrap() == 0xFF00_00FF {
                    displaced = true;
                }
            }
            for x in 5..10 {
                if fb.get_pixel(x, y).unwrap() == 0xFFFF_0000 {
                    displaced = true;
                }
            }
        }

        // Also verify radius of 0 does not modify the image
        let mut fb_zero = Framebuffer::new(10, 10).unwrap();
        fb_zero.set_pixel(5, 5, 0xFF00_FF00);
        apply_frosted_glass(&mut fb_zero, 0, 12345);
        assert_eq!(fb_zero.get_pixel(5, 5).unwrap(), 0xFF00_FF00);

        // Test fails if displacement didn't occur
        assert!(displaced, "Pixels were not displaced across the boundary.");
    }
}
