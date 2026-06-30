//! Histogram Equalization Post-Processing Effect
//!
//! A post-processing effect that enhances the contrast of an image by stretching
//! the intensity distribution. This is particularly useful for making details
//! more visible in images that are generally too dark or too bright.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Histogram Equalization effect.
#[derive(Debug, Clone, Copy)]
pub struct HistogramEqConfig {
    /// Blend factor between the original image and the fully equalized image.
    /// 0.0 means original image, 1.0 means fully equalized image.
    pub blend: f32,
}

impl Default for HistogramEqConfig {
    fn default() -> Self {
        Self { blend: 1.0 }
    }
}

/// Applies histogram equalization to the framebuffer.
///
/// This effect computes the luminance histogram, calculates the cumulative distribution
/// function (CDF), and maps original pixel colors to their equalized values to improve contrast.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - Configuration for the Histogram Equalization.
pub fn apply_histogram_eq(fb: &mut Framebuffer, config: &HistogramEqConfig) {
    if config.blend <= 0.0 {
        return;
    }

    let pixels = fb.as_mut_slice();
    let num_pixels = pixels.len();

    if num_pixels == 0 {
        return;
    }

    // 1. Calculate luminance histogram
    // We compute the histogram manually since `pixel_luminance` gives a value from 0 to 255.
    let mut histogram = [0u32; 256];

    #[cfg(feature = "parallel")]
    {
        // Parallel histogram calculation using chunks to avoid data races
        // We use thread-local chunks or simple fold-reduce.
        let hist = pixels
            .par_iter()
            .fold(
                || [0u32; 256],
                |mut local_hist, &p| {
                    let lum = pixel_luminance(p);
                    local_hist[lum as usize] += 1;
                    local_hist
                },
            )
            .reduce(
                || [0u32; 256],
                |mut a, b| {
                    for i in 0..256 {
                        a[i] += b[i];
                    }
                    a
                },
            );
        histogram.copy_from_slice(&hist);
    }
    #[cfg(not(feature = "parallel"))]
    {
        for &p in pixels.iter() {
            let lum = pixel_luminance(p);
            histogram[lum as usize] += 1;
        }
    }

    // 2. Compute Cumulative Distribution Function (CDF) and Look-Up Table (LUT)
    let mut cdf = [0u32; 256];
    let mut current_cdf = 0;
    let mut cdf_min = 0;
    for (i, &count) in histogram.iter().enumerate() {
        if count > 0 && cdf_min == 0 {
            cdf_min = current_cdf + count;
        }
        current_cdf += count;
        cdf[i] = current_cdf;
    }

    // To prevent division by zero
    let denominator = num_pixels as u32 - cdf_min;
    if denominator == 0 {
        return;
    }

    let mut lut = [0u8; 256];
    for i in 0..256 {
        if cdf[i] > cdf_min {
            let scaled = ((cdf[i] - cdf_min) * 255) / denominator;
            lut[i] = scaled as u8;
        } else {
            lut[i] = 0;
        }
    }

    // Fixed point arithmetic for blending
    let blend_fixed = (config.blend.clamp(0.0, 1.0) * 256.0) as u32;
    let inv_blend_fixed = 256 - blend_fixed;

    // 3. Apply LUT to pixels
    // To preserve hues, we scale RGB channels proportionally based on luminance change.
    #[cfg(feature = "parallel")]
    let pixel_iter = pixels.par_iter_mut();
    #[cfg(not(feature = "parallel"))]
    let pixel_iter = pixels.iter_mut();

    pixel_iter.for_each(|p| {
        let old_lum = pixel_luminance(*p);
        let new_lum = lut[old_lum as usize];

        // If luminance is perfectly 0, we can't scale proportionally safely, so leave it black
        // or shift it towards white if equalized to > 0.
        let alpha = *p & 0xFF00_0000;
        let r = (*p >> 16) & 0xFF;
        let g = (*p >> 8) & 0xFF;
        let b = *p & 0xFF;

        let (new_r, new_g, new_b) = if old_lum > 0 {
            // scale = new_lum / old_lum
            // Using fixed point logic to avoid float casts in loop
            let scale_fixed = (u32::from(new_lum) * 256) / u32::from(old_lum);
            let nr = ((r * scale_fixed) >> 8).min(255);
            let ng = ((g * scale_fixed) >> 8).min(255);
            let nb = ((b * scale_fixed) >> 8).min(255);
            (nr, ng, nb)
        } else if new_lum > 0 {
            (u32::from(new_lum), u32::from(new_lum), u32::from(new_lum))
        } else {
            (0, 0, 0)
        };

        // Blend between original and new
        let br = (r * inv_blend_fixed + new_r * blend_fixed) >> 8;
        let bg = (g * inv_blend_fixed + new_g * blend_fixed) >> 8;
        let bb = (b * inv_blend_fixed + new_b * blend_fixed) >> 8;

        *p = alpha | (br << 16) | (bg << 8) | bb;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_histogram_eq_improves_contrast() {
        let mut fb = Framebuffer::new(3, 3).unwrap();
        // Fill the rest with an intermediate value to avoid empty histogram issues
        fb.clear(0xFF_0A_0A_0A);

        // A very dark, low contrast image: all values between 10 and 20.
        // Luminance of (10,10,10) is 10.
        fb.set_pixel(0, 0, 0xFF_0A_0A_0A);
        fb.set_pixel(1, 1, 0xFF_0F_0F_0F); // 15
        fb.set_pixel(2, 2, 0xFF_14_14_14); // 20

        let config = HistogramEqConfig::default();
        apply_histogram_eq(&mut fb, &config);

        let p_min = fb.get_pixel(0, 0).unwrap();
        let p_max = fb.get_pixel(2, 2).unwrap();

        // The darkest pixel should map to 0 (since it is the minimum in the CDF)
        assert_eq!(pixel_luminance(p_min), 0);
        // The brightest pixel should map to 255 (the maximum in the CDF)
        assert_eq!(pixel_luminance(p_max), 255);
    }

    #[test]
    fn test_histogram_eq_blend_zero() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        fb.clear(0xFF_12_34_56);
        let config = HistogramEqConfig { blend: 0.0 };
        apply_histogram_eq(&mut fb, &config);

        assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF_12_34_56);
    }
}
