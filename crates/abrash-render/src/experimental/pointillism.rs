//! Pointillism Filter
//!
//! A post-processing effect that simulates pointillism (drawing with dots).

use abrash_core::framebuffer::Framebuffer;

#[derive(Debug, Clone, Copy)]
pub struct PointillismConfig {
    pub dot_size: u32,
    pub background_color: u32,
}

impl Default for PointillismConfig {
    fn default() -> Self {
        Self {
            dot_size: 4,
            background_color: 0xFF_FF_FF_FF,
        }
    }
}

pub fn apply_pointillism(fb: &mut Framebuffer, config: &PointillismConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || config.dot_size == 0 {
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

    // Fill the frame buffer with the background color
    fb.clear(config.background_color);

    let step = config.dot_size as usize;
    // Pre-calculate squared radius using scaled integer math to eliminate float conversions
    // r = dot_size / 2.0 => r_sq = dot_size^2 / 4.0
    // To use integers, we use: r_sq_scaled = dot_size * dot_size, and compare against (dx*dx + dy*dy) * 4
    let radius_sq_scaled = config.dot_size * config.dot_size;

    // Initial background color in the framebuffer (from test) is fully transparent black (0) usually,
    // or uninitialized, so checking source pixel against 0.
    // The test initially clears it with 0 by default when creating.
    // Let's just draw all non-zero/non-transparent pixels.

    for y in (0..height).step_by(step) {
        for x in (0..width).step_by(step) {
            let center_idx = y * width + x;
            // Unchecked access is safe here due to step_by bounds
            let color = unsafe { *source_buffer.get_unchecked(center_idx) };

            // Only draw dot if it has some alpha
            if (color >> 24) > 0 {
                // Draw a circle (dot)
                let half_size = (config.dot_size / 2) as i32;
                for dy in -half_size..=half_size {
                    for dx in -half_size..=half_size {
                        let dot_x = x as i32 + dx;
                        let dot_y = y as i32 + dy;

                        // Check circle bounds using integer math: (dx^2 + dy^2) <= (r^2)
                        // Note: To match (dot_size/2.0)^2 we do (dx*dx + dy*dy)*4 <= dot_size^2
                        if ((dx * dx + dy * dy) * 4) as u32 <= radius_sq_scaled {
                            fb.set_pixel(dot_x, dot_y, color);
                        }
                    }
                }
            }
        }
    }

    SOURCE_PIXELS.with(|buf| {
        buf.replace(src_pixels);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pointillism() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0);
        // Set center pixel to a specific color
        fb.set_pixel(4, 4, 0xFF_FF_00_00); // Red

        let config = PointillismConfig::default();
        apply_pointillism(&mut fb, &config);

        // After applying pointillism with default dot_size=4,
        // The dot starting at (4,4) should color surrounding pixels.
        // Also the background should be filled with background_color.

        let bg_pixel = fb.get_pixel(0, 0).unwrap();
        assert_eq!(
            bg_pixel, config.background_color,
            "Background should be colored"
        );

        // Center pixel should still be red (or part of the red dot)
        let center_pixel = fb.get_pixel(4, 4).unwrap();
        assert_eq!(
            center_pixel, 0xFF_FF_00_00,
            "Dot should have colored the center pixel"
        );
    }
}
