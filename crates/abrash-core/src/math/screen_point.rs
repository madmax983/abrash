#[allow(clippy::wildcard_imports)]
use super::*;
use std::mem::MaybeUninit;
use std::ops::{Add, Mul, Sub};

/// A 3D point that has been projected into 2D screen coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]

/// A 2D point on the screen with an associated depth value and $1/w$ coordinate.
///
/// # Examples
///
/// ```
/// use abrash_core::math::ScreenPoint;
/// let p = ScreenPoint { x: 0, y: 0, z: 0.0, inv_w: 1.0 };
/// assert_eq!(p.x, 0);
/// ```
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
