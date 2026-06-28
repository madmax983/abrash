//! Quadtree Image Segmentation Filter
//!
//! A procedural post-processing effect that recursively subdivides the image into
//! quadrants based on color variance. High variance areas (details/edges) are divided
//! into smaller blocks, while low variance areas (flat colors) remain as large blocks.
//! This creates a blocky, stylized, compression-artifact-like aesthetic.

use crate::framebuffer::Framebuffer;

/// Configuration for the Quadtree filter.
#[derive(Debug, Clone, Copy)]
pub struct QuadtreeConfig {
    /// The minimum block size (e.g., 4 or 8 pixels) to stop subdividing.
    pub min_block_size: usize,
    /// The variance threshold above which a block will be subdivided (0.0 to 1.0).
    pub variance_threshold: f32,
    /// Whether to draw borders around the blocks.
    pub draw_borders: bool,
    /// The color of the borders (ARGB).
    pub border_color: u32,
}

impl Default for QuadtreeConfig {
    fn default() -> Self {
        Self {
            min_block_size: 8,
            variance_threshold: 0.05,
            draw_borders: true,
            border_color: 0xFF_000000,
        }
    }
}

#[derive(Clone, Copy)]
struct Rect {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

fn compute_avg_and_variance(fb_slice: &[u32], fb_w: usize, rect: &Rect) -> (u32, f32) {
    let mut sum_r = 0u64;
    let mut sum_g = 0u64;
    let mut sum_b = 0u64;

    let mut sum_sq_r = 0u64;
    let mut sum_sq_g = 0u64;
    let mut sum_sq_b = 0u64;

    let mut count = 0u64;

    for dy in 0..rect.h {
        let y = rect.y + dy;
        let row_start = y * fb_w;
        for dx in 0..rect.w {
            let x = rect.x + dx;
            let pixel = fb_slice[row_start + x];
            let r = u64::from((pixel >> 16) & 0xFF);
            let g = u64::from((pixel >> 8) & 0xFF);
            let b = u64::from(pixel & 0xFF);

            sum_r += r;
            sum_g += g;
            sum_b += b;

            sum_sq_r += r * r;
            sum_sq_g += g * g;
            sum_sq_b += b * b;

            count += 1;
        }
    }

    if count == 0 {
        return (0xFF_000000, 0.0);
    }

    let avg_r = (sum_r / count) as u32;
    let avg_g = (sum_g / count) as u32;
    let avg_b = (sum_b / count) as u32;
    let avg_color = 0xFF_000000 | (avg_r << 16) | (avg_g << 8) | avg_b;

    // Variance = E[X^2] - (E[X])^2
    #[allow(clippy::suspicious_operation_groupings)]
    let var_r = (sum_sq_r as f32 / count as f32) - (avg_r as f32 * avg_r as f32);
    #[allow(clippy::suspicious_operation_groupings)]
    let var_g = (sum_sq_g as f32 / count as f32) - (avg_g as f32 * avg_g as f32);
    #[allow(clippy::suspicious_operation_groupings)]
    let var_b = (sum_sq_b as f32 / count as f32) - (avg_b as f32 * avg_b as f32);

    // Normalize variance (max possible variance for 0-255 is (255/2)^2 = ~16256.25)
    // We just use a rough normalized sum across channels.
    let total_var = (var_r + var_g + var_b) / (3.0 * 16256.25);

    (avg_color, total_var)
}

fn draw_block(fb_mut: &mut [u32], fb_w: usize, rect: &Rect, color: u32, config: &QuadtreeConfig) {
    for dy in 0..rect.h {
        let y = rect.y + dy;
        let row_start = y * fb_w;
        for dx in 0..rect.w {
            let x = rect.x + dx;

            let mut pixel_color = color;

            if config.draw_borders {
                if dx == 0 || dy == 0 || dx == rect.w - 1 || dy == rect.h - 1 {
                    pixel_color = config.border_color;
                }
            }

            fb_mut[row_start + x] = pixel_color;
        }
    }
}

fn subdivide_and_draw(
    src_slice: &[u32],
    fb_mut: &mut [u32],
    fb_w: usize,
    rect: Rect,
    config: &QuadtreeConfig,
) {
    let (avg_color, variance) = compute_avg_and_variance(src_slice, fb_w, &rect);

    let can_subdivide = rect.w >= config.min_block_size * 2 && rect.h >= config.min_block_size * 2;

    if variance > config.variance_threshold && can_subdivide {
        // Subdivide into 4 quadrants
        let half_w = rect.w / 2;
        let half_h = rect.h / 2;

        let q1 = Rect {
            x: rect.x,
            y: rect.y,
            w: half_w,
            h: half_h,
        };
        let q2 = Rect {
            x: rect.x + half_w,
            y: rect.y,
            w: rect.w - half_w,
            h: half_h,
        };
        let q3 = Rect {
            x: rect.x,
            y: rect.y + half_h,
            w: half_w,
            h: rect.h - half_h,
        };
        let q4 = Rect {
            x: rect.x + half_w,
            y: rect.y + half_h,
            w: rect.w - half_w,
            h: rect.h - half_h,
        };

        subdivide_and_draw(src_slice, fb_mut, fb_w, q1, config);
        subdivide_and_draw(src_slice, fb_mut, fb_w, q2, config);
        subdivide_and_draw(src_slice, fb_mut, fb_w, q3, config);
        subdivide_and_draw(src_slice, fb_mut, fb_w, q4, config);
    } else {
        // Leaf node, draw block
        draw_block(fb_mut, fb_w, &rect, avg_color, config);
    }
}

/// Applies the Quadtree segmentation filter to the framebuffer.
pub fn apply_quadtree(fb: &mut Framebuffer, config: &QuadtreeConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    // We need a source copy since we are writing blocks back and don't want
    // partially drawn blocks to affect variance calculations of siblings.
    let src_copy = fb.as_slice().to_vec();
    let fb_mut = fb.as_mut_slice();

    let root_rect = Rect {
        x: 0,
        y: 0,
        w: width,
        h: height,
    };

    subdivide_and_draw(&src_copy, fb_mut, width, root_rect, config);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quadtree_zero_variance() {
        let mut fb = Framebuffer::new(32, 32).unwrap();
        fb.clear(0xFF_FF0000); // Solid red

        let config = QuadtreeConfig {
            min_block_size: 4,
            variance_threshold: 0.05,
            draw_borders: false,
            border_color: 0,
        };

        apply_quadtree(&mut fb, &config);

        // Should remain a single solid red block
        assert_eq!(fb.get_pixel(16, 16).unwrap(), 0xFF_FF0000);
    }

    #[test]
    fn test_quadtree_high_variance() {
        let mut fb = Framebuffer::new(32, 32).unwrap();
        // Draw a checkerboard to force high variance
        for y in 0..32 {
            for x in 0..32 {
                let color = if (x / 16 + y / 16) % 2 == 0 {
                    0xFF_FFFFFF
                } else {
                    0xFF_000000
                };
                fb.set_pixel(x, y, color);
            }
        }

        let config = QuadtreeConfig {
            min_block_size: 16, // Will stop at 16x16
            variance_threshold: 0.01,
            draw_borders: true,
            border_color: 0xFF_00FF00,
        };

        apply_quadtree(&mut fb, &config);

        // Border should be drawn at 0, 0
        assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF_00FF00);
    }
}
