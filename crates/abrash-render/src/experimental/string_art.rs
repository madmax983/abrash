use abrash_core::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Applies the String Art generation effect.
///
/// This effect iteratively draws lines between pegs arranged on a circle to reproduce
/// the input image using thread-like structures.
///
/// * `fb`: The source Framebuffer to transform into string art.
/// * `num_pegs`: Number of pegs along the circle (e.g., 288).
/// * `num_lines`: Total number of string lines to draw (e.g., 3000).
pub fn apply_string_art(fb: &mut Framebuffer, num_pegs: usize, num_lines: usize) {
    let width = fb.width() as i32;
    let height = fb.height() as i32;

    if width == 0 || height == 0 || num_pegs < 2 || num_lines == 0 {
        return;
    }

    let cx = width / 2;
    let cy = height / 2;
    // Radius with a little padding
    let radius = (width.min(height) / 2) as f32 - 2.0;

    // 1. Calculate peg positions
    let mut pegs = Vec::with_capacity(num_pegs);
    for i in 0..num_pegs {
        let angle = (i as f32) * std::f32::consts::TAU / (num_pegs as f32);
        let x = cx as f32 + radius * angle.cos();
        let y = cy as f32 + radius * angle.sin();
        pegs.push((x as i32, y as i32));
    }

    // 2. Create darkness map (inverted luminance)
    // 0 = bright/white, 255 = dark/black
    let pixels = fb.as_mut_slice();
    let mut darkness_map = vec![0u8; pixels.len()];

    #[cfg(feature = "parallel")]
    let map_iter = darkness_map.par_iter_mut().zip(pixels.par_iter());
    #[cfg(not(feature = "parallel"))]
    let map_iter = darkness_map.iter_mut().zip(pixels.iter());

    map_iter.for_each(|(darkness, &pixel)| {
        let r = ((pixel >> 16) & 0xFF) as u32;
        let g = ((pixel >> 8) & 0xFF) as u32;
        let b = (pixel & 0xFF) as u32;

        // Rec. 709 luminance (scaled to 0..255)
        let lum = (19595 * r + 38469 * g + 7471 * b) >> 16;
        *darkness = 255 - lum.min(255) as u8;
    });

    // Clear the original framebuffer to white, we'll draw the string art on it.
    fb.clear(0xFFFF_FFFF);

    let mut current_peg = 0;

    // Line drawing parameters
    let line_weight = 20; // How much darkness a line subtracts

    // 3. Iteratively draw lines
    for _ in 0..num_lines {
        let mut best_score = -1;
        let mut best_peg = current_peg;

        let (x0, y0) = pegs[current_peg];

        // Evaluate all possible lines to other pegs
        for next_peg in 0..num_pegs {
            // Don't draw to the same peg or adjacent pegs
            if next_peg == current_peg
                || next_peg == (current_peg + 1) % num_pegs
                || next_peg == (current_peg + num_pegs - 1) % num_pegs
            {
                continue;
            }

            let (x1, y1) = pegs[next_peg];

            // Evaluate line score using Bresenham's
            let mut score = 0;
            let mut dx = (x1 - x0).abs();
            let mut sx = if x0 < x1 { 1 } else { -1 };
            let mut dy = -(y1 - y0).abs();
            let mut sy = if y0 < y1 { 1 } else { -1 };
            let mut err = dx + dy;

            let mut px = x0;
            let mut py = y0;

            loop {
                if px >= 0 && px < width && py >= 0 && py < height {
                    let idx = (py * width + px) as usize;
                    score += i32::from(darkness_map[idx]);
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

            if score > best_score {
                best_score = score;
                best_peg = next_peg;
            }
        }

        // Draw the winning line and reduce darkness
        let (x1, y1) = pegs[best_peg];

        let mut dx = (x1 - x0).abs();
        let mut sx = if x0 < x1 { 1 } else { -1 };
        let mut dy = -(y1 - y0).abs();
        let mut sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let mut px = x0;
        let mut py = y0;

        loop {
            if px >= 0 && px < width && py >= 0 && py < height {
                let idx = (py * width + px) as usize;
                // Subtract darkness from the map to prevent drawing the same line repeatedly
                darkness_map[idx] = darkness_map[idx].saturating_sub(line_weight);

                // Draw a dark semi-transparent pixel on the framebuffer for the string
                let fb_idx = (py * width + px) as usize;

                // Read current pixel, blend string color (e.g., black string with some opacity)
                let current_color = fb.as_slice()[fb_idx];

                // Simple alpha blend (burn)
                let r = ((current_color >> 16) & 0xFF) as i32;
                let g = ((current_color >> 8) & 0xFF) as i32;
                let b = (current_color & 0xFF) as i32;

                // Darken by line_weight
                let nr = r.saturating_sub(i32::from(line_weight)).max(0) as u32;
                let ng = g.saturating_sub(i32::from(line_weight)).max(0) as u32;
                let nb = b.saturating_sub(i32::from(line_weight)).max(0) as u32;

                fb.as_mut_slice()[fb_idx] = 0xFF00_0000 | (nr << 16) | (ng << 8) | nb;
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

        current_peg = best_peg;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "nova")]
    fn test_string_art_generation() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        // Clear to white
        fb.clear(0xFF_FF_FF_FF);

        // Draw a dark square in the middle to give the string art something to find
        fb.clear_rect(30, 30, 40, 40, 0xFF_00_00_00);

        apply_string_art(&mut fb, 100, 500);

        // After string art generation, we expect lines to be drawn over the white areas.
        // Some pixels should no longer be pure white.
        // Or pure black if the algorithm modifies the whole canvas (typically clearing it first).

        // Let's assume the canvas is cleared to white at the start, and dark lines are drawn.
        // The implementation should clear the canvas at the end and replace it with just the strings.
        let has_string_pixels = fb
            .as_slice()
            .iter()
            .any(|&p| p != 0xFF_FF_FF_FF && p != 0xFF_00_00_00);
        assert!(
            has_string_pixels,
            "String art did not generate any new string pixels."
        );
    }
}
