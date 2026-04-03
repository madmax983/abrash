//! Signed distance functions (SDF) for 2D and 3D analytical geometry.
//!
//! Each function returns the **signed distance** from point `p` to the boundary
//! of a shape:
//! - **negative** → `p` is *inside* the shape
//! - **zero** → `p` is *on* the boundary
//! - **positive** → `p` is *outside* the shape
//!
//! SDFs compose naturally:
//!
//! ```text
//! union        = min(a, b)
//! intersection = max(a, b)
//! subtraction  = max(a, -b)
//! ```
//!
//! Use [`smooth_union`], [`smooth_intersection`], [`smooth_subtraction`] for
//! blended shapes without hard edges.
//!
//! # Reference
//!
//! Primitives follow Inigo Quilez's canonical formulations
//! (<https://iquilezles.org/articles/distfunctions2d>).
//!
//! # Examples
//!
//! ```
//! use abrash_core::sdf::{circle_2d, rect_2d, smooth_union};
//! use abrash_core::math::Vec2;
//!
//! // Distance from the origin to a unit circle centred at (2, 0)
//! let d = circle_2d(Vec2::ZERO, Vec2::new(2.0, 0.0), 1.0);
//! assert!((d - 1.0).abs() < 1e-5); // outside by exactly 1
//!
//! // Smooth union of two shapes
//! let a = circle_2d(Vec2::ZERO, Vec2::ZERO, 1.0);     // -1 (inside unit circle)
//! let b = circle_2d(Vec2::ZERO, Vec2::new(1.5, 0.0), 1.0); // -0.5
//! let blended = smooth_union(a, b, 0.5);
//! assert!(blended < 0.0); // merged blob — still inside
//! ```

use crate::math::{Vec2, Vec3};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Clamp `t` to `[0, 1]`.
#[inline]
const fn saturate(t: f32) -> f32 {
    if t < 0.0 {
        0.0
    } else if t > 1.0 {
        1.0
    } else {
        t
    }
}

/// Linearly interpolate between `a` and `b`.
#[inline]
fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

// ── 2D primitives ─────────────────────────────────────────────────────────────

/// Signed distance from `p` to a circle centred at `centre` with `radius`.
#[must_use]
#[inline]
pub fn circle_2d(p: Vec2, centre: Vec2, radius: f32) -> f32 {
    (p - centre).length() - radius
}

/// Signed distance from `p` to an axis-aligned rectangle centred at `centre`
/// with half-extents `half_size` (i.e. the rect spans `centre ± half_size`).
#[must_use]
#[inline]
pub fn rect_2d(p: Vec2, centre: Vec2, half_size: Vec2) -> f32 {
    let d = p - centre;
    let q = Vec2::new(d.x.abs(), d.y.abs()) - half_size;
    let outside = Vec2::new(q.x.max(0.0), q.y.max(0.0)).length();
    let inside = q.x.max(q.y).min(0.0);
    outside + inside
}

/// Signed distance from `p` to a rectangle with rounded corners.
///
/// `radius` is the corner rounding radius (clamped to `min(half_size)` internally).
#[must_use]
#[inline]
pub fn rounded_rect_2d(p: Vec2, centre: Vec2, half_size: Vec2, radius: f32) -> f32 {
    let r = radius.min(half_size.x.min(half_size.y));
    let shrunk = Vec2::new(half_size.x - r, half_size.y - r);
    rect_2d(p, centre, shrunk) - r
}

/// Signed distance from `p` to a line segment from `a` to `b`.
///
/// Always non-negative (the "inside" of a line has zero thickness).
#[must_use]
#[inline]
pub fn segment_2d(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let ap = p - a;
    let t = (ap.dot(ab) / ab.dot(ab)).clamp(0.0, 1.0);
    (ap - ab * t).length()
}

/// Signed distance from `p` to a capsule (line segment with hemispherical caps).
///
/// `radius` is the capsule radius.
#[must_use]
#[inline]
pub fn capsule_2d(p: Vec2, a: Vec2, b: Vec2, radius: f32) -> f32 {
    segment_2d(p, a, b) - radius
}

/// Signed distance from `p` to a ring (annulus) centred at `centre`.
///
/// `inner_radius` and `outer_radius` define the band.
#[must_use]
#[inline]
pub fn ring_2d(p: Vec2, centre: Vec2, inner_radius: f32, outer_radius: f32) -> f32 {
    let d = (p - centre).length();
    let mid = (inner_radius + outer_radius) * 0.5;
    let half_thickness = (outer_radius - inner_radius) * 0.5;
    (d - mid).abs() - half_thickness
}

/// Signed distance from `p` to a triangle with vertices `a`, `b`, `c`.
///
/// The winding order does not matter — the function tests all three edges.
#[must_use]
#[inline]
pub fn triangle_2d(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> f32 {
    let e0 = b - a;
    let e1 = c - b;
    let e2 = a - c;

    let v0 = p - a;
    let v1 = p - b;
    let v2 = p - c;

    // For each edge, project v onto the edge and compute the "signed" clamped distance.
    let pq0 = v0 - e0 * (v0.dot(e0) / e0.dot(e0)).clamp(0.0, 1.0);
    let pq1 = v1 - e1 * (v1.dot(e1) / e1.dot(e1)).clamp(0.0, 1.0);
    let pq2 = v2 - e2 * (v2.dot(e2) / e2.dot(e2)).clamp(0.0, 1.0);

    // Cross products for sign detection
    let s = (e0.x * e2.y - e0.y * e2.x).signum();

    let d0 = Vec2::new(pq0.dot(pq0), s * (v0.x * e0.y - v0.y * e0.x));
    let d1 = Vec2::new(pq1.dot(pq1), s * (v1.x * e1.y - v1.y * e1.x));
    let d2 = Vec2::new(pq2.dot(pq2), s * (v2.x * e2.y - v2.y * e2.x));

    // Component-wise min of the three edge contributions
    let d = Vec2::new(d0.x.min(d1.x).min(d2.x), d0.y.min(d1.y).min(d2.y));

    -d.x.sqrt() * d.y.signum()
}

/// Signed distance from `p` to a regular N-sided polygon centred at the origin.
///
/// `n` is the number of sides (≥ 3), `radius` is the circumradius.
///
/// # Panics
///
/// Panics if `n < 3`.
#[must_use]
pub fn regular_polygon_2d(p: Vec2, n: u32, radius: f32) -> f32 {
    use std::f32::consts::PI;
    assert!(n >= 3, "polygon must have at least 3 sides");
    let an = PI / n as f32;
    let acs = Vec2::new(an.cos(), an.sin());

    // Fold into fundamental sector
    let mut q = p;
    let angle = q.y.atan2(q.x);
    let sector = (angle / (2.0 * an)).floor() * (2.0 * an);
    let cos_s = sector.cos();
    let sin_s = sector.sin();
    q = Vec2::new(q.x * cos_s + q.y * sin_s, -q.x * sin_s + q.y * cos_s);
    q = Vec2::new(q.x.abs(), q.y);

    let w = q - acs * radius;
    let w_clamped = if w.x < 0.0 {
        Vec2::new(q.x - radius * acs.x, q.y.min(0.0))
    } else {
        w
    };

    // Signed distance
    let outside = w_clamped.length();
    let dot = q.x * acs.x + q.y * acs.y;
    if dot < radius {
        -outside * if w.x < 0.0 { -1.0 } else { 1.0 }
    } else {
        outside
    }
}

/// Signed distance from `p` to a line defined by normal `n` (unit) and offset `d`.
///
/// The line equation is `dot(p, n) = d`.  Positive side is the side `n` points to.
#[must_use]
#[inline]
pub fn half_plane_2d(p: Vec2, normal: Vec2, d: f32) -> f32 {
    p.dot(normal) - d
}

// ── 3D primitives ─────────────────────────────────────────────────────────────

/// Signed distance from `p` to a sphere centred at `centre`.
#[must_use]
#[inline]
pub fn sphere_3d(p: Vec3, centre: Vec3, radius: f32) -> f32 {
    (p - centre).length() - radius
}

/// Signed distance from `p` to an axis-aligned box centred at `centre`
/// with half-extents `half_size`.
#[must_use]
#[inline]
pub fn box_3d(p: Vec3, centre: Vec3, half_size: Vec3) -> f32 {
    let d = p - centre;
    let q = Vec3::new(d.x.abs(), d.y.abs(), d.z.abs()) - half_size;
    let outside = Vec3::new(q.x.max(0.0), q.y.max(0.0), q.z.max(0.0)).length();
    let inside = q.x.max(q.y).max(q.z).min(0.0);
    outside + inside
}

/// Signed distance from `p` to a rounded box (box with spherical corners).
#[must_use]
#[inline]
pub fn rounded_box_3d(p: Vec3, centre: Vec3, half_size: Vec3, radius: f32) -> f32 {
    let shrunk = Vec3::new(
        half_size.x - radius,
        half_size.y - radius,
        half_size.z - radius,
    );
    box_3d(p, centre, shrunk) - radius
}

/// Signed distance from `p` to an infinite cylinder along the Y axis,
/// centred at `(cx, cy)` in XZ with `radius`.
#[must_use]
#[inline]
pub fn cylinder_3d(p: Vec3, centre: Vec3, radius: f32, half_height: f32) -> f32 {
    let d = Vec2::new(
        Vec2::new(p.x - centre.x, p.z - centre.z).length() - radius,
        (p.y - centre.y).abs() - half_height,
    );
    d.x.max(d.y).min(0.0) + Vec2::new(d.x.max(0.0), d.y.max(0.0)).length()
}

/// Signed distance from `p` to a torus with major radius `R` and minor radius `r`,
/// lying in the XZ plane centred at `centre`.
#[must_use]
#[inline]
pub fn torus_3d(p: Vec3, centre: Vec3, major_r: f32, minor_r: f32) -> f32 {
    let q = p - centre;
    let xz_len = Vec2::new(q.x, q.z).length();
    let inner = Vec2::new(xz_len - major_r, q.y);
    inner.length() - minor_r
}

/// Signed distance from `p` to a capsule (line segment + spherical caps),
/// from `a` to `b` with `radius`.
#[must_use]
#[inline]
pub fn capsule_3d(p: Vec3, a: Vec3, b: Vec3, radius: f32) -> f32 {
    let ab = b - a;
    let ap = p - a;
    let t = (ap.dot(ab) / ab.dot(ab)).clamp(0.0, 1.0);
    (ap - ab * t).length() - radius
}

/// Signed distance from `p` to an infinite plane defined by unit normal `n` and offset `d`.
///
/// Plane equation: `dot(p, n) = d`. Points on the normal side have positive distance.
#[must_use]
#[inline]
pub fn plane_3d(p: Vec3, normal: Vec3, d: f32) -> f32 {
    p.dot(normal) - d
}

// ── Boolean / blending operators ──────────────────────────────────────────────

/// Union: smallest distance to either shape.
#[must_use]
#[inline]
pub const fn union(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

/// Intersection: largest distance (inside both shapes).
#[must_use]
#[inline]
pub const fn intersection(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

/// Subtraction: `a` minus `b` (inside `a`, outside `b`).
#[must_use]
#[inline]
pub const fn subtraction(a: f32, b: f32) -> f32 {
    if a > -b { a } else { -b }
}

/// Smooth union blends two shapes with a soft merging radius `k`.
///
/// `k = 0` degenerates to `min(a, b)`. Larger `k` creates a fatter junction.
#[must_use]
#[inline]
pub fn smooth_union(a: f32, b: f32, k: f32) -> f32 {
    if k <= 0.0 {
        return a.min(b);
    }
    let h = saturate(0.5 + 0.5 * (b - a) / k);
    mix(b, a, h) - k * h * (1.0 - h)
}

/// Smooth intersection of two shapes.
#[must_use]
#[inline]
pub fn smooth_intersection(a: f32, b: f32, k: f32) -> f32 {
    if k <= 0.0 {
        return a.max(b);
    }
    let h = saturate(0.5 - 0.5 * (b - a) / k);
    mix(b, a, h) + k * h * (1.0 - h)
}

/// Smooth subtraction (`a` minus `b`).
#[must_use]
#[inline]
pub fn smooth_subtraction(a: f32, b: f32, k: f32) -> f32 {
    if k <= 0.0 {
        return a.max(-b);
    }
    let h = saturate(0.5 - 0.5 * (b + a) / k);
    mix(a, -b, h) + k * h * (1.0 - h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{Vec2, Vec3};

    const TOL: f32 = 1e-4;

    // ── 2D ────────────────────────────────────────────────────────────────────

    #[test]
    fn circle_inside_outside() {
        // Unit circle at origin
        assert!((circle_2d(Vec2::ZERO, Vec2::ZERO, 1.0) - (-1.0)).abs() < TOL);
        assert!((circle_2d(Vec2::new(2.0, 0.0), Vec2::ZERO, 1.0) - 1.0).abs() < TOL);
        assert!(circle_2d(Vec2::new(1.0, 0.0), Vec2::ZERO, 1.0).abs() < TOL);
    }

    #[test]
    fn rect_corners_and_interior() {
        let hs = Vec2::new(1.0, 1.0);
        // Inside: centre → -1
        assert!((rect_2d(Vec2::ZERO, Vec2::ZERO, hs) - (-1.0)).abs() < TOL);
        // On edge: right face → 0
        assert!(rect_2d(Vec2::new(1.0, 0.0), Vec2::ZERO, hs).abs() < TOL);
        // Outside, right: 0.5 units away
        assert!((rect_2d(Vec2::new(1.5, 0.0), Vec2::ZERO, hs) - 0.5).abs() < TOL);
    }

    #[test]
    fn rounded_rect_corners_pulled_in() {
        // At the exact sharp corner, the rounded rect is *outside* (positive) while the
        // plain rect is on the boundary (zero) — rounding bevels corners inward.
        let corner = Vec2::new(2.0, 2.0);
        let plain = rect_2d(corner, Vec2::ZERO, Vec2::new(2.0, 2.0));
        let rounded = rounded_rect_2d(corner, Vec2::ZERO, Vec2::new(2.0, 2.0), 0.5);
        assert!(plain.abs() < 1e-4); // plain: exactly on corner
        assert!(rounded > 0.0); // rounded: that corner is clipped off → outside
    }

    #[test]
    fn segment_closest_point() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(4.0, 0.0);
        // Perpendicular above midpoint
        assert!((segment_2d(Vec2::new(2.0, 3.0), a, b) - 3.0).abs() < TOL);
        // Beyond endpoint b
        assert!((segment_2d(Vec2::new(5.0, 0.0), a, b) - 1.0).abs() < TOL);
    }

    #[test]
    fn capsule_wraps_segment() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(4.0, 0.0);
        // At midpoint, perpendicular at distance 1.0, radius 0.5 → d = 0.5
        assert!((capsule_2d(Vec2::new(2.0, 1.0), a, b, 0.5) - 0.5).abs() < TOL);
    }

    #[test]
    fn ring_inner_outer() {
        // Point on inner edge → d ≈ 0
        assert!(ring_2d(Vec2::new(1.0, 0.0), Vec2::ZERO, 1.0, 2.0).abs() < TOL);
        // Point at centre → outside (positive), distance = inner_radius - half_thickness
        let d = ring_2d(Vec2::ZERO, Vec2::ZERO, 1.0, 2.0);
        assert!(d > 0.0);
    }

    #[test]
    fn triangle_inside_outside() {
        let a = Vec2::new(0.0, 1.0);
        let b = Vec2::new(-1.0, -1.0);
        let c = Vec2::new(1.0, -1.0);
        // Centroid is inside
        let centroid = Vec2::new(0.0, -1.0 / 3.0);
        assert!(triangle_2d(centroid, a, b, c) < 0.0);
        // Far point is outside
        assert!(triangle_2d(Vec2::new(10.0, 0.0), a, b, c) > 0.0);
    }

    #[test]
    fn half_plane_sign() {
        let normal = Vec2::new(1.0, 0.0); // vertical plane at x=2
        let d = 2.0;
        assert!(half_plane_2d(Vec2::new(3.0, 0.0), normal, d) > 0.0); // right side
        assert!(half_plane_2d(Vec2::new(1.0, 0.0), normal, d) < 0.0); // left side
    }

    // ── 3D ────────────────────────────────────────────────────────────────────

    #[test]
    fn sphere_3d_basic() {
        assert!((sphere_3d(Vec3::ZERO, Vec3::ZERO, 1.0) - (-1.0)).abs() < TOL);
        assert!((sphere_3d(Vec3::new(2.0, 0.0, 0.0), Vec3::ZERO, 1.0) - 1.0).abs() < TOL);
    }

    #[test]
    fn box_3d_basic() {
        let hs = Vec3::new(1.0, 1.0, 1.0);
        assert!((box_3d(Vec3::ZERO, Vec3::ZERO, hs) - (-1.0)).abs() < TOL);
        assert!(box_3d(Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO, hs).abs() < TOL);
    }

    #[test]
    fn capsule_3d_basic() {
        let a = Vec3::new(0.0, -1.0, 0.0);
        let b = Vec3::new(0.0, 1.0, 0.0);
        // Point at origin is inside (radius 0.5)
        assert!(capsule_3d(Vec3::ZERO, a, b, 0.5) < 0.0);
        // Point at radius = 0.5 from axis → on boundary
        assert!(capsule_3d(Vec3::new(0.5, 0.0, 0.0), a, b, 0.5).abs() < TOL);
    }

    #[test]
    fn torus_3d_basic() {
        // Point on the ring centre (major_r from Y axis, in XZ plane) → inside torus by minor_r
        let p = Vec3::new(2.0, 0.0, 0.0); // major_r = 2, minor_r = 0.5
        let d = torus_3d(p, Vec3::ZERO, 2.0, 0.5);
        assert!((d - (-0.5)).abs() < TOL);
    }

    #[test]
    fn plane_3d_sign() {
        let n = Vec3::new(0.0, 1.0, 0.0); // XZ plane
        assert!(plane_3d(Vec3::new(0.0, 1.0, 0.0), n, 0.0) > 0.0);
        assert!(plane_3d(Vec3::new(0.0, -1.0, 0.0), n, 0.0) < 0.0);
    }

    // ── Boolean ops ───────────────────────────────────────────────────────────

    #[test]
    fn boolean_ops_basic() {
        let a = -1.0_f32; // inside shape A
        let b = 0.5_f32; // outside shape B

        assert_eq!(union(a, b), -1.0);
        assert_eq!(intersection(a, b), 0.5);
        assert_eq!(subtraction(a, b), -0.5); // inside A, outside B → inside
    }

    #[test]
    fn smooth_union_equals_hard_at_k0() {
        let a = 1.0_f32;
        let b = 2.0_f32;
        assert!((smooth_union(a, b, 0.0) - union(a, b)).abs() < TOL);
    }

    #[test]
    fn smooth_union_blends() {
        // Two overlapping circles → smooth union should be < min(a,b)
        let a = -0.5_f32;
        let b = -0.3_f32;
        let su = smooth_union(a, b, 0.5);
        assert!(su < a.min(b)); // blended result is more "inside"
    }

    #[test]
    fn smooth_intersection_equals_hard_at_k0() {
        let a = 1.0_f32;
        let b = 0.5_f32;
        assert!((smooth_intersection(a, b, 0.0) - intersection(a, b)).abs() < TOL);
    }
}
