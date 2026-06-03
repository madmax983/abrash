//! JPEG Artifact Post-Processing Effect
//!
//! A retro effect that simulates blocky compression artifacts and color ringing
//! characteristic of low-quality JPEG images.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::XorShift32;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the JPEG Artifact effect.
#[derive(Debug, Clone, Copy)]
pub struct JpegArtifactConfig {
    /// The size of the macroblocks (typically 8 for JPEG).
    pub block_size: usize,
    /// Simulates quantization of colors. Lower values mean fewer colors (more banding).
    pub color_levels: u32,
    /// Adds noise around high-contrast areas to simulate ringing.
    pub noise_intensity: i32,
}

impl Default for JpegArtifactConfig {
    fn default() -> Self {
        Self {
            block_size: 8,
            color_levels: 4, // Very compressed
            noise_intensity: 15,
        }
    }
}

/// Applies a JPEG compression artifact effect to the framebuffer.
///
/// This works by:
/// 1. Dividing the image into discrete `NxN` blocks.
/// 2. Downsampling the color in each block.
/// 3. Applying color quantization (banding).
/// 4. Adding localized high-frequency noise.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - The configuration parameters for the artifacting.
pub fn apply_jpeg_artifact(fb: &mut Framebuffer, config: &JpegArtifactConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || config.block_size == 0 || config.color_levels == 0 {
        return;
    }

    let bs = config.block_size;
    let color_levels = config.color_levels.clamp(1, 255);

    // We want to map [0, 255] -> [0, levels-1] -> [0, 255]
    // To do this via integer math:
    // quantized = ((val * (levels - 1) + 127) / 255 * 255) / (levels - 1)
    let levels_minus_1 = color_levels - 1;

    let cols = (width + bs - 1) / bs;

    let row_chunks_len = bs * width;

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let block_row_iter = pixels.par_chunks_mut(row_chunks_len).enumerate();
    #[cfg(not(feature = "parallel"))]
    let block_row_iter = pixels.chunks_mut(row_chunks_len).enumerate();

    block_row_iter.for_each(|(by, block_row_pixels)| {
        let mut prng = XorShift32::new((by as u32 * 1000) ^ 0x1A2B_3C4D);

        // Actual height of this row chunk (may be less than bs at the bottom edge)
        let chunk_height = block_row_pixels.len() / width;

        for bx in 0..cols {
            let start_x = bx * bs;
            let end_x = (start_x + bs).min(width);

            // Calculate the block's average color
            let mut sum_r = 0u32;
            let mut sum_g = 0u32;
            let mut sum_b = 0u32;
            let mut count = 0u32;

            for y in 0..chunk_height {
                let row_offset = y * width;
                for x in start_x..end_x {
                    let color = block_row_pixels[row_offset + x];
                    sum_r += (color >> 16) & 0xFF;
                    sum_g += (color >> 8) & 0xFF;
                    sum_b += color & 0xFF;
                    count += 1;
                }
            }

            if count == 0 {
                continue;
            }

            let mut avg_r = sum_r / count;
            let mut avg_g = sum_g / count;
            let mut avg_b = sum_b / count;

            // Quantize the average block color
            if levels_minus_1 > 0 {
                avg_r = ((avg_r * levels_minus_1 + 127) / 255 * 255) / levels_minus_1;
                avg_g = ((avg_g * levels_minus_1 + 127) / 255 * 255) / levels_minus_1;
                avg_b = ((avg_b * levels_minus_1 + 127) / 255 * 255) / levels_minus_1;
            }

            let block_noise_seed = prng.next_u32();

            // Write the new blocky data back
            for y in 0..chunk_height {
                let row_offset = y * width;
                let abs_y = by * bs + y;
                for x in start_x..end_x {
                    let mut final_r = avg_r as i32;
                    let mut final_g = avg_g as i32;
                    let mut final_b = avg_b as i32;

                    // Add high-frequency noise (ringing)
                    // dependent on position in the block to simulate DCT ringing
                    if config.noise_intensity > 0 {
                        let nx = x % bs;
                        let ny = abs_y % bs;

                        // Fake a cosine-like distribution of error
                        let ring = ((nx ^ ny) as i32 % 3) - 1;
                        let random_factor =
                            (block_noise_seed.wrapping_add((x * abs_y) as u32) % 3) as i32 - 1;

                        let noise = ring * config.noise_intensity
                            + random_factor * (config.noise_intensity / 2);

                        final_r += noise;
                        final_g += noise;
                        final_b += noise;
                    }

                    final_r = final_r.clamp(0, 255);
                    final_g = final_g.clamp(0, 255);
                    final_b = final_b.clamp(0, 255);

                    // Reconstruct color
                    // Retain original alpha (which is usually FF)
                    let orig_alpha = block_row_pixels[row_offset + x] & 0xFF00_0000;
                    block_row_pixels[row_offset + x] = orig_alpha
                        | ((final_r as u32) << 16)
                        | ((final_g as u32) << 8)
                        | (final_b as u32);
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jpeg_artifact_basic() {
        let mut fb = Framebuffer::new(16, 16).unwrap();
        // Clear with a gradient so blocks will average it out
        for y in 0..16 {
            for x in 0..16 {
                let color = 0xFF00_0000 | (x as u32 * 10) << 16;
                fb.set_pixel(x as i32, y as i32, color);
            }
        }

        let config = JpegArtifactConfig {
            block_size: 8,
            color_levels: 255,  // No quantization to isolate block effect
            noise_intensity: 0, // No noise to isolate block effect
        };

        apply_jpeg_artifact(&mut fb, &config);

        // Pixel (0,0) and (7,7) should now be the same color (average of the 8x8 block)
        let p1 = fb.get_pixel(0, 0).unwrap();
        let p2 = fb.get_pixel(7, 7).unwrap();
        assert_eq!(p1, p2, "Pixels in the same block should be averaged");
    }
}
