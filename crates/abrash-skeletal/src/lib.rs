//! Skeletal animation system for the Abrash rendering engine.
//!
//! Provides bone hierarchies, CPU vertex skinning, animation clip evaluation
//! via `abrash-anim`, and optional glTF loading.

pub mod animator;
pub mod clip;
pub mod clip_evaluable;
pub mod pose;
pub mod skeleton;
pub mod skin;
pub mod skinning;

pub use animator::{BoneAnimator, SkeletonAnimator};
pub use clip::{AnimationChannel, AnimationClip, ChannelValues};
pub use pose::{Pose, SkinMatrices};
pub use skeleton::{Joint, JointId, Skeleton};
pub use skin::{SkinData, SkinnedMesh};
pub use skinning::skin_vertices;

#[cfg(feature = "gltf")]
pub mod gltf_loader;

#[cfg(feature = "gltf")]
pub use gltf_loader::{GltfError, GltfMaterial, GltfScene, load_gltf};
