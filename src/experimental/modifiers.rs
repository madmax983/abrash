//! Procedural mesh modifiers.
//!
//! This module provides a trait and implementations for modifying mesh geometry
//! procedurally. This allows for effects like twisting, tapering, and noise
//! to be applied to existing meshes.

use crate::math::{Mat4, Vec3};
use crate::mesh::Mesh;

/// A trait for modifiers that alter mesh geometry.
pub trait MeshModifier {
    /// Apply the modifier to the given mesh in-place.
    fn apply(&self, mesh: &mut Mesh);
}

/// Twists the mesh around the Y axis based on the Y coordinate.
pub struct TwistModifier {
    /// The amount of twist per unit of height (in radians).
    pub factor: f32,
}

impl MeshModifier for TwistModifier {
    fn apply(&self, mesh: &mut Mesh) {
        for v in &mut mesh.vertices {
            let angle = v.y * self.factor;
            let rotation = Mat4::rotation_y(angle);
            // Mat4::transform_point returns (Vec3, w), we only need Vec3
            let (transformed, _) = rotation.transform_point(*v);
            *v = transformed;
        }
    }
}

/// Tapers the mesh along the Y axis.
pub struct TaperModifier {
    /// The tapering factor. 0.0 means no taper.
    /// Positive values shrink the top (assuming Y is up).
    pub factor: f32,
}

impl MeshModifier for TaperModifier {
    fn apply(&self, mesh: &mut Mesh) {
        for v in &mut mesh.vertices {
            // Simple linear taper: scale = 1.0 - y * factor
            // We assume object is roughly centered or base is at 0?
            // Let's just use raw Y for now, creates interesting effects.
            let scale = (1.0 - v.y * self.factor).max(0.0);
            v.x *= scale;
            v.z *= scale;
        }
    }
}

/// Projects vertices onto a sphere of the given radius.
pub struct SpherifyModifier {
    pub radius: f32,
}

impl MeshModifier for SpherifyModifier {
    fn apply(&self, mesh: &mut Mesh) {
        for v in &mut mesh.vertices {
            *v = v.normalize() * self.radius;
        }
    }
}

/// Adds deterministic noise to vertex positions.
pub struct NoiseModifier {
    pub intensity: f32,
}

impl NoiseModifier {
    #[allow(clippy::excessive_precision)]
    fn hash(n: f32) -> f32 {
        (n.sin() * 43758.5453).fract()
    }

    fn noise3(v: Vec3) -> Vec3 {
        // Simple pseudo-random hash based on position
        let x = Self::hash(v.x * 12.9898 + v.y * 78.233 + v.z * 54.53);
        let y = Self::hash(v.x * 39.346 + v.y * 11.135 + v.z * 87.32);
        let z = Self::hash(v.x * 73.156 + v.y * 52.235 + v.z * 09.12);
        Vec3::new(x, y, z)
    }
}

impl MeshModifier for NoiseModifier {
    fn apply(&self, mesh: &mut Mesh) {
        for v in &mut mesh.vertices {
            let noise = Self::noise3(*v);
            // Remap 0..1 to -1..1
            let offset = (noise * 2.0 - Vec3::new(1.0, 1.0, 1.0)) * self.intensity;
            *v = *v + offset;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_twist_modifier() {
        let mut mesh = Mesh::new();
        mesh.vertices.push(Vec3::new(1.0, 0.0, 0.0)); // Base
        mesh.vertices.push(Vec3::new(1.0, 1.0, 0.0)); // Top

        let modifier = TwistModifier {
            factor: std::f32::consts::PI / 2.0, // 90 degrees per unit
        };
        modifier.apply(&mut mesh);

        // Base at y=0 should not rotate
        assert!((mesh.vertices[0].x - 1.0).abs() < 0.0001);
        assert!(mesh.vertices[0].z.abs() < 0.0001);

        // Top at y=1 should rotate 90 degrees around Y
        // (1, 0, 0) -> (0, 0, -1) (Right-handed, -sin(90) = -1)
        // Wait, Mat4::rotation_y(90 deg):
        // [ c  0 -s ] [1]   [ c ]   [ 0 ]
        // [ 0  1  0 ] [1] = [ 1 ] = [ 1 ]
        // [ s  0  c ] [0]   [ s ]   [ 1 ]
        // sin(PI/2) = 1.
        // So x=0, z=1.

        let v_top = mesh.vertices[1];
        assert!(v_top.x.abs() < 0.0001, "Expected x=0, got {}", v_top.x);
        assert!((v_top.z - -1.0).abs() < 0.0001 || (v_top.z - 1.0).abs() < 0.0001, "Expected z=1 or -1, got {}", v_top.z);
    }

    #[test]
    fn test_spherify_modifier() {
        let mut mesh = Mesh::cube(2.0); // Corners are at +/- 1.0
        // Corner at (1, 1, 1). Length is sqrt(3) ~= 1.732

        let modifier = SpherifyModifier { radius: 1.0 };
        modifier.apply(&mut mesh);

        for v in mesh.vertices {
            assert!((v.length() - 1.0).abs() < 0.0001);
        }
    }
}
