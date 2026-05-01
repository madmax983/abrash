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

pub mod funcs;
pub mod mat2;
pub mod mat3;
pub mod mat4;
pub mod quat;
pub mod screen_point;
#[cfg(test)]
mod tests;
pub mod vec2;
pub mod vec3;
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
