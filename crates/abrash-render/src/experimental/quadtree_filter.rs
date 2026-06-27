//! Quadtree Image Abstraction Filter
//!
//! Recursively subdivides the image based on color variance to produce
//! a blocky, stylized algorithmic aesthetic.

use crate::framebuffer::Framebuffer;

/// Configuration for the Quadtree filter.
#[derive(Debug, Clone)]
pub struct QuadtreeConfig {
    /// The maximum variance allowed before a region is subdivided.
    pub max_variance: f32,
    /// The minimum size of a subdivided region.
    pub min_size: usize,
}

impl Default for QuadtreeConfig {
    fn default() -> Self {
        Self {
            max_variance: 100.0,
            min_size: 4,
        }
    }
}

/// Computes the average color and variance of a rectangular region in the framebuffer.
fn compute_stats(fb: &Framebuffer, x: usize, y: usize, w: usize, h: usize) -> (u32, f32) {
    let mut r_sum = 0u64;
    let mut g_sum = 0u64;
    let mut b_sum = 0u64;
    let mut r_sq_sum = 0u64;
    let mut g_sq_sum = 0u64;
    let mut b_sq_sum = 0u64;
    let count = (w * h) as u64;

    if count == 0 {
        return (0, 0.0);
    }

    let width = fb.width() as usize;
    let pixels = fb.as_slice();

    for cy in y..y + h {
        for cx in x..x + w {
            let pixel = pixels[cy * width + cx];
            let r = u64::from((pixel >> 16) & 0xFF);
            let g = u64::from((pixel >> 8) & 0xFF);
            let b = u64::from(pixel & 0xFF);

            r_sum += r;
            g_sum += g;
            b_sum += b;

            r_sq_sum += r * r;
            g_sq_sum += g * g;
            b_sq_sum += b * b;
        }
    }

    let r_mean = r_sum / count;
    let g_mean = g_sum / count;
    let b_mean = b_sum / count;

    let r_var = (r_sq_sum / count).saturating_sub(r_mean * r_mean) as f32;
    let g_var = (g_sq_sum / count).saturating_sub(g_mean * g_mean) as f32;
    let b_var = (b_sq_sum / count).saturating_sub(b_mean * b_mean) as f32;

    let avg_color =
        0xFF00_0000 | ((r_mean as u32) << 16) | ((g_mean as u32) << 8) | (b_mean as u32);
    let variance = (r_var + g_var + b_var) / 3.0;

    (avg_color, variance)
}

/// Fills a rectangular region with a solid color.
fn fill_rect(fb: &mut Framebuffer, x: usize, y: usize, w: usize, h: usize, color: u32) {
    let width = fb.width() as usize;
    let pixels = fb.as_mut_slice();

    for cy in y..y + h {
        for cx in x..x + w {
            pixels[cy * width + cx] = color;
        }
    }
}

/// Recursively subdivides and renders the quadtree.
fn subdivide(
    src: &Framebuffer,
    dst: &mut Framebuffer,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    config: &QuadtreeConfig,
) {
    if w < config.min_size || h < config.min_size {
        let (avg_color, _) = compute_stats(src, x, y, w, h);
        fill_rect(dst, x, y, w, h, avg_color);
        return;
    }

    let (avg_color, variance) = compute_stats(src, x, y, w, h);

    if variance <= config.max_variance {
        fill_rect(dst, x, y, w, h, avg_color);
    } else {
        let hw = w / 2;
        let hh = h / 2;
        let w2 = w - hw;
        let h2 = h - hh;

        subdivide(src, dst, x, y, hw, hh, config);
        subdivide(src, dst, x + hw, y, w2, hh, config);
        subdivide(src, dst, x, y + hh, hw, h2, config);
        subdivide(src, dst, x + hw, y + hh, w2, h2, config);
    }
}

/// Applies the quadtree abstraction filter to the framebuffer.
///
/// # Panics
/// Panics if a temporary snapshot framebuffer cannot be allocated.
pub fn apply_quadtree(fb: &mut Framebuffer, config: &QuadtreeConfig) {
    // Create a read-only snapshot. We can't clone Framebuffer directly, so we copy pixels.
    let w = fb.width() as usize;
    let h = fb.height() as usize;

    if w == 0 || h == 0 {
        return;
    }

    let mut src_fb = Framebuffer::new(fb.width(), fb.height()).unwrap();
    src_fb.as_mut_slice().copy_from_slice(fb.as_slice());

    subdivide(&src_fb, fb, 0, 0, w, h, config);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quadtree_config_default() {
        let config = QuadtreeConfig::default();
        assert_eq!(config.max_variance, 100.0);
        assert_eq!(config.min_size, 4);
    }

    #[test]
    fn test_apply_quadtree() {
        let mut fb = Framebuffer::new(8, 8).unwrap();
        fb.clear(0xFFFF_0000); // Red

        // Half red, half blue
        for y in 0..8 {
            for x in 4..8 {
                fb.set_pixel(x as i32, y as i32, 0xFF00_00FF);
            }
        }

        let mut config = QuadtreeConfig::default();
        config.max_variance = 10.0;
        config.min_size = 2;

        apply_quadtree(&mut fb, &config);

        // Left side should remain mostly red, right side mostly blue
        let left_pixel = fb.get_pixel(2, 4).unwrap();
        let right_pixel = fb.get_pixel(6, 4).unwrap();
        assert_eq!(left_pixel & 0xFF_0000, 0xFF_0000);
        assert_eq!(right_pixel & 0x00_00FF, 0x00_00FF);
    }

    #[test]
    fn test_apply_quadtree_zero_size() {
        let mut fb = Framebuffer::new(0, 0).unwrap();
        let config = QuadtreeConfig::default();
        apply_quadtree(&mut fb, &config);
        // Should not panic
    }
}
