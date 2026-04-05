//! Falling Sand Cellular Automata Simulation
//!
//! Simulates gravity-based cellular automata on a 2D grid.
//! Supports multiple material types (Sand, Water, Wall, Empty) with
//! distinct physics rules for falling, sliding, and flowing.

use abrash_core::framebuffer::Framebuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Material {
    Empty,
    Sand,
    Water,
    Wall,
}

impl Material {
    pub fn color(&self) -> u32 {
        match self {
            Material::Empty => 0xFF_00_00_00,
            Material::Sand => 0xFF_E2_C2_75,
            Material::Water => 0xFF_34_98_DB,
            Material::Wall => 0xFF_7F_8C_8D,
        }
    }
}

pub struct SandGrid {
    width: u32,
    height: u32,
    grid: Vec<Material>,
    next_grid: Vec<Material>,
}

impl SandGrid {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height) as usize;
        Self {
            width,
            height,
            grid: vec![Material::Empty; size],
            next_grid: vec![Material::Empty; size],
        }
    }

    pub fn set_material(&mut self, x: i32, y: i32, material: Material) {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            let idx = (y as u32 * self.width + x as u32) as usize;
            self.grid[idx] = material;
        }
    }

    pub fn get_material(&self, x: i32, y: i32) -> Material {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            let idx = (y as u32 * self.width + x as u32) as usize;
            self.grid[idx]
        } else {
            Material::Wall // Treat out of bounds as walls
        }
    }

    fn get_lateral_material(&self, x: i32, y: i32, going_right: bool, direction_is_right: bool) -> Material {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            let idx = (y as u32 * self.width + x as u32) as usize;
            if going_right {
                if !direction_is_right {
                    self.next_grid[idx]
                } else {
                    self.grid[idx]
                }
            } else {
                if direction_is_right {
                    self.next_grid[idx]
                } else {
                    self.grid[idx]
                }
            }
        } else {
            Material::Wall
        }
    }

    pub fn step(&mut self) {
        // Clear next_grid, we only care about Walls. Everything else will be moved or stayed.
        for (i, &mat) in self.grid.iter().enumerate() {
            if mat == Material::Wall {
                self.next_grid[i] = Material::Wall;
            } else {
                self.next_grid[i] = Material::Empty;
            }
        }

        // Iterate bottom-up to prevent objects moving multiple times per frame
        for y in (0..self.height as i32).rev() {
            // Alternate iteration direction each row to prevent bias
            let going_right = y % 2 == 0;

            let mut x = if going_right { 0 } else { self.width as i32 - 1 };

            while if going_right { x < self.width as i32 } else { x >= 0 } {
                let current = self.get_material(x, y);

                if current == Material::Sand {
                    self.update_sand(x, y);
                } else if current == Material::Water {
                    self.update_water(x, y, going_right);
                }

                if going_right {
                    x += 1;
                } else {
                    x -= 1;
                }
            }
        }

        self.grid.copy_from_slice(&self.next_grid);
    }

    fn update_sand(&mut self, x: i32, y: i32) {
        let idx = (y as u32 * self.width + x as u32) as usize;

        let below = self.get_next_material(x, y + 1);
        let below_left = self.get_next_material(x - 1, y + 1);
        let below_right = self.get_next_material(x + 1, y + 1);

        if self.is_empty_or_fluid(below) {
            self.move_particle(x, y, x, y + 1, idx);
        } else if self.is_empty_or_fluid(below_left) && self.is_empty_or_fluid(below_right) {
            // randomly pick left or right
            if (x + y) % 2 == 0 {
                self.move_particle(x, y, x - 1, y + 1, idx);
            } else {
                self.move_particle(x, y, x + 1, y + 1, idx);
            }
        } else if self.is_empty_or_fluid(below_left) {
            self.move_particle(x, y, x - 1, y + 1, idx);
        } else if self.is_empty_or_fluid(below_right) {
            self.move_particle(x, y, x + 1, y + 1, idx);
        } else {
            // Only place sand if nothing else swapped into this position
            if self.next_grid[idx] == Material::Empty {
                self.next_grid[idx] = Material::Sand;
            }
        }
    }

    fn get_next_material(&self, x: i32, y: i32) -> Material {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            let idx = (y as u32 * self.width + x as u32) as usize;
            self.next_grid[idx]
        } else {
            Material::Wall // Treat out of bounds as walls
        }
    }

    fn update_water(&mut self, x: i32, y: i32, going_right: bool) {
        let idx = (y as u32 * self.width + x as u32) as usize;

        let below = self.get_next_material(x, y + 1);
        let below_left = self.get_next_material(x - 1, y + 1);
        let below_right = self.get_next_material(x + 1, y + 1);
        let left = self.get_lateral_material(x - 1, y, going_right, false);
        let right = self.get_lateral_material(x + 1, y, going_right, true);

        if below == Material::Empty {
            self.move_particle(x, y, x, y + 1, idx);
        } else if below_left == Material::Empty && below_right == Material::Empty {
            if (x + y) % 2 == 0 {
                self.move_particle(x, y, x - 1, y + 1, idx);
            } else {
                self.move_particle(x, y, x + 1, y + 1, idx);
            }
        } else if below_left == Material::Empty {
            self.move_particle(x, y, x - 1, y + 1, idx);
        } else if below_right == Material::Empty {
            self.move_particle(x, y, x + 1, y + 1, idx);
        } else if left == Material::Empty && right == Material::Empty {
             if (x + y) % 2 == 0 {
                 self.move_particle(x, y, x - 1, y, idx);
             } else {
                 self.move_particle(x, y, x + 1, y, idx);
             }
        } else if left == Material::Empty {
            self.move_particle(x, y, x - 1, y, idx);
        } else if right == Material::Empty {
            self.move_particle(x, y, x + 1, y, idx);
        } else {
            self.next_grid[idx] = Material::Water;
        }
    }

    fn is_empty_or_fluid(&self, mat: Material) -> bool {
        mat == Material::Empty || mat == Material::Water
    }

    fn move_particle(&mut self, src_x: i32, src_y: i32, dst_x: i32, dst_y: i32, src_idx: usize) {
        let dst_idx = (dst_y as u32 * self.width + dst_x as u32) as usize;

        let src_mat = self.grid[src_idx];

        // Wait, if next_grid at dst_idx has something, that something was either there to begin with
        // AND didn't move (like water), OR it moved there this frame.
        // Let's look at grid.
        let old_dst_mat = self.grid[dst_idx];
        let new_dst_mat = self.next_grid[dst_idx];

        self.next_grid[dst_idx] = src_mat;

        // Always preserve whatever was in the grid before, as long as it isn't Empty!
        // But if it's something that just moved there, we probably shouldn't override it, but wait
        // since we are moving into an empty space or fluid.
        // Let's do a simple swap based on old_dst_mat.
        if old_dst_mat == Material::Water {
            self.next_grid[src_idx] = Material::Water;
        } else if new_dst_mat != Material::Empty && new_dst_mat != Material::Wall {
             // In case water moved into the spot we just moved to, put the water in our spot.
             self.next_grid[src_idx] = new_dst_mat;
        }
    }

    pub fn draw(&self, fb: &mut Framebuffer) {
        let mut pixels = fb.as_mut_slice();
        for (i, material) in self.grid.iter().enumerate() {
            pixels[i] = material.color();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sand_falls() {
        let mut grid = SandGrid::new(10, 10);
        grid.set_material(5, 5, Material::Sand);

        grid.step();

        assert_eq!(grid.get_material(5, 5), Material::Empty);
        assert_eq!(grid.get_material(5, 6), Material::Sand);
    }

    #[test]
    fn test_water_flows_horizontally() {
        let mut grid = SandGrid::new(10, 10);

        // Put water on flat ground
        for x in 0..10 {
            grid.set_material(x, 9, Material::Wall);
        }
        grid.set_material(5, 8, Material::Water);

        grid.step();

        assert_eq!(grid.get_material(5, 8), Material::Empty);
        let left = grid.get_material(4, 8) == Material::Water;
        let right = grid.get_material(6, 8) == Material::Water;
        assert!(left || right, "Water should have flowed left or right");
    }

    #[test]
    fn test_sand_falls_through_water() {
        let mut grid = SandGrid::new(10, 10);

        // Put sand above water in an enclosed space so water cannot flow sideways
        grid.set_material(4, 5, Material::Wall);
        grid.set_material(6, 5, Material::Wall);
        grid.set_material(4, 6, Material::Wall);
        grid.set_material(6, 6, Material::Wall);
        grid.set_material(5, 7, Material::Wall);

        grid.set_material(5, 5, Material::Sand);
        grid.set_material(5, 6, Material::Water);

        grid.step();

        assert_eq!(grid.get_material(5, 5), Material::Water);
        assert_eq!(grid.get_material(5, 6), Material::Sand);
    }
}
