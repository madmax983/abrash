use abrash_core::math::Vec2;
use std::f32::consts::PI;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Precomputed line segments between two pins
#[derive(Clone, Default)]
struct CachedLine {
    // Storing pre-calculated indices (y * width + x) to avoid repeated multiplication during evaluate and draw
    indices: Vec<usize>,
}

/// String art generator.
pub struct StringArt {
    pub pins: Vec<Vec2>,
    lines_cache: Vec<Vec<CachedLine>>,
}

impl StringArt {
    pub fn new(num_pins: usize, center: Vec2, radius: f32, width: usize, height: usize) -> Self {
        let mut pins = Vec::with_capacity(num_pins);
        for i in 0..num_pins {
            let angle = (i as f32) / (num_pins as f32) * 2.0 * PI;
            let x = center.x + radius * angle.cos();
            let y = center.y + radius * angle.sin();
            pins.push(Vec2::new(x, y));
        }

        let mut lines_cache = vec![vec![CachedLine::default(); num_pins]; num_pins];
        for i in 0..num_pins {
            for j in 0..num_pins {
                if i != j && i != (j + 1) % num_pins && i != (j + num_pins - 1) % num_pins {
                    lines_cache[i][j] = Self::compute_line_indices(pins[i], pins[j], width, height);
                }
            }
        }

        Self { pins, lines_cache }
    }

    fn compute_line_indices(p0: Vec2, p1: Vec2, width: usize, height: usize) -> CachedLine {
        let mut indices = Vec::new();
        let mut x0 = p0.x as isize;
        let mut y0 = p0.y as isize;
        let x1 = p1.x as isize;
        let y1 = p1.y as isize;

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if x0 >= 0 && x0 < width as isize && y0 >= 0 && y0 < height as isize {
                indices.push((y0 as usize) * width + (x0 as usize));
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

        CachedLine { indices }
    }

    pub fn generate(&mut self, image: &mut [u8], _width: usize, _height: usize, num_lines: usize) -> Vec<(usize, usize)> {
        if self.pins.is_empty() || num_lines == 0 {
            return Vec::new();
        }

        let mut lines = Vec::with_capacity(num_lines);
        let mut current_pin = 0;
        let num_pins = self.pins.len();

        for _ in 0..num_lines {
            #[cfg(feature = "parallel")]
            let (best_next_pin, best_score) = (0..num_pins).into_par_iter().map(|next_pin| {
                if next_pin == current_pin || next_pin == (current_pin + 1) % num_pins || next_pin == (current_pin + num_pins - 1) % num_pins {
                    (next_pin, -1.0)
                } else {
                    let score = Self::evaluate_cached_line(image, &self.lines_cache[current_pin][next_pin]);
                    (next_pin, score)
                }
            }).reduce(|| (current_pin, -1.0), |a, b| if a.1 > b.1 { a } else { b });

            #[cfg(not(feature = "parallel"))]
            let (best_next_pin, best_score) = {
                let mut best_next_pin = current_pin;
                let mut best_score = -1.0;

                for next_pin in 0..num_pins {
                    if next_pin == current_pin || next_pin == (current_pin + 1) % num_pins || next_pin == (current_pin + num_pins - 1) % num_pins {
                        continue;
                    }

                    let score = Self::evaluate_cached_line(image, &self.lines_cache[current_pin][next_pin]);
                    if score > best_score {
                        best_score = score;
                        best_next_pin = next_pin;
                    }
                }
                (best_next_pin, best_score)
            };

            if best_next_pin == current_pin {
                break;
            }

            Self::draw_cached_line(image, &self.lines_cache[current_pin][best_next_pin], 50);
            lines.push((current_pin, best_next_pin));
            current_pin = best_next_pin;
        }

        lines
    }

    #[inline(always)]
    fn evaluate_cached_line(image: &[u8], line: &CachedLine) -> f32 {
        let mut score = 0.0;
        let count = line.indices.len();

        for &idx in &line.indices {
            // direct array access is fast since we pre-checked bounds during cache creation
            let darkness = 255.0 - (image[idx] as f32);
            score += darkness;
        }

        if count > 0 {
            score / (count as f32)
        } else {
            0.0
        }
    }

    #[inline(always)]
    fn draw_cached_line(image: &mut [u8], line: &CachedLine, lighten_amount: u8) {
        for &idx in &line.indices {
            image[idx] = image[idx].saturating_add(lighten_amount);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pin_generation() {
        let art = StringArt::new(4, Vec2::new(10.0, 10.0), 5.0, 20, 20);
        assert_eq!(art.pins.len(), 4);
    }

    #[test]
    fn test_basic_string_art_sequence() {
        let mut art = StringArt::new(16, Vec2::new(10.0, 10.0), 5.0, 20, 20);
        let mut image = vec![0u8; 20 * 20];
        let lines = art.generate(&mut image, 20, 20, 5);
        assert_eq!(lines.len(), 5);
    }
}
