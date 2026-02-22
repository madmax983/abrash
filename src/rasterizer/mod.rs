//! # The Rasterizer 🎨
//!
//! This module is the heart of the Abrash rendering engine. It is responsible for the "Rasterization" stage
//! of the graphics pipeline: turning 3D triangles (in Clip Space) into 2D pixels on the screen.
//!
//! ## The Graphics Pipeline
//!
//! The rasterizer sits between the Vertex Processing stage and the Framebuffer:
//!
//! 1.  **Input**: Vertices in Homogeneous Clip Space (output of the Vertex Shader/Transform stage).
//! 2.  **Clipping**: Triangles are clipped against the view frustum (handled by [`crate::clipping`]).
//! 3.  **Perspective Division**: Converting Homogeneous coordinates $(x, y, z, w)$ to Normalized Device Coordinates (NDC) $(x/w, y/w, z/w)$.
//! 4.  **Viewport Mapping**: Converting NDC to Screen Space $(x_{screen}, y_{screen})$.
//! 5.  **Scan Conversion**: Finding all pixels that lie inside the triangle.
//! 6.  **Interpolation**: Computing per-pixel attributes (Z-depth, Color, Texture Coords) from vertex attributes.
//! 7.  **Shading**: Calculating the final pixel color.
//!
//! ## Rasterization Modes
//!
//! Abrash supports multiple rasterization modes, trading off performance for visual fidelity:
//!
//! ### 1. Flat Shading (`flat`)
//! *   **Description**: The simplest mode. Fills the entire triangle with a single color.
//! *   **Use Case**: Wireframes, low-poly art, or debug views.
//! *   **Performance**: ⚡⚡⚡⚡⚡ (Fastest)
//! *   **Function**: [`fill_triangle_3d`]
//!
//! ### 2. Gouraud Shading (`gouraud`)
//! *   **Description**: Calculates lighting at vertices and interpolates the *color* across the triangle.
//! *   **Use Case**: Smooth lighting on curved surfaces without textures.
//! *   **Performance**: ⚡⚡⚡⚡ (Fast)
//! *   **Function**: [`fill_triangle_gouraud`]
//!
//! ### 3. Perspective-Correct Texture Mapping (`texture`)
//! *   **Description**: Maps a 2D image (Texture) onto the 3D triangle. Corrects for perspective distortion
//!     by interpolating $1/w$, $u/w$, and $v/w$.
//! *   **Use Case**: Realistic surfaces (wood, stone, skin).
//! *   **Performance**: ⚡⚡⚡ (Moderate)
//! *   **Function**: [`fill_triangle_textured`]
//!
//! ### 4. Phong Shading (`phong`)
//! *   **Description**: Interpolates *normals* across the triangle and calculates lighting *per-pixel*.
//! *   **Use Case**: High-quality specular highlights (shininess) on curved surfaces.
//! *   **Performance**: ⚡⚡ (Slow)
//! *   **Function**: [`fill_triangle_phong`]
//!
//! ### 5. Normal Mapping & Shadows (`texture`, `phong`)
//! *   **Description**: Combines normal maps (bump mapping) and shadow maps for high-fidelity rendering.
//! *   **Use Case**: AAA-style graphics with surface detail and occlusion.
//! *   **Performance**: ⚡ (Heavy)
//! *   **Function**: [`fill_triangle_normal_mapped`], [`fill_triangle_phong_shadowed`]
//!
//! ## Implementation Details
//!
//! The rasterizer uses a **scanline** approach. It breaks the triangle into two segments (top-half and bottom-half)
//! and iterates row by row.
//!
//! *   **Edge Walking**: Uses an internal `EdgeWalker` to interpolate X coordinates and attributes along the left and right edges.
//! *   **Span Drawing**: For each scanline, it iterates from `x_start` to `x_end`, interpolating attributes horizontally and writing to the framebuffer.
//! *   **SIMD**: Key paths (like texture mapping and lighting) are optimized with AVX2 intrinsics for modern CPUs.

pub mod core;
pub mod flat;
pub mod gouraud;
pub mod line;
pub mod phong;
pub mod reflection;
pub mod texture;

// Re-export public API
pub use self::core::{FIXED_SCALE, color_to_u32};
pub use self::flat::{draw_scanline_flat, draw_scanline_flat_blended, fill_triangle_3d};
pub use self::gouraud::{draw_scanline_gouraud, fill_triangle_gouraud};
pub use self::line::{draw_line_3d, fill_triangle_wireframe};
pub use self::phong::{
    fill_triangle_lit, fill_triangle_phong, fill_triangle_phong_shadowed, fill_triangle_point_lit,
};
pub use self::reflection::fill_triangle_reflection;
pub use self::texture::{
    PerspectiveSpanStart, PerspectiveTextureGradients, TexturedGouraudGradients,
    TexturedGouraudSpanStart, draw_scanline_textured_gouraud, draw_scanline_textured_perspective,
    fill_quad_textured, fill_quad_textured_gouraud, fill_triangle_normal_mapped,
    fill_triangle_textured, fill_triangle_textured_gouraud,
};

// Re-export internal helpers for experimental modules
pub(crate) use self::core::{EdgeWalker, is_backface, sort_by_y};
pub(crate) use self::texture::{PerspectiveTextureEdgeWalker, RECIPROCAL_TABLE};
