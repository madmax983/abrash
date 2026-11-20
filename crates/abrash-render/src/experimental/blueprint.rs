//! Blueprint Filter
//!
//! Transforms the framebuffer into an architectural/engineering blueprint.
//! Replaces the background with a deep blue, maps light pixels to white/cyan lines,
//! and overlays a faint engineering grid.

use super::edge_glow::EdgeGlowConfig;
use crate::framebuffer::Framebuffer;

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
            grid_size: 20,                 // 20px grid
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

    let Ok(mut temp_fb) = Framebuffer::new(width as u32, height as u32) else {
        return;
    };
    temp_fb.as_mut_slice().copy_from_slice(fb.as_slice());

    super::edge_glow::apply_edge_glow(
        &mut temp_fb,
        &EdgeGlowConfig {
            edge_color: config.line_color,
            edge_threshold: 25, // Using u8 threshold
            darken_factor: 1.0,
            intensity: 2.0,
        },
    );

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

    // `(v * line_channel) / 255` for every possible 8-bit `v`, built once per call
    // instead of dividing per pixel. Identical results by construction.
    let r_line = (line_color >> 16) & 0xFF;
    let g_line = (line_color >> 8) & 0xFF;
    let b_line = line_color & 0xFF;
    let mut lut_r = [0u32; 256];
    let mut lut_g = [0u32; 256];
    let mut lut_b = [0u32; 256];
    for v in 0..256u32 {
        lut_r[v as usize] = ((v * r_line) / 255).min(255) << 16;
        lut_g[v as usize] = ((v * g_line) / 255).min(255) << 8;
        lut_b[v as usize] = ((v * b_line) / 255).min(255);
    }

    iter.for_each(|(y, row)| {
        let row_offset = y * width;
        let src_row = &src_pixels[row_offset..row_offset + width];

        // Hoist the per-pixel modulos: the row test is constant for the row and the
        // column test is a wrapping counter.
        let row_is_grid = grid_size > 0 && y % grid_size as usize == 0;
        let grid_step = grid_size as usize;
        let mut col_phase = 0usize;

        for (dest_pixel, &src_pixel) in row.iter_mut().zip(src_row) {
            let is_grid_line = grid_size > 0 && (col_phase == 0 || row_is_grid);
            col_phase += 1;
            if col_phase == grid_step {
                col_phase = 0;
            }

            let r_src = (src_pixel >> 16) & 0xFF;
            let g_src = (src_pixel >> 8) & 0xFF;
            let b_src = src_pixel & 0xFF;

            // In edge_glow with darken_factor=1.0, non-edges are black.
            // So distance to black instead of background.
            let dr = r_src as i32;
            let dg = g_src as i32;
            let db = b_src as i32;
            let dist_sq = dr * dr + dg * dg + db * db;

            // If the pixel is very close to black, it's not an edge.
            if dist_sq < 1000 {
                if is_grid_line {
                    *dest_pixel = grid_color;
                } else {
                    *dest_pixel = bg_color;
                }
            } else {
                // Keep the edge (which should be close to line_color)
                *dest_pixel = 0xFF00_0000
                    | lut_r[r_src as usize]
                    | lut_g[g_src as usize]
                    | lut_b[b_src as usize];
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
        fb.clear(0xFF00_0000);
        // Draw a white rectangle
        for y in 40..60 {
            for x in 40..60 {
                fb.set_pixel(x as i32, y as i32, 0xFFFF_FFFF);
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

    /// Original per-pixel formulation (two modulos, three divisions) used as reference.
    fn reference(src: &Framebuffer, cfg: &BlueprintConfig) -> Vec<u32> {
        let (w, h) = (src.width() as usize, src.height() as usize);
        let mut tmp = Framebuffer::new(w as u32, h as u32).unwrap();
        tmp.as_mut_slice().copy_from_slice(src.as_slice());
        crate::experimental::edge_glow::apply_edge_glow(
            &mut tmp,
            &EdgeGlowConfig {
                edge_color: cfg.line_color,
                edge_threshold: 25,
                darken_factor: 1.0,
                intensity: 2.0,
            },
        );
        let mut out = vec![0u32; w * h];
        for y in 0..h {
            for x in 0..w {
                let p = tmp.as_slice()[y * w + x];
                let g = cfg.grid_size > 0
                    && (x % cfg.grid_size as usize == 0 || y % cfg.grid_size as usize == 0);
                let (r, gr, b) = ((p >> 16) & 0xFF, (p >> 8) & 0xFF, p & 0xFF);
                out[y * w + x] = if (r * r + gr * gr + b * b) < 1000 {
                    if g {
                        cfg.grid_color
                    } else {
                        cfg.background_color
                    }
                } else {
                    let l = cfg.line_color;
                    0xFF00_0000
                        | (((r * ((l >> 16) & 0xFF)) / 255).min(255) << 16)
                        | (((gr * ((l >> 8) & 0xFF)) / 255).min(255) << 8)
                        | ((b * (l & 0xFF)) / 255).min(255)
                };
            }
        }
        out
    }

    #[test]
    fn test_blueprint_matches_reference_formulation() {
        for &(w, h) in &[(37u32, 29u32), (64, 64)] {
            for &grid in &[0u32, 1, 3, 20, 100] {
                for &line in &[0xFFFF_FFFFu32, 0xFF80_C0E0, 0xFF01_0203] {
                    let mut fb = Framebuffer::new(w, h).unwrap();
                    let mut seed = 12345u32;
                    for y in 0..h {
                        for x in 0..w {
                            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                            let v = if (x / 8 + y / 8) % 2 == 0 {
                                0
                            } else {
                                (seed >> 24) & 0xFF
                            };
                            fb.set_pixel(
                                x as i32,
                                y as i32,
                                0xFF00_0000 | (v << 16) | (v << 8) | v,
                            );
                        }
                    }
                    let cfg = BlueprintConfig {
                        line_color: line,
                        grid_size: grid,
                        ..Default::default()
                    };
                    let expected = reference(&fb, &cfg);
                    apply_blueprint(&mut fb, &cfg);
                    assert_eq!(
                        fb.as_slice(),
                        expected.as_slice(),
                        "w={w} grid={grid} line={line:#x}"
                    );
                }
            }
        }
    }
}
