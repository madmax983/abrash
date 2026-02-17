//! 2D Rasterization primitives.
//!
//! Software rendering functions for 3D triangles (flat, gouraud, textured, lit).
//!
//! This module was refactored to split the monolithic implementation into cohesive submodules.

pub mod core;
pub mod flat;
pub mod gouraud;
pub mod line;
pub mod phong;
pub mod texture;

// Re-export public API
pub use self::core::{FIXED_SCALE, color_to_u32};
pub use self::flat::{draw_scanline_flat, draw_scanline_flat_blended, fill_triangle_3d};
pub use self::gouraud::{draw_scanline_gouraud, fill_triangle_gouraud};
pub use self::line::{draw_line_3d, fill_triangle_wireframe};
pub use self::phong::{
    fill_triangle_lit, fill_triangle_phong, fill_triangle_phong_shadowed, fill_triangle_point_lit,
};
pub use self::texture::{
    PerspectiveSpanStart, PerspectiveTextureGradients, draw_scanline_textured_perspective,
    fill_triangle_normal_mapped, fill_triangle_textured, fill_triangle_textured_gouraud,
};

// Re-export internal helpers for experimental modules
