#![allow(dead_code)]
#![allow(clippy::suboptimal_flops)]
#![allow(clippy::should_panic_without_expect)]
#![allow(clippy::ignore_without_reason)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::float_cmp)]
//! Ray primitive for intersection testing (picking, raycasting, BVH traversal).

use crate::geometry::{AABB, BoundingSphere};
use crate::math::Vec3;
use crate::plane::Plane;

/// A ray defined by an origin point and a unit direction.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec3;
/// use abrash_core::ray::Ray;
///
/// let ray = Ray::new(Vec3::ZERO, Vec3::Z);
/// assert_eq!(ray.at(3.0), Vec3::new(0.0, 0.0, 3.0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    /// The origin point of the ray.
    pub origin: Vec3,
    /// The unit direction of the ray.
    pub direction: Vec3,
    /// Pre-computed component-wise reciprocal of `direction` for fast AABB tests.
    /// Each component is `f32::INFINITY` when the corresponding direction component is zero.
    inv_direction: Vec3,
}

impl Ray {
    /// Creates a new ray from an origin and direction (direction will be normalized).
    #[must_use]
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        let dir = direction.normalize();
        let inv_direction = Vec3::new(
            if dir.x.abs() > 1e-8 {
                1.0 / dir.x
            } else {
                f32::INFINITY
            },
            if dir.y.abs() > 1e-8 {
                1.0 / dir.y
            } else {
                f32::INFINITY
            },
            if dir.z.abs() > 1e-8 {
                1.0 / dir.z
            } else {
                f32::INFINITY
            },
        );
        Self {
            origin,
            direction: dir,
            inv_direction,
        }
    }

    /// Returns the point along the ray at parameter `t`: `origin + direction * t`.
    ///
    /// Negative `t` goes behind the ray origin.
    #[must_use]
    #[inline]
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }

    /// Intersects the ray with an infinite plane. Returns the parameter `t` where
    /// the intersection occurs, or `None` if the ray is parallel to the plane.
    ///
    /// The intersection point is `ray.at(t)`.
    #[must_use]
    pub fn intersect_plane(&self, plane: &Plane) -> Option<f32> {
        let denom = plane.normal.dot(self.direction);
        if denom.abs() < 1e-8 {
            return None; // Ray is parallel to the plane
        }
        let t = -(plane.normal.dot(self.origin) + plane.d) / denom;
        Some(t)
    }

    /// Intersects the ray with an axis-aligned bounding box using the slab method.
    ///
    /// Returns `Some((t_enter, t_exit))` where the ray intersects the box, or `None`
    /// if there is no intersection. A negative `t_enter` means the ray origin is inside
    /// the box (or the box is behind the ray).
    ///
    /// # Performance
    ///
    /// Pre-computed `inv_direction` avoids three divisions in the hot path.
    #[must_use]
    pub fn intersect_aabb(&self, aabb: &AABB) -> Option<(f32, f32)> {
        let t1 = (aabb.min - self.origin) * self.inv_direction;
        let t2 = (aabb.max - self.origin) * self.inv_direction;

        let t_min = t1.min(t2);
        let t_max = t1.max(t2);

        let enter = t_min.x.max(t_min.y).max(t_min.z);
        let exit = t_max.x.min(t_max.y).min(t_max.z);

        if enter <= exit {
            Some((enter, exit))
        } else {
            None
        }
    }

    /// Returns `true` if the ray hits the AABB at `t >= 0` (in front of the origin).
    #[must_use]
    #[inline]
    pub fn hits_aabb(&self, aabb: &AABB) -> bool {
        self.intersect_aabb(aabb)
            .map_or(false, |(_, exit)| exit >= 0.0)
    }

    /// Intersects the ray with a sphere. Returns the smaller non-negative `t`, or `None`.
    ///
    /// Uses the analytic solution for the quadratic `|at(t) - center|² = radius²`.
    #[must_use]
    pub fn intersect_sphere(&self, sphere: &BoundingSphere) -> Option<f32> {
        let oc = self.origin - sphere.center;
        let b = oc.dot(self.direction);
        let c = oc.length_sq() - sphere.radius * sphere.radius;
        let discriminant = b * b - c;
        if discriminant < 0.0 {
            return None;
        }
        let sqrt_d = discriminant.sqrt();
        let t0 = -b - sqrt_d;
        let t1 = -b + sqrt_d;
        if t0 >= 0.0 {
            Some(t0)
        } else if t1 >= 0.0 {
            Some(t1)
        } else {
            None
        }
    }

    /// Intersects the ray with a triangle (Möller–Trumbore algorithm).
    ///
    /// Returns `Some(t)` where `t` is the distance along the ray to the hit, or `None`.
    /// Backface culling is **not** applied — both sides of the triangle are tested.
    ///
    /// # Arguments
    ///
    /// * `v0`, `v1`, `v2` — triangle vertices in counter-clockwise order.
    #[must_use]
    pub fn intersect_triangle(&self, v0: Vec3, v1: Vec3, v2: Vec3) -> Option<f32> {
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;

        let h = self.direction.cross(edge2);
        let det = edge1.dot(h);

        if det.abs() < 1e-8 {
            return None; // Ray is parallel to the triangle
        }

        let inv_det = 1.0 / det;
        let s = self.origin - v0;
        let u = inv_det * s.dot(h);
        if !(0.0..=1.0).contains(&u) {
            return None;
        }

        let q = s.cross(edge1);
        let v = inv_det * self.direction.dot(q);
        if v < 0.0 || u + v > 1.0 {
            return None;
        }

        let t = inv_det * edge2.dot(q);
        if t > 1e-8 { Some(t) } else { None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{AABB, BoundingSphere};
    use crate::math::Vec3;
    use crate::plane::Plane;

    // Tolerance for tests: fast_inv_sqrt introduces ~0.5% error when normalizing
    // unit vectors, which propagates into intersection t-values.
    const TOL: f32 = 0.02;

    #[test]
    fn ray_at() {
        let ray = Ray::new(Vec3::new(1.0, 2.0, 3.0), Vec3::new(0.0, 0.0, 1.0));
        let p = ray.at(5.0);
        assert!((p.x - 1.0).abs() < TOL);
        assert!((p.y - 2.0).abs() < TOL);
        assert!((p.z - 8.0).abs() < TOL); // 3 + 5 = 8
    }

    #[test]
    fn ray_intersect_plane_hit() {
        let ray = Ray::new(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
        let plane = Plane::from_point_normal(Vec3::ZERO, Vec3::Y);
        let t = ray.intersect_plane(&plane).expect("should hit");
        assert!((t - 5.0).abs() < TOL, "t={t}");
    }

    #[test]
    fn ray_intersect_plane_parallel() {
        let ray = Ray::new(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0));
        let plane = Plane::from_point_normal(Vec3::ZERO, Vec3::Y);
        assert!(ray.intersect_plane(&plane).is_none());
    }

    #[test]
    fn ray_intersect_aabb_hit() {
        let aabb = AABB::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
        let (t_enter, t_exit) = ray.intersect_aabb(&aabb).expect("should hit");
        assert!(t_enter < t_exit);
        assert!((t_enter - 4.0).abs() < TOL, "t_enter={t_enter}");
        assert!((t_exit - 6.0).abs() < TOL, "t_exit={t_exit}");
    }

    #[test]
    fn ray_intersect_aabb_miss() {
        let aabb = AABB::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let ray = Ray::new(Vec3::new(5.0, 5.0, 5.0), Vec3::new(1.0, 0.0, 0.0));
        assert!(ray.intersect_aabb(&aabb).is_none());
    }

    #[test]
    fn ray_intersect_sphere_hit() {
        let sphere = BoundingSphere {
            center: Vec3::new(0.0, 0.0, 0.0),
            radius: 1.0,
        };
        let ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
        let t = ray.intersect_sphere(&sphere).expect("should hit");
        assert!((t - 4.0).abs() < TOL, "t={t}");
    }

    #[test]
    fn ray_intersect_sphere_miss() {
        let sphere = BoundingSphere {
            center: Vec3::ZERO,
            radius: 1.0,
        };
        let ray = Ray::new(Vec3::new(5.0, 5.0, 5.0), Vec3::new(1.0, 0.0, 0.0));
        assert!(ray.intersect_sphere(&sphere).is_none());
    }

    #[test]
    fn ray_intersect_triangle_hit() {
        let v0 = Vec3::new(-1.0, -1.0, 0.0);
        let v1 = Vec3::new(1.0, -1.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);
        let ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
        let t = ray.intersect_triangle(v0, v1, v2).expect("should hit");
        assert!((t - 5.0).abs() < TOL, "t={t}");
    }

    #[test]
    fn ray_intersect_triangle_miss() {
        let v0 = Vec3::new(-1.0, -1.0, 0.0);
        let v1 = Vec3::new(1.0, -1.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);
        let ray = Ray::new(Vec3::new(5.0, 5.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
        assert!(ray.intersect_triangle(v0, v1, v2).is_none());
    }
}
