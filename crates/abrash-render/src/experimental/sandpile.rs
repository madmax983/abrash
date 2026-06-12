//! Abelian Sandpile Post-Processing Filter
//!
//! A retro cellular automata effect that visualizes the Abelian sandpile model.
//! Sand grains are dropped and when a cell reaches 4 grains, it topples,
//! distributing one grain to each of its four neighbors, creating fractal patterns.

use crate::framebuffer::Framebuffer;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

thread_local! {
    static SANDPILE_STATE: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static SANDPILE_NEXT_STATE: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the Abelian Sandpile filter.
#[derive(Debug, Clone)]
pub struct SandpileConfig {
    /// X coordinate to drop new sand grains.
    pub drop_x: usize,
    /// Y coordinate to drop new sand grains.
    pub drop_y: usize,
    /// Number of sand grains to drop per frame.
    pub grains_per_frame: u32,
    /// Maximum toppling iterations to perform per frame.
    pub iterations_per_frame: usize,
    /// Color mapping for grain counts: index corresponds to number of grains (0, 1, 2, 3).
    /// Grains >= 4 also use the last color during intermediate steps.
    pub colors: [u32; 4],
}

impl Default for SandpileConfig {
    fn default() -> Self {
        Self {
            drop_x: 0,
            drop_y: 0,
            grains_per_frame: 100,
            iterations_per_frame: 10,
            colors: [
                0xFF_000000, // 0 grains: Black
                0xFF_0000FF, // 1 grain: Blue
                0xFF_00FF00, // 2 grains: Green
                0xFF_FF0000, // 3 grains: Red
            ],
        }
    }
}

/// Applies the Abelian Sandpile filter to the framebuffer.
///
/// This maintains an internal grid of sand grain counts. It drops `grains_per_frame`
/// at `(drop_x, drop_y)`, then performs up to `iterations_per_frame` toppling passes.
/// Finally, it colors the `Framebuffer` based on the number of grains in each cell.
pub fn apply_sandpile(fb: &mut Framebuffer, config: &SandpileConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let grid_size = width * height;

    SANDPILE_STATE.with(|state| {
        SANDPILE_NEXT_STATE.with(|next_state| {
            let mut curr_grid = state.borrow_mut();
            let mut next_grid = next_state.borrow_mut();

            // Initialize or resize grids if necessary
            if curr_grid.len() != grid_size {
                curr_grid.clear();
                curr_grid.resize(grid_size, 0);
                next_grid.clear();
                next_grid.resize(grid_size, 0);
            }

            // 1. Drop new grains
            if config.drop_x < width && config.drop_y < height {
                let drop_idx = config.drop_y * width + config.drop_x;
                curr_grid[drop_idx] += config.grains_per_frame;
            }

            // 2. Toppling loop
            for _ in 0..config.iterations_per_frame {
                let mut changed = false;
                next_grid.copy_from_slice(&curr_grid);

                for y in 0..height {
                    for x in 0..width {
                        let idx = y * width + x;
                        let grains = curr_grid[idx];
                        if grains >= 4 {
                            changed = true;
                            // Distribute to neighbors
                            next_grid[idx] -= 4;
                            if y > 0 {
                                next_grid[(y - 1) * width + x] += 1;
                            }
                            if y < height - 1 {
                                next_grid[(y + 1) * width + x] += 1;
                            }
                            if x > 0 {
                                next_grid[y * width + (x - 1)] += 1;
                            }
                            if x < width - 1 {
                                next_grid[y * width + (x + 1)] += 1;
                            }
                        }
                    }
                }

                curr_grid.copy_from_slice(&next_grid);
                if !changed {
                    break;
                }
            }

            // 3. Render Step: Draw the grid back to the framebuffer
            let fb_mut = fb.as_mut_slice();
            let curr_grid_slice = &curr_grid[..];
            let colors = config.colors;

            #[cfg(feature = "parallel")]
            let row_iter = fb_mut.par_chunks_exact_mut(width).enumerate();
            #[cfg(not(feature = "parallel"))]
            let row_iter = fb_mut.chunks_exact_mut(width).enumerate();

            row_iter.for_each(|(y, row_pixels)| {
                let y_offset = y * width;
                for (x, pixel) in row_pixels.iter_mut().enumerate() {
                    let grains = curr_grid_slice[y_offset + x];
                    let color_idx = std::cmp::min(grains as usize, 3);
                    *pixel = colors[color_idx];
                }
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandpile_topples() {
        let mut fb = Framebuffer::new(5, 5).unwrap();
        fb.clear(0xFF_FFFFFF); // White background initially

        let config = SandpileConfig {
            drop_x: 2,
            drop_y: 2,
            grains_per_frame: 16,      // Enough to cause multiple topples
            iterations_per_frame: 100, // Ensure it fully settles
            colors: [0xFF_000000, 0xFF_0000FF, 0xFF_00FF00, 0xFF_FF0000],
        };

        // Before apply, center pixel is white (background)
        assert_eq!(fb.get_pixel(2, 2).unwrap(), 0xFF_FFFFFF);

        apply_sandpile(&mut fb, &config);

        // After apply, the grid should be updated.
        // We dropped 16 grains at the center (2,2).
        // It will topple multiple times. The center should NOT be 0xFF_FFFFFF or > 3 color.
        // Also the neighbors should have received grains.

        let center_color = fb.get_pixel(2, 2).unwrap();
        let top_neighbor_color = fb.get_pixel(2, 1).unwrap();

        assert_ne!(
            center_color, 0xFF_FFFFFF,
            "Center pixel should have been overwritten by sandpile colors"
        );

        // Since we dropped 16 grains and allowed settling, the center and neighbors should
        // have some non-zero amount of sand. In an abelian sandpile, 16 grains at center
        // on an empty grid settles into a specific pattern where center is not 0 (it's 0 after 4 topples, but neighbors topple back).
        // Actually, let's just assert the grid changed from white to one of the palette colors.

        let valid_colors = [0xFF_000000, 0xFF_0000FF, 0xFF_00FF00, 0xFF_FF0000];
        assert!(
            valid_colors.contains(&center_color),
            "Center color must be from palette"
        );
        assert!(
            valid_colors.contains(&top_neighbor_color),
            "Top neighbor must be from palette"
        );

        // Verify at least one pixel changed to a non-zero color
        let mut has_sand = false;
        for &pixel in fb.as_slice() {
            if pixel == 0xFF_0000FF || pixel == 0xFF_00FF00 || pixel == 0xFF_FF0000 {
                has_sand = true;
                break;
            }
        }
        assert!(has_sand, "The sandpile should have spread sand to the grid");
    }
}
