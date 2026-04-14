//! Parametric curve evaluation: Bezier, Catmull-Rom, and Hermite splines.
//!
//! All curves are parameterised by `t ∈ [0.0, 1.0]`:
//!
//! * **Quadratic Bezier** — 3 control points, one hump.
//! * **Cubic Bezier** — 4 control points, industry standard for paths and easing.
//! * **Catmull-Rom** — Interpolating spline through a sequence of points; tangents
//!   computed automatically from neighbours. Convenient for smooth paths.
//! * **Cubic Hermite** — Explicit endpoint values and tangents, used in animation.
//!
//!
//! # Examples
//!
//! ```
//! use abrash_core::curve::{CubicBezier, CatmullRom};
//! use abrash_core::math::Vec3;
//!
//! // Cubic Bezier path
//! let bez = CubicBezier::new(
//!     Vec3::new(0.0, 0.0, 0.0),
//!     Vec3::new(1.0, 2.0, 0.0),
//!     Vec3::new(2.0, 2.0, 0.0),
//!     Vec3::new(3.0, 0.0, 0.0),
//! );
//! let mid = bez.evaluate(0.5);
//! assert!((mid.x - 1.5).abs() < 0.01);
//!
//! // Catmull-Rom through waypoints
//! let spline = CatmullRom::new(vec![
//!     Vec3::new(0.0, 0.0, 0.0),
//!     Vec3::new(1.0, 1.0, 0.0),
//!     Vec3::new(2.0, 0.0, 0.0),
//!     Vec3::new(3.0, 1.0, 0.0),
//! ]);
//! let p = spline.evaluate(0.5); // midpoint of entire spline
//! assert!((p.x - 1.5).abs() < 0.1);
//! ```

use crate::math::{
    Vec3, bezier_cubic, bezier_cubic_tangent, bezier_quadratic, catmull_rom, hermite,
};

// ── Lerpable trait ────────────────────────────────────────────────────────────
// ── CubicBezier struct ────────────────────────────────────────────────────────

/// A cubic Bezier curve defined by 4 control points.
///
/// The curve passes through `p0` (at t=0) and `p3` (at t=1); `p1` and `p2`
/// are attraction points that shape the arc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CubicBezier {
    /// Start point (curve passes through this).
    pub p0: Vec3,
    /// First control point (influences departure tangent).
    pub p1: Vec3,
    /// Second control point (influences arrival tangent).
    pub p2: Vec3,
    /// End point (curve passes through this).
    pub p3: Vec3,
}

impl CubicBezier {
    /// Create a new cubic Bezier curve.
    #[must_use]
    pub const fn new(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3) -> Self {
        Self { p0, p1, p2, p3 }
    }

    /// Evaluate the curve at parameter `t ∈ [0, 1]` (de Casteljau).
    #[must_use]
    #[inline]
    pub fn evaluate(&self, t: f32) -> Vec3 {
        bezier_cubic(self.p0, self.p1, self.p2, self.p3, t)
    }

    /// Tangent direction at `t` (not normalized).
    #[must_use]
    #[inline]
    pub fn tangent(&self, t: f32) -> Vec3 {
        bezier_cubic_tangent(self.p0, self.p1, self.p2, self.p3, t)
    }

    /// Approximate arc length using `n` linear segments (higher = more accurate).
    ///
    /// Uses simple polyline arc length — sufficient for most game use cases.
    #[must_use]
    pub fn arc_length(&self, n: u32) -> f32 {
        let mut length = 0.0f32;
        let mut prev = self.p0;
        for i in 1..=n {
            let t = i as f32 / n as f32;
            let p = self.evaluate(t);
            let d = p - prev;
            length += (d.x * d.x + d.y * d.y + d.z * d.z).sqrt();
            prev = p;
        }
        length
    }

    /// Split the curve at `t` into two sub-curves (de Casteljau subdivision).
    #[must_use]
    pub fn split(&self, t: f32) -> (Self, Self) {
        let q0 = self.p0.lerp(self.p1, t);
        let q1 = self.p1.lerp(self.p2, t);
        let q2 = self.p2.lerp(self.p3, t);
        let r0 = q0.lerp(q1, t);
        let r1 = q1.lerp(q2, t);
        let s = r0.lerp(r1, t);
        (Self::new(self.p0, q0, r0, s), Self::new(s, r1, q2, self.p3))
    }

    /// Bounding box of the curve (conservative axis-aligned).
    ///
    /// Samples `n` points to approximate the AABB. Use `n ≥ 16` for close bounds.
    #[must_use]
    pub fn aabb_approx(&self, n: u32) -> (Vec3, Vec3) {
        let mut min = self.p0;
        let mut max = self.p0;
        for i in 1..=n {
            let t = i as f32 / n as f32;
            let p = self.evaluate(t);
            min = Vec3::new(min.x.min(p.x), min.y.min(p.y), min.z.min(p.z));
            max = Vec3::new(max.x.max(p.x), max.y.max(p.y), max.z.max(p.z));
        }
        (min, max)
    }
}

// ── CatmullRom spline ─────────────────────────────────────────────────────────

/// A Catmull-Rom spline passing through a sequence of control points.
///
/// The spline interpolates (passes through) every control point. Tangents are
/// computed automatically from neighbours, so you only specify the waypoints.
///
/// Requires at least 2 points. For a smooth loop, repeat the first point at
/// the end of the list.
#[derive(Debug, Clone, PartialEq)]
pub struct CatmullRom {
    points: Vec<Vec3>,
}

impl CatmullRom {
    /// Create a new Catmull-Rom spline from a list of control points.
    ///
    /// # Panics
    ///
    /// Panics if fewer than 2 points are provided.
    #[must_use]
    pub fn new(points: Vec<Vec3>) -> Self {
        assert!(points.len() >= 2, "CatmullRom requires at least 2 points");
        Self { points }
    }

    /// Number of control points.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.points.len()
    }

    /// Returns `true` if the spline has no points.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Number of segments (= `len() - 1`).
    #[must_use]
    pub const fn segment_count(&self) -> usize {
        self.points.len().saturating_sub(1)
    }

    /// Evaluate the spline at global parameter `t ∈ [0.0, 1.0]`.
    ///
    /// `t = 0` returns the first point, `t = 1` returns the last.
    #[must_use]
    pub fn evaluate(&self, t: f32) -> Vec3 {
        let n = self.points.len();
        if n == 1 {
            return self.points[0];
        }

        let scaled = t.max(0.0).min(1.0) * (n - 1) as f32;
        let seg = (scaled as usize).min(n - 2);
        let local_t = scaled - seg as f32;

        let i0 = seg.saturating_sub(1);
        let i1 = seg;
        let i2 = (seg + 1).min(n - 1);
        let i3 = (seg + 2).min(n - 1);

        let p0 = self.points[i0];
        let p1 = self.points[i1];
        let p2 = self.points[i2];
        let p3 = self.points[i3];

        Self::evaluate_segment(p0, p1, p2, p3, local_t)
    }

    /// Evaluate a specific segment by index `seg ∈ [0, segment_count())` at `t ∈ [0, 1]`.
    #[must_use]
    pub fn evaluate_at_segment(&self, seg: usize, t: f32) -> Vec3 {
        let n = self.points.len();
        let i0 = seg.saturating_sub(1);
        let i1 = seg;
        let i2 = (seg + 1).min(n - 1);
        let i3 = (seg + 2).min(n - 1);
        Self::evaluate_segment(
            self.points[i0],
            self.points[i1],
            self.points[i2],
            self.points[i3],
            t,
        )
    }

    /// Sample the spline uniformly into `n` points (including endpoints).
    #[must_use]
    pub fn sample(&self, n: usize) -> Vec<Vec3> {
        (0..n)
            .map(|i| self.evaluate(i as f32 / (n - 1).max(1) as f32))
            .collect()
    }

    /// Approximate total arc length with `samples` linear segments.
    #[must_use]
    pub fn arc_length(&self, samples: u32) -> f32 {
        let mut length = 0.0f32;
        let mut prev = self.evaluate(0.0);
        for i in 1..=samples {
            let t = i as f32 / samples as f32;
            let p = self.evaluate(t);
            let d = p - prev;
            length += (d.x * d.x + d.y * d.y + d.z * d.z).sqrt();
            prev = p;
        }
        length
    }

    fn evaluate_segment(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
        let t2 = t * t;
        let t3 = t2 * t;
        let c0 = -0.5 * t3 + t2 - 0.5 * t;
        let c1 = 1.5 * t3 - 2.5 * t2 + 1.0;
        let c2 = -1.5 * t3 + 2.0 * t2 + 0.5 * t;
        let c3 = 0.5 * t3 - 0.5 * t2;
        Vec3::new(
            c0 * p0.x + c1 * p1.x + c2 * p2.x + c3 * p3.x,
            c0 * p0.y + c1 * p1.y + c2 * p2.y + c3 * p3.y,
            c0 * p0.z + c1 * p1.z + c2 * p2.z + c3 * p3.z,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOL: f32 = 1e-4;

    fn v(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3::new(x, y, z)
    }

    // ── Bezier quadratic ──────────────────────────────────────────────────────

    #[test]
    fn bezier_quad_endpoints() {
        let p = bezier_quadratic(v(0.0, 0.0, 0.0), v(1.0, 1.0, 0.0), v(2.0, 0.0, 0.0), 0.0);
        assert!((p.x).abs() < TOL && (p.y).abs() < TOL);
        let p1 = bezier_quadratic(v(0.0, 0.0, 0.0), v(1.0, 1.0, 0.0), v(2.0, 0.0, 0.0), 1.0);
        assert!((p1.x - 2.0).abs() < TOL && (p1.y).abs() < TOL);
    }


    // ── Bezier cubic ──────────────────────────────────────────────────────────

    #[test]
    fn bezier_cubic_endpoints() {
        let p0 = v(0.0, 0.0, 0.0);
        let p3 = v(3.0, 0.0, 0.0);
        let start = bezier_cubic(p0, v(1.0, 1.0, 0.0), v(2.0, 1.0, 0.0), p3, 0.0);
        let end = bezier_cubic(p0, v(1.0, 1.0, 0.0), v(2.0, 1.0, 0.0), p3, 1.0);
        assert!((start.x - p0.x).abs() < TOL);
        assert!((end.x - p3.x).abs() < TOL);
    }

    #[test]
    fn bezier_cubic_symmetric_midpoint() {
        // S-curve: symmetric, midpoint should be at x=1.5 y=1.0
        let mid = bezier_cubic(
            v(0.0, 0.0, 0.0),
            v(0.0, 2.0, 0.0),
            v(3.0, 2.0, 0.0),
            v(3.0, 0.0, 0.0),
            0.5,
        );
        assert!((mid.x - 1.5).abs() < TOL, "x={}", mid.x);
        assert!((mid.y - 1.5).abs() < TOL, "y={}", mid.y);
    }

    #[test]
    fn bezier_cubic_split_rejoins() {
        let bez = CubicBezier::new(
            v(0.0, 0.0, 0.0),
            v(1.0, 2.0, 0.0),
            v(2.0, 2.0, 0.0),
            v(3.0, 0.0, 0.0),
        );
        let (left, right) = bez.split(0.5);
        // The split point should be the same from both halves
        let mid_orig = bez.evaluate(0.5);
        let mid_left = left.evaluate(1.0);
        let mid_right = right.evaluate(0.0);
        assert!((mid_left.x - mid_orig.x).abs() < TOL);
        assert!((mid_right.x - mid_orig.x).abs() < TOL);
    }

    #[test]
    fn bezier_arc_length_is_positive() {
        let bez = CubicBezier::new(
            v(0.0, 0.0, 0.0),
            v(1.0, 2.0, 0.0),
            v(2.0, 2.0, 0.0),
            v(3.0, 0.0, 0.0),
        );
        let len = bez.arc_length(64);
        // Chord length = 3.0, curve is longer
        assert!(len > 3.0, "arc_length={len}");
        assert!(len < 6.0, "arc_length unreasonably long={len}");
    }

    // ── Hermite ───────────────────────────────────────────────────────────────

    #[test]
    fn hermite_endpoints() {
        let p0 = v(0.0, 0.0, 0.0);
        let p1 = v(1.0, 1.0, 0.0);
        let m0 = v(1.0, 0.0, 0.0);
        let m1 = v(1.0, 0.0, 0.0);
        let start = hermite(p0, m0, p1, m1, 0.0);
        let end = hermite(p0, m0, p1, m1, 1.0);
        assert!((start.x - p0.x).abs() < TOL);
        assert!((end.x - p1.x).abs() < TOL && (end.y - p1.y).abs() < TOL);
    }

    // ── Catmull-Rom ───────────────────────────────────────────────────────────

    #[test]
    fn catmull_rom_passes_through_first_and_last() {
        let spline = CatmullRom::new(vec![
            v(0.0, 0.0, 0.0),
            v(1.0, 1.0, 0.0),
            v(2.0, 0.0, 0.0),
            v(3.0, 1.0, 0.0),
        ]);
        let start = spline.evaluate(0.0);
        let end = spline.evaluate(1.0);
        // First point: p0 = (0,0,0), Last: p3 = (3,1,0)
        assert!((start.x - 0.0).abs() < 0.01, "start.x={}", start.x);
        assert!((end.x - 3.0).abs() < 0.01, "end.x={}", end.x);
    }

    #[test]
    fn catmull_rom_midpoint_between_interior_points() {
        let spline = CatmullRom::new(vec![
            v(0.0, 0.0, 0.0),
            v(1.0, 0.0, 0.0),
            v(2.0, 0.0, 0.0),
            v(3.0, 0.0, 0.0),
        ]);
        // Collinear points — midpoint should be exactly at x=1.5
        let mid = spline.evaluate(0.5);
        assert!((mid.x - 1.5).abs() < 0.01, "mid.x={}", mid.x);
    }

    #[test]
    fn catmull_rom_count() {
        let s = CatmullRom::new(vec![v(0.0, 0.0, 0.0), v(1.0, 0.0, 0.0), v(2.0, 0.0, 0.0)]);
        assert_eq!(s.segment_count(), 2);
    }

    #[test]
    fn catmull_rom_sample_count() {
        let s = CatmullRom::new(vec![v(0.0, 0.0, 0.0), v(1.0, 1.0, 0.0), v(2.0, 0.0, 0.0)]);
        let samples = s.sample(10);
        assert_eq!(samples.len(), 10);
        // First and last samples should be near control points
        assert!((samples[0].x).abs() < 0.01);
        assert!((samples[9].x - 2.0).abs() < 0.01);
    }

    #[test]
    fn catmull_rom_arc_length_positive() {
        let s = CatmullRom::new(vec![
            v(0.0, 0.0, 0.0),
            v(1.0, 1.0, 0.0),
            v(2.0, 0.0, 0.0),
            v(3.0, 1.0, 0.0),
        ]);
        let len = s.arc_length(64);
        assert!(len > 3.0, "arc_length={len}");
    }

    #[test]
    fn catmull_rom_straight_line_arc_length() {
        let s = CatmullRom::new(vec![
            v(0.0, 0.0, 0.0),
            v(1.0, 0.0, 0.0),
            v(2.0, 0.0, 0.0),
            v(3.0, 0.0, 0.0),
        ]);
        let len = s.arc_length(128);
        // Straight line through x=0..3 should have arc length ~3.0
        assert!((len - 3.0).abs() < 0.05, "arc_length={len}");
    }

    #[test]
    fn bezier_tangent_at_start_points_toward_p1() {
        let bez = CubicBezier::new(
            v(0.0, 0.0, 0.0),
            v(1.0, 0.0, 0.0),
            v(2.0, 0.0, 0.0),
            v(3.0, 0.0, 0.0),
        );
        let tan = bez.tangent(0.0);
        // Tangent at t=0 should point in +X direction
        assert!(tan.x > 0.0, "tangent.x={}", tan.x);
        assert!(tan.y.abs() < TOL, "tangent.y={}", tan.y);
    }
}
