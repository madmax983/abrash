//! Gameboy Camera Filter
//!
//! Simulates the iconic aesthetic of the Nintendo Game Boy Camera.
//! This effect downsamples the image and applies a 4-color green palette
//! using a 4x4 Bayer matrix ordered dither.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

// Standard Game Boy greens (light to dark)
const PALETTE: [u32; 4] = [
    0xFF_9BBC0F, // Lightest
    0xFF_8BAC0F, // Light
    0xFF_306230, // Dark
    0xFF_0F380F, // Darkest
];

// 4x4 Bayer Dithering Matrix
// Scaled to 0-255 range for direct comparison with 8-bit luminance
const BAYER_4X4: [[u8; 4]; 4] = [
    [0, 128, 32, 160],
    [192, 64, 224, 96],
    [48, 176, 16, 144],
    [240, 112, 208, 80],
];

/// Applies the Gameboy Camera effect to the framebuffer.
pub fn apply_gameboy_camera(fb: &mut Framebuffer) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let iter = pixels.chunks_exact_mut(width).enumerate();

    iter.for_each(|(y, row)| {
        let bayer_y = y % 4;

        for (x, pixel) in row.iter_mut().enumerate() {
            let bayer_x = x % 4;

            // 1. Convert to grayscale luminance (0-255)
            let lum = pixel_luminance(*pixel);

            // 2. Apply ordered dithering
            // We scale the luminance to the 4-level palette range (0 to 3)
            // To do this smoothly with the Bayer matrix, we adjust the luminance
            // based on the threshold matrix before quantization.

            // The Bayer matrix threshold determines how much "bump" we give the pixel.
            // We map the 0-255 threshold to a fraction of a palette step.
            // There are 3 "steps" between 4 colors. (255 / 3 = 85 size per step).

            let threshold = BAYER_4X4[bayer_y][bayer_x] as f32 / 255.0;

            // Map luminance from [0, 255] to [0.0, 3.0]
            let normalized_lum = (lum as f32) / 255.0 * 3.0;

            // Add the dither threshold. If the fractional part of the luminance
            // exceeds the matrix threshold, it bumps up to the next color index.
            let mut color_idx = (normalized_lum + threshold).floor() as usize;

            // Clamp to valid palette indices
            if color_idx > 3 {
                color_idx = 3;
            }

            // The palette is ordered Lightest to Darkest, but high luminance
            // means Brightest, so we invert the index to map high luma -> index 0
            // and low luma -> index 3.
            let inverted_idx = 3 - color_idx;

            // 3. Output mapped palette color
            *pixel = PALETTE[inverted_idx];
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gameboy_camera_palette() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        // Fill with a gradient to ensure all colors are tested
        for y in 0..10 {
            for x in 0..10 {
                let lum = (x * 25) as u32;
                fb.set_pixel_unchecked(x as usize, y as usize, 0xFF_00_00_00 | (lum << 16) | (lum << 8) | lum);
            }
        }

        apply_gameboy_camera(&mut fb);

        // Verify every pixel is exactly one of the 4 palette colors
        for &pixel in fb.as_slice() {
            assert!(
                PALETTE.contains(&pixel),
                "Pixel {pixel:X} is not in the Gameboy palette"
            );
        }
    }

    #[test]
    fn test_gameboy_camera_0x0() {
        let mut fb = Framebuffer::new(0, 0).unwrap();
        apply_gameboy_camera(&mut fb); // Should not panic
        assert_eq!(fb.as_slice().len(), 0);
    }
}
