use abrash_core::framebuffer::Framebuffer;
#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// A simple fast inline Xorshift PRNG for shader-like effects.
struct XorShift32 {
    state: u32,
}

impl XorShift32 {
    #[inline(always)]
    fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0x12345678 } else { seed },
        }
    }

    #[inline(always)]
    fn next(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }
}

/// Applies a Frosted Glass scatter effect.
///
/// It randomly displaces the sampling coordinate of each pixel by up to `scatter_radius`
/// in both the X and Y axes.
///
/// Requires the `nova` feature. Will use multithreading if `parallel` feature is enabled.
pub fn apply_frosted_glass(fb: &mut Framebuffer, scatter_radius: i32, seed_offset: u32) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || scatter_radius <= 0 {
        return;
    }

    let source = fb.as_slice().to_vec();
    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let row_iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = pixels.chunks_exact_mut(width).enumerate();

    let scatter = scatter_radius;
    let width_i32 = width as i32;
    let height_i32 = height as i32;

    row_iter.for_each(|(y, row)| {
        let y_i32 = y as i32;
        for (x, pixel) in row.iter_mut().enumerate() {
            let x_i32 = x as i32;

            // Simple fast deterministic seed per pixel
            let seed = (x as u32).wrapping_mul(1973)
                .wrapping_add((y as u32).wrapping_mul(9277))
                .wrapping_add(seed_offset.wrapping_mul(26699))
                .wrapping_add(111111);

            let mut rng = XorShift32::new(seed);

            // Random displacement in [-scatter, scatter]
            let dx = (rng.next() % (2 * scatter as u32 + 1)) as i32 - scatter;
            let dy = (rng.next() % (2 * scatter as u32 + 1)) as i32 - scatter;

            let nx = (x_i32 + dx).clamp(0, width_i32 - 1) as usize;
            let ny = (y_i32 + dy).clamp(0, height_i32 - 1) as usize;

            *pixel = source[ny * width + nx];
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_frosted_glass() {
        let mut fb = Framebuffer::new(5, 5).unwrap();

        // Fill center with Red, rest with Black
        for y in 1..=3 {
            for x in 1..=3 {
                fb.set_pixel(x, y, 0xFFFF0000);
            }
        }

        // Scatter radius 1, seed 0
        apply_frosted_glass(&mut fb, 1, 0);

        let mut red_count = 0;
        for i in 0..25 {
            if fb.as_slice()[i] == 0xFFFF0000 {
                red_count += 1;
            }
        }
        // At least some pixels should sample the red center
        assert!(red_count > 0);
    }
}
