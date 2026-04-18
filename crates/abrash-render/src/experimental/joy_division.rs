//! Joy Division Topology Filter
//!
//! A procedural post-processing effect that transforms an image into a series
//! of horizontal topographical waveforms based on image luminance. It mimics
//! the iconic "Unknown Pleasures" album cover data visualization.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;
use std::cell::RefCell;

thread_local! {
    static SOURCE_FB: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration parameters for the Joy Division filter.
#[derive(Debug, Clone, Copy)]
pub struct JoyDivisionConfig {
    /// Distance between each horizontal line in pixels.
    pub line_spacing: usize,
    /// The multiplier for how high the peaks can go based on luminance.
    pub height_scale: f32,
    /// Color of the waveform lines.
    pub foreground_color: u32,
    /// Background color of the canvas.
    pub background_color: u32,
    /// Number of horizontal pixels to average together for smoothing out the peaks.
    pub smoothing_window: usize,
}

impl Default for JoyDivisionConfig {
    fn default() -> Self {
        Self {
            line_spacing: 10,
            height_scale: 40.0,
            foreground_color: 0xFF_FFFFFF, // White lines
            background_color: 0xFF_000000, // Black background
            smoothing_window: 5,
        }
    }
}

/// Applies a Joy Division topographical filter to the framebuffer.
///
/// This iterates over horizontal slices of the image, sampling luminance. It then draws
/// displaced horizontal lines and fills the space beneath them with the background color
/// to obscure lines drawn behind them, creating a 3D layering effect.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - Configuration settings for the effect.
pub fn apply_joy_division(fb: &mut Framebuffer, config: &JoyDivisionConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || config.line_spacing == 0 {
        return;
    }

    SOURCE_FB.with(|buf| {
        let mut src_fb_vec = buf.borrow_mut();
        let size = width * height;
        if src_fb_vec.len() < size {
            src_fb_vec.resize(size, 0);
        }
        let src_pixels = &mut src_fb_vec[..size];
        src_pixels.copy_from_slice(fb.as_slice());

        // Clear the destination to the background color
        fb.clear(config.background_color);
        let dest_pixels = fb.as_mut_slice();

        // To properly obscure things drawn behind, we iterate from back to front, meaning
        // top to bottom of the screen (y=0 to y=height). Wait! In our coordinate system
        // y=0 is top, y=height is bottom. But we displace UPWARDS (y - displacement).
        // If we draw y=10 then y=20, y=20's mask (from draw_y down to 20) will overwrite
        // the peak of y=10 if they overlap. That's exactly what we want (painter's algorithm).

        for base_y in (0..height).step_by(config.line_spacing) {
            for x in 0..width {
                // 1. Sample smoothed luminance
                let mut lum_sum = 0.0;
                let mut count = 0.0;

                let start_dx = x.saturating_sub(config.smoothing_window);
                let end_dx = (x + config.smoothing_window).min(width - 1);

                for sx in start_dx..=end_dx {
                    let pixel = src_pixels[base_y * width + sx];
                    lum_sum += pixel_luminance(pixel) as f32 / 255.0;
                    count += 1.0;
                }

                let avg_lum = if count > 0.0 { lum_sum / count } else { 0.0 };

                // 2. Calculate upward displacement
                // Smooth falloff towards the edges of the screen
                let center_dist = (x as f32 - width as f32 / 2.0).abs() / (width as f32 / 2.0);
                let edge_fade = (1.0 - center_dist.powi(2)).max(0.0);

                let displacement = (avg_lum * config.height_scale * edge_fade) as i32;

                // 3. Draw the peak pixel
                let draw_y = (base_y as i32 - displacement).clamp(0, height as i32 - 1) as usize;
                // Only draw if we're not overwriting a line from a higher base_y that is supposed to be in front.
                // Wait, if we iterate from top to bottom (base_y increasing), then base_y=0 is drawn first.
                // Then base_y=10 is drawn. It can overwrite base_y=0. This is back-to-front.
                // It works for mask-filling, but we must make sure we don't accidentally cover the top of the screen if draw_y is small.
                dest_pixels[draw_y * width + x] = config.foreground_color;

                // 4. Fill the area below the peak with the background color down to base_y
                // to act as a mask hiding lines drawn previously.
                for fill_y in (draw_y + 1)..=base_y {
                    dest_pixels[fill_y * width + x] = config.background_color;
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_joy_division_clears_background_and_draws_lines() {
        let mut fb = Framebuffer::new(20, 20).unwrap();
        // Set everything to red initially to see if it gets cleared
        fb.clear(0xFFFF_0000);

        // Draw a bright white pixel in the center
        fb.set_pixel(10, 10, 0xFF_FFFFFF);

        let config = JoyDivisionConfig {
            line_spacing: 5,
            height_scale: 5.0,
            foreground_color: 0xFF_00FF00, // Green lines
            background_color: 0xFF_0000FF, // Blue background
            smoothing_window: 1,           // No smoothing for exact center peak
        };

        apply_joy_division(&mut fb, &config);

        // 1. The original red should be gone. The background should be blue.
        let top_left = fb.get_pixel(0, 0).unwrap();
        // Since y=0 is part of the loop (0..20 step 5), base_y=0 will overwrite y=0 with foreground color if displacement is 0.
        // Let's check a pixel that is purely background, e.g., (1, 1). Wait, at base_y=0, displacement is 0 (since no pixel at y=0 is bright).
        // so dest[0] = 0xFF_00FF00 (Green). That's why top_left is Green (0xFF00FF00)!
        // Let's check a background pixel between lines, e.g., (0, 2).
        let bg_pixel = fb.get_pixel(0, 2).unwrap();
        assert_eq!(bg_pixel, 0xFF_0000FF, "Background was not cleared properly");

        // 2. We should see some green line pixels
        let mut found_green = false;
        for &pixel in fb.as_slice() {
            if pixel == 0xFF_00FF00 {
                found_green = true;
                break;
            }
        }
        assert!(found_green, "No green line pixels were drawn");
    }
}
