//! String Art Filter
//!
//! A post-processing effect that simulates physical string art by continuously
//! drawing the darkest path between a set of pins arranged in a circle,
//! lightening the underlying image to avoid drawing on the same path.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;
use std::f32::consts::PI;

/// Configuration for the String Art filter.
#[derive(Debug, Clone, Copy)]
pub struct StringArtConfig {
    /// Number of pins distributed around the circle.
    pub num_pins: usize,
    /// Number of lines (threads) to draw.
    pub num_lines: usize,
    /// Color of the thread (ARGB).
    pub thread_color: u32,
    /// Thread alpha for blending (0.0 to 1.0).
    pub thread_alpha: f32,
    /// Weight given to the thread when subtracting from the source image.
    pub subtraction_weight: f32,
    /// Background color for the final output.
    pub bg_color: u32,
}

impl Default for StringArtConfig {
    fn default() -> Self {
        Self {
            num_pins: 200,
            num_lines: 2000,
            thread_color: 0xFF_00_00_00, // Black thread
            thread_alpha: 0.1,           // Very faint thread
            subtraction_weight: 0.1,     // Lighten source by 10% on pass
            bg_color: 0xFF_FF_FF_FF,     // White background
        }
    }
}

/// Calculates the line points using Bresenham's line algorithm.
fn get_line_points(
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    width: i32,
    height: i32,
) -> Vec<(usize, usize)> {
    let mut points = Vec::new();
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if x0 >= 0 && x0 < width && y0 >= 0 && y0 < height {
            points.push((x0 as usize, y0 as usize));
        }
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
    points
}

/// Applies a String Art filter to the given framebuffer.
pub fn apply_string_art(fb: &mut Framebuffer, config: &StringArtConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    if width == 0 || height == 0 || config.num_pins < 3 || config.num_lines == 0 {
        return;
    }

    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    let radius = (cx.min(cy) - 2.0).max(1.0);

    // Calculate pin positions
    let mut pins = Vec::with_capacity(config.num_pins);
    for i in 0..config.num_pins {
        let angle = (i as f32 / config.num_pins as f32) * 2.0 * PI;
        let x = cx + angle.cos() * radius;
        let y = cy + angle.sin() * radius;
        pins.push((x as i32, y as i32));
    }

    // We need a working buffer representing the image's "darkness" remaining
    // and an output buffer for the actual drawing.
    let mut src_luma = vec![0.0f32; width * height];
    let src_pixels = fb.as_slice();
    for i in 0..(width * height) {
        let l = pixel_luminance(src_pixels[i]);
        src_luma[i] = 1.0 - (f32::from(l) / 255.0); // 1.0 is dark, 0.0 is light
    }

    // Clear the output framebuffer
    let out_pixels = fb.as_mut_slice();
    out_pixels.fill(config.bg_color);

    let tr = ((config.thread_color >> 16) & 0xFF) as f32;
    let tg = ((config.thread_color >> 8) & 0xFF) as f32;
    let tb = (config.thread_color & 0xFF) as f32;

    let mut current_pin = 0;

    for _ in 0..config.num_lines {
        let mut max_score = -1.0;
        let mut best_pin = current_pin;
        let mut best_line = Vec::new();

        for next_pin in 0..config.num_pins {
            if next_pin == current_pin {
                continue;
            }

            // Avoid adjacent pins which just draw the circle border
            let diff = (current_pin as isize - next_pin as isize).abs();
            let diff = diff.min(config.num_pins as isize - diff);
            if diff < 10 {
                continue;
            }

            let p0 = pins[current_pin];
            let p1 = pins[next_pin];
            let line = get_line_points(p0.0, p0.1, p1.0, p1.1, width as i32, height as i32);

            let mut score = 0.0;
            for &(x, y) in &line {
                score += src_luma[y * width + x];
            }
            if !line.is_empty() {
                score /= line.len() as f32; // Average darkness
            }

            if score > max_score {
                max_score = score;
                best_pin = next_pin;
                best_line = line;
            }
        }

        // Draw best line to output and subtract from source luma
        for &(x, y) in &best_line {
            let idx = y * width + x;
            // Lighten source so we don't pick it again
            src_luma[idx] = (src_luma[idx] - config.subtraction_weight).max(0.0);

            // Draw to output with alpha blending
            let bg_color = out_pixels[idx];
            let br = ((bg_color >> 16) & 0xFF) as f32;
            let bg = ((bg_color >> 8) & 0xFF) as f32;
            let bb = (bg_color & 0xFF) as f32;

            let out_r = (tr * config.thread_alpha + br * (1.0 - config.thread_alpha))
                .clamp(0.0, 255.0) as u32;
            let out_g = (tg * config.thread_alpha + bg * (1.0 - config.thread_alpha))
                .clamp(0.0, 255.0) as u32;
            let out_b = (tb * config.thread_alpha + bb * (1.0 - config.thread_alpha))
                .clamp(0.0, 255.0) as u32;

            out_pixels[idx] = 0xFF_00_00_00 | (out_r << 16) | (out_g << 8) | out_b;
        }

        current_pin = best_pin;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_art_no_crash() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        // Draw a black circle in the middle
        for y in 25..75 {
            for x in 25..75 {
                if (x - 50) * (x - 50) + (y - 50) * (y - 50) < 400 {
                    fb.set_pixel(x, y, 0xFF_00_00_00);
                } else {
                    fb.set_pixel(x, y, 0xFF_FF_FF_FF);
                }
            }
        }
        let config = StringArtConfig {
            num_pins: 50,
            num_lines: 100,
            ..Default::default()
        };
        apply_string_art(&mut fb, &config);
    }
}
