#![allow(clippy::imprecise_flops)]
#![allow(clippy::suspicious_operation_groupings)]
#![allow(clippy::must_use_candidate)]

//! 2D and 3D math types for graphics programming.
//!
//! # Coordinate System
//!
//! This library uses a **Right-Handed** coordinate system.
//! *   **X**: Right
//! *   **Y**: Up
//! *   **Z**: Backward (Camera looks down -Z)
//!
//! # Matrix Convention
//!
//! Matrices are stored in **Row-Major** order.
//!
//! Transformations follow the **Row-Vector** convention ($v \cdot M$), meaning vectors are treated as rows and multiplied on the left.
//!
//! $$ v' = v \cdot M $$
//!
//! This implies that the order of multiplication matches the order of transformations:
//!
//! ```
//! # use abrash_core::math::{Mat4, Vec3};
//! // Scale, then Rotate, then Translate
//! let scale = Mat4::scale(2.0, 2.0, 2.0);
//! let rotate = Mat4::rotation_y(1.57); // 90 degrees
//! let translate = Mat4::translation(10.0, 0.0, 0.0);
//!
//! // Combine transformations
//! let transform = scale * rotate * translate;
//!
//! // Apply to a vector
//! let v = Vec3::new(1.0, 0.0, 0.0);
//! let (v_prime, _) = transform.transform_point(v);
//! ```
//!
//! All operations use `f32` for compatibility with graphics APIs.

/// High-performance trigonometric and polynomial functions for hot rendering loops.
pub mod funcs;
/// 2x2 Matrices, typically used for 2D rotational transforms.
pub mod mat2;
/// 3x3 Matrices, commonly used for normal transforms and 3D rotations without translation.
pub mod mat3;
/// 4x4 Homogeneous Matrices, the backbone of the 3D projection and model-view pipeline.
pub mod mat4;
/// Quaternions, providing gimbal-lock-free 3D rotations and smooth spherical interpolation (Slerp).
pub mod quat;
/// 2D Screen-space coordinates for exact pixel addressing.
pub mod screen_point;
#[cfg(test)]
mod tests;
/// 2D Vectors, essential for UV mapping and screen-space math.
pub mod vec2;
/// 3D Vectors, the fundamental primitive for world-space geometry, normals, and colors.
pub mod vec3;
/// 4D Homogeneous Vectors, used in clip-space transformations before perspective division.
pub mod vec4;

pub use funcs::*;
pub use mat2::*;
pub use mat3::*;
pub use mat4::*;
pub use quat::*;
pub use screen_point::*;
pub use vec2::*;
pub use vec3::*;
pub use vec4::*;
