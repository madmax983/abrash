//! Strange Attractor Generators.
//!
//! Renders mathematical attractors like Clifford and Peter de Jong attractors.

use crate::framebuffer::Framebuffer;

/// Parameters for rendering a Strange Attractor
#[derive(Debug, Clone, Copy)]
pub struct AttractorConfig {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub iterations: usize,
    pub attractor_type: AttractorType,
    pub intensity: f64,
}

impl Default for AttractorConfig {
    fn default() -> Self {
        Self {
            a: -1.24458,
            b: -1.25191,
            c: -1.815_908,
            d: -1.90866,
            iterations: 1_000_000,
            attractor_type: AttractorType::Clifford,
            intensity: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttractorType {
    Clifford,
    PeterDeJong,
}

/// Renders a Strange Attractor into the given framebuffer.
pub fn render_attractor(fb: &mut Framebuffer, config: &AttractorConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let mut density_map = vec![0u32; width * height];
    let mut max_density = 0u32;

    let mut x = 0.0;
    let mut y = 0.0;

    let a = config.a;
    let b = config.b;
    let c = config.c;
    let d = config.d;

    // We do a brief warm-up to allow the attractor to settle
    for _ in 0..100 {
        let (nx, ny) = match config.attractor_type {
            AttractorType::Clifford => (
                (a * y).sin() + c * (a * x).cos(),
                (b * x).sin() + d * (b * y).cos(),
            ),
            AttractorType::PeterDeJong => {
                ((a * y).sin() - (b * x).cos(), (c * x).sin() - (d * y).cos())
            }
        };
        x = nx;
        y = ny;
    }

    // Main iteration loop
    for _ in 0..config.iterations {
        let (nx, ny) = match config.attractor_type {
            AttractorType::Clifford => (
                (a * y).sin() + c * (a * x).cos(),
                (b * x).sin() + d * (b * y).cos(),
            ),
            AttractorType::PeterDeJong => {
                ((a * y).sin() - (b * x).cos(), (c * x).sin() - (d * y).cos())
            }
        };
        x = nx;
        y = ny;

        // Map coordinates to screen space
        // Attractors typically fit within [-3, 3] or similar
        let mapped_x = ((x + 3.0) / 6.0 * width as f64) as isize;
        let mapped_y = ((y + 3.0) / 6.0 * height as f64) as isize;

        if mapped_x >= 0 && mapped_x < width as isize && mapped_y >= 0 && mapped_y < height as isize
        {
            let idx = mapped_y as usize * width + mapped_x as usize;
            density_map[idx] += 1;
            if density_map[idx] > max_density {
                max_density = density_map[idx];
            }
        }
    }

    // Coloring pass
    let max_density_f64 = f64::from(max_density);
    let pixels = fb.as_mut_slice();

    for (i, &density) in density_map.iter().enumerate() {
        if density > 0 {
            // Logarithmic mapping for smooth gradients and revealing structure
            // Use intensity parameter to allow user to tweak brightness
            let v = f64::from(density).ln() / max_density_f64.ln() * config.intensity;
            let v = v.clamp(0.0, 1.0);

            // Create a nice palette map (e.g. fire-like or neon)
            let r = (v.powf(0.5) * 255.0) as u32;
            let g = (v.powf(1.5) * 255.0) as u32;
            let b = (v.powf(2.5) * 255.0) as u32;

            // Simple additive blending to the existing framebuffer (assuming black bg)
            let existing = pixels[i];
            let ex_r = (existing >> 16) & 0xFF;
            let ex_g = (existing >> 8) & 0xFF;
            let ex_b = existing & 0xFF;

            let new_r = (ex_r + r).min(255);
            let new_g = (ex_g + g).min(255);
            let new_b = (ex_b + b).min(255);

            pixels[i] = 0xFF_00_00_00 | (new_r << 16) | (new_g << 8) | new_b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_attractor_0x0() {
        let mut fb = Framebuffer::new(0, 0).unwrap();
        let config = AttractorConfig::default();
        render_attractor(&mut fb, &config);
        assert_eq!(fb.width(), 0);
        assert_eq!(fb.height(), 0);
    }

    #[test]
    fn test_render_attractor_standard() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF_00_00_00);
        let config = AttractorConfig {
            iterations: 10000,
            ..Default::default()
        };
        render_attractor(&mut fb, &config);

        let mut has_non_black = false;
        for &p in fb.as_slice() {
            if p != 0xFF_00_00_00 {
                has_non_black = true;
                break;
            }
        }
        assert!(
            has_non_black,
            "Framebuffer should not be entirely black after rendering"
        );
    }
}
