//! # Strange Attractor Generator
//!
//! An experimental procedural generator that simulates chaotic systems, specifically
//! Strange Attractors (like Clifford and Peter de Jong attractors).
//!
//! ## Overview
//!
//! Strange attractors are mathematical systems that exhibit chaotic behavior, but
//! bounded within a specific phase space, producing beautiful, intricate fractal patterns.
//! We iterate a 2D map equation millions of times. Because many points overlap, we accumulate
//! "hits" into a 2D density grid (a histogram). Finally, we apply logarithmic tonemapping
//! to convert the density grid into a vibrant color image, preventing the most visited areas
//! from blowing out to solid white while keeping the faint wisps visible.

use crate::framebuffer::Framebuffer;

/// Defines the chaotic equation type used by the attractor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttractorType {
    /// Clifford Attractor
    /// x' = sin(a * y) + c * cos(a * x)
    /// y' = sin(b * x) + d * cos(b * y)
    Clifford,
    /// Peter de Jong Attractor
    /// x' = sin(a * y) - cos(b * x)
    /// y' = sin(c * x) - cos(d * y)
    PeterDeJong,
}

/// Configuration parameters for the strange attractor simulation.
#[derive(Debug, Clone)]
pub struct StrangeAttractorConfig {
    /// The type of mathematical attractor.
    pub attractor_type: AttractorType,
    /// Parameter A for the equation.
    pub a: f64,
    /// Parameter B for the equation.
    pub b: f64,
    /// Parameter C for the equation.
    pub c: f64,
    /// Parameter D for the equation.
    pub d: f64,
    /// Number of iterations (points to evaluate).
    pub iterations: usize,
    /// Multiplier applied during tone mapping to boost overall brightness.
    pub exposure: f32,
    /// Base color of the attractor. Tone mapping will tint towards white at high density.
    pub color: (u8, u8, u8),
}

impl Default for StrangeAttractorConfig {
    fn default() -> Self {
        Self {
            attractor_type: AttractorType::Clifford,
            a: -1.4,
            b: 1.6,
            c: 1.0,
            d: 0.7,
            iterations: 10_000_000,
            exposure: 1.0,
            color: (100, 200, 255), // Cyan-ish
        }
    }
}

/// Simulates and renders a Strange Attractor into the given Framebuffer.
pub struct StrangeAttractor {
    density_grid: Vec<u32>,
    width: usize,
    height: usize,
}

impl StrangeAttractor {
    /// Creates a new strange attractor generator matching the framebuffer's dimensions.
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            density_grid: vec![0; width * height],
            width,
            height,
        }
    }

    /// Resizes the internal density grid if the dimensions have changed.
    pub fn resize(&mut self, width: usize, height: usize) {
        if self.width != width || self.height != height {
            self.width = width;
            self.height = height;
            self.density_grid = vec![0; width * height];
        }
    }

    /// Renders the strange attractor to the provided framebuffer.
    ///
    /// It first evaluates the chaotic equation for N iterations, tallying hits
    /// in an intermediate density grid. Then it performs a second pass to log-map
    /// the densities into 0xAARRGGBB colors, blending with the base color.
    pub fn render(&mut self, fb: &mut Framebuffer, config: &StrangeAttractorConfig) {
        if self.width == 0 || self.height == 0 {
            return;
        }

        // Clear the density grid for a fresh frame
        self.density_grid.fill(0);

        let mut x = 0.0_f64;
        let mut y = 0.0_f64;

        // Bounding box for mapping points to screen space.
        // Most of these attractors fit well within [-3.0, 3.0].
        let min_bound = -3.0_f64;
        let max_bound = 3.0_f64;
        let scale_x = self.width as f64 / (max_bound - min_bound);
        let scale_y = self.height as f64 / (max_bound - min_bound);

        // --- Phase 1: Accumulate Density (Histogram) ---
        let mut max_density = 0u32;

        for _ in 0..config.iterations {
            let next_x;
            let next_y;

            match config.attractor_type {
                AttractorType::Clifford => {
                    next_x = (config.a * y).sin() + config.c * (config.a * x).cos();
                    next_y = (config.b * x).sin() + config.d * (config.b * y).cos();
                }
                AttractorType::PeterDeJong => {
                    next_x = (config.a * y).sin() - (config.b * x).cos();
                    next_y = (config.c * x).sin() - (config.d * y).cos();
                }
            }

            x = next_x;
            y = next_y;

            // Map coordinate to screen space
            let screen_x = ((x - min_bound) * scale_x) as isize;
            let screen_y = ((y - min_bound) * scale_y) as isize;

            if screen_x >= 0
                && screen_x < self.width as isize
                && screen_y >= 0
                && screen_y < self.height as isize
            {
                let idx = screen_y as usize * self.width + screen_x as usize;
                // Safety: Bounds checked above.
                let count = unsafe { self.density_grid.get_unchecked_mut(idx) };
                *count += 1;
                if *count > max_density {
                    max_density = *count;
                }
            }
        }

        // --- Phase 2: Logarithmic Tone Mapping & Rendering ---
        if max_density == 0 {
            return;
        }

        // We use log10 to map a massive range of hits (e.g. 1 to 500,000) into a manageable curve.
        let log_max = (max_density as f32).log10().max(0.0001);
        let (cr, cg, cb) = config.color;

        let fb_slice = fb.as_mut_slice();

        // ⚡ Bolt Optimization: Using standard iterators via `zip` allows LLVM to elide bounds checks.
        for (pixel, &density) in fb_slice.iter_mut().zip(self.density_grid.iter()) {
            if density == 0 {
                *pixel = 0xFF_00_00_00; // Black background
                continue;
            }

            let log_d = (density as f32).log10();
            // Normalized intensity [0.0, 1.0]
            let mut intensity = (log_d / log_max) * config.exposure;
            intensity = intensity.clamp(0.0, 1.0);

            // Gamma-like curve to make the faint areas pop more and the core feel hotter.
            // A simple power curve achieves this without expensive math.
            let curve = intensity.powf(0.8);

            // As intensity approaches 1.0, we want the color to blow out towards white (hot core).
            // At lower intensities, it stays true to the base color.
            let white_mix = intensity.powi(3); // Sharp curve towards white only at the very top end

            let r = lerp(f32::from(cr) * curve, 255.0, white_mix) as u32;
            let g = lerp(f32::from(cg) * curve, 255.0, white_mix) as u32;
            let b = lerp(f32::from(cb) * curve, 255.0, white_mix) as u32;

            // Clamp to u8 boundaries
            let r = r.clamp(0, 255);
            let g = g.clamp(0, 255);
            let b = b.clamp(0, 255);

            *pixel = 0xFF_00_00_00 | (r << 16) | (g << 8) | b;
        }
    }
}

/// Simple linear interpolation.
#[inline(always)]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strange_attractor_resizing() {
        let mut attractor = StrangeAttractor::new(100, 100);
        assert_eq!(attractor.density_grid.len(), 10000);

        attractor.resize(200, 50);
        assert_eq!(attractor.density_grid.len(), 10000);
        assert_eq!(attractor.width, 200);

        attractor.resize(10, 10);
        assert_eq!(attractor.density_grid.len(), 100);
    }

    #[test]
    fn test_strange_attractor_render() {
        let mut fb = Framebuffer::new(50, 50).unwrap();
        let mut attractor = StrangeAttractor::new(50, 50);
        let mut config = StrangeAttractorConfig::default();
        config.iterations = 1000; // Small number for quick test

        fb.clear(0xFF_00_00_00);

        attractor.render(&mut fb, &config);

        // Verify that *some* pixel was drawn (is not black)
        let has_drawn_pixel = fb.as_slice().iter().any(|&p| p != 0xFF_00_00_00);
        assert!(
            has_drawn_pixel,
            "Attractor should have rendered points to the framebuffer"
        );
    }
}
