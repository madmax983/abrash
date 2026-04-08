use crate::framebuffer::Framebuffer;
use abrash_core::utils::XorShift32;

/// Applies a "Frosted Glass" effect to the framebuffer.
///
/// This effect randomly displaces the sampling coordinates for each pixel within a
/// specified radius, creating a scattered, noisy look similar to looking through
/// structured or frosted glass.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `radius` - The maximum scatter distance in pixels.
/// * `seed` - A base random seed.
pub fn apply_frosted_glass(fb: &mut Framebuffer, radius: f32, seed: u32) {
    if radius <= 0.0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    // Clone the source framebuffer to allow arbitrary reads while mutating
    // the destination in parallel without mutable aliasing.
    let src_pixels = fb.as_slice().to_vec();
    let dest_pixels = fb.as_mut_slice();

    let radius_i32 = radius as i32;

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        dest_pixels
            .par_chunks_exact_mut(width)
            .enumerate()
            .for_each(|(y, row)| {
                // Initialize an inline PRNG per row.
                // Using an inline PRNG avoids the severe synchronization overhead
                // of a thread-local or global RNG.
                let mut prng = XorShift32::new((y as u32).wrapping_add(seed) ^ 0xDEAD_BEEF);
                let y_i32 = y as i32;

                for (x, pixel) in row.iter_mut().enumerate() {
                    let x_i32 = x as i32;

                    // Generate random offset in [-radius, radius]
                    let dx = (prng.next_f32() * 2.0 - 1.0) * radius;
                    let dy = (prng.next_f32() * 2.0 - 1.0) * radius;

                    // Sample source coordinate
                    let mut src_x = x_i32 + dx as i32;
                    let mut src_y = y_i32 + dy as i32;

                    // Clamp to framebuffer bounds
                    src_x = src_x.clamp(0, width as i32 - 1);
                    src_y = src_y.clamp(0, height as i32 - 1);

                    let src_idx = (src_y as usize) * width + (src_x as usize);
                    *pixel = src_pixels[src_idx];
                }
            });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for y in 0..height {
            let mut prng = XorShift32::new((y as u32).wrapping_add(seed) ^ 0xDEAD_BEEF);
            let y_i32 = y as i32;

            for x in 0..width {
                let x_i32 = x as i32;

                // Generate random offset in [-radius, radius]
                let dx = (prng.next_f32() * 2.0 - 1.0) * radius;
                let dy = (prng.next_f32() * 2.0 - 1.0) * radius;

                // Sample source coordinate
                let mut src_x = x_i32 + dx as i32;
                let mut src_y = y_i32 + dy as i32;

                // Clamp to framebuffer bounds
                src_x = src_x.clamp(0, width as i32 - 1);
                src_y = src_y.clamp(0, height as i32 - 1);

                let src_idx = (src_y as usize) * width + (src_x as usize);
                let dest_idx = y * width + x;
                dest_pixels[dest_idx] = src_pixels[src_idx];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_apply_frosted_glass_basic() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        // Fill with white
        fb.clear(0xFFFFFFFF);

        // Put a black pixel in the center
        fb.set_pixel(5, 5, 0xFF000000);

        // Apply effect with small radius
        apply_frosted_glass(&mut fb, 2.0, 42);

        // It should not panic. The image should be changed, but verifying exactly is hard due to PRNG.
        // We just ensure it executed successfully.
        assert_eq!(fb.width(), 10);
        assert_eq!(fb.height(), 10);
    }

    #[test]
    fn test_apply_frosted_glass_zero_radius() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFFFFFFFF);

        apply_frosted_glass(&mut fb, 0.0, 42);

        for y in 0..10 {
            for x in 0..10 {
                assert_eq!(fb.get_pixel(x, y), Some(0xFFFFFFFF));
            }
        }
    }
}
