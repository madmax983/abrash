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

pub mod anaglyph;
pub mod arboretum;
pub mod ascii_display;
pub mod autostereogram;
pub mod black_hole;
pub mod boids;
pub mod brickify;
pub mod fractal;

pub mod blueprint;
pub mod cel_shade;
pub mod chroma_key;
pub mod cloth;
pub mod color_blindness;
pub mod color_splash;
pub mod conway;
pub mod crepuscular;
pub mod crosshatch;
pub mod cross_stitch;
pub mod crt;
pub mod depth_fog;
pub mod digital_rain;
pub mod directional_blur;
pub mod dither;
pub mod duotone;
pub mod edge_glow;
pub mod emboss;
pub mod falling_sand;
pub mod fire;
pub mod fisheye;
pub mod frosted_glass;
pub mod glitch;
pub mod halftone;
pub mod harmonograph;
pub mod hologram;
pub mod isosurface;
pub mod jelly;
pub mod kaleidoscope;
pub mod kuwahara;
pub mod led_matrix;
pub mod lsystem;
pub mod mandelbrot;
pub mod melt;
pub mod mode7;
pub mod modifiers;
pub mod neon_outline;
pub mod night_vision;
pub mod palette;
pub mod pencil_sketch;
pub mod pixel_sort;
pub mod pixelate;
pub mod plasma;
pub mod pop_art;
pub mod posterize;
pub mod procedural_mesh;
pub mod radial_blur;
pub mod raytracer;
pub mod reaction_diffusion;
pub mod sdf;
pub mod selective_color;
pub mod sharpen;
pub mod slitscan;
pub mod sonar;
pub mod speed_lines;
pub mod starfield;
pub mod steganography;
pub mod swirl;
pub mod thermal;
pub mod tilt_shift;
pub mod topography;
pub mod vhs;
pub mod vision;
pub mod volume;
pub mod voronoi;
pub mod voxel_explosion;
pub mod voxelizer;
pub mod water_ripple;
pub mod wobble;
