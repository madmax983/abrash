//! String Art Generator Filter
//!
//! A post-processing effect that converts an image into string art (thread art)
//! by stretching continuous straight lines between points (pegs) on a perimeter
//! circle to approximate the darkness of the original image.

use abrash_core::framebuffer::Framebuffer;

/// Applies a string art generation effect to the framebuffer.
///
/// It treats the image as a darkness map, places pegs along a circle in the center
/// of the image, and uses a greedy algorithm to draw lines between pegs that
/// pass over the darkest remaining areas.
///
/// # Panics
///
/// Panics if the internal framebuffer allocation or intermediate maps fail due to memory exhaustion.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `num_pegs` - The number of points around the circular perimeter.
/// * `num_lines` - The number of string lines to draw.
/// * `line_weight` - The darkness/opacity of each drawn line (0.0 to 1.0).
pub fn apply_string_art(fb: &mut Framebuffer, num_pegs: usize, num_lines: usize, line_weight: f32) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    if width == 0 || height == 0 || num_pegs < 2 || num_lines == 0 {
        return;
    }

    // 1. Calculate darkness map (0.0 = white, 1.0 = black).
    // We treat the current framebuffer as the target image.
    let mut darkness = vec![0.0f32; width * height];
    let pixels = fb.as_slice();
    for (i, &color) in pixels.iter().enumerate() {
        let r = ((color >> 16) & 0xFF) as f32;
        let g = ((color >> 8) & 0xFF) as f32;
        let b = (color & 0xFF) as f32;
        // Simple luminance calculation.
        let lum = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;
        darkness[i] = 1.0 - lum;
    }

    // 2. Pre-calculate peg positions around a circle.
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    let radius = (width.min(height) as f32 / 2.0) - 1.0;

    let mut pegs = Vec::with_capacity(num_pegs);
    for i in 0..num_pegs {
        let angle = (i as f32 / num_pegs as f32) * std::f32::consts::PI * 2.0;
        let px = cx + angle.cos() * radius;
        let py = cy + angle.sin() * radius;
        pegs.push((px as i32, py as i32));
    }

    // 3. Clear the framebuffer to white, since we are drawing the strings on a clean canvas.
    fb.clear(0xFFFF_FFFF);

    // 4. Greedily find lines.
    let mut current_peg = 0;
    let weight_reduction = line_weight;

    for _ in 0..num_lines {
        let mut best_peg = current_peg;
        let mut best_score = -1.0;
        let mut best_line_pixels = Vec::new();

        // Check all other pegs to find the one that passes over the most darkness.
        for next_peg in 0..num_pegs {
            // Don't draw to the immediate neighbors or self to avoid perimeter drawing
            let diff = (next_peg as isize - current_peg as isize).abs();
            let dist = diff.min(num_pegs as isize - diff);
            if dist < 5 {
                continue;
            }

            let p0 = pegs[current_peg];
            let p1 = pegs[next_peg];

            // Bresenham's line algorithm to collect pixels and score
            let mut x0 = p0.0;
            let mut y0 = p0.1;
            let x1 = p1.0;
            let y1 = p1.1;

            let dx = (x1 - x0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let dy = -(y1 - y0).abs();
            let sy = if y0 < y1 { 1 } else { -1 };
            let mut err = dx + dy;

            let mut score = 0.0;
            let mut count = 0;

            // ⚡ Bolt: Using a pre-allocated vector and clearing it avoids repeated heap allocations in this hot loop.
            // Even better: since we just need the max score, we can only collect the line pixels when we actually
            // find a new best score, saving massive allocation overhead inside the inner loop.
            // We just re-simulate the line below if it's chosen!

            loop {
                // Inline bounds checking
                if (x0 as u32) < width as u32 && (y0 as u32) < height as u32 {
                    let idx = (y0 as usize) * width + (x0 as usize);
                    score += darkness[idx];
                    count += 1;
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

            // Average score per pixel of the line
            if count > 0 {
                let avg_score = score / count as f32;
                if avg_score > best_score {
                    best_score = avg_score;
                    best_peg = next_peg;
                }
            }
        }

        // Now that we have the best peg, we re-trace the line to collect the pixels so we can subtract darkness.
        // This trades a tiny bit of computation (re-running Bresenham once per line) for a massive reduction
        // in heap allocations (avoiding allocating a Vec for every single peg checked).
        if best_peg != current_peg && best_score >= 0.0 {
            let p0 = pegs[current_peg];
            let p1 = pegs[best_peg];

            let mut x0 = p0.0;
            let mut y0 = p0.1;
            let x1 = p1.0;
            let y1 = p1.1;

            let dx = (x1 - x0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let dy = -(y1 - y0).abs();
            let sy = if y0 < y1 { 1 } else { -1 };
            let mut err = dx + dy;

            loop {
                if (x0 as u32) < width as u32 && (y0 as u32) < height as u32 {
                    let idx = (y0 as usize) * width + (x0 as usize);
                    best_line_pixels.push(idx);
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

        // Draw the best line and reduce darkness along its path
        if best_peg != current_peg && !best_line_pixels.is_empty() {
            let p0 = pegs[current_peg];
            let p1 = pegs[best_peg];

            // Subtract darkness
            for &idx in &best_line_pixels {
                darkness[idx] = (darkness[idx] - weight_reduction).max(0.0);
            }

            // Draw line on framebuffer (simple black line with alpha/weight simulation)
            let draw_color = 0xFF00_0000u32; // Black

            let mut x0 = p0.0;
            let mut y0 = p0.1;
            let x1 = p1.0;
            let y1 = p1.1;

            let dx = (x1 - x0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let dy = -(y1 - y0).abs();
            let sy = if y0 < y1 { 1 } else { -1 };
            let mut err = dx + dy;

            let fb_pixels = fb.as_mut_slice();

            // Very simple alpha blending for black line
            let alpha = (line_weight * 255.0) as u32;
            let inv_alpha = 255 - alpha;

            loop {
                if x0 >= 0 && x0 < width as i32 && y0 >= 0 && y0 < height as i32 {
                    let idx = (y0 as usize) * width + (x0 as usize);
                    let bg = fb_pixels[idx];
                    let r = (((bg >> 16) & 0xFF) * inv_alpha) / 255;
                    let g = (((bg >> 8) & 0xFF) * inv_alpha) / 255;
                    let b = ((bg & 0xFF) * inv_alpha) / 255;
                    fb_pixels[idx] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
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

            current_peg = best_peg;
        } else {
            // No good line found, stop early
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::framebuffer::Framebuffer;

    #[test]
    fn test_apply_string_art_draws_lines() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        // Clear with white
        fb.clear(0xFFFF_FFFF);
        // Draw a black dot in the center to attract lines
        fb.set_pixel(50, 50, 0xFF00_0000);

        apply_string_art(&mut fb, 100, 10, 0.5);

        // A functioning string art algorithm should have drawn some lines.
        // It shouldn't just be pure white with one black dot anymore.
        let non_white_pixels = fb.as_slice().iter().filter(|&&p| p != 0xFFFF_FFFF).count();
        // Expect more than just the center dot to be non-white.
        assert!(non_white_pixels > 1, "Expected lines to be drawn");
    }
}
