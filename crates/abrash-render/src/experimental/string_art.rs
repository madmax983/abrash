use std::f32::consts::PI;

/// A generator for string art that simulates thread woven between pegs on a circle.
pub struct StringArt {
    num_pegs: usize,
    #[allow(dead_code)]
    width: usize,
    #[allow(dead_code)]
    height: usize,
    peg_positions: Vec<(f32, f32)>,
    /// Flattened cache of pixel indices between pairs of pegs to avoid Bresenham recalculation.
    /// Access via: `line_cache_flat[line_cache_offsets[p1 * num_pegs + p2].0 .. + .1]`
    line_cache_flat: Vec<usize>,
    line_cache_offsets: Vec<(usize, usize)>,
}

impl StringArt {
    /// Creates a new StringArt generator.
    pub fn new(num_pegs: usize, width: usize, height: usize) -> Self {
        let radius = (width.min(height) as f32 / 2.0) - 1.0;
        let center_x = width as f32 / 2.0;
        let center_y = height as f32 / 2.0;

        let mut peg_positions = Vec::with_capacity(num_pegs);
        for i in 0..num_pegs {
            let angle = (i as f32 / num_pegs as f32) * 2.0 * PI;
            let x = center_x + radius * angle.cos();
            let y = center_y + radius * angle.sin();
            peg_positions.push((x, y));
        }

        // Precalculate Bresenham lines to cache array indices.
        // We use a flat Vec for caching instead of nested Vec<Vec> to avoid N^2 heap allocations.
        let total_lines = num_pegs * num_pegs;
        let estimated_line_len = width.max(height);

        let mut line_cache_flat = Vec::with_capacity(total_lines * estimated_line_len);
        let mut line_cache_offsets = Vec::with_capacity(total_lines);

        for p1 in 0..num_pegs {
            for p2 in 0..num_pegs {
                if p1 == p2 {
                    line_cache_offsets.push((0, 0));
                    continue;
                }

                let start_idx = line_cache_flat.len();
                let (x0, y0) = peg_positions[p1];
                let (x1, y1) = peg_positions[p2];

                let mut x0 = x0 as isize;
                let mut y0 = y0 as isize;
                let x1 = x1 as isize;
                let y1 = y1 as isize;

                let dx = (x1 - x0).abs();
                let dy = -(y1 - y0).abs();
                let sx = if x0 < x1 { 1 } else { -1 };
                let sy = if y0 < y1 { 1 } else { -1 };
                let mut err = dx + dy;

                loop {
                    if x0 >= 0 && x0 < width as isize && y0 >= 0 && y0 < height as isize {
                        let idx = y0 as usize * width + x0 as usize;
                        line_cache_flat.push(idx);
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

                let len = line_cache_flat.len() - start_idx;
                line_cache_offsets.push((start_idx, len));
            }
        }

        Self {
            num_pegs,
            width,
            height,
            peg_positions,
            line_cache_flat,
            line_cache_offsets,
        }
    }

    /// Generates a sequence of peg indices to form the string art.
    /// Returns a list of peg indices.
    ///
    /// # Panics
    /// Panics if the `darkness_map` length does not exactly match `width * height`.
    pub fn generate(&self, darkness_map: &[u8], num_lines: usize) -> Vec<usize> {
        assert_eq!(
            darkness_map.len(),
            self.width * self.height,
            "darkness_map length must match width * height"
        );
        let mut sequence = Vec::with_capacity(num_lines + 1);
        let mut current_peg = 0;
        sequence.push(current_peg);

        // We make a mutable copy of the map so we can "clear" the path we take.
        let mut map = darkness_map.to_vec();

        for _ in 0..num_lines {
            let mut best_peg = current_peg;
            let mut max_score = -1.0;

            // Find the peg that yields the line with the highest darkness score
            for next_peg in 0..self.num_pegs {
                if next_peg == current_peg {
                    continue;
                }

                let score = self.evaluate_line(current_peg, next_peg, &map);
                if score > max_score {
                    max_score = score;
                    best_peg = next_peg;
                }
            }

            // Once the best next peg is found, update sequence and subtract line from the map
            sequence.push(best_peg);
            self.draw_line_on_map(current_peg, best_peg, &mut map);
            current_peg = best_peg;
        }

        sequence
    }

    /// Calculates the (x, y) coordinate of a peg.
    pub fn peg_position(&self, peg_index: usize) -> (f32, f32) {
        self.peg_positions[peg_index]
    }

    /// Evaluates the score of a line between two pegs using the cached line indices.
    fn evaluate_line(&self, peg1: usize, peg2: usize, map: &[u8]) -> f32 {
        let offset_idx = peg1 * self.num_pegs + peg2;
        let (start, len) = self.line_cache_offsets[offset_idx];

        if len == 0 {
            return 0.0;
        }

        let mut sum = 0.0;
        let line_indices = &self.line_cache_flat[start..start + len];

        for &idx in line_indices {
            // Safety: map indices were bounds-checked during precalculation
            sum += map[idx] as f32;
        }

        sum / len as f32
    }

    /// Reduces the darkness of pixels along the line using the cached line indices.
    fn draw_line_on_map(&self, peg1: usize, peg2: usize, map: &mut [u8]) {
        let offset_idx = peg1 * self.num_pegs + peg2;
        let (start, len) = self.line_cache_offsets[offset_idx];

        if len == 0 {
            return;
        }

        let line_indices = &self.line_cache_flat[start..start + len];

        for &idx in line_indices {
            // Subtract the simulated thickness of the thread, clamped to zero
            map[idx] = map[idx].saturating_sub(20);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peg_position() {
        let art = StringArt::new(4, 100, 100);
        // Pegs should be at the edges of the circle (radius = (100/2) - 1 = 49)
        // Center is (50, 50). So angle 0 should give x = 50 + 49 = 99, y = 50.
        let (x0, y0) = art.peg_position(0);
        assert!((x0 - 99.0).abs() < 1.0 && (y0 - 50.0).abs() < 1.0); // Right edge
    }

    #[test]
    fn test_generate_string_art() {
        let art = StringArt::new(100, 100, 100);
        let mut map = vec![0; 100 * 100];
        // Draw a dark line across the middle
        for x in 0..100 {
            map[50 * 100 + x] = 255; // 255 = maximum darkness
        }

        let sequence = art.generate(&map, 10);
        assert!(!sequence.is_empty(), "Sequence should not be empty");
        assert!(sequence.len() > 1, "Sequence should contain multiple pegs");
    }
}
