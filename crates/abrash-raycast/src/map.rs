//! Map implementation for grid-based raycasting.

use crate::types::Cell;

/// A simple owned grid map backed by a flat `Vec<Cell>`.
#[derive(Clone, Debug)]
pub struct ArrayGridMap {
    width: u32,
    height: u32,
    cells: Vec<Cell>,
}

impl ArrayGridMap {
    /// Create a new map filled with `Cell::Empty`.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::Empty; (width as usize) * (height as usize)],
        }
    }

    /// Set the cell at `(x, y)`.
    ///
    /// # Panics
    /// Panics if `x >= width` or `y >= height`.
    pub fn set(&mut self, x: u32, y: u32, cell: Cell) {
        assert!(
            x < self.width,
            "x ({x}) out of bounds (width {})",
            self.width
        );
        assert!(
            y < self.height,
            "y ({y}) out of bounds (height {})",
            self.height
        );
        let idx = (y as usize) * (self.width as usize) + (x as usize);
        self.cells[idx] = cell;
    }

    /// Width of the grid in cells.
    #[inline]
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Height of the grid in cells.
    #[inline]
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Return the cell at `(x, y)`. Coordinates are zero-based.
    ///
    /// # Panics
    /// May panic if `x >= width()` or `y >= height()`.
    #[inline]
    #[must_use]
    pub fn cell_at(&self, x: u32, y: u32) -> Cell {
        assert!(
            x < self.width,
            "x ({x}) out of bounds (width {})",
            self.width
        );
        assert!(
            y < self.height,
            "y ({y}) out of bounds (height {})",
            self.height
        );
        let idx = (y as usize) * (self.width as usize) + (x as usize);
        self.cells[idx]
    }
}
