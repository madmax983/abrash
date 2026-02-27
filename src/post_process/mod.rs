//! # Post-Processing Pipeline 🎞️
//!
//! This module provides screen-space effects that are applied to the framebuffer *after* the rasterization stage.
//! Post-processing is essential for adding polish, realism, and artistic style to the rendered image.
//!
//! ## Overview
//!
//! The pipeline operates on the [`crate::framebuffer::Framebuffer`] (Color) and [`crate::zbuffer::ZBuffer`] (Depth).
//! Most effects modify the framebuffer in-place, allowing you to chain multiple effects together.
//!
//! ## Available Effects
//!
//! ### 1. Filters ([`filters`])
//! Simple pixel-shader-like effects that modify colors based on simple rules or convolution kernels.
//! *   **Color Correction**: [`apply_grayscale`], [`apply_sepia`], [`apply_invert`]
//! *   **Stylization**: [`apply_scanlines`], [`apply_vignette`]
//! *   **Lens Effects**: [`apply_chromatic_aberration`]
//! *   **Edge Detection**: [`apply_sobel`]
//!
//! ### 2. Depth of Field ([`dof`])
//! *   **Function**: [`apply_depth_of_field`]
//! *   **Description**: Simulates camera lens focus by blurring out-of-focus areas based on depth.
//!
//! ### 3. Ambient Occlusion ([`ssao`])
//! *   **Function**: [`apply_ssao`]
//! *   **Description**: Adds contact shadows by estimating occlusion from the depth buffer.
//!
//! ### 4. Bloom ([`bloom`])
//! *   **Function**: [`apply_bloom`]
//! *   **Description**: Adds a glow to bright areas of the image.
//!
//! ## Usage Example
//!
//! Here is how you might chain multiple effects to create a cinematic look:
//!
//! ```
//! use abrash::framebuffer::Framebuffer;
//! use abrash::zbuffer::ZBuffer;
//! use abrash::math::Mat4;
//! use abrash::post_process::{apply_ssao, apply_depth_of_field, apply_vignette, apply_grayscale};
//!
//! // 1. Setup Buffers (Assuming they are filled by the rasterizer)
//! let width = 800;
//! let height = 600;
//! let mut fb = Framebuffer::new(width, height).unwrap();
//! let mut zb = ZBuffer::new(width, height).unwrap();
//!
//! // Assume projection matrix used during rendering
//! let proj = Mat4::perspective(1.57, 1.33, 0.1, 100.0);
//!
//! // 2. Apply Post-Processing Chain
//!
//! // Step A: SSAO (Needs Depth) - Adds shadows to corners/crevices
//! apply_ssao(&mut fb, &zb, &proj, 0.5, 0.025, 2.0);
//!
//! // Step B: Depth of Field (Needs Depth) - Blurs background
//! // Focus at depth 5.0, range 2.0, blur radius 3
//! apply_depth_of_field(&mut fb, &zb, 5.0, 2.0, 3);
//!
//! // Step C: Vignette (Stylistic) - Darkens edges
//! apply_vignette(&mut fb, 0.5, 0.5);
//!
//! // Step D: Color Grading (Optional)
//! // apply_grayscale(&mut fb); // Uncomment for noir style
//! ```

pub mod bloom;
pub mod blur;
pub mod filters;
pub mod ssao;
pub mod vision;

pub use self::bloom::*;
pub use self::blur::*;
pub use self::filters::*;
pub use self::ssao::*;
pub use self::vision::*;

pub mod dof;
pub use self::dof::*;
