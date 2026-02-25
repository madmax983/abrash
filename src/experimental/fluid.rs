//! Fluid Simulation Module
//!
//! Implements a 2D grid-based fluid solver (Stable Fluids) for generating dynamic textures.
//!
//! # Concepts
//!
//! *   **Density**: The amount of "stuff" (smoke, dye) at each grid cell.
//! *   **Velocity**: The speed and direction of flow at each grid cell.
//! *   **Advection**: Moving density along the velocity field.
//! *   **Diffusion**: Spreading density/velocity to neighbors.
//! *   **Projection**: Enforcing incompressibility (mass conservation).

use crate::texture::Texture;

/// A simple 2D fluid solver.
pub struct Fluid {
    /// Grid size (square grid, size x size).
    pub size: usize,
    /// Time step.
    pub dt: f32,
    /// Diffusion rate.
    pub diff: f32,
    /// Viscosity.
    pub visc: f32,

    // Density fields
    s: Vec<f32>,       // Previous density
    density: Vec<f32>, // Current density

    // Velocity fields
    vx: Vec<f32>,
    vy: Vec<f32>,
    vx0: Vec<f32>,
    vy0: Vec<f32>,
}

impl Fluid {
    /// Creates a new fluid simulation instance.
    ///
    /// # Arguments
    ///
    /// *   `size` - Grid dimension (e.g., 64 for a 64x64 grid).
    /// *   `diffusion` - Rate at which density spreads (e.g., 0.0001).
    /// *   `viscosity` - Resistance to flow (e.g., 0.00001).
    /// *   `dt` - Time step per update (e.g., 0.1).
    #[must_use]
    pub fn new(size: usize, diffusion: f32, viscosity: f32, dt: f32) -> Self {
        let len = size * size;
        Self {
            size,
            dt,
            diff: diffusion,
            visc: viscosity,
            s: vec![0.0; len],
            density: vec![0.0; len],
            vx: vec![0.0; len],
            vy: vec![0.0; len],
            vx0: vec![0.0; len],
            vy0: vec![0.0; len],
        }
    }

    /// Adds density to a specific cell.
    pub fn add_density(&mut self, x: usize, y: usize, amount: f32) {
        let idx = self.ix(x, y);
        self.density[idx] += amount;
    }

    /// Adds velocity to a specific cell.
    pub fn add_velocity(&mut self, x: usize, y: usize, amount_x: f32, amount_y: f32) {
        let idx = self.ix(x, y);
        self.vx[idx] += amount_x;
        self.vy[idx] += amount_y;
    }

    /// Advances the simulation by one time step.
    pub fn step(&mut self) {
        let n = self.size;
        let visc = self.visc;
        let diff = self.diff;
        let dt = self.dt;

        // Diffuse velocity
        diffuse(1, &mut self.vx0, &self.vx, visc, dt, n);
        diffuse(2, &mut self.vy0, &self.vy, visc, dt, n);

        // Project (make incompressible)
        project(&mut self.vx0, &mut self.vy0, &mut self.vx, &mut self.vy, n);

        // Advect velocity
        advect(1, &mut self.vx, &self.vx0, &self.vx0, &self.vy0, dt, n);
        advect(2, &mut self.vy, &self.vy0, &self.vx0, &self.vy0, dt, n);

        // Project again
        project(&mut self.vx, &mut self.vy, &mut self.vx0, &mut self.vy0, n);

        // Diffuse density
        diffuse(0, &mut self.s, &self.density, diff, dt, n);

        // Advect density
        advect(0, &mut self.density, &self.s, &self.vx, &self.vy, dt, n);
    }

    /// Renders the density field to a Texture.
    ///
    /// Maps density to a blue-ish smoke color.
    pub fn render_to_texture(&self, texture: &mut Texture) {
        let w = texture.width();
        let h = texture.height();

        // Scale fluid grid to texture size
        let scale_x = self.size as f32 / w as f32;
        let scale_y = self.size as f32 / h as f32;

        let pixels = texture.pixels_mut();

        for y in 0..h {
            for x in 0..w {
                // Nearest neighbor sampling of fluid grid
                let fx = (x as f32 * scale_x) as usize;
                let fy = (y as f32 * scale_y) as usize;

                // Clamp to be safe
                let fx = fx.min(self.size - 1);
                let fy = fy.min(self.size - 1);

                let d = self.density[self.ix(fx, fy)];

                // Color mapping: Black -> Blue -> White
                // Density 0..1
                let val = d.clamp(0.0, 1.0);

                // Simple blue tint
                let r = (val * 200.0) as u32;
                let g = (val * 200.0) as u32;
                let b = (val * 255.0) as u32;

                let color = 0xFF000000 | (r << 16) | (g << 8) | b;
                pixels[(y as usize * w as usize) + x as usize] = color;
            }
        }
    }

    fn ix(&self, x: usize, y: usize) -> usize {
        let x = x.clamp(0, self.size - 1);
        let y = y.clamp(0, self.size - 1);
        x + y * self.size
    }
}

// Private helper functions

fn ix(x: usize, y: usize, n: usize) -> usize {
    let x = x.clamp(0, n - 1);
    let y = y.clamp(0, n - 1);
    x + y * n
}

fn set_bnd(b: i32, x: &mut [f32], n: usize) {
    // Handle edges
    for i in 1..n - 1 {
        x[ix(0, i, n)] = if b == 1 {
            -x[ix(1, i, n)]
        } else {
            x[ix(1, i, n)]
        };
        x[ix(n - 1, i, n)] = if b == 1 {
            -x[ix(n - 2, i, n)]
        } else {
            x[ix(n - 2, i, n)]
        };
    }
    for i in 1..n - 1 {
        x[ix(i, 0, n)] = if b == 2 {
            -x[ix(i, 1, n)]
        } else {
            x[ix(i, 1, n)]
        };
        x[ix(i, n - 1, n)] = if b == 2 {
            -x[ix(i, n - 2, n)]
        } else {
            x[ix(i, n - 2, n)]
        };
    }

    // Corners
    x[ix(0, 0, n)] = 0.5 * (x[ix(1, 0, n)] + x[ix(0, 1, n)]);
    x[ix(0, n - 1, n)] = 0.5 * (x[ix(1, n - 1, n)] + x[ix(0, n - 2, n)]);
    x[ix(n - 1, 0, n)] = 0.5 * (x[ix(n - 2, 0, n)] + x[ix(n - 1, 1, n)]);
    x[ix(n - 1, n - 1, n)] = 0.5 * (x[ix(n - 2, n - 1, n)] + x[ix(n - 1, n - 2, n)]);
}

fn lin_solve(b: i32, x: &mut [f32], x0: &[f32], a: f32, c: f32, iter: usize, n: usize) {
    let c_recip = 1.0 / c;
    for _ in 0..iter {
        for j in 1..n - 1 {
            for i in 1..n - 1 {
                x[ix(i, j, n)] = (x0[ix(i, j, n)]
                    + a * (x[ix(i + 1, j, n)]
                        + x[ix(i - 1, j, n)]
                        + x[ix(i, j + 1, n)]
                        + x[ix(i, j - 1, n)]))
                    * c_recip;
            }
        }
        set_bnd(b, x, n);
    }
}

fn diffuse(b: i32, x: &mut [f32], x0: &[f32], diff: f32, dt: f32, n: usize) {
    let a = dt * diff * (n - 2) as f32 * (n - 2) as f32;
    lin_solve(b, x, x0, a, 1.0 + 4.0 * a, 4, n); // 4 iterations for speed
}

fn project(vx: &mut [f32], vy: &mut [f32], p: &mut [f32], div: &mut [f32], n: usize) {
    let h = 1.0 / n as f32;
    for j in 1..n - 1 {
        for i in 1..n - 1 {
            div[ix(i, j, n)] = -0.5
                * h
                * (vx[ix(i + 1, j, n)] - vx[ix(i - 1, j, n)] + vy[ix(i, j + 1, n)]
                    - vy[ix(i, j - 1, n)]);
            p[ix(i, j, n)] = 0.0;
        }
    }
    set_bnd(0, div, n);
    set_bnd(0, p, n);

    lin_solve(0, p, div, 1.0, 4.0, 4, n);

    for j in 1..n - 1 {
        for i in 1..n - 1 {
            vx[ix(i, j, n)] -= 0.5 * (p[ix(i + 1, j, n)] - p[ix(i - 1, j, n)]) / h;
            vy[ix(i, j, n)] -= 0.5 * (p[ix(i, j + 1, n)] - p[ix(i, j - 1, n)]) / h;
        }
    }
    set_bnd(1, vx, n);
    set_bnd(2, vy, n);
}

fn advect(b: i32, d: &mut [f32], d0: &[f32], vx: &[f32], vy: &[f32], dt: f32, n: usize) {
    let dt0 = dt * (n - 2) as f32;
    let n_float = n as f32;

    for j in 1..n - 1 {
        for i in 1..n - 1 {
            let mut x = i as f32 - dt0 * vx[ix(i, j, n)];
            let mut y = j as f32 - dt0 * vy[ix(i, j, n)];

            if x < 0.5 {
                x = 0.5;
            }
            if x > n_float - 1.5 {
                x = n_float - 1.5;
            }
            if y < 0.5 {
                y = 0.5;
            }
            if y > n_float - 1.5 {
                y = n_float - 1.5;
            }

            let i0 = x as usize;
            let i1 = i0 + 1;
            let j0 = y as usize;
            let j1 = j0 + 1;

            let s1 = x - i0 as f32;
            let s0 = 1.0 - s1;
            let t1 = y - j0 as f32;
            let t0 = 1.0 - t1;

            d[ix(i, j, n)] = s0 * (t0 * d0[ix(i0, j0, n)] + t1 * d0[ix(i0, j1, n)])
                + s1 * (t0 * d0[ix(i1, j0, n)] + t1 * d0[ix(i1, j1, n)]);
        }
    }
    set_bnd(b, d, n);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fluid_creation() {
        let f = Fluid::new(32, 0.0, 0.0, 0.1);
        assert_eq!(f.size, 32);
        assert_eq!(f.density.len(), 32 * 32);
    }

    #[test]
    fn test_add_density() {
        let mut f = Fluid::new(32, 0.0, 0.0, 0.1);
        f.add_density(16, 16, 10.0);
        let idx = f.ix(16, 16);
        assert!((f.density[idx] - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_fluid_step() {
        let mut f = Fluid::new(32, 0.0001, 0.0, 0.1);
        f.add_density(16, 16, 100.0);
        f.add_velocity(16, 16, 1.0, 0.0);

        // Step
        f.step();

        // Density should have spread
        // We can check that neighbors have > 0 density
        let idx = f.ix(16, 16);
        let idx_right = f.ix(17, 16);

        assert!(f.density[idx] < 100.0); // Should decrease at source
        assert!(f.density[idx_right] > 0.0); // Should move right/spread
    }
}
