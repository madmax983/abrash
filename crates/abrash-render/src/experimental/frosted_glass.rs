//! Frosted Glass Post-Processing Filter
//!
//! A retro-style filter that reduces the perceived clarity of the framebuffer
//! by simulating a view through textured/frosted privacy glass. It works by
//! applying random spatial displacement to each pixel's sampling coordinate.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::random::Rng;
use std::cell::RefCell;

thread_local! {
    static SOURCE_PIXELS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Applies a frosted glass displacement effect to the framebuffer.
///
/// Displaces each pixel by randomly sampling a nearby pixel within a square
/// radius of `intensity`. A PRNG is used per-pixel.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `intensity` - The maximum displacement distance (in pixels).
/// * `seed` - The random seed to base the spatial noise on. Change this every frame for animated noise, or keep it constant for static glass.
pub fn apply_frosted_glass(fb: &mut Framebuffer, intensity: f32, seed: u64) {
    if intensity <= 0.0 {
        return;
    }

    let width = fb.width() as i32;
    let height = fb.height() as i32;

    if width == 0 || height == 0 {
        return;
    }

    let intensity_i = intensity.ceil() as i32;

    // We need to clone the framebuffer to safely read scattered pixels
    // without reading back pixels we've already modified.
    // ⚡ Bolt: Eliminate per-frame heap allocation by using a thread-local static buffer.
    SOURCE_PIXELS.with(|buf| {
        let mut src_pixels = buf.borrow_mut();
        src_pixels.clear();
        src_pixels.extend_from_slice(fb.as_slice());
        let src_buf = src_pixels.as_slice();

        let dest_pixels = fb.as_mut_slice();

        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;

            dest_pixels
                .par_chunks_exact_mut(width as usize)
                .enumerate()
                .for_each(|(y, row)| {
                    // Seed based on row and user seed, making it deterministic per-pixel for a given seed
                    // and avoiding thread-local RNG synchronization overhead.
                    let mut rng = Rng::seeded(seed.wrapping_add((y as u64) * 1_000_000));
                    frost_row(row, y as i32, height, intensity_i, src_buf, &mut rng);
                });
        }

        #[cfg(not(feature = "parallel"))]
        {
            let mut rng = Rng::seeded(seed);

            for (y, row) in dest_pixels.chunks_exact_mut(width as usize).enumerate() {
                frost_row(row, y as i32, height, intensity_i, src_buf, &mut rng);
            }
        }
    });
}

/// Fills one destination row by sampling `src_buf` at random offsets in
/// `[-intensity_i, +intensity_i]` on both axes, clamped to the framebuffer.
///
/// The RNG is drawn in the same order for every pixel (dx then dy), so output
/// is independent of which branch below handles the pixel. Pixels at least
/// `intensity_i` away from every edge cannot leave the buffer, so they skip the
/// clamps.
fn frost_row(
    row: &mut [u32],
    y: i32,
    height: i32,
    intensity_i: i32,
    src_buf: &[u32],
    rng: &mut Rng,
) {
    let width = row.len() as i32;
    let row_interior = y >= intensity_i && y + intensity_i < height;
    let x_lo = intensity_i.min(width);
    let x_hi = (width - intensity_i).max(x_lo);

    for (x, pixel) in row.iter_mut().enumerate() {
        let x = x as i32;
        let dx = rng.i32_range(-intensity_i, intensity_i);
        let dy = rng.i32_range(-intensity_i, intensity_i);

        let sx = x + dx;
        let sy = y + dy;

        // `.max(0).min(max)` instead of `Ord::clamp` avoids its `min <= max` assert.
        let (sx, sy) = if row_interior && x >= x_lo && x < x_hi {
            (sx, sy)
        } else {
            (sx.max(0).min(width - 1), sy.max(0).min(height - 1))
        };

        *pixel = src_buf[(sy * width + sx) as usize];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frosted_glass_mutates_pixels() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        // Fill half with white, half with black
        for y in 0..10 {
            for x in 0..10 {
                let color = if x < 5 { 0xFFFF_FFFF } else { 0xFF00_0000 };
                fb.set_pixel(x, y, color);
            }
        }

        let original = fb.as_slice().to_vec();

        // Apply a strong frosted glass effect
        apply_frosted_glass(&mut fb, 3.0, 12345);

        let modified = fb.as_slice().to_vec();

        // The image should have been modified (boundary blurred)
        assert_ne!(original, modified);
    }

    #[test]
    fn test_frosted_glass_zero_intensity() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        // Fill half with white, half with black
        for y in 0..10 {
            for x in 0..10 {
                let color = if x < 5 { 0xFFFF_FFFF } else { 0xFF00_0000 };
                fb.set_pixel(x, y, color);
            }
        }

        let original = fb.as_slice().to_vec();

        apply_frosted_glass(&mut fb, 0.0, 12345);

        let modified = fb.as_slice().to_vec();

        assert_eq!(original, modified);
    }

    #[test]
    fn test_frosted_glass_matches_clamp_everywhere_reference() {
        for &(w, h, intensity) in &[
            (37u32, 29u32, 4.0f32),
            (8, 8, 10.0),
            (50, 3, 2.5),
            (1, 9, 3.0),
        ] {
            let mut fb = Framebuffer::new(w, h).unwrap();
            for (i, p) in fb.as_mut_slice().iter_mut().enumerate() {
                *p = 0xFF00_0000 | (i as u32).wrapping_mul(2_654_435_761);
            }
            let src = fb.as_slice().to_vec();
            apply_frosted_glass(&mut fb, intensity, 99);

            let k = intensity.ceil() as i32;
            let (wi, hi) = (w as i32, h as i32);
            #[cfg(not(feature = "parallel"))]
            let mut rng = Rng::seeded(99);
            for y in 0..hi {
                #[cfg(feature = "parallel")]
                let mut rng = Rng::seeded(99u64.wrapping_add((y as u64) * 1_000_000));
                for x in 0..wi {
                    let sx = (x + rng.i32_range(-k, k)).max(0).min(wi - 1);
                    let sy = (y + rng.i32_range(-k, k)).max(0).min(hi - 1);
                    assert_eq!(
                        fb.as_slice()[(y * wi + x) as usize],
                        src[(sy * wi + sx) as usize],
                        "{w}x{h} k={k} at ({x},{y})"
                    );
                }
            }
        }
    }
}
