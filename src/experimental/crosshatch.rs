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
/// Bolt Performance Optimization:
    /// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
    /// remainder chunk handling and bounds checking, enabling better vectorization
    /// and measurable performance improvements.
/// Bolt Performance Optimization:
/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
/// remainder chunk handling and bounds checking, enabling better vectorization
/// and measurable performance improvements.
pub fn apply_crosshatch(fb: &mut Framebuffer, spacing: usize) {
    if spacing == 0 {
        return;
    }

    let width = fb.width() as usize;
    let _height = fb.height() as usize;
    let pixels = fb.as_mut_slice();

    // Fast path: if parallel feature is enabled, use Rayon
    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;

        // Process rows in parallel
        // To avoid needing a full secondary buffer for luminance,
        // we can compute luminance on the fly from the original pixel
        // before we overwrite it.
        pixels
            .par_chunks_exact_mut(width)
            .enumerate()
            .for_each(|(y, row)| {
                for (x, pixel) in row.iter_mut().enumerate() {
                    let lum = pixel_luminance(*pixel);
                    let original_alpha = *pixel & 0xFF00_0000;

                    let mut is_ink = false;

                    if lum < 200 {
                        // T4: Light - Top-left to bottom-right hatch
                        if (x + y) % spacing == 0 {
                            is_ink = true;
                        }
                    }
                    if lum < 150 {
                        // T3: Medium Light - Top-right to bottom-left hatch
                        if x.abs_diff(y) % spacing == 0 {
                            is_ink = true;
                        }
                    }
                    if lum < 100 {
                        // T2: Medium Dark - Horizontal lines
                        if y % spacing == 0 {
                            is_ink = true;
                        }
                    }
                    if lum < 50 {
                        // T1: Darkest - Vertical lines
                        if x % spacing == 0 {
                            is_ink = true;
                        }
                    }

                    if is_ink {
                        *pixel = original_alpha; // Black
                    } else {
                        *pixel = original_alpha | 0x00FF_FFFF; // White
                    }
                }
            });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for (y, row) in pixels.chunks_exact_mut(width).enumerate() {
            for (x, pixel) in row.iter_mut().enumerate() {
                let lum = pixel_luminance(*pixel);
                let original_alpha = *pixel & 0xFF00_0000;

                let mut is_ink = false;

                if lum < 200 {
                    if (x + y) % spacing == 0 {
                        is_ink = true;
                    }
                }
                if lum < 150 {
                    if x.abs_diff(y) % spacing == 0 {
                        is_ink = true;
                    }
                }
                if lum < 100 {
                    if y % spacing == 0 {
                        is_ink = true;
                    }
                }
                if lum < 50 {
                    if x % spacing == 0 {
                        is_ink = true;
                    }
                }

                if is_ink {
                    *pixel = original_alpha;
                } else {
                    *pixel = original_alpha | 0x00FF_FFFF;
                }
            }
        }
    }
}
