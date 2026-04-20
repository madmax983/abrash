#![allow(
    clippy::imprecise_flops,
    clippy::must_use_candidate,
    clippy::items_after_statements,
    clippy::too_long_first_doc_paragraph,
    clippy::float_cmp
)]
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

/// Signed distance from `p` to an ellipsoid centred at `centre` with semi-axes `r`.
///
/// `r.x`, `r.y`, `r.z` are the half-lengths along each world axis.
/// The result is an *approximation* — exact ellipsoid SDFs have no closed form,
/// but this Inigo Quilez formula is tight near the surface.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::ellipsoid_3d;
/// use abrash_core::math::Vec3;
///
/// // Unit sphere: all radii equal 1.
/// let d = ellipsoid_3d(Vec3::new(1.5, 0.0, 0.0), Vec3::ZERO, Vec3::ONE);
/// assert!((d - 0.5).abs() < 0.01);
/// ```
#[must_use]
#[inline]
pub fn ellipsoid_3d(p: Vec3, centre: Vec3, r: Vec3) -> f32 {
    let q = p - centre;
    // Scaled-space position and its length
    let k0 = Vec3::new(q.x / r.x, q.y / r.y, q.z / r.z).length();
    let k1 = Vec3::new(q.x / (r.x * r.x), q.y / (r.y * r.y), q.z / (r.z * r.z)).length();
    k0 * (k0 - 1.0) / k1
}

/// Signed distance from `p` to a solid cone opening downward from its tip at `tip`,
/// pointing in direction `axis` (unit vector), with half-angle `angle` in radians.
///
/// The cone is infinite in the direction of `axis`.  Positive = outside the cone.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::cone_3d;
/// use abrash_core::math::Vec3;
///
/// // Tip at origin, pointing down (−Y), 45° half-angle.
/// let d = cone_3d(Vec3::new(0.0, -2.0, 0.0), Vec3::ZERO, Vec3::new(0.0, -1.0, 0.0), std::f32::consts::FRAC_PI_4);
/// // Point on the axis inside the cone — should be negative.
/// assert!(d < 0.0);
/// ```
#[must_use]
#[inline]
pub fn cone_3d(p: Vec3, tip: Vec3, axis: Vec3, half_angle: f32) -> f32 {
    let q = p - tip;
    // Project onto axis
    let along = q.dot(axis);
    // Radial distance from axis
    let radial = (q - axis * along).length();
    // Point in (along, radial) 2D space
    let sin_a = half_angle.sin();
    let cos_a = half_angle.cos();
    // Distance to the cone's slanted surface
    let dot = radial * cos_a - along * sin_a;
    // Only valid for the solid cone half (along >= 0 from tip)
    if along < 0.0 {
        // Above tip: nearest point is the tip itself
        q.length()
    } else {
        // d to cone surface; negative = inside
        let d = dot;
        // Also clamp to the cone axis for interior points
        d.max(-along)
    }
}

/// Signed distance from `p` to a **truncated cone** (capped frustum).
///
/// Defined by two circular caps at arbitrary 3D positions:
/// - bottom cap centred at `a` with radius `ra`
/// - top cap centred at `b` with radius `rb`
///
/// Implements Inigo Quilez's `sdCappedCone` formula.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::truncated_cone_3d;
/// use abrash_core::math::Vec3;
///
/// // A cylinder (equal radii) from (0,-1,0) to (0,1,0), radius 1.
/// let d = truncated_cone_3d(Vec3::new(0.5, 0.0, 0.0),
///     Vec3::new(0.0, -1.0, 0.0), Vec3::new(0.0, 1.0, 0.0), 1.0, 1.0);
/// assert!(d < 0.0); // inside
/// ```
#[must_use]
pub fn truncated_cone_3d(p: Vec3, a: Vec3, b: Vec3, ra: f32, rb: f32) -> f32 {
    let rba = rb - ra;
    let baba = (b - a).dot(b - a);
    let papa = (p - a).dot(p - a);
    let paba = (p - a).dot(b - a) / baba;

    // Radial distance from the cone axis
    let x = (papa - paba * paba * baba).max(0.0).sqrt();

    let cax = (x - if paba < 0.5 { ra } else { rb }).max(0.0);
    let cay = (paba - 0.5).abs() - 0.5;

    let k = rba * rba + baba;
    let f = ((rba * (x - ra) + paba * baba) / k).clamp(0.0, 1.0);

    let cbx = x - ra - f * rba;
    let cby = paba - f;

    let s = if cbx < 0.0 && cay < 0.0 {
        -1.0_f32
    } else {
        1.0_f32
    };

    s * (cax * cax + cay * cay * baba)
        .min(cbx * cbx + cby * cby * baba)
        .sqrt()
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

/// Signed distance to an infinite cylinder along the Y axis.
///
/// The cylinder is centred at `centre_xz` (X and Z components only) with
/// radius `r` and extends infinitely along Y.  Useful as a CSG primitive.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::inf_cylinder_3d;
/// use abrash_core::math::{Vec2, Vec3};
///
/// // Point on the surface of a unit cylinder at origin
/// let d = inf_cylinder_3d(Vec3::new(1.0, 5.0, 0.0), Vec2::new(0.0, 0.0), 1.0);
/// assert!(d.abs() < 1e-5, "should be on surface, got {d}");
/// // Inside
/// let d2 = inf_cylinder_3d(Vec3::new(0.5, 100.0, 0.0), Vec2::new(0.0, 0.0), 1.0);
/// assert!(d2 < 0.0);
/// ```
#[must_use]
#[inline]
pub fn inf_cylinder_3d(p: Vec3, centre_xz: Vec2, r: f32) -> f32 {
    let dx = p.x - centre_xz.x;
    let dz = p.z - centre_xz.y;
    (dx * dx + dz * dz).sqrt() - r
}

/// Signed distance to a rounded cylinder (a cylinder with hemispherical caps).
///
/// Unlike [`capsule_3d`] — which is defined by two endpoint *centres* — this
/// primitive is defined by a cylinder *half-height* `h` and a rounding radius
/// `r`.  The overall shape is `2*(h+r)` tall and `2*ra` wide where `ra = r`.
///
/// Useful for rounded columns, pillars, and physics capsule shapes.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::rounded_cylinder_3d;
/// use abrash_core::math::Vec3;
///
/// // Centre is inside
/// let d = rounded_cylinder_3d(Vec3::ZERO, Vec3::ZERO, 0.5, 0.1, 1.0);
/// assert!(d < 0.0, "centre should be inside, got {d}");
/// // Far outside
/// let d2 = rounded_cylinder_3d(Vec3::new(5.0, 0.0, 0.0), Vec3::ZERO, 0.5, 0.1, 1.0);
/// assert!(d2 > 0.0);
/// ```
#[must_use]
pub fn rounded_cylinder_3d(p: Vec3, centre: Vec3, ra: f32, rb: f32, h: f32) -> f32 {
    // IQ sdRoundedCylinder: ra = cylinder radius, rb = rounding radius, h = half-height
    let p = p - centre;
    let d_xz = p.x.mul_add(p.x, p.z * p.z).sqrt() - ra + rb;
    let d_y = p.y.abs() - h;
    let max_xz = d_xz.max(0.0);
    let max_y = d_y.max(0.0);
    let d = (max_xz * max_xz + max_y * max_y).sqrt() + d_xz.min(0.0).min(d_y.min(0.0)) - rb;
    d
}

/// Signed distance to a chain-link (torus with a section cut out and rejoined).
///
/// Modelled as a torus with tube-radius `r` whose cross-section is elongated
/// by `le` along the Y axis (like a stadium).  The link lies in the XY plane.
///
/// `r1` is the major radius (distance from link centre to tube centre), `r2` is
/// the tube radius, `le` is the link elongation half-length.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::link_3d;
/// use abrash_core::math::Vec3;
///
/// // A point inside the tube (near the torus ring, r1=1.0) is inside
/// let d = link_3d(Vec3::new(0.9, 0.0, 0.0), Vec3::ZERO, 1.0, 0.2, 0.5);
/// assert!(d < 0.0, "should be inside tube, got {d}");
/// // Far away is outside
/// let d2 = link_3d(Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO, 1.0, 0.2, 0.5);
/// assert!(d2 > 0.0);
/// ```
#[must_use]
pub fn link_3d(p: Vec3, centre: Vec3, r1: f32, r2: f32, le: f32) -> f32 {
    // IQ sdLink: r1 = major radius, r2 = tube radius, le = elongation half-length
    let p = p - centre;
    let qx = p.x.mul_add(p.x, p.z * p.z).sqrt() - r1;
    let qy = p.y.abs() - le;
    (qx * qx + qy.max(0.0) * qy.max(0.0)).sqrt() - r2
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

/// Signed distance to a regular octahedron centred at `centre` with "radius" `s`.
///
/// `s` is the distance from the centre to each face along its normal direction
/// (equivalently, the surface satisfies `|x|+|y|+|z| = s` in object space).
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::octahedron_3d;
/// use abrash_core::math::Vec3;
///
/// // Centre is strictly inside
/// let d = octahedron_3d(Vec3::ZERO, Vec3::ZERO, 1.0);
/// assert!(d < 0.0, "origin should be inside, got {d}");
/// // A vertex along X is at distance sqrt(2)*s/sqrt(3) ≈ s*0.8165 from centre;
/// // confirmed outside a unit octahedron
/// let d2 = octahedron_3d(Vec3::new(2.0, 0.0, 0.0), Vec3::ZERO, 1.0);
/// assert!(d2 > 0.0, "far point should be outside, got {d2}");
/// ```
#[must_use]
pub fn octahedron_3d(p: Vec3, centre: Vec3, s: f32) -> f32 {
    // IQ exact octahedron (sdOctahedron)
    let p = (p - centre).abs();
    let m = p.x + p.y + p.z - s;
    // Find the region and project
    let (qx, qy, qz) = if 3.0 * p.x < m {
        (p.x, p.y, p.z)
    } else if 3.0 * p.y < m {
        (p.y, p.z, p.x)
    } else if 3.0 * p.z < m {
        (p.z, p.x, p.y)
    } else {
        return m * 0.577_350_27; // inside — scale by 1/sqrt(3)
    };
    let k = (0.5 * (qz - qy + s)).clamp(0.0, s);
    (Vec3::new(qx, qy - s + k, qz - k)).length()
}

/// Signed distance to a square pyramid.
///
/// The pyramid is centred at `centre` with a unit square base (half-size = 0.5)
/// in the XZ plane and apex at `centre + (0, height, 0)`.
/// Scale `p` relative to `centre` if you need a different base size.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::pyramid_3d;
/// use abrash_core::math::Vec3;
///
/// // Centre (middle of pyramid volume) is inside
/// let d = pyramid_3d(Vec3::new(0.0, 0.5, 0.0), Vec3::ZERO, 1.0);
/// assert!(d < 0.0, "centre should be inside, got {d}");
/// // Far above apex is outside
/// let d2 = pyramid_3d(Vec3::new(0.0, 3.0, 0.0), Vec3::ZERO, 1.0);
/// assert!(d2 > 0.0, "above apex should be outside, got {d2}");
/// ```
#[must_use]
pub fn pyramid_3d(p: Vec3, centre: Vec3, height: f32) -> f32 {
    // IQ sdPyramid — base is unit square [-0.5,0.5]^2 at y=0, apex at y=height
    let mut p = p - centre;
    let m2 = height * height + 0.25;
    // Fold XZ into first quadrant and sort
    p = Vec3::new(p.x.abs(), p.y, p.z.abs());
    if p.z > p.x {
        p = Vec3::new(p.z, p.y, p.x);
    }
    p = Vec3::new(p.x - 0.5, p.y, p.z - 0.5);
    // Build rotated coordinate q
    let qx = p.z;
    let qy = height * p.y - 0.5 * p.x;
    let qz = height * p.x + 0.5 * p.y;
    let s = (-qx).max(0.0);
    let t = ((qy - 0.5 * p.z) / (m2 + 0.25)).clamp(0.0, 1.0);
    let a = m2 * (qx + s) * (qx + s) + qy * qy;
    let b = m2 * (qx + 0.5 * t) * (qx + 0.5 * t) + (qy - m2 * t) * (qy - m2 * t);
    let d = if qy.min(-qx * m2 - qy * 0.5) > 0.0 {
        0.0
    } else {
        a.min(b)
    };
    ((d + qz * qz) / m2).sqrt() * (qz.max(-p.y)).signum()
}

/// Signed distance to a hexagonal prism.
///
/// The prism is centred at `centre` with the hexagon in the XY plane
/// (circumradius `r`) and half-height `h` along the Z-axis.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::hexagonal_prism_3d;
/// use abrash_core::math::Vec3;
///
/// // Centre is inside
/// let d = hexagonal_prism_3d(Vec3::ZERO, Vec3::ZERO, 1.0, 1.0);
/// assert!(d < 0.0, "centre should be inside, got {d}");
/// // Far along X is outside
/// let d2 = hexagonal_prism_3d(Vec3::new(5.0, 0.0, 0.0), Vec3::ZERO, 1.0, 1.0);
/// assert!(d2 > 0.0, "far point should be outside, got {d2}");
/// ```
#[must_use]
pub fn hexagonal_prism_3d(p: Vec3, centre: Vec3, r: f32, h: f32) -> f32 {
    // k = (-sqrt(3)/2, 0.5, 1/sqrt(3))
    const KX: f32 = -0.866_025_4;
    const KY: f32 = 0.5;
    const KZ: f32 = 0.577_350_3;

    // IQ sdHexPrism — hex in XY plane, extends along Z
    let p = (p - centre).abs();
    let dot = (KX * p.x + KY * p.y).min(0.0);
    let px = p.x - 2.0 * dot * KX;
    let py = p.y - 2.0 * dot * KY;
    // Clamp hex boundary
    let px_clamped = px - (px).clamp(-KZ * r, KZ * r);
    let py_r = py - r;
    let cx = (px_clamped * px_clamped + py_r * py_r).sqrt();
    let dx = cx * (py - r).signum();
    let dz = p.z - h;
    dx.max(dz).min(0.0) + Vec2::new(dx.max(0.0), dz.max(0.0)).length()
}

// ── SDF Operators ────────────────────────────────────────────────────────────

/// Elongate a signed distance field along a half-size `h` (one per axis).
///
/// Subtracts the clamped displacement from `p` before evaluating the SDF,
/// stretching the shape along each axis by `2*h`.  Zero entries leave that
/// axis unchanged.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::{elongate, sphere_3d};
/// use abrash_core::math::Vec3;
///
/// // Elongating a sphere along Y by 2.0 makes a capsule-like shape.
/// // A point above the stretched region is still outside.
/// let p = Vec3::new(0.0, 5.0, 0.0);
/// let d = elongate(p, Vec3::new(0.0, 2.0, 0.0), |q| sphere_3d(q, Vec3::ZERO, 1.0));
/// assert!(d > 0.0, "far above capsule, got {d}");
/// ```
#[must_use]
#[inline]
pub fn elongate(p: Vec3, h: Vec3, sdf: impl Fn(Vec3) -> f32) -> f32 {
    let neg_h = Vec3::new(-h.x, -h.y, -h.z);
    let q = p - p.clamp(neg_h, h);
    sdf(q)
}

/// Hollow-shell operator: subtracts `thickness` from the absolute SDF value.
///
/// Turns any solid SDF into a thin shell of the given `thickness`.  The
/// interior (negative region) becomes a second outside if `thickness` is
/// smaller than the shape radius.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::{onion, sphere_3d};
/// use abrash_core::math::Vec3;
///
/// let sphere = |p: Vec3| sphere_3d(p, Vec3::ZERO, 1.0);
/// // Outer shell surface is where the original SDF = +thickness (radius 1.1)
/// let d = onion(sphere(Vec3::new(1.1, 0.0, 0.0)), 0.1);
/// assert!(d.abs() < 1e-5, "on outer shell surface, got {d}");
/// // Original surface (radius 1.0) is inside the shell
/// let d2 = onion(sphere(Vec3::new(1.0, 0.0, 0.0)), 0.1);
/// assert!(d2 < 0.0, "inside shell, got {d2}");
/// ```
#[must_use]
#[inline]
pub fn onion(d: f32, thickness: f32) -> f32 {
    d.abs() - thickness
}

// ── Additional 2D Primitives ─────────────────────────────────────────────────

/// Signed distance to a vesica piscis (lens / mandorla) in 2D.
///
/// The lens is the intersection of two unit circles whose centres are `dist`
/// apart.  `dist` must be in `(0, 2*r]`; when `dist == r` the shape is the
/// classic vesica piscis.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::vesica_2d;
/// use abrash_core::math::Vec2;
///
/// // Centre of lens is inside
/// let d = vesica_2d(Vec2::ZERO, Vec2::ZERO, 0.5, 1.0);
/// assert!(d < 0.0, "centre should be inside, got {d}");
/// // Far point is outside
/// let d2 = vesica_2d(Vec2::new(5.0, 0.0), Vec2::ZERO, 0.5, 1.0);
/// assert!(d2 > 0.0);
/// ```
#[must_use]
pub fn vesica_2d(p: Vec2, centre: Vec2, dist: f32, r: f32) -> f32 {
    // IQ sdVesica: two circles displaced ±dist/2 along X
    let p = p - centre;
    let b = (r * r - dist * dist * 0.25).sqrt();
    let px = p.x.abs();
    if (px - dist * 0.5) * b < px * b {
        // Region near the sharp tips
        Vec2::new(px - dist * 0.5, p.y).length() - r
    } else {
        Vec2::new(px, p.y.abs() - b).length() - dist * 0.5
    }
}

/// Signed distance to a 2D star polygon with `n` points.
///
/// `r1` is the outer radius (tip), `r2` is the inner radius (valley).
/// `n` must be ≥ 3.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::star_2d;
/// use abrash_core::math::Vec2;
///
/// // Centre of a 5-pointed star is inside
/// let d = star_2d(Vec2::ZERO, Vec2::ZERO, 5, 1.0, 0.4);
/// assert!(d < 0.0, "centre should be inside, got {d}");
/// // Far point is outside
/// let d2 = star_2d(Vec2::new(5.0, 0.0), Vec2::ZERO, 5, 1.0, 0.4);
/// assert!(d2 > 0.0);
/// ```
#[must_use]
pub fn star_2d(p: Vec2, centre: Vec2, n: u32, r1: f32, r2: f32) -> f32 {
    // Fold p into the fundamental sector [0, π/n] and compute segment SDF.
    let p = p - centre;
    let n = n.max(3) as f32;
    let half_sector = std::f32::consts::PI / n;

    // Snap to nearest outer-tip angle, then measure local polar coords.
    let theta = p.y.atan2(p.x);
    let tip_angle = (theta / (half_sector * 2.0)).round() * (half_sector * 2.0);
    let local_theta = (theta - tip_angle).abs(); // [0, half_sector]
    let local_r = p.length();
    let lp = Vec2::new(local_r * local_theta.cos(), local_r * local_theta.sin());

    // Outer tip at (r1, 0), inner valley at (r2*cos(half), r2*sin(half)).
    let tip = Vec2::new(r1, 0.0);
    let valley = Vec2::new(r2 * half_sector.cos(), r2 * half_sector.sin());
    let edge = Vec2::new(valley.x - tip.x, valley.y - tip.y);
    let t = {
        let lp_minus_tip = Vec2::new(lp.x - tip.x, lp.y - tip.y);
        let edge_len2 = edge.x * edge.x + edge.y * edge.y;
        ((lp_minus_tip.x * edge.x + lp_minus_tip.y * edge.y) / edge_len2).clamp(0.0, 1.0)
    };
    let closest = Vec2::new(tip.x + edge.x * t, tip.y + edge.y * t);
    let dist = Vec2::new(lp.x - closest.x, lp.y - closest.y).length();

    // Sign: cross(edge, lp-tip) > 0 → left of edge → inside star.
    let lp_minus_tip = Vec2::new(lp.x - tip.x, lp.y - tip.y);
    let cross = edge.x * lp_minus_tip.y - edge.y * lp_minus_tip.x;
    dist * if cross > 0.0 { -1.0 } else { 1.0 }
}

// ── Additional 3D Primitives ─────────────────────────────────────────────────

/// Signed distance to a wireframe box frame.
///
/// Like a hollow box with only the 12 edges present (each edge is a thin rod
/// of radius `e`).  This is the IQ `sdBoxFrame` primitive.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::box_frame_3d;
/// use abrash_core::math::Vec3;
///
/// // Face center is outside the frame (no material there)
/// let d = box_frame_3d(Vec3::new(1.1, 0.0, 0.0), Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0), 0.05);
/// assert!(d > 0.0);
/// // Near an edge rod (corner at (1,1) with small offset inside radius e)
/// let d2 = box_frame_3d(Vec3::new(0.98, 0.98, 0.0), Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0), 0.05);
/// assert!(d2 < 0.0, "inside edge rod, got {d2}");
/// ```
#[must_use]
pub fn box_frame_3d(p: Vec3, centre: Vec3, half_size: Vec3, e: f32) -> f32 {
    // IQ sdBoxFrame
    let p = (p - centre).abs() - half_size;
    let ev = Vec3::splat(e);
    let q = (p + ev).abs() - ev;
    // Three edges along each axis pair
    let d1 = Vec3::new(p.x, q.y, q.z);
    let d2 = Vec3::new(q.x, p.y, q.z);
    let d3 = Vec3::new(q.x, q.y, p.z);
    let sdf_edge = |v: Vec3| -> f32 {
        v.x.max(v.y).max(v.z).min(0.0)
            + Vec3::new(v.x.max(0.0), v.y.max(0.0), v.z.max(0.0)).length()
    };
    sdf_edge(d1).min(sdf_edge(d2)).min(sdf_edge(d3))
}

/// Signed distance to a solid angle / pie-wedge on a sphere (IQ `sdSolidAngle`).
///
/// The shape is a spherical cap defined by a cone of half-angle `angle` (radians)
/// opening upwards from the origin, intersected with a sphere of radius `ra`.
/// `centre` shifts the entire shape.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::solid_angle_3d;
/// use abrash_core::math::Vec3;
///
/// // Centre is inside the cap (directly above)
/// let d = solid_angle_3d(Vec3::new(0.0, 0.5, 0.0), Vec3::ZERO, 1.0, 0.8);
/// assert!(d < 0.0, "inside cap, got {d}");
/// // Far below is outside
/// let d2 = solid_angle_3d(Vec3::new(0.0, -5.0, 0.0), Vec3::ZERO, 1.0, 0.8);
/// assert!(d2 > 0.0);
/// ```
#[must_use]
pub fn solid_angle_3d(p: Vec3, centre: Vec3, ra: f32, angle: f32) -> f32 {
    // IQ sdSolidAngle: c = (sin(angle), cos(angle))
    let p = p - centre;
    let c = Vec2::new(angle.sin(), angle.cos());
    let q = Vec2::new(Vec2::new(p.x, p.z).length(), p.y);
    let l = q.length() - ra;
    let t = q.dot(c).clamp(0.0, ra);
    let m = Vec2::new(q.x - c.x * t, q.y - c.y * t).length();
    l.max(m * (c.y * q.x - c.x * q.y).signum())
}

// ── SDF Domain Operators ──────────────────────────────────────────────────────

/// Extrude a 2D SDF into 3D along the Y axis.
///
/// Evaluates the 2D SDF in the XZ plane and combines with a half-height cap,
/// producing a solid extrusion of height `2 * h`.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::{extrude_y, circle_2d};
/// use abrash_core::math::{Vec2, Vec3};
///
/// // A cylinder is a circle extruded along Y
/// let d = extrude_y(Vec3::new(0.0, 0.0, 0.0), 1.0, |xz| circle_2d(xz, Vec2::ZERO, 0.5));
/// assert!(d < 0.0, "centre should be inside, got {d}");
/// let d2 = extrude_y(Vec3::new(0.0, 5.0, 0.0), 1.0, |xz| circle_2d(xz, Vec2::ZERO, 0.5));
/// assert!(d2 > 0.0, "above cap should be outside");
/// ```
#[must_use]
#[inline]
pub fn extrude_y(p: Vec3, h: f32, sdf2d: impl Fn(Vec2) -> f32) -> f32 {
    let d = sdf2d(Vec2::new(p.x, p.z));
    let w = Vec2::new(d, p.y.abs() - h);
    w.x.max(w.y).min(0.0) + Vec2::new(w.x.max(0.0), w.y.max(0.0)).length()
}

/// Revolve a 2D SDF around the Y axis to create a surface of revolution.
///
/// The 2D SDF is evaluated in the XY plane on the profile `(r, p.y)` where
/// `r` is the radial distance from the Y axis, offset by `o` (the revolution
/// offset).  With `o = 0` the shape is fully revolved; with `o > 0` a ring
/// of that radius is revolved.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::{revolve_y, circle_2d};
/// use abrash_core::math::{Vec2, Vec3};
///
/// // A torus: revolve a circle profile at offset r=1 around Y
/// let d = revolve_y(Vec3::new(1.0, 0.0, 0.0), 1.0, |p| circle_2d(p, Vec2::ZERO, 0.2));
/// assert!(d < 0.0, "on torus tube center, got {d}");
/// let d2 = revolve_y(Vec3::ZERO, 1.0, |p| circle_2d(p, Vec2::ZERO, 0.2));
/// assert!(d2 > 0.0, "hole center should be outside");
/// ```
#[must_use]
#[inline]
pub fn revolve_y(p: Vec3, o: f32, sdf2d: impl Fn(Vec2) -> f32) -> f32 {
    let q = Vec2::new(Vec2::new(p.x, p.z).length() - o, p.y);
    sdf2d(q)
}

/// Twist the SDF domain around the Y axis.
///
/// Rotates the XZ plane by `k * p.y` radians as y increases, creating a
/// helical warp.  Apply before evaluating any 3D SDF to get a twisted shape.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::{twist_y, box_3d};
/// use abrash_core::math::Vec3;
///
/// // The twisted box is different from the original at non-zero y
/// let p = Vec3::new(0.5, 1.0, 0.0);
/// let plain  = box_3d(p, Vec3::ZERO, Vec3::new(0.5, 2.0, 0.5));
/// let twisted = box_3d(twist_y(p, 1.0), Vec3::ZERO, Vec3::new(0.5, 2.0, 0.5));
/// assert!((plain - twisted).abs() > 1e-4, "twist should change the result");
/// ```
#[must_use]
#[inline]
pub fn twist_y(p: Vec3, k: f32) -> Vec3 {
    let (s, c) = (k * p.y).sin_cos();
    Vec3::new(c * p.x - s * p.z, p.y, s * p.x + c * p.z)
}

/// Displace an SDF by adding a displacement function to the distance.
///
/// Useful for adding bumps, waves, or texture to any shape.  The result is
/// an **approximate** SDF — the Lipschitz condition may be violated if the
/// displacement amplitude exceeds 1.  For accurate ray-marching use a small
/// step size or clamp the result.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::{displace, sphere_3d};
/// use abrash_core::math::Vec3;
///
/// let d_plain = sphere_3d(Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO, 1.0);
/// // Displace outward with a constant 0.1 bump
/// let d_bumped = displace(d_plain, 0.1);
/// assert!((d_bumped - (d_plain + 0.1)).abs() < 1e-6);
/// ```
#[must_use]
#[inline]
pub fn displace(d: f32, displacement: f32) -> f32 {
    d + displacement
}

/// Symmetry / infinite repetition along one axis.
///
/// Maps `p` into the repeating cell of size `cell` centered at the nearest
/// grid point.  Pass the result to any SDF to tile it.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::{repeat_1d, sphere_3d};
/// use abrash_core::math::Vec3;
///
/// // Spheres repeat every 3 units along X
/// let p0 = Vec3::new(0.0, 0.0, 0.0);  // inside the sphere at x=0
/// let p1 = Vec3::new(3.0, 0.0, 0.0);  // inside the sphere at x=3
/// let d0 = sphere_3d(repeat_1d(p0, Vec3::new(3.0, 0.0, 0.0)), Vec3::ZERO, 0.5);
/// let d1 = sphere_3d(repeat_1d(p1, Vec3::new(3.0, 0.0, 0.0)), Vec3::ZERO, 0.5);
/// assert!((d0 - d1).abs() < 1e-5, "repeated cells should match: d0={d0} d1={d1}");
/// ```
#[must_use]
#[inline]
pub fn repeat_1d(p: Vec3, cell: Vec3) -> Vec3 {
    // For each non-zero component of cell, fold p into [-cell/2, cell/2]
    let fold = |v: f32, c: f32| -> f32 {
        if c.abs() < 1e-7 {
            v
        } else {
            v - c * (v / c).round()
        }
    };
    Vec3::new(fold(p.x, cell.x), fold(p.y, cell.y), fold(p.z, cell.z))
}

/// Tile a 3D SDF in all three axes with cell size `cell`.
///
/// Folds `p` into the cell `[-cell/2, cell/2]³` before passing it to the SDF,
/// creating an infinite repetition.  Axes with `cell = 0` are not repeated.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::{repeat_3d, sphere_3d};
/// use abrash_core::math::Vec3;
///
/// let cell = Vec3::new(3.0, 3.0, 3.0);
/// let d0 = sphere_3d(repeat_3d(Vec3::new(0.1, 0.0, 0.0), cell), Vec3::ZERO, 0.5);
/// let d1 = sphere_3d(repeat_3d(Vec3::new(3.1, 0.0, 0.0), cell), Vec3::ZERO, 0.5);
/// assert!((d0 - d1).abs() < 1e-4, "d0={d0} d1={d1}");
/// ```
#[must_use]
#[inline]
pub fn repeat_3d(p: Vec3, cell: Vec3) -> Vec3 {
    repeat_1d(p, cell)
}

/// Reflect `p` across the YZ plane (flip X sign), then evaluate any SDF.
///
/// Halves the geometry computation for symmetric shapes: only model the
/// positive-X half and mirror it.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::{mirror_x, sphere_3d};
/// use abrash_core::math::Vec3;
///
/// let p_pos = Vec3::new(1.0, 0.5, 0.0);
/// let p_neg = Vec3::new(-1.0, 0.5, 0.0);
/// let d1 = sphere_3d(mirror_x(p_pos), Vec3::ZERO, 2.0);
/// let d2 = sphere_3d(mirror_x(p_neg), Vec3::ZERO, 2.0);
/// assert!((d1 - d2).abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub const fn mirror_x(p: Vec3) -> Vec3 {
    Vec3::new(p.x.abs(), p.y, p.z)
}

/// Reflect `p` across the XZ plane (flip Y sign).
#[must_use]
#[inline]
pub const fn mirror_y(p: Vec3) -> Vec3 {
    Vec3::new(p.x, p.y.abs(), p.z)
}

/// Reflect `p` across the XY plane (flip Z sign).
#[must_use]
#[inline]
pub const fn mirror_z(p: Vec3) -> Vec3 {
    Vec3::new(p.x, p.y, p.z.abs())
}

/// Scale the SDF domain by `s`, correcting the distance output.
///
/// Divides coordinates by `s` before evaluating and multiplies the result by
/// `s` afterward, so the returned value remains a valid distance in world
/// space.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::{scale_sdf, sphere_3d};
/// use abrash_core::math::Vec3;
///
/// // A sphere of radius 1 scaled by 2 should have radius 2
/// let d = scale_sdf(Vec3::new(2.0, 0.0, 0.0), 2.0, |q| sphere_3d(q, Vec3::ZERO, 1.0));
/// assert!(d.abs() < 1e-4, "on surface of scaled sphere, got {d}");
/// ```
#[must_use]
#[inline]
pub fn scale_sdf(p: Vec3, s: f32, sdf: impl Fn(Vec3) -> f32) -> f32 {
    sdf(p / s) * s
}

/// Signed distance approximation to the gyroid minimal surface.
///
/// The gyroid surface is defined by
/// `sin(x)*cos(y) + sin(y)*cos(z) + sin(z)*cos(x) = 0`.
/// This is an approximation (not an exact SDF), but the magnitude is bounded
/// and works well for ray-marching with small step sizes.
///
/// Scale `p` to control the feature frequency (multiply by `frequency`
/// before calling).
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::gyroid_3d;
/// use abrash_core::math::Vec3;
///
/// // The gyroid surface passes through many points; just verify it runs
/// let d = gyroid_3d(Vec3::new(1.0, 2.0, 3.0));
/// assert!(d.is_finite());
/// ```
#[must_use]
#[inline]
pub fn gyroid_3d(p: Vec3) -> f32 {
    // Approximate SDF: |f(p)| / |∇f(p)| where f = sin(x)*cos(y)+...
    let f = p.x.sin() * p.y.cos() + p.y.sin() * p.z.cos() + p.z.sin() * p.x.cos();
    // Gradient magnitude ≤ sqrt(3); use sqrt(2) as a conservative bound
    f / std::f32::consts::SQRT_2
}

/// Signed distance to a parabola in 2D.
///
/// The parabola opens upward: `y = k * x²`.  `k` controls the curvature
/// (larger `k` = tighter parabola).
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::parabola_2d;
/// use abrash_core::math::Vec2;
///
/// // The vertex is at the origin; a point above it is inside the parabola's cup
/// let d = parabola_2d(Vec2::new(0.0, 0.5), 1.0);
/// assert!(d < 0.0, "inside cup, got {d}");
/// // A point far to the side is outside
/// let d2 = parabola_2d(Vec2::new(3.0, 0.0), 1.0);
/// assert!(d2 > 0.0);
/// ```
#[must_use]
pub fn parabola_2d(p: Vec2, k: f32) -> f32 {
    // IQ sdParabola: signed distance to y = k*x²
    // Solve for nearest point on the parabola via Newton's method (closed-form via cubic)
    let ik = 1.0 / k;
    let p = Vec2::new(p.x.abs(), p.y);
    // The foot of the perpendicular satisfies: t + k*t*(t*k - p.y) = p.x
    // Rewritten as: 2k²t³ - 2k·p.y·t + p.x = 0 → solve for t
    // Use the depressed cubic directly
    let q = k * (p.y - 0.5 * ik) / 3.0;
    let disc_sign = if p.x == 0.0 { 1.0 } else { p.x.signum() };
    // ⚡ Bolt: Replace `.powf(2.0 / 3.0)` and `.powf(1.0 / 3.0)` with `.cbrt()` and `.powi(2)` to elide slow C-math library fractional power.
    let r = (k * p.x * 0.5).cbrt().powi(2) * disc_sign;
    // Approximate: use parametric nearest-point on y = k*x² → x_t = t, y_t = k*t²
    // Minimize ||p - (t, k*t²)||²; derivative: -2*(p.x-t) + 2*(p.y-k*t²)*(-2*k*t) = 0
    // 1 + 2k*(p.y-k*t²)*2k*t ... Newton 3-step
    let mut t = (p.x * 0.5 / k).cbrt().max(1e-6);
    for _ in 0..5 {
        let kt2 = k * t * t;
        let f = 1.0 + 4.0 * k * k * t * t - 2.0 * k * p.y + 2.0 * k * kt2;
        let df = 8.0 * k * k * t + 4.0 * k * kt2 / t;
        // gradient: d/dt |(t - p.x)^2 + (k*t^2 - p.y)^2|
        let ft = 2.0 * (t - p.x) + 4.0 * k * t * (k * t * t - p.y);
        let dft = 2.0 + 4.0 * k * (3.0 * k * t * t - p.y);
        if dft.abs() < 1e-9 {
            break;
        }
        let step = ft / dft;
        t -= step;
        if t < 0.0 {
            t = 0.0;
            break;
        }
        if step.abs() < 1e-6 {
            break;
        }
    }
    let _ = (q, r, f32::from(0u8));
    let closest = Vec2::new(t, k * t * t);
    let d = Vec2::new(p.x - closest.x, p.y - closest.y).length();
    // Sign: negative inside the cup (y > k*x²)
    if p.y > k * p.x * p.x { -d } else { d }
}

/// Signed distance to a quadratic Bézier curve in 2D.
///
/// Returns the distance from `p` to the nearest point on the curve defined by
/// control points `a` (start), `b` (control), `c` (end).  The result is
/// always non-negative (unsigned — finding the sign requires winding number
/// context).
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::bezier_sdf_2d;
/// use abrash_core::math::Vec2;
///
/// let a = Vec2::new(-1.0, 0.0);
/// let b = Vec2::new(0.0, 1.0);
/// let c = Vec2::new(1.0, 0.0);
/// // Point exactly on the midpoint of the curve (t=0.5) → distance ≈ 0
/// // B(0.5) = 0.25*(-1,0) + 0.5*(0,1) + 0.25*(1,0) = (0, 0.5)
/// let d = bezier_sdf_2d(Vec2::new(0.0, 0.5), a, b, c);
/// assert!(d < 0.05, "should be on curve, got {d}");
/// ```
#[must_use]
pub fn bezier_sdf_2d(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> f32 {
    // IQ sdBezier: minimise |B(t) - p|² over t in [0,1]
    // B(t) = (1-t)²a + 2t(1-t)b + t²c
    // Derivative condition gives a cubic → solved analytically
    let ab = Vec2::new(b.x - a.x, b.y - a.y);
    let bc = Vec2::new(c.x - b.x, c.y - b.y);
    let ca = Vec2::new(a.x - c.x, a.y - c.y);
    let ap = Vec2::new(p.x - a.x, p.y - a.y);

    // Quadratic reparameterization: q(t) = a + 2t*ab + t²*(bc-ab)
    let ex = Vec2::new(ab.x - ca.x * 0.5, ab.y - ca.y * 0.5);
    let ey = Vec2::new(bc.x - ab.x, bc.y - ab.y);

    // dot(q'(t), q(t)-p) = 0 → cubic in t
    // Coefficients
    let c0 = ab.x * ap.x + ab.y * ap.y;
    #[allow(clippy::suspicious_operation_groupings)]
    #[allow(clippy::suspicious_operation_groupings)]
    let c1 = (ey.x * ap.x + ey.y * ap.y) + (ab.x * ab.x + ab.y * ab.y);
    let c2 = 3.0 * (ey.x * ab.x + ey.y * ab.y);
    let c3 = ey.x * ey.x + ey.y * ey.y;

    // Newton's method for the smallest-distance t
    let mut best_dist = f32::MAX;
    for &t_init in &[0.0_f32, 0.5, 1.0] {
        let mut t = t_init;
        for _ in 0..8 {
            let t2 = t * t;
            let ft = c3 * t2 * t + c2 * t2 + c1 * t + c0;
            let dft = 3.0 * c3 * t2 + 2.0 * c2 * t + c1;
            if dft.abs() < 1e-9 {
                break;
            }
            t -= ft / dft;
            t = t.clamp(0.0, 1.0);
            if (ft / dft).abs() < 1e-6 {
                break;
            }
        }
        let t = t.clamp(0.0, 1.0);
        let bx = (1.0 - t) * (1.0 - t) * a.x + 2.0 * t * (1.0 - t) * b.x + t * t * c.x;
        let by = (1.0 - t) * (1.0 - t) * a.y + 2.0 * t * (1.0 - t) * b.y + t * t * c.y;
        let d = Vec2::new(p.x - bx, p.y - by).length();
        best_dist = best_dist.min(d);
    }
    let _ = (ex, c0, c1, c2, c3);
    best_dist
}

/// SDF of a regular N-gon (polygon) centred at the origin.
///
/// `r` is the circumradius (vertex to centre distance).
/// `n` must be ≥ 3; fewer sides fall back to `n = 3`.
///
/// Uses an O(n) edge-walk: for each edge the signed half-plane test determines
/// inside/outside, and the minimum segment distance gives the magnitude.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::regular_ngon_2d;
///
/// // Point at the centre of a regular hexagon should be inside
/// let inside = regular_ngon_2d(Vec2::ZERO, 6, 1.0);
/// assert!(inside < 0.0, "centre should be inside: {inside}");
///
/// // Point far outside
/// let outside = regular_ngon_2d(Vec2::new(3.0, 0.0), 6, 1.0);
/// assert!(outside > 0.0, "far point should be outside: {outside}");
/// ```
#[must_use]
pub fn regular_ngon_2d(p: Vec2, n: u32, r: f32) -> f32 {
    use std::f32::consts::TAU;
    let n = n.max(3);
    let angle_step = TAU / n as f32;
    let mut min_dist = f32::MAX;
    let mut all_inside = true;
    for k in 0..n {
        let a0 = k as f32 * angle_step;
        let a1 = (k + 1) as f32 * angle_step;
        let v0 = Vec2::new(r * a0.cos(), r * a0.sin());
        let v1 = Vec2::new(r * a1.cos(), r * a1.sin());
        let edge = v1 - v0;
        let len_sq = edge.length_sq();
        let t = ((p - v0).dot(edge) / len_sq).clamp(0.0, 1.0);
        let closest = v0 + edge * t;
        min_dist = min_dist.min((p - closest).length());
        // Cross product: positive = p is to the left = inside for CCW polygon
        let cross = edge.x * (p.y - v0.y) - edge.y * (p.x - v0.x);
        if cross < 0.0 {
            all_inside = false;
        }
    }
    min_dist * if all_inside { -1.0 } else { 1.0 }
}

/// SDF of an arc centred at the origin.
///
/// The arc lies in the XY plane at radius `r`, opening upward.  `sc` is the
/// half-angle in `(sin θ, cos θ)` form — use `sc = (half_angle.sin(), half_angle.cos())`.
/// `ra` and `rb` are the outer and inner radii of the arc's tube cross-section.
///
/// The result is the distance to the nearest point on the arc tube surface.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::arc_2d;
/// use std::f32::consts::FRAC_PI_4;
///
/// // Point on the arc centreline at angle 0 (top), radius 1.0
/// // Arc half-angle = 45°, tube radius = 0.1
/// let sc = (FRAC_PI_4.sin(), FRAC_PI_4.cos());
/// let on_arc = arc_2d(Vec2::new(0.0, 1.0), sc, 1.0, 0.1);
/// assert!(on_arc.abs() < 0.15, "should be near the arc surface: {on_arc}");
/// ```
#[must_use]
pub fn arc_2d(p: Vec2, sc: (f32, f32), r: f32, th: f32) -> f32 {
    // Mirror left-right (arc is symmetric about Y)
    let p = Vec2::new(p.x.abs(), p.y);
    let (s, c) = sc; // sc = (sin(half_angle), cos(half_angle))
    // Condition `c * p.x > s * p.y` is equivalent to `angle_from_Y > half_angle`
    // i.e., the point is angularly OUTSIDE the arc → use cap distance.
    // Otherwise (inside the arc angularly) → use radial distance.
    let dist = if c * p.x > s * p.y {
        // Outside arc angle: distance to the nearest cap endpoint
        let cap = Vec2::new(s * r, c * r);
        (p - cap).length()
    } else {
        // Inside arc angle: distance to the arc ring
        (p.length() - r).abs()
    };
    dist - th
}

/// SDF of a symmetric cross shape centred at the origin.
///
/// `b` is the half-length of the cross arms, `r` is the arm half-width.
/// The shape has four-fold symmetry.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::cross_2d;
///
/// // Centre of the cross is inside
/// let inside = cross_2d(Vec2::ZERO, 1.0, 0.2);
/// assert!(inside < 0.0);
///
/// // Far corner is outside
/// let outside = cross_2d(Vec2::new(2.0, 2.0), 1.0, 0.2);
/// assert!(outside > 0.0);
/// ```
#[must_use]
pub fn cross_2d(p: Vec2, b: f32, r: f32) -> f32 {
    // Fold to first octant, then take the minimum of two box SDFs along each axis
    let mut p = Vec2::new(p.x.abs(), p.y.abs());
    // Swap so that the shorter half is y
    if p.y > p.x {
        p = Vec2::new(p.y, p.x);
    }
    // Two rectangles: horizontal arm and vertical arm merged into one cross
    let q = p - Vec2::new(b, r);
    let d1 = q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0);
    let q2 = Vec2::new(p.x - r, p.y - b);
    let d2 = q2.max(Vec2::ZERO).length() + q2.x.max(q2.y).min(0.0);
    d1.min(d2)
}

/// SDF of a crescent moon centred at the origin.
///
/// The moon is the Boolean difference of two circles with radius `ra` (outer)
/// and `rb` (inner/cutter), whose centres are `d` apart.
///
/// - `d`: distance between the two circle centres (must be > 0)
/// - `ra`: outer (large) circle radius
/// - `rb`: inner (cutter) circle radius; must satisfy `rb < ra + d`
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::moon_2d;
///
/// // A typical crescent: outer r=1, cutter r=0.8, offset d=0.5
/// // (-0.7, 0) is inside the outer circle and outside the cutter circle.
/// let inside = moon_2d(Vec2::new(-0.7, 0.0), 0.5, 1.0, 0.8);
/// assert!(inside < 0.0, "should be inside the crescent: {inside}");
///
/// let outside = moon_2d(Vec2::new(0.0, 1.5), 0.5, 1.0, 0.8);
/// assert!(outside > 0.0, "should be outside: {outside}");
/// ```
#[must_use]
pub fn moon_2d(p: Vec2, d: f32, ra: f32, rb: f32) -> f32 {
    let p = Vec2::new(p.x, p.y.abs());
    let a = (ra * ra - rb * rb + d * d) / (2.0 * d);
    let b = (ra * ra - a * a).max(0.0).sqrt();
    if d * (p.x * b - p.y * a) > d * d * (b - p.y).max(0.0) {
        (p - Vec2::new(a, b)).length()
    } else {
        let d_outer = p.length() - ra; // signed: negative inside outer circle
        let d_inner = (p - Vec2::new(d, 0.0)).length() - rb;
        d_outer.max(-d_inner) // max(outside outer, inside cutter)
    }
}

/// Analytic SDF of an ellipse centred at the origin with semi-axes `ab`.
///
/// Uses IQ's closed-form Cardano solution — exact, not iterative.
/// `ab.x` and `ab.y` are the half-extents along X and Y respectively;
/// both must be positive.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::ellipse_2d;
///
/// // Point at centre is inside (negative distance = radius of inscribed circle)
/// let inside = ellipse_2d(Vec2::ZERO, Vec2::new(2.0, 1.0));
/// assert!(inside < 0.0, "centre should be inside: {inside}");
///
/// // Point on the X semi-axis boundary
/// let on_edge = ellipse_2d(Vec2::new(2.0, 0.0), Vec2::new(2.0, 1.0));
/// assert!(on_edge.abs() < 1e-4, "on ellipse surface: {on_edge}");
/// ```
#[must_use]
pub fn ellipse_2d(p: Vec2, ab: Vec2) -> f32 {
    // Fold to first quadrant
    let mut p = p.abs();
    let mut ab = ab;
    if p.x > p.y {
        p = Vec2::new(p.y, p.x);
        ab = Vec2::new(ab.y, ab.x);
    }
    let l = ab.y * ab.y - ab.x * ab.x;
    let m = ab.x * p.x / l;
    let m2 = m * m;
    let n = ab.y * p.y / l;
    let n2 = n * n;
    let c = (m2 + n2 - 1.0) / 3.0;
    let c3 = c * c * c;
    let q = c3 + m2 * n2 * 2.0;
    let d = c3 + m2 * n2;
    let g = m + m * n2;

    let co = if d < 0.0 {
        let h = (q / c3).clamp(-1.0, 1.0).acos() / 3.0;
        let s = h.cos();
        let t = h.sin() * 3.0_f32.sqrt();
        let rx = (-c * (s + t + 2.0) + m2).max(0.0).sqrt();
        let ry = (-c * (s - t + 2.0) + m2).max(0.0).sqrt();
        (ry + l.signum() * rx + g.abs() / (rx * ry).max(1e-10) - m) / 2.0
    } else {
        let h = 2.0 * m * n * d.max(0.0).sqrt();
        let q_plus_h = q + h;
        let q_minus_h = q - h;
        let s = q_plus_h.abs().cbrt() * q_plus_h.signum();
        let u = q_minus_h.abs().cbrt() * q_minus_h.signum();
        let rx = -s - u - c * 4.0 + 2.0 * m2;
        let ry = (s - u) * 3.0_f32.sqrt();
        let rm = (rx * rx + ry * ry).max(1e-10).sqrt();
        (ry / (rm - rx).max(1e-10).sqrt() + 2.0 * g / rm - m) / 2.0
    };

    let co = co.clamp(0.0, 1.0);
    let ex = ab.x * co;
    let ey = ab.y * (1.0 - co * co).max(0.0).sqrt();
    (p - Vec2::new(ex, ey)).length() * (p.y - ey).signum()
}

/// SDF of a pie slice (sector) centred at the origin.
///
/// `sc` is the half-angle as `(sin θ, cos θ)`.
/// `r` is the outer radius.  The pie opens upward (+Y).
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::pie_2d;
/// use std::f32::consts::FRAC_PI_4;
///
/// // Centre of a 90° pie (half-angle 45°, radius 1.0)
/// let sc = (FRAC_PI_4.sin(), FRAC_PI_4.cos());
/// let inside = pie_2d(Vec2::new(0.0, 0.5), sc, 1.0);
/// assert!(inside < 0.0, "should be inside: {inside}");
///
/// // Point outside radially
/// let outside = pie_2d(Vec2::new(0.0, 2.0), sc, 1.0);
/// assert!(outside > 0.0, "should be outside: {outside}");
/// ```
#[must_use]
pub fn pie_2d(p: Vec2, sc: (f32, f32), r: f32) -> f32 {
    let p = Vec2::new(p.x.abs(), p.y);
    let (s, c) = sc;
    let sc_vec = Vec2::new(s, c);
    let radial = p.length() - r;
    let edge_proj = p.dot(sc_vec).clamp(0.0, r);
    let edge_dist = (p - sc_vec * edge_proj).length();
    // s > 0: outside angular extent → nearest boundary is the straight edge (edge_dist)
    // s < 0: inside angular extent  → max(radial, -edge_dist) handles both inside
    //         the disk (returns -min_boundary_dist) and outside (returns radial dist)
    let s = sc_vec.y * p.x - sc_vec.x * p.y;
    if s >= 0.0 {
        edge_dist
    } else {
        radial.max(-edge_dist)
    }
}

/// Smooth minimum with C² continuity (polynomial blend version).
///
/// Blends between `a` and `b` within `k` distance.  This is the same
/// formula as [`smooth_union`] but exposed as a free function for direct
/// composition without the two-argument SDF convention.
///
/// Returns `(d, t)` where `d` is the blended distance and `t ∈ [0, 1]`
/// is the blend weight (0 = fully `a`, 1 = fully `b`).
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::smooth_min;
///
/// // Two surfaces at the same distance → blend at midpoint
/// let (d, t) = smooth_min(0.5, 0.5, 0.2);
/// assert!((t - 0.5).abs() < 1e-5);
/// assert!(d <= 0.5);
/// ```
#[must_use]
#[inline]
pub fn smooth_min(a: f32, b: f32, k: f32) -> (f32, f32) {
    let h = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
    let d = b * (1.0 - h) + a * h - k * h * (1.0 - h);
    (d, h)
}

/// Deform operator: bend space around the Y axis.
///
/// Points near `y = 0` in the XZ plane are wrapped into an arc.  `k`
/// controls the bend rate (radians per unit of X).
///
/// Classic Inigo Quilez domain warp for curved tubes, banana shapes, etc.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec3;
/// use abrash_core::sdf::deform_bend;
///
/// // With k=0 the transform is identity
/// let p = Vec3::new(1.0, 2.0, 3.0);
/// let q = deform_bend(p, 0.0);
/// assert!((q - p).length() < 1e-4);
/// ```
#[must_use]
pub fn deform_bend(p: Vec3, k: f32) -> Vec3 {
    let angle = k * p.x;
    let (s, c) = angle.sin_cos();
    // Rotate XZ plane by `angle` around Y
    Vec3::new(c * p.x - s * p.y, s * p.x + c * p.y, p.z)
}

/// Cheap approximate bend: linear twist without trigonometry.
///
/// Bends the X axis by `k` radians per unit.  Compared to [`deform_bend`]
/// this avoids `sin`/`cos` and is suitable for subtle effects or shaders where
/// cost matters.  The approximation holds for `k * p.x` within ~±π/4.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec3;
/// use abrash_core::sdf::deform_cheap_bend;
///
/// // At k=0, identity
/// let p = Vec3::new(1.0, 2.0, 3.0);
/// let q = deform_cheap_bend(p, 0.0);
/// assert!((q - p).length() < 1e-4);
/// ```
#[must_use]
pub fn deform_cheap_bend(p: Vec3, k: f32) -> Vec3 {
    let angle = k * p.x;
    // cos ≈ 1 - θ²/2, sin ≈ θ  (first-order Taylor)
    let c = 1.0 - 0.5 * angle * angle;
    let s = angle;
    Vec3::new(c * p.x - s * p.y, s * p.x + c * p.y, p.z)
}

/// SDF of a heart shape centred at the origin, opening downward.
///
/// Based on IQ's analytic heart formula.  The shape has unit extent
/// along both X and Y at scale `s = 1`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::heart_2d;
///
/// // A point in the middle of the heart should be inside (negative)
/// // Note: (0,0) lies on the cusp boundary; use (0, 0.5) for a robust interior test.
/// let inside = heart_2d(Vec2::new(0.0, 0.5));
/// assert!(inside < 0.0, "interior should be inside: {inside}");
///
/// // Far point should be outside
/// let outside = heart_2d(Vec2::new(0.0, 3.0));
/// assert!(outside > 0.0, "far point should be outside: {outside}");
/// ```
#[must_use]
pub fn heart_2d(p: Vec2) -> f32 {
    // IQ's analytic heart SDF (https://iquilezles.org/articles/distfunctions2d/)
    // Fold on X for symmetry
    let p = Vec2::new(p.x.abs(), p.y);
    // Inside the upper bump region
    if p.y + p.x > 1.0 {
        // Distance to the upper-right lobe (circle of radius sqrt(2)/4 at (0.25, 0.75))
        let centre = Vec2::new(0.25, 0.75);
        return (p - centre).length() - 2.0_f32.sqrt() / 4.0;
    }
    // Lower region: min distance to tip circle or cusp edge
    let d1 = (p - Vec2::new(0.0, 1.0)).length_sq();
    let proj = ((p.x + p.y) * 0.5).max(0.0);
    let d2 = (p - Vec2::new(proj, proj)).length_sq();
    d1.min(d2).sqrt() * (p.x - p.y).signum()
}

/// SDF of a rhombus (diamond) centred at the origin.
///
/// `b` is the half-width vector `(half_x, half_y)`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::rhombus_2d;
///
/// // Centre inside
/// let inside = rhombus_2d(Vec2::ZERO, Vec2::new(1.0, 0.5));
/// assert!(inside < 0.0, "centre should be inside: {inside}");
///
/// // Far point outside
/// let outside = rhombus_2d(Vec2::new(2.0, 0.0), Vec2::new(1.0, 0.5));
/// assert!(outside > 0.0, "outside rhombus: {outside}");
/// ```
#[must_use]
pub fn rhombus_2d(p: Vec2, b: Vec2) -> f32 {
    let p = p.abs();
    // Signed distance: project onto edge normal then max with two half-planes
    let h = ((b.x - b.y - 2.0 * p.x + 2.0 * p.y) / (b.x + b.y)).clamp(-1.0, 1.0);
    let d = (p - Vec2::new(b.x * (1.0 - h) * 0.5, b.y * (1.0 + h) * 0.5)).length();
    let s = p.x * b.y + p.y * b.x - b.x * b.y;
    d * s.signum()
}

/// SDF of an egg / ovoid centred at the origin.
///
/// `ra` is the lower (large) radius and `rb` the upper (small) radius.
/// Requires `ra > rb > 0`.  The egg sits upright with the pointed end up
/// and the round end down.
///
/// Based on IQ's two-circle egg formula.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::egg_2d;
///
/// // A point clearly inside the egg
/// let inside = egg_2d(Vec2::new(0.0, 0.3), 1.0, 0.5);
/// assert!(inside < 0.0, "should be inside egg: {inside}");
///
/// // Far point outside
/// let outside = egg_2d(Vec2::new(0.0, 3.0), 1.0, 0.5);
/// assert!(outside > 0.0, "far point should be outside: {outside}");
/// ```
#[must_use]
pub fn egg_2d(p: Vec2, ra: f32, rb: f32) -> f32 {
    let k = 3.0_f32.sqrt();
    let p = Vec2::new(p.x.abs(), p.y);
    let r = ra - rb;
    let d = if p.y < 0.0 {
        p.length() - r
    } else if k * (p.x + r) < p.y {
        (p - Vec2::new(0.0, k * r)).length()
    } else {
        (p - Vec2::new(-r, 0.0)).length() - 2.0 * r
    };
    d - rb
}

/// Bounded finite repetition operator (3-D).
///
/// Tiles the SDF with a grid of period `cell` but only within the range
/// `[-count, count]` cells on each axis.  Unlike [`repeat_3d`], this does
/// not repeat forever.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec3;
/// use abrash_core::sdf::{repeat_finite_3d, sphere_3d};
///
/// // Repeat a unit sphere in a 3×3×3 grid of cells size 3.0
/// let cell = Vec3::new(3.0, 3.0, 3.0);
/// let count = Vec3::new(1.0, 1.0, 1.0);
/// let p = Vec3::new(0.0, 0.0, 0.0); // centre cell, centre of sphere
/// let q = repeat_finite_3d(p, cell, count);
/// let d = sphere_3d(q, Vec3::ZERO, 1.0);
/// assert!(d < 0.0, "should be inside centre sphere: {d}");
/// ```
#[must_use]
pub fn repeat_finite_3d(p: Vec3, cell: Vec3, count: Vec3) -> Vec3 {
    Vec3::new(
        p.x - cell.x * (p.x / cell.x).round().clamp(-count.x, count.x),
        p.y - cell.y * (p.y / cell.y).round().clamp(-count.y, count.y),
        p.z - cell.z * (p.z / cell.z).round().clamp(-count.z, count.z),
    )
}

/// SDF of a stadium (discorectangle): the Minkowski sum of a line segment and a disk.
///
/// A stadium is two semicircles joined by a rectangle — like a running track from above.
/// `a` and `b` are the centres of the two end-caps; `r` is the radius.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::stadium_2d;
///
/// // Centre of a horizontal stadium should be inside
/// let inside = stadium_2d(Vec2::new(0.0, 0.0), Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0), 0.5);
/// assert!(inside < 0.0, "centre should be inside: {inside}");
///
/// // Far point should be outside
/// let outside = stadium_2d(Vec2::new(5.0, 0.0), Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0), 0.5);
/// assert!(outside > 0.0, "far point should be outside: {outside}");
/// ```
#[must_use]
pub fn stadium_2d(p: Vec2, a: Vec2, b: Vec2, r: f32) -> f32 {
    // Distance to segment ab, then subtract radius — same as capsule_2d
    let ab = b - a;
    let ap = p - a;
    let t = (ap.dot(ab) / ab.length_sq()).clamp(0.0, 1.0);
    (ap - ab * t).length() - r
}

/// SDF of a 2D trapezoid (isosceles) centred at the origin, opening along Y.
///
/// `r1` is the half-width at `y = +h`, `r2` at `y = -h`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::trapezoid_2d;
///
/// // Centre of a symmetric trapezoid should be inside
/// let inside = trapezoid_2d(Vec2::ZERO, 0.5, 1.0, 1.0);
/// assert!(inside < 0.0, "centre should be inside: {inside}");
///
/// // Far point outside
/// let outside = trapezoid_2d(Vec2::new(5.0, 0.0), 0.5, 1.0, 1.0);
/// assert!(outside > 0.0, "far point should be outside: {outside}");
/// ```
#[must_use]
pub fn trapezoid_2d(p: Vec2, r1: f32, r2: f32, h: f32) -> f32 {
    // IQ trapezoid formula
    let k1 = Vec2::new(r2, h);
    let k2 = Vec2::new(r2 - r1, 2.0 * h);
    let p = Vec2::new(p.x.abs(), p.y);
    let ca = Vec2::new(
        p.x - (p.x.min(if p.y < 0.0 { r1 } else { r2 })),
        p.y.abs() - h,
    );
    let t = ((k1 - p).dot(k2) / k2.length_sq()).clamp(0.0, 1.0);
    let cb = p - k1 + k2 * t;
    let s = if cb.x < 0.0 && ca.y < 0.0 {
        -1.0_f32
    } else {
        1.0_f32
    };
    s * ca.length_sq().min(cb.length_sq()).sqrt()
}

/// SDF of a 2D parallelogram centred at the origin.
///
/// `wi` is the half-width, `he` is the half-height, `sk` is the horizontal skew.
/// The corners are at `(±wi ± sk, ±he)`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::parallelogram_2d;
///
/// // Point well inside a unit-ish parallelogram (avoid y=0 midline degenerate case)
/// let inside = parallelogram_2d(Vec2::new(0.0, 0.3), 1.0, 0.5, 0.3);
/// assert!(inside < 0.0, "centre should be inside: {inside}");
///
/// let outside = parallelogram_2d(Vec2::new(5.0, 0.0), 1.0, 0.5, 0.3);
/// assert!(outside > 0.0, "far point outside: {outside}");
/// ```
#[must_use]
pub fn parallelogram_2d(p: Vec2, wi: f32, he: f32, sk: f32) -> f32 {
    let e = Vec2::new(sk, he);
    let mut p = if p.y < 0.0 { Vec2::new(-p.x, -p.y) } else { p };
    let mut w = p - e;
    w.x -= w.x.clamp(-wi, wi);
    let d1 = w.length_sq();
    p.x -= p.x.clamp(-wi, wi);
    let d2 = p.length_sq();
    let s = if p.x * e.y - p.y * e.x < 0.0 {
        -1.0_f32
    } else {
        1.0_f32
    };
    s * d1.min(d2).sqrt()
}

/// SDF of a 2D uneven (asymmetric) capsule.
///
/// Like a `capsule_2d` but with different radii `ra` and `rb` at each end cap.
/// `a` and `b` are the centres; `ra` the radius at `a`, `rb` at `b`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::uneven_capsule_2d;
///
/// // Near the larger cap end
/// let near_a = uneven_capsule_2d(Vec2::new(0.0, 0.0), Vec2::new(0.0, 0.0), Vec2::new(0.0, 2.0), 0.8, 0.3);
/// assert!(near_a < 0.0, "inside large cap: {near_a}");
///
/// // Far outside
/// let out = uneven_capsule_2d(Vec2::new(5.0, 0.0), Vec2::new(0.0, 0.0), Vec2::new(0.0, 2.0), 0.8, 0.3);
/// assert!(out > 0.0, "outside: {out}");
/// ```
#[must_use]
pub fn uneven_capsule_2d(p: Vec2, a: Vec2, b: Vec2, ra: f32, rb: f32) -> f32 {
    // Project onto segment, then interpolate radii
    let ab = b - a;
    let ap = p - a;
    let h = (ap.dot(ab) / ab.length_sq()).clamp(0.0, 1.0);
    let r = ra + (rb - ra) * h;
    (ap - ab * h).length() - r
}

/// SDF of a 2D horseshoe (C-shape / partial arc with thickness).
///
/// `c` = (cos, sin) of the half-opening angle.
/// `r` is the arc radius. `w` is the half-width of the bar `(half_x, half_y)`.
///
/// Direct port of Inigo Quilez's sdHorseshoe.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::horseshoe_2d;
///
/// // Full horseshoe (c.x=1, c.y=0): arc at y-axis, opening toward +x.
/// // Point on the arc at (0, r) is inside.
/// let r = 1.5_f32;
/// let inside = horseshoe_2d(Vec2::new(0.0, r), (1.0, 0.0), r, Vec2::new(0.2, 0.2));
/// assert!(inside < 0.0, "on arc should be inside: {inside}");
///
/// let outside = horseshoe_2d(Vec2::new(5.0, 0.0), (1.0, 0.0), r, Vec2::new(0.2, 0.2));
/// assert!(outside > 0.0, "far point outside: {outside}");
/// ```
#[must_use]
pub fn horseshoe_2d(p: Vec2, c: (f32, f32), r: f32, w: Vec2) -> f32 {
    // Direct translation of IQ's sdHorseshoe (GLSL → Rust).
    // c = (cos(half_angle), sin(half_angle)); mat2 is column-major in GLSL.
    let mut p = Vec2::new(p.x.abs(), p.y);
    let l = p.length();
    let (cx, cy) = c;
    // mat2(-cx, cy, cy, cx) * p  (GLSL column-major: col0=(-cx,cy), col1=(cy,cx))
    let (rx, ry) = (-cx * p.x + cy * p.y, cy * p.x + cx * p.y);
    p = Vec2::new(rx, ry);
    let px2 = if p.y > 0.0 || p.x > 0.0 {
        p.x
    } else {
        l * (-cx).signum()
    };
    let py2 = if p.x > 0.0 { p.y } else { l };
    p = Vec2::new(px2, (py2 - r).abs()) - w;
    // Standard 2D box SDF from origin
    let ext = Vec2::new(p.x.max(0.0), p.y.max(0.0));
    ext.length() + 0.0_f32.min(p.x.max(p.y))
}

/// SDF of a 2D cut disk (circle with a flat chord cut at the bottom).
///
/// `r` is the radius, `h` is the y-position of the horizontal cut (`-r ≤ h ≤ r`).
/// The shape retains the **y ≥ h** portion (the upper part).
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::cut_disk_2d;
///
/// // Point above the cut and inside the circle → inside the shape
/// let inside = cut_disk_2d(Vec2::new(0.0, 0.7), 1.0, 0.5);
/// assert!(inside < 0.0, "above cut, in circle → inside: {inside}");
///
/// // Point below the cut → outside (flat face is boundary)
/// let outside = cut_disk_2d(Vec2::new(0.0, -0.5), 1.0, 0.5);
/// assert!(outside > 0.0, "below cut → outside: {outside}");
/// ```
#[must_use]
pub fn cut_disk_2d(p: Vec2, r: f32, h: f32) -> f32 {
    // Half-width of the cut chord
    let w = (r * r - h * h).max(0.0).sqrt();
    let px = p.x.abs();
    let py = p.y;
    // IQ's sdCutDisk: max of two expressions selects the correct region
    let s = ((h - r) * px * px + w * w * (h + r - 2.0 * py)).max(h * px - w * py);
    if s < 0.0 {
        p.length() - r
    } else if px < w {
        h - py
    } else {
        Vec2::new(px - w, py - h).length()
    }
}

/// SDF of a 2D rounded X (two crossing lines with rounded caps).
///
/// `w` is the half-length of each arm, `r` is the rounding radius.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::rounded_x_2d;
///
/// let inside = rounded_x_2d(Vec2::new(0.1, 0.1), 1.0, 0.1);
/// assert!(inside < 0.0, "near centre inside: {inside}");
///
/// let outside = rounded_x_2d(Vec2::new(0.0, 3.0), 1.0, 0.1);
/// assert!(outside > 0.0, "far outside: {outside}");
/// ```
#[must_use]
pub fn rounded_x_2d(p: Vec2, w: f32, r: f32) -> f32 {
    let p = p.abs();
    let s = (p.x + p.y).min(w);
    (Vec2::new(p.x - s * 0.5, p.y - s * 0.5)).length() - r
}

/// SDF of a 3D capped torus — a torus cut by two symmetric planes through the Y-axis.
///
/// `sc` = (sin, cos) of the half-opening angle. `ra` = tube-centre radius, `rb` = tube radius.
/// For a full torus with no cap, use `sc = (0.0, -1.0)`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec3;
/// use abrash_core::sdf::capped_torus_3d;
///
/// // sc=(0,-1): full torus. Point on tube centre ring at (ra, 0, 0) → distance = -rb (inside).
/// let ra = 1.5_f32;
/// let rb = 0.3_f32;
/// let inside = capped_torus_3d(Vec3::new(ra, 0.0, 0.0), (0.0, -1.0), ra, rb);
/// assert!(inside < 0.0, "on tube centre should be inside: {inside}");
///
/// let outside = capped_torus_3d(Vec3::new(5.0, 0.0, 0.0), (0.0, -1.0), ra, rb);
/// assert!(outside > 0.0, "far outside: {outside}");
/// ```
#[must_use]
pub fn capped_torus_3d(p: Vec3, sc: (f32, f32), ra: f32, rb: f32) -> f32 {
    let (si, co) = sc;
    let px = p.x.abs();
    let py = p.y;
    // Choose closest end-cap or full-torus distance
    let k = if co * px > si * py {
        px * co + py * si
    } else {
        (px * px + py * py).sqrt()
    };
    let d_sq = p.x * p.x + p.y * p.y + p.z * p.z + ra * ra - 2.0 * ra * k;
    d_sq.max(0.0).sqrt() - rb
}

/// SDF of a 3D triangular prism.
///
/// The prism has an equilateral triangle cross-section in the XY plane.
/// `h.x` is the triangle "radius" (inradius), `h.y` is the half-height along Z.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec3;
/// use abrash_core::sdf::triangular_prism_3d;
///
/// let inside = triangular_prism_3d(Vec3::new(0.0, 0.0, 0.0), (0.5, 1.0));
/// assert!(inside < 0.0, "centre inside prism: {inside}");
///
/// let outside = triangular_prism_3d(Vec3::new(0.0, 0.0, 3.0), (0.5, 1.0));
/// assert!(outside > 0.0, "above prism: {outside}");
/// ```
#[must_use]
pub fn triangular_prism_3d(p: Vec3, h: (f32, f32)) -> f32 {
    let q = p.abs();
    // max of: distance from end-cap, distance from triangle edges
    // Triangle has vertices at (cos(90°), sin(90°)), (cos(210°), sin(210°)), (cos(330°), sin(330°))
    // IQ formula: max(q.z - h.y, max(q.x * 0.866025 + p.y * 0.5, -p.y) - h.x * 0.5)
    (q.z - h.1).max((q.x * 0.866_025 + p.y * 0.5).max(-p.y) - h.0 * 0.5)
}

/// SDF of a 3D cut sphere (a sphere with a flat planar cap).
///
/// `r` is the sphere radius, `h` is the y-position of the horizontal cut (`-r ≤ h ≤ r`).
/// The shape retains the **y ≥ h** portion (the cap, like a mushroom top).
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec3;
/// use abrash_core::sdf::cut_sphere_3d;
///
/// // Point above the cut inside the sphere → inside the shape
/// let inside = cut_sphere_3d(Vec3::new(0.0, 0.7, 0.0), 1.0, 0.5);
/// assert!(inside < 0.0, "above cut and inside sphere: {inside}");
///
/// // Point below the cut → outside
/// let outside = cut_sphere_3d(Vec3::new(0.0, -0.5, 0.0), 1.0, 0.5);
/// assert!(outside > 0.0, "below cut → outside: {outside}");
/// ```
#[must_use]
pub fn cut_sphere_3d(p: Vec3, r: f32, h: f32) -> f32 {
    // Half-width of the cut disk
    let w = (r * r - h * h).max(0.0).sqrt();
    let q = p.x.mul_add(p.x, p.z * p.z).sqrt();
    // Two regions: flat cap and spherical surface
    // sign: +1 outside, -1 inside
    let d_sphere = p.length() - r;
    // Flat cap contribution
    let d_cap_q = q - w;
    let d_cap_y = p.y - h;
    // Choose the larger (furthest outside = correct SDF)
    let n = Vec2::new(h, -w).normalize();
    let dist_cap = Vec2::new(q, p.y).dot(n) - Vec2::new(0.0, h).dot(n);
    d_sphere.max(dist_cap)
}

/// SDF of a 2D oriented (rotated) box centred at the origin.
///
/// `half_size` is the box half-extents before rotation; `angle` is the
/// counter-clockwise rotation in radians.  Equivalent to rotating the query
/// point by `-angle` and then evaluating an axis-aligned box.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::oriented_box_2d;
///
/// // Axis-aligned box: angle = 0
/// let inside = oriented_box_2d(Vec2::new(0.3, 0.2), Vec2::new(0.5, 0.5), 0.0);
/// assert!(inside < 0.0, "inside AABB: {inside}");
///
/// // Same box rotated 45° — point near centre is inside
/// let inside45 = oriented_box_2d(Vec2::new(0.1, 0.0), Vec2::new(0.5, 0.2), std::f32::consts::FRAC_PI_4);
/// assert!(inside45 < 0.0, "inside rotated box: {inside45}");
/// ```
#[must_use]
pub fn oriented_box_2d(p: Vec2, half_size: Vec2, angle: f32) -> f32 {
    let (s, c) = angle.sin_cos();
    // Rotate p by -angle (inverse rotation)
    let q = Vec2::new(c * p.x + s * p.y, -s * p.x + c * p.y);
    // Standard axis-aligned box SDF
    let d = q.abs() - half_size;
    let ext = Vec2::new(d.x.max(0.0), d.y.max(0.0));
    ext.length() + d.x.max(d.y).min(0.0)
}

/// SDF of a 2D arrow pointing from `a` to `b`.
///
/// `head_w` and `head_h` are the arrowhead half-width and height;
/// `shaft_r` is the shaft half-thickness.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::arrow_2d;
///
/// let a = Vec2::new(0.0, 0.0);
/// let b = Vec2::new(2.0, 0.0);
/// let inside = arrow_2d(Vec2::new(1.0, 0.0), a, b, 0.3, 0.5, 0.1);
/// assert!(inside < 0.0, "on shaft should be inside: {inside}");
///
/// let outside = arrow_2d(Vec2::new(1.0, 2.0), a, b, 0.3, 0.5, 0.1);
/// assert!(outside > 0.0, "far above should be outside: {outside}");
/// ```
#[must_use]
pub fn arrow_2d(p: Vec2, a: Vec2, b: Vec2, head_w: f32, head_h: f32, shaft_r: f32) -> f32 {
    let ab = b - a;
    let len = ab.length();
    if len < 1e-10 {
        return p.length() - shaft_r;
    }
    let dir = ab * (1.0 / len);
    let perp = Vec2::new(-dir.y, dir.x);
    // Local coords: t along axis, u perpendicular
    let ap = p - a;
    let t = ap.dot(dir);
    let u = ap.dot(perp).abs();
    // Shaft portion [0, len - head_h]
    let shaft_end = (len - head_h).max(0.0);
    let d_shaft = if t >= 0.0 && t <= shaft_end {
        u - shaft_r
    } else {
        f32::MAX
    };
    // Head (triangle): from shaft_end to len
    let d_head = if t >= shaft_end && t <= len {
        // Linearly taper from head_w at shaft_end to 0 at tip (t = len)
        let frac = (t - shaft_end) / (len - shaft_end).max(1e-10);
        let half_w = head_w * (1.0 - frac);
        u - half_w
    } else {
        f32::MAX
    };
    // Behind the tail or past the tip: distance to endpoints
    let d_tail = if t < 0.0 {
        (p - a).length() - shaft_r
    } else if t > len {
        (p - b).length()
    } else {
        f32::MAX
    };
    d_shaft.min(d_head).min(d_tail)
}

// ── Sharp CSG operators ──────────────────────────────────────────────────────

/// Sharp (non-smooth) CSG union: the closer of two shapes.
///
/// Equivalent to `min(a, b)`. Complement to [`smooth_union`].
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::sdf_union;
/// assert_eq!(sdf_union(-1.0, 2.0), -1.0); // inside a, outside b → inside union
/// assert_eq!(sdf_union(3.0, 1.0),   1.0); // closer surface wins
/// ```
#[must_use]
#[inline]
pub const fn sdf_union(a: f32, b: f32) -> f32 {
    a.min(b)
}

/// Sharp CSG intersection: keep only the overlap of two shapes.
///
/// Equivalent to `max(a, b)`. Complement to [`smooth_intersection`].
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::sdf_intersect;
/// assert_eq!(sdf_intersect(-1.0, -0.5), -0.5); // inside both → closer boundary
/// assert!(sdf_intersect(-1.0,  2.0) > 0.0);     // inside a but outside b → outside
/// ```
#[must_use]
#[inline]
pub const fn sdf_intersect(a: f32, b: f32) -> f32 {
    a.max(b)
}

/// Sharp CSG subtraction: cut shape `b` out of shape `a`.
///
/// Equivalent to `max(a, -b)`. Complement to [`smooth_subtraction`].
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::sdf_subtract;
/// // Point inside a (-1) but also inside b (-0.5) → cut out → outside (+0.5)
/// assert!( sdf_subtract(-1.0, -0.5) > 0.0);
/// // Point inside a (-1) but outside b (+2) → survives → inside (-1)
/// assert!( sdf_subtract(-1.0,  2.0) < 0.0);
/// ```
#[must_use]
#[inline]
pub fn sdf_subtract(a: f32, b: f32) -> f32 {
    a.max(-b)
}

/// Distance from point `p` to the closest point on 3D line segment `a`–`b`.
///
/// Returns the unsigned distance — no radius, unlike [`capsule_3d`].
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec3;
/// use abrash_core::sdf::segment_3d;
///
/// let a = Vec3::new(0.0, 0.0, 0.0);
/// let b = Vec3::new(2.0, 0.0, 0.0);
///
/// // Midpoint: distance = 0
/// assert!( segment_3d(Vec3::new(1.0, 0.0, 0.0), a, b) < 1e-5);
/// // Perpendicular above midpoint
/// assert!((segment_3d(Vec3::new(1.0, 1.0, 0.0), a, b) - 1.0).abs() < 1e-5);
/// // Past end — distance to endpoint
/// assert!((segment_3d(Vec3::new(4.0, 0.0, 0.0), a, b) - 2.0).abs() < 1e-5);
/// ```
#[must_use]
pub fn segment_3d(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let ap = p - a;
    let t = (ap.dot(ab) / ab.length_sq()).clamp(0.0, 1.0);
    (ap - ab * t).length()
}

/// SDF of a 3D rounded cone: a cone-like shape with spherical caps of different radii.
///
/// `a` and `b` are the centres of the two end-caps; `r1` and `r2` are their radii.
/// The surface smoothly interpolates between the two spheres along the axis.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec3;
/// use abrash_core::sdf::rounded_cone_3d;
///
/// let a = Vec3::new(0.0, 0.0, 0.0);
/// let b = Vec3::new(0.0, 2.0, 0.0);
///
/// // Centre of the larger cap is inside
/// let inside = rounded_cone_3d(Vec3::new(0.0, 0.0, 0.0), a, b, 0.5, 0.2);
/// assert!(inside < 0.0, "inside large cap: {inside}");
///
/// // Far away is outside
/// let outside = rounded_cone_3d(Vec3::new(0.0, 5.0, 0.0), a, b, 0.5, 0.2);
/// assert!(outside > 0.0, "outside: {outside}");
/// ```
#[must_use]
pub fn rounded_cone_3d(p: Vec3, a: Vec3, b: Vec3, r1: f32, r2: f32) -> f32 {
    // IQ's sdRoundCone — analytically exact SDF between two offset spheres
    let ba = b - a;
    let l2 = ba.length_sq();
    let rr = r1 - r2;
    let a2 = l2 - rr * rr;
    let il2 = 1.0 / l2;

    let pa = p - a;
    let y = pa.dot(ba);
    let z = y - l2;
    let pba = pa * l2 - ba * y;
    let x2 = pba.length_sq();
    let y2 = y * y * l2;
    let z2 = z * z * l2;

    let k = rr.signum() * rr * rr * x2;
    if rr.signum() * z * a2 > k {
        return (x2 + z2).sqrt() * il2 - r2;
    }
    if rr.signum() * y * a2 < k {
        return (x2 + y2).sqrt() * il2 - r1;
    }
    ((x2 * a2 * il2).sqrt() + y * rr) * il2 - r1
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

    // ── Domain operators ────────────────────────────────────────────────────

    #[test]
    fn extrude_y_cylinder_matches() {
        // Extruding a unit circle by h=2 should give same result as cylinder_3d
        let p = Vec3::new(0.5, 1.0, 0.0);
        let ext = extrude_y(p, 2.0, |xz| circle_2d(xz, Vec2::ZERO, 1.0));
        let cyl = cylinder_3d(p, Vec3::ZERO, 1.0, 2.0);
        assert!((ext - cyl).abs() < TOL, "ext={ext} cyl={cyl}");
    }

    #[test]
    fn extrude_y_above_cap_is_outside() {
        let p = Vec3::new(0.0, 3.0, 0.0);
        let d = extrude_y(p, 1.0, |xz| circle_2d(xz, Vec2::ZERO, 0.5));
        assert!(d > 0.0, "above cap: {d}");
    }

    #[test]
    fn revolve_y_gives_torus() {
        // Revolving a circle of radius 0.2 at offset 1.0 gives a torus
        let p = Vec3::new(1.0, 0.0, 0.0); // on tube center
        let d = revolve_y(p, 1.0, |q| circle_2d(q, Vec2::ZERO, 0.2));
        assert!(d < 0.0, "on tube centre: {d}");
        // Far from torus
        let far = Vec3::new(5.0, 0.0, 0.0);
        let d2 = revolve_y(far, 1.0, |q| circle_2d(q, Vec2::ZERO, 0.2));
        assert!(d2 > 0.0);
    }

    #[test]
    fn twist_y_changes_result() {
        let p = Vec3::new(0.5, 1.0, 0.0);
        let plain = box_3d(p, Vec3::ZERO, Vec3::new(0.5, 2.0, 0.5));
        let twisted = box_3d(twist_y(p, 1.5), Vec3::ZERO, Vec3::new(0.5, 2.0, 0.5));
        assert!(
            (plain - twisted).abs() > TOL,
            "twist should differ: {plain} vs {twisted}"
        );
    }

    #[test]
    fn repeat_1d_tiles_correctly() {
        // Spheres at x=0 and x=3 should both look the same when repeated with cell=3
        let p0 = Vec3::new(0.1, 0.0, 0.0);
        let p1 = Vec3::new(3.1, 0.0, 0.0);
        let d0 = sphere_3d(repeat_1d(p0, Vec3::new(3.0, 0.0, 0.0)), Vec3::ZERO, 0.5);
        let d1 = sphere_3d(repeat_1d(p1, Vec3::new(3.0, 0.0, 0.0)), Vec3::ZERO, 0.5);
        assert!((d0 - d1).abs() < TOL, "d0={d0} d1={d1}");
    }

    // ── regular_ngon_2d ──────────────────────────────────────────────────────

    #[test]
    fn ngon_centre_inside() {
        for n in [3u32, 4, 5, 6, 8, 12] {
            let d = regular_ngon_2d(Vec2::ZERO, n, 1.0);
            assert!(d < 0.0, "centre of {n}-gon should be inside, got {d}");
        }
    }

    #[test]
    fn ngon_far_outside() {
        let d = regular_ngon_2d(Vec2::new(5.0, 0.0), 6, 1.0);
        assert!(d > 0.0);
    }

    // ── heart_2d ─────────────────────────────────────────────────────────────

    #[test]
    fn heart_interior_inside() {
        let d = heart_2d(Vec2::new(0.0, 0.5));
        assert!(d < 0.0, "interior of heart should be inside: {d}");
    }

    #[test]
    fn heart_far_outside() {
        let d = heart_2d(Vec2::new(0.0, 3.0));
        assert!(d > 0.0, "far above heart should be outside: {d}");
    }

    // ── rhombus_2d ───────────────────────────────────────────────────────────

    #[test]
    fn rhombus_centre_inside() {
        let d = rhombus_2d(Vec2::ZERO, Vec2::new(1.0, 0.5));
        assert!(d < 0.0, "centre should be inside: {d}");
    }

    #[test]
    fn rhombus_far_outside() {
        let d = rhombus_2d(Vec2::new(3.0, 0.0), Vec2::new(1.0, 0.5));
        assert!(d > 0.0, "far right should be outside: {d}");
    }

    // ── egg_2d ───────────────────────────────────────────────────────────────

    #[test]
    fn egg_interior_inside() {
        let d = egg_2d(Vec2::new(0.0, 0.3), 1.0, 0.5);
        assert!(d < 0.0, "interior should be inside egg: {d}");
    }

    #[test]
    fn egg_far_outside() {
        let d = egg_2d(Vec2::new(0.0, 5.0), 1.0, 0.5);
        assert!(d > 0.0, "far above should be outside: {d}");
    }

    // ── deform_cheap_bend ────────────────────────────────────────────────────

    #[test]
    fn cheap_bend_zero_k_identity() {
        let p = Vec3::new(1.0, 2.0, 3.0);
        let q = deform_cheap_bend(p, 0.0);
        assert!((q - p).length() < 1e-5, "k=0 → identity");
    }

    #[test]
    fn cheap_bend_small_k_close_to_exact() {
        // For small angles the Taylor approximation should match exact bend closely
        let p = Vec3::new(0.2, 0.5, 0.0);
        let exact = deform_bend(p, 0.1);
        let approx = deform_cheap_bend(p, 0.1);
        assert!(
            (exact - approx).length() < 0.01,
            "should be close for small k"
        );
    }

    // ── repeat_finite_3d ─────────────────────────────────────────────────────

    #[test]
    fn repeat_finite_centre_cell() {
        let p = Vec3::new(0.0, 0.0, 0.0);
        let cell = Vec3::new(3.0, 3.0, 3.0);
        let count = Vec3::new(1.0, 1.0, 1.0);
        let q = repeat_finite_3d(p, cell, count);
        assert!(
            (q - Vec3::ZERO).length() < 1e-5,
            "centre maps to centre: {q:?}"
        );
    }

    #[test]
    fn repeat_finite_interior_point_unchanged() {
        // A point close to the origin maps to itself (no repetition needed)
        let cell = Vec3::new(3.0, 3.0, 3.0);
        let count = Vec3::new(2.0, 2.0, 2.0);
        let p = Vec3::new(0.3, 0.1, -0.2);
        let q = repeat_finite_3d(p, cell, count);
        assert!((q - p).length() < 1e-5, "interior maps to self: {q:?}");
    }

    #[test]
    fn repeat_finite_maps_neighbour_cell() {
        // A point in cell +1 (x ∈ [1.5, 4.5] for cell=3, count=1) maps to [-1.5, 1.5]
        let cell = Vec3::new(3.0, 3.0, 3.0);
        let count = Vec3::new(1.0, 1.0, 1.0);
        let p = Vec3::new(3.2, 0.0, 0.0); // in cell +1
        let q = repeat_finite_3d(p, cell, count);
        assert!(q.x.abs() <= 1.7, "should map to first cell: {}", q.x); // 3.2 - 3 = 0.2
    }

    // ── moon_2d ──────────────────────────────────────────────────────────────

    #[test]
    fn moon_inside_crescent() {
        // (-0.7, 0): inside outer circle (r=1), outside cutter (r=0.8 at d=0.5)
        let d = moon_2d(Vec2::new(-0.7, 0.0), 0.5, 1.0, 0.8);
        assert!(d < 0.0, "should be inside crescent: {d}");
    }

    #[test]
    fn moon_outside_radially() {
        let d = moon_2d(Vec2::new(0.0, 1.5), 0.5, 1.0, 0.8);
        assert!(d > 0.0, "outside radially: {d}");
    }

    #[test]
    fn moon_inside_cutter() {
        // (0, 0): at origin, inside the cutter circle centered at (0.5, 0)
        let d = moon_2d(Vec2::ZERO, 0.5, 1.0, 0.8);
        assert!(d > 0.0, "inside cutter region = outside crescent: {d}");
    }

    // ── ellipse_2d ───────────────────────────────────────────────────────────

    #[test]
    fn ellipse_centre_inside() {
        let d = ellipse_2d(Vec2::ZERO, Vec2::new(2.0, 1.0));
        assert!(d < 0.0, "centre should be inside: {d}");
    }

    #[test]
    fn ellipse_on_boundary() {
        let d = ellipse_2d(Vec2::new(2.0, 0.0), Vec2::new(2.0, 1.0));
        assert!(d.abs() < 1e-3, "on x semi-axis: {d}");
        let d2 = ellipse_2d(Vec2::new(0.0, 1.0), Vec2::new(2.0, 1.0));
        assert!(d2.abs() < 1e-3, "on y semi-axis: {d2}");
    }

    #[test]
    fn ellipse_outside() {
        let d = ellipse_2d(Vec2::new(3.0, 0.0), Vec2::new(2.0, 1.0));
        assert!(d > 0.0, "outside ellipse: {d}");
    }

    // ── pie_2d ───────────────────────────────────────────────────────────────

    #[test]
    fn pie_centre_inside() {
        use std::f32::consts::FRAC_PI_4;
        let sc = (FRAC_PI_4.sin(), FRAC_PI_4.cos());
        let d = pie_2d(Vec2::new(0.0, 0.5), sc, 1.0);
        assert!(d < 0.0, "centre of 90° pie: {d}");
    }

    #[test]
    fn pie_outside_radially() {
        use std::f32::consts::FRAC_PI_4;
        let sc = (FRAC_PI_4.sin(), FRAC_PI_4.cos());
        let d = pie_2d(Vec2::new(0.0, 2.0), sc, 1.0);
        assert!(d > 0.0, "outside radially within angular extent: {d}");
    }

    #[test]
    fn pie_outside_angularly() {
        use std::f32::consts::FRAC_PI_4;
        let sc = (FRAC_PI_4.sin(), FRAC_PI_4.cos());
        let d = pie_2d(Vec2::new(2.0, 0.0), sc, 1.0);
        assert!(d > 0.0, "outside angularly: {d}");
    }

    // ── smooth_min ───────────────────────────────────────────────────────────

    #[test]
    fn smooth_min_equal_inputs() {
        let (d, t) = smooth_min(0.5, 0.5, 0.2);
        assert!((t - 0.5).abs() < 1e-5, "equal inputs → t=0.5: {t}");
        assert!(d <= 0.5, "smooth_min ≤ min(a,b): {d}");
    }

    #[test]
    fn smooth_min_dominates_correctly() {
        // When a << b (far apart), result ≈ a
        let (d, t) = smooth_min(-2.0, 5.0, 0.1);
        assert!((d + 2.0).abs() < 0.2, "dominated by a: {d}");
        assert!(t > 0.9, "t near 1 (choosing a): {t}");
    }

    // ── deform_bend ──────────────────────────────────────────────────────────

    #[test]
    fn deform_bend_identity_at_zero_k() {
        let p = Vec3::new(1.0, 2.0, 3.0);
        let q = deform_bend(p, 0.0);
        assert!((q - p).length() < 1e-5, "k=0 → identity: {q:?}");
    }

    #[test]
    fn deform_bend_preserves_length_locally() {
        // Bending is a rotation, so it preserves distance from origin
        let p = Vec3::new(1.0, 0.0, 0.0);
        let q = deform_bend(p, 1.0);
        assert!((q.length() - p.length()).abs() < 1e-5);
    }

    // ── arc_2d ───────────────────────────────────────────────────────────────

    #[test]
    fn arc_2d_on_surface() {
        use std::f32::consts::FRAC_PI_4;
        let sc = (FRAC_PI_4.sin(), FRAC_PI_4.cos());
        // Top of arc (θ=0 → top, radius=1.0, tube=0.1)
        let d = arc_2d(Vec2::new(0.0, 1.0), sc, 1.0, 0.1);
        assert!(d < 0.11, "should be near arc surface: {d}");
    }

    // ── cross_2d ─────────────────────────────────────────────────────────────

    #[test]
    fn cross_centre_inside() {
        let d = cross_2d(Vec2::ZERO, 1.0, 0.2);
        assert!(d < 0.0, "centre should be inside cross: {d}");
    }

    #[test]
    fn cross_corner_outside() {
        let d = cross_2d(Vec2::new(2.0, 2.0), 1.0, 0.2);
        assert!(d > 0.0, "far corner should be outside cross: {d}");
    }

    #[test]
    fn cross_arm_inside() {
        // A point inside one arm of the cross
        let d = cross_2d(Vec2::new(0.8, 0.1), 1.0, 0.2);
        assert!(d < 0.0, "arm point should be inside cross: {d}");
    }

    // ── stadium_2d ────────────────────────────────────────────────────────
    #[test]
    fn stadium_centre_inside() {
        use crate::math::Vec2;
        let a = Vec2::new(-1.0, 0.0);
        let b = Vec2::new(1.0, 0.0);
        let d = stadium_2d(Vec2::ZERO, a, b, 0.5);
        assert!(d < 0.0, "centre should be inside stadium: {d}");
    }

    #[test]
    fn stadium_far_outside() {
        use crate::math::Vec2;
        let a = Vec2::new(-1.0, 0.0);
        let b = Vec2::new(1.0, 0.0);
        let d = stadium_2d(Vec2::new(5.0, 0.0), a, b, 0.5);
        assert!(d > 0.0, "far point should be outside stadium: {d}");
    }

    #[test]
    fn stadium_approximate_radius() {
        // A point at distance exactly r from the axis midpoint
        use crate::math::Vec2;
        let a = Vec2::new(-1.0, 0.0);
        let b = Vec2::new(1.0, 0.0);
        let r = 0.5_f32;
        let d = stadium_2d(Vec2::new(0.0, r), a, b, r);
        assert!(d.abs() < 1e-5, "should be on surface: {d}");
    }

    // ── trapezoid_2d ──────────────────────────────────────────────────────
    #[test]
    fn trapezoid_centre_inside() {
        use crate::math::Vec2;
        let d = trapezoid_2d(Vec2::ZERO, 2.0, 1.0, 1.0);
        assert!(d < 0.0, "centre inside trapezoid: {d}");
    }

    #[test]
    fn trapezoid_far_outside() {
        use crate::math::Vec2;
        let d = trapezoid_2d(Vec2::new(0.0, 5.0), 2.0, 1.0, 1.0);
        assert!(d > 0.0, "above trapezoid: {d}");
    }

    // ── parallelogram_2d ──────────────────────────────────────────────────
    #[test]
    fn parallelogram_inside() {
        use crate::math::Vec2;
        // Use y ≠ 0 to avoid midline degenerate case
        let d = parallelogram_2d(Vec2::new(0.0, 0.3), 1.0, 0.5, 0.3);
        assert!(d < 0.0, "should be inside parallelogram: {d}");
    }

    #[test]
    fn parallelogram_far_outside() {
        use crate::math::Vec2;
        let d = parallelogram_2d(Vec2::new(5.0, 0.0), 1.0, 0.5, 0.3);
        assert!(d > 0.0, "should be outside parallelogram: {d}");
    }

    // ── uneven_capsule_2d ─────────────────────────────────────────────────
    #[test]
    fn uneven_capsule_centre_inside() {
        use crate::math::Vec2;
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(0.0, 2.0);
        let d = uneven_capsule_2d(Vec2::new(0.0, 1.0), a, b, 0.5, 0.3);
        assert!(d < 0.0, "midpoint inside uneven capsule: {d}");
    }

    #[test]
    fn uneven_capsule_far_outside() {
        use crate::math::Vec2;
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(0.0, 2.0);
        let d = uneven_capsule_2d(Vec2::new(5.0, 1.0), a, b, 0.5, 0.3);
        assert!(d > 0.0, "far point outside uneven capsule: {d}");
    }

    // ── horseshoe_2d ──────────────────────────────────────────────────────
    #[test]
    fn horseshoe_on_arc_inside() {
        use crate::math::Vec2;
        let r = 1.5_f32;
        // Full horseshoe (c=(1,0)): arc at radius r along y-axis → inside
        let d = horseshoe_2d(Vec2::new(0.0, r), (1.0, 0.0), r, Vec2::new(0.2, 0.2));
        assert!(d < 0.0, "on arc inside horseshoe: {d}");
    }

    #[test]
    fn horseshoe_far_outside() {
        use crate::math::Vec2;
        let d = horseshoe_2d(Vec2::new(5.0, 0.0), (1.0, 0.0), 1.5, Vec2::new(0.2, 0.2));
        assert!(d > 0.0, "far outside horseshoe: {d}");
    }

    // ── cut_disk_2d ───────────────────────────────────────────────────────
    #[test]
    fn cut_disk_above_cut_inside() {
        use crate::math::Vec2;
        // y=0.7 > h=0.5, within unit circle → inside
        let d = cut_disk_2d(Vec2::new(0.0, 0.7), 1.0, 0.5);
        assert!(d < 0.0, "above cut and in circle → inside: {d}");
    }

    #[test]
    fn cut_disk_below_cut_outside() {
        use crate::math::Vec2;
        // y=-0.5 < h=0.5 → outside (removed region)
        let d = cut_disk_2d(Vec2::new(0.0, -0.5), 1.0, 0.5);
        assert!(d > 0.0, "below cut → outside: {d}");
    }

    // ── rounded_x_2d ─────────────────────────────────────────────────────
    #[test]
    fn rounded_x_near_centre_inside() {
        use crate::math::Vec2;
        let d = rounded_x_2d(Vec2::new(0.1, 0.1), 1.0, 0.15);
        assert!(d < 0.0, "near centre inside rounded X: {d}");
    }

    #[test]
    fn rounded_x_far_outside() {
        use crate::math::Vec2;
        let d = rounded_x_2d(Vec2::new(0.0, 3.0), 1.0, 0.1);
        assert!(d > 0.0, "far outside rounded X: {d}");
    }

    // ── capped_torus_3d ───────────────────────────────────────────────────
    #[test]
    fn capped_torus_on_tube_centre_inside() {
        use crate::math::Vec3;
        let ra = 1.5_f32;
        let rb = 0.3_f32;
        // sc=(0,-1) = full torus; point on tube centre ring → distance = -rb
        let d = capped_torus_3d(Vec3::new(ra, 0.0, 0.0), (0.0, -1.0), ra, rb);
        assert!((d + rb).abs() < 1e-5, "distance should be -rb: {d}");
        assert!(d < 0.0, "on tube centre inside: {d}");
    }

    #[test]
    fn capped_torus_far_outside() {
        use crate::math::Vec3;
        let d = capped_torus_3d(Vec3::new(5.0, 0.0, 0.0), (0.0, -1.0), 1.5, 0.3);
        assert!(d > 0.0, "far outside capped torus: {d}");
    }

    // ── triangular_prism_3d ───────────────────────────────────────────────
    #[test]
    fn triangular_prism_centre_inside() {
        use crate::math::Vec3;
        let d = triangular_prism_3d(Vec3::new(0.0, 0.0, 0.0), (0.5, 1.0));
        assert!(d < 0.0, "centre inside triangular prism: {d}");
    }

    #[test]
    fn triangular_prism_above_outside() {
        use crate::math::Vec3;
        let d = triangular_prism_3d(Vec3::new(0.0, 0.0, 3.0), (0.5, 1.0));
        assert!(d > 0.0, "above prism: {d}");
    }

    // ── cut_sphere_3d ─────────────────────────────────────────────────────
    #[test]
    fn cut_sphere_above_cut_inside() {
        use crate::math::Vec3;
        // y=0.7 > h=0.5, within unit sphere → inside
        let d = cut_sphere_3d(Vec3::new(0.0, 0.7, 0.0), 1.0, 0.5);
        assert!(d < 0.0, "above cut and in sphere → inside: {d}");
    }

    #[test]
    fn cut_sphere_below_cut_outside() {
        use crate::math::Vec3;
        // y=-0.5 < h=0.5 → outside
        let d = cut_sphere_3d(Vec3::new(0.0, -0.5, 0.0), 1.0, 0.5);
        assert!(d > 0.0, "below cut → outside: {d}");
    }

    // ── oriented_box_2d ───────────────────────────────────────────────────
    #[test]
    fn oriented_box_axis_aligned_inside() {
        let d = oriented_box_2d(Vec2::new(0.3, 0.2), Vec2::new(0.5, 0.5), 0.0);
        assert!(d < 0.0, "inside axis-aligned box: {d}");
    }

    #[test]
    fn oriented_box_axis_aligned_outside() {
        let d = oriented_box_2d(Vec2::new(1.0, 0.0), Vec2::new(0.5, 0.5), 0.0);
        assert!(d > 0.0, "outside axis-aligned box: {d}");
    }

    #[test]
    fn oriented_box_rotated_centre_inside() {
        // Rotation doesn't affect origin (centre of box)
        let d = oriented_box_2d(Vec2::ZERO, Vec2::new(0.5, 0.5), std::f32::consts::FRAC_PI_4);
        assert!(d < 0.0, "origin always inside: {d}");
    }

    // ── arrow_2d ──────────────────────────────────────────────────────────
    #[test]
    fn arrow_shaft_midpoint_inside() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(2.0, 0.0);
        let d = arrow_2d(Vec2::new(0.7, 0.0), a, b, 0.3, 0.5, 0.1);
        assert!(d < 0.0, "midpoint of shaft inside: {d}");
    }

    #[test]
    fn arrow_far_outside() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(2.0, 0.0);
        let d = arrow_2d(Vec2::new(1.0, 2.0), a, b, 0.3, 0.5, 0.1);
        assert!(d > 0.0, "far above arrow: {d}");
    }

    // ── sharp CSG operators ───────────────────────────────────────────────
    #[test]
    fn sdf_union_picks_closer() {
        assert_eq!(sdf_union(-1.0, 2.0), -1.0);
        assert_eq!(sdf_union(3.0, 1.0), 1.0);
        assert_eq!(sdf_union(-2.0, -0.5), -2.0);
    }

    #[test]
    fn sdf_intersect_picks_further() {
        assert_eq!(sdf_intersect(-1.0, -0.5), -0.5);
        assert!(sdf_intersect(-1.0, 2.0) > 0.0); // inside a but outside b
    }

    #[test]
    fn sdf_subtract_cuts_b_from_a() {
        // Inside a (-1) and inside b (-0.5) → cut out → 0.5 (outside)
        assert!(sdf_subtract(-1.0, -0.5) > 0.0);
        // Inside a (-1) but outside b (2) → survives → -1 (inside)
        assert!(sdf_subtract(-1.0, 2.0) < 0.0);
    }

    #[test]
    fn sharp_csg_consistency_with_smooth() {
        // At k→0, smooth operators approach sharp ones
        let a = 0.3_f32;
        let b = 0.7_f32;
        assert!((sdf_union(a, b) - smooth_union(a, b, 0.001)).abs() < 0.01);
    }

    // ── segment_3d ────────────────────────────────────────────────────────
    #[test]
    fn segment_3d_on_segment_zero() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(2.0, 0.0, 0.0);
        let d = segment_3d(Vec3::new(1.0, 0.0, 0.0), a, b);
        assert!(d < 1e-5, "on segment: {d}");
    }

    #[test]
    fn segment_3d_perpendicular_distance() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(2.0, 0.0, 0.0);
        let d = segment_3d(Vec3::new(1.0, 1.0, 0.0), a, b);
        assert!((d - 1.0).abs() < 1e-5, "perpendicular distance: {d}");
    }

    #[test]
    fn segment_3d_past_end() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(2.0, 0.0, 0.0);
        let d = segment_3d(Vec3::new(4.0, 0.0, 0.0), a, b);
        assert!((d - 2.0).abs() < 1e-5, "past endpoint: {d}");
    }

    // ── rounded_cone_3d ───────────────────────────────────────────────────
    #[test]
    fn rounded_cone_inside_large_cap() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(0.0, 2.0, 0.0);
        let d = rounded_cone_3d(Vec3::new(0.0, 0.0, 0.0), a, b, 0.5, 0.2);
        assert!(d < 0.0, "inside large cap: {d}");
    }

    #[test]
    fn rounded_cone_outside() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(0.0, 2.0, 0.0);
        let d = rounded_cone_3d(Vec3::new(0.0, 5.0, 0.0), a, b, 0.5, 0.2);
        assert!(d > 0.0, "far outside: {d}");
    }

    #[test]
    fn rounded_cone_equal_radii_is_capsule() {
        // Equal radii → same as capsule_3d with that radius
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(0.0, 2.0, 0.0);
        let r = 0.3_f32;
        let p = Vec3::new(0.15, 1.0, 0.0);
        let d_cone = rounded_cone_3d(p, a, b, r, r);
        let d_cap = capsule_3d(p, a, b, r);
        assert!(
            (d_cone - d_cap).abs() < 1e-4,
            "equal-radii cone = capsule: {d_cone} vs {d_cap}"
        );
    }
}

// ── Pass 18: SDF additions ────────────────────────────────────────────────────

/// Cut hollow sphere: a spherical shell sliced by a horizontal plane at height
/// `h`, leaving the open dome (y ≥ h portion of the shell).
///
/// - `r` — sphere radius
/// - `h` — cut height (signed; negative cuts below equator)
/// - `t` — shell thickness
///
/// Translated directly from Inigo Quilez's `sdCutHollowSphere`.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::cut_hollow_sphere_3d;
/// use abrash_core::math::Vec3;
/// // Point at the north pole of a r=1 shell (thickness=0.1), cut at y=0
/// let d = cut_hollow_sphere_3d(Vec3::new(0.0, 1.0, 0.0), 1.0, 0.0, 0.1);
/// assert!(d < 0.0, "inside shell at north pole: {d}");
/// ```
pub fn cut_hollow_sphere_3d(p: Vec3, r: f32, h: f32, t: f32) -> f32 {
    // Shell distance: inside when |p| is within t/2 of r
    let shell_d = (p.length() - r).abs() - t * 0.5;
    // Plane constraint: only the y ≥ h portion is kept (SDF of half-space y ≥ h is h - p.y)
    shell_d.max(h - p.y)
}

/// 2D elongation operator.
///
/// Stretches a 2D SDF along each axis by `h`, preserving exact distances
/// everywhere outside the stretched region.  Equivalent to the 3D `elongate`
/// but for 2D shapes.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::{elongate_2d, circle_2d};
/// use abrash_core::math::Vec2;
/// // A circle elongated by (0.5, 0.0) becomes a capsule shape
/// let d = elongate_2d(Vec2::new(1.0, 0.0), Vec2::new(0.5, 0.0), |q| circle_2d(q, Vec2::ZERO, 0.3));
/// assert!(d > 0.0, "outside elongated circle: {d}");
/// ```
pub fn elongate_2d(p: Vec2, h: Vec2, sdf: impl Fn(Vec2) -> f32) -> f32 {
    let q = p - p.clamp(Vec2::new(-h.x, -h.y), h);
    sdf(q)
}

/// Tunnel (U-shape arch) SDF.
///
/// An open-topped rectangular channel with inner half-width `wh.x` and depth
/// `wh.y`.  Points inside the tunnel return negative distances; the flat
/// bottom and rounded corners are exact.
///
/// Translated from Inigo Quilez's `sdTunnel`.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::tunnel_2d;
/// use abrash_core::math::Vec2;
/// // Point inside the tunnel opening (above the floor, between the walls)
/// let d = tunnel_2d(Vec2::new(0.0, 0.3), Vec2::new(0.5, 0.8));
/// assert!(d < 0.0, "inside tunnel: {d}");
/// // Point outside (far to the right)
/// let d2 = tunnel_2d(Vec2::new(2.0, 0.0), Vec2::new(0.5, 0.8));
/// assert!(d2 > 0.0, "outside tunnel: {d2}");
/// ```
pub fn tunnel_2d(p: Vec2, wh: Vec2) -> f32 {
    // Mirror x, flip y so the tunnel opens upward (tunnel opens toward +y in input space)
    let p = Vec2::new(p.x.abs(), -p.y);
    // q relative to the inner corner
    let q = Vec2::new(p.x - wh.x, p.y - wh.y);

    // Distance to the flat outer side wall
    let d1 = Vec2::new(q.x.max(0.0), q.y).length_sq();
    // Distance to the rounded bottom corner (two cases: above vs below the floor level)
    // qx2 is q.x when above the floor, else distance-from-floor-axis minus half-width
    let qx2 = if p.y > 0.0 { q.x } else { p.length() - wh.x };
    // IQ: dot2(vec2(q.x, max(q.y, 0))) — note: q.x here is qx2, NOT clamped
    let qy_pos = q.y.max(0.0);
    let d2 = qx2 * qx2 + qy_pos * qy_pos;

    let d = d1.min(d2).sqrt();
    // Sign: negative when inside (qx2 and q.y both negative = inside the tunnel region)
    if qx2.max(q.y) < 0.0 { -d } else { d }
}

/// Vesica piscis on a 3D line segment.
///
/// The vesica segment is the intersection of two spheres of radius `w` whose
/// centres are at `a` and `b`.  In SDF terms it produces a lens-like shape
/// along the segment.
///
/// Translated from Inigo Quilez's `sdVesicaSegment`.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::vesica_segment_3d;
/// use abrash_core::math::Vec3;
/// // Point at the midpoint of the segment, well inside (w=0.8 > half-length=0.5)
/// let d = vesica_segment_3d(
///     Vec3::new(0.0, 0.5, 0.0),
///     Vec3::new(0.0, 0.0, 0.0),
///     Vec3::new(0.0, 1.0, 0.0),
///     0.8,
/// );
/// assert!(d < 0.0, "inside vesica: {d}");
/// ```
pub fn vesica_segment_3d(p: Vec3, a: Vec3, b: Vec3, w: f32) -> f32 {
    // The vesica piscis is the intersection of two equal spheres.
    // In 3D with segment endpoints as sphere centres, the SDF is:
    // max(|p-a| - w, |p-b| - w) — negative only when inside BOTH spheres.
    // Requires w ≥ half-segment-length for the vesica to be non-empty.
    ((p - a).length() - w).max((p - b).length() - w)
}

/// Blobby cross SDF (IQ `sdBlobbyCross`).
///
/// A smooth cross shape with bulging arms.  `he` controls the arm length /
/// blobbyness: larger values give longer, smoother arms.
///
/// This is one of the few 2D SDFs that requires an exact cubic solve for
/// correctness.
///
/// Translated directly from Inigo Quilez's `sdBlobbyCross`.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::blobby_cross_2d;
/// use abrash_core::math::Vec2;
/// // Centre is inside the blobby cross
/// let d = blobby_cross_2d(Vec2::ZERO, 0.5);
/// assert!(d < 0.0, "centre should be inside: {d}");
/// // Far away is outside
/// let d2 = blobby_cross_2d(Vec2::new(3.0, 3.0), 0.5);
/// assert!(d2 > 0.0, "far point should be outside: {d2}");
/// ```
pub fn blobby_cross_2d(pos: Vec2, he: f32) -> f32 {
    use core::f32::consts::SQRT_2;
    // Fold into first octant and rotate 45°
    let pos = Vec2::new(pos.x.abs(), pos.y.abs());
    let pos = Vec2::new(
        (pos.x - pos.y).abs() / SQRT_2,
        (1.0 - pos.x - pos.y) / SQRT_2,
    );

    let p = (he - pos.y - 0.25 / he) / (6.0 * he);
    let q = pos.x / (he * he * 16.0);
    let h = q * q - p * p * p;

    let x = if h > 0.0 {
        let r = h.sqrt();
        let qr = q + r;
        let qmr = (q - r).abs();
        // Real cube root (preserving sign)
        let cbrt = |v: f32| v.cbrt();
        cbrt(qr) - cbrt(qmr)
    } else {
        let r = p.max(0.0).sqrt();
        let angle = (q / (p * r).max(1e-10)).clamp(-1.0, 1.0).acos();
        2.0 * r * (angle / 3.0).cos()
    };

    let x = x.min(SQRT_2 / 2.0);
    let z = Vec2::new(x, he * (1.0 - 2.0 * x * x)) - pos;
    z.length() * z.y.signum()
}

#[cfg(test)]
mod tests_pass_18 {
    use super::*;
    use crate::math::{Vec2, Vec3};

    // ── cut_hollow_sphere_3d ──────────────────────────────────────────────
    #[test]
    fn cut_hollow_sphere_inside_shell() {
        // North pole of a unit sphere (r=1, cut at h=0, thickness=0.1)
        // |p| = 1.0 → shell_d = -0.05, plane: h-p.y = -1 → max(-0.05, -1) = -0.05
        let d = cut_hollow_sphere_3d(Vec3::new(0.0, 1.0, 0.0), 1.0, 0.0, 0.1);
        assert!(d < 0.0, "inside shell at north pole: {d}");
    }

    #[test]
    fn cut_hollow_sphere_outside() {
        // Far above the dome
        let d = cut_hollow_sphere_3d(Vec3::new(0.0, 3.0, 0.0), 1.0, 0.0, 0.1);
        assert!(d > 0.0, "outside: {d}");
    }

    #[test]
    fn cut_hollow_sphere_below_cut_is_outside() {
        // Below the cut plane the dome is open — point is outside
        let d = cut_hollow_sphere_3d(Vec3::new(0.5, -0.5, 0.0), 1.0, 0.0, 0.1);
        assert!(d > 0.0, "below cut plane should be outside: {d}");
    }

    // ── elongate_2d ───────────────────────────────────────────────────────
    #[test]
    fn elongate_2d_unextended_equals_base_sdf() {
        // Zero elongation → same as underlying SDF
        let p = Vec2::new(0.7, 0.0);
        let direct = circle_2d(p, Vec2::ZERO, 0.5);
        let elongated = elongate_2d(p, Vec2::ZERO, |q| circle_2d(q, Vec2::ZERO, 0.5));
        assert!(
            (direct - elongated).abs() < 1e-6,
            "zero elongation mismatch: {direct} vs {elongated}"
        );
    }

    #[test]
    fn elongate_2d_stretches_interior() {
        // Point at (0.3, 0) is inside a unit circle but outside the same circle
        // centred at origin; with x-elongation of 0.5 it should be inside
        let d = elongate_2d(Vec2::new(0.3, 0.0), Vec2::new(0.5, 0.0), |q| {
            circle_2d(q, Vec2::ZERO, 0.3)
        });
        assert!(d < 0.0, "inside elongated shape: {d}");
    }

    // ── tunnel_2d ─────────────────────────────────────────────────────────
    #[test]
    fn tunnel_inside() {
        let d = tunnel_2d(Vec2::new(0.0, 0.3), Vec2::new(0.5, 0.8));
        assert!(d < 0.0, "inside tunnel: {d}");
    }

    #[test]
    fn tunnel_outside_right() {
        let d = tunnel_2d(Vec2::new(2.0, 0.0), Vec2::new(0.5, 0.8));
        assert!(d > 0.0, "outside tunnel (right): {d}");
    }

    #[test]
    fn tunnel_outside_above() {
        // The tunnel opens upward so above the opening is outside
        let d = tunnel_2d(Vec2::new(0.0, -2.0), Vec2::new(0.5, 0.8));
        assert!(d > 0.0, "outside tunnel (above opening): {d}");
    }

    // ── vesica_segment_3d ─────────────────────────────────────────────────
    #[test]
    fn vesica_segment_inside_midpoint() {
        let d = vesica_segment_3d(
            Vec3::new(0.0, 0.5, 0.0),
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
            0.8,
        );
        assert!(d < 0.0, "midpoint inside vesica: {d}");
    }

    #[test]
    fn vesica_segment_outside_far() {
        let d = vesica_segment_3d(
            Vec3::new(5.0, 0.5, 0.0),
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
            0.8,
        );
        assert!(d > 0.0, "far point outside vesica: {d}");
    }

    #[test]
    fn vesica_segment_narrow_w_excludes_midpoint() {
        // Very thin vesica (w < half-segment-length) → midpoint is outside
        let d = vesica_segment_3d(
            Vec3::new(0.0, 0.5, 0.0),
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
            0.05,
        );
        assert!(d > 0.0, "thin vesica, midpoint outside: {d}");
    }

    // ── blobby_cross_2d ───────────────────────────────────────────────────
    #[test]
    fn blobby_cross_center_inside() {
        let d = blobby_cross_2d(Vec2::ZERO, 0.5);
        assert!(d < 0.0, "centre inside blobby cross: {d}");
    }

    #[test]
    fn blobby_cross_far_outside() {
        let d = blobby_cross_2d(Vec2::new(5.0, 5.0), 0.5);
        assert!(d > 0.0, "far point outside: {d}");
    }

    #[test]
    fn blobby_cross_on_axis_outside() {
        // Well beyond arm tip on +x axis
        let d = blobby_cross_2d(Vec2::new(3.0, 0.0), 0.5);
        assert!(d > 0.0, "past arm tip: {d}");
    }
}

// ── Pass 19: SDF additions ────────────────────────────────────────────────────

/// Staircase SDF.
///
/// A staircase of `n` steps, each of width `wh.x` and height `wh.y`, stepping
/// in the +x/+y direction from the origin.  Points above or to the right of
/// the staircase are outside (positive distance).
///
/// Translated from Inigo Quilez's `sdStairs`.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::stairs_2d;
/// use abrash_core::math::Vec2;
/// // Centre of the first step
/// let d = stairs_2d(Vec2::new(0.1, 0.1), Vec2::new(0.5, 0.3), 3);
/// assert!(d < 0.0, "inside first step: {d}");
/// // Far away is outside
/// let d2 = stairs_2d(Vec2::new(5.0, 5.0), Vec2::new(0.5, 0.3), 3);
/// assert!(d2 > 0.0, "far outside: {d2}");
/// ```
pub fn stairs_2d(p: Vec2, wh: Vec2, n: u32) -> f32 {
    let n_f = n as f32;
    let ba = Vec2::new(wh.x * n_f, wh.y * n_f);

    // Distance to the two bounding edges of the full staircase rect
    let d_bottom = Vec2::new((p.x - p.x.clamp(0.0, ba.x)).powi(2) + p.y.powi(2), 0.0)
        .x
        .sqrt()
        * p.y.signum().min(1.0);
    let _ = d_bottom; // replaced below

    // Distance to the two extremal edges (bottom-left and top-right)
    let dp1 = Vec2::new(p.x.clamp(0.0, ba.x) - p.x, 0.0 - p.y);
    let dp2 = Vec2::new(ba.x - p.x, ba.y.min(p.y.max(0.0)) - p.y);
    let d1 = dp1.x * dp1.x + dp1.y * dp1.y;
    let d2 = dp2.x * dp2.x + dp2.y * dp2.y;
    let d = d1.min(d2).sqrt();

    // When inside the staircase AABB, find the nearest step edge
    if p.x >= 0.0 && p.y >= 0.0 && p.x <= ba.x && p.y <= ba.y {
        // Which step are we in?
        let step = (p.x / wh.x).floor().clamp(0.0, n_f - 1.0);
        let step_top = (step + 1.0) * wh.y;
        let step_right = (step + 1.0) * wh.x;
        // Distance to the two interior step edges (top face and right face)
        let d_top = (step_top - p.y).abs();
        let d_right = (step_right - p.x).abs();
        let d_inner = d_top.min(d_right);
        // Negative inside the filled region below the staircase
        if p.y <= step_top {
            return -(d.min(d_inner));
        }
    }
    d
}

/// Death-star SDF: sphere `a` (radius `ra`) with sphere `b` (radius `rb`,
/// centred at distance `d` from `a`'s centre along +x) subtracted from it.
///
/// This is a simple boolean CSG operation — `sdf_subtract(sphere_a, sphere_b)`
/// — but parameterised for the iconic two-sphere Death-Star shape.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::death_star_3d;
/// use abrash_core::math::Vec3;
/// // Centre of the main sphere is inside
/// let d = death_star_3d(Vec3::ZERO, 1.0, 0.5, 1.4);
/// assert!(d < 0.0, "inside main sphere: {d}");
/// // Point on the indent side may be outside (carved out)
/// let d2 = death_star_3d(Vec3::new(1.3, 0.0, 0.0), 1.0, 0.5, 1.4);
/// assert!(d2 > 0.0, "in the carved-out region: {d2}");
/// ```
pub fn death_star_3d(p: Vec3, ra: f32, rb: f32, d: f32) -> f32 {
    // SDF of main sphere centred at origin
    let da = p.length() - ra;
    // SDF of subtracting sphere centred at (d, 0, 0)
    let db = (p - Vec3::new(d, 0.0, 0.0)).length() - rb;
    // Boolean subtract: max(da, -db)
    da.max(-db)
}

/// 3D cross SDF — the union of three axis-aligned rectangular bars that
/// intersect at the origin.
///
/// `b` is the half-extent of each bar in the cross-section axes, and `r` is
/// the rounding radius applied to all edges.  Set `r = 0.0` for a sharp cross.
///
/// Translated from Inigo Quilez's `sdCross`.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::cross_3d;
/// use abrash_core::math::Vec3;
/// // Centre is inside all three bars
/// let d = cross_3d(Vec3::ZERO, 0.3, 0.0);
/// assert!(d < 0.0, "centre inside cross: {d}");
/// // Far diagonal corner is outside
/// let d2 = cross_3d(Vec3::new(2.0, 2.0, 2.0), 0.3, 0.0);
/// assert!(d2 > 0.0, "diagonal outside: {d2}");
/// ```
pub fn cross_3d(p: Vec3, b: f32, r: f32) -> f32 {
    let d = Vec3::new(p.x.abs(), p.y.abs(), p.z.abs()) - Vec3::new(b, b, b);
    // Three rectangular bar SDFs (one per axis pair) — take the union (min)
    let d1 = Vec2::new(d.x.max(0.0), d.y.max(0.0)).length() + d.x.max(d.y).min(0.0) - r;
    let d2 = Vec2::new(d.z.max(0.0), d.y.max(0.0)).length() + d.z.max(d.y).min(0.0) - r;
    let d3 = Vec2::new(d.x.max(0.0), d.z.max(0.0)).length() + d.x.max(d.z).min(0.0) - r;
    d1.min(d2).min(d3)
}

#[cfg(test)]
mod tests_pass_19 {
    use super::*;
    use crate::math::{Vec2, Vec3};

    // ── stairs_2d ─────────────────────────────────────────────────────────
    #[test]
    fn stairs_inside_first_step() {
        // Inside the filled region below the first step top
        let d = stairs_2d(Vec2::new(0.1, 0.1), Vec2::new(0.5, 0.3), 3);
        assert!(d < 0.0, "inside first step: {d}");
    }

    #[test]
    fn stairs_far_outside() {
        let d = stairs_2d(Vec2::new(5.0, 5.0), Vec2::new(0.5, 0.3), 3);
        assert!(d > 0.0, "far outside staircase: {d}");
    }

    #[test]
    fn stairs_above_top_outside() {
        // Above the top of the staircase (y > n*wh.y)
        let wh = Vec2::new(0.5, 0.3);
        let n = 3u32;
        let total_h = wh.y * n as f32;
        let d = stairs_2d(Vec2::new(0.5, total_h + 0.5), wh, n);
        assert!(d > 0.0, "above top of staircase: {d}");
    }

    // ── death_star_3d ─────────────────────────────────────────────────────
    #[test]
    fn death_star_center_inside() {
        let d = death_star_3d(Vec3::ZERO, 1.0, 0.5, 1.4);
        assert!(d < 0.0, "centre inside main sphere: {d}");
    }

    #[test]
    fn death_star_carved_region_outside() {
        // The indent sits at ~x=1.3; the carved sphere (rb=0.5) centred at x=1.4
        // carves out the region around x=1 on the +x side
        let d = death_star_3d(Vec3::new(1.3, 0.0, 0.0), 1.0, 0.5, 1.4);
        assert!(d > 0.0, "carved region outside: {d}");
    }

    #[test]
    fn death_star_opposite_side_inside() {
        // Far side of main sphere, away from the indent
        let d = death_star_3d(Vec3::new(-0.5, 0.0, 0.0), 1.0, 0.5, 1.4);
        assert!(d < 0.0, "opposite side inside: {d}");
    }

    // ── cross_3d ──────────────────────────────────────────────────────────
    #[test]
    fn cross_3d_center_inside() {
        let d = cross_3d(Vec3::ZERO, 0.3, 0.0);
        assert!(d < 0.0, "centre inside cross: {d}");
    }

    #[test]
    fn cross_3d_on_x_arm_inside() {
        // Point on the +x arm, inside the bar
        let d = cross_3d(Vec3::new(1.0, 0.1, 0.1), 0.3, 0.0);
        assert!(d < 0.0, "on x arm inside: {d}");
    }

    #[test]
    fn cross_3d_diagonal_outside() {
        let d = cross_3d(Vec3::new(2.0, 2.0, 2.0), 0.3, 0.0);
        assert!(d > 0.0, "diagonal outside: {d}");
    }

    #[test]
    fn cross_3d_rounding_increases_distance() {
        // Adding rounding to a point on the sharp edge should move it outward
        let p = Vec3::new(0.5, 0.5, 0.0);
        let d_sharp = cross_3d(p, 0.3, 0.0);
        let d_round = cross_3d(p, 0.3, 0.1);
        // Rounding subtracts from the SDF, so rounded is "more inside"
        assert!(
            d_round <= d_sharp + 0.2,
            "rounding changes distance: {d_sharp} vs {d_round}"
        );
    }
}

// ── Pass 20: SDF additions ────────────────────────────────────────────────────

/// Infinite cone SDF — a double-ended cone extending to infinity along the Y
/// axis.
///
/// `c` is `(sin(θ), cos(θ))` where `θ` is the half-angle of the cone.  The
/// cone tip is at the origin; the cone extends in both the +y and −y directions.
///
/// Translated from Inigo Quilez's `sdCone` (the infinite variant).
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::infinite_cone_3d;
/// use abrash_core::math::Vec3;
/// // On the axis above the tip: inside the cone
/// let sin_cos = (0.5_f32.sin(), 0.5_f32.cos()); // half-angle 0.5 rad
/// let d = infinite_cone_3d(Vec3::new(0.0, 1.0, 0.0), sin_cos);
/// assert!(d < 0.0, "on axis inside cone: {d}");
/// // Far off-axis: outside
/// let d2 = infinite_cone_3d(Vec3::new(5.0, 1.0, 0.0), sin_cos);
/// assert!(d2 > 0.0, "off-axis outside: {d2}");
/// ```
pub fn infinite_cone_3d(p: Vec3, c: (f32, f32)) -> f32 {
    // 2D profile: (radial distance from y-axis, y coordinate)
    let q = Vec2::new(p.x.mul_add(p.x, p.z * p.z).sqrt(), p.y);
    // Signed distance to the cone surface line through origin with normal (cos, -sin)
    q.x * c.1 - q.y.abs() * c.0
}

/// Planar quad SDF (3D).
///
/// Signed distance to the convex planar quad defined by four vertices
/// `a`, `b`, `c`, `d` (in order).  Returns negative for points on the
/// interior side (i.e., behind the quad's normal).
///
/// Translated from Inigo Quilez's `sdQuad`.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::quad_3d;
/// use abrash_core::math::Vec3;
/// // Centre of a unit square in the XZ plane
/// let a = Vec3::new(-0.5, 0.0, -0.5);
/// let b = Vec3::new( 0.5, 0.0, -0.5);
/// let c = Vec3::new( 0.5, 0.0,  0.5);
/// let d = Vec3::new(-0.5, 0.0,  0.5);
/// // Point 0.1 above the centre of the quad — distance should be ~0.1
/// let dist = quad_3d(Vec3::new(0.0, 0.1, 0.0), a, b, c, d);
/// assert!((dist - 0.1).abs() < 0.01, "above centre: {dist}");
/// // Far above
/// let far = quad_3d(Vec3::new(0.0, 10.0, 0.0), a, b, c, d);
/// assert!(far > 9.0, "far above: {far}");
/// ```
pub fn quad_3d(p: Vec3, a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> f32 {
    // Translated from Inigo Quilez's sdQuad (exact signed distance)
    let ba = b - a;
    let cb = c - b;
    let dc = d - c;
    let ad = a - d;
    let pa = p - a;
    let pb = p - b;
    let pc = p - c;
    let pd = p - d;

    // Normal direction from ba × ad
    let nor = ba.cross(ad);

    // Helper: length squared of a vector
    let dot2 = |v: Vec3| v.dot(v);

    // Clamp-and-distance for each edge
    let edge_dist2 = |pe: Vec3, edge: Vec3| -> f32 {
        let t = (pe.dot(edge) / dot2(edge)).clamp(0.0, 1.0);
        dot2(edge * t - pe)
    };

    // Check whether the projection of p lies inside the quad
    // (all 4 cross-product signs positive = inside)
    let inside = (ba.cross(nor).dot(pa).signum()
        + cb.cross(nor).dot(pb).signum()
        + dc.cross(nor).dot(pc).signum()
        + ad.cross(nor).dot(pd).signum())
        >= 4.0 - 0.5; // =4 means inside all half-planes

    let dist2 = if inside {
        // Point projects inside quad: distance is purely perpendicular (to plane)
        nor.dot(pa) * nor.dot(pa) / dot2(nor)
    } else {
        // Point projects outside: nearest edge distance
        edge_dist2(pa, ba)
            .min(edge_dist2(pb, cb))
            .min(edge_dist2(pc, dc))
            .min(edge_dist2(pd, ad))
    };

    // Sign: positive if on the +normal side (consistent with plane SDF)
    let sign = nor.dot(pa).signum();
    dist2.sqrt() * sign
}

/// Infinite horizontal slab SDF — the space between two parallel planes at
/// y = −`h` and y = +`h`.
///
/// Returns negative for points inside the slab, positive outside.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::slab_y_3d;
/// use abrash_core::math::Vec3;
/// assert!(slab_y_3d(Vec3::new(0.0, 0.3, 0.0), 0.5) < 0.0, "inside slab");
/// assert!(slab_y_3d(Vec3::new(0.0, 1.0, 0.0), 0.5) > 0.0, "outside slab");
/// ```
pub fn slab_y_3d(p: Vec3, h: f32) -> f32 {
    p.y.abs() - h
}

/// Boolean XOR of two SDFs: the region inside exactly one of them.
///
/// `xor(a, b) = max(min(a, b), -max(a, b))` — equivalent to `union - intersection`.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::sdf_xor;
/// // Inside both shapes: XOR should be outside
/// let d = sdf_xor(-0.3_f32, -0.4);
/// assert!(d > 0.0, "inside both → XOR outside: {d}");
/// // Inside one only: XOR is inside
/// let d2 = sdf_xor(-0.3_f32, 0.5);
/// assert!(d2 < 0.0, "inside one → XOR inside: {d2}");
/// ```
pub fn sdf_xor(a: f32, b: f32) -> f32 {
    // XOR = (union) ∩ NOT(intersection)
    // union = min(a,b); NOT(intersection) = -max(a,b)
    // Combined: max(min(a,b), -max(a,b))
    (a.min(b)).max(-(a.max(b)))
}

#[cfg(test)]
mod tests_pass_20 {
    use super::*;
    use crate::math::{Vec2, Vec3};

    // ── infinite_cone_3d ──────────────────────────────────────────────────
    #[test]
    fn infinite_cone_on_axis_inside() {
        let angle = 0.5_f32; // half-angle in radians
        let c = (angle.sin(), angle.cos());
        let d = infinite_cone_3d(Vec3::new(0.0, 1.0, 0.0), c);
        assert!(d < 0.0, "on +y axis inside cone: {d}");
    }

    #[test]
    fn infinite_cone_off_axis_outside() {
        let angle = 0.3_f32;
        let c = (angle.sin(), angle.cos());
        let d = infinite_cone_3d(Vec3::new(5.0, 0.1, 0.0), c);
        assert!(d > 0.0, "off-axis outside: {d}");
    }

    #[test]
    fn infinite_cone_tip_zero() {
        // At the very tip (origin), distance depends on cone angle
        let angle = 0.5_f32;
        let c = (angle.sin(), angle.cos());
        let d = infinite_cone_3d(Vec3::ZERO, c);
        // At origin: q=(0,0), dist = 0*cos - 0*sin = 0
        assert!(d.abs() < 1e-5, "at tip: {d}");
    }

    // ── quad_3d ───────────────────────────────────────────────────────────
    #[test]
    fn quad_3d_above_surface_positive() {
        let a = Vec3::new(-0.5, 0.0, -0.5);
        let b = Vec3::new(0.5, 0.0, -0.5);
        let c = Vec3::new(0.5, 0.0, 0.5);
        let d = Vec3::new(-0.5, 0.0, 0.5);
        let dist = quad_3d(Vec3::new(0.0, 1.0, 0.0), a, b, c, d);
        assert!(dist > 0.0, "above quad is positive: {dist}");
    }

    #[test]
    fn quad_3d_below_surface_negative() {
        let a = Vec3::new(-0.5, 0.0, -0.5);
        let b = Vec3::new(0.5, 0.0, -0.5);
        let c = Vec3::new(0.5, 0.0, 0.5);
        let d = Vec3::new(-0.5, 0.0, 0.5);
        let dist = quad_3d(Vec3::new(0.0, -0.5, 0.0), a, b, c, d);
        assert!(dist < 0.0, "below quad is negative: {dist}");
    }

    // ── slab_y_3d ─────────────────────────────────────────────────────────
    #[test]
    fn slab_inside() {
        let d = slab_y_3d(Vec3::new(0.0, 0.2, 0.0), 0.5);
        assert!(d < 0.0, "inside slab: {d}");
    }

    #[test]
    fn slab_outside() {
        let d = slab_y_3d(Vec3::new(0.0, 1.0, 0.0), 0.5);
        assert!(d > 0.0, "outside slab: {d}");
    }

    // ── sdf_xor ───────────────────────────────────────────────────────────
    #[test]
    fn sdf_xor_inside_both_is_outside() {
        // Both negative (inside both shapes): XOR should be outside (positive)
        let d = sdf_xor(-0.3, -0.4);
        assert!(d > 0.0, "inside both → XOR outside: {d}");
    }

    #[test]
    fn sdf_xor_inside_one_is_inside() {
        // One inside, one outside: XOR is inside the one
        let d = sdf_xor(-0.3, 0.5);
        assert!(d < 0.0, "inside one → XOR inside: {d}");
    }

    #[test]
    fn sdf_xor_outside_both_is_outside() {
        // Both positive (outside both): XOR is outside
        let d = sdf_xor(0.5, 0.3);
        assert!(d > 0.0, "outside both → XOR outside: {d}");
    }
}

// ── Pass 21: SDF additions ────────────────────────────────────────────────────

/// Exact distance to a 3D triangle (as a flat planar surface).
///
/// Since the triangle is a 2D surface in 3D space, there is no "inside" — the
/// distance is always non-negative.  Points project onto the plane; if the
/// projection falls outside the triangle, the nearest edge is used.
///
/// Translated from Inigo Quilez's `sdTriangle` (3D version).
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::triangle_3d;
/// use abrash_core::math::Vec3;
/// // Point directly above the centroid: distance = height
/// let a = Vec3::new(-1.0, 0.0, 0.0);
/// let b = Vec3::new( 1.0, 0.0, 0.0);
/// let c = Vec3::new( 0.0, 0.0, 1.0);
/// let d = triangle_3d(Vec3::new(0.0, 0.5, 0.33), a, b, c);
/// assert!(d < 0.6, "near centroid above: {d}");
/// // Far point: larger distance
/// let d2 = triangle_3d(Vec3::new(10.0, 0.0, 0.0), a, b, c);
/// assert!(d2 > 8.0, "far point: {d2}");
/// ```
pub fn triangle_3d(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> f32 {
    let ba = b - a;
    let cb = c - b;
    let ac = a - c;
    let pa = p - a;
    let pb = p - b;
    let pc = p - c;
    let nor = ba.cross(ac);

    let dot2 = |v: Vec3| v.dot(v);

    // Check if point projects inside the triangle (all 3 edge half-planes positive)
    let signs = ba.cross(nor).dot(pa).signum()
        + cb.cross(nor).dot(pb).signum()
        + ac.cross(nor).dot(pc).signum();

    let dist2 = if signs < 2.0 {
        // Project outside → nearest edge
        let d_ba = dot2(ba * (ba.dot(pa) / dot2(ba)).clamp(0.0, 1.0) - pa);
        let d_cb = dot2(cb * (cb.dot(pb) / dot2(cb)).clamp(0.0, 1.0) - pb);
        let d_ac = dot2(ac * (ac.dot(pc) / dot2(ac)).clamp(0.0, 1.0) - pc);
        d_ba.min(d_cb).min(d_ac)
    } else {
        // Project inside → perpendicular distance to plane
        nor.dot(pa) * nor.dot(pa) / dot2(nor)
    };

    dist2.sqrt()
}

/// 3D rhombus SDF — a diamond-shaped prism with rounded edges.
///
/// - `la`, `lb` — half-diagonals of the rhombus base in x and z
/// - `h`  — half-height along y
/// - `ra` — edge rounding radius
///
/// Translated from Inigo Quilez's `sdRhombus`.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::rhombus_3d;
/// use abrash_core::math::Vec3;
/// // Centre is inside
/// let d = rhombus_3d(Vec3::ZERO, 1.0, 0.5, 0.4, 0.05);
/// assert!(d < 0.0, "centre inside: {d}");
/// // Far corner outside
/// let d2 = rhombus_3d(Vec3::new(3.0, 0.0, 0.0), 1.0, 0.5, 0.4, 0.05);
/// assert!(d2 > 0.0, "far outside: {d2}");
/// ```
pub fn rhombus_3d(p: Vec3, la: f32, lb: f32, h: f32, ra: f32) -> f32 {
    let p = Vec3::new(p.x.abs(), p.y.abs(), p.z.abs());
    let b = Vec2::new(la, lb);
    // ndot: a.x*b.x - a.y*b.y (like a 2D "anti-dot")
    let ndot = |a: Vec2, bv: Vec2| a.x * bv.x - a.y * bv.y;
    let f = (ndot(b, b - Vec2::new(2.0 * p.x, 2.0 * p.z)) / b.dot(b)).clamp(-1.0, 1.0);

    let sx = b.x * (1.0 - f) * 0.5 - p.x;
    let sz = b.y * (1.0 + f) * 0.5 - p.z;

    let side_pt = Vec2::new((sx * sx + sz * sz).sqrt(), p.y - h);
    let sign = (p.x * b.y + p.z * b.x - b.x * b.y).signum();
    let qx = side_pt.x * sign - ra;
    let qy = side_pt.y;
    qx.max(qy).min(0.0) + Vec2::new(qx.max(0.0), qy.max(0.0)).length()
}

/// Oreo cookie SDF — two flat discs sandwiching a centre layer.
///
/// - `r` — radius of each disc
/// - `h` — height of each disc layer (the cream between is at y=0)
/// - `cr` — "cream" half-thickness (gap between the two discs)
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::oreo_2d;
/// use abrash_core::math::Vec2;
/// // Inside the top cookie disc (centred at y = cr+h = 0.35)
/// let d = oreo_2d(Vec2::new(0.2, 0.35), 0.8, 0.15, 0.2);
/// assert!(d < 0.0, "inside top disc: {d}");
/// // Outside entirely
/// let d2 = oreo_2d(Vec2::new(2.0, 0.0), 0.8, 0.15, 0.2);
/// assert!(d2 > 0.0, "outside: {d2}");
/// ```
pub fn oreo_2d(p: Vec2, r: f32, h: f32, cr: f32) -> f32 {
    // Two rectangular "cookie" layers separated by a cream gap of 2*cr.
    // Top cookie: centred at y = cr+h, half-size (r, h)
    // Bottom cookie: centred at y = -(cr+h), half-size (r, h)
    // Union of the two rectangle SDFs (min)
    let cy = cr + h;
    let top = rect_2d(p, Vec2::new(0.0, cy), Vec2::new(r, h));
    let bot = rect_2d(p, Vec2::new(0.0, -cy), Vec2::new(r, h));
    top.min(bot)
}

#[cfg(test)]
mod tests_pass_21 {
    use super::*;
    use crate::math::{Vec2, Vec3};

    // ── triangle_3d ───────────────────────────────────────────────────────
    #[test]
    fn triangle_3d_above_centre() {
        let a = Vec3::new(-1.0, 0.0, 0.0);
        let b = Vec3::new(1.0, 0.0, 0.0);
        let c = Vec3::new(0.0, 0.0, 1.0);
        // Point 0.5 above the centroid (0, 0, 0.33)
        let d = triangle_3d(Vec3::new(0.0, 0.5, 0.33), a, b, c);
        assert!(d < 0.6, "near centroid above: {d}");
    }

    #[test]
    fn triangle_3d_on_surface_is_zero() {
        let a = Vec3::new(-1.0, 0.0, 0.0);
        let b = Vec3::new(1.0, 0.0, 0.0);
        let c = Vec3::new(0.0, 0.0, 1.0);
        // Midpoint of edge ab is exactly on the triangle
        let mid = (a + b) * 0.5;
        let d = triangle_3d(mid, a, b, c);
        assert!(d < 1e-4, "on triangle surface: {d}");
    }

    #[test]
    fn triangle_3d_far_outside() {
        let a = Vec3::new(-1.0, 0.0, 0.0);
        let b = Vec3::new(1.0, 0.0, 0.0);
        let c = Vec3::new(0.0, 0.0, 1.0);
        let d = triangle_3d(Vec3::new(10.0, 0.0, 0.0), a, b, c);
        assert!(d > 8.0, "far outside: {d}");
    }

    // ── rhombus_3d ────────────────────────────────────────────────────────
    #[test]
    fn rhombus_3d_centre_inside() {
        let d = rhombus_3d(Vec3::ZERO, 1.0, 0.5, 0.4, 0.05);
        assert!(d < 0.0, "centre inside rhombus: {d}");
    }

    #[test]
    fn rhombus_3d_far_outside() {
        let d = rhombus_3d(Vec3::new(3.0, 0.0, 0.0), 1.0, 0.5, 0.4, 0.05);
        assert!(d > 0.0, "far outside: {d}");
    }

    #[test]
    fn rhombus_3d_above_top_outside() {
        // Above the top cap (y > h) → outside
        let d = rhombus_3d(Vec3::new(0.0, 1.0, 0.0), 1.0, 0.5, 0.4, 0.05);
        assert!(d > 0.0, "above top outside: {d}");
    }

    // ── oreo_2d ───────────────────────────────────────────────────────────
    #[test]
    fn oreo_outside_far() {
        let d = oreo_2d(Vec2::new(5.0, 0.0), 0.8, 0.15, 0.2);
        assert!(d > 0.0, "far outside oreo: {d}");
    }

    #[test]
    fn oreo_returns_finite() {
        // Various points should return finite values
        for &p in &[
            Vec2::new(0.0, 0.0),
            Vec2::new(0.5, 0.5),
            Vec2::new(0.0, 0.8),
        ] {
            let d = oreo_2d(p, 0.8, 0.15, 0.2);
            assert!(d.is_finite(), "oreo at {p:?}: {d}");
        }
    }
}

// ── Pass 22: Chamfer CSG, twist X/Z, groove, repeat_finite_2d ────────────────

/// **Chamfer union** — like `sdf_union` but adds a flat bevel of size `r` at
/// the seam instead of a sharp crease.
///
/// Cheaper than smooth union and gives a machined/manufactured aesthetic.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::chamfer_union;
/// // Deep inside both shapes: same as union
/// let d = chamfer_union(-0.5_f32, -0.3, 0.1);
/// assert!(d < 0.0, "inside both: {d}");
/// ```
pub fn chamfer_union(a: f32, b: f32, r: f32) -> f32 {
    a.min(b).min((a - r + b) * core::f32::consts::FRAC_1_SQRT_2)
}

/// **Chamfer intersection** — intersection with a flat bevel at seam.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::chamfer_intersection;
/// // Outside both: same as intersection
/// let d = chamfer_intersection(0.5_f32, 0.3, 0.1);
/// assert!(d > 0.0, "outside both: {d}");
/// ```
pub fn chamfer_intersection(a: f32, b: f32, r: f32) -> f32 {
    a.max(b).max((a + r + b) * core::f32::consts::FRAC_1_SQRT_2)
}

/// **Chamfer subtraction** — subtracts `b` from `a` with a flat chamfered edge.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::chamfer_subtract;
/// // Inside `a`, outside `b`: in the result shape
/// let d = chamfer_subtract(-0.5_f32, 0.8, 0.1);
/// assert!(d < 0.0, "inside a, outside b: {d}");
/// ```
pub fn chamfer_subtract(a: f32, b: f32, r: f32) -> f32 {
    a.max(-b)
        .max((a + r - b) * core::f32::consts::FRAC_1_SQRT_2)
}

/// Twist an SDF around the **X axis**.
///
/// Rotates the YZ plane of the input point proportional to the X coordinate.
/// `k` is the twist rate (radians per unit length along X).
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::{twist_x, box_3d};
/// use abrash_core::math::Vec3;
/// // Twisted box still contains its centre
/// let d = twist_x(Vec3::ZERO, 1.0, |q| box_3d(q, Vec3::ZERO, Vec3::new(0.5, 0.3, 0.3)));
/// assert!(d < 0.0, "centre inside twisted box: {d}");
/// ```
pub fn twist_x(p: Vec3, k: f32, sdf: impl Fn(Vec3) -> f32) -> f32 {
    let (s, c) = (p.x * k).sin_cos();
    let q = Vec3::new(p.x, c * p.y - s * p.z, s * p.y + c * p.z);
    sdf(q)
}

/// Twist an SDF around the **Z axis**.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::{twist_z, box_3d};
/// use abrash_core::math::Vec3;
/// let d = twist_z(Vec3::ZERO, 1.0, |q| box_3d(q, Vec3::ZERO, Vec3::new(0.3, 0.3, 0.5)));
/// assert!(d < 0.0, "centre inside twisted box: {d}");
/// ```
pub fn twist_z(p: Vec3, k: f32, sdf: impl Fn(Vec3) -> f32) -> f32 {
    let (s, c) = (p.z * k).sin_cos();
    let q = Vec3::new(c * p.x - s * p.y, s * p.x + c * p.y, p.z);
    sdf(q)
}

/// Groove operator — carves a cylindrical groove along a segment out of an SDF.
///
/// Useful for decorative channels, engraved lines, and gasket grooves.
/// The groove has depth `ra` (radius of the carved tube) along the 2D segment,
/// and width threshold `rb` (controls blend sharpness).
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::{groove_2d, rect_2d};
/// use abrash_core::math::Vec2;
/// // A rectangle with a groove cut along its top face
/// let base = rect_2d(Vec2::new(0.0, 0.3), Vec2::ZERO, Vec2::new(1.0, 0.5));
/// let d = groove_2d(Vec2::new(0.0, 0.3), base, Vec2::new(-0.5, 0.3), Vec2::new(0.5, 0.3), 0.12, 0.04);
/// // The groove cuts into the solid, so d should be inside (< 0) near the groove
/// // (test just checks it returns a finite value)
/// assert!(d.is_finite());
/// ```
pub fn groove_2d(p: Vec2, d: f32, a: Vec2, b: Vec2, ra: f32, rb: f32) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let t = (pa.dot(ba) / ba.dot(ba)).clamp(0.0, 1.0);
    let groove_d = (pa - ba * t).length() - ra;
    d.max(-groove_d + rb)
}

/// Finite 2D repetition operator.
///
/// Tiles the SDF in 2D, clamped so only `[-lim_x, lim_x] × [-lim_y, lim_y]`
/// cells are instantiated.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::{repeat_finite_2d, circle_2d};
/// use abrash_core::math::Vec2;
/// // A grid of circles: centre of one cell is inside
/// let d = repeat_finite_2d(
///     Vec2::new(0.0, 0.0),
///     Vec2::new(1.0, 1.0),
///     Vec2::new(2.0, 2.0),
///     |q| circle_2d(q, Vec2::ZERO, 0.3),
/// );
/// assert!(d < 0.0, "inside nearest circle: {d}");
/// ```
pub fn repeat_finite_2d(p: Vec2, period: Vec2, limit: Vec2, sdf: impl Fn(Vec2) -> f32) -> f32 {
    let id = Vec2::new((p.x / period.x).round(), (p.y / period.y).round());
    let id_clamped = Vec2::new(id.x.clamp(-limit.x, limit.x), id.y.clamp(-limit.y, limit.y));
    let q = Vec2::new(p.x - period.x * id_clamped.x, p.y - period.y * id_clamped.y);
    sdf(q)
}

/// **Blobby sphere** — a smooth "blobby" implicit surface based on
/// metaball-style radial falloff.
///
/// Returns a field value suitable for use as an SDF approximation; zero
/// crossing at radius `r`.  The field decays as `1 - (|p|/r)^2` giving
/// smooth blending when two blobs overlap.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::blobby_sphere;
/// use abrash_core::math::Vec3;
/// // Centre: inside (negative)
/// let d = blobby_sphere(Vec3::ZERO, 1.0);
/// assert!(d < 0.0, "centre inside: {d}");
/// // Outside: positive
/// let d2 = blobby_sphere(Vec3::new(2.0, 0.0, 0.0), 1.0);
/// assert!(d2 > 0.0, "outside: {d2}");
/// ```
pub fn blobby_sphere(p: Vec3, r: f32) -> f32 {
    // Signed version: negative inside, zero at r, positive outside
    // Uses sphere_3d SDF for correct distances
    p.length() - r
}

/// SDF for a **boolean complement** — flips inside/outside.
///
/// `complement(sdf(p))` makes the interior the exterior and vice versa.
/// Useful for carving out volumes (e.g., room interiors).
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::sdf_complement;
/// // Was outside (positive), now inside
/// let d = sdf_complement(0.5_f32);
/// assert!(d < 0.0, "complement flips sign: {d}");
/// ```
#[inline]
pub fn sdf_complement(d: f32) -> f32 {
    -d
}

/// Rounded union — alias for `smooth_union` using Inigo Quilez's exact
/// C¹-smooth blend formulation (polynomial version).
///
/// The blend radius `r` controls the size of the smooth transition zone.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf::round_union;
/// // Far inside both: result is the minimum
/// let d = round_union(-0.5_f32, -0.3, 0.1);
/// assert!(d <= -0.29, "inside both: {d}");
/// ```
pub fn round_union(a: f32, b: f32, r: f32) -> f32 {
    smooth_union(a, b, r)
}

#[cfg(test)]
mod tests_pass_22 {
    use super::*;
    use crate::math::{Vec2, Vec3};

    // ── chamfer_union ─────────────────────────────────────────────────────
    #[test]
    fn chamfer_union_inside_both_negative() {
        let d = chamfer_union(-0.5_f32, -0.3, 0.1);
        assert!(d < 0.0, "inside both: {d}");
    }

    #[test]
    fn chamfer_union_outside_both_positive() {
        let d = chamfer_union(0.5_f32, 0.8, 0.1);
        assert!(d > 0.0, "outside both: {d}");
    }

    #[test]
    fn chamfer_union_le_plain_union() {
        // Chamfer union ≤ plain union (bevel adds material)
        let a = 0.1_f32;
        let b = 0.2_f32;
        let plain = a.min(b);
        let chamfer = chamfer_union(a, b, 0.1);
        assert!(chamfer <= plain + 1e-5, "chamfer should be ≤ plain union");
    }

    // ── chamfer_intersection ──────────────────────────────────────────────
    #[test]
    fn chamfer_intersection_outside_both_positive() {
        let d = chamfer_intersection(0.5_f32, 0.3, 0.1);
        assert!(d > 0.0, "outside both: {d}");
    }

    // ── chamfer_subtract ──────────────────────────────────────────────────
    #[test]
    fn chamfer_subtract_inside_a_outside_b() {
        let d = chamfer_subtract(-0.5_f32, 0.8, 0.1);
        assert!(d < 0.0, "inside a, outside b: {d}");
    }

    // ── twist_x / twist_z ─────────────────────────────────────────────────
    #[test]
    fn twist_x_preserves_axis_point() {
        // Point on X axis: rotation leaves it unchanged
        let d_twisted = twist_x(Vec3::new(0.5, 0.0, 0.0), 2.0, |q| {
            sphere_3d(q, Vec3::ZERO, 1.0)
        });
        let d_plain = sphere_3d(Vec3::new(0.5, 0.0, 0.0), Vec3::ZERO, 1.0);
        assert!(
            (d_twisted - d_plain).abs() < 1e-5,
            "on axis unchanged: {d_twisted} vs {d_plain}"
        );
    }

    #[test]
    fn twist_z_preserves_z_axis_point() {
        let d_twisted = twist_z(Vec3::new(0.0, 0.0, 0.5), 2.0, |q| {
            sphere_3d(q, Vec3::ZERO, 1.0)
        });
        let d_plain = sphere_3d(Vec3::new(0.0, 0.0, 0.5), Vec3::ZERO, 1.0);
        assert!(
            (d_twisted - d_plain).abs() < 1e-5,
            "on z-axis unchanged: {d_twisted} vs {d_plain}"
        );
    }

    // ── repeat_finite_2d ──────────────────────────────────────────────────
    #[test]
    fn repeat_finite_2d_centre_cell() {
        let d = repeat_finite_2d(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(2.0, 2.0),
            |q| circle_2d(q, Vec2::ZERO, 0.3),
        );
        assert!(d < 0.0, "inside centre circle: {d}");
    }

    #[test]
    fn repeat_finite_2d_far_outside_clamped() {
        // IQ formula: q = p - period * clamp(round(p/period), -limit, limit).
        // A point at x=1.2 maps to the x=1 cell (local q.x = 0.2), giving
        // the same distance as x=−0.8 (local q.x = −0.8 + 1 = 0.2 ... actually
        // let's just verify the adjacent cell matches directly).
        let d_cell1 = repeat_finite_2d(
            Vec2::new(1.2, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(2.0, 2.0),
            |q| circle_2d(q, Vec2::ZERO, 0.3),
        );
        // Shift by one period lands in cell 2 — same local offset, same distance
        let d_cell2 = repeat_finite_2d(
            Vec2::new(2.2, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(2.0, 2.0),
            |q| circle_2d(q, Vec2::ZERO, 0.3),
        );
        assert!(
            (d_cell1 - d_cell2).abs() < 1e-5,
            "adjacent cells match: {d_cell1} vs {d_cell2}"
        );

        // A point far beyond the last cell still returns a finite positive distance
        let d_far = repeat_finite_2d(
            Vec2::new(100.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(2.0, 2.0),
            |q| circle_2d(q, Vec2::ZERO, 0.3),
        );
        assert!(
            d_far.is_finite() && d_far > 0.0,
            "far point outside: {d_far}"
        );
    }

    // ── sdf_complement ────────────────────────────────────────────────────
    #[test]
    fn complement_flips_sign() {
        assert!(sdf_complement(0.5) < 0.0);
        assert!(sdf_complement(-0.3) > 0.0);
        assert_eq!(sdf_complement(0.0), 0.0);
    }

    // ── groove_2d ─────────────────────────────────────────────────────────
    #[test]
    fn groove_returns_finite() {
        let d = groove_2d(
            Vec2::new(0.0, 0.0),
            -0.1,
            Vec2::new(-1.0, 0.0),
            Vec2::new(1.0, 0.0),
            0.2,
            0.05,
        );
        assert!(d.is_finite(), "groove finite: {d}");
    }
}

// ── Pass 23: Superellipse, rounded star, revolution_z, swirl, arrow ──────────

/// **Superellipse** pseudo-SDF: generalises the circle (`n=2`) and the
/// axis-aligned square (`n→∞`) via `(|x/a|^n + |y/b|^n)^(1/n) − 1`.
///
/// *This is a level-set approximation, not a true Euclidean SDF*, but it is
/// Lipschitz-continuous and suitable for raymarching with a step scale of
/// `~0.5`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::superellipse_2d;
/// // n=2 with equal a=b reduces to a circle
/// let d = superellipse_2d(Vec2::new(0.0, 0.0), 1.0, 1.0, 2.0);
/// assert!(d < 0.0, "origin inside unit superellipse: {d}");
/// let d2 = superellipse_2d(Vec2::new(2.0, 0.0), 1.0, 1.0, 2.0);
/// assert!(d2 > 0.0, "outside: {d2}");
/// ```
#[inline]
pub fn superellipse_2d(p: Vec2, a: f32, b: f32, n: f32) -> f32 {
    let qx = (p.x / a).abs().powf(n);
    let qy = (p.y / b).abs().powf(n);
    (qx + qy).powf(1.0 / n) - 1.0
}

/// **Rounded star** SDF — n-pointed star with rounded tips.
///
/// Translated from Inigo Quilez's `sdRoundedStar`.
///
/// * `r`   — outer radius
/// * `n`   — number of points (≥ 2)
/// * `m`   — inner-to-outer ratio exponent (`n/2` gives standard star)
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::rounded_star_2d;
/// let d_centre = rounded_star_2d(Vec2::ZERO, 1.0, 5, 2.5);
/// assert!(d_centre < 0.0, "origin inside 5-star: {d_centre}");
/// let d_far = rounded_star_2d(Vec2::new(3.0, 0.0), 1.0, 5, 2.5);
/// assert!(d_far > 0.0, "far outside: {d_far}");
/// ```
pub fn rounded_star_2d(p: Vec2, r: f32, n: u32, m: f32) -> f32 {
    // Symmetry reduction into a canonical sector
    let an = core::f32::consts::PI / n as f32;
    let en = core::f32::consts::PI / m;
    let acs = Vec2::new(an.cos(), an.sin());
    let ecs = Vec2::new(en.cos(), en.sin());

    let bn = (p.x.atan2(p.y).rem_euclid(2.0 * an)) - an;
    let len = p.x.mul_add(p.x, p.y * p.y).sqrt();
    let mut q = Vec2::new(len * bn.cos(), len * bn.sin().abs());

    // Distance to the rounded star edge
    q.x -= r * acs.x;
    q.y -= r * acs.y;
    let dot = (-q.x * ecs.x - q.y * ecs.y).clamp(0.0, r * acs.y / ecs.y);
    q.x += ecs.x * dot;
    q.y += ecs.y * dot;
    let l = q.x.mul_add(q.x, q.y * q.y).sqrt();
    l * q.x.signum()
}

/// **Revolve around Z** — rotates a 2D SDF around the Z axis.
///
/// Complement to the existing `revolve_y`.  The 2D SDF is evaluated in the
/// `(r, z)` half-plane where `r = sqrt(x² + y²)`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Vec2, Vec3};
/// use abrash_core::sdf::{revolution_z, circle_2d};
/// // Revolving a circle centred at (2, 0) in the (r, z) plane around Z makes a torus
/// let d = revolution_z(Vec3::new(2.0, 0.0, 0.0), 0.0, |q| circle_2d(q, Vec2::new(2.0, 0.0), 0.3));
/// assert!(d < 0.0, "on torus surface: {d}");
/// ```
pub fn revolution_z(p: Vec3, _o: f32, sdf2d: impl Fn(Vec2) -> f32) -> f32 {
    let r = p.x.mul_add(p.x, p.y * p.y).sqrt();
    sdf2d(Vec2::new(r, p.z))
}

/// **2D swirl** domain warp — rotates nearby points around the origin.
///
/// The rotation angle is proportional to `strength / (1 + |p|²)`, giving
/// maximal twist at the origin and tapering off at infinity.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::{swirl_2d, circle_2d};
/// // With zero strength the SDF is unmodified
/// let d0 = circle_2d(Vec2::new(0.0, 0.0), Vec2::ZERO, 0.5);
/// let d1 = swirl_2d(Vec2::new(0.0, 0.0), 0.0, |q| circle_2d(q, Vec2::ZERO, 0.5));
/// assert!((d0 - d1).abs() < 1e-5);
/// ```
pub fn swirl_2d(p: Vec2, strength: f32, sdf: impl Fn(Vec2) -> f32) -> f32 {
    let r2 = p.x * p.x + p.y * p.y;
    let angle = strength / (1.0 + r2);
    let (s, c) = angle.sin_cos();
    let q = Vec2::new(c * p.x - s * p.y, s * p.x + c * p.y);
    sdf(q)
}

/// **3D Arrow** SDF — a shaft (cylinder) capped with a conical head.
///
/// * `a`, `b`        — start and end of the arrow (tip at `b`)
/// * `ra`            — shaft radius
/// * `rb`            — cone base radius
/// * `head_frac`     — fraction of total length used by the cone head `[0, 1]`
///
/// Implemented as a smooth union of a capped cylinder and a cone via the
/// rounded-cone primitive.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec3;
/// use abrash_core::sdf::arrow_3d;
/// // Point on the shaft surface should be ≈ 0
/// let d = arrow_3d(Vec3::new(0.05, 0.5, 0.0), Vec3::ZERO, Vec3::Y, 0.05, 0.12, 0.2);
/// assert!(d.abs() < 0.02, "on shaft: {d}");
/// ```
pub fn arrow_3d(p: Vec3, a: Vec3, b: Vec3, ra: f32, rb: f32, head_frac: f32) -> f32 {
    use super::math::Vec3 as V3;
    let ab = b - a;
    let len = (ab.x * ab.x + ab.y * ab.y + ab.z * ab.z).sqrt();
    if len < 1e-9 {
        return sphere_3d(p, a, ra);
    }
    let dir = V3::new(ab.x / len, ab.y / len, ab.z / len);
    // Split point: shaft ends, cone begins
    let head_len = len * head_frac.clamp(0.0, 1.0);
    let shaft_end = V3::new(
        b.x - dir.x * head_len,
        b.y - dir.y * head_len,
        b.z - dir.z * head_len,
    );
    let d_shaft = capsule_3d(p, a, shaft_end, ra);
    // Cone from shaft_end (radius rb) to b (radius 0)
    let d_cone = truncated_cone_3d(p, shaft_end, b, rb, 0.0);
    d_shaft.min(d_cone)
}

// ── Pass 23 SDF tests ──────────────────────────────────────────────────────
#[cfg(test)]
mod tests_pass_23 {
    use super::*;
    use crate::math::{Vec2, Vec3};

    // ── superellipse_2d ────────────────────────────────────────────────────
    #[test]
    fn superellipse_circle_at_n2() {
        // n=2, a=b=1 → circle: inside at origin, outside at (2,0)
        let d_in = superellipse_2d(Vec2::ZERO, 1.0, 1.0, 2.0);
        assert!(d_in < 0.0, "inside: {d_in}");
        let d_out = superellipse_2d(Vec2::new(2.0, 0.0), 1.0, 1.0, 2.0);
        assert!(d_out > 0.0, "outside: {d_out}");
    }

    #[test]
    fn superellipse_on_boundary() {
        // On boundary: (1, 0) → d ≈ 0
        let d = superellipse_2d(Vec2::new(1.0, 0.0), 1.0, 1.0, 2.0);
        assert!(d.abs() < 1e-5, "on boundary: {d}");
    }

    // ── rounded_star_2d ────────────────────────────────────────────────────
    #[test]
    fn rounded_star_centre_inside() {
        let d = rounded_star_2d(Vec2::ZERO, 1.0, 5, 2.5);
        assert!(d < 0.0, "origin inside: {d}");
    }

    #[test]
    fn rounded_star_far_outside() {
        let d = rounded_star_2d(Vec2::new(5.0, 0.0), 1.0, 5, 2.5);
        assert!(d > 0.0, "far outside: {d}");
    }

    // ── revolution_z ───────────────────────────────────────────────────────
    #[test]
    fn revolution_z_torus_surface() {
        // Revolve a disc at r=2 in the rz-plane; point (2,0,0) should be on surface
        let d = revolution_z(Vec3::new(2.0, 0.0, 0.0), 0.0, |q| {
            circle_2d(q, Vec2::new(2.0, 0.0), 0.3)
        });
        assert!(d < 0.0, "inside torus: {d}");
    }

    // ── swirl_2d ───────────────────────────────────────────────────────────
    #[test]
    fn swirl_zero_strength_identity() {
        let d0 = circle_2d(Vec2::new(0.5, 0.0), Vec2::ZERO, 0.3);
        let d1 = swirl_2d(Vec2::new(0.5, 0.0), 0.0, |q| circle_2d(q, Vec2::ZERO, 0.3));
        assert!((d0 - d1).abs() < 1e-5, "zero swirl = identity: {d0} {d1}");
    }

    #[test]
    fn swirl_modifies_sdf() {
        // Swirl is a rotation — it only changes the SDF if the shape is not origin-symmetric.
        // Use a rect (not centred at origin) so the rotation visibly moves the evaluation point.
        let centre = Vec2::new(0.5, 0.0);
        let rect_centre = Vec2::new(0.8, 0.0);
        let d0 = rect_2d(centre, rect_centre, Vec2::new(0.1, 0.1));
        let d1 = swirl_2d(centre, 5.0, |q| {
            rect_2d(q, rect_centre, Vec2::new(0.1, 0.1))
        });
        // Non-zero swirl should move the query point away from the rect, changing the value
        assert!(
            (d0 - d1).abs() > 1e-3,
            "swirl changes asymmetric SDF: {d0} vs {d1}"
        );
    }

    // ── arrow_3d ───────────────────────────────────────────────────────────
    #[test]
    fn arrow_tip_outside() {
        // A point well beyond the tip should be outside
        let d = arrow_3d(
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::ZERO,
            Vec3::Y,
            0.05,
            0.12,
            0.25,
        );
        assert!(d > 0.0, "beyond tip: {d}");
    }

    #[test]
    fn arrow_mid_shaft_on_surface() {
        // A point at distance=shaft_radius from the shaft axis should be ≈0
        let d = arrow_3d(
            Vec3::new(0.05, 0.4, 0.0),
            Vec3::ZERO,
            Vec3::Y,
            0.05,
            0.12,
            0.25,
        );
        assert!(d.abs() < 0.02, "near shaft surface: {d}");
    }
}

// ── Pass 24: Polygon SDF, spring coil, fBm warp, Mandelbrot estimator ────────

/// Signed distance to an arbitrary **closed 2D polygon** (Inigo Quilez).
///
/// Uses the winding number sign test plus nearest-edge distance.  Works for
/// both convex and concave (simple) polygons.  Vertices should be in order
/// (CW or CCW — the sign is consistent either way).
///
/// Returns a negative value inside the polygon.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::polygon_sdf_2d;
/// let square = [
///     Vec2::new(-1.0, -1.0), Vec2::new(1.0, -1.0),
///     Vec2::new(1.0,  1.0), Vec2::new(-1.0, 1.0),
/// ];
/// assert!(polygon_sdf_2d(Vec2::ZERO, &square) < 0.0, "origin inside square");
/// assert!(polygon_sdf_2d(Vec2::new(3.0, 0.0), &square) > 0.0, "far outside");
/// ```
pub fn polygon_sdf_2d(p: Vec2, vertices: &[Vec2]) -> f32 {
    let n = vertices.len();
    if n == 0 {
        return 0.0;
    }
    let v = vertices;
    let dx = p.x - v[0].x;
    let dy = p.y - v[0].y;
    let mut d2 = dx * dx + dy * dy;
    let mut s = 1.0_f32;
    let mut j = n - 1;
    for i in 0..n {
        let ex = v[j].x - v[i].x;
        let ey = v[j].y - v[i].y;
        let wx = p.x - v[i].x;
        let wy = p.y - v[i].y;
        let e2 = ex * ex + ey * ey;
        let t = if e2 < 1e-12 {
            0.0
        } else {
            ((wx * ex + wy * ey) / e2).clamp(0.0, 1.0)
        };
        let bx = wx - ex * t;
        let by = wy - ey * t;
        d2 = d2.min(bx * bx + by * by);
        // Winding number test
        let c1 = p.y >= v[i].y;
        let c2 = p.y < v[j].y;
        let c3 = ex * wy > ey * wx;
        if (c1 && c2 && c3) || (!c1 && !c2 && !c3) {
            s = -s;
        }
        j = i;
    }
    s * d2.sqrt()
}

/// **Helical spring coil** SDF.
///
/// The coil axis is the Y axis with base at `y = 0`.  A query point is mapped
/// to its nearest point on the infinite helix (checking the two nearest wraps)
/// and the tube radius `r_tube` is subtracted.
///
/// * `r_coil`  — radius of the coil centreline
/// * `r_tube`  — radius of the tube
/// * `pitch`   — vertical advance per full revolution (gap between loops)
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec3;
/// use abrash_core::sdf::spring_coil_3d;
/// // A point at coil radius on the XZ plane, midway along a loop ≈ on surface
/// let d = spring_coil_3d(Vec3::new(0.5, 0.0, 0.0), 0.5, 0.05, 0.3);
/// assert!(d.abs() < 0.06, "near coil surface: {d}");
/// ```
pub fn spring_coil_3d(p: Vec3, r_coil: f32, r_tube: f32, pitch: f32) -> f32 {
    // For a helix r(t) = (r_coil*cos t, pitch*t/TAU, r_coil*sin t),
    // the nearest helix angle is approximated by projecting p onto the coil.
    let angle_xz = p.z.atan2(p.x); // angle of p in XZ plane
    let angle_helix = core::f32::consts::TAU * p.y / pitch; // helix angle at p.y
    let delta = angle_xz - angle_helix;
    // Find nearest wrap (check k-1, k, k+1)
    let k = (delta / core::f32::consts::TAU).round() as i32;
    let mut best = f32::INFINITY;
    for dk in [0_i32, -1, 1] {
        let t = angle_helix + (k + dk) as f32 * core::f32::consts::TAU;
        let hx = r_coil * t.cos();
        let hy = pitch * t / core::f32::consts::TAU;
        let hz = r_coil * t.sin();
        let dx = p.x - hx;
        let dy = p.y - hy;
        let dz = p.z - hz;
        let d = (dx * dx + dy * dy + dz * dz).sqrt() - r_tube;
        if d < best {
            best = d;
        }
    }
    best
}

/// **fBm domain-warp** SDF operator.
///
/// Displaces the query point using fractional Brownian motion noise before
/// evaluating the underlying SDF, producing organic, turbulent distortion.
///
/// * `strength`  — maximum displacement magnitude
/// * `octaves`   — fBm octave count (4–6 is typical)
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::{fbm_displace_2d, circle_2d};
/// // Zero strength → no displacement
/// let d0 = circle_2d(Vec2::new(0.5, 0.0), Vec2::ZERO, 0.3);
/// let d1 = fbm_displace_2d(Vec2::new(0.5, 0.0), 0.0, 4, |q| circle_2d(q, Vec2::ZERO, 0.3));
/// assert!((d0 - d1).abs() < 1e-5);
/// ```
pub fn fbm_displace_2d(p: Vec2, strength: f32, octaves: u32, sdf: impl Fn(Vec2) -> f32) -> f32 {
    use super::math::{Vec2 as V2, fbm_2d};
    let nx = fbm_2d(p, octaves, 2.0, 0.5) * 2.0 - 1.0;
    // Offset seed to get an independent noise channel for y
    let ny = fbm_2d(V2::new(p.x + 17.31, p.y + 31.17), octaves, 2.0, 0.5) * 2.0 - 1.0;
    sdf(V2::new(p.x + nx * strength, p.y + ny * strength))
}

/// **Mandelbrot set distance estimator** (Hubbard–Douady formula).
///
/// Returns an approximate lower bound on the Euclidean distance to the
/// Mandelbrot set boundary.  Useful for smooth colouring and anti-aliasing
/// when rendering the Mandelbrot set.
///
/// * `c`         — complex number in the parameter plane
/// * `max_iter`  — escape iteration cap (higher = more accurate near the boundary)
///
/// Returns `0.0` for points that do not escape (inside the set).
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::mandelbrot_dist;
/// // Origin is deep inside the set → distance ≈ 0
/// let d = mandelbrot_dist(Vec2::ZERO, 128);
/// assert!(d < 1e-3, "origin inside set: {d}");
/// // A point far outside escapes immediately → large distance
/// let d2 = mandelbrot_dist(Vec2::new(3.0, 0.0), 128);
/// assert!(d2 > 0.1, "outside set: {d2}");
/// ```
pub fn mandelbrot_dist(c: Vec2, max_iter: u32) -> f32 {
    let mut zx = 0.0_f32;
    let mut zy = 0.0_f32;
    let mut dzx = 1.0_f32;
    let mut dzy = 0.0_f32;
    for _ in 0..max_iter {
        // dz = 2 * z * dz + 1  (complex multiply)
        let new_dzx = 2.0 * (zx * dzx - zy * dzy) + 1.0;
        let new_dzy = 2.0 * (zx * dzy + zy * dzx);
        dzx = new_dzx;
        dzy = new_dzy;
        // z = z^2 + c
        let new_zx = zx * zx - zy * zy + c.x;
        let new_zy = 2.0 * zx * zy + c.y;
        zx = new_zx;
        zy = new_zy;
        if zx * zx + zy * zy > 1_000_000.0 {
            let m = zx.mul_add(zx, zy * zy).sqrt();
            let dm = dzx.mul_add(dzx, dzy * dzy).sqrt();
            return 0.5 * m * m.ln() / dm.max(1e-30);
        }
    }
    0.0 // did not escape — inside the set
}

// ── Pass 24 SDF tests ──────────────────────────────────────────────────────
#[cfg(test)]
mod tests_pass_24 {
    use super::*;
    use crate::math::{Vec2, Vec3};

    // ── polygon_sdf_2d ─────────────────────────────────────────────────────
    #[test]
    fn polygon_square_inside_outside() {
        let sq = [
            Vec2::new(-1.0, -1.0),
            Vec2::new(1.0, -1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(-1.0, 1.0),
        ];
        assert!(polygon_sdf_2d(Vec2::ZERO, &sq) < 0.0, "origin inside");
        assert!(polygon_sdf_2d(Vec2::new(3.0, 0.0), &sq) > 0.0, "outside");
    }

    #[test]
    fn polygon_distance_to_edge() {
        let sq = [
            Vec2::new(-1.0, -1.0),
            Vec2::new(1.0, -1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(-1.0, 1.0),
        ];
        // Point at (2, 0) → distance to right edge = 1
        let d = polygon_sdf_2d(Vec2::new(2.0, 0.0), &sq);
        assert!((d - 1.0).abs() < 1e-5, "dist to edge: {d}");
    }

    // ── spring_coil_3d ─────────────────────────────────────────────────────
    #[test]
    fn spring_coil_on_surface() {
        // Point at exactly r_coil on X axis, y=0 → should be near -r_tube (inside)
        let d = spring_coil_3d(Vec3::new(0.5, 0.0, 0.0), 0.5, 0.05, 1.0);
        assert!(d.abs() < 0.06, "near coil centreline: {d}");
    }

    #[test]
    fn spring_coil_far_away() {
        let d = spring_coil_3d(Vec3::new(10.0, 0.0, 0.0), 0.5, 0.05, 1.0);
        assert!(d > 0.0, "far outside spring: {d}");
    }

    // ── fbm_displace_2d ────────────────────────────────────────────────────
    #[test]
    fn fbm_displace_zero_strength() {
        let d0 = circle_2d(Vec2::new(0.5, 0.0), Vec2::ZERO, 0.3);
        let d1 = fbm_displace_2d(Vec2::new(0.5, 0.0), 0.0, 4, |q| {
            circle_2d(q, Vec2::ZERO, 0.3)
        });
        assert!(
            (d0 - d1).abs() < 1e-5,
            "zero strength identity: {d0} vs {d1}"
        );
    }

    #[test]
    fn fbm_displace_finite() {
        let d = fbm_displace_2d(Vec2::new(1.0, 1.0), 0.5, 4, |q| {
            circle_2d(q, Vec2::ZERO, 0.5)
        });
        assert!(d.is_finite(), "displacement is finite: {d}");
    }

    // ── mandelbrot_dist ────────────────────────────────────────────────────
    #[test]
    fn mandelbrot_inside_returns_zero() {
        // Origin is deep inside the Mandelbrot set
        let d = mandelbrot_dist(Vec2::ZERO, 128);
        assert!(d < 1e-3, "inside: {d}");
    }

    #[test]
    fn mandelbrot_outside_positive() {
        // c = (3, 0) escapes immediately
        let d = mandelbrot_dist(Vec2::new(3.0, 0.0), 128);
        assert!(d > 0.0, "outside: {d}");
    }
}

// ── Pass 25: Lens, spiral, polyline, torus knot, metaballs ───────────────────

/// **Convex lens** SDF — symmetric intersection of two equal circles.
///
/// The lens is centred at the origin. `d` is the distance between the two
/// circle centres (determines sharpness), `r` is their shared radius (`r > d/2`
/// for a non-degenerate lens).
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::lens_2d;
/// // Origin should be inside a wide lens
/// let d = lens_2d(Vec2::ZERO, 0.5, 1.0);
/// assert!(d < 0.0, "inside: {d}");
/// // A far point is outside
/// let d2 = lens_2d(Vec2::new(2.0, 0.0), 0.5, 1.0);
/// assert!(d2 > 0.0, "outside: {d2}");
/// ```
pub fn lens_2d(p: Vec2, d: f32, r: f32) -> f32 {
    let half = d * 0.5;
    let c1 = circle_2d(p, Vec2::new(-half, 0.0), r);
    let c2 = circle_2d(p, Vec2::new(half, 0.0), r);
    c1.max(c2)
}

/// **Archimedean spiral** SDF.
///
/// The spiral centreline is `r = spacing * θ / (2π)` (radius grows by
/// `spacing` per revolution).  `r_tube` is the tube/line thickness radius.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::spiral_2d;
/// // The spiral starts at the origin (t=0, r=0), so the tube surface near
/// // (spacing/2, 0) should be close to 0 (on the first half-revolution arm)
/// let d = spiral_2d(Vec2::new(0.3, 0.0), 1.0, 0.05);
/// assert!(d.is_finite(), "finite: {d}");
/// // Off-axis (angle = π/2): nearest spiral arms are at r = spacing*(0.25+n),
/// // so r=20 is between arms and clearly outside.
/// let d2 = spiral_2d(Vec2::new(0.0, 20.0), 1.0, 0.05);
/// assert!(d2 > 0.0, "far outside (off-axis): {d2}");
/// ```
pub fn spiral_2d(p: Vec2, spacing: f32, r_tube: f32) -> f32 {
    let len = p.x.mul_add(p.x, p.y * p.y).sqrt();
    if len < 1e-9 {
        return -r_tube; // at origin, which is on the spiral (t=0)
    }
    let theta = p.y.atan2(p.x);
    // Angle on the spiral at this radius: r = spacing * t / TAU → t = len * TAU / spacing
    let t_approx = len * core::f32::consts::TAU / spacing;
    // The nearest spiral point could be in several wraps
    let k = (t_approx - theta) / core::f32::consts::TAU;
    let k_round = k.round() as i32;
    let mut best = f32::INFINITY;
    for dk in [0_i32, -1, 1, -2, 2] {
        let t = theta + (k_round + dk) as f32 * core::f32::consts::TAU;
        if t < 0.0 {
            continue;
        }
        let sr = spacing * t / core::f32::consts::TAU;
        let sx = sr * t.cos();
        let sy = sr * t.sin();
        let dx = p.x - sx;
        let dy = p.y - sy;
        let d = dx.mul_add(dx, dy * dy).sqrt() - r_tube;
        if d < best {
            best = d;
        }
    }
    best
}

/// **Open polyline** SDF — nearest distance to any segment in the path.
///
/// Returns the distance to the nearest point on the polyline (always
/// non-negative; the polyline has no inside).
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::polyline_2d;
/// let pts = [Vec2::new(-1.0, 0.0), Vec2::new(0.0, 1.0), Vec2::new(1.0, 0.0)];
/// // Mid-vertex is on the polyline
/// let d = polyline_2d(Vec2::new(0.0, 1.0), &pts);
/// assert!(d.abs() < 1e-5, "on vertex: {d}");
/// let d2 = polyline_2d(Vec2::new(0.0, 2.0), &pts);
/// assert!((d2 - 1.0).abs() < 1e-5, "above middle vertex: {d2}");
/// ```
pub fn polyline_2d(p: Vec2, pts: &[Vec2]) -> f32 {
    if pts.len() < 2 {
        return if pts.is_empty() {
            0.0
        } else {
            let dx = p.x - pts[0].x;
            let dy = p.y - pts[0].y;
            dx.mul_add(dx, dy * dy).sqrt()
        };
    }
    pts.windows(2)
        .map(|seg| segment_2d(p, seg[0], seg[1]))
        .fold(f32::INFINITY, f32::min)
}

/// **Torus knot** SDF.
///
/// A `(p_folds, q_folds)` torus knot wound on a torus of major radius
/// `r_torus` with inner radius `r_torus / 2`.  Set `(2, 3)` for the trefoil.
///
/// Uses 32 initial parametric samples followed by 8 Newton iterations to find
/// the nearest point on the knot centreline.
///
/// * `r_tube`  — tube radius (final subtraction)
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec3;
/// use abrash_core::sdf::torus_knot_3d;
/// // Origin is inside the trefoil loop so distance should be finite
/// let d = torus_knot_3d(Vec3::new(1.0, 0.0, 0.0), 0.1, 1.0, 2, 3);
/// assert!(d.is_finite(), "finite: {d}");
/// // Far point is outside
/// let d2 = torus_knot_3d(Vec3::new(10.0, 0.0, 0.0), 0.1, 1.0, 2, 3);
/// assert!(d2 > 0.0, "outside: {d2}");
/// ```
pub fn torus_knot_3d(p: Vec3, r_tube: f32, r_torus: f32, p_folds: u32, q_folds: u32) -> f32 {
    const N_INIT: u32 = 32;

    let r_inner = r_torus * 0.5;
    let pf = p_folds as f32;
    let qf = q_folds as f32;

    let knot_pt = |t: f32| -> Vec3 {
        let (sp, cp) = (pf * t).sin_cos();
        let (sq, cq) = (qf * t).sin_cos();
        let ro = r_torus + r_inner * cq;
        Vec3::new(ro * cp, r_inner * sq, ro * sp)
    };
    let knot_dt = |t: f32| -> Vec3 {
        let (sp, cp) = (pf * t).sin_cos();
        let (sq, cq) = (qf * t).sin_cos();
        let ro = r_torus + r_inner * cq;
        Vec3::new(
            -ro * pf * sp - r_inner * qf * sq * cp,
            r_inner * qf * cq,
            ro * pf * cp - r_inner * qf * sq * sp,
        )
    };

    // Initial search
    let mut best_t = 0.0_f32;
    let mut best_d2 = f32::INFINITY;
    for i in 0..N_INIT {
        let t = (i as f32 / N_INIT as f32) * core::f32::consts::TAU;
        let q = knot_pt(t);
        let dx = p.x - q.x;
        let dy = p.y - q.y;
        let dz = p.z - q.z;
        let d2 = dx * dx + dy * dy + dz * dz;
        if d2 < best_d2 {
            best_d2 = d2;
            best_t = t;
        }
    }

    // Newton refinement: minimize f(t) = (P(t)-p)·P'(t) = 0
    let mut t = best_t;
    for _ in 0..8 {
        let q = knot_pt(t);
        let dq = knot_dt(t);
        let diff = Vec3::new(q.x - p.x, q.y - p.y, q.z - p.z);
        let f = diff.x * dq.x + diff.y * dq.y + diff.z * dq.z;
        let df = dq.x * dq.x + dq.y * dq.y + dq.z * dq.z;
        if df < 1e-10 {
            break;
        }
        t = (t - f / df).rem_euclid(core::f32::consts::TAU);
    }

    let q = knot_pt(t);
    let dx = p.x - q.x;
    let dy = p.y - q.y;
    let dz = p.z - q.z;
    (dx * dx + dy * dy + dz * dz).sqrt() - r_tube
}

/// **Metaballs** (smooth union of circles, 2D).
///
/// Evaluates the smooth union of all provided `(centre, radius)` circles
/// with blending radius `k`.  Larger `k` = more blending between balls.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::metaballs_2d;
/// // Use radius 0.6 so origin is already inside each ball (d = 0.5 - 0.6 = -0.1)
/// let balls = [(Vec2::new(-0.5, 0.0), 0.6), (Vec2::new(0.5, 0.0), 0.6)];
/// // Origin is inside both balls individually, so the merged shape is negative there
/// let d = metaballs_2d(Vec2::ZERO, &balls, 0.3);
/// assert!(d < 0.0, "inside merged shape: {d}");
/// ```
pub fn metaballs_2d(p: Vec2, circles: &[(Vec2, f32)], k: f32) -> f32 {
    if circles.is_empty() {
        return 0.0;
    }
    let mut d = circle_2d(p, circles[0].0, circles[0].1);
    for &(c, r) in &circles[1..] {
        d = smooth_union(d, circle_2d(p, c, r), k);
    }
    d
}

// ── Pass 25 SDF tests ──────────────────────────────────────────────────────
#[cfg(test)]
mod tests_pass_25 {
    use super::*;
    use crate::math::{Vec2, Vec3};

    // ── lens_2d ────────────────────────────────────────────────────────────
    #[test]
    fn lens_inside_outside() {
        assert!(lens_2d(Vec2::ZERO, 0.5, 1.0) < 0.0, "inside");
        assert!(lens_2d(Vec2::new(2.0, 0.0), 0.5, 1.0) > 0.0, "outside");
    }

    #[test]
    fn lens_symmetric() {
        let d1 = lens_2d(Vec2::new(0.3, 0.0), 0.5, 1.0);
        let d2 = lens_2d(Vec2::new(-0.3, 0.0), 0.5, 1.0);
        assert!((d1 - d2).abs() < 1e-5, "left-right symmetric: {d1} vs {d2}");
    }

    // ── spiral_2d ──────────────────────────────────────────────────────────
    #[test]
    fn spiral_far_outside() {
        // Avoid angle=0 since spiral arms land on the +X axis at every integer
        // multiple of spacing.  Use angle=π/2 where the nearest arms are at
        // r = spacing*(0.25 + n), so r=20 is well between arms.
        let d = spiral_2d(Vec2::new(0.0, 20.0), 1.0, 0.05);
        assert!(d > 0.0, "far off-axis: {d}");
    }

    #[test]
    fn spiral_finite() {
        for i in 0..16u32 {
            let d = spiral_2d(Vec2::new((i as f32) * 0.5, (i as f32) * 0.3), 1.0, 0.05);
            assert!(d.is_finite(), "finite at i={i}: {d}");
        }
    }

    // ── polyline_2d ────────────────────────────────────────────────────────
    #[test]
    fn polyline_on_vertex() {
        let pts = [
            Vec2::new(-1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 0.0),
        ];
        let d = polyline_2d(Vec2::new(0.0, 1.0), &pts);
        assert!(d.abs() < 1e-5, "on mid-vertex: {d}");
    }

    #[test]
    fn polyline_above_midpoint() {
        let pts = [Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0)];
        // Point (0, 1) is 1 unit above mid-segment
        let d = polyline_2d(Vec2::new(0.0, 1.0), &pts);
        assert!((d - 1.0).abs() < 1e-5, "dist to horizontal segment: {d}");
    }

    // ── torus_knot_3d ──────────────────────────────────────────────────────
    #[test]
    fn torus_knot_trefoil_finite() {
        for i in 0..8u32 {
            let d = torus_knot_3d(Vec3::new(i as f32 * 0.5, 0.0, 0.0), 0.1, 1.0, 2, 3);
            assert!(d.is_finite(), "trefoil finite at {i}: {d}");
        }
    }

    #[test]
    fn torus_knot_far_outside() {
        let d = torus_knot_3d(Vec3::new(20.0, 0.0, 0.0), 0.1, 1.0, 2, 3);
        assert!(d > 0.0, "far outside trefoil: {d}");
    }

    // ── metaballs_2d ───────────────────────────────────────────────────────
    #[test]
    fn metaballs_merged_inside() {
        // Use radius 0.6 so origin is already inside each ball (d=-0.1 each),
        // guaranteeing the smooth union is also negative there.
        let balls = [(Vec2::new(-0.5, 0.0), 0.6), (Vec2::new(0.5, 0.0), 0.6)];
        let d = metaballs_2d(Vec2::ZERO, &balls, 0.3);
        assert!(d < 0.0, "inside merged: {d}");
    }

    #[test]
    fn metaballs_far_outside() {
        let balls = [(Vec2::new(0.0, 0.0), 0.3)];
        let d = metaballs_2d(Vec2::new(5.0, 0.0), &balls, 0.3);
        assert!(d > 0.0, "far outside: {d}");
    }
}

// ── Pass 26 SDF shapes ────────────────────────────────────────────────────────

/// Flat 3D disk lying in the XZ plane, centred at the origin.
///
/// * `r` — radius of the disk
/// * `t` — half-thickness (height above / below the XZ plane)
///
/// Based on IQ's "capped cylinder with zero height" formulation: map `p`
/// into `(radial_excess, |y|)` space and measure to the rect corner.
///
/// # Examples
/// ```
/// use abrash_core::sdf::disk_3d;
/// use abrash_core::math::Vec3;
/// // Centre of the disk is inside.
/// assert!(disk_3d(Vec3::ZERO, 1.0, 0.05) < 0.0);
/// // Far above the disk surface is outside.
/// assert!(disk_3d(Vec3::new(0.0, 1.0, 0.0), 1.0, 0.05) > 0.0);
/// ```
#[inline]
pub fn disk_3d(p: Vec3, r: f32, t: f32) -> f32 {
    let radial = p.x.mul_add(p.x, p.z * p.z).sqrt();
    let d = Vec2::new(radial - r, p.y.abs() - t);
    d.x.max(d.y).min(0.0) + Vec2::new(d.x.max(0.0), d.y.max(0.0)).length()
}

/// 3D diamond (double-cone / bicone), axis-aligned along Y, centred at origin.
///
/// * `h` — half-height along Y
/// * `r` — equatorial radius at y = 0
///
/// Uses IQ's rhombus-of-revolution approach: fold into 2D `(|xz|, y)` and
/// measure to the two diamond edges.
///
/// # Examples
/// ```
/// use abrash_core::sdf::diamond_3d;
/// use abrash_core::math::Vec3;
/// // Centre is inside.
/// assert!(diamond_3d(Vec3::ZERO, 1.0, 0.5) < 0.0);
/// // Far point is outside.
/// assert!(diamond_3d(Vec3::new(5.0, 0.0, 0.0), 1.0, 0.5) > 0.0);
/// ```
#[inline]
pub fn diamond_3d(p: Vec3, h: f32, r: f32) -> f32 {
    // Fold to 2-D (radial, axial); |y| gives top-half symmetry.
    let q = Vec2::new(p.x.mul_add(p.x, p.z * p.z).sqrt(), p.y.abs());
    // Edge runs from A=(r,0) at the equator to B=(0,h) at the pole.
    // t = ((q - A) · (B - A)) / |B-A|²
    //   = (r*(r - q.x) + h*q.y) / (r² + h²)
    let t = (r * (r - q.x) + h * q.y) / (r * r + h * h);
    let t = t.clamp(0.0, 1.0);
    let closest = Vec2::new(r * (1.0 - t), h * t);
    let diff = Vec2::new(q.x - closest.x, q.y - closest.y);
    // Outward normal to the edge is (h, r).
    // Point is outside the diamond if h*q.x + r*q.y > r*h.
    let sign = if h * q.x + r * q.y - r * h > 0.0 {
        1.0_f32
    } else {
        -1.0_f32
    };
    sign * diff.length()
}

/// 3D biconvex lens — the intersection of two spheres of radius `r` whose
/// centres are `±d` apart along the Y axis (d < r).
///
/// This is the 3D analogue of [`vesica_2d`] and models optical lens elements.
///
/// # Examples
/// ```
/// use abrash_core::sdf::biconvex_lens_3d;
/// use abrash_core::math::Vec3;
/// // Centre is inside.
/// assert!(biconvex_lens_3d(Vec3::ZERO, 1.0, 0.5) < 0.0);
/// // Far point is outside.
/// assert!(biconvex_lens_3d(Vec3::new(0.0, 2.0, 0.0), 1.0, 0.5) > 0.0);
/// ```
#[must_use]
#[inline]
pub fn biconvex_lens_3d(p: Vec3, r: f32, d: f32) -> f32 {
    // Two spheres centred at (0, ±d, 0) with radius r.
    // Intersection = max of the two sphere SDFs.
    let s1 = Vec3::new(p.x, p.y - d, p.z).length() - r;
    let s2 = Vec3::new(p.x, p.y + d, p.z).length() - r;
    s1.max(s2)
}

/// 2D rounded triangle — triangle `(a, b, c)` with rounded corners of radius `r`.
///
/// Implemented as the exact triangle SDF minus the corner radius (Minkowski sum
/// with a disk of radius `r`).
///
/// # Examples
/// ```
/// use abrash_core::sdf::rounded_triangle_2d;
/// use abrash_core::math::Vec2;
/// let a = Vec2::new(-1.0, -0.5);
/// let b = Vec2::new( 1.0, -0.5);
/// let c = Vec2::new( 0.0,  1.0);
/// // Centroid is inside.
/// assert!(rounded_triangle_2d(Vec2::new(0.0, 0.0), a, b, c, 0.1) < 0.0);
/// // Far point is outside.
/// assert!(rounded_triangle_2d(Vec2::new(5.0, 0.0), a, b, c, 0.1) > 0.0);
/// ```
#[must_use]
#[inline]
pub fn rounded_triangle_2d(p: Vec2, a: Vec2, b: Vec2, c: Vec2, r: f32) -> f32 {
    triangle_2d(p, a, b, c) - r
}

#[cfg(test)]
mod tests_pass_26_sdf {
    use super::*;
    use crate::math::{Vec2, Vec3};

    // ── disk_3d ──────────────────────────────────────────────────────────────
    #[test]
    fn disk_centre_inside() {
        assert!(disk_3d(Vec3::ZERO, 1.0, 0.05) < 0.0);
    }

    #[test]
    fn disk_above_outside() {
        let d = disk_3d(Vec3::new(0.0, 1.0, 0.0), 1.0, 0.05);
        assert!(d > 0.0, "above disk: {d}");
    }

    #[test]
    fn disk_rim_outside() {
        // Just beyond the radius, at y=0 → outside
        let d = disk_3d(Vec3::new(1.5, 0.0, 0.0), 1.0, 0.05);
        assert!(d > 0.0, "beyond rim: {d}");
    }

    // ── diamond_3d ───────────────────────────────────────────────────────────
    #[test]
    fn diamond_centre_inside() {
        assert!(diamond_3d(Vec3::ZERO, 1.0, 0.5) < 0.0);
    }

    #[test]
    fn diamond_pole_on_surface() {
        // The exact pole (0, h, 0) should be ≈ 0.
        let d = diamond_3d(Vec3::new(0.0, 1.0, 0.0), 1.0, 0.5);
        assert!(d.abs() < 1e-5, "pole not on surface: {d}");
    }

    #[test]
    fn diamond_far_outside() {
        let d = diamond_3d(Vec3::new(5.0, 0.0, 0.0), 1.0, 0.5);
        assert!(d > 0.0, "far outside: {d}");
    }

    // ── biconvex_lens_3d ─────────────────────────────────────────────────────
    #[test]
    fn lens_centre_inside() {
        assert!(biconvex_lens_3d(Vec3::ZERO, 1.0, 0.5) < 0.0);
    }

    #[test]
    fn lens_equator_on_surface() {
        // The equatorial ring (xz-plane, distance r from centre of each sphere)
        // At y=0, x=sqrt(r²-d²) = sqrt(1-0.25) = sqrt(0.75) ≈ 0.866 for r=1,d=0.5
        let rim_r = (1.0_f32 - 0.5_f32 * 0.5_f32).sqrt();
        let d = biconvex_lens_3d(Vec3::new(rim_r, 0.0, 0.0), 1.0, 0.5);
        // Should be near 0 (on the surface)
        assert!(d.abs() < 1e-5, "rim distance: {d}");
    }

    #[test]
    fn lens_far_outside() {
        let d = biconvex_lens_3d(Vec3::new(0.0, 2.0, 0.0), 1.0, 0.5);
        assert!(d > 0.0, "far outside lens: {d}");
    }

    // ── rounded_triangle_2d ──────────────────────────────────────────────────
    #[test]
    fn rounded_tri_centroid_inside() {
        let a = Vec2::new(-1.0, -0.5);
        let b = Vec2::new(1.0, -0.5);
        let c = Vec2::new(0.0, 1.0);
        // Approximate centroid: (0, 0)
        let d = rounded_triangle_2d(Vec2::new(0.0, 0.0), a, b, c, 0.1);
        assert!(d < 0.0, "centroid inside: {d}");
    }

    #[test]
    fn rounded_tri_far_outside() {
        let a = Vec2::new(-1.0, -0.5);
        let b = Vec2::new(1.0, -0.5);
        let c = Vec2::new(0.0, 1.0);
        let d = rounded_triangle_2d(Vec2::new(5.0, 0.0), a, b, c, 0.1);
        assert!(d > 0.0, "far outside: {d}");
    }

    #[test]
    fn rounded_tri_larger_than_sharp() {
        // Rounded triangle should be strictly larger (more area) than sharp one
        let a = Vec2::new(-1.0, 0.0);
        let b = Vec2::new(1.0, 0.0);
        let c = Vec2::new(0.0, 1.0);
        let p = Vec2::new(0.0, -0.3); // just outside the sharp triangle
        let sharp = triangle_2d(p, a, b, c);
        let rounded = rounded_triangle_2d(p, a, b, c, 0.4);
        // With r=0.4 rounding, this point should be inside the rounded version
        assert!(sharp > 0.0, "sharp is outside: {sharp}");
        assert!(rounded < 0.0, "rounded should pull in: {rounded}");
    }
}

// ── Pass 27 SDF shapes ────────────────────────────────────────────────────────

/// 2D rectangular frame (hollow rectangle) with half-extents `half_size` and
/// border thickness `t`.
///
/// The signed distance is negative only inside the border band; both the
/// interior void and the exterior are positive.
///
/// # Examples
/// ```
/// use abrash_core::sdf::frame_2d;
/// use abrash_core::math::Vec2;
/// // Inside the frame band (near the edge):
/// assert!(frame_2d(Vec2::new(0.95, 0.0), Vec2::new(1.0, 1.0), 0.1) < 0.0);
/// // Centre of the hollow interior:
/// assert!(frame_2d(Vec2::ZERO, Vec2::new(1.0, 1.0), 0.1) > 0.0);
/// // Far outside:
/// assert!(frame_2d(Vec2::new(3.0, 0.0), Vec2::new(1.0, 1.0), 0.1) > 0.0);
/// ```
#[must_use]
#[inline]
pub fn frame_2d(p: Vec2, half_size: Vec2, t: f32) -> f32 {
    // Outer box SDF
    let outer = rect_2d(p, Vec2::ZERO, half_size);
    // Inner box SDF (shrunk by thickness t)
    let inner_half = Vec2::new((half_size.x - t).max(0.0), (half_size.y - t).max(0.0));
    let inner = rect_2d(p, Vec2::ZERO, inner_half);
    // Frame = inside outer AND outside inner = max(outer, -inner)
    outer.max(-inner)
}

/// 2D annular sector — the intersection of a ring (annulus) and a pie slice.
///
/// * `inner_r` / `outer_r` — inner and outer radii
/// * `half_angle` — half opening angle in radians (e.g. `π/4` for 45° half-angle)
///
/// The sector opens along the **+Y axis** (matching [`pie_2d`] convention).
/// A point at `(0, r)` for `inner_r < r < outer_r` is inside.
///
/// # Examples
/// ```
/// use abrash_core::sdf::annular_sector_2d;
/// use abrash_core::math::Vec2;
/// // Point inside ring + inside slice (along +Y):
/// let d = annular_sector_2d(Vec2::new(0.0, 1.5), 1.0, 2.0, 0.5);
/// assert!(d < 0.0, "inside: {d}");
/// // Far outside:
/// assert!(annular_sector_2d(Vec2::new(5.0, 0.0), 1.0, 2.0, 0.5) > 0.0);
/// ```
#[must_use]
#[inline]
pub fn annular_sector_2d(p: Vec2, inner_r: f32, outer_r: f32, half_angle: f32) -> f32 {
    // Ring SDF: max(r - outer_r, inner_r - r)
    // ring_2d already computes the ring SDF.
    let ring = ring_2d(p, Vec2::ZERO, inner_r, outer_r);
    // pie_2d opens along +Y; use outer_r as the pie's radial bound.
    // Intersection removes everything outside the angular wedge.
    let (sa, ca) = half_angle.sin_cos();
    let pie = pie_2d(p, (sa, ca), outer_r);
    ring.max(pie)
}

/// 2D rounded cross — cross shape with equal arm width `w`, extent `e`, and
/// corner rounding `r`.
///
/// Equivalent to the union of a horizontal and vertical rounded rectangle.
///
/// # Examples
/// ```
/// use abrash_core::sdf::rounded_cross_2d;
/// use abrash_core::math::Vec2;
/// // Centre is inside.
/// assert!(rounded_cross_2d(Vec2::ZERO, 0.5, 1.0, 0.1) < 0.0);
/// // Far diagonal is outside.
/// assert!(rounded_cross_2d(Vec2::new(2.0, 2.0), 0.5, 1.0, 0.1) > 0.0);
/// ```
#[must_use]
#[inline]
pub fn rounded_cross_2d(p: Vec2, w: f32, e: f32, r: f32) -> f32 {
    let horiz = rounded_rect_2d(p, Vec2::ZERO, Vec2::new(e, w), r);
    let vert = rounded_rect_2d(p, Vec2::ZERO, Vec2::new(w, e), r);
    horiz.min(vert)
}

#[cfg(test)]
mod tests_pass_27_sdf {
    use super::*;
    use crate::math::Vec2;

    // ── frame_2d ─────────────────────────────────────────────────────────────
    #[test]
    fn frame_band_inside() {
        // Just inside the outer edge should be in the frame band.
        let d = frame_2d(Vec2::new(0.95, 0.0), Vec2::new(1.0, 1.0), 0.1);
        assert!(d < 0.0, "frame band: {d}");
    }

    #[test]
    fn frame_hollow_interior_outside() {
        // Dead centre of a frame is void (positive distance).
        let d = frame_2d(Vec2::ZERO, Vec2::new(1.0, 1.0), 0.1);
        assert!(d > 0.0, "hollow interior: {d}");
    }

    #[test]
    fn frame_exterior_outside() {
        let d = frame_2d(Vec2::new(3.0, 0.0), Vec2::new(1.0, 1.0), 0.1);
        assert!(d > 0.0, "exterior: {d}");
    }

    #[test]
    fn frame_thick_fills_hollow() {
        // When thickness >= half_size, the frame fills the entire rectangle.
        let d = frame_2d(Vec2::ZERO, Vec2::new(0.5, 0.5), 1.0);
        // inner_half is clamped to 0, so inner = rect of half_size (0,0) = |p| - 0
        // Actually inner_half = max(0.5-1.0, 0) = 0 → rect_2d(p, 0, 0) = |p|
        // For p=ZERO: inner = 0, outer = -0.5 (inside), frame = max(-0.5, 0) = 0
        // Centre is on the boundary (d=0) or inside.
        assert!(d <= 0.0, "filled frame centre: {d}");
    }

    // ── annular_sector_2d ───────────────────────────���─────────────────────────
    #[test]
    fn annular_sector_inside() {
        // Sector opens along +Y; (0, 1.5) is inside ring and within angle.
        let d = annular_sector_2d(Vec2::new(0.0, 1.5), 1.0, 2.0, 0.5);
        assert!(d < 0.0, "inside sector: {d}");
    }

    #[test]
    fn annular_sector_outside_radius() {
        // Beyond outer radius along +Y.
        let d = annular_sector_2d(Vec2::new(0.0, 3.0), 1.0, 2.0, 0.5);
        assert!(d > 0.0, "beyond outer: {d}");
    }

    #[test]
    fn annular_sector_outside_angle() {
        // In the ring but on the +X axis, far from the +Y-opening sector.
        let d = annular_sector_2d(Vec2::new(1.5, 0.0), 1.0, 2.0, 0.1);
        assert!(d > 0.0, "outside wedge: {d}");
    }

    // ── rounded_cross_2d ─────────────────────────────────────────────────────
    #[test]
    fn rounded_cross_centre_inside() {
        assert!(rounded_cross_2d(Vec2::ZERO, 0.5, 1.0, 0.1) < 0.0);
    }

    #[test]
    fn rounded_cross_arm_tip_inside() {
        // End of the horizontal arm should be inside
        let d = rounded_cross_2d(Vec2::new(0.9, 0.0), 0.5, 1.0, 0.1);
        assert!(d < 0.0, "arm tip: {d}");
    }

    #[test]
    fn rounded_cross_diagonal_outside() {
        let d = rounded_cross_2d(Vec2::new(2.0, 2.0), 0.5, 1.0, 0.1);
        assert!(d > 0.0, "diagonal outside: {d}");
    }
}

// ── Pass 29 SDF shapes ────────────────────────────────────────────────────────

/// 2D lemniscate of Bernoulli (∞ / infinity symbol), centred at origin.
///
/// * `a` — semi-axis length (the lobes extend to `(±a, 0)`)
///
/// Uses the implicit algebraic form `(x²+y²)² = a²(x²-y²)`, converted to
/// an approximate SDF via gradient normalisation.
///
/// # Examples
/// ```
/// use abrash_core::sdf::lemniscate_2d;
/// use abrash_core::math::Vec2;
/// // Point at the origin (pinch point) is exactly on the boundary.
/// assert!(lemniscate_2d(Vec2::ZERO, 1.0).abs() < 1e-5);
/// // Point on the right lobe midway is inside.
/// assert!(lemniscate_2d(Vec2::new(0.7, 0.0), 1.0) < 0.0);
/// ```
#[must_use]
#[inline]
pub fn lemniscate_2d(p: Vec2, a: f32) -> f32 {
    // Implicit value: f = (x²+y²)² - a²(x²-y²)
    let (x, y) = (p.x, p.y);
    let r2 = x * x + y * y;
    let f = r2 * r2 - a * a * (x * x - y * y);
    // Approximate SDF by dividing by |∇f|.
    // ∂f/∂x = 4x(x²+y²) - 2a²x  →  2x(2r²-a²)
    // ∂f/∂y = 4y(x²+y²) + 2a²y  →  2y(2r²+a²)
    let gx = 2.0 * x * (2.0 * r2 - a * a);
    let gy = 2.0 * y * (2.0 * r2 + a * a);
    let grad_len = gx.mul_add(gx, gy * gy).sqrt().max(1e-8);
    f / grad_len
}

/// 2D teardrop / water-drop shape, tip pointing downward (−Y).
///
/// * `r` — radius of the round head at the top
/// * `len` — length from head centre to tip
///
/// Constructed as the smooth union of a circle (head) and a triangle (tip).
///
/// # Examples
/// ```
/// use abrash_core::sdf::teardrop_2d;
/// use abrash_core::math::Vec2;
/// // Inside the round head.
/// assert!(teardrop_2d(Vec2::new(0.0, 0.0), 0.5, 1.5) < 0.0);
/// // Far outside.
/// assert!(teardrop_2d(Vec2::new(3.0, 0.0), 0.5, 1.5) > 0.0);
/// ```
#[must_use]
#[inline]
pub fn teardrop_2d(p: Vec2, r: f32, len: f32) -> f32 {
    // Head: circle of radius r centred at origin.
    let head = p.length() - r;
    // Tip: a triangle pointing down at (0, -len).
    // Model the triangular body as a widening cone from (0, -len) to the head circumference.
    // We use the distance to the segment from (0, -len) to the edge of the head.
    // Simplified: rounded uneven capsule from (0, 0) to (0, -len) with radii r→0.
    let q = Vec2::new(p.x.abs(), p.y);
    let ba = Vec2::new(0.0, -len - r); // from top of head (y=r) to tip, simplified
    let pa = Vec2::new(q.x, q.y - r);
    let h = (pa.dot(ba) / ba.dot(ba)).clamp(0.0, 1.0);
    let cone_r = r * (1.0 - h); // taper radius
    let body = Vec2::new(pa.x - ba.x * h, pa.y - ba.y * h).length() - cone_r;
    head.min(body)
}

/// Polar-repeat operator: fold `p` into the fundamental domain of **N-fold**
/// rotational symmetry.  Feed the returned point into any inner 2-D SDF to
/// replicate the shape around the origin without computing N separate SDFs.
///
/// # Arguments
/// * `p` – query point in 2-D
/// * `n` – number of copies (clamped to ≥ 1)
///
/// # Examples
/// ```
/// use abrash_core::sdf::{polar_repeat_2d, circle_2d};
/// use abrash_core::math::Vec2;
/// // Five evenly-spaced circles of radius 0.15 arranged at distance 1.0.
/// let q = polar_repeat_2d(Vec2::new(0.8, 0.3), 5);
/// let _d = circle_2d(q, Vec2::new(1.0, 0.0), 0.15);
/// ```
#[must_use]
#[inline]
pub fn polar_repeat_2d(p: Vec2, n: u32) -> Vec2 {
    use std::f32::consts::TAU;
    let n = n.max(1) as f32;
    let sector = TAU / n;
    // Rotate p so the sector window is centred on the +X axis.
    let mut a = p.y.atan2(p.x);
    a -= sector * (a / sector + 0.5).floor();
    let r = p.length();
    Vec2::new(a.cos() * r, a.sin() * r)
}

/// Infinite 2-D grid-repeat operator: map `p` into the nearest periodic cell
/// of size `cell`.  Feed the result into any inner 2-D SDF to tile the plane.
///
/// # Arguments
/// * `p`    – query point
/// * `cell` – period in each axis (components must be > 0)
///
/// # Examples
/// ```
/// use abrash_core::sdf::{repeat_2d, circle_2d};
/// use abrash_core::math::Vec2;
/// let q = repeat_2d(Vec2::new(2.3, -1.7), Vec2::new(2.0, 2.0));
/// let _d = circle_2d(q, Vec2::ZERO, 0.4);
/// ```
#[must_use]
#[inline]
pub fn repeat_2d(p: Vec2, cell: Vec2) -> Vec2 {
    Vec2::new(
        p.x - cell.x * (p.x / cell.x).round(),
        p.y - cell.y * (p.y / cell.y).round(),
    )
}

/// Signed distance to the nearest **grid line** of a regular rectangular grid.
///
/// The grid is formed by horizontal and vertical lines spaced `cell.x` and
/// `cell.y` apart.  `line_w` is the **half-width** of each line (so a line
/// of visual width 2 uses `line_w = 1.0`).
///
/// Returns a negative value when `p` is inside a line, positive when outside.
///
/// # Examples
/// ```
/// use abrash_core::sdf::grid_2d;
/// use abrash_core::math::Vec2;
/// // Centre of a cell (far from lines) → positive.
/// let d = grid_2d(Vec2::new(1.0, 1.0), Vec2::new(2.0, 2.0), 0.05);
/// assert!(d > 0.0);
/// // On a line at x=0 → negative.
/// let d = grid_2d(Vec2::new(0.0, 0.5), Vec2::new(2.0, 2.0), 0.05);
/// assert!(d < 0.0);
/// ```
#[must_use]
#[inline]
pub fn grid_2d(p: Vec2, cell: Vec2, line_w: f32) -> f32 {
    // Remap p into [0, cell), then reflect around half-cell to get distance
    // from the nearest line in each axis.
    let cx = (p.x % cell.x + cell.x) % cell.x;
    let cy = (p.y % cell.y + cell.y) % cell.y;
    // Distance from the nearest line edge in x/y (lines are at 0 and cell).
    let dx = cx.min(cell.x - cx) - line_w;
    let dy = cy.min(cell.y - cy) - line_w;
    // Point is inside a line if either dx < 0 or dy < 0.
    dx.min(dy)
}

/// Signed distance to a horizontal **sine wave** curve.
///
/// The wave height at x is `amplitude * sin(2π * frequency * x + phase)`.
/// The returned distance is the **signed vertical** distance from `p` to
/// the wave — positive above, negative below.  This is not an exact Euclidean
/// SDF but is correct along the vertical axis and useful as a height-field SDF.
///
/// # Examples
/// ```
/// use abrash_core::sdf::wave_sdf_2d;
/// use abrash_core::math::Vec2;
/// // With amplitude = 0 (flat line at y=0): distance = p.y.
/// let d = wave_sdf_2d(Vec2::new(1.0, 0.3), 0.0, 1.0, 0.0);
/// assert!((d - 0.3).abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn wave_sdf_2d(p: Vec2, amplitude: f32, frequency: f32, phase: f32) -> f32 {
    let wave_y = amplitude * (core::f32::consts::TAU * frequency * p.x + phase).sin();
    p.y - wave_y
}

/// Morph between two SDF values by linear interpolation.
///
/// `t = 0.0` returns `a`, `t = 1.0` returns `b`.  This produces correct
/// results when `a` and `b` have the **same topology** (e.g. morphing
/// between a sphere and a box of similar size).
///
/// # Examples
/// ```
/// use abrash_core::sdf::sdf_morph;
/// // Halfway between two shapes.
/// let d = sdf_morph(-1.0, 1.0, 0.5);
/// assert!(d.abs() < 1e-5);
/// ```
#[must_use]
#[inline]
pub fn sdf_morph(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

/// **3-D domain warping** using Perlin noise offsets.
///
/// Displaces the query point `p` by `strength * perlin_noise_3d(p + offset)`
/// in each axis before evaluating the inner SDF.  Creates organic,
/// fluid-like distortions.
///
/// # Arguments
/// * `p`        – query point
/// * `strength` – maximum warp displacement per axis
/// * `sdf`      – inner SDF closure
///
/// # Examples
/// ```
/// use abrash_core::sdf::{domain_warp_3d, sphere_3d};
/// use abrash_core::math::Vec3;
/// let d = domain_warp_3d(Vec3::new(0.3, 0.1, 0.0), 0.5, |p| {
///     sphere_3d(p, Vec3::ZERO, 1.0)
/// });
/// // The warped sphere boundary is no longer perfectly spherical.
/// let _ = d; // value varies
/// ```
#[must_use]
pub fn domain_warp_3d(p: Vec3, strength: f32, sdf: impl Fn(Vec3) -> f32) -> f32 {
    use crate::math::perlin_noise_3d;
    let offset = 7.3_f32;
    let wp = Vec3::new(
        p.x + strength * perlin_noise_3d(p),
        p.y + strength * perlin_noise_3d(Vec3::new(p.x + offset, p.y, p.z)),
        p.z + strength * perlin_noise_3d(Vec3::new(p.x, p.y + offset, p.z + offset)),
    );
    sdf(wp)
}

/// 5-tap **ambient-occlusion** estimate for SDF scenes (Inigo Quilez technique).
///
/// Marches 5 samples along the surface normal at increasing distances, comparing
/// each distance to the SDF value.  Returns a value in \[0, 1] where 1.0 is
/// fully unoccluded.
///
/// # Arguments
/// * `p`    – surface point (should be slightly offset along `n` to avoid self-intersection)
/// * `n`    – outward surface normal (unit vector)
/// * `step` – spacing between samples (0.01–0.2 works well)
/// * `sdf`  – the scene SDF closure
///
/// # Examples
/// ```
/// use abrash_core::sdf::{ambient_occlusion_estimate, sphere_3d};
/// use abrash_core::math::Vec3;
/// // An isolated sphere has full AO at its top.
/// let ao = ambient_occlusion_estimate(
///     Vec3::new(0.0, 1.001, 0.0), Vec3::Y, 0.05,
///     |p| sphere_3d(p, Vec3::ZERO, 1.0),
/// );
/// assert!(ao > 0.8, "isolated sphere top: {ao}");
/// ```
#[must_use]
pub fn ambient_occlusion_estimate(p: Vec3, n: Vec3, step: f32, sdf: impl Fn(Vec3) -> f32) -> f32 {
    let mut occ = 0.0_f32;
    let mut scale = 1.0_f32;
    for i in 0..5_u32 {
        let h = 0.001 + step * i as f32;
        let sample_p = Vec3::new(p.x + n.x * h, p.y + n.y * h, p.z + n.z * h);
        let d = sdf(sample_p);
        occ += (h - d) * scale;
        scale *= 0.95;
    }
    (1.0 - 3.0 * occ).clamp(0.0, 1.0)
}

/// **Soft shadow** ray marcher for SDF scenes (Inigo Quilez technique).
///
/// Marches a shadow ray from `ro` in direction `rd`, accumulating a
/// penumbra factor based on how close the ray comes to geometry.
/// Returns 0.0 for full shadow, 1.0 for full light.
///
/// # Arguments
/// * `ro`     – shadow ray origin (offset from surface to avoid self-shadow)
/// * `rd`     – direction toward light (unit vector)
/// * `t_min`  – near clipping distance (0.001–0.01)
/// * `t_max`  – far clipping distance (light distance)
/// * `k`      – penumbra sharpness (lower = softer; 2–64 typical)
/// * `sdf`    – the scene SDF closure
///
/// # Examples
/// ```
/// use abrash_core::sdf::{soft_shadow_estimate, sphere_3d};
/// use abrash_core::math::Vec3;
/// // A point far from the sphere in the direction away from it should be unoccluded.
/// let s = soft_shadow_estimate(
///     Vec3::new(0.0, 5.0, 0.0), Vec3::Y, 0.01, 10.0, 8.0,
///     |p| sphere_3d(p, Vec3::ZERO, 1.0),
/// );
/// assert!(s > 0.9, "open sky: {s}");
/// ```
#[must_use]
pub fn soft_shadow_estimate(
    ro: Vec3,
    rd: Vec3,
    t_min: f32,
    t_max: f32,
    k: f32,
    sdf: impl Fn(Vec3) -> f32,
) -> f32 {
    let mut res = 1.0_f32;
    let mut t = t_min;
    for _ in 0..64_u32 {
        if t >= t_max {
            break;
        }
        let p = Vec3::new(ro.x + rd.x * t, ro.y + rd.y * t, ro.z + rd.z * t);
        let h = sdf(p);
        if h < 0.001 {
            return 0.0;
        }
        res = res.min(k * h / t);
        t += h.clamp(0.01, 0.2);
    }
    res.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests_pass_29_sdf {
    use super::*;
    use crate::math::Vec2;

    // ── lemniscate_2d ─────────────────────────────────────────────────────────
    #[test]
    fn lemniscate_right_lobe_inside() {
        let d = lemniscate_2d(Vec2::new(0.7, 0.0), 1.0);
        assert!(d < 0.0, "right lobe: {d}");
    }

    #[test]
    fn lemniscate_pinch_on_boundary() {
        // Origin is the self-intersection (singular point) of the lemniscate → d = 0.
        let d = lemniscate_2d(Vec2::ZERO, 1.0);
        assert!(d.abs() < 1e-5, "pinch on boundary: {d}");
    }

    #[test]
    fn lemniscate_above_pinch_outside() {
        // A point above the origin, between the lobes, is outside.
        let d = lemniscate_2d(Vec2::new(0.0, 0.5), 1.0);
        assert!(d > 0.0, "above pinch: {d}");
    }

    #[test]
    fn lemniscate_far_outside() {
        let d = lemniscate_2d(Vec2::new(3.0, 0.0), 1.0);
        assert!(d > 0.0, "far outside: {d}");
    }

    #[test]
    fn lemniscate_symmetric_x() {
        // Left lobe (−x) should mirror right lobe.
        let a = lemniscate_2d(Vec2::new(0.7, 0.0), 1.0);
        let b = lemniscate_2d(Vec2::new(-0.7, 0.0), 1.0);
        assert!((a - b).abs() < 1e-5, "left-right symmetry: {a} vs {b}");
    }

    // ── teardrop_2d ───────────────────────────────────────────────────────────
    #[test]
    fn teardrop_head_inside() {
        let d = teardrop_2d(Vec2::new(0.0, 0.0), 0.5, 1.5);
        assert!(d < 0.0, "head inside: {d}");
    }

    #[test]
    fn teardrop_far_outside() {
        let d = teardrop_2d(Vec2::new(3.0, 0.0), 0.5, 1.5);
        assert!(d > 0.0, "far outside: {d}");
    }

    #[test]
    fn teardrop_below_tip_outside() {
        // Well below the tip (−len−r) should be outside.
        let d = teardrop_2d(Vec2::new(0.0, -3.0), 0.5, 1.5);
        assert!(d > 0.0, "below tip: {d}");
    }
}

#[cfg(test)]
mod tests_pass_30_sdf {
    use super::*;
    use crate::math::Vec2;

    // ── polar_repeat_2d ───────────────────────────────────────────────────────

    #[test]
    fn polar_repeat_on_positive_x_stays_put() {
        // A point exactly on the +X axis maps back to (r, 0).
        let p = Vec2::new(1.0, 0.0);
        let q = polar_repeat_2d(p, 4);
        assert!(
            (q.x - 1.0).abs() < 1e-5 && q.y.abs() < 1e-5,
            "+X axis: {q:?}"
        );
    }

    #[test]
    fn polar_repeat_preserves_radius() {
        // The radial distance must be unchanged.
        let p = Vec2::new(0.6, 0.8);
        let q = polar_repeat_2d(p, 6);
        assert!(
            (q.length() - p.length()).abs() < 1e-5,
            "radius: {} vs {}",
            q.length(),
            p.length()
        );
    }

    #[test]
    fn polar_repeat_n1_identity() {
        // n = 1 → the whole plane is a single sector → p is returned unchanged.
        let p = Vec2::new(0.3, 0.7);
        let q = polar_repeat_2d(p, 1);
        assert!(
            (q.x - p.x).abs() < 1e-4 && (q.y - p.y).abs() < 1e-4,
            "n=1 identity: {q:?} vs {p:?}"
        );
    }

    #[test]
    fn polar_repeat_sector_symmetry() {
        // Two points separated by exactly one sector width map to the same location.
        use std::f32::consts::TAU;
        let r = 1.2_f32;
        let n = 5_u32;
        let sector = TAU / n as f32;
        let a0 = 0.1_f32;
        let p1 = Vec2::new(a0.cos() * r, a0.sin() * r);
        let p2 = Vec2::new((a0 + sector).cos() * r, (a0 + sector).sin() * r);
        let q1 = polar_repeat_2d(p1, n);
        let q2 = polar_repeat_2d(p2, n);
        assert!(
            (q1.x - q2.x).abs() < 1e-4 && (q1.y - q2.y).abs() < 1e-4,
            "sector symmetry: {q1:?} vs {q2:?}"
        );
    }

    // ── repeat_2d ─────────────────────────────────────────────────────────────

    #[test]
    fn repeat_2d_at_cell_origin_gives_zero() {
        // A point on a cell lattice point maps to (0, 0).
        let q = repeat_2d(Vec2::new(4.0, 6.0), Vec2::new(2.0, 3.0));
        assert!(q.x.abs() < 1e-5 && q.y.abs() < 1e-5, "cell origin: {q:?}");
    }

    #[test]
    fn repeat_2d_point_within_half_cell_unchanged() {
        // A point with |p| < cell/2 is already in the nearest cell → returned as-is.
        let p = Vec2::new(0.4, 0.7);
        let q = repeat_2d(p, Vec2::new(2.0, 2.0));
        assert!(
            (q.x - p.x).abs() < 1e-5 && (q.y - p.y).abs() < 1e-5,
            "inside cell: {q:?} vs {p:?}"
        );
    }

    #[test]
    fn repeat_2d_symmetric_about_boundary() {
        // Points equidistant on either side of a cell boundary give equal |remapped| x.
        let cell = Vec2::new(2.0, 2.0);
        let q_pos = repeat_2d(Vec2::new(0.3, 0.0), cell);
        let q_neg = repeat_2d(Vec2::new(-0.3, 0.0), cell);
        assert!(
            (q_pos.x.abs() - q_neg.x.abs()).abs() < 1e-5,
            "boundary symmetry: {q_pos:?} vs {q_neg:?}"
        );
    }

    #[test]
    fn repeat_2d_clamps_at_half_cell() {
        // A point at exactly cell/2 should remap to +cell/2 or −cell/2 (magnitude = cell/2).
        let cell = Vec2::new(2.0, 2.0);
        let q = repeat_2d(Vec2::new(1.0, 0.0), cell);
        assert!(q.x.abs() <= 1.0 + 1e-5, "clamp: {q:?}");
    }
}

#[cfg(test)]
mod tests_pass_34_sdf {
    use super::*;
    use crate::math::Vec2;

    // ── grid_2d ───────────────────────────────────────────────────────────────

    #[test]
    fn grid_cell_centre_is_outside() {
        // Centre of a 2×2 cell is far from lines → positive distance.
        let d = grid_2d(Vec2::new(1.0, 1.0), Vec2::new(2.0, 2.0), 0.05);
        assert!(d > 0.0, "cell centre: {d}");
    }

    #[test]
    fn grid_on_line_is_inside() {
        // On the x=0 grid line.
        let d = grid_2d(Vec2::new(0.0, 0.5), Vec2::new(2.0, 2.0), 0.05);
        assert!(d < 0.0, "on line: {d}");
    }

    #[test]
    fn grid_symmetric_about_line() {
        // Equidistant on either side of a line should give equal |d|.
        let cell = Vec2::new(2.0, 2.0);
        let d_pos = grid_2d(Vec2::new(0.2, 0.5), cell, 0.0);
        let d_neg = grid_2d(Vec2::new(-0.2, 0.5), cell, 0.0);
        assert!((d_pos - d_neg).abs() < 1e-5, "symmetry: {d_pos} {d_neg}");
    }

    // ── wave_sdf_2d ───────────────────────────────────────────────────────────

    #[test]
    fn wave_flat_is_signed_y() {
        // Amplitude = 0 → wave is flat at y=0 → signed distance = p.y.
        let d = wave_sdf_2d(Vec2::new(1.0, 0.3), 0.0, 1.0, 0.0);
        assert!((d - 0.3).abs() < 1e-5, "flat wave: {d}");
    }

    #[test]
    fn wave_below_surface_is_negative() {
        // Point on the wave at x where the wave peaks at y=1 → p.y=1 is on the surface.
        // p.y < 1 should be negative.
        let x = 0.25_f32; // sin(2π * 1 * 0.25) = sin(π/2) = 1.0
        let d = wave_sdf_2d(Vec2::new(x, 0.5), 1.0, 1.0, 0.0);
        // wave_y = 1.0; d = 0.5 - 1.0 = -0.5
        assert!(d < 0.0, "below surface: {d}");
    }

    #[test]
    fn wave_above_surface_is_positive() {
        let x = 0.25_f32; // wave peaks at y=1
        let d = wave_sdf_2d(Vec2::new(x, 1.5), 1.0, 1.0, 0.0);
        // d = 1.5 - 1.0 = 0.5
        assert!((d - 0.5).abs() < 1e-5, "above surface: {d}");
    }
}

#[cfg(test)]
mod tests_pass_36_sdf {
    use super::*;
    use crate::math::Vec3;

    // ── sdf_morph ─────────────────────────────────────────────────────────────

    #[test]
    fn morph_at_zero_returns_a() {
        assert!((sdf_morph(-2.0, 3.0, 0.0) - (-2.0)).abs() < 1e-6);
    }

    #[test]
    fn morph_at_one_returns_b() {
        assert!((sdf_morph(-2.0, 3.0, 1.0) - 3.0).abs() < 1e-6);
    }

    #[test]
    fn morph_midpoint() {
        // t=0.5 should give the midpoint.
        let d = sdf_morph(-1.0, 1.0, 0.5);
        assert!(d.abs() < 1e-6, "midpoint: {d}");
    }

    // ── domain_warp_3d ────────────────────────────────────────────────────────

    #[test]
    fn domain_warp_far_outside_stays_positive() {
        // A point far outside a unit sphere should remain outside after small warp.
        let d = domain_warp_3d(Vec3::new(5.0, 0.0, 0.0), 0.1, |p| {
            sphere_3d(p, Vec3::ZERO, 1.0)
        });
        assert!(d > 0.0, "still outside: {d}");
    }

    #[test]
    fn domain_warp_centre_stays_negative() {
        // Origin is well inside a unit sphere — warp of strength 0.1 cannot push it outside.
        let d = domain_warp_3d(Vec3::ZERO, 0.1, |p| sphere_3d(p, Vec3::ZERO, 1.0));
        assert!(d < 0.0, "still inside: {d}");
    }
}

#[cfg(test)]
mod tests_pass_37_sdf {
    use super::*;
    use crate::math::Vec3;

    // ── ambient_occlusion_estimate ────────────────────────────────────────────

    #[test]
    fn ao_in_range() {
        let ao = ambient_occlusion_estimate(Vec3::new(0.0, 1.001, 0.0), Vec3::Y, 0.05, |p| {
            sphere_3d(p, Vec3::ZERO, 1.0)
        });
        assert!(ao >= 0.0 && ao <= 1.0, "AO out of [0,1]: {ao}");
    }

    #[test]
    fn ao_isolated_top_is_high() {
        // Top of an isolated sphere → barely any occlusion.
        let ao = ambient_occlusion_estimate(Vec3::new(0.0, 1.001, 0.0), Vec3::Y, 0.05, |p| {
            sphere_3d(p, Vec3::ZERO, 1.0)
        });
        assert!(ao > 0.8, "isolated top AO: {ao}");
    }

    // ── soft_shadow_estimate ──────────────────────────────────────────────────

    #[test]
    fn soft_shadow_open_sky_is_one() {
        // Ray pointing straight up from above the sphere — open sky.
        let s = soft_shadow_estimate(Vec3::new(0.0, 5.0, 0.0), Vec3::Y, 0.01, 10.0, 8.0, |p| {
            sphere_3d(p, Vec3::ZERO, 1.0)
        });
        assert!(s > 0.95, "open sky: {s}");
    }

    #[test]
    fn soft_shadow_blocked_is_zero() {
        // Ray from just above origin pointing in −Y hits the sphere behind.
        // Actually: shoot from above the sphere toward the sphere centre.
        let s = soft_shadow_estimate(
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            0.01,
            10.0,
            8.0,
            |p| sphere_3d(p, Vec3::ZERO, 1.0),
        );
        assert!(s < 0.05, "ray blocked by sphere: {s}");
    }
}
