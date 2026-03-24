//! Skeletal animation system for the Abrash rendering engine.
//!
//! Provides bone hierarchies, CPU vertex skinning, animation clip evaluation
//! via `abrash-anim`, and optional glTF loading.

pub mod clip;
pub mod pose;
pub mod skeleton;
pub mod skin;
pub mod skinning;

pub use clip::{AnimationChannel, AnimationClip, ChannelTarget, ChannelValues};
pub use pose::{Pose, SkinMatrices};
pub use skeleton::{Joint, JointId, Skeleton};
pub use skin::{SkinData, SkinnedMesh};
pub use skinning::skin_vertices;
