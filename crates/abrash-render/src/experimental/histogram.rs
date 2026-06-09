//! Histogram Overlay Post-Processing Filter
//!
//! Analyzes the framebuffer's pixel luminance distribution and renders a
//! semi-transparent histogram overlay on top of the image.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

/// Configuration for the Histogram Overlay effect.
#[derive(Debug, Clone, Copy)]
pub struct HistogramConfig {
    /// X coordinate of the top-left corner of the histogram overlay.
    pub x: usize,
    /// Y coordinate of the top-left corner of the histogram overlay.
    pub y: usize,
    /// Width of the histogram overlay in pixels.
    pub width: usize,
    /// Height of the histogram overlay in pixels.
    pub height: usize,
    /// Color of the histogram graph bars (ARGB).
    pub color: u32,
    /// Background color of the histogram bounding box (ARGB, typically semi-transparent).
    pub bg_color: u32,
}

impl Default for HistogramConfig {
    fn default() -> Self {
        Self {
            x: 10,
            y: 10,
            width: 256,
            height: 100,
            color: 0xFF_00FF00,    // Solid green
            bg_color: 0x80_000000, // 50% transparent black
        }
    }
}

/// Computes the luminance histogram of the framebuffer and draws it as an overlay.
///
/// # Arguments
///
/// * `fb` - The framebuffer to analyze and draw over.
/// * `config` - Configuration defining the position, size, and styling of the overlay.
pub fn apply_histogram_overlay(fb: &mut Framebuffer, config: &HistogramConfig) {
    if config.width == 0 || config.height == 0 || config.width > 1024 {
        return;
    }

    let mut buckets = vec![0usize; config.width];
    let pixels = fb.as_slice();

    if pixels.is_empty() {
        return;
    }

    // Pass 1: Compute histogram
    let mut max_count = 0usize;
    for &pixel in pixels {
        let luma = pixel_luminance(pixel);
        // luma is 0..255. Map this to 0..config.width
        let bucket_idx = ((luma as usize * config.width) / 256).min(config.width - 1);
        buckets[bucket_idx] += 1;
        if buckets[bucket_idx] > max_count {
            max_count = buckets[bucket_idx];
        }
    }

    if max_count == 0 {
        return; // nothing to draw
    }

    // Pass 2: Draw overlay
    let fb_width = fb.width() as usize;
    let fb_height = fb.height() as usize;

    let start_x = config.x;
    let end_x = (config.x + config.width).min(fb_width);
    let start_y = config.y;
    let end_y = (config.y + config.height).min(fb_height);

    for y in start_y..end_y {
        for x in start_x..end_x {
            let bucket_idx = x - config.x;

            // Invert Y so 0 is at the bottom of the graph
            let graph_y = config.height - 1 - (y - config.y);

            let count = buckets[bucket_idx];
            let bar_height = (count * config.height) / max_count;

            let final_color = if graph_y < bar_height {
                config.color
            } else {
                config.bg_color
            };

            // Alpha blending (assuming source is solid or simple blending)
            let current_pixel = fb.get_pixel(x as i32, y as i32).unwrap_or(0);
            let blended_color = blend_colors(current_pixel, final_color);

            fb.set_pixel(x as i32, y as i32, blended_color);
        }
    }
}

/// Simple alpha blending helper
const fn blend_colors(bg: u32, fg: u32) -> u32 {
    let alpha = (fg >> 24) & 0xFF;
    if alpha == 0 {
        return bg;
    }
    if alpha == 255 {
        return fg;
    }
    let inv_alpha = 255 - alpha;

    let bg_r = (bg >> 16) & 0xFF;
    let bg_g = (bg >> 8) & 0xFF;
    let bg_b = bg & 0xFF;

    let fg_r = (fg >> 16) & 0xFF;
    let fg_g = (fg >> 8) & 0xFF;
    let fg_b = fg & 0xFF;

    let r = (fg_r * alpha + bg_r * inv_alpha) / 255;
    let g = (fg_g * alpha + bg_g * inv_alpha) / 255;
    let b = (fg_b * alpha + bg_b * inv_alpha) / 255;

    0xFF_000000 | (r << 16) | (g << 8) | b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_histogram_overlay() {
        let mut fb = Framebuffer::new(300, 200).unwrap();
        // Fill half with white, half with black
        for y in 0..200 {
            for x in 0..300 {
                if x < 150 {
                    fb.set_pixel(x as i32, y as i32, 0xFF_FFFFFF); // White
                } else {
                    fb.set_pixel(x as i32, y as i32, 0xFF_000000); // Black
                }
            }
        }

        let config = HistogramConfig {
            x: 10,
            y: 10,
            width: 256,
            height: 100,
            color: 0xFF_00FF00, // Solid green
            bg_color: 0xFF_FF0000, // Solid red background for easy assertion
        };
        apply_histogram_overlay(&mut fb, &config);

        // At x=10, y=10 (top-left of overlay, meaning graph_y is near 100).
        // Since the image is half white (luma 255) and half black (luma 0),
        // bucket 0 and bucket 255 will be at max_count, so bar_height=100.
        // Wait, for bucket 0 (at x=10), bar_height=100, so graph_y (99) < bar_height (100) -> color green
        let top_left = fb.get_pixel(config.x as i32, config.y as i32).unwrap();
        assert_eq!(top_left, config.color, "Graph bar should be drawn at bucket 0");

        // At bucket 128 (x = 10 + 128 = 138), the count is 0.
        // So graph_y (99) >= bar_height (0) -> color red
        let mid_top = fb.get_pixel(138, 10).unwrap();
        assert_eq!(mid_top, config.bg_color, "Background should be drawn in empty bucket");
    }
}
