#[cfg(feature = "parallel")]
use rayon::prelude::*;

use abrash_core::framebuffer::Framebuffer;

/// Applies a sepia tone filter to the framebuffer.
///
/// Uses the standard sepia matrix coefficients:
/// R' = (R * 0.393) + (G * 0.769) + (B * 0.189)
/// G' = (R * 0.349) + (G * 0.686) + (B * 0.168)
/// B' = (R * 0.272) + (G * 0.534) + (B * 0.131)
///
/// # Panics
///
/// This function does not panic.
pub fn apply_sepia(fb: &mut Framebuffer) {
    let width = fb.width() as usize;
    let slice = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    {
        slice.par_chunks_exact_mut(width).for_each(|row| {
            for pixel in row.iter_mut() {
                apply_sepia_pixel(pixel);
            }
        });
    }

    #[cfg(not(feature = "parallel"))]
    {
        slice.chunks_exact_mut(width).for_each(|row| {
            for pixel in row.iter_mut() {
                apply_sepia_pixel(pixel);
            }
        });
    }
}

#[inline(always)]
fn apply_sepia_pixel(pixel: &mut u32) {
    let p = *pixel;
    let r = ((p >> 16) & 0xFF) as f32;
    let g = ((p >> 8) & 0xFF) as f32;
    let b = (p & 0xFF) as f32;

    let tr = (r * 0.393 + g * 0.769 + b * 0.189) as u32;
    let tg = (r * 0.349 + g * 0.686 + b * 0.168) as u32;
    let tb = (r * 0.272 + g * 0.534 + b * 0.131) as u32;

    let final_r = tr.min(255);
    let final_g = tg.min(255);
    let final_b = tb.min(255);

    *pixel = (p & 0xFF00_0000) | (final_r << 16) | (final_g << 8) | final_b;
}
