use crate::framebuffer::Framebuffer;

/// A simple Linear Congruential Generator for deterministic random numbers.
struct Lcg {
    state: u32,
}

impl Lcg {
    const fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    const fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        self.state
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32) / (u32::MAX as f32)
    }
}

#[cfg(feature = "parallel")]
use rayon::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DistanceMetric {
    Euclidean,
    Manhattan,
    Chebyshev,
}

pub struct VoronoiFilter {
    num_seeds: usize,
    seed: u32,
    metric: DistanceMetric,
    draw_borders: bool,
}

impl VoronoiFilter {
    #[must_use]
    pub const fn new(num_seeds: usize, seed: u32) -> Self {
        Self {
            num_seeds,
            seed,
            metric: DistanceMetric::Euclidean,
            draw_borders: false,
        }
    }

    #[must_use]
    pub const fn with_metric(mut self, metric: DistanceMetric) -> Self {
        self.metric = metric;
        self
    }

    #[must_use]
    pub const fn with_borders(mut self, draw_borders: bool) -> Self {
        self.draw_borders = draw_borders;
        self
    }

    /// Applies the Voronoi filter to the given framebuffer.
    ///
    /// # Panics
    ///
    /// Panics if the framebuffer slice length does not perfectly match `width * height`.
    pub fn apply(&self, fb: &mut Framebuffer) {
        let width = fb.width() as usize;
        let height = fb.height() as usize;
        let mut rng = Lcg::new(self.seed);

        // Generate seeds
        let mut seeds = Vec::with_capacity(self.num_seeds);
        for _ in 0..self.num_seeds {
            let x = rng.next_f32() * (width as f32);
            let y = rng.next_f32() * (height as f32);
            let color = 0xFF00_0000 | (rng.next_u32() & 0x00FF_FFFF);
            seeds.push((x, y, color));
        }

        let metric = self.metric;
        let draw_borders = self.draw_borders;

        let dest = fb.as_mut_slice();
        assert_eq!(dest.len(), width * height);

        #[cfg(feature = "parallel")]
        let row_iter = dest.par_chunks_exact_mut(width).enumerate();
        #[cfg(not(feature = "parallel"))]
        let row_iter = dest.chunks_exact_mut(width).enumerate();

        row_iter.for_each(|(y, row)| {
            let fy = y as f32;
            for (x, pixel) in row.iter_mut().enumerate() {
                let fx = x as f32;

                let mut min_dist = f32::MAX;
                let mut min_dist_2 = f32::MAX;
                let mut closest_color = 0;

                for &(sx, sy, color) in &seeds {
                    let dx = (fx - sx).abs();
                    let dy = (fy - sy).abs();

                    let dist = match metric {
                        DistanceMetric::Euclidean => dx.hypot(dy),
                        DistanceMetric::Manhattan => dx + dy,
                        DistanceMetric::Chebyshev => dx.max(dy),
                    };

                    if dist < min_dist {
                        min_dist_2 = min_dist;
                        min_dist = dist;
                        closest_color = color;
                    } else if dist < min_dist_2 {
                        min_dist_2 = dist;
                    }
                }

                if draw_borders && (min_dist_2 - min_dist) < 1.5 {
                    *pixel = 0xFF00_0000; // Black border
                } else {
                    *pixel = closest_color;
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_voronoi_filter_basic() {
        let mut fb = Framebuffer::new(16, 16).unwrap();
        fb.clear(0xFF00_0000); // Black

        let filter = VoronoiFilter::new(4, 12345);
        filter.apply(&mut fb);

        // At least one pixel should be non-black (the seeds will generate some color)
        let mut has_color = false;
        for y in 0..16 {
            for x in 0..16 {
                if fb.get_pixel(x, y).unwrap() != 0xFF00_0000 {
                    has_color = true;
                    break;
                }
            }
        }
        assert!(has_color, "Voronoi filter should modify the framebuffer");
    }
}
