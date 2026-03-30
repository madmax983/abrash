#![allow(clippy::all, unused_variables, dead_code, unused_imports, unused_mut)]
//! Raycasting engine for the Abrash graphics project.
//!
//! Stateless free functions, borrow-only, `Send + Sync` by construction.
//! Designed for ECS integration: no owned state, no synchronization needed.

pub(crate) mod batch;
pub(crate) mod cast;
pub(crate) mod dda;
pub(crate) mod map;
pub(crate) mod types;

// Clean public facade
pub use batch::{cast_los_batch, cast_rays_batch};
pub use cast::{cast_los, cast_ray, cast_ray_detailed};
pub use map::{ArrayGridMap, GridMap};
pub use types::{Cell, DetailedHit, RayHit, Side, Vec2Fixed};
