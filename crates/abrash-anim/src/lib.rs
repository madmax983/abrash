//! Composable animation system for the Abrash rendering engine.
//!
//! Provides phase-based animation evaluation inspired by Arthropod's `anim-graph`.
//! This crate is rendering-agnostic — it operates on any type implementing
//! `abrash_core::Animatable`.

pub mod clock;
pub mod easing;
pub mod evaluable;

pub use clock::{AnimationClock, ClockEvent, PlaybackMode};
pub use easing::Easing;
pub use evaluable::{Evaluable, Sample};
