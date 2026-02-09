//! 2D Rasterization primitives.
//!
//! Software rendering functions for 3D triangles (flat, gouraud, textured, lit).

pub mod common;
pub mod flat;
pub mod gouraud;
pub mod textured;

// Public API
pub use common::color_to_u32;
pub use flat::fill_triangle_3d;
pub use gouraud::{fill_triangle_gouraud, fill_triangle_lit};
pub use textured::fill_triangle_textured;

// Crate-internal helpers (used by TileRenderer)
pub(crate) use common::{
    is_backface, sort_by_y, EdgeWalker,
};
pub(crate) use textured::{
    PerspectiveSpanStart, PerspectiveTextureEdgeWalker,
    PerspectiveTextureGradients, RECIPROCAL_TABLE,
};
