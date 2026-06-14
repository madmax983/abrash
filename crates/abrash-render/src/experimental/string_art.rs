//! String Art Filter Module
//!
//! A procedural art generator that simulates "String Art" (or thread art)
//! by repeatedly drawing straight lines between a set of pins arranged in a circle.
//! It uses a greedy algorithm to find the darkest path through an image and
//! reconstructs it using only straight overlapping lines.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

use std::cell::RefCell;

thread_local! {
    static TARGET_BUFFER: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the String Art filter.
#[derive(Debug, Clone, Copy)]
pub struct StringArtConfig {
    /// The number of pins arranged in a circle.
    pub num_pins: usize,
    /// The number of lines (threads) to draw.
    pub num_lines: usize,
    /// The color of the thread being drawn.
    pub thread_color: u32,
    /// The background color.
    pub background_color: u32,
    /// The opacity of the thread (0.0 to 1.0).
    pub thread_alpha: f32,
    /// The radius of the pin circle (0.0 to 1.0, relative to the smallest screen dimension).
    pub radius: f32,
}

impl Default for StringArtConfig {
    fn default() -> Self {
        Self {
            num_pins: 256,
            num_lines: 1000,
            thread_color: 0xFF_00_00_00, // Black
            background_color: 0xFF_FF_FF_FF, // White
            thread_alpha: 0.1,
            radius: 0.95,
        }
    }
}

/// Applies the String Art filter to the given framebuffer.
pub fn apply_string_art(fb: &mut Framebuffer, config: &StringArtConfig) {
    if config.num_pins < 3 || config.num_lines == 0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let width_i32 = width as i32;
    let height_i32 = height as i32;

    if width == 0 || height == 0 {
        return;
    }

    // 1. Calculate pin positions
    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0;
    let max_radius = (width.min(height) as f32 / 2.0) * config.radius;

    let mut pins = Vec::with_capacity(config.num_pins);
    for i in 0..config.num_pins {
        let angle = (i as f32 / config.num_pins as f32) * std::f32::consts::TAU;
        let px = (center_x + angle.cos() * max_radius) as i32;
        let py = (center_y + angle.sin() * max_radius) as i32;
        pins.push((px.clamp(0, width_i32 - 1), py.clamp(0, height_i32 - 1)));
    }

    // Convert pixel coordinates to an array of 0-255 luminances representing the "target darkness".
    // 0 is white (no thread needed), 255 is black (needs thread).
    TARGET_BUFFER.with(|buf| {
        let mut target = buf.borrow_mut();
        target.resize(width * height, 0);

        let src_pixels = fb.as_slice();
        for (i, &pixel) in src_pixels.iter().enumerate() {
            // We want to draw thread in dark areas, so target darkness = 255 - luminance.
            target[i] = 255 - pixel_luminance(pixel);
        }

        // Fill background
        let bg = config.background_color;
        for pixel in fb.as_mut_slice() {
            *pixel = bg;
        }

        let mut current_pin = 0;
        let thread_alpha_u8 = (config.thread_alpha * 255.0).clamp(0.0, 255.0) as u8;

        for _ in 0..config.num_lines {
            let (x0, y0) = pins[current_pin];

            // Find the best next pin
            let mut best_next_pin = current_pin;
            let mut best_score = -1.0;

            // We evaluate all other pins to find the darkest path.
            // Using Rayon here speeds up the evaluation when there are many pins.
            #[cfg(feature = "parallel")]
            let next_pin_scores: Vec<(usize, f32)> = {
                // To safely pass `target` to rayon, we slice it:
                let target_slice = &target[..];
                (0..config.num_pins)
                    .into_par_iter()
                    .filter(|&i| i != current_pin)
                    .map(|i| {
                        let (x1, y1) = pins[i];
                        let score = evaluate_line(target_slice, width_i32, x0, y0, x1, y1);
                        (i, score)
                    })
                    .collect()
            };

            #[cfg(not(feature = "parallel"))]
            let next_pin_scores: Vec<(usize, f32)> = (0..config.num_pins)
                .filter(|&i| i != current_pin)
                .map(|i| {
                    let (x1, y1) = pins[i];
                    let score = evaluate_line(&target, width_i32, x0, y0, x1, y1);
                    (i, score)
                })
                .collect();

            for (pin, score) in next_pin_scores {
                if score > best_score {
                    best_score = score;
                    best_next_pin = pin;
                }
            }

            // Draw the line on the framebuffer and erase from the target
            let (x1, y1) = pins[best_next_pin];
            draw_and_erase_line(
                fb,
                &mut target,
                width_i32,
                x0,
                y0,
                x1,
                y1,
                config.thread_color,
                thread_alpha_u8,
            );

            current_pin = best_next_pin;
        }
    });
}

/// Evaluates a line using Bresenham's algorithm. Returns the average darkness.
fn evaluate_line(target: &[u8], width: i32, x0: i32, y0: i32, x1: i32, y1: i32) -> f32 {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut sum = 0;
    let mut count = 0;

    let mut cx = x0;
    let mut cy = y0;

    loop {
        let idx = (cy * width + cx) as usize;
        sum += u32::from(target[idx]);
        count += 1;

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
        return 0.0;
    }
    sum as f32 / count as f32
}

/// Draws a line on the framebuffer and subtracts darkness from the target buffer.
fn draw_and_erase_line(
    fb: &mut Framebuffer,
    target: &mut [u8],
    width: i32,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: u32,
    alpha: u8,
) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut cx = x0;
    let mut cy = y0;

    let pixels = fb.as_mut_slice();

    // For additive/subtractive blending, we use pure int math
    let cr = ((color >> 16) & 0xFF) as u32;
    let cg = ((color >> 8) & 0xFF) as u32;
    let cb = (color & 0xFF) as u32;
    let inv_alpha = 255 - u32::from(alpha);

    loop {
        let idx = (cy * width + cx) as usize;

        // Erase from target (subtract alpha, clamp to 0)
        target[idx] = target[idx].saturating_sub(alpha);

        // Alpha blend onto framebuffer
        let dest = pixels[idx];
        let dr = ((dest >> 16) & 0xFF) as u32;
        let dg = ((dest >> 8) & 0xFF) as u32;
        let db = (dest & 0xFF) as u32;

        let r = (cr * u32::from(alpha) + dr * inv_alpha) / 255;
        let g = (cg * u32::from(alpha) + dg * inv_alpha) / 255;
        let b = (cb * u32::from(alpha) + db * inv_alpha) / 255;

        pixels[idx] = 0xFF00_0000 | (r << 16) | (g << 8) | b;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_string_art() {
        let mut fb = Framebuffer::new(20, 20).unwrap();
        fb.clear(0xFF_00_00_00); // Black background = max darkness

        let config = StringArtConfig {
            num_pins: 10,
            num_lines: 5,
            thread_color: 0xFF_FF_00_00, // Red threads
            background_color: 0xFF_FF_FF_FF, // White background
            thread_alpha: 1.0,
            radius: 1.0,
        };

        apply_string_art(&mut fb, &config);

        // Since thread_alpha is 1.0, any drawn pixel will be fully red.
        // And the rest will be the white background.
        let has_red = fb.as_slice().iter().any(|&p| p == 0xFF_FF_00_00);
        let has_white = fb.as_slice().iter().any(|&p| p == 0xFF_FF_FF_FF);

        assert!(has_red, "Should have drawn some red threads");
        assert!(has_white, "Should have a white background");
    }

    #[test]
    fn test_apply_string_art_zero_lines() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFF_00_00_00);

        let config = StringArtConfig {
            num_lines: 0,
            ..Default::default()
        };

        apply_string_art(&mut fb, &config);

        // Framebuffer should be unmodified
        for &pixel in fb.as_slice() {
            assert_eq!(pixel, 0xFF_00_00_00);
        }
    }
}
