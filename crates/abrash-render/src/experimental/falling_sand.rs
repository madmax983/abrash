//! Falling Sand cellular automaton
//!
//! A 2D cellular automaton that simulates falling sand, colliding with the
//! 3D scene rendered to the Framebuffer.

#![cfg(feature = "nova")]

use crate::framebuffer::Framebuffer;
use rand::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    Empty,
    Sand(u32), // Contains the color of the sand
}

pub struct FallingSand {
    pub width: usize,
    pub height: usize,
    pub grid: Vec<Cell>,
    pub next_grid: Vec<Cell>,
}

impl FallingSand {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            grid: vec![Cell::Empty; width * height],
            next_grid: vec![Cell::Empty; width * height],
        }
    }

    /// Emits a sand particle at the top of the screen at a random x coordinate
    pub fn emit_sand(&mut self, amount: usize, color: u32) {
        if self.width == 0 || self.height == 0 {
            return;
        }
        let mut rng = rand::thread_rng();
        let max_y = std::cmp::min(5, self.height);
        for _ in 0..amount {
            let x = rng.gen_range(0..self.width);
            let y = rng.gen_range(0..max_y);

            let idx = y * self.width + x;
            if self.grid[idx] == Cell::Empty {
                self.grid[idx] = Cell::Sand(color);
            }
        }
    }

    /// Simulates the cellular automaton and draws it to the Framebuffer.
    /// Sand stops if it hits a pixel in the Framebuffer that is not the clear_color.
    pub fn update_and_draw(&mut self, fb: &mut Framebuffer, clear_color: u32) {
        // Clear next grid
        self.next_grid.fill(Cell::Empty);

        let fb_pixels = fb.as_mut_slice();
        let mut rng = rand::thread_rng();

        for y in (0..self.height).rev() {
            for x in 0..self.width {
                let idx = y * self.width + x;

                if let Cell::Sand(color) = self.grid[idx] {
                    let mut moved = false;

                    if y + 1 < self.height {
                        let down_idx = (y + 1) * self.width + x;

                        // Check if the pixel below is empty in BOTH the cellular grid and the framebuffer
                        if self.next_grid[down_idx] == Cell::Empty
                            && fb_pixels[down_idx] == clear_color
                        {
                            self.next_grid[down_idx] = Cell::Sand(color);
                            moved = true;
                        } else {
                            // Try diagonal left/right randomly
                            let left = rng.gen_bool(0.5);
                            let dir = if left { -1 } else { 1 };

                            let nx = x as i32 + dir;
                            if nx >= 0 && nx < self.width as i32 {
                                let diag_idx = (y + 1) * self.width + nx as usize;
                                if self.next_grid[diag_idx] == Cell::Empty
                                    && fb_pixels[diag_idx] == clear_color
                                {
                                    self.next_grid[diag_idx] = Cell::Sand(color);
                                    moved = true;
                                }
                            }

                            // If first diagonal failed, try the other
                            if !moved {
                                let other_nx = x as i32 - dir;
                                if other_nx >= 0 && other_nx < self.width as i32 {
                                    let diag_idx = (y + 1) * self.width + other_nx as usize;
                                    if self.next_grid[diag_idx] == Cell::Empty
                                        && fb_pixels[diag_idx] == clear_color
                                    {
                                        self.next_grid[diag_idx] = Cell::Sand(color);
                                        moved = true;
                                    }
                                }
                            }
                        }
                    }

                    // If it couldn't move down or diagonally, it stays where it is
                    if !moved {
                        self.next_grid[idx] = Cell::Sand(color);
                    }
                }
            }
        }

        // Swap grids
        std::mem::swap(&mut self.grid, &mut self.next_grid);

        // Draw to framebuffer
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = y * self.width + x;
                if let Cell::Sand(color) = self.grid[idx] {
                    fb_pixels[idx] = color;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sand_falls() {
        let width = 10;
        let height = 10;
        let mut sand = FallingSand::new(width, height);
        let mut fb = Framebuffer::new(width as u32, height as u32).unwrap();
        let clear_color = 0xFF000000;
        fb.clear(clear_color);

        // Place sand at (5, 0)
        let color = 0xFFFFFF00;
        sand.grid[5] = Cell::Sand(color);

        // Update 1 tick
        sand.update_and_draw(&mut fb, clear_color);

        // Sand should move to (5, 1)
        assert_eq!(sand.grid[5], Cell::Empty);
        assert_eq!(sand.grid[15], Cell::Sand(color));

        // fb should be colored at (5, 1)
        assert_eq!(fb.get_pixel(5, 1).unwrap(), color);
    }
}
