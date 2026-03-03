//! # Experimental Features
//!
//! This module contains experimental features, prototypes, and reference implementations that are
//! not yet part of the stable core.
//!
//! ## Warning
//!
//! ⚠️ **Unstable API**: Modules in this directory are subject to breaking changes or removal without notice.
//! Do not rely on them for production code.
//!
//! ## Contents
//!
//! *   [`raytracer`]: A simple CPU-based recursive raytracer (Whitted-style) used for validating scene correctness and generating reference images.
//! *   [`jelly`]: Experimental Soft-Body physics simulation (Mass-Spring system).
//! *   [`cloth`]: Cloth simulation using Verlet integration.
//! *   [`sdf`]: Signed Distance Field rendering experiments.
//! *   [`procedural_mesh`]: Procedural mesh generation (terrain, noise).
//!
//! ## Feature Flags
//!
//! To enable these modules, you must enable the `nova` feature in your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! abrash = { version = "0.1", features = ["nova"] }
//! ```

pub mod arboretum;
pub mod cloth;
pub mod crepuscular;
pub mod crosshatch;
pub mod crt;
pub mod dither;
pub mod glitch;
pub mod halftone;
pub mod isosurface;
pub mod jelly;
pub mod kuwahara;
pub mod pixel_sort;
pub mod pixelate;
pub mod procedural_mesh;
pub mod radial_blur;
pub mod raytracer;
pub mod sdf;
pub mod sharpen;
pub mod ssao;
pub mod vision;
pub mod volume;
pub mod voxel_explosion;
pub mod voxelizer;
