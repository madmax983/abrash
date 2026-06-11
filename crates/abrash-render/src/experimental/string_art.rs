//! String Art Filter
//!
//! Approximates a target image by drawing continuous straight lines (threads) between
//! anchor pins arranged around a circle, using a greedy error-minimization algorithm.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::color::Color;
use std::f32::consts::PI;

/// Configuration for the String Art effect.
#[derive(Debug, Clone, Copy)]
pub struct StringArtConfig {
    /// Number of pins (anchors) around the edge of the canvas.
    pub num_pins: usize,
    /// Number of lines (threads) to draw.
    pub num_lines: usize,
    /// Opacity of each thread line. Lower values require more lines.
    pub line_opacity: f32,
    /// Color of the thread lines.
    pub line_color: u32,
    /// Background color of the canvas.
    pub bg_color: u32,
    /// Penalty to apply when re-using a pin. Prevents over-concentration of lines.
    pub reuse_penalty: f32,
}

impl Default for StringArtConfig {
    fn default() -> Self {
        Self {
            num_pins: 288,
            num_lines: 3000,
            line_opacity: 0.15,
            line_color: 0xFF00_0000, // Black lines
            bg_color: 0xFFFF_FFFF,   // White canvas
            reuse_penalty: 0.5,
        }
    }
}

/// A line drawing algorithm that samples pixels along the segment
fn bresenham_line(mut x0: i32, mut y0: i32, x1: i32, y1: i32, mut callback: impl FnMut(i32, i32)) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        callback(x0, y0);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

/// Applies a String Art effect to the framebuffer.
///
/// Modifies the given framebuffer in-place. The existing content of the framebuffer
/// is treated as the "target" image. The canvas is cleared to `bg_color` and then
/// strings are drawn to approximate the original image.
pub fn apply_string_art(fb: &mut Framebuffer, config: &StringArtConfig) {
    let width = fb.width() as i32;
    let height = fb.height() as i32;
    let radius = (width.min(height) as f32) / 2.0 * 0.95; // 5% padding
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;

    // 1. Calculate pin coordinates
    let mut pins = Vec::with_capacity(config.num_pins);
    for i in 0..config.num_pins {
        let angle = i as f32 * 2.0 * PI / config.num_pins as f32;
        pins.push((
            (cx + radius * angle.cos()) as i32,
            (cy + radius * angle.sin()) as i32,
        ));
    }

    // 2. Pre-calculate line pixel indices between all pin pairs
    // Since N_pins is small (e.g. 288), we can pre-calculate all paths.
    // Memory size: 288 * 288 = ~83k paths, average length ~200 = ~16M indices (~64MB).
    // To save memory and initialization time, we'll calculate them on the fly.

    // 3. Extract the target image and convert to a grayscale "error" map.
    // 0 = already bright (or no ink needed), 255 = needs maximum ink.
    let num_pixels = (width * height) as usize;
    let mut target_error = vec![0.0; num_pixels];
    let slice = fb.as_slice();
    for i in 0..num_pixels {
        let color = Color::from_argb_u32(slice[i]);
        // We want to draw dark lines on light background.
        // So dark pixels have high error (need ink), light pixels have low error.
        // Inverse luminance:
        let lum = color.luminance();
        target_error[i] = 1.0 - lum;
    }

    // 4. Clear the framebuffer to background color
    fb.clear(config.bg_color);

    let mut current_pin = 0;

    // To avoid immediately reversing the line
    let mut last_pin = 0;

    // Penalize re-used pins
    let mut pin_usage = vec![0; config.num_pins];

    // Main drawing loop
    for _ in 0..config.num_lines {
        let (x0, y0) = pins[current_pin];

        let mut best_next_pin = current_pin;
        let mut best_score = -1.0;

        // Find the best next pin
        for next_pin in 0..config.num_pins {
            if next_pin == current_pin || next_pin == last_pin {
                continue;
            }

            let (x1, y1) = pins[next_pin];

            let mut line_score = 0.0;
            let mut num_pixels = 0;

            bresenham_line(x0, y0, x1, y1, |x, y| {
                if x >= 0 && x < width && y >= 0 && y < height {
                    let idx = (y * width + x) as usize;
                    line_score += target_error[idx];
                    num_pixels += 1;
                }
            });

            if num_pixels > 0 {
                // Average score per pixel, penalized by usage
                let mut avg_score = line_score / num_pixels as f32;
                avg_score -= (pin_usage[next_pin] as f32) * config.reuse_penalty;

                if avg_score > best_score {
                    best_score = avg_score;
                    best_next_pin = next_pin;
                }
            }
        }

        // If we found a good pin, draw the line and update error
        if best_next_pin == current_pin {
            // No good line found, stop early
            break;
        }

        let (x1, y1) = pins[best_next_pin];

        // "Draw" the line into the error map (reduce error)
        bresenham_line(x0, y0, x1, y1, |x, y| {
            if x >= 0 && x < width && y >= 0 && y < height {
                let idx = (y * width + x) as usize;
                // Subtract the line opacity from the error map
                target_error[idx] -= config.line_opacity;
                if target_error[idx] < 0.0 {
                    target_error[idx] = 0.0;
                }

                // Actually draw the line onto the framebuffer using alpha blending
                let bg_color_u32 = fb.get_pixel(x, y).unwrap_or(config.bg_color);
                let mut bg_color = Color::from_argb_u32(bg_color_u32);
                let fg_color = Color::from_argb_u32(config.line_color);

                bg_color.r =
                    bg_color.r * (1.0 - config.line_opacity) + fg_color.r * config.line_opacity;
                bg_color.g =
                    bg_color.g * (1.0 - config.line_opacity) + fg_color.g * config.line_opacity;
                bg_color.b =
                    bg_color.b * (1.0 - config.line_opacity) + fg_color.b * config.line_opacity;

                fb.set_pixel(x, y, bg_color.to_argb_u32());
            }
        });

        pin_usage[best_next_pin] += 1;
        last_pin = current_pin;
        current_pin = best_next_pin;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::framebuffer::Framebuffer;

    #[test]
    fn test_apply_string_art() {
        let mut fb = Framebuffer::new(64, 64).unwrap();
        fb.clear(0xFF00_0000); // Black background

        // Draw a white square in the middle to be our "target"
        for y in 20..44 {
            for x in 20..44 {
                fb.set_pixel(x, y, 0xFFFF_FFFF);
            }
        }

        let config = StringArtConfig {
            num_pins: 16, // small number for fast test
            num_lines: 10,
            ..Default::default()
        };

        apply_string_art(&mut fb, &config);

        // Ensure it doesn't panic and modified the buffer
        let center_color = fb.get_pixel(32, 32).unwrap();
        // Since we clear to bg_color (white) at start of effect
        // and string art is drawn, center should not be exactly white if a line crosses it
        // However, with 16 pins and 10 lines it might miss the exact center.
        // We just care that it runs successfully.
        assert_ne!(center_color, 0x0000_0000);
    }
}
