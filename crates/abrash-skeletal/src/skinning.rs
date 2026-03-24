//! CPU vertex skinning — transforms vertices by weighted bone influences.

use abrash_core::math::Vec3;

use crate::pose::SkinMatrices;
use crate::skin::SkinnedMesh;

/// Apply skin matrices to a skinned mesh, producing deformed positions.
///
/// For each vertex:
/// `out[v] = sum(weight[i] * (skin_matrix[joint[i]] * rest_pos))` for `i` in `0..4`
///
/// # Panics
///
/// Panics if `out_positions` length doesn't match the mesh vertex count.
pub fn skin_vertices(mesh: &SkinnedMesh, skin_matrices: &SkinMatrices, out_positions: &mut [Vec3]) {
    let vertex_count = mesh.mesh.vertices.len();
    assert_eq!(
        out_positions.len(),
        vertex_count,
        "out_positions length ({}) != vertex count ({vertex_count})",
        out_positions.len()
    );

    for v in 0..vertex_count {
        let rest_pos = mesh.mesh.vertices[v];
        let joints = mesh.skin.joint_indices[v];
        let weights = mesh.skin.weights[v];

        let mut skinned = Vec3::ZERO;
        for i in 0..4 {
            let w = weights[i];
            if w > 0.0 {
                let joint_idx = joints[i] as usize;
                let mat = &skin_matrices.matrices[joint_idx];
                let (transformed, _) = mat.transform_point(rest_pos);
                skinned = skinned + transformed * w;
            }
        }
        out_positions[v] = skinned;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pose::SkinMatrices;
    use crate::skin::{SkinData, SkinnedMesh};
    use abrash_core::math::{Mat4, Vec3};
    use abrash_core::mesh::Mesh;
    use abrash_core::quat::Quat;
    use std::f32::consts::FRAC_PI_2;

    const EPSILON: f32 = 1e-4;

    fn assert_vec3_near(actual: Vec3, expected: Vec3, label: &str) {
        assert!(
            (actual.x - expected.x).abs() < EPSILON
                && (actual.y - expected.y).abs() < EPSILON
                && (actual.z - expected.z).abs() < EPSILON,
            "{label}: expected ({}, {}, {}), got ({}, {}, {})",
            expected.x,
            expected.y,
            expected.z,
            actual.x,
            actual.y,
            actual.z
        );
    }

    fn make_skinned_mesh(vertices: Vec<Vec3>, skin: SkinData) -> SkinnedMesh {
        let mut mesh = Mesh::new();
        mesh.vertices = vertices;
        SkinnedMesh { mesh, skin }
    }

    #[test]
    fn identity_skinning_preserves_positions() {
        let verts = vec![
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(0.0, 0.0, 3.0),
        ];
        let skin = SkinData {
            joint_indices: vec![[0, 0, 0, 0]; 3],
            weights: vec![[1.0, 0.0, 0.0, 0.0]; 3],
        };
        let skinned_mesh = make_skinned_mesh(verts.clone(), skin);
        let skin_matrices = SkinMatrices {
            matrices: vec![Mat4::identity()],
        };

        let mut out = vec![Vec3::ZERO; 3];
        skin_vertices(&skinned_mesh, &skin_matrices, &mut out);

        for (i, v) in verts.iter().enumerate() {
            assert_vec3_near(out[i], *v, &format!("vertex {i}"));
        }
    }

    #[test]
    fn single_bone_rotation_90_y() {
        // One bone rotated 90 degrees around Y.
        // Vertex at (1, 0, 0) should move to (0, 0, -1) in right-handed system.
        let verts = vec![Vec3::new(1.0, 0.0, 0.0)];
        let skin = SkinData {
            joint_indices: vec![[0, 0, 0, 0]],
            weights: vec![[1.0, 0.0, 0.0, 0.0]],
        };
        let skinned_mesh = make_skinned_mesh(verts, skin);

        let rot_mat = Mat4::rotation_y(FRAC_PI_2);
        let skin_matrices = SkinMatrices {
            matrices: vec![rot_mat],
        };

        let mut out = vec![Vec3::ZERO; 1];
        skin_vertices(&skinned_mesh, &skin_matrices, &mut out);

        assert_vec3_near(out[0], Vec3::new(0.0, 0.0, -1.0), "rotated vertex");
    }

    #[test]
    fn multi_bone_weighted_blend() {
        // Vertex at (1, 0, 0), influenced 50/50 by two bones.
        // Bone 0: identity (vertex stays at (1, 0, 0)).
        // Bone 1: translation by (2, 0, 0) so vertex goes to (3, 0, 0).
        // Result should be average: (2, 0, 0).
        let verts = vec![Vec3::new(1.0, 0.0, 0.0)];
        let skin = SkinData {
            joint_indices: vec![[0, 1, 0, 0]],
            weights: vec![[0.5, 0.5, 0.0, 0.0]],
        };
        let skinned_mesh = make_skinned_mesh(verts, skin);

        let skin_matrices = SkinMatrices {
            matrices: vec![Mat4::identity(), Mat4::translation(2.0, 0.0, 0.0)],
        };

        let mut out = vec![Vec3::ZERO; 1];
        skin_vertices(&skinned_mesh, &skin_matrices, &mut out);

        // 0.5 * (1,0,0) + 0.5 * (3,0,0) = (2, 0, 0)
        assert_vec3_near(out[0], Vec3::new(2.0, 0.0, 0.0), "blended vertex");
    }

    #[test]
    fn zero_weight_bones_skipped() {
        // Vertex at (1, 0, 0). Joint 0 has weight 1.0, joints 1-3 have weight 0.0.
        // Even though joints 1-3 have a wild translation, it shouldn't affect the result.
        let verts = vec![Vec3::new(1.0, 0.0, 0.0)];
        let skin = SkinData {
            joint_indices: vec![[0, 1, 1, 1]],
            weights: vec![[1.0, 0.0, 0.0, 0.0]],
        };
        let skinned_mesh = make_skinned_mesh(verts, skin);

        let skin_matrices = SkinMatrices {
            matrices: vec![Mat4::identity(), Mat4::translation(999.0, 999.0, 999.0)],
        };

        let mut out = vec![Vec3::ZERO; 1];
        skin_vertices(&skinned_mesh, &skin_matrices, &mut out);

        assert_vec3_near(out[0], Vec3::new(1.0, 0.0, 0.0), "zero-weight vertex");
    }

    #[test]
    fn four_bone_influence() {
        // Vertex at origin, influenced equally by 4 bones that each translate differently.
        // Bone 0: T(4, 0, 0) -> (4, 0, 0)
        // Bone 1: T(0, 4, 0) -> (0, 4, 0)
        // Bone 2: T(0, 0, 4) -> (0, 0, 4)
        // Bone 3: T(0, 0, 0) -> (0, 0, 0) (identity)
        // Result: 0.25 * each = (1, 1, 1)
        let verts = vec![Vec3::ZERO];
        let skin = SkinData {
            joint_indices: vec![[0, 1, 2, 3]],
            weights: vec![[0.25, 0.25, 0.25, 0.25]],
        };
        let skinned_mesh = make_skinned_mesh(verts, skin);

        let skin_matrices = SkinMatrices {
            matrices: vec![
                Mat4::translation(4.0, 0.0, 0.0),
                Mat4::translation(0.0, 4.0, 0.0),
                Mat4::translation(0.0, 0.0, 4.0),
                Mat4::identity(),
            ],
        };

        let mut out = vec![Vec3::ZERO; 1];
        skin_vertices(&skinned_mesh, &skin_matrices, &mut out);

        assert_vec3_near(out[0], Vec3::new(1.0, 1.0, 1.0), "4-bone blend");
    }

    #[test]
    fn multiple_vertices() {
        // Test skinning multiple vertices at once.
        let verts = vec![
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ];
        let skin = SkinData {
            joint_indices: vec![[0, 0, 0, 0]; 3],
            weights: vec![[1.0, 0.0, 0.0, 0.0]; 3],
        };
        let skinned_mesh = make_skinned_mesh(verts, skin);

        // Scale by 2 on all axes.
        let skin_matrices = SkinMatrices {
            matrices: vec![Mat4::scale(2.0, 2.0, 2.0)],
        };

        let mut out = vec![Vec3::ZERO; 3];
        skin_vertices(&skinned_mesh, &skin_matrices, &mut out);

        assert_vec3_near(out[0], Vec3::new(2.0, 0.0, 0.0), "scaled v0");
        assert_vec3_near(out[1], Vec3::new(0.0, 2.0, 0.0), "scaled v1");
        assert_vec3_near(out[2], Vec3::new(0.0, 0.0, 2.0), "scaled v2");
    }

    #[test]
    fn end_to_end_skeleton_skinning() {
        // Integration test: build a skeleton, compute FK, get skin matrices, skin a vertex.
        //
        // Skeleton: root (identity bind) + child at (1,0,0).
        // Bind pose: root at origin, child at (1,0,0) in world.
        // Inverse bind for root = identity, for child = T(-1,0,0).
        //
        // Animate: rotate root 90 deg around Y.
        // Now child's world pos = (0, 0, -1).
        // Skin matrix for child = inverse_bind * global = T(-1,0,0) * (T(1,0,0) * rot_y(90))
        //
        // A vertex at (1, 0, 0) bound to the child bone:
        // First inverse_bind moves it to bone-local space, then global puts it in animated space.

        use crate::pose::Pose;
        use crate::skeleton::{Joint, JointId, Skeleton};
        use abrash_core::transform::Transform;

        let rot90y = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);

        let joints = vec![
            Joint {
                name: "root".to_string(),
                parent: None,
                inverse_bind_matrix: Mat4::identity(),
                bind_transform: Transform::identity(),
            },
            Joint {
                name: "child".to_string(),
                parent: Some(JointId(0)),
                inverse_bind_matrix: Mat4::translation(-1.0, 0.0, 0.0),
                bind_transform: Transform::from_position(Vec3::new(1.0, 0.0, 0.0)),
            },
        ];
        let skel = Skeleton::new(joints);

        // Animated pose: root rotated 90 around Y, child at local (1,0,0).
        let pose = Pose::new(vec![
            Transform {
                position: Vec3::ZERO,
                rotation: rot90y,
                scale: Vec3::ONE,
            },
            Transform::from_position(Vec3::new(1.0, 0.0, 0.0)),
        ]);

        let globals = skel.compute_global_transforms(&pose);
        let skin_mats = skel.compute_skin_matrices(&globals);

        // Vertex at child's bind position (1,0,0), fully bound to child (joint 1).
        let verts = vec![Vec3::new(1.0, 0.0, 0.0)];
        let skin = SkinData {
            joint_indices: vec![[1, 0, 0, 0]],
            weights: vec![[1.0, 0.0, 0.0, 0.0]],
        };
        let skinned_mesh = make_skinned_mesh(verts, skin);

        let mut out = vec![Vec3::ZERO; 1];
        skin_vertices(&skinned_mesh, &skin_mats, &mut out);

        // The child's skin matrix = T(-1,0,0) * (T(1,0,0) * rot_y(90))
        // T(1,0,0) * rot_y(90) computed:
        //   T(1,0,0) has m[3] = [1,0,0,1]
        //   Multiplied by rot_y(90)...
        //
        // Let's just verify the output position is correct.
        // The vertex (1,0,0) in bind space is at the child's bind origin.
        // After animation, the child's origin moves to (0,0,-1) in world.
        // The vertex was at child origin in bind, so it should be at child origin in animated = (0,0,-1).
        assert_vec3_near(
            out[0],
            Vec3::new(0.0, 0.0, -1.0),
            "end-to-end skinned vertex",
        );
    }
}
