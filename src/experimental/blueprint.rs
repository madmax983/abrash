//! Blueprint Filter
//!
//! Transforms the framebuffer into an architectural/engineering blueprint.
//! Replaces the background with a deep blue, maps light pixels to white/cyan lines,
//! and overlays a faint engineering grid.

use crate::framebuffer::Framebuffer;
use super::edge_glow::EdgeGlowConfig;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Configuration for the Blueprint effect.
#[derive(Debug, Clone, Copy)]
pub struct BlueprintConfig {
    /// The background color (default is a classic blueprint blue).
    pub background_color: u32,
    /// The color of the lines (default is white).
    pub line_color: u32,
    /// The color of the grid lines (default is a light blue).
    pub grid_color: u32,
    /// The size of the grid cells in pixels. If 0, no grid is drawn.
    pub grid_size: u32,
}

impl Default for BlueprintConfig {
    fn default() -> Self {
        Self {
            background_color: 0xFF00_3399, // Classic deep blue
            line_color: 0xFFFF_FFFF,       // White lines
            grid_color: 0xFF33_66CC,       // Lighter blue for grid
            grid_size: 20,                // 20px grid
        }
    }
}

/// Applies a blueprint effect to the framebuffer.
///
/// Converts the image into a blueprint style by applying an edge detection filter
/// and replacing colors with a blueprint palette.
///
/// # Panics
///
/// This function can panic if the internal framebuffer allocation fails.
///
/// # Arguments
/// * `fb` - The framebuffer to modify in-place.
pub fn apply_blueprint(fb: &mut Framebuffer, config: &BlueprintConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let Ok(mut temp_fb) = Framebuffer::new(width as u32, height as u32) else { return; };
    temp_fb.as_mut_slice().copy_from_slice(fb.as_slice());

    super::edge_glow::apply_edge_glow(&mut temp_fb, &EdgeGlowConfig {
        edge_color: config.line_color,
        edge_threshold: 25, // Using u8 threshold
        darken_factor: 1.0,
        intensity: 2.0,
    });

    let src_pixels = temp_fb.as_slice();
    let dest_pixels = fb.as_mut_slice();

    // Map the edge-detected result to the blueprint colors and add the grid
    #[cfg(feature = "parallel")]
    let iter = dest_pixels.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let iter = dest_pixels.chunks_exact_mut(width).enumerate();

    let bg_color = config.background_color;
    let line_color = config.line_color;
    let grid_color = config.grid_color;
    let grid_size = config.grid_size;

    iter.for_each(|(y, row)| {
        let row_offset = y * width;
        let src_row = &src_pixels[row_offset..row_offset + width];

        for (x, (dest_pixel, &src_pixel)) in row.iter_mut().zip(src_row).enumerate() {
            let is_grid_line = grid_size > 0 && (x % grid_size as usize == 0 || y % grid_size as usize == 0);

            let r_src = (src_pixel >> 16) & 0xFF;
            let g_src = (src_pixel >> 8) & 0xFF;
            let b_src = src_pixel & 0xFF;

            // In edge_glow with darken_factor=1.0, non-edges are black.
            // So distance to black instead of background.
            let r_bg = 0;
            let g_bg = 0;
            let b_bg = 0;

            let dist_sq = (r_src as i32 - r_bg as i32).pow(2) +
                          (g_src as i32 - g_bg as i32).pow(2) +
                          (b_src as i32 - b_bg as i32).pow(2);

            // If the pixel is very close to black, it's not an edge.
            if dist_sq < 1000 {
                if is_grid_line {
                    *dest_pixel = grid_color;
                } else {
                    *dest_pixel = bg_color;
                }
            } else {
                // Keep the edge (which should be close to line_color)
                // Optionally blend with bg_color to retain anti-aliased edge appearance
                let r_line = (line_color >> 16) & 0xFF;
                let g_line = (line_color >> 8) & 0xFF;
                let b_line = line_color & 0xFF;

                let r_out = ((r_src * r_line) / 255).min(255);
                let g_out = ((g_src * g_line) / 255).min(255);
                let b_out = ((b_src * b_line) / 255).min(255);

                *dest_pixel = 0xFF00_0000 | (r_out << 16) | (g_out << 8) | b_out;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_blueprint() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF000000);
        // Draw a white rectangle
        for y in 40..60 {
            for x in 40..60 {
                fb.set_pixel(x as i32, y as i32, 0xFFFFFFFF);
            }
        }

        let config = BlueprintConfig::default();
        apply_blueprint(&mut fb, &config);

        // Grid lines at 0, 20, 40, etc.
        assert_eq!(fb.get_pixel(0, 0).unwrap(), config.grid_color);
        // Non-grid, non-edge background
        assert_eq!(fb.get_pixel(1, 1).unwrap(), config.background_color);

        // Edge_glow output for edges should be something bright, but depends on the exact algorithm
        // We just ensure it's not background color or grid color on the literal edge.
        let edge_pixel = fb.get_pixel(40, 40).unwrap();
        assert_ne!(edge_pixel, config.background_color);
        assert_ne!(edge_pixel, config.grid_color);
    }
}
