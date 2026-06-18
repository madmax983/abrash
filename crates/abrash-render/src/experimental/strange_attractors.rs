use abrash_core::framebuffer::Framebuffer;

/// Defines the mathematical formula used for generating the attractor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttractorType {
    /// Clifford Attractor
    /// x' = sin(a * y) + c * cos(a * x)
    /// y' = sin(b * x) + d * cos(b * y)
    Clifford,
    /// Peter de Jong Attractor
    /// x' = sin(a * y) - cos(b * x)
    /// y' = sin(c * x) - cos(d * y)
    DeJong,
    /// Symmetric Icon Attractor (simplified)
    SymmetricIcon,
}

/// Configuration for the Strange Attractors generator.
#[derive(Debug, Clone)]
pub struct StrangeAttractorsConfig {
    /// The type of attractor to generate.
    pub attractor_type: AttractorType,
    /// Number of iterations to evaluate the formula.
    pub iterations: usize,
    /// Parameter A
    pub a: f32,
    /// Parameter B
    pub b: f32,
    /// Parameter C
    pub c: f32,
    /// Parameter D
    pub d: f32,
    /// Color to map the density to (ARGB).
    pub color: u32,
    /// The scale factor applied to the bounds.
    pub scale: f32,
}

impl Default for StrangeAttractorsConfig {
    fn default() -> Self {
        Self {
            attractor_type: AttractorType::Clifford,
            iterations: 1_000_000,
            a: -1.4,
            b: 1.6,
            c: 1.0,
            d: 0.7,
            color: 0xFF00_FFFF, // Cyan
            scale: 1.0,
        }
    }
}

/// Applies a Strange Attractors procedural generator to the framebuffer.
///
/// This operates by iterating mathematical formulas to generate a 2D density map,
/// and then logarithmically tonemaps the densities into colors on the framebuffer.
pub fn apply_strange_attractors(fb: &mut Framebuffer, config: &StrangeAttractorsConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    // Track hit density
    let mut density_map = vec![0u32; width * height];
    let mut max_density = 0u32;

    let mut x = 0.1f32;
    let mut y = 0.1f32;

    let half_w = width as f32 / 2.0;
    let half_h = height as f32 / 2.0;

    let scale_factor = (half_w.min(half_h)) * 0.4 * config.scale;

    for _ in 0..config.iterations {
        let (nx, ny) = match config.attractor_type {
            AttractorType::Clifford => {
                let nx = (config.a * y).sin() + config.c * (config.a * x).cos();
                let ny = (config.b * x).sin() + config.d * (config.b * y).cos();
                (nx, ny)
            }
            AttractorType::DeJong => {
                let nx = (config.a * y).sin() - (config.b * x).cos();
                let ny = (config.c * x).sin() - (config.d * y).cos();
                (nx, ny)
            }
            AttractorType::SymmetricIcon => {
                // Modified symmetric icon
                let nx = y + (config.a * x).sin() + (config.c * y).cos();
                let ny = -x + (config.b * y).sin() + (config.d * x).cos();
                (nx, ny)
            }
        };

        x = nx;
        y = ny;

        let px = (half_w + x * scale_factor) as i32;
        let py = (half_h + y * scale_factor) as i32;

        if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
            let idx = (py as usize) * width + (px as usize);
            density_map[idx] += 1;
            if density_map[idx] > max_density {
                max_density = density_map[idx];
            }
        }
    }

    // Tonemap to framebuffer (Logarithmic mapping to reveal details)
    if max_density > 1 {
        let log_max = (max_density as f32).ln();

        let target_r = ((config.color >> 16) & 0xFF) as f32;
        let target_g = ((config.color >> 8) & 0xFF) as f32;
        let target_b = (config.color & 0xFF) as f32;

        let pixels = fb.as_mut_slice();
        for (i, &density) in density_map.iter().enumerate() {
            if density > 0 {
                // Logarithmic intensity
                let intensity = (density as f32).ln() / log_max;

                // Read current background pixel
                let bg = pixels[i];
                let bg_r = ((bg >> 16) & 0xFF) as f32;
                let bg_g = ((bg >> 8) & 0xFF) as f32;
                let bg_b = (bg & 0xFF) as f32;

                // Additive blend with background
                let out_r = (bg_r + target_r * intensity).min(255.0) as u32;
                let out_g = (bg_g + target_g * intensity).min(255.0) as u32;
                let out_b = (bg_b + target_b * intensity).min(255.0) as u32;

                pixels[i] = 0xFF00_0000 | (out_r << 16) | (out_g << 8) | out_b;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_strange_attractors_bounds() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF00_0000);

        let config = StrangeAttractorsConfig {
            iterations: 10_000,
            scale: 10.0, // Push bounds out
            ..Default::default()
        };

        // Should not panic due to out of bounds
        apply_strange_attractors(&mut fb, &config);

        // Some pixels should be drawn
        let mut drawn = false;
        for &pixel in fb.as_slice() {
            if pixel != 0xFF00_0000 {
                drawn = true;
                break;
            }
        }
        assert!(drawn, "Attractor should draw pixels on the framebuffer");
    }

    #[test]
    fn test_apply_strange_attractors_types() {
        let types = vec![
            AttractorType::Clifford,
            AttractorType::DeJong,
            AttractorType::SymmetricIcon,
        ];

        for t in types {
            let mut fb = Framebuffer::new(50, 50).unwrap();
            fb.clear(0xFF00_0000);

            let config = StrangeAttractorsConfig {
                attractor_type: t,
                iterations: 5000,
                ..Default::default()
            };

            apply_strange_attractors(&mut fb, &config);

            let mut drawn = false;
            for &pixel in fb.as_slice() {
                if pixel != 0xFF00_0000 {
                    drawn = true;
                    break;
                }
            }
            assert!(drawn, "Attractor type {:?} failed to draw", t);
        }
    }
}
