//! Constructive Solid Geometry (CSG) Module
//!
//! Allows combining SDF primitives using Union, Difference, Intersection, and Smooth Union.
//! Can generate meshes from the resulting CSG tree.

use crate::experimental::sdf::SdfPrimitive;
use crate::experimental::isosurface::extract_isosurface;
use crate::mesh::Mesh;
use crate::math::Vec3;

#[derive(Clone, Debug)]
pub enum CsgOp {
    Union,
    Difference,
    Intersection,
    SmoothUnion(f32),
}

#[derive(Clone, Debug)]
pub enum CsgNode {
    Leaf(SdfPrimitive),
    Node(CsgOp, Box<CsgNode>, Box<CsgNode>),
    Translate(Vec3, Box<CsgNode>),
}

impl CsgNode {
    /// Evaluate the signed distance at point p.
    pub fn eval(&self, p: Vec3) -> f32 {
        match self {
            Self::Leaf(prim) => prim.distance(p),
            Self::Translate(offset, child) => child.eval(p - *offset),
            Self::Node(op, left, right) => {
                let d1 = left.eval(p);
                let d2 = right.eval(p);
                match op {
                    CsgOp::Union => d1.min(d2),
                    CsgOp::Intersection => d1.max(d2),
                    CsgOp::Difference => d1.max(-d2),
                    CsgOp::SmoothUnion(k) => {
                         let h = (0.5 + 0.5 * (d2 - d1) / k).clamp(0.0, 1.0);
                         mix(d2, d1, h) - k * h * (1.0 - h)
                    }
                }
            }
        }
    }

    /// Generates a mesh from the CSG tree using Marching Tetrahedra.
    pub fn to_mesh(&self, min: Vec3, max: Vec3, resolution: usize) -> Mesh {
        extract_isosurface(|p| self.eval(p), min, max, resolution)
    }
}

fn mix(x: f32, y: f32, a: f32) -> f32 {
    x * (1.0 - a) + y * a
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experimental::sdf::SdfPrimitive;
    use crate::math::Vec3;

    #[test]
    fn test_csg_union() {
        // Union of two spheres
        let s1 = CsgNode::Leaf(SdfPrimitive::Sphere {
            radius: 1.0,
            center: Vec3::new(-0.5, 0.0, 0.0),
        });
        let s2 = CsgNode::Leaf(SdfPrimitive::Sphere {
            radius: 1.0,
            center: Vec3::new(0.5, 0.0, 0.0),
        });
        let union = CsgNode::Node(CsgOp::Union, Box::new(s1), Box::new(s2));

        // Center (0,0,0): Dist to s1 center (-0.5) is 0.5. Radius 1.0. Dist = -0.5.
        // Dist to s2 center (0.5) is 0.5. Radius 1.0. Dist = -0.5.
        // Union = min(-0.5, -0.5) = -0.5.
        assert!((union.eval(Vec3::new(0.0, 0.0, 0.0)) - (-0.5)).abs() < 0.001);
    }

    #[test]
    fn test_csg_difference() {
        // Sphere minus smaller sphere
        let s1 = CsgNode::Leaf(SdfPrimitive::Sphere {
            radius: 1.0,
            center: Vec3::new(0.0, 0.0, 0.0),
        });
        let s2 = CsgNode::Leaf(SdfPrimitive::Sphere {
            radius: 0.5,
            center: Vec3::new(0.0, 0.0, 0.0),
        });
        let diff = CsgNode::Node(CsgOp::Difference, Box::new(s1), Box::new(s2));

        // Point at center (0,0,0).
        // d1 = -1.0.
        // d2 = -0.5.
        // Diff = max(d1, -d2) = max(-1.0, 0.5) = 0.5.
        // Should be positive (outside) because we subtracted the inner sphere.
        assert!((diff.eval(Vec3::new(0.0, 0.0, 0.0)) - 0.5).abs() < 0.001);

        // Point at 0.8.
        // d1 = 0.8 - 1.0 = -0.2.
        // d2 = 0.8 - 0.5 = 0.3.
        // Diff = max(-0.2, -0.3) = -0.2.
        // Should be negative (inside).
        assert!((diff.eval(Vec3::new(0.8, 0.0, 0.0)) - (-0.2)).abs() < 0.001);
    }

    #[test]
    fn test_smooth_union() {
        // Two spheres slightly apart
        let s1 = CsgNode::Leaf(SdfPrimitive::Sphere {
            radius: 1.0,
            center: Vec3::new(-1.2, 0.0, 0.0),
        });
        let s2 = CsgNode::Leaf(SdfPrimitive::Sphere {
            radius: 1.0,
            center: Vec3::new(1.2, 0.0, 0.0),
        });

        // Without smoothing, at 0, dist is 0.2
        let union = CsgNode::Node(CsgOp::Union, Box::new(s1.clone()), Box::new(s2.clone()));
        let dist_sharp = union.eval(Vec3::new(0.0, 0.0, 0.0));
        assert!((dist_sharp - 0.2).abs() < 0.001);

        // With smoothing
        let smooth = CsgNode::Node(CsgOp::SmoothUnion(0.5), Box::new(s1), Box::new(s2));
        let dist_smooth = smooth.eval(Vec3::new(0.0, 0.0, 0.0));

        // Should be less than 0.2 (blending makes it bulge)
        assert!(dist_smooth < 0.2);
    }

    #[test]
    fn test_to_mesh() {
        let s = CsgNode::Leaf(SdfPrimitive::Sphere {
            radius: 1.0,
            center: Vec3::new(0.0, 0.0, 0.0),
        });

        let mesh = s.to_mesh(
            Vec3::new(-1.5, -1.5, -1.5),
            Vec3::new(1.5, 1.5, 1.5),
            10
        );

        assert!(mesh.vertices.len() > 0);
        assert!(mesh.indices.len() > 0);
    }
}
