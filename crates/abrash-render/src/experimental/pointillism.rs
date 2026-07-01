use abrash_core::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the pointillism post-processing effect.
#[derive(Debug, Clone)]
pub struct PointillismConfig {
    /// Size of the grid cells for sampling
    pub cell_size: usize,
    /// The maximum radius of the splatted points
    pub max_radius: f32,
    /// How much position jitter to add (0.0 to 1.0)
    pub position_jitter: f32,
}

impl Default for PointillismConfig {
    fn default() -> Self {
        Self {
            cell_size: 8,
            max_radius: 6.0,
            position_jitter: 1.0,
        }
    }
}

// Procedural hash function
const fn hash2(x: i32, y: i32) -> u32 {
    let mut h = (x as u32)
        .wrapping_mul(0x9E37_79B1)
        .wrapping_add((y as u32).wrapping_mul(0x85EB_CA6B));
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;
    h
}

// Normalized procedural hash
fn hash2_f32(x: i32, y: i32) -> f32 {
    (hash2(x, y) & 0x00FF_FFFF) as f32 / 0x00FF_FFFF as f32
}

pub fn apply_pointillism(fb: &mut Framebuffer, src_fb: &Framebuffer, config: &PointillismConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || config.cell_size == 0 || fb.width() != src_fb.width() || fb.height() != src_fb.height() {
        return;
    }

    let pixels = fb.as_mut_slice();
    let src_pixels = src_fb.as_slice();

    let cell_size = config.cell_size as i32;
    let max_radius_sq = config.max_radius * config.max_radius;
    let search_radius = (config.max_radius.ceil() as i32).max(cell_size) / cell_size + 1;

    #[cfg(feature = "parallel")]
    let row_iter = pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let row_iter = pixels.chunks_exact_mut(width).enumerate();

    row_iter.for_each(|(y, row_pixels)| {
        let py = y as i32;
        let cell_y = py / cell_size;

        for (x, dest_pixel) in row_pixels.iter_mut().enumerate() {
            let px = x as i32;
            let cell_x = px / cell_size;

            let mut best_dist_sq = f32::INFINITY;
            let mut best_color = *dest_pixel;

            // Search neighbor cells
            for dy in -search_radius..=search_radius {
                for dx in -search_radius..=search_radius {
                    let cx = cell_x + dx;
                    let cy = cell_y + dy;

                    // Generate a random position and radius for the point in this cell
                    let h1 = hash2_f32(cx, cy);
                    let h2 = hash2_f32(cx + 949, cy + 123);
                    let h3 = hash2_f32(cx + 423, cy + 994);

                    let base_px = (cx * cell_size) as f32 + (cell_size as f32 * 0.5);
                    let base_py = (cy * cell_size) as f32 + (cell_size as f32 * 0.5);

                    let jitter_x =
                        (h1 * 2.0 - 1.0) * (cell_size as f32 * 0.5) * config.position_jitter;
                    let jitter_y =
                        (h2 * 2.0 - 1.0) * (cell_size as f32 * 0.5) * config.position_jitter;

                    let point_x = base_px + jitter_x;
                    let point_y = base_py + jitter_y;

                    let radius = config.max_radius * (0.3 + 0.7 * h3);
                    let radius_sq = radius * radius;

                    let dist_dx = px as f32 - point_x;
                    let dist_dy = py as f32 - point_y;
                    let dist_sq = dist_dx * dist_dx + dist_dy * dist_dy;

                    // If pixel is within this cell's point, use it (we keep the closest center)
                    if dist_sq <= radius_sq && dist_sq < best_dist_sq {
                        best_dist_sq = dist_sq;

                        // Sample color from src_fb at the point center
                        let sample_x = (point_x as i32).clamp(0, width as i32 - 1);
                        let sample_y = (point_y as i32).clamp(0, height as i32 - 1);
                        best_color = src_pixels[sample_y as usize * width + sample_x as usize];
                    }
                }
            }

            *dest_pixel = best_color;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_pointillism_modifies_fb() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        let mut src_fb = Framebuffer::new(100, 100).unwrap();

        fb.clear(0xFF_000000);
        src_fb.clear(0xFF_FFFFFF);

        let config = PointillismConfig {
            cell_size: 10,
            max_radius: 5.0,
            position_jitter: 0.0,
        };

        apply_pointillism(&mut fb, &src_fb, &config);

        // It should have painted white dots on the black background.
        let mut has_white = false;
        for &pixel in fb.as_slice() {
            if pixel != 0xFF_000000 {
                has_white = true;
                break;
            }
        }
        assert!(
            has_white,
            "Pointillism filter did not modify the framebuffer"
        );
    }
}
