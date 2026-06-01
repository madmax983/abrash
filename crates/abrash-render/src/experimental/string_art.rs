//! String Art Filter
//!
//! A procedural post-processing effect that simulates "string art" or "thread art".
//! It places a series of virtual pegs around a circular boundary and iteratively draws
//! overlapping semi-transparent lines (strings) to reconstruct the image based on its luminance.

use abrash_core::framebuffer::Framebuffer;
use std::f32::consts::PI;

/// Configuration for the String Art filter.
#[derive(Debug, Clone, Copy)]
pub struct StringArtConfig {
    /// The number of virtual pegs around the circle.
    pub num_pegs: usize,
    /// The number of strings (lines) to draw.
    pub num_strings: usize,
    /// The darkness contribution of each string drawn.
    pub string_alpha: f32,
}

impl Default for StringArtConfig {
    fn default() -> Self {
        Self {
            num_pegs: 256,
            num_strings: 2000,
            string_alpha: 0.1,
        }
    }
}

/// Applies a procedural string art effect to the framebuffer.
///
/// Converts the image to grayscale, generates a set of pegs in a circle, and repeatedly
/// finds the line between pegs that best matches the darkest un-covered area of the image.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - Settings for the effect.
pub fn apply_string_art(fb: &mut Framebuffer, config: &StringArtConfig) {
    if config.num_pegs == 0 || config.num_strings == 0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let num_pixels = width * height;
    if num_pixels == 0 {
        return;
    }

    let pixels = fb.as_mut_slice();

    // 1. Create a darkness map.
    // Darkness is 1.0 (black) to 0.0 (white).
    // Original image colors are extracted using luminance.
    let mut darkness_map = vec![0.0f32; num_pixels];
    for (i, p) in pixels.iter().enumerate() {
        let r = ((*p >> 16) & 0xFF) as f32;
        let g = ((*p >> 8) & 0xFF) as f32;
        let b = (*p & 0xFF) as f32;
        // Standard relative luminance
        let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        darkness_map[i] = 1.0 - (luminance / 255.0).clamp(0.0, 1.0);
    }

    // 2. Pre-calculate structural coordinates for circular peg positions.
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    let radius = (width.min(height) as f32 / 2.0) - 1.0;

    let mut pegs = Vec::with_capacity(config.num_pegs);
    for i in 0..config.num_pegs {
        let angle = (i as f32 / config.num_pegs as f32) * 2.0 * PI;
        let px = cx + radius * angle.cos();
        let py = cy + radius * angle.sin();
        pegs.push((px as isize, py as isize));
    }

    // 3. Clear the screen to white (canvas).
    for p in pixels.iter_mut() {
        *p = 0xFF_FFFFFF;
    }

    let mut current_peg = 0;

    // We will draw lines by modifying the darkness map and the framebuffer.
    let w_isize = width as isize;
    let h_isize = height as isize;

    for _ in 0..config.num_strings {
        let mut best_score = -1.0f32;
        let mut best_peg = current_peg;

        let (x0, y0) = pegs[current_peg];

        // Search for the best next peg to connect to
        for next_peg in 0..config.num_pegs {
            // Avoid drawing to the same or adjacent pegs to prevent dense borders
            let diff = (next_peg as isize - current_peg as isize).abs();
            let dist = diff.min(config.num_pegs as isize - diff);
            if dist < 10 {
                continue;
            }

            let (x1, y1) = pegs[next_peg];
            let mut score = 0.0;
            let mut count = 0;

            // Bresenham's line algorithm to score the path
            let dx = (x1 - x0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let dy = -(y1 - y0).abs();
            let sy = if y0 < y1 { 1 } else { -1 };
            let mut err = dx + dy;
            let mut cx_line = x0;
            let mut cy_line = y0;

            loop {
                if cx_line >= 0 && cx_line < w_isize && cy_line >= 0 && cy_line < h_isize {
                    let idx = (cy_line * w_isize + cx_line) as usize;
                    score += darkness_map[idx];
                    count += 1;
                }

                if cx_line == x1 && cy_line == y1 {
                    break;
                }
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    cx_line += sx;
                }
                if e2 <= dx {
                    err += dx;
                    cy_line += sy;
                }
            }

            if count > 0 {
                score /= count as f32; // Average darkness along the line
                if score > best_score {
                    best_score = score;
                    best_peg = next_peg;
                }
            }
        }

        // If no good peg found, we could stop early, but let's just proceed
        if best_peg == current_peg {
            break;
        }

        // Draw the line to the best peg and subtract its contribution from the darkness map
        let (x1, y1) = pegs[best_peg];
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let mut cx_line = x0;
        let mut cy_line = y0;

        let alpha = config.string_alpha;
        // Invert alpha for color blending (drawing black lines on white canvas)
        // 0xFF_FFFFFF is white. We want to blend towards black (0xFF_000000).
        // Since we are just drawing black strings, we can just subtract from RGB components.

        loop {
            if cx_line >= 0 && cx_line < w_isize && cy_line >= 0 && cy_line < h_isize {
                let idx = (cy_line * w_isize + cx_line) as usize;

                // Erase the darkness we just "covered" so we don't draw over it repeatedly
                darkness_map[idx] = (darkness_map[idx] - alpha).max(0.0);

                // Draw onto framebuffer
                let color = pixels[idx];
                let r = ((color >> 16) & 0xFF) as f32;
                let g = ((color >> 8) & 0xFF) as f32;
                let b = (color & 0xFF) as f32;

                // Blend towards black
                let nr = (r * (1.0 - alpha)).max(0.0) as u32;
                let ng = (g * (1.0 - alpha)).max(0.0) as u32;
                let nb = (b * (1.0 - alpha)).max(0.0) as u32;

                pixels[idx] = 0xFF00_0000 | (nr << 16) | (ng << 8) | nb;
            }

            if cx_line == x1 && cy_line == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                cx_line += sx;
            }
            if e2 <= dx {
                err += dx;
                cy_line += sy;
            }
        }

        current_peg = best_peg;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_art_empty() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        let config = StringArtConfig {
            num_pegs: 32,
            num_strings: 10,
            ..Default::default()
        };
        apply_string_art(&mut fb, &config);

        // Since the framebuffer was empty (black), it will draw strings.
        // It clears to white first, so we should expect mostly white pixels.
        // Let's just check it doesn't panic.
        assert_eq!(fb.get_pixel(0, 0), Some(0xFF_FFFFFF));
    }
}
