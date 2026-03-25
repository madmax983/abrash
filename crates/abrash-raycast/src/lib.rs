#![allow(clippy::all, unused_variables, dead_code, unused_imports, unused_mut)]
//! Raycasting engine for the Abrash graphics project.
//!
//! Stateless free functions, borrow-only, `Send + Sync` by construction.
//! Designed for ECS integration: no owned state, no synchronization needed.

pub mod batch;
pub mod cast;
pub mod dda;
pub mod map;
pub mod types;
