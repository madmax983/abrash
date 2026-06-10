//! String Art Renderer
//!
//! A procedural renderer that simulates physical string art.
//! It places a set of pins (typically in a circle) and iteratively draws lines (strings)
//! between them to approximate a target image or procedural pattern.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

/// Configuration for the String Art algorithm.
#[derive(Debug, Clone, Copy)]
pub struct StringArtConfig {
    /// Number of pins on the perimeter.
    pub num_pins: usize,
    /// Total number of lines (strings) to draw.
    pub num_lines: usize,
    /// The opacity of a single thread (0.0 to 1.0).
    pub line_opacity: f32,
    /// The color of the thread (RGB).
    pub line_color: u32,
    /// The background color (RGB).
    pub bg_color: u32,
    /// A value (0.0 to 1.0) defining how much the algorithm penalizes repeatedly using the same pin.
    /// Higher values encourage more distributed patterns.
    pub penalty_factor: f32,
}

impl Default for StringArtConfig {
    fn default() -> Self {
        Self {
            num_pins: 256,
            num_lines: 3000,
            line_opacity: 0.15,
            line_color: 0xFF_00_00_00, // Black string
            bg_color: 0xFF_FF_FF_FF,   // White background
            penalty_factor: 0.1,
        }
    }
}

/// A naive 2D line drawing function (Bresenham) modified to 'subtract' line weight
/// from an internal target density map (used during the algorithm).
fn bresenham_line_sub(
    error_map: &mut [f32],
    width: i32,
    height: i32,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    weight: f32,
) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut cx = x0;
    let mut cy = y0;

    loop {
        if cx >= 0 && cx < width && cy >= 0 && cy < height {
            let idx = (cy * width + cx) as usize;
            error_map[idx] -= weight;
            if error_map[idx] < 0.0 {
                error_map[idx] = 0.0;
            }
        }

        if cx == x1 && cy == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            cx += sx;
        }
        if e2 <= dx {
            err += dx;
            cy += sy;
        }
    }
}

/// A naive 2D line drawing function for the final framebuffer.
fn draw_line_alpha(
    fb: &mut Framebuffer,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: u32,
    opacity: f32,
) {
    let width = fb.width() as i32;
    let height = fb.height() as i32;

    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut cx = x0;
    let mut cy = y0;

    let line_r = ((color >> 16) & 0xFF) as f32;
    let line_g = ((color >> 8) & 0xFF) as f32;
    let line_b = (color & 0xFF) as f32;

    loop {
        if cx >= 0 && cx < width && cy >= 0 && cy < height {
            if let Some(p) = fb.get_pixel(cx, cy) {
                let bg_r = ((p >> 16) & 0xFF) as f32;
                let bg_g = ((p >> 8) & 0xFF) as f32;
                let bg_b = (p & 0xFF) as f32;

                let out_r = (line_r * opacity + bg_r * (1.0 - opacity)) as u32;
                let out_g = (line_g * opacity + bg_g * (1.0 - opacity)) as u32;
                let out_b = (line_b * opacity + bg_b * (1.0 - opacity)) as u32;

                fb.set_pixel(cx, cy, 0xFF00_0000 | (out_r << 16) | (out_g << 8) | out_b);
            }
        }

        if cx == x1 && cy == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            cx += sx;
        }
        if e2 <= dx {
            err += dx;
            cy += sy;
        }
    }
}

/// Evaluates the total 'score' of a line in the error map.
/// A higher score means drawing this line covers more dense areas.
fn evaluate_line(
    error_map: &[f32],
    width: i32,
    height: i32,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
) -> f32 {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut cx = x0;
    let mut cy = y0;

    let mut score = 0.0;
    let mut count = 0;

    loop {
        if cx >= 0 && cx < width && cy >= 0 && cy < height {
            let idx = (cy * width + cx) as usize;
            score += error_map[idx];
            count += 1;
        }

        if cx == x1 && cy == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            cx += sx;
        }
        if e2 <= dx {
            err += dx;
            cy += sy;
        }
    }

    if count == 0 {
        0.0
    } else {
        score / count as f32
    }
}

/// Applies a string art algorithm to the framebuffer.
///
/// It uses the current framebuffer contents as the target image,
/// calculates the error map, and then draws the string art pattern
/// over a clean background.
pub fn apply_string_art(fb: &mut Framebuffer, config: &StringArtConfig) {
    let width = fb.width() as i32;
    let height = fb.height() as i32;
    let uwidth = width as usize;
    let uheight = height as usize;

    // 1. Calculate pins (arranged in a circle)
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    let radius = (width.min(height) as f32 / 2.0) - 2.0; // small margin

    let mut pins = Vec::with_capacity(config.num_pins);
    for i in 0..config.num_pins {
        let angle = (i as f32 / config.num_pins as f32) * std::f32::consts::TAU;
        let px = (cx + radius * angle.cos()) as i32;
        let py = (cy + radius * angle.sin()) as i32;
        pins.push((px, py));
    }

    // 2. Prepare target error map from current framebuffer
    // We want to draw black strings on white, so dark pixels in original = high error (needs string).
    let mut error_map = vec![0.0; uwidth * uheight];

    let is_dark_bg = pixel_luminance(config.bg_color) < 128;

    for (i, p) in fb.as_slice().iter().enumerate() {
        let luma = f32::from(pixel_luminance(*p)) / 255.0;
        // if background is light (e.g. white), target is dark (e.g. black). So high luma = 0 error, low luma = 1 error.
        // if background is dark, target is light. So low luma = 0 error, high luma = 1 error.
        if is_dark_bg {
            error_map[i] = luma;
        } else {
            error_map[i] = 1.0 - luma;
        }
    }

    // Clear to background color
    fb.clear(config.bg_color);

    // 3. Greedy algorithm to find strings
    let mut current_pin = 0;
    let mut pin_usage = vec![0.0; config.num_pins];

    for _ in 0..config.num_lines {
        let mut best_pin = 0;
        let mut best_score = -1.0;

        // Evaluate all other pins to find the best line
        for next_pin in 0..config.num_pins {
            if next_pin == current_pin {
                continue;
            }

            // Avoid drawing lines to immediately adjacent pins
            let pin_diff = (next_pin as i32 - current_pin as i32).abs();
            let pin_diff = pin_diff.min(config.num_pins as i32 - pin_diff);
            if pin_diff < 5 {
                continue;
            }

            let (x0, y0) = pins[current_pin];
            let (x1, y1) = pins[next_pin];

            let mut score = evaluate_line(&error_map, width, height, x0, y0, x1, y1);

            // Penalize using the same pin repeatedly
            score -= pin_usage[next_pin] * config.penalty_factor;

            if score > best_score {
                best_score = score;
                best_pin = next_pin;
            }
        }

        // Draw the best line in the error map (subtract density)
        let (x0, y0) = pins[current_pin];
        let (x1, y1) = pins[best_pin];

        // The weight subtraction should correlate with the visual opacity
        bresenham_line_sub(
            &mut error_map,
            width,
            height,
            x0,
            y0,
            x1,
            y1,
            config.line_opacity,
        );

        // Draw the actual line in the framebuffer
        draw_line_alpha(fb, x0, y0, x1, y1, config.line_color, config.line_opacity);

        pin_usage[best_pin] += 1.0;
        current_pin = best_pin;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_art_modifies_fb() {
        let mut fb = Framebuffer::new(32, 32).unwrap();
        fb.clear(0xFF_00_00_00); // Black target
        // Add a white circle in the middle
        for y in 0..32 {
            for x in 0..32 {
                let dx = x as f32 - 16.0;
                let dy = y as f32 - 16.0;
                if dx * dx + dy * dy < 100.0 {
                    fb.set_pixel(x, y, 0xFF_FF_FF_FF);
                }
            }
        }

        let mut config = StringArtConfig::default();
        config.num_lines = 10;
        config.num_pins = 32;

        apply_string_art(&mut fb, &config);

        // Background should be cleared to white (default bg)
        // There should be some non-white pixels (strings)
        let mut has_strings = false;
        for p in fb.as_slice() {
            if *p != 0xFF_FF_FF_FF {
                has_strings = true;
                break;
            }
        }
        assert!(has_strings);
    }
}
