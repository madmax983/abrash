//! Dithering algorithms for retro aesthetics.
//!
//! Provides Ordered Dithering (Bayer) and Floyd-Steinberg error diffusion
//! to reduce color depth while preserving visual detail.

use crate::framebuffer::Framebuffer;

/// Dithering algorithm to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DitherMode {
    /// Ordered Dithering with 2x2 Bayer matrix.
    Ordered2x2,
    /// Ordered Dithering with 4x4 Bayer matrix.
    Ordered4x4,
    /// Ordered Dithering with 8x8 Bayer matrix.
    Ordered8x8,
    /// Floyd-Steinberg Error Diffusion.
    FloydSteinberg,
}

/// Configuration for the dithering effect.
#[derive(Debug, Clone, Copy)]
pub struct DitherConfig {
    /// The algorithm to use.
    pub mode: DitherMode,
    /// Target bits per channel (1-8).
    /// e.g., 1 bit = 2 levels (0, 255).
    ///       2 bits = 4 levels (0, 85, 170, 255).
    pub color_depth: u8,
}

impl Default for DitherConfig {
    fn default() -> Self {
        Self {
            mode: DitherMode::Ordered4x4,
            color_depth: 1, // 1-bit per channel (8 colors total)
        }
    }
}

/// Applies dithering to the framebuffer in-place.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify.
/// * `config` - Dithering configuration.
pub fn apply_dither(fb: &mut Framebuffer, config: DitherConfig) {
    match config.mode {
        DitherMode::Ordered2x2 => apply_ordered_dither(fb, config.color_depth, &BAYER_2X2, 2),
        DitherMode::Ordered4x4 => apply_ordered_dither(fb, config.color_depth, &BAYER_4X4, 4),
        DitherMode::Ordered8x8 => apply_ordered_dither(fb, config.color_depth, &BAYER_8X8, 8),
        DitherMode::FloydSteinberg => apply_floyd_steinberg(fb, config.color_depth),
    }
}

// Bayer Matrices (Normalized 0..Size*Size-1)

#[rustfmt::skip]
const BAYER_2X2: [u8; 4] = [
    0, 2,
    3, 1
];

#[rustfmt::skip]
const BAYER_4X4: [u8; 16] = [
    0, 8, 2, 10,
    12, 4, 14, 6,
    3, 11, 1, 9,
    15, 7, 13, 5
];

#[rustfmt::skip]
const BAYER_8X8: [u8; 64] = [
    0, 32, 8, 40, 2, 34, 10, 42,
    48, 16, 56, 24, 50, 18, 58, 26,
    12, 44, 4, 36, 14, 46, 6, 38,
    60, 28, 52, 20, 62, 30, 54, 22,
    3, 35, 11, 43, 1, 33, 9, 41,
    51, 19, 59, 27, 49, 17, 57, 25,
    15, 47, 7, 39, 13, 45, 5, 37,
    63, 31, 55, 23, 61, 29, 53, 21
];

fn apply_ordered_dither(fb: &mut Framebuffer, depth: u8, matrix: &[u8], size: usize) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let pixels = fb.as_mut_slice();

    let levels = (1 << depth) - 1;
    let step = 255.0 / levels as f32;
    // Scale factor for the Bayer matrix to match the step size
    // matrix value M in [0, N^2-1].
    // Normalized: M / N^2 - 0.5 (range -0.5 to 0.5 approx)
    // Offset = Normalized * step
    let matrix_scale = step / (size * size) as f32;

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let pixel = pixels[idx];

            let bayer_val = f32::from(matrix[(y % size) * size + (x % size)]);
            // Center the dither around 0 (-0.5 to 0.5 range of step)
            // Actually, standard formula: val + (bayer/max * step) - (step/2)
            // Simplified: val + scale * (bayer - limit/2)
            let offset = (bayer_val - (size * size) as f32 * 0.5) * matrix_scale;

            let r = ((pixel >> 16) & 0xFF) as f32;
            let g = ((pixel >> 8) & 0xFF) as f32;
            let b = (pixel & 0xFF) as f32;

            let r_new = quantize(r + offset, depth);
            let g_new = quantize(g + offset, depth);
            let b_new = quantize(b + offset, depth);

            pixels[idx] = (pixel & 0xFF00_0000)
                | (u32::from(r_new) << 16)
                | (u32::from(g_new) << 8)
                | u32::from(b_new);
        }
    }
}

fn apply_floyd_steinberg(fb: &mut Framebuffer, depth: u8) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let pixels = fb.as_mut_slice();

    // We need to store errors as floats to accumulate properly?
    // Or just work on pixels directly?
    // Working on pixels directly is tricky because of clamping.
    // Standard approach: use a temporary buffer of floats/i16 for the current and next row.
    // Or just modify pixels and accept some clamping error.
    // Let's use a float buffer for better quality.
    let mut buffer: Vec<f32> = Vec::with_capacity(width * height * 3);

    // Initial fill
    for p in pixels.iter() {
        buffer.push(((p >> 16) & 0xFF) as f32);
        buffer.push(((p >> 8) & 0xFF) as f32);
        buffer.push((p & 0xFF) as f32);
    }

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 3;
            let r_old = buffer[idx];
            let g_old = buffer[idx + 1];
            let b_old = buffer[idx + 2];

            let r_new = f32::from(quantize(r_old, depth));
            let g_new = f32::from(quantize(g_old, depth));
            let b_new = f32::from(quantize(b_old, depth));

            // Write back quantized pixel immediately
            let p_idx = y * width + x;
            pixels[p_idx] = (pixels[p_idx] & 0xFF00_0000)
                | ((r_new as u32) << 16)
                | ((g_new as u32) << 8)
                | (b_new as u32);

            let r_err = r_old - r_new;
            let g_err = g_old - g_new;
            let b_err = b_old - b_new;

            // Distribute error
            // right (+1, 0): 7/16
            if x + 1 < width {
                let n_idx = (y * width + (x + 1)) * 3;
                add_error(&mut buffer, n_idx, r_err, g_err, b_err, 7.0 / 16.0);
            }
            // down-left (-1, +1): 3/16
            if x > 0 && y + 1 < height {
                let n_idx = ((y + 1) * width + (x - 1)) * 3;
                add_error(&mut buffer, n_idx, r_err, g_err, b_err, 3.0 / 16.0);
            }
            // down (0, +1): 5/16
            if y + 1 < height {
                let n_idx = ((y + 1) * width + x) * 3;
                add_error(&mut buffer, n_idx, r_err, g_err, b_err, 5.0 / 16.0);
            }
            // down-right (+1, +1): 1/16
            if x + 1 < width && y + 1 < height {
                let n_idx = ((y + 1) * width + (x + 1)) * 3;
                add_error(&mut buffer, n_idx, r_err, g_err, b_err, 1.0 / 16.0);
            }
        }
    }
}

fn add_error(buffer: &mut [f32], idx: usize, r_err: f32, g_err: f32, b_err: f32, factor: f32) {
    buffer[idx] += r_err * factor;
    buffer[idx + 1] += g_err * factor;
    buffer[idx + 2] += b_err * factor;
}

/// Quantizes a value to the specified bit depth.
fn quantize(val: f32, depth: u8) -> u8 {
    let val = val.clamp(0.0, 255.0);
    let levels = (1 << depth) - 1;
    let step = 255.0 / levels as f32;

    let level = (val / step).round();
    (level * step).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantize() {
        // 1-bit depth: 0 or 255
        assert_eq!(quantize(0.0, 1), 0);
        assert_eq!(quantize(255.0, 1), 255);
        assert_eq!(quantize(100.0, 1), 0); // Closer to 0? 100/255 = 0.39 -> 0
        assert_eq!(quantize(200.0, 1), 255); // Closer to 255? 200/255 = 0.78 -> 1

        // 2-bit depth: 0, 85, 170, 255
        // Step = 85.
        assert_eq!(quantize(40.0, 2), 0); // 40/85 = 0.47 -> 0
        assert_eq!(quantize(50.0, 2), 85); // 50/85 = 0.58 -> 1 -> 85
        // Wait, 1.49 rounds to 1. 1.5 rounds to 2.
        // 127 is closer to 170? |127-170|=43. |127-85|=42.
        // 42 < 43. So it should be 85.
        assert_eq!(quantize(127.0, 2), 85); // Let's check logic.
        // 127 / 85 = 1.494. Round -> 1. 1*85 = 85. Correct.
    }

    #[test]
    fn test_ordered_dither_changes_pixels() {
        let mut fb = Framebuffer::new(4, 4).unwrap();
        // Set to mid-gray
        fb.clear(0xFF808080);

        let config = DitherConfig {
            mode: DitherMode::Ordered2x2,
            color_depth: 1,
        };

        apply_dither(&mut fb, config);

        // Check that we have variation
        let mut has_white = false;
        let mut has_black = false;

        for y in 0..4 {
            for x in 0..4 {
                let p = fb.get_pixel(x, y).unwrap();
                let r = (p >> 16) & 0xFF;
                if r > 200 {
                    has_white = true;
                }
                if r < 50 {
                    has_black = true;
                }
            }
        }

        assert!(has_white, "Should have white pixels");
        assert!(has_black, "Should have black pixels");
    }

    #[test]
    fn test_floyd_steinberg_smooth() {
        let width = 10;
        let height = 2;
        let mut fb = Framebuffer::new(width, height).unwrap();
        // Gradient
        for x in 0..width {
            let val = (x as f32 / width as f32 * 255.0) as u32;
            let color = 0xFF000000 | (val << 16) | (val << 8) | val;
            fb.set_pixel(x as i32, 0, color);
            fb.set_pixel(x as i32, 1, color);
        }

        let config = DitherConfig {
            mode: DitherMode::FloydSteinberg,
            color_depth: 1,
        };

        apply_dither(&mut fb, config);

        // Gradient should be dithered
        let p0 = fb.get_pixel(0, 0).unwrap() & 0xFF;
        let p9 = fb.get_pixel(9, 0).unwrap() & 0xFF;

        assert_eq!(p0, 0, "Start should be black");
        assert_eq!(p9, 255, "End should be white");

        // Middle should be mixed
        let _p5 = fb.get_pixel(5, 0).unwrap() & 0xFF;
        // Could be either, but FS usually produces patterns.
        // Just checking it runs without panic.
    }
}
