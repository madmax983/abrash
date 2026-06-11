//! # String Art Filter
//!
//! A procedural post-processing effect that simulates string art.
//! It uses a greedy error-minimization algorithm over a pixel array
//! to approximate the target image by drawing straight lines (strings)
//! between anchor pins located around the image perimeter.
//!
//! Accumulating opacity and penalizing the re-use of identical anchor pins
//! avoids over-concentration of lines and ensures an even distribution of threads.

use abrash_core::framebuffer::Framebuffer;
use crate::math::Vec2;

use std::f32::consts::PI;

/// Applies a String Art stylization filter to the framebuffer.
///
/// * `fb`: The Framebuffer containing the original image.
/// * `num_pins`: The number of anchor pins placed around the perimeter (e.g., 200).
/// * `num_lines`: The number of strings/lines to draw (e.g., 1000).
/// * `line_opacity`: The opacity of each string drawn (e.g., 0.1 for 10% opacity).
/// * `string_color`: The base color of the strings (e.g., 0xFF000000 for black).
/// * `background_color`: The background color of the output canvas (e.g., 0xFFFFFFFF for white).
pub fn apply_string_art(
    fb: &mut Framebuffer,
    num_pins: usize,
    num_lines: usize,
    line_opacity: f32,
    string_color: u32,
    background_color: u32,
) {
    if num_pins < 3 || num_lines == 0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;
    if width == 0 || height == 0 {
        return;
    }

    // 1. Calculate Pin Locations (circle around the center)
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    let radius = (width.min(height) as f32 / 2.0) - 1.0;

    let mut pins = Vec::with_capacity(num_pins);
    for i in 0..num_pins {
        let angle = (i as f32 / num_pins as f32) * 2.0 * PI;
        let x = cx + angle.cos() * radius;
        let y = cy + angle.sin() * radius;
        pins.push(Vec2::new(x, y));
    }

    // 2. Convert source image to grayscale luminance matrix (our "error" matrix)
    // Darker pixels mean higher target error to reduce.
    // 0.0 = white (no ink needed), 1.0 = black (max ink needed)
    let src_pixels = fb.as_slice();
    let mut error_map = vec![0.0f32; width * height];
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let c = src_pixels[idx];
            let r = ((c >> 16) & 0xFF) as f32 / 255.0;
            let g = ((c >> 8) & 0xFF) as f32 / 255.0;
            let b = (c & 0xFF) as f32 / 255.0;
            let lum = 0.299 * r + 0.587 * g + 0.114 * b;
            error_map[idx] = 1.0 - lum; // Invert: dark areas need more strings
        }
    }

    // 3. Prepare an output canvas
    let mut out_pixels = vec![background_color; width * height];

    let a_str = ((string_color >> 24) & 0xFF) as f32 / 255.0 * line_opacity;
    let r_str = ((string_color >> 16) & 0xFF) as f32;
    let g_str = ((string_color >> 8) & 0xFF) as f32;
    let b_str = (string_color & 0xFF) as f32;

    // We penalize recently used pins to avoid clustering
    let mut pin_penalties = vec![0.0f32; num_pins];

    // Helper function to get line pixels using Bresenham
    let get_line_pixels = |p0: usize, p1: usize| -> Vec<(usize, usize)> {
        let mut pixels = Vec::new();
        let (mut x0, mut y0) = (pins[p0].x as i32, pins[p0].y as i32);
        let (x1, y1) = (pins[p1].x as i32, pins[p1].y as i32);

        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if x0 >= 0 && x0 < width as i32 && y0 >= 0 && y0 < height as i32 {
                pixels.push((x0 as usize, y0 as usize));
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
        pixels
    };

    // 4. Greedy generation loop
    let mut current_pin = 0;

    for _ in 0..num_lines {
        let mut best_pin = 0;
        let mut best_score = -1.0;
        let mut best_line_pixels = Vec::new();

        // Evaluate all possible next pins
        for next_pin in 0..num_pins {
            // Avoid same pin or immediate neighbors
            if next_pin == current_pin
                || (next_pin as i32 - current_pin as i32).abs() <= (num_pins as i32 / 20).max(1)
                || (next_pin as i32 - current_pin as i32).abs() >= (num_pins as i32 - (num_pins as i32 / 20).max(1))
            {
                continue;
            }

            let line_pixels = get_line_pixels(current_pin, next_pin);

            // Score the line based on how much error it reduces
            let mut score = 0.0;
            for &(px, py) in &line_pixels {
                score += error_map[py * width + px];
            }

            // Average score per pixel
            if !line_pixels.is_empty() {
                score /= line_pixels.len() as f32;
            }

            // Apply penalty for recently used pins
            score -= pin_penalties[next_pin];

            if score > best_score {
                best_score = score;
                best_pin = next_pin;
                best_line_pixels = line_pixels;
            }
        }

        // Draw the best line
        for &(px, py) in &best_line_pixels {
            let idx = py * width + px;

            // Reduce error (subtract line contribution from error map)
            error_map[idx] = (error_map[idx] - line_opacity * 0.5).max(0.0);

            // Blend onto output canvas
            let c_bg = out_pixels[idx];
            let r_bg = ((c_bg >> 16) & 0xFF) as f32;
            let g_bg = ((c_bg >> 8) & 0xFF) as f32;
            let b_bg = (c_bg & 0xFF) as f32;

            let inv_a = 1.0 - a_str;
            let r_out = (r_str * a_str + r_bg * inv_a) as u32;
            let g_out = (g_str * a_str + g_bg * inv_a) as u32;
            let b_out = (b_str * a_str + b_bg * inv_a) as u32;

            out_pixels[idx] = 0xFF00_0000 | (r_out << 16) | (g_out << 8) | b_out;
        }

        // Decay penalties
        for p in &mut pin_penalties {
            *p *= 0.9;
        }
        // Add penalty to the chosen pin
        pin_penalties[best_pin] += 1.0;

        current_pin = best_pin;
    }

    // 5. Copy back to framebuffer
    fb.as_mut_slice().copy_from_slice(&out_pixels);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_art_empty() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        apply_string_art(&mut fb, 200, 10, 0.1, 0xFF00_0000, 0xFFFFFFFF);
        // Shouldn't panic.
    }

    #[test]
    fn test_string_art_small() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        // Fill with black (high error)
        fb.clear(0xFF00_0000);
        apply_string_art(&mut fb, 20, 50, 0.5, 0xFF00_0000, 0xFFFFFFFF);
        // Just checking it runs without issues
        assert_eq!(fb.width(), 10);
    }
}
