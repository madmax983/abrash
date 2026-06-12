//! String Art Filter.
//!
//! A procedural post-processing effect that approximates an image using a single continuous thread
//! woven between pins on a circle's edge, mimicking a physical string art canvas.

use abrash_core::framebuffer::Framebuffer;

/// A configuration struct for the String Art filter.
#[derive(Debug, Clone)]
pub struct StringArtConfig {
    /// Number of pins on the canvas edge.
    pub num_pins: usize,
    /// Number of string lines to draw.
    pub num_lines: usize,
    /// Opacity (0.0 to 1.0) of each string segment.
    pub line_opacity: f32,
    /// Line color (AARRGGBB format, ignores A, sets it dynamically based on opacity).
    pub line_color: u32,
    /// Canvas background color.
    pub background_color: u32,
    /// Invert the image luminance (useful for dark lines on light background).
    pub invert: bool,
}

impl Default for StringArtConfig {
    fn default() -> Self {
        Self {
            num_pins: 256,
            num_lines: 3000,
            line_opacity: 0.1,
            line_color: 0xFF00_0000,
            background_color: 0xFFFF_FFFF,
            invert: true,
        }
    }
}

/// Applies the String Art effect to the given framebuffer in-place.
pub fn apply_string_art(fb: &mut Framebuffer, config: &StringArtConfig) {
    let w = fb.width() as usize;
    let h = fb.height() as usize;
    let cx = w / 2;
    let cy = h / 2;
    let radius = (w.min(h) / 2) as f32 * 0.95;

    // 1. Calculate Pin Positions
    let mut pins = Vec::with_capacity(config.num_pins);
    for i in 0..config.num_pins {
        let angle = i as f32 * 2.0 * std::f32::consts::PI / config.num_pins as f32;
        let x = cx as f32 + radius * angle.cos();
        let y = cy as f32 + radius * angle.sin();
        pins.push((x as i32, y as i32));
    }

    // 2. Pre-calculate source luminance image (target dark image we want to recreate)
    let src_pixels = fb.as_slice();
    let mut target_image = vec![0.0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            let p = src_pixels[y * w + x];
            let r = ((p >> 16) & 0xFF) as f32;
            let g = ((p >> 8) & 0xFF) as f32;
            let b = (p & 0xFF) as f32;
            let mut lum = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;
            if config.invert {
                lum = 1.0 - lum;
            }
            target_image[y * w + x] = lum;
        }
    }

    // Clear output framebuffer to background
    fb.clear(config.background_color);

    // Color processing
    let base_a = (config.line_opacity * 255.0).clamp(0.0, 255.0) as u32;
    let c = config.line_color & 0x00FF_FFFF;
    let line_color_rgba = (base_a << 24) | c;

    // Greedy string line drawing
    let mut current_pin = 0;

    for _ in 0..config.num_lines {
        let mut best_score = -1.0;
        let mut best_pin = -1;

        let min_skip = (config.num_pins / 20).max(1);

        for offset in min_skip..(config.num_pins - min_skip) {
            let test_pin = (current_pin + offset) % config.num_pins;

            // Calculate line score
            let p1 = pins[current_pin];
            let p2 = pins[test_pin];

            let mut score = 0.0;
            let mut num_pixels = 0;

            // Fast Bresenham for scoring
            let mut x0 = p1.0;
            let mut y0 = p1.1;
            let x1 = p2.0;
            let y1 = p2.1;

            let dx = (x1 - x0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let dy = -(y1 - y0).abs();
            let sy = if y0 < y1 { 1 } else { -1 };
            let mut err = dx + dy;

            while x0 >= 0 && x0 < w as i32 && y0 >= 0 && y0 < h as i32 {
                score += target_image[(y0 as usize) * w + (x0 as usize)];
                num_pixels += 1;

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

            if num_pixels > 0 {
                score /= num_pixels as f32;
            }

            if score > best_score {
                best_score = score;
                best_pin = test_pin as i32;
            }
        }

        if best_pin == -1 {
            best_pin = (current_pin + config.num_pins / 2) as i32 % config.num_pins as i32;
        }

        let p1 = pins[current_pin];
        let p2 = pins[best_pin as usize];

        // Subtract line from target image and draw to framebuffer
        let mut x0 = p1.0;
        let mut y0 = p1.1;
        let x1 = p2.0;
        let y1 = p2.1;
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let alpha = config.line_opacity;
        let inv_alpha = 1.0 - alpha;

        let fg_r = ((line_color_rgba >> 16) & 0xFF) as f32;
        let fg_g = ((line_color_rgba >> 8) & 0xFF) as f32;
        let fg_b = (line_color_rgba & 0xFF) as f32;

        while x0 >= 0 && x0 < w as i32 && y0 >= 0 && y0 < h as i32 {
            // Subtract opacity
            target_image[(y0 as usize) * w + (x0 as usize)] -= config.line_opacity;
            // Draw on framebuffer (using simple alpha blending)
            let p_idx = (y0 as usize) * w + (x0 as usize);
            let bg = fb.as_slice()[p_idx];

            let bg_r = ((bg >> 16) & 0xFF) as f32;
            let bg_g = ((bg >> 8) & 0xFF) as f32;
            let bg_b = (bg & 0xFF) as f32;

            let r = (fg_r * alpha + bg_r * inv_alpha) as u32;
            let g = (fg_g * alpha + bg_g * inv_alpha) as u32;
            let b = (fg_b * alpha + bg_b * inv_alpha) as u32;

            fb.as_mut_slice()[p_idx] = 0xFF00_0000 | (r << 16) | (g << 8) | b;

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

        current_pin = best_pin as usize;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_art() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        let config = StringArtConfig {
            num_pins: 10,
            num_lines: 5,
            ..Default::default()
        };
        apply_string_art(&mut fb, &config);
        assert_eq!(fb.as_slice()[0], config.background_color); // Ensure background was cleared
    }
}
