#[cfg(feature = "parallel")]
use rayon::prelude::*;

use crate::framebuffer::Framebuffer;

/// Applies a film grain effect to the framebuffer in-place.
/// Modulates the pixel intensity with pseudo-random noise.
///
/// # Arguments
///
/// *   `amount` - The intensity of the noise (0.0 to 1.0).
/// *   `seed` - The random seed, typically changes each frame.
pub fn apply_film_grain(fb: &mut Framebuffer, amount: f32, seed: u32) {
    if amount <= 0.0 {
        return;
    }

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    {
        pixels.par_iter_mut().enumerate().for_each(|(i, pixel)| {
            process_pixel(i, pixel, amount, seed);
        });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for (i, pixel) in pixels.iter_mut().enumerate() {
            process_pixel(i, pixel, amount, seed);
        }
    }
}

#[inline(always)]
fn process_pixel(i: usize, pixel: &mut u32, amount: f32, seed: u32) {
    let p = *pixel;
    let a = p & 0xFF00_0000;
    let r = (p >> 16) & 0xFF;
    let g = (p >> 8) & 0xFF;
    let b = p & 0xFF;

    // Simple coordinate-based PRNG hash (WANG hash variant)
    let mut h = (i as u32).wrapping_add(seed);
    h = (h ^ 61) ^ (h >> 16);
    h = h.wrapping_add(h << 3);
    h = h ^ (h >> 4);
    h = h.wrapping_mul(0x27d4eb2d);
    h = h ^ (h >> 15);

    // Map hash to noise range [-1.0, 1.0]
    let noise = (h as f32 / std::u32::MAX as f32) * 2.0 - 1.0;

    // Scale noise by amount and map to [-255, 255]
    let noise_val = (noise * amount * 255.0) as i32;

    let new_r = (r as i32 + noise_val).clamp(0, 255) as u32;
    let new_g = (g as i32 + noise_val).clamp(0, 255) as u32;
    let new_b = (b as i32 + noise_val).clamp(0, 255) as u32;

    *pixel = a | (new_r << 16) | (new_g << 8) | new_b;
}
