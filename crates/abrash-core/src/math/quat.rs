//! Canonical [`Quat`] re-exported into the `math` namespace.
//!
//! This module used to carry its own `Quat` implementation, duplicating
//! [`crate::quat::Quat`]. There is now exactly one quaternion type; both
//! `abrash_core::quat::Quat` and `abrash_core::math::Quat` name it.
//!
//! [`Quat`]: crate::quat::Quat

pub use crate::quat::Quat;
