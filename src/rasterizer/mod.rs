//! 2D Rasterization primitives.
//!
//! Software rendering functions for 2D shapes (lines, circles, triangles).

pub mod lines;
pub mod primitives;
pub mod triangles;

pub(crate) mod edge;
pub(crate) mod scanline;

// Re-export public API
pub use lines::{draw_hline, draw_line, draw_vline};
pub use primitives::{draw_circle, draw_polygon, fill_circle, plot_pixel};
pub use triangles::{
    fill_triangle, fill_triangle_3d, fill_triangle_flat, fill_triangle_gouraud, fill_triangle_lit,
};
