//! Pencil Sketch Filter
//!
//! A post-processing effect that simulates a hand-drawn pencil sketch.
//! It works by combining edge detection (to draw the strokes) with
//! procedural hatching to simulate shading and texture based on luminance.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Pencil Sketch effect.
#[derive(Debug, Clone, Copy)]
pub struct PencilSketchConfig {
    /// Threshold for edge detection.
    pub edge_threshold: u32,
    /// Pencil stroke color (usually black/dark grey).
    pub stroke_color: u32,
    /// Paper color (usually white/off-white).
    pub paper_color: u32,
    /// Intensity of the hatching effect based on luminance (0.0 to 1.0).
    pub hatch_intensity: f32,
}

impl Default for PencilSketchConfig {
    fn default() -> Self {
        Self {
            edge_threshold: 40,
            stroke_color: 0xFF_22_22_22, // Dark charcoal
            paper_color: 0xFF_F0_F0_EA,  // Slightly warm off-white paper
            hatch_intensity: 0.8,
        }
    }
}

/// Applies a pencil sketch effect to the framebuffer.
pub fn apply_pencil_sketch(fb: &mut Framebuffer, config: &PencilSketchConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    // Pre-calculate blended stroke color for hatching outside the loop
    let r_s = (config.stroke_color >> 16) & 0xFF;
    let g_s = (config.stroke_color >> 8) & 0xFF;
    let b_s = config.stroke_color & 0xFF;

    let r_p = (config.paper_color >> 16) & 0xFF;
    let g_p = (config.paper_color >> 8) & 0xFF;
    let b_p = config.paper_color & 0xFF;

    let blended_stroke_color = 0xFF00_0000
        | (u32::midpoint(r_s, r_p) << 16)
        | (u32::midpoint(g_s, g_p) << 8)
        | u32::midpoint(b_s, b_p);

    let hatch_threshold = (config.hatch_intensity * 100.0) as u32;

    // ⚡ Bolt: Eliminate per-frame heap allocation by using a thread-local static buffer.
    thread_local! {
        static SOURCE_PIXELS: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    let mut src_pixels = SOURCE_PIXELS.with(std::cell::RefCell::take);
    src_pixels.clear();
    src_pixels.extend_from_slice(fb.as_slice());
    let source_buffer = src_pixels.as_slice();

    let pixels = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let iter = pixels.chunks_exact_mut(width).enumerate();

    iter.for_each(|(y, row)| {
        for (x, pixel) in row.iter_mut().enumerate() {
            // Bounds check for Sobel 3x3
            if x == 0 || y == 0 || x >= width - 1 || y >= height - 1 {
                *pixel = config.paper_color;
                continue;
            }

            // --- 1. Edge Detection (Sobel) ---
            // Fetch 3x3 neighborhood luminance using unchecked indexing for speed
            let idx = y * width + x;
            let tl = pixel_luminance(unsafe { *source_buffer.get_unchecked(idx - width - 1) });
            let tc = pixel_luminance(unsafe { *source_buffer.get_unchecked(idx - width) });
            let tr = pixel_luminance(unsafe { *source_buffer.get_unchecked(idx - width + 1) });
            let cl = pixel_luminance(unsafe { *source_buffer.get_unchecked(idx - 1) });
            let cr = pixel_luminance(unsafe { *source_buffer.get_unchecked(idx + 1) });
            let bl = pixel_luminance(unsafe { *source_buffer.get_unchecked(idx + width - 1) });
            let bc = pixel_luminance(unsafe { *source_buffer.get_unchecked(idx + width) });
            let br = pixel_luminance(unsafe { *source_buffer.get_unchecked(idx + width + 1) });

            // Sobel X
            let gx = (i32::from(tr) + 2 * i32::from(cr) + i32::from(br))
                - (i32::from(tl) + 2 * i32::from(cl) + i32::from(bl));

            // Sobel Y
            let gy = (i32::from(bl) + 2 * i32::from(bc) + i32::from(br))
                - (i32::from(tl) + 2 * i32::from(tc) + i32::from(tr));

            // Approximate gradient magnitude
            let magnitude = (gx.abs() + gy.abs()) as u32;

            let is_edge = magnitude > config.edge_threshold;

            if is_edge {
                // Draw strong stroke
                *pixel = config.stroke_color;
            } else {
                // --- 2. Tonal Hatching ---
                let luminance = unsafe { pixel_luminance(*source_buffer.get_unchecked(idx)) };

                // Simple procedural hatching pattern
                let mut is_hatch = false;

                // luminance threshold mapping (luminance is 0-255):
                // 0.7 * 255 = 178
                // 0.5 * 255 = 127
                // 0.3 * 255 = 76

                // The darker the region, the more hatching strokes we apply.
                if luminance < 178 {
                    is_hatch |= (x + y) % 4 == 0;
                    if luminance < 127 {
                        is_hatch |= (x.wrapping_sub(y)) % 4 == 0;
                        if luminance < 76 {
                            is_hatch |= x % 3 == 0;
                        }
                    }
                }

                if is_hatch {
                    // Add some noise to hatching
                    // Use a simple pseudo-random hash based on coordinates
                    let hash = ((x * 3_266_489_917) ^ (y * 2_654_435_761)).wrapping_mul(0x85eb_ca6b);

                    if ((hash % 100) as u32) < hatch_threshold {
                        *pixel = blended_stroke_color;
                    } else {
                        *pixel = config.paper_color;
                    }
                } else {
                    *pixel = config.paper_color;
                }
            }
        }
    });

    SOURCE_PIXELS.with(|buf| {
        buf.replace(src_pixels);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pencil_sketch_config_default() {
        let config = PencilSketchConfig::default();
        assert_eq!(config.stroke_color, 0xFF_22_22_22);
        assert_eq!(config.paper_color, 0xFF_F0_F0_EA);
    }

    #[test]
    fn test_apply_pencil_sketch() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        // Draw a solid black square on white background to create sharp edges
        fb.clear(0xFF_FF_FF_FF); // White background
        for y in 3..7 {
            for x in 3..7 {
                fb.set_pixel(x as i32, y as i32, 0xFF_00_00_00); // Black square
            }
        }

        let config = PencilSketchConfig::default();
        apply_pencil_sketch(&mut fb, &config);

        // Edges of the square should be colored with stroke_color
        // (x=3 is the left edge, x=6 is the right edge)
        let edge_pixel = fb.get_pixel(3, 3).unwrap();
        assert_eq!(
            edge_pixel, config.stroke_color,
            "Edge pixel should be colored as a pencil stroke"
        );

        // The background (0,0) should be colored with paper_color
        let bg_pixel = fb.get_pixel(0, 0).unwrap();
        assert_eq!(
            bg_pixel, config.paper_color,
            "Background should be paper color"
        );
    }
}
