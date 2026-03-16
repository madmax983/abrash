//! Stable engine-facing render API.
//!
//! This module provides the consumer-facing abstractions for the Abrash engine.
//! External projects should depend on these types rather than the algorithm-level
//! functions in [`crate::rasterizer`].

pub mod handles;
pub mod target;

pub use handles::{Handle, MaterialHandle, MeshHandle, ResourcePool, TextureHandle};
pub use target::RenderTarget;
