//! Blueprint post-processing effect.
//!
//! Detects edges and overlays them on a grid background to simulate a technical drawing.

use crate::framebuffer::Framebuffer;
use crate::utils::pixel_luminance;

use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

thread_local! {
    static LUMA_BUFFER: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the Blueprint post-processing filter.
#[derive(Debug, Clone, Copy)]
pub struct BlueprintConfig {
    /// Size of the grid squares in pixels.
    pub grid_size: usize,
    /// Color of the grid lines (ARGB).
    pub grid_color: u32,
    /// Color of the background (ARGB).
    pub bg_color: u32,
    /// Color of the detected edges (ARGB).
    pub line_color: u32,
    /// Minimum magnitude for an edge to be detected (0-255).
    pub edge_threshold: u8,
}

impl Default for BlueprintConfig {
    fn default() -> Self {
        Self {
            grid_size: 20,
            grid_color: 0xFF_336699, // Lighter blue
            bg_color: 0xFF_003366,   // Dark blue
            line_color: 0xFF_FFFFFF, // White lines
            edge_threshold: 30,      // Same default as edge glow
        }
    }
}

/// Applies a Blueprint filter to the framebuffer in-place.
pub fn apply_blueprint(fb: &mut Framebuffer, config: &BlueprintConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let pixels = fb.as_mut_slice();
    let needed_size = width * height;

    if needed_size == 0 || config.grid_size == 0 {
        return;
    }

    LUMA_BUFFER.with(|buf| {
        let mut luma_buffer = buf.borrow_mut();
        if luma_buffer.len() < needed_size {
            luma_buffer.resize(needed_size, 0);
        }
        let luma_slice = &mut luma_buffer[..needed_size];

        // 1. Convert to Luminance
        #[cfg(feature = "parallel")]
        {
            luma_slice
                .par_iter_mut()
                .zip(pixels.par_iter())
                .for_each(|(l, p)| {
                    *l = pixel_luminance(*p);
                });
        }
        #[cfg(not(feature = "parallel"))]
        {
            for (i, p) in pixels.iter().enumerate() {
                luma_slice[i] = pixel_luminance(*p);
            }
        }

        let threshold = i32::from(config.edge_threshold);
        let grid_color = config.grid_color & 0x00FFFFFF;
        let bg_color = config.bg_color & 0x00FFFFFF;
        let line_color = config.line_color & 0x00FFFFFF;
        let grid_size = config.grid_size;

        // 2. Apply Sobel and Grid
        // We do not skip the 1-pixel border for the grid, only for Sobel.

        #[cfg(feature = "parallel")]
        let row_iter = pixels.par_chunks_exact_mut(width).enumerate();
        #[cfg(not(feature = "parallel"))]
        let row_iter = pixels.chunks_exact_mut(width).enumerate();

        row_iter.for_each(|(y, row_pixels)| {
            let row_offset = y * width;
            let is_grid_y = y % grid_size == 0;

            for x in 0..width {
                let original_pixel = row_pixels[x];
                let alpha = original_pixel & 0xFF00_0000;
                let mut is_edge = false;

                if x > 0 && x < width - 1 && y > 0 && y < height - 1 {
                    let prev_row_offset = row_offset - width;
                    let next_row_offset = row_offset + width;

                    let tl = i32::from(luma_slice[prev_row_offset + x - 1]);
                    let t = i32::from(luma_slice[prev_row_offset + x]);
                    let tr = i32::from(luma_slice[prev_row_offset + x + 1]);
                    let l = i32::from(luma_slice[row_offset + x - 1]);
                    let r = i32::from(luma_slice[row_offset + x + 1]);
                    let bl = i32::from(luma_slice[next_row_offset + x - 1]);
                    let b = i32::from(luma_slice[next_row_offset + x]);
                    let br = i32::from(luma_slice[next_row_offset + x + 1]);

                    let gx = (tr + 2 * r + br) - (tl + 2 * l + bl);
                    let gy = (bl + 2 * b + br) - (tl + 2 * t + tr);
                    let mag = gx.abs() + gy.abs();

                    if mag > threshold {
                        is_edge = true;
                    }
                }

                if is_edge {
                    // Edge detected: Set to line color
                    row_pixels[x] = alpha | line_color;
                } else {
                    // Not an edge: Apply grid
                    let is_grid_x = x % grid_size == 0;
                    if is_grid_x || is_grid_y {
                        row_pixels[x] = alpha | grid_color;
                    } else {
                        row_pixels[x] = alpha | bg_color;
                    }
                }
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_blueprint_solid_color() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFF_FFFFFF); // Solid white

        let config = BlueprintConfig {
            grid_size: 5,
            grid_color: 0xFF_111111,
            bg_color: 0xFF_222222,
            line_color: 0xFF_333333,
            edge_threshold: 10,
        };

        apply_blueprint(&mut fb, &config);

        // Since the framebuffer was solid color, there are no edges.
        // It should just be the grid and the background.
        for y in 0..10 {
            for x in 0..10 {
                let p = fb.get_pixel(x, y).unwrap() & 0x00FFFFFF;
                if x % 5 == 0 || y % 5 == 0 {
                    assert_eq!(p, 0x00_111111, "Grid pixel at ({x}, {y}) is incorrect");
                } else {
                    assert_eq!(
                        p, 0x00_222222,
                        "Background pixel at ({x}, {y}) is incorrect"
                    );
                }
            }
        }
    }

    #[test]
    fn test_blueprint_edge() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFF_000000); // Black background

        // Draw a 4x4 white square in the middle
        for y in 3..7 {
            for x in 3..7 {
                fb.set_pixel(x, y, 0xFF_FFFFFF);
            }
        }

        let config = BlueprintConfig {
            grid_size: 5,
            grid_color: 0xFF_111111,
            bg_color: 0xFF_222222,
            line_color: 0xFF_333333,
            edge_threshold: 10,
        };

        apply_blueprint(&mut fb, &config);

        // The edges of the white square should be detected and set to `line_color`.
        // A pixel like (3, 3) was black but right next to white, so it might be an edge, or (4,4) etc.
        // Let's just check that at least one pixel is the line color.
        let mut found_edge = false;
        for p in fb.as_slice() {
            if (*p & 0x00FFFFFF) == 0x00_333333 {
                found_edge = true;
                break;
            }
        }
        assert!(
            found_edge,
            "Edge should have been detected and colored with line_color"
        );
    }
}
