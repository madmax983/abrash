//! Cel Shading Filter
//!
//! A post-processing effect that creates a "toon" or "cel-shaded" look.
//! It detects edges using a Sobel filter to draw outlines, and quantizes
//! the colors of the remaining pixels to a set number of bands/levels.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Cel Shading effect.
#[derive(Debug, Clone, Copy)]
pub struct CelShaderConfig {
    /// Number of color bands/levels (e.g., 4 or 8).
    pub color_levels: u8,
    /// Threshold for edge detection (outline strength).
    pub edge_threshold: u32,
    /// Color of the outline (usually black).
    pub outline_color: u32,
}

impl Default for CelShaderConfig {
    fn default() -> Self {
        Self {
            color_levels: 4,
            edge_threshold: 50,
            outline_color: 0xFF_00_00_00, // Black
        }
    }
}

/// Applies a cel-shading effect to the framebuffer.
pub fn apply_cel_shader(fb: &mut Framebuffer, config: &CelShaderConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || config.color_levels == 0 {
        return;
    }

    // ⚡ Bolt: Eliminate per-frame heap allocation by using a thread-local static buffer.
    thread_local! {
        static SOURCE_PIXELS: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    let mut src_pixels = SOURCE_PIXELS.with(std::cell::RefCell::take);
    src_pixels.clear();
    src_pixels.extend_from_slice(fb.as_slice());
    let source_buffer = src_pixels.as_slice();

    let pixels = fb.as_mut_slice();
    let levels = f32::from(config.color_levels);

    #[cfg(feature = "parallel")]
    let iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let iter = pixels.chunks_exact_mut(width).enumerate();

    iter.for_each(|(y, row)| {
        for (x, pixel) in row.iter_mut().enumerate() {
            // Bounds check for Sobel 3x3
            if x == 0 || y == 0 || x >= width - 1 || y >= height - 1 {
                // Quantize border pixels without edge detection
                quantize_pixel(pixel, levels);
                continue;
            }

            // --- 1. Edge Detection (Sobel) ---
            let tl = pixel_luminance(source_buffer[(y - 1) * width + (x - 1)]);
            let tc = pixel_luminance(source_buffer[(y - 1) * width + x]);
            let tr = pixel_luminance(source_buffer[(y - 1) * width + (x + 1)]);
            let cl = pixel_luminance(source_buffer[y * width + (x - 1)]);
            let cr = pixel_luminance(source_buffer[y * width + (x + 1)]);
            let bl = pixel_luminance(source_buffer[(y + 1) * width + (x - 1)]);
            let bc = pixel_luminance(source_buffer[(y + 1) * width + x]);
            let br = pixel_luminance(source_buffer[(y + 1) * width + (x + 1)]);

            // Sobel X
            let gx = (i32::from(tr) + 2 * i32::from(cr) + i32::from(br))
                - (i32::from(tl) + 2 * i32::from(cl) + i32::from(bl));

            // Sobel Y
            let gy = (i32::from(bl) + 2 * i32::from(bc) + i32::from(br))
                - (i32::from(tl) + 2 * i32::from(tc) + i32::from(tr));

            // Approximate gradient magnitude
            let magnitude = (gx.abs() + gy.abs()) as u32;

            if magnitude > config.edge_threshold {
                // Draw outline
                *pixel = config.outline_color;
            } else {
                // --- 2. Color Quantization ---
                quantize_pixel(pixel, levels);
            }
        }
    });

    SOURCE_PIXELS.with(|buf| {
        buf.replace(src_pixels);
    });
}

#[inline(always)]
fn quantize_pixel(pixel: &mut u32, levels: f32) {
    let p = *pixel;
    let a = p & 0xFF00_0000;
    let r = ((p >> 16) & 0xFF) as f32;
    let g = ((p >> 8) & 0xFF) as f32;
    let b = (p & 0xFF) as f32;

    let r_quant = (f32::floor((r / 255.0) * levels) / levels * 255.0) as u32;
    let g_quant = (f32::floor((g / 255.0) * levels) / levels * 255.0) as u32;
    let b_quant = (f32::floor((b / 255.0) * levels) / levels * 255.0) as u32;

    *pixel = a | (r_quant << 16) | (g_quant << 8) | b_quant;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cel_shader_config_default() {
        let config = CelShaderConfig::default();
        assert_eq!(config.color_levels, 4);
        assert_eq!(config.outline_color, 0xFF_00_00_00);
    }

    #[test]
    fn test_apply_cel_shader_quantize() {
        let mut fb = Framebuffer::new(3, 3).unwrap();
        // Fill with a mid-tone grey
        fb.clear(0xFF_88_88_88);

        let config = CelShaderConfig {
            color_levels: 4,
            edge_threshold: 200, // High threshold so no edges are detected
            outline_color: 0xFF_00_00_00,
        };

        apply_cel_shader(&mut fb, &config);

        // 0x88 is 136. (136/255) * 4 = 2.13 -> floor(2.13) = 2.0 -> (2.0/4) * 255 = 127 = 0x7F
        // The expected quantized color is 0xFF_7F_7F_7F.
        assert_eq!(fb.get_pixel(1, 1).unwrap(), 0xFF_7F_7F_7F);
    }

    #[test]
    fn test_apply_cel_shader_edge() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        // Draw a solid black square on white background to create sharp edges
        fb.clear(0xFF_FF_FF_FF); // White background
        for y in 3..7 {
            for x in 3..7 {
                fb.set_pixel(x as i32, y as i32, 0xFF_55_55_55); // Dark square
            }
        }

        let config = CelShaderConfig::default();
        apply_cel_shader(&mut fb, &config);

        // Edges of the square should be colored with outline_color
        // (x=3 is the left edge)
        let edge_pixel = fb.get_pixel(3, 3).unwrap();
        assert_eq!(
            edge_pixel, config.outline_color,
            "Edge pixel should be colored as an outline"
        );
    }
}
