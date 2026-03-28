//! Evaluated animation pose and skin matrices.

use abrash_core::math::Mat4;
use abrash_core::transform::Transform;

/// A computed pose: one local `Transform` per joint.
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
    use abrash_core::math::Vec3;

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
