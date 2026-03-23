//! Skeletal animation system for the Abrash rendering engine.
//!
//! Provides bone hierarchies, CPU vertex skinning, animation clip evaluation
//! via `abrash-anim`, and optional glTF loading.

pub mod pose;
pub mod skeleton;

pub use pose::{Pose, SkinMatrices};
pub use skeleton::{Joint, JointId, Skeleton};
