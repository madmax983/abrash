//! # String Art Filter
//!
//! An experimental post-processing effect that reconstructs the image using a single continuous
//! thread weaving between circular pegs, simulating physical string art.

use abrash_core::framebuffer::Framebuffer;

/// Configuration for the string art generator.
#[derive(Debug, Clone)]
pub struct StringArtConfig {
    /// Number of pegs around the circle
    pub num_pegs: usize,
    /// Total number of line segments to draw
    pub num_lines: usize,
    /// How much darkness each string removes from the target map
    pub line_weight: f32,
    /// Color of the thread in 0xAARRGGBB format
    pub thread_color: u32,
}

impl Default for StringArtConfig {
    fn default() -> Self {
        Self {
            num_pegs: 256,
            num_lines: 3000,
            line_weight: 0.1,
            thread_color: 0x2200_0000, // Very transparent black thread
        }
    }
}

/// Generates string art from a source image.
///
/// This uses a greedy algorithm: at each peg, it looks for the next peg that covers
/// the darkest available path, draws a line, and subtracts that line's darkness from the map.
///
/// # Panics
///
/// Panics if the output framebuffer fails to allocate due to invalid dimensions or OOM.
#[must_use]
pub fn generate_string_art(source: &Framebuffer, config: &StringArtConfig) -> Framebuffer {
    let width = source.width() as usize;
    let height = source.height() as usize;
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    let radius = (width.min(height) as f32 / 2.0) - 2.0;

    let mut pegs = Vec::with_capacity(config.num_pegs);
    for i in 0..config.num_pegs {
        let angle = i as f32 * 2.0 * std::f32::consts::PI / config.num_pegs as f32;
        let x = (cx + angle.cos() * radius) as i32;
        let y = (cy + angle.sin() * radius) as i32;
        pegs.push((x, y));
    }

    let mut darkness_map = vec![0.0f32; width * height];
    let src_pixels = source.as_slice();
    for i in 0..(width * height) {
        let color = src_pixels[i];
        let r = ((color >> 16) & 0xFF) as f32;
        let g = ((color >> 8) & 0xFF) as f32;
        let b = (color & 0xFF) as f32;
        let lum = (r * 0.299 + g * 0.587 + b * 0.114) / 255.0;
        darkness_map[i] = 1.0 - lum;
    }

    let num_pairs = config.num_pegs * config.num_pegs;
    let mut line_cache = Vec::new();
    let mut line_offsets = vec![0; num_pairs + 1];

    for i in 0..config.num_pegs {
        for j in 0..config.num_pegs {
            let pair_idx = i * config.num_pegs + j;
            line_offsets[pair_idx] = line_cache.len();

            let (x0, y0) = pegs[i];
            let (x1, y1) = pegs[j];

            let dx = (x1 - x0).abs();
            let dy = -(y1 - y0).abs();
            let mut err = dx + dy;
            let sx = if x0 < x1 { 1 } else { -1 };
            let sy = if y0 < y1 { 1 } else { -1 };

            let mut px = x0;
            let mut py = y0;

            loop {
                if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                    let idx = (py as usize) * width + (px as usize);
                    line_cache.push(idx);
                }
                if px == x1 && py == y1 {
                    break;
                }
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    px += sx;
                }
                if e2 <= dx {
                    err += dx;
                    py += sy;
                }
            }
        }
    }
    line_offsets[num_pairs] = line_cache.len();

    let mut output = Framebuffer::new(width as u32, height as u32).unwrap();
    output.clear(0xFFFF_FFFF); // White background

    let mut current_peg = 0;

    for _ in 0..config.num_lines {
        let mut best_peg = current_peg;
        let mut best_score = -1.0;

        for next_peg in 0..config.num_pegs {
            let diff = (next_peg as isize - current_peg as isize).abs();
            if diff < 10 || diff > (config.num_pegs as isize - 10) {
                continue;
            }

            let pair_idx = current_peg * config.num_pegs + next_peg;
            let start = line_offsets[pair_idx];
            let end = line_offsets[pair_idx + 1];
            let indices = &line_cache[start..end];

            let mut score = 0.0;
            for &idx in indices {
                score += darkness_map[idx];
            }
            let avg_score = if indices.is_empty() {
                0.0
            } else {
                score / indices.len() as f32
            };

            if avg_score > best_score {
                best_score = avg_score;
                best_peg = next_peg;
            }
        }

        let pair_idx = current_peg * config.num_pegs + best_peg;
        let start = line_offsets[pair_idx];
        let end = line_offsets[pair_idx + 1];
        let indices = &line_cache[start..end];

        for &idx in indices {
            let val: f32 = darkness_map[idx] - config.line_weight;
            darkness_map[idx] = if val < 0.0 { 0.0 } else { val };
        }

        let (x0, y0) = pegs[current_peg];
        let (x1, y1) = pegs[best_peg];
        draw_line_2d_alpha(&mut output, x0, y0, x1, y1, config.thread_color);

        current_peg = best_peg;
    }

    output
}

fn draw_line_2d_alpha(
    fb: &mut Framebuffer,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    color: u32,
) {
    let width = fb.width() as i32;
    let height = fb.height() as i32;

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let mut err = dx + dy;
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };

    let a = (color >> 24) & 0xFF;
    let fg_r = (color >> 16) & 0xFF;
    let fg_g = (color >> 8) & 0xFF;
    let fg_b = color & 0xFF;

    // Fast path for opaque
    if a == 255 {
        loop {
            if x0 >= 0 && x0 < width && y0 >= 0 && y0 < height {
                fb.set_pixel(x0, y0, color);
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
        return;
    }

    let alpha = a as f32 / 255.0;
    let inv_alpha = 1.0 - alpha;

    loop {
        if x0 >= 0 && x0 < width && y0 >= 0 && y0 < height {
            let bg = fb.as_slice()[(y0 * width + x0) as usize];
            let bg_r = (bg >> 16) & 0xFF;
            let bg_g = (bg >> 8) & 0xFF;
            let bg_b = bg & 0xFF;

            let r = ((fg_r as f32 * alpha) + (bg_r as f32 * inv_alpha)) as u32;
            let g = ((fg_g as f32 * alpha) + (bg_g as f32 * inv_alpha)) as u32;
            let b = ((fg_b as f32 * alpha) + (bg_b as f32 * inv_alpha)) as u32;

            fb.set_pixel(x0, y0, 0xFF00_0000 | (r << 16) | (g << 8) | b);
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
    fn test_string_art_generation() {
        let mut fb = Framebuffer::new(32, 32).unwrap();
        // create a white box with a black circle
        fb.clear(0xFFFF_FFFF);
        for y in 8..24 {
            for x in 8..24 {
                if (x - 16) * (x - 16) + (y - 16) * (y - 16) < 64 {
                    fb.set_pixel(x as i32, y as i32, 0xFF00_0000);
                }
            }
        }

        let config = StringArtConfig {
            num_pegs: 32,
            num_lines: 100,
            ..Default::default()
        };

        let output = generate_string_art(&fb, &config);

        assert_eq!(output.width(), 32);
        assert_eq!(output.height(), 32);

        // it should have drawn some dark pixels on the white background
        let mut has_dark = false;
        let mut has_white = false;
        for &p in output.as_slice() {
            if p == 0xFFFF_FFFF {
                has_white = true;
            } else {
                has_dark = true;
            }
        }
        assert!(has_white);
        assert!(has_dark);
    }
}
