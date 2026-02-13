//! Frustum Culling primitives.

use crate::math::{Mat4, Vec3};
use crate::mesh::BoundingSphere;

/// A geometric plane defined by a normal and a distance from the origin.
/// Equation: `normal . point + distance = 0`
#[derive(Debug, Clone, Copy)]
pub struct Plane {
    pub normal: Vec3,
    pub distance: f32,
}

impl Plane {
    /// Normalize the plane equation so that the normal has length 1.
    pub fn normalize(&mut self) {
        let len = self.normal.length();
        if len > 0.0001 {
            let inv_len = 1.0 / len;
            self.normal = self.normal * inv_len;
            self.distance *= inv_len;
        }
    }

    /// Signed distance from a point to the plane.
    /// Positive if on the side of the normal.
    #[must_use]
    pub fn distance_to_point(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.distance
    }
}

/// A View Frustum defined by 6 planes.
/// Used for object-level culling.
pub struct Frustum {
    pub planes: [Plane; 6],
}

impl Frustum {
    /// Extract frustum planes from View-Projection matrix.
    /// Assumes Row-Major matrix where `v_clip = v_world * M`.
    ///
    /// The planes are extracted such that the normal points **inside** the frustum.
    #[must_use]
    pub fn from_matrix(m: Mat4) -> Self {
        // In Row-Vector convention v' = v * M,
        // x' = v . Col0
        // y' = v . Col1
        // z' = v . Col2
        // w' = v . Col3
        let c0 = m.col(0);
        let c1 = m.col(1);
        let c2 = m.col(2);
        let c3 = m.col(3);

        // Frustum planes in Clip Space: -w <= x,y,z <= w
        // Left:   x >= -w  => x + w >= 0
        // Right:  x <= w   => w - x >= 0
        // Bottom: y >= -w  => y + w >= 0
        // Top:    y <= w   => w - y >= 0
        // Near:   z >= -w  => z + w >= 0
        // Far:    z <= w   => w - z >= 0

        let mut planes = [
            // Left
            Plane {
                normal: Vec3::new(c3.x + c0.x, c3.y + c0.y, c3.z + c0.z),
                distance: c3.w + c0.w,
            },
            // Right
            Plane {
                normal: Vec3::new(c3.x - c0.x, c3.y - c0.y, c3.z - c0.z),
                distance: c3.w - c0.w,
            },
            // Bottom
            Plane {
                normal: Vec3::new(c3.x + c1.x, c3.y + c1.y, c3.z + c1.z),
                distance: c3.w + c1.w,
            },
            // Top
            Plane {
                normal: Vec3::new(c3.x - c1.x, c3.y - c1.y, c3.z - c1.z),
                distance: c3.w - c1.w,
            },
            // Near
            Plane {
                normal: Vec3::new(c3.x + c2.x, c3.y + c2.y, c3.z + c2.z),
                distance: c3.w + c2.w,
            },
            // Far
            Plane {
                normal: Vec3::new(c3.x - c2.x, c3.y - c2.y, c3.z - c2.z),
                distance: c3.w - c2.w,
            },
        ];

        for p in &mut planes {
            p.normalize();
        }

        Self { planes }
    }

    /// Check if a sphere intersects or is inside the frustum.
    /// Returns `true` if the sphere is visible (partially or fully).
    /// Returns `false` if the sphere is fully outside any plane.
    #[must_use]
    pub fn intersects(&self, sphere: &BoundingSphere) -> bool {
        for plane in &self.planes {
            // Distance is positive inside, negative outside.
            // If center distance is less than -radius, it's fully outside.
            if plane.distance_to_point(sphere.center) < -sphere.radius {
                return false;
            }
        }
        true
    }
}
