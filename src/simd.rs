//! SIMD-optimized operations using inline assembly.
//!
//! Provides x86-64 SSE/AVX implementations of hot-path operations.
//! Falls back to scalar implementations on non-x86 platforms.

#[cfg(target_arch = "x86_64")]
pub mod x86_64;

#[cfg(target_arch = "x86_64")]
pub mod primitives;

#[cfg(target_arch = "x86_64")]
pub mod primitives_batched;

#[cfg(target_arch = "x86_64")]
pub mod vertex_transform;

#[cfg(target_arch = "x86_64")]
pub mod vertex_transform_intrinsics;

#[cfg(target_arch = "x86_64")]
pub use x86_64::*;

#[cfg(target_arch = "x86_64")]
pub use primitives::*;

#[cfg(target_arch = "x86_64")]
pub use primitives_batched::*;

#[cfg(target_arch = "x86_64")]
pub use vertex_transform::*;

#[cfg(target_arch = "x86_64")]
pub use vertex_transform_intrinsics::*;

// Re-export types for convenience
pub use crate::math::{Mat4, Vec3};
