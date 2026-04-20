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
/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
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
    let mut quantize_lut = [0u8; 512];
    for i in 0..512 {
        quantize_lut[i] = quantize((i as f32) - 128.0, depth);
    }

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

    // Precompute offset table mapped to integer space
    let mut offset_table = vec![0i32; size * size];
    for i in 0..size * size {
        let bayer_val = f32::from(matrix[i]);
        let offset = (bayer_val - (size * size) as f32 * 0.5) * matrix_scale;
        // ⚡ Bolt: Replace f32::round() with fast integer casting
        // The coordinates are shifted by 16384.0 to ensure they are always positive
        offset_table[i] = ((offset + 16384.5) as i32 as f32 - 16384.0) as i32;
    }

    for (y, row) in pixels.chunks_exact_mut(width).take(height).enumerate() {
        let y_mod = y % size;
        let row_offset_base = y_mod * size;

        for (x, pixel_out) in row.iter_mut().enumerate() {
            let x_mod = x % size;
            let offset_val = offset_table[row_offset_base + x_mod];

            let pixel = *pixel_out;
            let r = ((pixel >> 16) & 0xFF) as i32;
            let g = ((pixel >> 8) & 0xFF) as i32;
            let b = (pixel & 0xFF) as i32;

            // Offset by 128 to match LUT mapping
            let r_new = quantize_lut[(r + offset_val + 128).clamp(0, 511) as usize];
            let g_new = quantize_lut[(g + offset_val + 128).clamp(0, 511) as usize];
            let b_new = quantize_lut[(b + offset_val + 128).clamp(0, 511) as usize];

            *pixel_out = (pixel & 0xFF00_0000)
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

    if width == 0 || height == 0 {
        return;
    }

    let mut quantize_lut = [0u8; 512];
    for i in 0..512 {
        quantize_lut[i] = quantize((i as f32) - 128.0, depth);
    }

    // Only allocate two rows instead of the entire image
    let mut curr_row = vec![0.0f32; width * 3];
    let mut next_row = vec![0.0f32; width * 3];

    // Initialize first row
    for x in 0..width {
        let p = pixels[x];
        curr_row[x * 3] = ((p >> 16) & 0xFF) as f32;
        curr_row[x * 3 + 1] = ((p >> 8) & 0xFF) as f32;
        curr_row[x * 3 + 2] = (p & 0xFF) as f32;
    }

    for y in 0..height {
        // Initialize next row if it exists
        if y + 1 < height {
            let row_offset = (y + 1) * width;
            for x in 0..width {
                let p = pixels[row_offset + x];
                next_row[x * 3] = ((p >> 16) & 0xFF) as f32;
                next_row[x * 3 + 1] = ((p >> 8) & 0xFF) as f32;
                next_row[x * 3 + 2] = (p & 0xFF) as f32;
            }
        }

        let row_offset = y * width;
        for x in 0..width {
            let idx = x * 3;
            let r_old = curr_row[idx];
            let g_old = curr_row[idx + 1];
            let b_old = curr_row[idx + 2];

            // Offset by 128 to match LUT mapping
            // ⚡ Bolt: Replace f32::round() with fast integer casting
            let r_new = quantize_lut[(((r_old + 16384.5) as i32 as f32 - 16384.0) as i32 + 128).clamp(0, 511) as usize];
            let g_new = quantize_lut[(((g_old + 16384.5) as i32 as f32 - 16384.0) as i32 + 128).clamp(0, 511) as usize];
            let b_new = quantize_lut[(((b_old + 16384.5) as i32 as f32 - 16384.0) as i32 + 128).clamp(0, 511) as usize];

            let p_idx = row_offset + x;
            pixels[p_idx] = (pixels[p_idx] & 0xFF00_0000)
                | (u32::from(r_new) << 16)
                | (u32::from(g_new) << 8)
                | u32::from(b_new);

            let r_err = r_old - f32::from(r_new);
            let g_err = g_old - f32::from(g_new);
            let b_err = b_old - f32::from(b_new);

            if x + 1 < width {
                add_error(&mut curr_row, idx + 3, r_err, g_err, b_err, 7.0 / 16.0);
            }
            if y + 1 < height {
                if x > 0 {
                    add_error(&mut next_row, idx - 3, r_err, g_err, b_err, 3.0 / 16.0);
                }
                add_error(&mut next_row, idx, r_err, g_err, b_err, 5.0 / 16.0);
                if x + 1 < width {
                    add_error(&mut next_row, idx + 3, r_err, g_err, b_err, 1.0 / 16.0);
                }
            }
        }
        std::mem::swap(&mut curr_row, &mut next_row);
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

    // ⚡ Bolt: Replace f32::round() with fast integer casting
    let level = ((val / step) + 16384.5) as i32 as f32 - 16384.0;
    (((level * step) + 16384.5) as i32 as f32 - 16384.0) as u8
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

    #[test]
    fn test_apply_ordered_dither_exact() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        // Constant gray value slightly above halfway point
        fb.clear(0xFF_8C_8C_8C); // 140

        // Depth 1 means thresholds at 0 and 255.
        let config = DitherConfig {
            mode: DitherMode::Ordered2x2,
            color_depth: 1,
        };

        apply_dither(&mut fb, config);

        // BAYER_2X2 is:
        // [ 0, 2 ]
        // [ 3, 1 ]
        // With step = 255.0. Size = 2.
        // matrix_scale = 255.0 / 4.0 = 63.75
        //
        // Offsets:
        // (0 - 2.0) * 63.75 = -127.5
        // (2 - 2.0) * 63.75 = 0.0
        // (3 - 2.0) * 63.75 = 63.75
        // (1 - 2.0) * 63.75 = -63.75
        //
        // Input: 140
        //
        // Pixel (0,0): val=0 -> offset = -128. Input+offset = 12. Quantize(12) = 0
        // Pixel (1,0): val=2 -> offset = 0. Input+offset = 140. Quantize(140) = 255
        // Pixel (0,1): val=3 -> offset = 64. Input+offset = 204. Quantize(204) = 255
        // Pixel (1,1): val=1 -> offset = -64. Input+offset = 76. Quantize(76) = 0

        assert_eq!(fb.get_pixel(0, 0).unwrap() & 0xFF, 0);
        assert_eq!(fb.get_pixel(1, 0).unwrap() & 0xFF, 255);
        assert_eq!(fb.get_pixel(0, 1).unwrap() & 0xFF, 255);
        assert_eq!(fb.get_pixel(1, 1).unwrap() & 0xFF, 0);
    }

    #[test]
    fn test_apply_floyd_steinberg_exact() {
        let mut fb = Framebuffer::new(3, 2).unwrap();
        // Start with a mid-gray image
        fb.clear(0xFF_80_80_80); // 128

        let config = DitherConfig {
            mode: DitherMode::FloydSteinberg,
            color_depth: 1,
        };

        apply_dither(&mut fb, config);

        // Top Left (0, 0)
        // input 128 -> quantize -> 255
        // Error = 128 - 255 = -127
        // (1, 0) gets + 7/16 * (-127) = -55.56
        // -> input becomes 128 - 55.56 = 72.44
        // (0, 1) gets + 5/16 * (-127) = -39.68
        // (1, 1) gets + 1/16 * (-127) = -7.93

        // (0, 0) is white
        assert_eq!(fb.get_pixel(0, 0).unwrap() & 0xFF, 255);

        // Next pixel (1, 0) is 72.44 -> quantize -> 0
        // Error = 72.44 - 0 = 72.44
        assert_eq!(fb.get_pixel(1, 0).unwrap() & 0xFF, 0);

        // (0, 1) had base 128, got -39.68 from (0,0) and +3/16*(72.44) from (1,0)
        // 128 - 39.68 + 13.58 = 101.9 -> quantize -> 0
        assert_eq!(fb.get_pixel(0, 1).unwrap() & 0xFF, 0);
    }
}
