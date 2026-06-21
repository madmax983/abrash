//! Histogram generation and equalization filter.
//!
//! A module that computes an image's luminance histogram and applies histogram
//! equalization to improve contrast.

use crate::framebuffer::Framebuffer;

/// Computes the luminance histogram of the given framebuffer.
/// Returns an array of 256 integers representing the count of pixels at each luminance level.
#[must_use]
pub fn compute_histogram(fb: &Framebuffer) -> [u32; 256] {
    let mut hist = [0; 256];
    let pixels = fb.as_slice();

    for &pixel in pixels {
        let r = ((pixel >> 16) & 0xFF) as u32;
        let g = ((pixel >> 8) & 0xFF) as u32;
        let b = (pixel & 0xFF) as u32;

        // Fast luminance approximation: L = (R + 2G + B) / 4
        // (Alternatively use standard 0.299R + 0.587G + 0.114B)
        let lum = ((r * 299 + g * 587 + b * 114) / 1000).min(255) as usize;
        hist[lum] += 1;
    }

    hist
}

/// Computes the cumulative distribution function (CDF) of the histogram.
#[must_use]
pub fn compute_cdf(hist: &[u32; 256]) -> [u32; 256] {
    let mut cdf = [0; 256];
    let mut sum = 0;
    for (i, &count) in hist.iter().enumerate() {
        sum += count;
        cdf[i] = sum;
    }
    cdf
}

/// Applies histogram equalization to the framebuffer.
/// This stretches out the luminance range of the image to improve contrast.
pub fn apply_histogram_equalization(fb: &mut Framebuffer) {
    if fb.width() == 0 || fb.height() == 0 {
        return;
    }

    let hist = compute_histogram(fb);
    let cdf = compute_cdf(&hist);
    let total_pixels = fb.width() * fb.height();

    // Find first non-zero CDF value to use for mapping
    let mut cdf_min = 0;
    for &val in &cdf {
        if val > 0 {
            cdf_min = val;
            break;
        }
    }

    // Precompute mapping table
    let mut mapping = [0u8; 256];
    let denominator = total_pixels - cdf_min;
    if denominator > 0 {
        for i in 0..256 {
            let num = (cdf[i] as f32 - cdf_min as f32).max(0.0);
            let val = ((num / denominator as f32) * 255.0).round() as u8;
            mapping[i] = val;
        }
    } else {
        for i in 0..256 {
            mapping[i] = i as u8;
        }
    }

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        pixels.par_iter_mut().for_each(|pixel| {
            let r = ((*pixel >> 16) & 0xFF) as usize;
            let g = ((*pixel >> 8) & 0xFF) as usize;
            let b = (*pixel & 0xFF) as usize;
            let a = *pixel & 0xFF00_0000;

            // Apply mapping directly to each channel to preserve color balance roughly
            // Alternatively, convert to HSV/HSL, map L, and convert back, but this is faster.
            let new_r = u32::from(mapping[r]);
            let new_g = u32::from(mapping[g]);
            let new_b = u32::from(mapping[b]);

            *pixel = a | (new_r << 16) | (new_g << 8) | new_b;
        });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for pixel in pixels.iter_mut() {
            let r = ((*pixel >> 16) & 0xFF) as usize;
            let g = ((*pixel >> 8) & 0xFF) as usize;
            let b = (*pixel & 0xFF) as usize;
            let a = *pixel & 0xFF00_0000;

            let new_r = u32::from(mapping[r]);
            let new_g = u32::from(mapping[g]);
            let new_b = u32::from(mapping[b]);

            *pixel = a | (new_r << 16) | (new_g << 8) | new_b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_histogram() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        fb.set_pixel(0, 0, 0xFF000000); // Black (L = 0)
        fb.set_pixel(1, 0, 0xFFFFFFFF); // White (L = 255)
        fb.set_pixel(0, 1, 0xFF000000); // Black (L = 0)
        fb.set_pixel(1, 1, 0xFF808080); // Gray (L ~ 128)

        let hist = compute_histogram(&fb);
        assert_eq!(hist[0], 2);
        assert_eq!(hist[255], 1);

        let mut gray_count = 0;
        for (i, count) in hist.iter().enumerate() {
            if i > 0 && i < 255 {
                gray_count += count;
            }
        }
        assert_eq!(gray_count, 1);
    }

    #[test]
    fn test_apply_histogram_equalization() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        // A low-contrast image (all pixels between 100 and 150)
        fb.set_pixel(0, 0, 0xFF646464); // rgb(100, 100, 100)
        fb.set_pixel(1, 0, 0xFF969696); // rgb(150, 150, 150)
        fb.set_pixel(0, 1, 0xFF787878); // rgb(120, 120, 120)
        fb.set_pixel(1, 1, 0xFF828282); // rgb(130, 130, 130)

        apply_histogram_equalization(&mut fb);

        let p0 = fb.get_pixel(0, 0).unwrap() & 0xFFFFFF;
        let p1 = fb.get_pixel(1, 0).unwrap() & 0xFFFFFF;

        // After equalization, the minimum value should map to 0 and maximum to 255
        assert_eq!(p0, 0x000000, "Min value should become black");
        assert_eq!(p1, 0xFFFFFF, "Max value should become white");
    }
}
