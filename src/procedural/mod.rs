//! Procedural Generation Module
//!
//! This module contains tools for generating content algorithmically,
//! including textures, meshes (L-Systems), and terrain.

pub mod l_system;
pub mod terrain;
pub mod textures;

// Re-export common types
pub use l_system::LSystem;
pub use terrain::TerrainGenerator;
pub use textures::*;
