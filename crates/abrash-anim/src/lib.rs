#![allow(missing_docs)]
#![allow(clippy::missing_docs_in_private_items)]
//! Composable animation system for the Abrash rendering engine.
//!
//! Provides phase-based animation evaluation inspired by Arthropod's `anim-graph`.
//! This crate is rendering-agnostic — it operates on any type implementing
//! `abrash_core::Animatable`.

pub mod clock;
pub mod easing;
pub mod evaluable;
pub mod hold;
pub mod keyframe;
pub mod sequence;
pub mod timeline;

pub use clock::{AnimationClock, ClockEvent, PlaybackMode};
pub use easing::Easing;
pub use evaluable::{Evaluable, Sample};
pub use hold::Hold;
pub use keyframe::Keyframe;
pub use sequence::Sequence;
pub use timeline::Timeline;
