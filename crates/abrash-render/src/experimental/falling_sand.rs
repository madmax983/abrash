//! Falling Sand Simulation
//!
//! An experimental cellular automata simulation for falling particles like Sand,
//! Water, and Wood.

use crate::framebuffer::Framebuffer;
use abrash_core::utils::XorShift32;

/// Types of particles in the simulation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParticleType {
    Empty,
    Sand,
    Water,
    Wood,
}

/// A single particle in the simulation grid.
#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub ptype: ParticleType,
    pub color: u32,
    pub last_updated: u32,
}

impl Default for Particle {
    fn default() -> Self {
        Self {
            ptype: ParticleType::Empty,
            color: 0x00000000,
            last_updated: 0,
        }
    }
}

/// Configuration and state for the Falling Sand simulation.
pub struct FallingSandSimulation {
    width: usize,
    height: usize,
    grid: Vec<Particle>,
    frame_count: u32,
    rng: XorShift32,
}

impl FallingSandSimulation {
    /// Creates a new falling sand simulation grid.
    #[must_use]
    pub fn new(width: usize, height: usize, seed: u32) -> Self {
        Self {
            width,
            height,
            grid: vec![Particle::default(); width * height],
            frame_count: 1,
            rng: XorShift32::new(seed),
        }
    }

    /// Sets a particle at the given coordinates.
    pub fn set_particle(&mut self, x: usize, y: usize, ptype: ParticleType, color: u32) {
        if x < self.width && y < self.height {
            let idx = self.get_index(x, y);
            self.grid[idx] = Particle {
                ptype,
                color,
                last_updated: self.frame_count,
            };
        }
    }

    /// Retrieves the particle at the given coordinates.
    #[must_use]
    pub fn get_particle(&self, x: usize, y: usize) -> Particle {
        if x < self.width && y < self.height {
            self.grid[self.get_index(x, y)]
        } else {
            Particle {
                ptype: ParticleType::Wood, // Treat out-of-bounds as solid wood
                color: 0,
                last_updated: 0,
            }
        }
    }

    fn get_index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    /// Swaps two particles in the grid.
    fn swap(&mut self, x1: usize, y1: usize, x2: usize, y2: usize) {
        let idx1 = self.get_index(x1, y1);
        let idx2 = self.get_index(x2, y2);
        self.grid.swap(idx1, idx2);
    }

    /// Updates the simulation by one tick.
    pub fn update(&mut self) {
        self.frame_count = self.frame_count.wrapping_add(1);
        if self.frame_count == 0 {
            self.frame_count = 1; // 0 is default
        }

        let dir = if self.rng.next_u32() % 2 == 0 { 1 } else { -1 };

        // Process from bottom to top
        for y in (0..self.height).rev() {
            // Process left-to-right or right-to-left randomly to avoid bias
            let x_iter: Box<dyn Iterator<Item = usize>> = if dir == 1 {
                Box::new(0..self.width)
            } else {
                Box::new((0..self.width).rev())
            };

            for x in x_iter {
                let idx = self.get_index(x, y);
                let p = self.grid[idx];

                if p.ptype == ParticleType::Empty || p.ptype == ParticleType::Wood {
                    continue;
                }

                if p.last_updated == self.frame_count {
                    continue;
                }

                // Mark as updated
                self.grid[idx].last_updated = self.frame_count;

                match p.ptype {
                    ParticleType::Sand => self.update_sand(x, y),
                    ParticleType::Water => self.update_water(x, y),
                    _ => {}
                }
            }
        }
    }

    fn update_sand(&mut self, x: usize, y: usize) {
        if y + 1 >= self.height {
            return; // At bottom
        }

        let below = self.get_particle(x, y + 1);
        if self.can_displace(ParticleType::Sand, below.ptype) {
            self.swap(x, y, x, y + 1);
            return;
        }

        // Try diagonals
        let left_first = self.rng.next_u32() % 2 == 0;
        let mut dxs = [1, -1];
        if left_first {
            dxs = [-1, 1];
        }

        for dx in dxs {
            let nx = x as isize + dx;
            if nx >= 0 && (nx as usize) < self.width {
                let nx = nx as usize;
                let diag = self.get_particle(nx, y + 1);
                if self.can_displace(ParticleType::Sand, diag.ptype) {
                    self.swap(x, y, nx, y + 1);
                    return;
                }
            }
        }
    }

    fn update_water(&mut self, x: usize, y: usize) {
        if y + 1 < self.height {
            let below = self.get_particle(x, y + 1);
            if self.can_displace(ParticleType::Water, below.ptype) {
                self.swap(x, y, x, y + 1);
                return;
            }

            // Try diagonals
            let left_first = self.rng.next_u32() % 2 == 0;
            let mut dxs = [1, -1];
            if left_first {
                dxs = [-1, 1];
            }

            let mut moved = false;
            for dx in dxs {
                let nx = x as isize + dx;
                if nx >= 0 && (nx as usize) < self.width {
                    let nx = nx as usize;
                    let diag = self.get_particle(nx, y + 1);
                    if self.can_displace(ParticleType::Water, diag.ptype) {
                        self.swap(x, y, nx, y + 1);
                        moved = true;
                        break;
                    }
                }
            }

            if moved {
                return;
            }
        }

        // Try sideways
        let left_first = self.rng.next_u32() % 2 == 0;
        let mut dxs = [1, -1];
        if left_first {
            dxs = [-1, 1];
        }

        for dx in dxs {
            let nx = x as isize + dx;
            if nx >= 0 && (nx as usize) < self.width {
                let nx = nx as usize;
                let side = self.get_particle(nx, y);
                if self.can_displace(ParticleType::Water, side.ptype) {
                    self.swap(x, y, nx, y);
                    return;
                }
            }
        }
    }

    fn can_displace(&self, moving: ParticleType, target: ParticleType) -> bool {
        if target == ParticleType::Empty {
            return true;
        }
        if moving == ParticleType::Sand && target == ParticleType::Water {
            return true; // Sand sinks in water
        }
        false
    }

    /// Renders the simulation to the given framebuffer.
    ///
    /// The simulation grid must be smaller or equal to the framebuffer size.
    pub fn apply(&self, fb: &mut Framebuffer) {
        let max_x = self.width.min(fb.width() as usize);
        let max_y = self.height.min(fb.height() as usize);

        for y in 0..max_y {
            for x in 0..max_x {
                let p = self.get_particle(x, y);
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
    fn test_falling_sand() {
        let mut sim = FallingSandSimulation::new(10, 10, 42);

        // Put sand at 5, 5
        sim.set_particle(5, 5, ParticleType::Sand, 0xFF_FF0000);
        assert_eq!(sim.get_particle(5, 5).ptype, ParticleType::Sand);

        sim.update();

        // Should fall to 5, 6
        assert_eq!(sim.get_particle(5, 5).ptype, ParticleType::Empty);
        assert_eq!(sim.get_particle(5, 6).ptype, ParticleType::Sand);
    }

    #[test]
    fn test_sand_sinks_in_water() {
        let mut sim = FallingSandSimulation::new(10, 10, 42);

        sim.set_particle(5, 5, ParticleType::Sand, 0xFF_FF0000);
        sim.set_particle(5, 6, ParticleType::Water, 0xFF_0000FF);
        sim.set_particle(5, 7, ParticleType::Wood, 0xFF_000000);
        sim.set_particle(4, 7, ParticleType::Wood, 0xFF_000000);
        sim.set_particle(6, 7, ParticleType::Wood, 0xFF_000000);
        sim.set_particle(4, 6, ParticleType::Wood, 0xFF_000000);
        sim.set_particle(6, 6, ParticleType::Wood, 0xFF_000000);

        sim.update();

        // Sand should be at 5,6 and Water at 5,5
        assert_eq!(sim.get_particle(5, 5).ptype, ParticleType::Water);
        assert_eq!(sim.get_particle(5, 6).ptype, ParticleType::Sand);
    }
}
