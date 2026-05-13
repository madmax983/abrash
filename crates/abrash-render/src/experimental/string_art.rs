//! String Art (Thread Art) Post-Processing Filter
//!
//! Simulates physical string art by drawing straight semi-transparent lines
//! between pins arranged in a circle to approximate the image's luminance.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

/// Configuration for the String Art filter.
#[derive(Debug, Clone, Copy)]
pub struct StringArtConfig {
    /// Number of pins placed around the circle.
    pub num_pins: u32,
    /// Number of lines (threads) to draw.
    pub num_lines: u32,
    /// Color of the thread (ARGB). Should be somewhat transparent.
    pub line_color: u32,
    /// Color of the background (ARGB).
    pub background_color: u32,
    /// The amount to subtract from the error map when a line is drawn.
    pub line_weight: u8,
    /// Whether to invert the target image before finding strings (if true, draws strings on light areas)
    pub invert: bool,
}

impl Default for StringArtConfig {
    fn default() -> Self {
        Self {
            num_pins: 256,
            num_lines: 2000,
            line_color: 0x44_FF_FF_FF,       // Semi-transparent white
            background_color: 0xFF_00_00_00, // Black background
            line_weight: 20,
            invert: false,
        }
    }
}

struct Point {
    x: i32,
    y: i32,
}

/// Applies the String Art filter to the given Framebuffer.
pub fn apply_string_art(fb: &mut Framebuffer, config: StringArtConfig) {
    if config.num_pins < 2 { return; }
    let width = fb.width();
    let height = fb.height();

    // We constrain the circle to the smallest dimension
    let radius = (width.min(height) as f32 / 2.0) - 1.0;
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;

    // 1. Calculate pin coordinates
    let mut pins = Vec::with_capacity(config.num_pins as usize);
    for i in 0..config.num_pins {
        let angle = (i as f32 / config.num_pins as f32) * std::f32::consts::TAU;
        pins.push(Point {
            x: (cx + radius * angle.cos()) as i32,
            y: (cy + radius * angle.sin()) as i32,
        });
    }

    // 2. Prepare the error map (luminance of original image)
    // We want to draw strings where it's "darkest" (if not inverted)
    // To make logic simpler, let's map the pixels to a "target darkness"
    // 255 = needs a lot of string, 0 = needs no string
    let mut error_map = vec![0u8; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let px = fb.get_pixel(x as i32, y as i32).unwrap_or(0);
            let mut lum = pixel_luminance(px);
            if config.invert {
                lum = 255 - lum;
            }
            // We want strings in the light parts if invert is true
            // If invert is false, strings go in the dark parts.
            // Let's standardise: error_map = how much string is needed.
            // Default (invert=false) means we draw strings over dark areas.
            // Wait, standard string art usually draws black strings on a white canvas to replicate a photo.
            // So dark areas in the photo = need strings.
            // If the photo is dark, `lum` is low. So `255 - lum` is high.
            error_map[(y * width + x) as usize] = if config.invert { lum } else { 255 - lum };
        }
    }

    // 3. Clear framebuffer to background
    fb.clear(config.background_color);

    // 4. Iteratively find best lines
    let mut current_pin = 0;

    for _ in 0..config.num_lines {
        let mut best_next_pin = 0;
        let mut best_score = -1.0;

        let p0 = &pins[current_pin];

        // Evaluate all possible next pins
        for next_pin in 0..config.num_pins as usize {
            if next_pin == current_pin {
                continue;
            }

            // Skip nearby pins to avoid drawing the circle outline
            let dist_pins = (next_pin as i32 - current_pin as i32).abs();
            let dist_pins_wrap = (config.num_pins as i32 - dist_pins).abs();
            if dist_pins.min(dist_pins_wrap) < 10 {
                continue;
            }

            let p1 = &pins[next_pin];

            // Raycast and accumulate score
            let score = calculate_line_score(p0.x, p0.y, p1.x, p1.y, width as i32, height as i32, &error_map);

            if score > best_score {
                best_score = score;
                best_next_pin = next_pin;
            }
        }

        // Draw the best line on the framebuffer
        let p1 = &pins[best_next_pin];
        draw_string_line(fb, p0.x, p0.y, p1.x, p1.y, config.line_color);

        // Subtract line from error map
        subtract_line_error(p0.x, p0.y, p1.x, p1.y, width as i32, height as i32, &mut error_map, config.line_weight);

        current_pin = best_next_pin;
    }
}

/// Evaluates a line using Bresenham's algorithm and returns its average score.
fn calculate_line_score(x0: i32, y0: i32, x1: i32, y1: i32, width: i32, height: i32, error_map: &[u8]) -> f32 {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut x = x0;
    let mut y = y0;

    let mut sum = 0;
    let mut count = 0;

    loop {
        if x >= 0 && x < width && y >= 0 && y < height {
            sum += u32::from(error_map[(y * width + x) as usize]);
            count += 1;
        }

        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }

    if count == 0 {
        return 0.0;
    }
    sum as f32 / count as f32
}

/// Subtracts the line's "ink" from the error map so it isn't drawn repeatedly.
fn subtract_line_error(x0: i32, y0: i32, x1: i32, y1: i32, width: i32, height: i32, error_map: &mut [u8], weight: u8) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut x = x0;
    let mut y = y0;

    loop {
        if x >= 0 && x < width && y >= 0 && y < height {
            let idx = (y * width + x) as usize;
            error_map[idx] = error_map[idx].saturating_sub(weight);
        }

        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// Draws a semi-transparent line on the framebuffer.
fn draw_string_line(fb: &mut Framebuffer, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut x = x0;
    let mut y = y0;

    loop {
        // Blend color
        if let Some(bg) = fb.get_pixel(x, y) {
            fb.set_pixel(x, y, blend_colors(bg, color));
        }

        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// Simple alpha blending for ARGB.
const fn blend_colors(bg: u32, fg: u32) -> u32 {
    let a_fg = ((fg >> 24) & 0xFF) as u32;
    if a_fg == 0 {
        return bg;
    }
    if a_fg == 255 {
        return fg;
    }

    let a_bg = 255 - a_fg;

    let r_fg = (fg >> 16) & 0xFF;
    let g_fg = (fg >> 8) & 0xFF;
    let b_fg = fg & 0xFF;

    let r_bg = (bg >> 16) & 0xFF;
    let g_bg = (bg >> 8) & 0xFF;
    let b_bg = bg & 0xFF;

    let r = (r_fg * a_fg + r_bg * a_bg) / 255;
    let g = (g_fg * a_fg + g_bg * a_bg) / 255;
    let b = (b_fg * a_fg + b_bg * a_bg) / 255;

    0xFF00_0000 | (r << 16) | (g << 8) | b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_string_art_modifies_framebuffer() {
        let mut fb = Framebuffer::new(32, 32).unwrap();
        // Fill with white
        fb.clear(0xFF_FF_FF_FF);

        // Run string art filter (should draw lines to recreate the white area if invert=true)
        // By default, invert=false, so it expects black lines on white canvas?
        // Wait, standard error map means 255-lum.
        // If image is white, lum=255. 255-255=0. Error map is 0, no lines drawn.

        let config = StringArtConfig::default();
        apply_string_art(&mut fb, config);

        // Wait, it should clear to config.background_color (black).
        // The background color is 0xFF_00_00_00.
        // If no strings are drawn, the whole buffer is black.
        assert_eq!(fb.get_pixel(0, 0).unwrap(), config.background_color);
    }

    #[test]
    fn test_apply_string_art_draws_lines() {
        let mut fb = Framebuffer::new(32, 32).unwrap();
        // Fill with black (luminance 0) -> error map = 255 (needs string!)
        fb.clear(0xFF_00_00_00);

        let mut config = StringArtConfig::default();
        config.num_lines = 100;
        config.line_color = 0xAA_FF_00_00;

        apply_string_art(&mut fb, config);

        // Background should be cleared to black, but since the original image was black,
        // it needs strings drawn over it (to replicate "darkness"). So it should draw red lines.
        let mut has_red = false;
        for y in 0..32 {
            for x in 0..32 {
                let px = fb.get_pixel(x, y).unwrap();
                if px != config.background_color {
                    has_red = true;
                    break;
                }
            }
        }

        assert!(has_red, "String art should have drawn red lines over the black areas.");
    }
}
