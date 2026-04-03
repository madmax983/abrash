//! Integer-coordinate 2D rectangle for screen-space operations.
//!
//! [`IRect`] represents an axis-aligned rectangle with `i32` coordinates.
//! The convention is **half-open**: `[min, max)` — `min` is inclusive, `max`
//! is exclusive.  This matches array indexing, pixel loops, and the blitter's
//! clip conventions throughout the engine.
//!
//! # Examples
//!
//! ```
//! use abrash_core::irect::IRect;
//! use abrash_core::ivec::IVec2;
//!
//! // Clipping a sprite destination rect to the screen bounds
//! let screen = IRect::from_size(320, 240);
//! let sprite = IRect::new(IVec2::new(-10, 100), IVec2::new(54, 164));
//! let clipped = screen.intersect(sprite).unwrap_or(IRect::ZERO);
//! assert_eq!(clipped.min.x, 0);
//! assert_eq!(clipped.min.y, 100);
//! ```

use crate::ivec::IVec2;

/// Axis-aligned integer rectangle using **half-open** `[min, max)` convention.
///
/// - `min` — top-left corner, **inclusive**.
/// - `max` — bottom-right corner, **exclusive**.
///
/// An empty rect satisfies `min.x >= max.x || min.y >= max.y`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct IRect {
    pub min: IVec2,
    pub max: IVec2,
}

impl IRect {
    /// Empty rectangle at the origin.
    pub const ZERO: Self = Self {
        min: IVec2::ZERO,
        max: IVec2::ZERO,
    };

    /// Construct from `min` (inclusive) and `max` (exclusive).
    #[must_use]
    #[inline]
    pub const fn new(min: IVec2, max: IVec2) -> Self {
        Self { min, max }
    }

    /// Construct from top-left corner and size.
    ///
    /// ```
    /// use abrash_core::irect::IRect;
    /// use abrash_core::ivec::IVec2;
    /// let r = IRect::from_pos_size(IVec2::new(10, 20), 30, 40);
    /// assert_eq!(r.max, IVec2::new(40, 60));
    /// ```
    #[must_use]
    #[inline]
    pub const fn from_pos_size(pos: IVec2, w: i32, h: i32) -> Self {
        Self::new(pos, IVec2::new(pos.x + w, pos.y + h))
    }

    /// Construct from origin `(0, 0)` with the given size.
    #[must_use]
    #[inline]
    pub const fn from_size(w: i32, h: i32) -> Self {
        Self::new(IVec2::ZERO, IVec2::new(w, h))
    }

    /// Construct as the smallest rect that contains both `a` and `b` (two points).
    #[must_use]
    #[inline]
    pub const fn from_points(a: IVec2, b: IVec2) -> Self {
        Self::new(a.min(b), a.max(b))
    }

    // ── Dimensions ────────────────────────────────────────────────────────────

    /// Width of the rectangle (may be zero or negative for empty/invalid rects).
    #[must_use]
    #[inline]
    pub const fn width(self) -> i32 {
        self.max.x - self.min.x
    }

    /// Height of the rectangle.
    #[must_use]
    #[inline]
    pub const fn height(self) -> i32 {
        self.max.y - self.min.y
    }

    /// Size as `(width, height)`.
    #[must_use]
    #[inline]
    pub const fn size(self) -> IVec2 {
        IVec2::new(self.width(), self.height())
    }

    /// Area (in pixels). Returns 0 or negative for empty rects.
    #[must_use]
    #[inline]
    pub const fn area(self) -> i32 {
        self.width() * self.height()
    }

    /// Returns `true` if the rectangle has zero or negative size in either axis.
    #[must_use]
    #[inline]
    pub const fn is_empty(self) -> bool {
        self.max.x <= self.min.x || self.max.y <= self.min.y
    }

    // ── Position helpers ──────────────────────────────────────────────────────

    /// Center point (rounds toward zero for odd sizes).
    #[must_use]
    #[inline]
    pub const fn center(self) -> IVec2 {
        IVec2::new(
            i32::midpoint(self.min.x, self.max.x),
            i32::midpoint(self.min.y, self.max.y),
        )
    }

    /// Top-left corner (same as `min`).
    #[must_use]
    #[inline]
    pub const fn top_left(self) -> IVec2 {
        self.min
    }

    /// Top-right corner.
    #[must_use]
    #[inline]
    pub const fn top_right(self) -> IVec2 {
        IVec2::new(self.max.x, self.min.y)
    }

    /// Bottom-left corner.
    #[must_use]
    #[inline]
    pub const fn bottom_left(self) -> IVec2 {
        IVec2::new(self.min.x, self.max.y)
    }

    /// Bottom-right corner (same as `max`; exclusive — one pixel past the edge).
    #[must_use]
    #[inline]
    pub const fn bottom_right(self) -> IVec2 {
        self.max
    }

    // ── Set operations ────────────────────────────────────────────────────────

    /// Intersection of two rectangles.
    ///
    /// Returns `None` if the rects do not overlap.
    #[must_use]
    #[inline]
    pub const fn intersect(self, other: Self) -> Option<Self> {
        let result = Self::new(self.min.max(other.min), self.max.min(other.max));
        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    /// Smallest rectangle that contains both `self` and `other`.
    ///
    /// Ignores empty rects (an empty rect does not contribute to the union).
    #[must_use]
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }
        Self::new(self.min.min(other.min), self.max.max(other.max))
    }

    /// Expand by `amount` pixels on all sides.
    #[must_use]
    #[inline]
    pub const fn inflate(self, amount: i32) -> Self {
        Self::new(
            IVec2::new(self.min.x - amount, self.min.y - amount),
            IVec2::new(self.max.x + amount, self.max.y + amount),
        )
    }

    /// Shrink by `amount` pixels on all sides (may produce an empty rect).
    #[must_use]
    #[inline]
    pub const fn deflate(self, amount: i32) -> Self {
        self.inflate(-amount)
    }

    /// Translate by an offset.
    #[must_use]
    #[inline]
    pub const fn translate(self, offset: IVec2) -> Self {
        Self::new(
            IVec2::new(self.min.x + offset.x, self.min.y + offset.y),
            IVec2::new(self.max.x + offset.x, self.max.y + offset.y),
        )
    }

    // ── Containment ───────────────────────────────────────────────────────────

    /// Returns `true` if `point` is inside `[min, max)`.
    #[must_use]
    #[inline]
    pub const fn contains_point(self, point: IVec2) -> bool {
        point.x >= self.min.x
            && point.y >= self.min.y
            && point.x < self.max.x
            && point.y < self.max.y
    }

    /// Returns `true` if `other` is fully inside `self`.
    #[must_use]
    #[inline]
    pub const fn contains_rect(self, other: Self) -> bool {
        other.min.x >= self.min.x
            && other.min.y >= self.min.y
            && other.max.x <= self.max.x
            && other.max.y <= self.max.y
    }

    /// Returns `true` if `self` and `other` overlap (share at least one pixel).
    #[must_use]
    #[inline]
    pub const fn overlaps(self, other: Self) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
    }

    // ── Clamp a point ─────────────────────────────────────────────────────────

    /// Clamp `point` to lie within `[min, max)`.
    ///
    /// The result is always inside the rect (assuming `!is_empty()`).
    #[must_use]
    #[inline]
    pub const fn clamp_point(self, point: IVec2) -> IVec2 {
        IVec2::new(
            if point.x < self.min.x {
                self.min.x
            } else if point.x >= self.max.x {
                self.max.x - 1
            } else {
                point.x
            },
            if point.y < self.min.y {
                self.min.y
            } else if point.y >= self.max.y {
                self.max.y - 1
            } else {
                point.y
            },
        )
    }

    // ── Split ─────────────────────────────────────────────────────────────────

    /// Split horizontally at x = `x` (exclusive).
    ///
    /// Returns `(left, right)` where `left` covers `[min.x, x)` and `right` covers `[x, max.x)`.
    #[must_use]
    #[inline]
    pub const fn split_x(self, x: i32) -> (Self, Self) {
        let left = Self::new(self.min, IVec2::new(x, self.max.y));
        let right = Self::new(IVec2::new(x, self.min.y), self.max);
        (left, right)
    }

    /// Split vertically at y = `y` (exclusive).
    ///
    /// Returns `(top, bottom)`.
    #[must_use]
    #[inline]
    pub const fn split_y(self, y: i32) -> (Self, Self) {
        let top = Self::new(self.min, IVec2::new(self.max.x, y));
        let bottom = Self::new(IVec2::new(self.min.x, y), self.max);
        (top, bottom)
    }
}

impl std::fmt::Display for IRect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[({}, {})..({}, {}))",
            self.min.x, self.min.y, self.max.x, self.max.y
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction() {
        let r = IRect::from_size(320, 240);
        assert_eq!(r.width(), 320);
        assert_eq!(r.height(), 240);
        assert_eq!(r.area(), 76800);
        assert!(!r.is_empty());
    }

    #[test]
    fn from_pos_size() {
        let r = IRect::from_pos_size(IVec2::new(10, 20), 30, 40);
        assert_eq!(r.min, IVec2::new(10, 20));
        assert_eq!(r.max, IVec2::new(40, 60));
    }

    #[test]
    fn empty_rect() {
        let r = IRect::from_size(0, 10);
        assert!(r.is_empty());
        let r2 = IRect::from_size(-1, 10);
        assert!(r2.is_empty());
    }

    #[test]
    fn intersect_overlap() {
        let a = IRect::from_size(10, 10);
        let b = IRect::from_pos_size(IVec2::new(5, 5), 10, 10);
        let clip = a.intersect(b).expect("should overlap");
        assert_eq!(clip.min, IVec2::new(5, 5));
        assert_eq!(clip.max, IVec2::new(10, 10));
    }

    #[test]
    fn intersect_disjoint() {
        let a = IRect::from_size(10, 10);
        let b = IRect::from_pos_size(IVec2::new(20, 0), 5, 5);
        assert!(a.intersect(b).is_none());
    }

    #[test]
    fn intersect_touching_edge_is_disjoint() {
        // Half-open: [0,10) and [10,20) share no pixels.
        let a = IRect::from_size(10, 10);
        let b = IRect::from_pos_size(IVec2::new(10, 0), 10, 10);
        assert!(a.intersect(b).is_none());
    }

    #[test]
    fn union_disjoint() {
        let a = IRect::from_size(5, 5);
        let b = IRect::from_pos_size(IVec2::new(10, 10), 5, 5);
        let u = a.union(b);
        assert_eq!(u.min, IVec2::ZERO);
        assert_eq!(u.max, IVec2::new(15, 15));
    }

    #[test]
    fn union_with_empty() {
        let a = IRect::from_size(5, 5);
        let empty = IRect::ZERO;
        assert_eq!(a.union(empty), a);
        assert_eq!(empty.union(a), a);
    }

    #[test]
    fn inflate_deflate() {
        let r = IRect::from_pos_size(IVec2::new(5, 5), 10, 10);
        let big = r.inflate(2);
        assert_eq!(big.min, IVec2::new(3, 3));
        assert_eq!(big.max, IVec2::new(17, 17));
        assert_eq!(big.deflate(2), r);
    }

    #[test]
    fn translate() {
        let r = IRect::from_size(10, 10);
        let moved = r.translate(IVec2::new(5, -3));
        assert_eq!(moved.min, IVec2::new(5, -3));
        assert_eq!(moved.max, IVec2::new(15, 7));
    }

    #[test]
    fn contains_point() {
        let r = IRect::from_size(10, 10);
        assert!(r.contains_point(IVec2::ZERO));
        assert!(r.contains_point(IVec2::new(9, 9)));
        assert!(!r.contains_point(IVec2::new(10, 0))); // exclusive
        assert!(!r.contains_point(IVec2::new(-1, 0)));
    }

    #[test]
    fn contains_rect() {
        let outer = IRect::from_size(20, 20);
        let inner = IRect::from_pos_size(IVec2::new(5, 5), 5, 5);
        assert!(outer.contains_rect(inner));
        assert!(!inner.contains_rect(outer));
    }

    #[test]
    fn overlaps() {
        let a = IRect::from_size(10, 10);
        let b = IRect::from_pos_size(IVec2::new(5, 5), 10, 10);
        let c = IRect::from_pos_size(IVec2::new(20, 20), 5, 5);
        assert!(a.overlaps(b));
        assert!(!a.overlaps(c));
    }

    #[test]
    fn clamp_point() {
        let r = IRect::from_size(10, 10);
        assert_eq!(r.clamp_point(IVec2::new(5, 5)), IVec2::new(5, 5));
        assert_eq!(r.clamp_point(IVec2::new(-5, 5)), IVec2::new(0, 5));
        assert_eq!(r.clamp_point(IVec2::new(15, 5)), IVec2::new(9, 5));
    }

    #[test]
    fn split_x_and_y() {
        let r = IRect::from_size(10, 10);
        let (left, right) = r.split_x(4);
        assert_eq!(left.width(), 4);
        assert_eq!(right.width(), 6);
        let (top, bottom) = r.split_y(6);
        assert_eq!(top.height(), 6);
        assert_eq!(bottom.height(), 4);
    }

    #[test]
    fn center() {
        let r = IRect::from_size(10, 10);
        assert_eq!(r.center(), IVec2::new(5, 5));
    }

    #[test]
    fn display() {
        let r = IRect::from_size(320, 240);
        assert_eq!(r.to_string(), "[(0, 0)..(320, 240))");
    }
}
