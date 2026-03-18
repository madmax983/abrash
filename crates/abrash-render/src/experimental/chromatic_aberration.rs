//! Chromatic Aberration post-processing effect.
//!
//! Applies a color fringe effect by shifting RGB channels independently.

use crate::framebuffer::Framebuffer;

use std::cell::RefCell;

thread_local! {
    static SOURCE_PIXELS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the Chromatic Aberration filter.
#[derive(Debug, Clone, Copy)]
pub struct ChromaticAberrationConfig {
    /// Shift in pixels for the Red channel (X, Y)
    pub red_shift: (i32, i32),
    /// Shift in pixels for the Green channel (X, Y)
    pub green_shift: (i32, i32),
    /// Shift in pixels for the Blue channel (X, Y)
    pub blue_shift: (i32, i32),
}

impl Default for ChromaticAberrationConfig {
    fn default() -> Self {
        Self {
            red_shift: (2, 0),
            green_shift: (0, 0),
            blue_shift: (-2, 0),
        }
    }
}

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Applies a Chromatic Aberration filter to the framebuffer.
///
/// # Panics
///
/// Panics if the framebuffer slice length does not exactly match `width * height`.
pub fn apply_chromatic_aberration(fb: &mut Framebuffer, config: &ChromaticAberrationConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    assert_eq!(fb.as_slice().len(), width * height);

    // Clone the source framebuffer to avoid aliasing issues when parallelizing
    let mut source_pixels = SOURCE_PIXELS.with(std::cell::RefCell::take);
    let fb_slice = fb.as_slice();
    if source_pixels.len() != fb_slice.len() {
        source_pixels.resize(fb_slice.len(), 0);
    }
    source_pixels.copy_from_slice(fb_slice);
    let dest_pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let row_iter = dest_pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = dest_pixels.chunks_exact_mut(width).enumerate();

    let w_i32 = width as i32;
    let h_i32 = height as i32;

    row_iter.for_each(|(y, row)| {
        let y_i32 = y as i32;

        for (x, pixel) in row.iter_mut().enumerate() {
            let x_i32 = x as i32;

            // Calculate source coordinates for each channel
            let rx = (x_i32 - config.red_shift.0).clamp(0, w_i32 - 1);
            let ry = (y_i32 - config.red_shift.1).clamp(0, h_i32 - 1);

            let gx = (x_i32 - config.green_shift.0).clamp(0, w_i32 - 1);
            let gy = (y_i32 - config.green_shift.1).clamp(0, h_i32 - 1);

            let bx = (x_i32 - config.blue_shift.0).clamp(0, w_i32 - 1);
            let by = (y_i32 - config.blue_shift.1).clamp(0, h_i32 - 1);

            // Sample from source buffer
            let r_pixel = source_pixels[(ry * w_i32 + rx) as usize];
            let g_pixel = source_pixels[(gy * w_i32 + gx) as usize];
            let b_pixel = source_pixels[(by * w_i32 + bx) as usize];

            // Extract channels and maintain alpha from green channel (or original)
            let a = g_pixel & 0xFF_00_00_00;
            let r = r_pixel & 0x00_FF_00_00;
            let g = g_pixel & 0x00_00_FF_00;
            let b = b_pixel & 0x00_00_00_FF;

            // Combine and write back
            *pixel = a | r | g | b;
        }
    });

    SOURCE_PIXELS.with(|cell| cell.replace(source_pixels));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_chromatic_aberration() {
        let mut fb = Framebuffer::new(5, 5).unwrap();

        // Setup: center pixel is white (0xFFFFFFFF)
        // All other pixels are black (0xFF000000)
        fb.clear(0xFF_00_00_00);
        fb.set_pixel(2, 2, 0xFF_FF_FF_FF);

        let config = ChromaticAberrationConfig {
            red_shift: (1, 0),   // Red shifted right by 1
            green_shift: (0, 0), // Green stays at center
            blue_shift: (-1, 0), // Blue shifted left by 1
        };

        apply_chromatic_aberration(&mut fb, &config);

        // Verify Red channel shifted to (3, 2)
        let red_pixel = fb.get_pixel(3, 2).unwrap();
        assert_eq!((red_pixel >> 16) & 0xFF, 0xFF);

        // Verify Green channel stayed at (2, 2)
        let green_pixel = fb.get_pixel(2, 2).unwrap();
        assert_eq!((green_pixel >> 8) & 0xFF, 0xFF);

        // Verify Blue channel shifted to (1, 2)
        let blue_pixel = fb.get_pixel(1, 2).unwrap();
        assert_eq!(blue_pixel & 0xFF, 0xFF);

        // Verify the original center pixel doesn't have red or blue anymore
        assert_eq!((green_pixel >> 16) & 0xFF, 0x00);
        assert_eq!(green_pixel & 0xFF, 0x00);
    }
}
