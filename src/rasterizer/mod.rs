//! 2D Rasterization primitives.
//!
//! Software rendering functions for 3D triangles (flat, gouraud, textured, lit).
//!
//! This module was refactored to split the monolithic implementation into cohesive submodules.

pub mod core;
pub mod flat;
pub mod gouraud;
pub mod phong;
pub mod texture;

// Re-export public API
pub use self::flat::{fill_triangle_3d, draw_scanline_flat, draw_scanline_flat_blended};
pub use self::gouraud::{fill_triangle_gouraud, draw_scanline_gouraud};
pub use self::phong::{
    fill_triangle_phong, fill_triangle_phong_shadowed, fill_triangle_point_lit, fill_triangle_lit,
};
pub use self::texture::{
    draw_scanline_textured_perspective, fill_triangle_normal_mapped, fill_triangle_textured,
    PerspectiveSpanStart, PerspectiveTextureGradients,
};
pub use self::core::{color_to_u32, FIXED_SCALE};

// Re-export internal helpers for experimental modules
pub(crate) use self::core::sort_by_y;
