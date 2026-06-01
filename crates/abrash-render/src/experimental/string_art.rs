use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec2;
#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// A generator for creating String Art from a target image.
pub struct StringArtGenerator {
    target: Framebuffer,
    num_pegs: usize,
    pegs: Vec<Vec2>,
    lines: usize,
}

impl StringArtGenerator {
    /// Creates a new generator with the given target image and parameters.
    pub fn new(target: Framebuffer, num_pegs: usize, lines: usize) -> Self {
        let width = target.width() as f32;
        let height = target.height() as f32;
        let radius = width.min(height) / 2.0 - 2.0;
        let cx = width / 2.0;
        let cy = height / 2.0;

        let mut pegs = Vec::with_capacity(num_pegs);
        for i in 0..num_pegs {
            let angle = i as f32 * 2.0 * std::f32::consts::PI / num_pegs as f32;
            let x = cx + radius * angle.cos();
            let y = cy + radius * angle.sin();
            pegs.push(Vec2::new(x, y));
        }

        Self {
            target,
            num_pegs,
            pegs,
            lines,
        }
    }

    /// Generates the sequence of peg indices that best match the target image.
    pub fn generate(&mut self) -> Vec<usize> {
        let width = self.target.width() as usize;
        let height = self.target.height() as usize;

        // Invert image to "darkness" map (0 = white, 255 = black)
        // because we are adding lines to reduce darkness.
        let mut error_map = vec![0.0; width * height];
        for y in 0..height {
            for x in 0..width {
                let pixel = self.target.get_pixel(x as i32, y as i32).unwrap_or(0);
                let r = ((pixel >> 16) & 0xFF) as f32;
                let g = ((pixel >> 8) & 0xFF) as f32;
                let b = (pixel & 0xFF) as f32;
                // Simple grayscale
                let luma = 0.299 * r + 0.587 * g + 0.114 * b;
                error_map[y * width + x] = 255.0 - luma; // Higher means darker, needs line
            }
        }

        // Pre-calculate lines between all peg pairs
        // Cache them as 1D array indices to elide bounds checking later
        // line_cache[p0][p1] -> Vec<usize>
        let mut line_cache = vec![vec![Vec::new(); self.num_pegs]; self.num_pegs];
        for i in 0..self.num_pegs {
            for j in 0..self.num_pegs {
                if i == j {
                    continue;
                }
                let dist = (j as i32 - i as i32).abs();
                let dist = dist.min(self.num_pegs as i32 - dist);
                if dist < 10 {
                    continue; // Skip adjacent pegs
                }

                let p0 = self.pegs[i];
                let p1 = self.pegs[j];

                let line_pixels = bresenham(p0.x as i32, p0.y as i32, p1.x as i32, p1.y as i32);

                let mut indices = Vec::with_capacity(line_pixels.len());
                for &(x, y) in &line_pixels {
                    if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
                        indices.push(y as usize * width + x as usize);
                    }
                }
                line_cache[i][j] = indices;
            }
        }

        let mut path = Vec::with_capacity(self.lines + 1);
        let mut current_peg = 0;
        path.push(current_peg);

        for _ in 0..self.lines {
            let current_cache = &line_cache[current_peg];

            #[cfg(feature = "parallel")]
            let best_result = {
                (0..self.num_pegs).into_par_iter().map(|next_peg| {
                    if next_peg == current_peg {
                        return (-1.0, next_peg, &[][..]);
                    }
                    let indices: &[usize] = &current_cache[next_peg];
                    if indices.is_empty() {
                        return (-1.0, next_peg, &[][..]);
                    }

                    let mut score = 0.0;
                    for &idx in indices {
                        score += error_map[idx];
                    }
                    score /= indices.len() as f32;

                    (score, next_peg, indices)
                }).max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            };

            #[cfg(not(feature = "parallel"))]
            let best_result = {
                (0..self.num_pegs).into_iter().map(|next_peg| {
                    if next_peg == current_peg {
                        return (-1.0, next_peg, &[][..]);
                    }
                    let indices: &[usize] = &current_cache[next_peg];
                    if indices.is_empty() {
                        return (-1.0, next_peg, &[][..]);
                    }

                    let mut score = 0.0;
                    for &idx in indices {
                        score += error_map[idx];
                    }
                    score /= indices.len() as f32;

                    (score, next_peg, indices)
                }).max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            };

            if let Some((_, best_peg, best_indices)) = best_result {
                path.push(best_peg);

                // Subtract the line from the error map
                for &idx in best_indices {
                    error_map[idx] = (error_map[idx] - 20.0).max(0.0);
                }

                current_peg = best_peg;
            } else {
                break;
            }
        }

        path
    }
}

// Simple Bresenham line algorithm
fn bresenham(mut x0: i32, mut y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
    let mut pixels = Vec::new();
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        pixels.push((x0, y0));
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
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generator_initialization() {
        let fb = Framebuffer::new(100, 100).unwrap();
        let generator = StringArtGenerator::new(fb, 288, 1000);
        assert_eq!(generator.num_pegs, 288);
        assert_eq!(generator.lines, 1000);
    }

    #[test]
    fn test_generator_produces_path() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        // Draw something simple: a dark circle in the middle
        for y in 25..75 {
            for x in 25..75 {
                let dx = x as f32 - 50.0;
                let dy = y as f32 - 50.0;
                if dx * dx + dy * dy < 25.0 * 25.0 {
                    fb.set_pixel(x, y, 0xFF00_0000); // Black
                }
            }
        }
        let mut generator = StringArtGenerator::new(fb, 288, 10);
        let path = generator.generate();
        assert!(!path.is_empty(), "Generator should produce a non-empty path");
        assert_eq!(path.len(), 11, "Path should have lines + 1 elements");
    }
}
