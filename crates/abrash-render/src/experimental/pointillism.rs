//! Pointillism Filter
//!
//! A post-processing effect that simulates pointillism painting by drawing
//! random colored circles sampled from the underlying framebuffer.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::XorShift32;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Pointillism effect.
#[derive(Debug, Clone, Copy)]
pub struct PointillismConfig {
    /// The number of dots to draw.
    pub num_dots: usize,
    /// The minimum radius of a dot.
    pub min_radius: f32,
    /// The maximum radius of a dot.
    pub max_radius: f32,
    /// Whether to clear the background before drawing the dots.
    pub clear_background: bool,
    /// The color to clear the background with, if `clear_background` is true.
    pub background_color: u32,
}

impl Default for PointillismConfig {
    fn default() -> Self {
        Self {
            num_dots: 100_000,
            min_radius: 1.0,
            max_radius: 5.0,
            clear_background: true,
            background_color: 0xFF_F0_F0_F0, // Off-white canvas
        }
    }
}

/// Applies a Pointillism stylization filter to the framebuffer.
///
/// This filter reads the current colors from the framebuffer, clears it (optionally),
/// and redraws the image using thousands of small overlapping colored dots.
pub fn apply_pointillism(fb: &mut Framebuffer, config: &PointillismConfig) {
    let width = fb.width() as i32;
    let height = fb.height() as i32;

    if width <= 0 || height <= 0 || config.num_dots == 0 {
        return;
    }

    // We must read from the original pixels
    let mut src_pixels = vec![0; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            if let Some(col) = fb.get_pixel(x, y) {
                src_pixels[(y * width + x) as usize] = col;
            }
        }
    }

    if config.clear_background {
        fb.clear(config.background_color);
    }

    // Since drawing dots can overlap, we can't easily parallelize per-pixel writing without
    // race conditions, unless we divide into regions or use atomics.
    // For simplicity and speed in software, we'll draw them sequentially using a fast RNG.

    let mut rng = XorShift32::new(123_456_789);

    for _ in 0..config.num_dots {
        let x = (rng.next_u32() % (width as u32)) as i32;
        let y = (rng.next_u32() % (height as u32)) as i32;

        let src_idx = (y * width + x) as usize;
        let color = src_pixels[src_idx];

        let r_range = config.max_radius - config.min_radius;
        let radius = config.min_radius + (rng.next_u32() as f32 / u32::MAX as f32) * r_range;

        let radius_i = radius.ceil() as i32;
        let radius_sq = radius * radius;

        let start_y = (y - radius_i).max(0);
        let end_y = (y + radius_i).min(height - 1);
        let start_x = (x - radius_i).max(0);
        let end_x = (x + radius_i).min(width - 1);

        for py in start_y..=end_y {
            for px in start_x..=end_x {
                let dx = (px - x) as f32;
                let dy = (py - y) as f32;

                if dx * dx + dy * dy <= radius_sq {
                    fb.set_pixel(px, py, color);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_pointillism() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFF_FF_00_00); // Red

        let config = PointillismConfig {
            num_dots: 10,
            min_radius: 1.0,
            max_radius: 2.0,
            clear_background: true,
            background_color: 0xFF_00_00_00, // Black
        };

        apply_pointillism(&mut fb, &config);

        // Framebuffer should contain black background and some red dots
        let mut has_black = false;
        let mut has_red = false;

        for y in 0..10 {
            for x in 0..10 {
                let pixel = fb.get_pixel(x, y).unwrap();
                if pixel == 0xFF_00_00_00 {
                    has_black = true;
                } else if pixel == 0xFF_FF_00_00 {
                    has_red = true;
                }
            }
        }

        assert!(has_black);
        assert!(has_red);
    }
}
