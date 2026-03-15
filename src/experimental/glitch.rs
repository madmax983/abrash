//! Glitch Effect Module
//!
//! Simulates digital signal corruption, tracking errors, and chromatic aberration.
//! This effect is designed for "cyberpunk" or "retro-future" aesthetics.
//!
//! # Features
//! * **RGB Split**: Chromatic aberration with random jitter.
//! * **Scanline Jitter**: Horizontal displacement of random scanlines.
//! * **Digital Noise**: Random pixel variations.
//! * **Block Displacement**: Random rectangular regions swapped or copied.

use crate::framebuffer::Framebuffer;
use crate::utils::XorShift32;

/// Parameters for controlling the glitch effect intensity.
#[derive(Debug, Clone, Copy)]
pub struct GlitchParams {
    /// Global intensity multiplier (0.0 to 1.0).
    pub intensity: f32,
    /// Seed for random number generation (e.g., frame count or time).
    pub seed_time: u32,
    /// Maximum horizontal shift for RGB channels (pixels).
    pub color_shift_amount: u32,
    /// Maximum horizontal jitter for scanlines (pixels).
    pub jitter_amount: u32,
    /// Probability of a block displacement occurring (0.0 to 1.0).
    pub block_displacement_prob: f32,
}

impl Default for GlitchParams {
    fn default() -> Self {
        Self {
            intensity: 0.5,
            seed_time: 0,
            color_shift_amount: 5,
            jitter_amount: 10,
            block_displacement_prob: 0.1,
        }
    }
}

/// Applies the glitch effect to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The target framebuffer to modify in-place.
/// * `params` - Configuration for the effect.
pub fn apply_glitch(fb: &mut Framebuffer, params: &GlitchParams) {
    if params.intensity <= 0.001 {
        return;
    }

    let mut rng = XorShift32::new(params.seed_time);
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let pixels = fb.as_mut_slice();

    // 1. Scanline Jitter (Horizontal Shift)
    // We do this first so color shifts are applied on top of jittered geometry?
    // Or after? Doing it first makes sense as "sync loss".
    if params.jitter_amount > 0 {
        apply_scanline_jitter(pixels, width, height, params, &mut rng);
    }

    // 2. RGB Split (Chromatic Aberration)
    if params.color_shift_amount > 0 {
        apply_rgb_split(pixels, width, height, params, &mut rng);
    }

    // 3. Block Displacement
    if params.block_displacement_prob > 0.0 && rng.next_f32() < params.block_displacement_prob {
        apply_block_displacement(pixels, width, height, params, &mut rng);
    }

    // 4. Digital Noise
    // Only apply sparsely based on intensity
    if params.intensity > 0.0 {
        apply_digital_noise(pixels, width, height, params, &mut rng);
    }
}

fn apply_scanline_jitter(
    pixels: &mut [u32],
    width: usize,
    height: usize,
    params: &GlitchParams,
    rng: &mut XorShift32,
) {
    // Determine how many lines to jitter based on intensity
    // Ensure at least 1 line if intensity is significant
    // Use max(3) to ensure we hit enough lines to be visible in tests/gameplay
    let num_jitter_lines = (height as f32 * params.intensity * 0.5).max(3.0) as usize;

    for _ in 0..num_jitter_lines {
        let y = (rng.next_u32() as usize) % height;
        let shift = (rng.next_f32_signed() * params.jitter_amount as f32 * params.intensity) as i32;

        if shift == 0 {
            continue;
        }

        let row_start = y * width;
        let row_end = row_start + width;
        let row = &mut pixels[row_start..row_end];

        // Create a temporary buffer for the row
        // Allocating per line is slow, but acceptable for experimental feature.
        // Optimization: Use a thread-local scratch buffer if this becomes hot path.
        let mut temp_row = vec![0u32; width];
        temp_row.copy_from_slice(row);

        for (x, item) in row.iter_mut().enumerate().take(width) {
            let src_x = (x as i32 - shift).clamp(0, (width - 1) as i32) as usize;
            *item = temp_row[src_x];
        }
    }
}

fn apply_rgb_split(
    pixels: &mut [u32],
    width: usize,
    height: usize,
    params: &GlitchParams,
    rng: &mut XorShift32,
) {
    // Only apply to random bands of the screen for "glitchy" feel
    let num_bands = (rng.next_u32() % 5) + 1;

    for _ in 0..num_bands {
        let band_height = (rng.next_u32() % (height as u32 / 4)) as usize;
        let start_y = (rng.next_u32() as usize) % (height.saturating_sub(band_height).max(1));

        let shift_r =
            (rng.next_f32_signed() * params.color_shift_amount as f32 * params.intensity) as i32;
        let shift_b =
            (rng.next_f32_signed() * params.color_shift_amount as f32 * params.intensity) as i32;

        if shift_r == 0 && shift_b == 0 {
            continue;
        }

        // We process the band row by row
        for y in start_y..(start_y + band_height).min(height) {
            let row_start = y * width;
            let row_end = row_start + width;
            let row = &mut pixels[row_start..row_end];

            // Allocation again - acceptable for prototype
            let mut temp_row = vec![0u32; width];
            temp_row.copy_from_slice(row);

            for x in 0..width {
                // Original Green/Alpha stays at x
                let g = (temp_row[x] >> 8) & 0xFF;
                let a = (temp_row[x] >> 24) & 0xFF;

                // Red from shifted position
                let src_r_x = (x as i32 - shift_r).clamp(0, (width - 1) as i32) as usize;
                let r = (temp_row[src_r_x] >> 16) & 0xFF;

                // Blue from shifted position
                let src_b_x = (x as i32 - shift_b).clamp(0, (width - 1) as i32) as usize;
                let b = temp_row[src_b_x] & 0xFF;

                row[x] = (a << 24) | (r << 16) | (g << 8) | b;
            }
        }
    }
}

fn apply_block_displacement(
    pixels: &mut [u32],
    width: usize,
    height: usize,
    params: &GlitchParams,
    rng: &mut XorShift32,
) {
    // Size of block
    let block_w = (width as f32 * 0.1 * params.intensity).max(4.0) as usize;
    let block_h = (height as f32 * 0.1 * params.intensity).max(4.0) as usize;

    // Ensure block fits in buffer (though glitchy, we shouldn't panic)
    let block_w = block_w.min(width);
    let block_h = block_h.min(height);

    // Source position
    let src_x = (rng.next_u32() as usize) % (width.saturating_sub(block_w).max(1));
    let src_y = (rng.next_u32() as usize) % (height.saturating_sub(block_h).max(1));

    // Dest position (shifted)
    let shift_x = (rng.next_f32_signed() * 20.0 * params.intensity) as i32;
    let shift_y = (rng.next_f32_signed() * 20.0 * params.intensity) as i32;

    let dst_x = (src_x as i32 + shift_x).clamp(0, (width - block_w) as i32) as usize;
    let dst_y = (src_y as i32 + shift_y).clamp(0, (height - block_h) as i32) as usize;

    // Copy block
    // We need to buffer the source block first because src and dst might overlap
    let mut block_buffer = Vec::with_capacity(block_w * block_h);
    for dy in 0..block_h {
        for dx in 0..block_w {
            let idx = (src_y + dy) * width + (src_x + dx);
            block_buffer.push(pixels[idx]);
        }
    }

    // Write to dest
    for dy in 0..block_h {
        for dx in 0..block_w {
            let idx = (dst_y + dy) * width + (dst_x + dx);
            pixels[idx] = block_buffer[dy * block_w + dx];
        }
    }
}

fn apply_digital_noise(
    pixels: &mut [u32],
    width: usize,
    height: usize,
    params: &GlitchParams,
    rng: &mut XorShift32,
) {
    // Probability of a noise pixel
    let noise_prob = 0.05 * params.intensity;
    // Number of pixels to noise
    let num_noise = ((width * height) as f32 * noise_prob) as usize;

    for _ in 0..num_noise {
        let idx = (rng.next_u32() as usize) % pixels.len();

        // Random color noise or brightness noise?
        // Let's do brightness inversion or color tint
        let p = pixels[idx];
        let mode = rng.next_u32() % 3;

        pixels[idx] = match mode {
            0 => p ^ 0x00FF_FFFF, // Invert color
            1 => p | 0x00FF_0000, // Red tint
            2 => p & 0xFF00_FF00, // Mask out Red and Blue (Green only)
            _ => p,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glitch_no_op() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFFFFFFFF);
        let original = fb.as_slice().to_vec();

        let params = GlitchParams {
            intensity: 0.0,
            ..Default::default()
        };

        apply_glitch(&mut fb, &params);

        assert_eq!(fb.as_slice(), original.as_slice());
    }

    #[test]
    fn test_scanline_jitter() {
        let mut fb = Framebuffer::new(20, 20).unwrap();
        fb.clear(0xFFFFFFFF); // White Background
        // Draw vertical stripe
        for y in 0..20 {
            fb.set_pixel(10, y, 0xFF000000); // Black line at x=10
        }

        let params = GlitchParams {
            intensity: 1.0,
            jitter_amount: 20, // Enough to displace
            seed_time: 12345,
            color_shift_amount: 0,
            block_displacement_prob: 0.0,
        };

        apply_glitch(&mut fb, &params);

        // Check if the line is still perfectly vertical
        let mut perfect = true;
        for y in 0..20 {
            if let Some(p) = fb.get_pixel(10, y)
                && p != 0xFF000000
            {
                perfect = false;
                break;
            }
        }

        // With high intensity and jitter, it should NOT be perfect
        assert!(!perfect, "Scanline jitter failed to displace pixels");
    }

    #[test]
    fn test_rgb_split() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFFFFFFFF); // White

        let params = GlitchParams {
            intensity: 1.0,
            jitter_amount: 0,
            seed_time: 12345,
            color_shift_amount: 5,
            block_displacement_prob: 0.0,
        };

        apply_glitch(&mut fb, &params);

        // Check for color fringing (pixels that are no longer white/grayscale)
        let mut _fringing = false;
        for p in fb.as_slice() {
            let r = (p >> 16) & 0xFF;
            let g = (p >> 8) & 0xFF;
            let b = p & 0xFF;

            if r != g || g != b {
                _fringing = true;
                break;
            }
        }

        // Note: RGB split might shift white into white if shift is 0 or if neighbors are white.
        // But with white background, shifting white onto white is still white.
        // Let's use a pattern.

        fb.clear(0xFF000000); // Black
        // White square in middle
        for y in 4..6 {
            for x in 4..6 {
                fb.set_pixel(x, y, 0xFFFFFFFF);
            }
        }

        apply_glitch(&mut fb, &params);

        let mut found_color = false;
        for p in fb.as_slice() {
            let r = (p >> 16) & 0xFF;
            let g = (p >> 8) & 0xFF;
            let b = p & 0xFF;

            // Look for pure Red, Green or Blue pixels (or mixed but not white)
            // e.g. Cyan (G+B) means Red moved away.
            if (r != g || g != b) && *p != 0xFF000000 {
                found_color = true;
                break;
            }
        }

        assert!(found_color, "RGB split failed to produce color fringing");
    }
}
