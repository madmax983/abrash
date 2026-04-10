//! Skeleton hierarchy and forward kinematics.

use abrash_core::math::Mat4;
use abrash_core::transform::Transform;

use crate::pose::{Pose, SkinMatrices};

/// Index into a skeleton's joint array.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JointId(pub u16);

/// A single joint in the skeleton.
#[derive(Debug, Clone)]
pub struct Joint {
    /// Human-readable name (e.g. "`LeftShoulder`").
    pub name: String,
    /// Parent joint, or `None` for a root joint.
    pub parent: Option<JointId>,
    /// The inverse of the bind-pose world matrix for this joint.
    /// Used to transform vertices from model space to bone-local space.
    pub inverse_bind_matrix: Mat4,
    /// Local-space rest (bind) transform for this joint.
    pub bind_transform: Transform,
}

/// A skeleton: an ordered list of joints where parents precede children.
///
/// This ordering enables single-pass forward kinematics (no recursion needed).
#[derive(Debug, Clone)]
pub struct Skeleton {
    /// Joints ordered so that every parent's index is less than its children's.
    pub joints: Vec<Joint>,
}

impl Skeleton {
    /// Create a new skeleton, validating that parents precede children.
    ///
    /// # Panics
    ///
    /// Panics if any joint references a parent index >= its own index,
    /// or a parent index that is out of bounds.
    #[must_use]
    pub fn new(joints: Vec<Joint>) -> Self {
        for (i, joint) in joints.iter().enumerate() {
            if let Some(parent_id) = joint.parent {
                let parent_idx = parent_id.0 as usize;
                assert!(
                    parent_idx < i,
                    "Joint {i} (\"{}\") has parent index {parent_idx} which is not < {i}. \
                     Parents must precede children in the joint array.",
                    joint.name
                );
            }
        }
        Self { joints }
    }

    /// Number of joints in the skeleton.
    #[must_use]
    #[inline]
    pub const fn joint_count(&self) -> usize {
        self.joints.len()
    }

    /// Updates world-space transforms for every joint into the provided `globals` buffer, avoiding reallocation.
    ///
    /// Uses the row-vector convention: `global[i] = local[i].to_mat4() * global[parent]`.
    /// Root joints (no parent) use their local transform directly.
    ///
    /// # Panics
    ///
    /// Panics if `pose.local_transforms.len() != self.joints.len()`.
    pub fn update_global_transforms(&self, pose: &Pose, globals: &mut Vec<Mat4>) {
        let n = self.joints.len();
        assert_eq!(
            pose.local_transforms.len(),
            n,
            "Pose has {} transforms but skeleton has {n} joints",
            pose.local_transforms.len()
        );

        globals.clear();
        for (i, joint) in self.joints.iter().enumerate() {
            let local_mat = pose.local_transforms[i].to_mat4();
            let global = joint.parent.map_or(local_mat, |parent_id| {
                // Row-vector convention: local * parent_global
                local_mat * globals[parent_id.0 as usize]
            });
            globals.push(global);
        }
    }

    /// Compute world-space transforms for every joint via forward kinematics.
    ///
    /// Note: This dynamically allocates a `Vec<Mat4>`. If evaluating poses
    /// per-frame, consider using `update_global_transforms` with a pre-allocated
    /// buffer to eliminate dynamic heap allocations.
    ///
    /// # Panics
    ///
    /// Panics if `pose.local_transforms.len() != self.joints.len()`.
    #[must_use]
    pub fn compute_global_transforms(&self, pose: &Pose) -> Vec<Mat4> {
        let mut globals = Vec::with_capacity(self.joints.len());
        self.update_global_transforms(pose, &mut globals);
        globals
    }

    /// Updates skin matrices from global transforms into the provided `out` buffer, avoiding reallocation.
    ///
    /// Each skin matrix = `inverse_bind[i] * global[i]` (row-vector convention).
    /// This transforms vertices from bind space through bone-local space to world space.
    ///
    /// # Panics
    ///
    /// Panics if `global_transforms.len() != self.joints.len()`.
    pub fn update_skin_matrices(&self, global_transforms: &[Mat4], out: &mut SkinMatrices) {
        let n = self.joints.len();
        assert_eq!(
            global_transforms.len(),
            n,
            "Got {} global transforms but skeleton has {n} joints",
            global_transforms.len()
        );

        out.matrices.clear();
        out.matrices.extend(
            self.joints
                .iter()
                .zip(global_transforms)
                .map(|(joint, global)| joint.inverse_bind_matrix * *global)
        );
    }

    /// Compute skin matrices from global transforms.
    ///
    /// Note: This dynamically allocates a `Vec<Mat4>`. If evaluating poses
    /// per-frame, consider using `update_skin_matrices` with a pre-allocated
    /// buffer to eliminate dynamic heap allocations.
    ///
    /// # Panics
    ///
    /// Panics if `global_transforms.len() != self.joints.len()`.
    #[must_use]
    pub fn compute_skin_matrices(&self, global_transforms: &[Mat4]) -> SkinMatrices {
        let mut out = SkinMatrices { matrices: Vec::with_capacity(self.joints.len()) };
        self.update_skin_matrices(global_transforms, &mut out);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::math::Vec3;
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

    fn assert_mat4_near(a: &Mat4, b: &Mat4, label: &str) {
        for row in 0..4 {
            for col in 0..4 {
                assert!(
                    (a.m[row][col] - b.m[row][col]).abs() < EPSILON,
                    "{label}: mismatch at [{row}][{col}]: {} vs {}",
                    a.m[row][col],
                    b.m[row][col]
                );
            }
        }
    }

    #[test]
    fn single_root_identity_produces_identity() {
        let joints = vec![Joint {
            name: "root".to_string(),
            parent: None,
            inverse_bind_matrix: Mat4::identity(),
            bind_transform: Transform::identity(),
        }];
        let skel = Skeleton::new(joints);
        let pose = Pose::from_bind(&skel);
        let globals = skel.compute_global_transforms(&pose);

        assert_eq!(globals.len(), 1);
        assert_mat4_near(&globals[0], &Mat4::identity(), "root global");
    }

    #[test]
    fn two_joint_chain_offset() {
        // Root at origin, child offset by (1, 0, 0) in local space.
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
                inverse_bind_matrix: Mat4::identity(),
                bind_transform: Transform::from_position(Vec3::new(1.0, 0.0, 0.0)),
            },
        ];
        let skel = Skeleton::new(joints);
        let pose = Pose::from_bind(&skel);
        let globals = skel.compute_global_transforms(&pose);

        // Root is identity.
        assert_mat4_near(&globals[0], &Mat4::identity(), "root global");

        // Child: local = T(1,0,0). global = T(1,0,0) * Identity = T(1,0,0).
        // Verify by transforming origin through the global matrix.
        let (child_world_pos, _) = globals[1].transform_point(Vec3::ZERO);
        assert_vec3_near(child_world_pos, Vec3::new(1.0, 0.0, 0.0), "child world pos");
    }

    #[test]
    fn three_joint_chain_with_rotation() {
        // Root at origin with 90-degree Y rotation.
        // Child at (1, 0, 0) local offset.
        // Grandchild at (1, 0, 0) local offset.
        //
        // With 90-deg Y rotation at root (right-handed):
        //   Root global: rotation_y(90)
        //   A point (1,0,0) in root space -> (0,0,-1) in world space
        //
        //   Child local: T(1,0,0), so child global = T(1,0,0) * rot_y(90)
        //   The child's origin in world = (1,0,0) transformed by rot_y(90):
        //   In row-vector: (1,0,0) * rot_y(90) = (0, 0, -1)
        //
        //   Grandchild local: T(1,0,0), so grandchild global = T(1,0,0) * child_global
        //   The grandchild's origin in world = (1,0,0) transformed by child_global
        //   child_global = T(1,0,0) * rot_y(90)
        //   (1,0,0) * T(1,0,0) = (2,0,0)
        //   (2,0,0) * rot_y(90) = ?
        //   rot_y(90) maps (1,0,0)->(0,0,-1), so (2,0,0) -> (0,0,-2)

        let rot90y = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);

        let joints = vec![
            Joint {
                name: "root".to_string(),
                parent: None,
                inverse_bind_matrix: Mat4::identity(),
                bind_transform: Transform {
                    position: Vec3::ZERO,
                    rotation: rot90y,
                    scale: Vec3::ONE,
                },
            },
            Joint {
                name: "child".to_string(),
                parent: Some(JointId(0)),
                inverse_bind_matrix: Mat4::identity(),
                bind_transform: Transform::from_position(Vec3::new(1.0, 0.0, 0.0)),
            },
            Joint {
                name: "grandchild".to_string(),
                parent: Some(JointId(1)),
                inverse_bind_matrix: Mat4::identity(),
                bind_transform: Transform::from_position(Vec3::new(1.0, 0.0, 0.0)),
            },
        ];
        let skel = Skeleton::new(joints);
        let pose = Pose::from_bind(&skel);
        let globals = skel.compute_global_transforms(&pose);

        // Root: just rot_y(90)
        let (root_origin, _) = globals[0].transform_point(Vec3::ZERO);
        assert_vec3_near(root_origin, Vec3::ZERO, "root origin");

        // Child's origin in world: (1,0,0) local, transformed by root global
        // child_global = T(1,0,0) * rot_y(90)
        // Origin through child_global: (0,0,0)*T(1,0,0) = (1,0,0), then (1,0,0)*rot_y(90) = (0,0,-1)
        let (child_origin, _) = globals[1].transform_point(Vec3::ZERO);
        assert_vec3_near(
            child_origin,
            Vec3::new(0.0, 0.0, -1.0),
            "child world origin",
        );

        // Grandchild's origin: local (1,0,0), parent is child
        // grandchild_global = T(1,0,0) * child_global = T(1,0,0) * T(1,0,0) * rot_y(90)
        // (0,0,0)*T(1,0,0) = (1,0,0), *T(1,0,0) = (2,0,0), *rot_y(90) = (0,0,-2)
        let (gc_origin, _) = globals[2].transform_point(Vec3::ZERO);
        assert_vec3_near(
            gc_origin,
            Vec3::new(0.0, 0.0, -2.0),
            "grandchild world origin",
        );
    }

    #[test]
    fn skin_matrices_identity_bind() {
        // If inverse_bind = identity and global = identity, skin matrix = identity.
        let joints = vec![Joint {
            name: "root".to_string(),
            parent: None,
            inverse_bind_matrix: Mat4::identity(),
            bind_transform: Transform::identity(),
        }];
        let skel = Skeleton::new(joints);
        let pose = Pose::from_bind(&skel);
        let globals = skel.compute_global_transforms(&pose);
        let skin = skel.compute_skin_matrices(&globals);

        assert_eq!(skin.matrices.len(), 1);
        assert_mat4_near(&skin.matrices[0], &Mat4::identity(), "skin matrix 0");
    }

    #[test]
    fn skin_matrices_cancel_bind_pose() {
        // When the pose matches the bind pose, skin matrices should be identity
        // (because inverse_bind * bind_global = identity).
        //
        // Root at (1, 0, 0).
        // inverse_bind = inverse of T(1,0,0) = T(-1,0,0).
        let bind_transform = Transform::from_position(Vec3::new(1.0, 0.0, 0.0));
        // Inverse of a pure translation T(x,y,z) is T(-x,-y,-z).
        let inverse_bind = Mat4::translation(-1.0, 0.0, 0.0);

        let joints = vec![Joint {
            name: "root".to_string(),
            parent: None,
            inverse_bind_matrix: inverse_bind,
            bind_transform,
        }];
        let skel = Skeleton::new(joints);
        let pose = Pose::from_bind(&skel);
        let globals = skel.compute_global_transforms(&pose);
        let skin = skel.compute_skin_matrices(&globals);

        // inverse_bind * global = T(-1,0,0) * T(1,0,0) = identity
        assert_mat4_near(&skin.matrices[0], &Mat4::identity(), "skin cancels bind");
    }

    #[test]
    #[should_panic(expected = "parent index")]
    fn invalid_parent_ordering_panics() {
        // Joint 0 claims parent is joint 1 — that violates ordering.
        let joints = vec![
            Joint {
                name: "bad_child".to_string(),
                parent: Some(JointId(1)),
                inverse_bind_matrix: Mat4::identity(),
                bind_transform: Transform::identity(),
            },
            Joint {
                name: "root".to_string(),
                parent: None,
                inverse_bind_matrix: Mat4::identity(),
                bind_transform: Transform::identity(),
            },
        ];
        let _ = Skeleton::new(joints); // should panic
    }

    #[test]
    #[should_panic(expected = "parent index")]
    fn self_referencing_parent_panics() {
        let joints = vec![Joint {
            name: "self_ref".to_string(),
            parent: Some(JointId(0)),
            inverse_bind_matrix: Mat4::identity(),
            bind_transform: Transform::identity(),
        }];
        let _ = Skeleton::new(joints); // should panic
    }

    #[test]
    fn joint_count_returns_correct_value() {
        let joints = vec![
            Joint {
                name: "a".to_string(),
                parent: None,
                inverse_bind_matrix: Mat4::identity(),
                bind_transform: Transform::identity(),
            },
            Joint {
                name: "b".to_string(),
                parent: Some(JointId(0)),
                inverse_bind_matrix: Mat4::identity(),
                bind_transform: Transform::identity(),
            },
            Joint {
                name: "c".to_string(),
                parent: Some(JointId(0)),
                inverse_bind_matrix: Mat4::identity(),
                bind_transform: Transform::identity(),
            },
        ];
        let skel = Skeleton::new(joints);
        assert_eq!(skel.joint_count(), 3);
    }

    #[test]
    fn fk_with_rotated_parent_translates_child_correctly() {
        // Root rotated 90 degrees around Y.
        // Child has local offset (0, 0, 1).
        //
        // In row-vector convention:
        // child_global = T(0,0,1) * rot_y(90)
        // Origin through child: (0,0,0)*T(0,0,1) = (0,0,1), then (0,0,1)*rot_y(90)
        //
        // rot_y(90) applied to (0,0,1): cos(90)*0 + sin(90)*1 = 1 for x? Let's check.
        // Row-vector: [x y z 1] * rot_y
        // For rotation_y in row-major row-vector:
        //   x' = x*cos + z*sin
        //   z' = -x*sin + z*cos
        // So (0,0,1) -> x'=0+sin(90)=1, z'=0+cos(90)=0 -> (1,0,0)

        let rot90y = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);

        let joints = vec![
            Joint {
                name: "root".to_string(),
                parent: None,
                inverse_bind_matrix: Mat4::identity(),
                bind_transform: Transform {
                    position: Vec3::ZERO,
                    rotation: rot90y,
                    scale: Vec3::ONE,
                },
            },
            Joint {
                name: "child".to_string(),
                parent: Some(JointId(0)),
                inverse_bind_matrix: Mat4::identity(),
                bind_transform: Transform::from_position(Vec3::new(0.0, 0.0, 1.0)),
            },
        ];
        let skel = Skeleton::new(joints);
        let pose = Pose::from_bind(&skel);
        let globals = skel.compute_global_transforms(&pose);

        let (child_world, _) = globals[1].transform_point(Vec3::ZERO);
        assert_vec3_near(
            child_world,
            Vec3::new(1.0, 0.0, 0.0),
            "child (0,0,1) after parent rot_y(90)",
        );
    }
}
