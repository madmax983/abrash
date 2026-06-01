use crate::framebuffer::Framebuffer;
use std::f32::consts::PI;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Simulates a string art image using a sequence of connecting lines.
///
/// Converts the target image into a darkness map and iteratively draws a single continuous
/// "thread" between an array of virtual pegs arranged in a circle, carving away
/// at the darkness map to minimize the overall error against the original image.
///
/// * `fb` - The input framebuffer, will be overwritten with the generated lines.
/// * `num_pegs` - Number of pegs placed around the image perimeter. (e.g. 288)
/// * `num_lines` - Number of line segments (threads) to draw. (e.g. 2000-4000)
/// * `line_weight` - Opacity of the thread (0.0 to 1.0). (e.g. 0.1)
pub fn apply_string_art(fb: &mut Framebuffer, num_pegs: usize, num_lines: usize, line_weight: f32) {
    if num_pegs < 2 || num_lines == 0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;
    if width == 0 || height == 0 {
        return;
    }

    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0;
    let radius = (center_x.min(center_y)) - 2.0;

    // 1. Convert Framebuffer to a Darkness Map (0.0 = white, 1.0 = black)
    // We carve away from darkness. The target is an array of f32 for precise error calculation.
    let mut darkness_map = vec![0.0f32; width * height];
    let source_pixels = fb.as_slice();

    for y in 0..height {
        for x in 0..width {
            let p = source_pixels[y * width + x];
            let r = ((p >> 16) & 0xFF) as f32;
            let g = ((p >> 8) & 0xFF) as f32;
            let b = (p & 0xFF) as f32;

            // Luminance: standard Rec. 709
            let luminance = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.0;

            darkness_map[y * width + x] = (1.0 - luminance).clamp(0.0, 1.0);
        }
    }

    // 2. Pre-calculate Peg Positions
    let mut pegs = Vec::with_capacity(num_pegs);
    for i in 0..num_pegs {
        let angle = (i as f32 / num_pegs as f32) * 2.0 * PI;
        let px = (center_x + radius * angle.cos()).clamp(0.0, width as f32 - 1.0) as i32;
        let py = (center_y + radius * angle.sin()).clamp(0.0, height as f32 - 1.0) as i32;
        pegs.push((px, py));
    }

    // 3. Pre-calculate line pixel indices for all valid peg connections into a flat matrix
    let mut line_indices = Vec::new(); // Flat array of all pixel indices for all lines
    let mut line_spans = vec![(0usize, 0usize); num_pegs * num_pegs]; // (start, length) for each line

    for p1_idx in 0..num_pegs {
        for p2_idx in 0..num_pegs {
            // Skip invalid pairs
            if p2_idx == p1_idx
                || (p2_idx as i32 - p1_idx as i32).abs() < 10
                || (p2_idx as i32 - p1_idx as i32).abs() > (num_pegs as i32 - 10)
            {
                continue;
            }

            let p1 = pegs[p1_idx];
            let p2 = pegs[p2_idx];

            let start_idx = line_indices.len();
            let mut count = 0;

            let mut x = p1.0;
            let mut y = p1.1;
            let dx = (p2.0 - p1.0).abs();
            let sx = if p1.0 < p2.0 { 1 } else { -1 };
            let dy = -(p2.1 - p1.1).abs();
            let sy = if p1.1 < p2.1 { 1 } else { -1 };
            let mut err = dx + dy;

            loop {
                if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
                    line_indices.push((y as usize) * width + (x as usize));
                    count += 1;
                }

                if x == p2.0 && y == p2.1 {
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
            line_spans[p1_idx * num_pegs + p2_idx] = (start_idx, count);
        }
    }

    // Prepare an empty (white) output buffer to draw the final lines
    let mut output_pixels = vec![0xFFFF_FFFFu32; width * height];

    let mut current_peg = 0;
    let weight_to_carve = line_weight;
    let mut prev_peg = current_peg;

    // 4. Iteratively find the best line to draw
    for _ in 0..num_lines {
        let best_peg: usize;

        #[cfg(not(feature = "parallel"))]
        {
            let mut max_score = -1.0;
            let mut best_p = current_peg;

            for next_peg in 0..num_pegs {
                let (start, len) = line_spans[current_peg * num_pegs + next_peg];
                if len == 0 {
                    continue;
                }
                if next_peg == prev_peg {
                    continue;
                }

                let end = start + len;
                let indices = &line_indices[start..end];

                let mut score = 0.0;
                for &idx in indices {
                    score += darkness_map[idx];
                }

                let avg_score = score / len as f32;

                if avg_score > max_score {
                    max_score = avg_score;
                    best_p = next_peg;
                }
            }
            best_peg = best_p;
        }

        #[cfg(feature = "parallel")]
        {
            let (best_p, _max_score) = (0..num_pegs)
                .into_par_iter()
                .map(|next_peg| {
                    let (start, len) = line_spans[current_peg * num_pegs + next_peg];
                    if len == 0 || next_peg == prev_peg {
                        return (next_peg, -1.0_f32);
                    }

                    let end = start + len;
                    let indices = &line_indices[start..end];

                    let mut score = 0.0;
                    for &idx in indices {
                        // Safe because we pre-calculated indices within bounds
                        score += darkness_map[idx];
                    }

                    let avg_score = score / len as f32;

                    (next_peg, avg_score)
                })
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or((current_peg, -1.0));

            best_peg = best_p;
        }

        // If we couldn't find a good peg, just stop
        if best_peg == current_peg {
            break;
        }

        // 5. "Draw" the selected line: carve away from the darkness map and draw on output
        let (start, len) = line_spans[current_peg * num_pegs + best_peg];
        let end = start + len;
        let indices = &line_indices[start..end];
        for &idx in indices {
            // Carve away the darkness
            darkness_map[idx] = (darkness_map[idx] - weight_to_carve).max(0.0);

            // Blend black into the output image based on line weight
            let current_color = output_pixels[idx];
            let bg_r = ((current_color >> 16) & 0xFF) as f32;
            let bg_g = ((current_color >> 8) & 0xFF) as f32;
            let bg_b = (current_color & 0xFF) as f32;

            let inv_w = 1.0 - weight_to_carve;
            let r = (bg_r * inv_w) as u32;
            let g = (bg_g * inv_w) as u32;
            let b = (bg_b * inv_w) as u32;

            output_pixels[idx] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
        }

        prev_peg = current_peg;
        current_peg = best_peg;
    }

    // 6. Copy the final string art back to the framebuffer
    fb.as_mut_slice().copy_from_slice(&output_pixels);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_apply_string_art() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        // Fill with black to represent darkness. The algorithm should draw black lines on white bg.
        fb.clear(0xFF00_0000);

        apply_string_art(&mut fb, 50, 100, 0.5);

        let has_thread = fb.as_slice().iter().any(|&p| p != 0xFFFF_FFFF);
        assert!(
            has_thread,
            "apply_string_art should draw lines onto the framebuffer"
        );
    }
}
