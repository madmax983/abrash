#![allow(dead_code)]
#![allow(clippy::suboptimal_flops)]
#![allow(clippy::should_panic_without_expect)]
#![allow(clippy::ignore_without_reason)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::float_cmp)]
#[allow(clippy::wildcard_imports)]
use super::*;
use std::mem::MaybeUninit;
use std::ops::{Add, Mul, Sub};

/// A 3D point that has been projected into 2D screen coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(missing_docs)]
pub struct ScreenPoint {
    pub x: i32,
    pub y: i32,
    pub z: f32,
    /// Reciprocal of the Homogeneous W coordinate ($1/w$).
    ///
    /// This value is critical for perspective-correct texture mapping and attribute interpolation.
    /// By storing $1/w$, the rasterizer can interpolate attributes in screen space linearly
    /// (e.g., $u/w$, $v/w$) and then recover the true perspective-correct value per pixel
    /// by dividing by the interpolated $1/w$.
    ///
    /// Storing this avoids recomputing the division during triangle setup,
    /// saving ~10-20 CPU cycles per vertex.
    pub inv_w: f32,
}
