//! 2D Rasterization primitives.
//!
//! Software rendering functions for 3D triangles (flat, gouraud, textured, lit).
//!
//! # Rasterization Rules
//!
//! This module implements a standard **Scanline Rasterization** algorithm.
//!
//! 1.  **Triangle Setup**: Vertices are sorted by Y-coordinate.
//! 2.  **Edge Walking**: The left and right edges of the triangle are traced row by row.
//! 3.  **Span Filling**: For each scanline, a horizontal span of pixels is filled between the left and right edges.
//! 4.  **Top-Left Rule**: To prevent double-drawing on shared edges, the rasterizer follows standard fill conventions.
//!
//! # Performance
//!
//! *   **Fixed-Point Math**: Internal interpolation often uses 16.16 fixed-point arithmetic for speed.
//! *   **Z-Buffering**: Depth testing is performed per-pixel.
//! *   **Clipping**: Triangles are clipped to the view frustum before rasterization to ensure safety.

mod common;
mod flat;
mod gouraud;
mod normal_map;
mod phong;
mod texture;
mod wireframe;

// Public API
pub use common::color_to_u32;
pub use flat::{fill_triangle_3d, fill_triangle_lit};
pub use gouraud::fill_triangle_gouraud;
pub use normal_map::fill_triangle_normal_mapped;
pub use phong::fill_triangle_phong;
pub use texture::fill_triangle_textured;
pub use wireframe::{draw_line_3d, fill_triangle_wireframe};

// Internal exports for tile_renderer (crate-private)
pub(crate) use common::{is_backface, sort_by_y, EdgeWalker, RECIPROCAL_TABLE};
pub(crate) use texture::{
    PerspectiveSpanStart, PerspectiveTextureEdgeWalker, PerspectiveTextureGradients,
};
