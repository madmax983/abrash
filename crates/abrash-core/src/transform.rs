//! Decomposed TRS transform for smooth animation interpolation.

use crate::math::{Mat4, Vec3};
use crate::quat::Quat;

/// A decomposed Transform-Rotate-Scale (TRS) transform.
///
/// Unlike `Mat4`, this can be smoothly interpolated — position and scale
/// use linear interpolation while rotation uses SLERP via `Quat`.
///
/// # Convention
///
/// Composition order for row-vector convention (v·M):
/// `v * Scale * Rotation * Translation`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    /// Identity transform: origin, no rotation, uniform scale 1.
    #[must_use]
    #[inline]
    pub const fn identity() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::identity(),
            scale: Vec3::ONE,
        }
    }

    /// Create a transform with only a position (identity rotation, unit scale).
    #[must_use]
    #[inline]
    pub const fn from_position(position: Vec3) -> Self {
        Self {
            position,
            rotation: Quat::identity(),
            scale: Vec3::ONE,
        }
    }

    /// Compose into a 4×4 matrix: Scale * Rotation * Translation (row-vector convention).
    #[must_use]
    pub fn to_mat4(&self) -> Mat4 {
        let s = Mat4::scale(self.scale.x, self.scale.y, self.scale.z);
        let r = self.rotation.to_mat4();
        let t = Mat4::translation(self.position.x, self.position.y, self.position.z);
        s * r * t
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    const EPSILON: f32 = 1e-5;

    #[test]
    fn identity_produces_identity_matrix() {
        let t = Transform::identity();
        let m = t.to_mat4();
        let i = Mat4::identity();
        for row in 0..4 {
            for col in 0..4 {
                assert!(
                    (m.m[row][col] - i.m[row][col]).abs() < EPSILON,
                    "Mismatch at [{row}][{col}]"
                );
            }
        }
    }

    #[test]
    fn translation_only() {
        let t = Transform {
            position: Vec3::new(5.0, 10.0, 15.0),
            rotation: Quat::identity(),
            scale: Vec3::ONE,
        };
        let m = t.to_mat4();
        let expected = Mat4::translation(5.0, 10.0, 15.0);
        for row in 0..4 {
            for col in 0..4 {
                assert!(
                    (m.m[row][col] - expected.m[row][col]).abs() < EPSILON,
                    "Mismatch at [{row}][{col}]: got={} expected={}",
                    m.m[row][col],
                    expected.m[row][col]
                );
            }
        }
    }

    #[test]
    fn scale_only() {
        let t = Transform {
            position: Vec3::ZERO,
            rotation: Quat::identity(),
            scale: Vec3::new(2.0, 3.0, 4.0),
        };
        let m = t.to_mat4();
        let expected = Mat4::scale(2.0, 3.0, 4.0);
        for row in 0..4 {
            for col in 0..4 {
                assert!(
                    (m.m[row][col] - expected.m[row][col]).abs() < EPSILON,
                    "Mismatch at [{row}][{col}]"
                );
            }
        }
    }

    #[test]
    fn rotation_only() {
        let t = Transform {
            position: Vec3::ZERO,
            rotation: Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2),
            scale: Vec3::ONE,
        };
        let m = t.to_mat4();
        let expected = Mat4::rotation_y(FRAC_PI_2);
        for row in 0..4 {
            for col in 0..4 {
                assert!(
                    (m.m[row][col] - expected.m[row][col]).abs() < EPSILON,
                    "Mismatch at [{row}][{col}]: got={} expected={}",
                    m.m[row][col],
                    expected.m[row][col]
                );
            }
        }
    }

    #[test]
    fn combined_srt() {
        let t = Transform {
            position: Vec3::new(10.0, 0.0, 0.0),
            rotation: Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2),
            scale: Vec3::new(2.0, 2.0, 2.0),
        };
        let m = t.to_mat4();
        let expected = Mat4::scale(2.0, 2.0, 2.0)
            * Mat4::rotation_y(FRAC_PI_2)
            * Mat4::translation(10.0, 0.0, 0.0);
        for row in 0..4 {
            for col in 0..4 {
                assert!(
                    (m.m[row][col] - expected.m[row][col]).abs() < EPSILON,
                    "Mismatch at [{row}][{col}]: got={} expected={}",
                    m.m[row][col],
                    expected.m[row][col]
                );
            }
        }
    }

    #[test]
    fn from_position_helper() {
        let t = Transform::from_position(Vec3::new(1.0, 2.0, 3.0));
        assert!((t.position.x - 1.0).abs() < EPSILON);
        assert!((t.rotation.w - 1.0).abs() < EPSILON);
        assert!((t.scale.x - 1.0).abs() < EPSILON);
    }
}
