//! Core types for raycasting results.

use abrash_core::fixed16_16::Fixed16_16;

/// Which face of a grid cell was hit by a ray.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    /// Facing positive Y (upwards on the map).
    North,
    /// Facing negative Y (downwards on the map).
    South,
    /// Facing positive X (rightwards on the map).
    East,
    /// Facing negative X (leftwards on the map).
    West,
}

/// Contents of a grid cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cell {
    /// Passable empty space.
    Empty,
    /// Solid wall with a texture ID.
    Solid(u16),
    /// Portal to another sector/zone with an ID.
    Portal(u16),
}

impl Cell {
    /// Returns `true` if this cell blocks ray traversal.
    #[inline]
    #[must_use]
    pub const fn is_solid(&self) -> bool {
        matches!(self, Self::Solid(_))
    }
}

/// A 2D vector in 16.16 fixed-point.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Vec2Fixed {
    /// The X-axis location using fixed-point precision.
    pub x: Fixed16_16,
    /// The Y-axis location using fixed-point precision.
    pub y: Fixed16_16,
}

impl Vec2Fixed {
    /// The zero vector.
    pub const ZERO: Self = Self {
        x: Fixed16_16::ZERO,
        y: Fixed16_16::ZERO,
    };

    /// Create from two `Fixed16_16` values.
    #[inline]
    #[must_use]
    pub const fn new(x: Fixed16_16, y: Fixed16_16) -> Self {
        Self { x, y }
    }

    /// Create from integer coordinates.
    #[inline]
    #[must_use]
    pub const fn from_ints(x: i32, y: i32) -> Self {
        Self {
            x: Fixed16_16::from_int(x),
            y: Fixed16_16::from_int(y),
        }
    }

    /// Create from `f32` coordinates.
    #[inline]
    #[must_use]
    pub fn from_f32(x: f32, y: f32) -> Self {
        Self {
            x: Fixed16_16::from_f32(x),
            y: Fixed16_16::from_f32(y),
        }
    }
}

/// Result of a ray hitting a grid cell wall.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RayHit {
    /// Perpendicular distance to the hit point (avoids fisheye).
    pub distance: Fixed16_16,
    /// X coordinate of the cell that was hit.
    pub cell_x: u32,
    /// Y coordinate of the cell that was hit.
    pub cell_y: u32,
    /// Which face of the cell was hit.
    pub side: Side,
}

/// Extended hit information including exact intersection point and texture coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DetailedHit {
    /// The base ray hit result.
    pub hit: RayHit,
    /// Exact world-space intersection point.
    pub point: Vec2Fixed,
    /// Outward-facing normal of the hit surface.
    pub normal: Vec2Fixed,
    /// Texture U coordinate along the hit wall face (0..1 in fixed-point).
    pub texture_u: Fixed16_16,
}
