//! Falling Sand Cellular Automata Simulation
//!
//! A procedural simulation grid for falling sand, water, and static walls.

#![cfg(feature = "nova")]

use crate::framebuffer::Framebuffer;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParticleType {
    Empty,
    Sand,
    Water,
    Wall,
}

#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub ptype: ParticleType,
    pub color: u32,
    pub updated_this_frame: bool,
}

impl Default for Particle {
    fn default() -> Self {
        Self {
            ptype: ParticleType::Empty,
            color: 0x0000_0000,
            updated_this_frame: false,
        }
    }
}

pub struct FallingSand {
    pub width: usize,
    pub height: usize,
    pub grid: Vec<Particle>,
    pub next_grid: Vec<Particle>,
}

impl FallingSand {
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            grid: vec![Particle::default(); width * height],
            next_grid: vec![Particle::default(); width * height],
        }
    }

    pub fn set_particle(&mut self, x: usize, y: usize, ptype: ParticleType, color: u32) {
        if x < self.width && y < self.height {
            let idx = y * self.width + x;
            self.grid[idx] = Particle {
                ptype,
                color,
                updated_this_frame: false,
            };
        }
    }

    pub fn update(&mut self) {
        self.next_grid.copy_from_slice(&self.grid);

        for p in &mut self.next_grid {
            p.updated_this_frame = false;
        }

        // Process bottom-to-top to allow particles to fall properly
        for y in (0..self.height).rev() {
            for x in 0..self.width {
                let idx = y * self.width + x;
                let p = self.grid[idx];

                if p.ptype == ParticleType::Empty || p.ptype == ParticleType::Wall {
                    continue;
                }

                // If somehow it was already updated
                if self.next_grid[idx].updated_this_frame {
                    continue;
                }

                match p.ptype {
                    ParticleType::Sand => self.update_sand(x, y, idx, p),
                    ParticleType::Water => self.update_water(x, y, idx, p),
                    _ => {}
                }
            }
        }

        self.grid.copy_from_slice(&self.next_grid);
    }

    fn update_sand(&mut self, x: usize, y: usize, idx: usize, p: Particle) {
        if y + 1 < self.height {
            let down = (y + 1) * self.width + x;
            if self.is_empty_in_next(down) {
                self.move_particle(idx, down, p);
                return;
            }

            let left_ok = x > 0;
            let right_ok = x + 1 < self.width;
            let dir = (x + y) % 2 == 0;

            if dir {
                if left_ok {
                    let down_left = (y + 1) * self.width + (x - 1);
                    if self.is_empty_in_next(down_left) {
                        self.move_particle(idx, down_left, p);
                        return;
                    }
                }
                if right_ok {
                    let down_right = (y + 1) * self.width + (x + 1);
                    if self.is_empty_in_next(down_right) {
                        self.move_particle(idx, down_right, p);
                        return;
                    }
                }
            } else {
                if right_ok {
                    let down_right = (y + 1) * self.width + (x + 1);
                    if self.is_empty_in_next(down_right) {
                        self.move_particle(idx, down_right, p);
                        return;
                    }
                }
                if left_ok {
                    let down_left = (y + 1) * self.width + (x - 1);
                    if self.is_empty_in_next(down_left) {
                        self.move_particle(idx, down_left, p);
                        return;
                    }
                }
            }
        }
    }

    fn update_water(&mut self, x: usize, y: usize, idx: usize, p: Particle) {
        if y + 1 < self.height {
            let down = (y + 1) * self.width + x;
            if self.is_empty_in_next(down) {
                self.move_particle(idx, down, p);
                return;
            }

            let left_ok = x > 0;
            let right_ok = x + 1 < self.width;
            let dir = (x + y) % 2 == 0;

            if dir {
                if left_ok {
                    let down_left = (y + 1) * self.width + (x - 1);
                    if self.is_empty_in_next(down_left) {
                        self.move_particle(idx, down_left, p);
                        return;
                    }
                }
                if right_ok {
                    let down_right = (y + 1) * self.width + (x + 1);
                    if self.is_empty_in_next(down_right) {
                        self.move_particle(idx, down_right, p);
                        return;
                    }
                }
            } else {
                if right_ok {
                    let down_right = (y + 1) * self.width + (x + 1);
                    if self.is_empty_in_next(down_right) {
                        self.move_particle(idx, down_right, p);
                        return;
                    }
                }
                if left_ok {
                    let down_left = (y + 1) * self.width + (x - 1);
                    if self.is_empty_in_next(down_left) {
                        self.move_particle(idx, down_left, p);
                        return;
                    }
                }
            }
        }

        let left_ok = x > 0;
        let right_ok = x + 1 < self.width;
        let dir = (x + y) % 2 == 0;

        if dir {
            if left_ok {
                let left = y * self.width + (x - 1);
                if self.is_empty_in_next(left) {
                    self.move_particle(idx, left, p);
                    return;
                }
            }
            if right_ok {
                let right = y * self.width + (x + 1);
                if self.is_empty_in_next(right) {
                    self.move_particle(idx, right, p);
                    return;
                }
            }
        } else {
            if right_ok {
                let right = y * self.width + (x + 1);
                if self.is_empty_in_next(right) {
                    self.move_particle(idx, right, p);
                    return;
                }
            }
            if left_ok {
                let left = y * self.width + (x - 1);
                if self.is_empty_in_next(left) {
                    self.move_particle(idx, left, p);
                    return;
                }
            }
        }
    }

    fn is_empty_in_next(&self, idx: usize) -> bool {
        self.next_grid[idx].ptype == ParticleType::Empty
    }

    fn move_particle(&mut self, from_idx: usize, to_idx: usize, mut p: Particle) {
        p.updated_this_frame = true;
        self.next_grid[to_idx] = p;
        self.next_grid[from_idx] = Particle::default();
    }

    pub fn draw(&self, fb: &mut Framebuffer) {
        let width = self.width.min(fb.width() as usize);
        let height = self.height.min(fb.height() as usize);

        for y in 0..height {
            for x in 0..width {
                let idx = y * self.width + x;
                let p = self.grid[idx];
                if p.ptype != ParticleType::Empty {
                    fb.set_pixel(x as i32, y as i32, p.color);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_falling_sand_basic() {
        let mut sim = FallingSand::new(10, 10);
        sim.set_particle(5, 5, ParticleType::Sand, 0xFFFFFFFF);
        sim.update();
        assert_eq!(sim.grid[5 * 10 + 5].ptype, ParticleType::Empty);
        assert_eq!(sim.grid[6 * 10 + 5].ptype, ParticleType::Sand);
    }

    #[test]
    fn test_falling_sand_floor() {
        let mut sim = FallingSand::new(10, 10);
        sim.set_particle(5, 9, ParticleType::Sand, 0xFFFFFFFF);
        sim.update();
        assert_eq!(sim.grid[9 * 10 + 5].ptype, ParticleType::Sand);
    }

    #[test]
    fn test_water_horizontal() {
        let mut sim = FallingSand::new(10, 10);
        sim.set_particle(4, 9, ParticleType::Wall, 0xFFFFFFFF);
        sim.set_particle(5, 9, ParticleType::Wall, 0xFFFFFFFF);
        sim.set_particle(6, 9, ParticleType::Wall, 0xFFFFFFFF);
        sim.set_particle(5, 8, ParticleType::Water, 0xFF0000FF);
        sim.update();
        let moved = sim.grid[8 * 10 + 4].ptype == ParticleType::Water
            || sim.grid[8 * 10 + 6].ptype == ParticleType::Water;
        assert!(moved);
    }

    #[test]
    fn test_water_horizontal_collision() {
        let mut sim = FallingSand::new(10, 10);
        sim.set_particle(4, 9, ParticleType::Wall, 0xFFFFFFFF);
        sim.set_particle(5, 9, ParticleType::Wall, 0xFFFFFFFF);
        sim.set_particle(6, 9, ParticleType::Wall, 0xFFFFFFFF);
        sim.set_particle(4, 8, ParticleType::Water, 0xFF0000FF);
        sim.set_particle(5, 8, ParticleType::Water, 0xFF0000FF);
        sim.update();
        let water_count = sim
            .grid
            .iter()
            .filter(|p| p.ptype == ParticleType::Water)
            .count();
        assert_eq!(water_count, 2);
    }
}
