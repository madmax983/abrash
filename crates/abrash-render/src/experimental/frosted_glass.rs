use abrash_core::framebuffer::Framebuffer;
#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Applies a frosted glass effect to the framebuffer.
///
/// Displaces pixels randomly within a specified radius to simulate looking through
/// textured or frosted glass.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `radius` - The maximum scatter distance in pixels.
pub fn apply_frosted_glass(fb: &mut Framebuffer, radius: u32) {
    if radius == 0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    // Clone the source framebuffer to safely sample displaced pixels without mutable aliasing
    let src = fb.as_slice().to_vec();

    // Fast inline PRNG (Xorshift32) to generate random offsets per pixel
    #[inline(always)]
    fn xorshift32(state: &mut u32) -> u32 {
        let mut x = *state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        *state = x;
        x
    }

    let radius_i32 = radius as i32;

    let process_row = |(y, row): (usize, &mut [u32])| {
        // Seed the PRNG with something distinct for each row to prevent repeating patterns
        let mut seed = 1_234_567_890 ^ (y as u32 * 1_337);
        if seed == 0 {
            seed = 1;
        }

        for x in 0..width {
            // Generate random displacement within roughly [-radius, radius]
            // We use modular arithmetic for speed rather than a perfect uniform distribution
            let rx = (xorshift32(&mut seed) % (radius * 2 + 1)) as i32 - radius_i32;
            let ry = (xorshift32(&mut seed) % (radius * 2 + 1)) as i32 - radius_i32;

            let src_x = (x as i32 + rx).clamp(0, width as i32 - 1) as usize;
            let src_y = (y as i32 + ry).clamp(0, height as i32 - 1) as usize;

            row[x] = src[src_y * width + src_x];
        }
    };

    #[cfg(feature = "parallel")]
    {
        fb.as_mut_slice()
            .par_chunks_exact_mut(width)
            .enumerate()
            .for_each(process_row);
    }
    #[cfg(not(feature = "parallel"))]
    {
        fb.as_mut_slice()
            .chunks_exact_mut(width)
            .enumerate()
            .for_each(process_row);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_frosted_glass() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFF000000);
        // Draw a single white pixel in the center
        fb.set_pixel(5, 5, 0xFFFFFFFF);

        apply_frosted_glass(&mut fb, 2);

        // Since the effect scatters the white pixel randomly within a radius of 2,
        // the original pixel at (5, 5) shouldn't necessarily be the only white pixel,
        // and some surrounding pixels might be white now. We can assert that the
        // total number of white pixels might change depending on the implementation details
        // (if multiple pixels fetch from it), but at least the image shouldn't be identical
        // to a 1-pixel dot, or at least the implementation should run without crashing.

        // A simple test: make sure it modifies the buffer.
        // We will test by putting a white block and making sure it bleeds out.
        let mut fb2 = Framebuffer::new(10, 10).unwrap();
        fb2.clear(0xFF000000);
        for y in 4..=6 {
            for x in 4..=6 {
                fb2.set_pixel(x, y, 0xFFFFFFFF);
            }
        }

        apply_frosted_glass(&mut fb2, 3);

        // Original white count is 9. Due to scattering, the central block will likely lose some white,
        // and surrounding black area will gain some white, but the total count might be different
        // depending on PRNG. What we do know is that a pixel outside the 4..=6 box should now be white.
        let mut bled_outside = false;
        for y in 0..10 {
            for x in 0..10 {
                if (x < 4 || x > 6 || y < 4 || y > 6) && fb2.get_pixel(x, y).unwrap() == 0xFFFFFFFF
                {
                    bled_outside = true;
                    break;
                }
            }
        }

        assert!(
            bled_outside,
            "Frosted glass failed to scatter pixels outside the original block"
        );
    }
}
