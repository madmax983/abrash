//! Map traits and implementations for grid-based raycasting.

use crate::types::Cell;

/// A rectangular grid of cells that rays can be cast against.
pub trait GridMap {
    /// Width of the grid in cells.
    fn width(&self) -> u32;

    /// Height of the grid in cells.
    fn height(&self) -> u32;

    /// Return the cell at `(x, y)`. Coordinates are zero-based.
    ///
    /// # Panics
    /// May panic if `x >= width()` or `y >= height()`.
    fn cell_at(&self, x: u32, y: u32) -> Cell;
}

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
}

impl GridMap for ArrayGridMap {
    #[inline]
    fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    fn height(&self) -> u32 {
        self.height
    }

    #[inline]
    fn cell_at(&self, x: u32, y: u32) -> Cell {
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
