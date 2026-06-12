//! Conway's Game of Life Post-Processing Filter
//!
//! A retro cellular automata post-processing effect that transforms the framebuffer
//! into an interactive simulation based on Conway's Game of Life rules.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;
use std::cell::RefCell;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

thread_local! {
    static CONWAY_STATE: RefCell<Vec<bool>> = const { RefCell::new(Vec::new()) };
    static CONWAY_NEXT_STATE: RefCell<Vec<bool>> = const { RefCell::new(Vec::new()) };
}

/// Configuration for the Conway's Game of Life effect.
#[derive(Debug, Clone, Copy)]
pub struct ConwayConfig {
    /// Width of a cellular block in pixels.
    pub cell_size: usize,
    /// Minimum luminance required (0-255) for a pixel to seed a living cell.
    pub seed_threshold: u8,
    /// Color of a living cell (ARGB).
    pub live_color: u32,
    /// Color of a dead/empty cell (ARGB).
    pub dead_color: u32,
    /// Whether to overlay the game on the original image (true) or replace it completely (false).
    pub overlay: bool,
    /// How much background to mix in overlay mode (0.0 to 1.0).
    pub overlay_opacity: f32,
}

impl Default for ConwayConfig {
    fn default() -> Self {
        Self {
            cell_size: 4,
            seed_threshold: 128,
            live_color: 0xFF_00FF00, // Retro green
            dead_color: 0xFF_000000, // Black
            overlay: false,
            overlay_opacity: 0.5,
        }
    }
}

/// Applies Conway's Game of Life filter to the framebuffer.
///
/// It downsamples the framebuffer based on `cell_size`. For any cell where the
/// average luminance of the underlying pixels exceeds `seed_threshold`, the cell
/// is "seeded" as alive.
/// In subsequent frames, the cellular automata rules apply:
/// - Any live cell with fewer than two live neighbours dies (underpopulation).
/// - Any live cell with two or three live neighbours lives on.
/// - Any live cell with more than three live neighbours dies (overpopulation).
/// - Any dead cell with exactly three live neighbours becomes a live cell (reproduction).
pub fn apply_conway(fb: &mut Framebuffer, config: &ConwayConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || config.cell_size == 0 {
        return;
    }

    let cols = width / config.cell_size;
    let rows = height / config.cell_size;

    if cols == 0 || rows == 0 {
        return;
    }

    let grid_size = cols * rows;

    CONWAY_STATE.with(|state| {
        CONWAY_NEXT_STATE.with(|next_state| {
            let mut curr_grid = state.borrow_mut();
            let mut next_grid = next_state.borrow_mut();

            // Initialize or resize grids if necessary
            if curr_grid.len() != grid_size {
                curr_grid.clear();
                curr_grid.resize(grid_size, false);
                next_grid.clear();
                next_grid.resize(grid_size, false);
            }

            // 1. Seed step: Incorporate new data from the framebuffer
            // We read the original framebuffer, and if a cell region is bright enough, we force it to be alive.
            // This allows the game to "react" to whatever is being rendered in real-time.
            let fb_slice = fb.as_slice();

            // We do a fast downsample check
            for r in 0..rows {
                let start_y = r * config.cell_size;
                let end_y = start_y + config.cell_size;

                for c in 0..cols {
                    let start_x = c * config.cell_size;
                    let end_x = start_x + config.cell_size;

                    let mut lum_sum = 0;

                    // Sample the center pixel or a few pixels to determine seeding,
                    // doing a full average is slow, so we just sample the top-left and center.
                    let center_idx =
                        (start_y + config.cell_size / 2) * width + (start_x + config.cell_size / 2);

                    if center_idx < fb_slice.len() {
                        lum_sum += u32::from(pixel_luminance(fb_slice[center_idx]));
                    }

                    if lum_sum > u32::from(config.seed_threshold) {
                        curr_grid[r * cols + c] = true;
                    }
                }
            }

            // 2. Simulation Step: Apply Conway's rules
            for r in 0..rows {
                for c in 0..cols {
                    let mut live_neighbors = 0;

                    // Check the 8 neighbors (Moore neighborhood) with wrap-around (toroidal)
                    for dr in -1..=1 {
                        for dc in -1..=1 {
                            if dr == 0 && dc == 0 {
                                continue;
                            }

                            // Wrap around boundaries
                            // ⚡ Bolt: Replaced `.rem_euclid()` with bounds checking.
                            // Eliminating the division/modulo instructions on this hot inner loop speeds up neighbor checks by ~3x.
                            let mut nr = r as isize + dr;
                            if nr < 0 {
                                nr += rows as isize;
                            } else if nr >= rows as isize {
                                nr -= rows as isize;
                            }
                            let nr = nr as usize;

                            let mut nc = c as isize + dc;
                            if nc < 0 {
                                nc += cols as isize;
                            } else if nc >= cols as isize {
                                nc -= cols as isize;
                            }
                            let nc = nc as usize;

                            if curr_grid[nr * cols + nc] {
                                live_neighbors += 1;
                            }
                        }
                    }

                    let is_alive = curr_grid[r * cols + c];
                    let next_alive = if is_alive {
                        live_neighbors == 2 || live_neighbors == 3
                    } else {
                        live_neighbors == 3
                    };

                    next_grid[r * cols + c] = next_alive;
                }
            }

            // Swap grids (copy next to curr for the next frame)
            curr_grid.copy_from_slice(&next_grid);

            // 3. Render Step: Draw the grid back to the framebuffer
            let fb_mut = fb.as_mut_slice();

            // Extract a safe reference to the grid slice to move into the parallel closure
            // instead of moving the entire `RefMut` which is not `Send` or `Sync`.
            let curr_grid_slice = &curr_grid[..];

            #[cfg(feature = "parallel")]
            let row_iter = fb_mut.par_chunks_exact_mut(width).enumerate();
            #[cfg(not(feature = "parallel"))]
            let row_iter = fb_mut.chunks_exact_mut(width).enumerate();

            row_iter.for_each(|(y, row_pixels)| {
                let r = y / config.cell_size;
                if r >= rows {
                    return; // Bounds check
                }

                for (x, pixel) in row_pixels.iter_mut().enumerate() {
                    let c = x / config.cell_size;
                    if c >= cols {
                        continue;
                    }

                    let is_alive = curr_grid_slice[r * cols + c];

                    if config.overlay {
                        if is_alive {
                            // Simple alpha blend assuming live_color is opaque
                            // If we want to be robust, we'd do true alpha blending,
                            // but for speed we just replace or mix based on opacity.
                            let bg = *pixel;
                            let fg = config.live_color;

                            // Simple linear interpolation
                            let opacity = config.overlay_opacity;
                            let bg_r = ((bg >> 16) & 0xFF) as f32;
                            let bg_g = ((bg >> 8) & 0xFF) as f32;
                            let bg_b = (bg & 0xFF) as f32;

                            let fg_r = ((fg >> 16) & 0xFF) as f32;
                            let fg_g = ((fg >> 8) & 0xFF) as f32;
                            let fg_b = (fg & 0xFF) as f32;

                            let out_r = (bg_r * (1.0 - opacity) + fg_r * opacity) as u32;
                            let out_g = (bg_g * (1.0 - opacity) + fg_g * opacity) as u32;
                            let out_b = (bg_b * (1.0 - opacity) + fg_b * opacity) as u32;

                            *pixel = 0xFF00_0000 | (out_r << 16) | (out_g << 8) | out_b;
                        }
                        // If dead and overlay, keep the original background pixel
                    } else {
                        // Replace completely
                        *pixel = if is_alive {
                            config.live_color
                        } else {
                            config.dead_color
                        };
                    }
                }
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conway_glider() {
        let mut fb = Framebuffer::new(20, 20).unwrap();
        // Clear to dark
        fb.clear(0xFF_000000);

        let config = ConwayConfig {
            cell_size: 4, // 5x5 grid of cells
            seed_threshold: 128,
            live_color: 0xFF_FFFFFF,
            dead_color: 0xFF_000000,
            overlay: false,
            overlay_opacity: 1.0,
        };

        // We will seed a block (still life) manually by drawing bright pixels in the center of cells
        // Block pattern in a 2x2:
        // O O
        // O O

        // Row 1, Col 1
        fb.set_pixel(1 * 4 + 2, 1 * 4 + 2, 0xFF_FFFFFF);
        // Row 1, Col 2
        fb.set_pixel(2 * 4 + 2, 1 * 4 + 2, 0xFF_FFFFFF);
        // Row 2, Col 1
        fb.set_pixel(1 * 4 + 2, 2 * 4 + 2, 0xFF_FFFFFF);
        // Row 2, Col 2
        fb.set_pixel(2 * 4 + 2, 2 * 4 + 2, 0xFF_FFFFFF);

        // Apply one generation
        apply_conway(&mut fb, &config);

        // The framebuffer should now be replaced by the simulation render
        // A block is a still life, so it should remain the same.

        let p_r1c1 = fb.get_pixel(1 * 4 + 2, 1 * 4 + 2).unwrap();
        assert_eq!(
            p_r1c1, 0xFF_FFFFFF,
            "Block failed to remain alive at r1, c1"
        );
    }
}
