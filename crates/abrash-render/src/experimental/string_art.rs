//! String Art / Thread Art Generator
//!
//! An artistic post-processing effect that approximates an image using hundreds
//! of intersecting straight lines (threads) drawn between pegs arranged in a circle.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;

use std::cell::RefCell;

thread_local! {
    static SCORE_BUFFER: RefCell<Vec<i32>> = const { RefCell::new(Vec::new()) };
    static PEGS_CACHE: RefCell<Vec<(i32, i32)>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the String Art effect.
#[derive(Debug, Clone, Copy)]
pub struct StringArtConfig {
    /// Number of pegs around the circle (e.g., 256).
    pub num_pegs: usize,
    /// Number of lines/threads to draw (e.g., 2000).
    pub num_lines: usize,
    /// Color of the thread (ARGB, e.g., `0xFF_000000` for black thread).
    pub thread_color: u32,
    /// Background color of the canvas (ARGB, e.g., `0xFF_FFFFFF` for white canvas).
    pub background_color: u32,
    /// How much weight/thickness each line has when subtracting from the score.
    pub line_weight: i32,
}

impl Default for StringArtConfig {
    fn default() -> Self {
        Self {
            num_pegs: 288,
            num_lines: 3000,
            thread_color: 0x22_000000,     // Very faint black lines
            background_color: 0xFF_FFFFFF, // White background
            line_weight: 15,
        }
    }
}

/// Applies a String Art approximation to the framebuffer.
///
/// It first calculates the luminance of the entire image as a "score".
/// Darker areas have higher scores, meaning they "need" more thread.
/// It then greedily finds lines between pegs that intersect the highest score,
/// drawing the line and subtracting the score along its path.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - Configuration for the string art effect.
pub fn apply_string_art(fb: &mut Framebuffer, config: &StringArtConfig) {
    if config.num_pegs < 3 || config.num_lines == 0 {
        return;
    }

    let width = fb.width() as i32;
    let height = fb.height() as i32;

    if width == 0 || height == 0 {
        return;
    }

    // 1. Convert framebuffer into a score buffer
    // Score represents "how much string we need". Darker pixels need more string.
    SCORE_BUFFER.with(|score_cell| {
        let mut score_vec = score_cell.borrow_mut();
        let num_pixels = (width * height) as usize;

        if score_vec.len() != num_pixels {
            score_vec.resize(num_pixels, 0);
        }

        let score = &mut score_vec[..num_pixels];
        let pixels = fb.as_slice();

        // White (255) -> 0 score
        // Black (0) -> 255 score
        for (i, &p) in pixels.iter().enumerate() {
            let luma = pixel_luminance(p);
            score[i] = 255 - i32::from(luma);
        }

        // 2. Pre-calculate peg positions around a circle
        PEGS_CACHE.with(|pegs_cell| {
            let mut pegs_vec = pegs_cell.borrow_mut();
            if pegs_vec.len() != config.num_pegs {
                pegs_vec.resize(config.num_pegs, (0, 0));
            }

            let pegs = &mut pegs_vec[..config.num_pegs];

            let center_x = width as f32 / 2.0;
            let center_y = height as f32 / 2.0;
            // Radius is slightly smaller than the shortest edge to fit on canvas
            let radius = (width.min(height) as f32) / 2.0 * 0.95;

            for (i, peg) in pegs.iter_mut().enumerate() {
                let angle = (i as f32 / config.num_pegs as f32) * std::f32::consts::TAU;
                let (sin_a, cos_a) = angle.sin_cos();
                let x = (center_x + cos_a * radius) as i32;
                let y = (center_y + sin_a * radius) as i32;

                *peg = (x.clamp(0, width - 1), y.clamp(0, height - 1));
            }

            // 3. Clear the framebuffer to the background color
            fb.clear(config.background_color);

            // 4. Greedily find best lines
            let mut current_peg = 0;
            // To prevent lines jumping between adjacent pegs forming an outline ring
            let min_jump = config.num_pegs / 10;

            for _ in 0..config.num_lines {
                let mut best_score = -1;
                let mut best_peg = current_peg;

                let (x0, y0) = pegs[current_peg];

                for p_idx in 0..config.num_pegs {
                    // Calculate distance around the circle
                    let mut diff = (p_idx as i32 - current_peg as i32).abs();
                    if diff > (config.num_pegs / 2) as i32 {
                        diff = config.num_pegs as i32 - diff;
                    }

                    if diff < min_jump as i32 {
                        continue;
                    }

                    let (x1, y1) = pegs[p_idx];

                    // Evaluate line score using Bresenham
                    let line_score = evaluate_line_score(x0, y0, x1, y1, width, height, score);

                    if line_score > best_score {
                        best_score = line_score;
                        best_peg = p_idx;
                    }
                }

                // If we couldn't find a good peg, just pick the opposite side
                if best_peg == current_peg {
                    best_peg = (current_peg + config.num_pegs / 2) % config.num_pegs;
                }

                let (x1, y1) = pegs[best_peg];

                // 5. Draw the line on the framebuffer and subtract score
                draw_and_subtract_line(
                    fb,
                    x0,
                    y0,
                    x1,
                    y1,
                    width,
                    height,
                    config.thread_color,
                    config.line_weight,
                    score,
                );

                current_peg = best_peg;
            }
        });
    });
}

// Custom Bresenham to just read score
fn evaluate_line_score(
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    width: i32,
    _height: i32,
    score_buf: &[i32],
) -> i32 {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut total_score = 0;

    loop {
        let idx = (y0 * width + x0) as usize;
        total_score += score_buf[idx];

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

    total_score
}

// Custom Bresenham to draw line and mutate score
#[allow(clippy::too_many_arguments)]
fn draw_and_subtract_line(
    fb: &mut Framebuffer,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    width: i32,
    _height: i32,
    color: u32,
    weight: i32,
    score_buf: &mut [i32],
) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let alpha = (color >> 24) & 0xFF;
    let inv_alpha = 255 - alpha;

    let r_src = (color >> 16) & 0xFF;
    let g_src = (color >> 8) & 0xFF;
    let b_src = color & 0xFF;

    let dest = fb.as_mut_slice();

    loop {
        let idx = (y0 * width + x0) as usize;

        // Subtract score
        score_buf[idx] -= weight;

        // Draw pixel with alpha blending over white/existing background
        let bg = dest[idx];

        if alpha == 255 {
            dest[idx] = color;
        } else {
            let r_dst = (bg >> 16) & 0xFF;
            let g_dst = (bg >> 8) & 0xFF;
            let b_dst = bg & 0xFF;

            let r = (r_src * alpha + r_dst * inv_alpha) / 255;
            let g = (g_src * alpha + g_dst * inv_alpha) / 255;
            let b = (b_src * alpha + b_dst * inv_alpha) / 255;

            dest[idx] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_string_art() {
        let mut fb = Framebuffer::new(50, 50).unwrap();
        // A black circle in the middle
        fb.clear(0xFF_FFFFFF);
        for y in 10..40 {
            for x in 10..40 {
                let dx = x as f32 - 25.0;
                let dy = y as f32 - 25.0;
                if dx * dx + dy * dy < 15.0 * 15.0 {
                    fb.set_pixel(x, y, 0xFF_000000);
                }
            }
        }

        let config = StringArtConfig {
            num_pegs: 32,
            num_lines: 50,
            thread_color: 0x88_000000,
            background_color: 0xFF_FFFFFF,
            line_weight: 20,
        };

        apply_string_art(&mut fb, &config);

        // Verify the background is mostly white but some dark lines have been drawn
        let mut has_lines = false;
        for &p in fb.as_slice() {
            let r = (p >> 16) & 0xFF;
            // Since it uses 0x88 alpha, lines won't be perfectly black
            if r < 200 {
                has_lines = true;
                break;
            }
        }

        assert!(has_lines, "String art should draw dark lines on the canvas");
    }
}
