//! Evaluated animation pose and skin matrices.

use abrash_core::math::Mat4;
use abrash_core::transform::Transform;

/// A computed pose: one local `Transform` per joint.
///
/// Contains the evaluated local-space transforms for a skeleton at a specific
/// point in time. Can be blended with other poses or converted into global
/// skin matrices for vertex displacement.
///
/// # Examples
/// ```
/// use abrash_skeletal::pose::Pose;
/// use abrash_core::transform::Transform;
///
/// let pose = Pose::new(vec![Transform::identity()]);
/// assert_eq!(pose.local_transforms.len(), 1);
/// ```
#[derive(Debug, Clone)]
pub struct Pose {
    /// Local-space transform for each joint, indexed by `JointId`.
    pub local_transforms: Vec<Transform>,
}

impl Pose {
    /// Create a pose from a vector of transforms.
    #[must_use]
    pub const fn new(transforms: Vec<Transform>) -> Self {
        Self {
            local_transforms: transforms,
        }
    }

    /// Create a bind pose from a skeleton's rest transforms.
    #[must_use]
    pub fn from_bind(skeleton: &crate::skeleton::Skeleton) -> Self {
        // ⚡ Bolt: Uses `extend` with `with_capacity` instead of `.collect::<Vec<_>>()`.
        let mut local_transforms = Vec::with_capacity(skeleton.joints.len());
        local_transforms.extend(skeleton.joints.iter().map(|j| j.bind_transform));
        Self { local_transforms }
    }
}

/// Joint matrices ready for vertex skinning.
///
/// Each matrix = `inverse_bind_matrix * global_transform` for that joint.
/// In row-vector convention this means: a vertex in bind space is first
/// moved to bone-local space (inverse bind), then to world space (global).
#[derive(Debug, Clone)]
pub struct SkinMatrices {
    /// One skin matrix per joint.
    pub matrices: Vec<Mat4>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skeleton::{Joint, JointId, Skeleton};
    use abrash_core::math::Vec3;

    #[test]
    fn pose_from_bind_matches_joint_transforms() {
        let joints = vec![
            Joint {
                name: "root".to_string(),
                parent: None,
                inverse_bind_matrix: Mat4::identity(),
                bind_transform: Transform::from_position(Vec3::new(1.0, 2.0, 3.0)),
            },
            Joint {
                name: "child".to_string(),
                parent: Some(JointId(0)),
                inverse_bind_matrix: Mat4::identity(),
                bind_transform: Transform::from_position(Vec3::new(4.0, 5.0, 6.0)),
            },
        ];
        let skel = Skeleton::new(joints);
        let pose = Pose::from_bind(&skel);

        assert_eq!(pose.local_transforms.len(), 2);
        let epsilon = 1e-5;
        assert!((pose.local_transforms[0].position.x - 1.0).abs() < epsilon);
        assert!((pose.local_transforms[1].position.x - 4.0).abs() < epsilon);
    }

    #[test]
    fn pose_new_stores_transforms() {
        let transforms = vec![
            Transform::identity(),
            Transform::from_position(Vec3::new(10.0, 0.0, 0.0)),
        ];
        let pose = Pose::new(transforms);
        assert_eq!(pose.local_transforms.len(), 2);
        let epsilon = 1e-5;
        assert!((pose.local_transforms[1].position.x - 10.0).abs() < epsilon);
    }
}
