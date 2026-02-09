//! 2D Rasterization primitives.
//!
//! Software rendering functions for 3D triangles (flat, gouraud, textured, lit).

pub mod common;
pub mod flat;
pub mod gouraud;
pub mod lit;
pub mod textured;

// Internal helpers needed by tile_renderer
pub(crate) use common::{
    EdgeWalker, is_backface, sort_by_y,
};
pub(crate) use textured::{
    PerspectiveSpanStart, PerspectiveTextureEdgeWalker, RECIPROCAL_TABLE,
};

// Public API
pub use common::color_to_u32;
pub use flat::fill_triangle_3d;
pub use gouraud::fill_triangle_gouraud;
pub use lit::fill_triangle_lit;
pub use textured::{fill_triangle_textured, PerspectiveTextureGradients};
