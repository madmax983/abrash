#![allow(dead_code)]
#![allow(clippy::suboptimal_flops)]
#![allow(clippy::should_panic_without_expect)]
#![allow(clippy::ignore_without_reason)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::float_cmp)]
//! Plane and view-frustum primitives.
//!
//! `Plane` stores the implicit form `ax + by + cz + d = 0` with a unit normal `(a, b, c)`.
//! `Frustum` holds the 6 planes of a view frustum extracted from a view-projection matrix
//! and exposes fast sphere, AABB, and point containment tests.

use crate::geometry::AABB;
use crate::math::{Mat4, Vec3};

/// A half-space defined by a plane equation `normal · p + d = 0`.
///
/// The `normal` field is always unit length after construction via the public constructors.
/// Points where `distance > 0` are on the same side as the normal ("front").
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec3;
/// use abrash_core::plane::Plane;
///
/// // XY plane (z = 0), normal pointing +Z
/// let p = Plane::from_point_normal(Vec3::ZERO, Vec3::Z);
/// assert!(p.distance(Vec3::new(0.0, 0.0, 1.0)) > 0.0);
/// assert!(p.distance(Vec3::new(0.0, 0.0, -1.0)) < 0.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane {
    /// Unit normal of the plane.
    pub normal: Vec3,
    /// Negative distance from origin along the normal: `d = -normal · point_on_plane`.
    pub d: f32,
}

impl Plane {
    /// Construct from a point on the plane and a normal (will be normalized).
    #[must_use]
    pub fn from_point_normal(point: Vec3, normal: Vec3) -> Self {
        let n = normal.normalize();
        Self {
            normal: n,
            d: -n.dot(point),
        }
    }

    /// Construct from three counter-clockwise points (right-hand rule for normal).
    #[must_use]
    pub fn from_3_points(a: Vec3, b: Vec3, c: Vec3) -> Self {
        let ab = b - a;
        let ac = c - a;
        let normal = ab.cross(ac).normalize();
        Self {
            normal,
            d: -normal.dot(a),
        }
    }

    /// Signed distance from `point` to this plane.
    ///
    /// Positive means the point is in front (normal side), negative means behind.
    #[must_use]
    #[inline]
    pub fn distance(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.d
    }

    /// Returns `true` if `point` is on or in front of the plane.
    #[must_use]
    #[inline]
    pub fn is_in_front(&self, point: Vec3) -> bool {
        self.distance(point) >= 0.0
    }
}

/// The six planes of a view frustum.
///
/// Extracted from a combined view-projection matrix using Gribb & Hartmann's method.
/// All plane normals point **inward** (into the frustum), so a point is inside when
/// it satisfies all six `plane.distance(p) >= -radius` tests.
///
/// # Examples
///
/// ```
/// use abrash_core::math::{Mat4, Vec3};
/// use abrash_core::plane::Frustum;
/// use std::f32::consts::PI;
///
/// let view = Mat4::look_at(
///     Vec3::new(0.0, 0.0, 5.0),
///     Vec3::ZERO,
///     Vec3::UP,
/// );
/// let proj = Mat4::perspective(PI / 3.0, 1.0, 0.1, 100.0);
/// let frustum = Frustum::from_mat4(view * proj);
///
/// // Origin is in the frustum
/// assert!(frustum.contains_point(Vec3::ZERO));
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Frustum {
    /// Left, Right, Bottom, Top, Near, Far planes (all normals point inward).
    pub planes: [Plane; 6],
}

impl Frustum {
    /// Extract frustum planes from a combined view-projection matrix.
    ///
    /// Uses the Gribb-Hartmann method (add/subtract rows of the VP matrix).
    /// Planes are normalized so distances are in world-space units.
    #[must_use]
    pub fn from_mat4(vp: Mat4) -> Self {
        let m = &vp.m;

        // Row-vector convention: clip = v * VP.
        // Plane equations from Gribb & Hartmann (column-major equivalent adapted for row-major):
        // Left:   row3 + row0
        // Right:  row3 - row0
        // Bottom: row3 + row1
        // Top:    row3 - row1
        // Near:   row3 + row2
        // Far:    row3 - row2
        let make_plane = |a: f32, b: f32, c: f32, d: f32| {
            let len = (a * a + b * b + c * c).sqrt();
            if len > 1e-8 {
                let inv = 1.0 / len;
                Plane {
                    normal: Vec3::new(a * inv, b * inv, c * inv),
                    d: d * inv,
                }
            } else {
                Plane {
                    normal: Vec3::ZERO,
                    d: 0.0,
                }
            }
        };

        let planes = [
            // Left
            make_plane(
                m[0][3] + m[0][0],
                m[1][3] + m[1][0],
                m[2][3] + m[2][0],
                m[3][3] + m[3][0],
            ),
            // Right
            make_plane(
                m[0][3] - m[0][0],
                m[1][3] - m[1][0],
                m[2][3] - m[2][0],
                m[3][3] - m[3][0],
            ),
            // Bottom
            make_plane(
                m[0][3] + m[0][1],
                m[1][3] + m[1][1],
                m[2][3] + m[2][1],
                m[3][3] + m[3][1],
            ),
            // Top
            make_plane(
                m[0][3] - m[0][1],
                m[1][3] - m[1][1],
                m[2][3] - m[2][1],
                m[3][3] - m[3][1],
            ),
            // Near
            make_plane(
                m[0][3] + m[0][2],
                m[1][3] + m[1][2],
                m[2][3] + m[2][2],
                m[3][3] + m[3][2],
            ),
            // Far
            make_plane(
                m[0][3] - m[0][2],
                m[1][3] - m[1][2],
                m[2][3] - m[2][2],
                m[3][3] - m[3][2],
            ),
        ];

        Self { planes }
    }

    /// Returns `true` if `point` is inside (or on the boundary of) the frustum.
    #[must_use]
    #[inline]
    pub fn contains_point(&self, point: Vec3) -> bool {
        self.planes.iter().all(|p| p.distance(point) >= 0.0)
    }

    /// Returns `true` if the sphere is at least partially inside the frustum.
    ///
    /// A sphere is fully outside when its center is further than `radius` behind any plane.
    #[must_use]
    #[inline]
    pub fn contains_sphere(&self, center: Vec3, radius: f32) -> bool {
        self.planes.iter().all(|p| p.distance(center) >= -radius)
    }

    /// Returns `true` if the AABB is at least partially inside the frustum.
    ///
    /// Uses the "positive vertex" test: for each plane, find the AABB corner most aligned
    /// with the inward normal and check it's not fully outside.
    #[must_use]
    pub fn contains_aabb(&self, aabb: &AABB) -> bool {
        for plane in &self.planes {
            // Positive vertex: pick the corner most in the direction of the inward normal.
            let px = if plane.normal.x >= 0.0 {
                aabb.max.x
            } else {
                aabb.min.x
            };
            let py = if plane.normal.y >= 0.0 {
                aabb.max.y
            } else {
                aabb.min.y
            };
            let pz = if plane.normal.z >= 0.0 {
                aabb.max.z
            } else {
                aabb.min.z
            };
            if plane.distance(Vec3::new(px, py, pz)) < 0.0 {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec3;
    use std::f32::consts::PI;

    fn make_frustum() -> Frustum {
        let view = Mat4::look_at(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::UP);
        let proj = Mat4::perspective(PI / 2.0, 1.0, 1.0, 50.0);
        Frustum::from_mat4(view * proj)
    }

    // fast_inv_sqrt approximation introduces ~0.5% error in normalize.
    const TOL: f32 = 0.01;

    #[test]
    fn plane_distance_correct() {
        let p = Plane::from_point_normal(Vec3::ZERO, Vec3::Y);
        assert!((p.distance(Vec3::new(0.0, 1.0, 0.0)) - 1.0).abs() < TOL);
        assert!((p.distance(Vec3::new(0.0, -1.0, 0.0)) + 1.0).abs() < TOL);
    }

    #[test]
    fn plane_from_3_points() {
        let p = Plane::from_3_points(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        // Normal should be +Z
        assert!((p.normal.z - 1.0).abs() < TOL, "normal.z={}", p.normal.z);
    }

    #[test]
    fn frustum_contains_origin() {
        let f = make_frustum();
        assert!(f.contains_point(Vec3::ZERO));
    }

    #[test]
    fn frustum_rejects_behind_camera() {
        let f = make_frustum();
        // Camera at (0,0,5) looking toward origin, so (0,0,10) is behind
        assert!(!f.contains_point(Vec3::new(0.0, 0.0, 10.0)));
    }

    #[test]
    fn frustum_sphere_partially_inside() {
        let f = make_frustum();
        // Large sphere at origin is inside
        assert!(f.contains_sphere(Vec3::ZERO, 0.5));
    }

    #[test]
    fn frustum_sphere_fully_outside() {
        let f = make_frustum();
        // Very far behind camera
        assert!(!f.contains_sphere(Vec3::new(0.0, 0.0, 100.0), 1.0));
    }

    #[test]
    fn frustum_aabb_inside() {
        let f = make_frustum();
        let aabb = AABB::new(Vec3::new(-0.5, -0.5, -0.5), Vec3::new(0.5, 0.5, 0.5));
        assert!(f.contains_aabb(&aabb));
    }

    #[test]
    fn frustum_aabb_outside() {
        let f = make_frustum();
        let aabb = AABB::new(Vec3::new(0.0, 0.0, 10.0), Vec3::new(1.0, 1.0, 11.0));
        assert!(!f.contains_aabb(&aabb));
    }
}
