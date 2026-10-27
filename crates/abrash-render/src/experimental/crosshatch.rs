//! Crosshatch Stylization Filter
//!
//! A non-photorealistic post-processing effect that draws crosshatching
//! lines over the image based on the underlying luminance. Darker areas
//! receive thicker or more overlapping hatching lines.

use crate::framebuffer::Framebuffer;
use crate::utils::pixel_luminance;

/// Applies a crosshatch effect to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `spacing` - The base distance between hatching lines.
/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
pub fn apply_crosshatch(fb: &mut Framebuffer, spacing: usize) {
    if spacing == 0 {
        return;
    }

    let width = fb.width() as usize;
    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        pixels
            .par_chunks_exact_mut(width)
            .enumerate()
            .for_each(|(y, row)| hatch_row(row, y, spacing));
    }

    #[cfg(not(feature = "parallel"))]
    for (y, row) in pixels.chunks_exact_mut(width).enumerate() {
        hatch_row(row, y, spacing);
    }
}

const FLAG_DIAG: u8 = 1; // (x + y) % spacing == 0
const FLAG_ANTI: u8 = 2; // x.abs_diff(y) % spacing == 0
const FLAG_ROW: u8 = 4; // y % spacing == 0
const FLAG_COL: u8 = 8; // x % spacing == 0

/// For each luminance, the set of hatch directions that draw ink.
const TIER_MASK: [u8; 256] = {
    let mut t = [0u8; 256];
    let mut lum = 0;
    while lum < 256 {
        let mut m = 0;
        if lum < 200 {
            m |= FLAG_DIAG;
        }
        if lum < 150 {
            m |= FLAG_ANTI;
        }
        if lum < 100 {
            m |= FLAG_ROW;
        }
        if lum < 50 {
            m |= FLAG_COL;
        }
        t[lum] = m;
        lum += 1;
    }
    t
};

/// Hatches one row. With `c = x % spacing` tracked by a wrapping counter,
/// `(x + y) % spacing == 0` is `c == (spacing - y % spacing) % spacing` and
/// `x.abs_diff(y) % spacing == 0` is `c == y % spacing` (x and y are congruent
/// either way round), so no per-pixel division is needed.
fn hatch_row(row: &mut [u32], y: usize, spacing: usize) {
    let y_mod = y % spacing;
    let diag_c = if y_mod == 0 { 0 } else { spacing - y_mod };
    let row_flag = if y_mod == 0 { FLAG_ROW } else { 0 };
    let mut c = 0;

    for pixel in row.iter_mut() {
        let flags = u8::from(c == diag_c) * FLAG_DIAG
            | u8::from(c == y_mod) * FLAG_ANTI
            | row_flag
            | u8::from(c == 0) * FLAG_COL;
        let original_alpha = *pixel & 0xFF00_0000;
        let ink = flags & TIER_MASK[usize::from(pixel_luminance(*pixel))] != 0;

        *pixel = if ink {
            original_alpha
        } else {
            original_alpha | 0x00FF_FFFF
        };

        c += 1;
        if c == spacing {
            c = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(pixel: u32, x: usize, y: usize, spacing: usize) -> u32 {
        let lum = pixel_luminance(pixel);
        let alpha = pixel & 0xFF00_0000;
        let ink = (lum < 200 && (x + y) % spacing == 0)
            || (lum < 150 && x.abs_diff(y) % spacing == 0)
            || (lum < 100 && y % spacing == 0)
            || (lum < 50 && x % spacing == 0);
        if ink { alpha } else { alpha | 0x00FF_FFFF }
    }

    #[test]
    fn matches_modulo_reference() {
        let (w, h) = (37u32, 29u32);
        let mut src = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let c = (x * 7 + y * 13) % 256;
                src.push(0xFF00_0000 | (c << 16) | (c << 8) | c);
            }
        }
        for spacing in [1usize, 2, 3, 5, 6, 40] {
            let mut fb = Framebuffer::new(w, h).unwrap();
            fb.as_mut_slice().copy_from_slice(&src);
            apply_crosshatch(&mut fb, spacing);
            for y in 0..h as usize {
                for x in 0..w as usize {
                    let i = y * w as usize + x;
                    assert_eq!(fb.as_slice()[i], reference(src[i], x, y, spacing));
                }
            }
        }
    }
}
