//! Trait for types that can be smoothly interpolated in animations.

use crate::math::{Vec2, Vec3};
use crate::quat::Quat;
use crate::transform::Transform;

/// A type that supports interpolation for animation.
///
/// Implement this for any type you want to animate with `abrash-anim`.
pub trait Animatable: Clone + 'static {
    /// Interpolate between `self` and `other`.
    /// `t` is clamped to 0.0-1.0. At t=0 returns self, at t=1 returns other.
    #[must_use]
    fn interpolate(&self, other: &Self, t: f32) -> Self;

    /// The additive identity (zero value).
    fn zero() -> Self;
}

impl Animatable for f32 {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        self + (other - self) * t
    }

    fn zero() -> Self {
        0.0
    }
}

impl Animatable for Vec2 {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        self.lerp(*other, t)
    }

    fn zero() -> Self {
        Self::new(0.0, 0.0)
    }
}

impl Animatable for Vec3 {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        self.lerp(*other, t)
    }

    fn zero() -> Self {
        Self::ZERO
    }
}

impl Animatable for Quat {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        self.slerp(other, t)
    }

    fn zero() -> Self {
        Self::identity()
    }
}

impl Animatable for Transform {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        Self {
            position: self.position.interpolate(&other.position, t),
            rotation: self.rotation.interpolate(&other.rotation, t),
            scale: self.scale.interpolate(&other.scale, t),
        }
    }

    fn zero() -> Self {
        Self::identity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f32_interpolate_midpoint() {
        let a: f32 = 0.0;
        let result = a.interpolate(&10.0, 0.5);
        assert!((result - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn f32_interpolate_boundaries() {
        let a: f32 = 2.0;
        let b: f32 = 8.0;
        assert!((a.interpolate(&b, 0.0) - 2.0).abs() < f32::EPSILON);
        assert!((a.interpolate(&b, 1.0) - 8.0).abs() < f32::EPSILON);
    }

    #[test]
    fn f32_zero() {
        assert!((f32::zero()).abs() < f32::EPSILON);
    }

    use crate::math::{Vec2, Vec3};

    #[test]
    fn vec2_interpolate_midpoint() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(10.0, 20.0);
        let mid = a.interpolate(&b, 0.5);
        assert!((mid.x - 5.0).abs() < f32::EPSILON);
        assert!((mid.y - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn vec2_zero() {
        let z = Vec2::zero();
        assert!((z.x).abs() < f32::EPSILON);
        assert!((z.y).abs() < f32::EPSILON);
    }

    #[test]
    fn vec3_interpolate_midpoint() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(10.0, 20.0, 30.0);
        let mid = a.interpolate(&b, 0.5);
        assert!((mid.x - 5.0).abs() < f32::EPSILON);
        assert!((mid.y - 10.0).abs() < f32::EPSILON);
        assert!((mid.z - 15.0).abs() < f32::EPSILON);
    }

    use crate::quat::Quat;

    #[test]
    fn quat_interpolate_uses_slerp() {
        let a = Quat::identity();
        let b = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), std::f32::consts::FRAC_PI_2);
        let mid = a.interpolate(&b, 0.5);
        let v = mid.rotate_vec3(Vec3::new(1.0, 0.0, 0.0));
        let expected_x = (std::f32::consts::FRAC_PI_2 / 2.0).cos();
        assert!((v.x - expected_x).abs() < 1e-5);
    }

    #[test]
    fn quat_zero_is_identity() {
        let z = Quat::zero();
        assert!((z.w - 1.0).abs() < 1e-5);
    }

    use crate::transform::Transform;

    #[test]
    fn transform_interpolate_lerps_position() {
        let a = Transform::from_position(Vec3::new(0.0, 0.0, 0.0));
        let b = Transform::from_position(Vec3::new(10.0, 0.0, 0.0));
        let mid = a.interpolate(&b, 0.5);
        assert!((mid.position.x - 5.0).abs() < 1e-5);
    }

    #[test]
    fn transform_interpolate_slerps_rotation() {
        let a = Transform::identity();
        let mut b = Transform::identity();
        b.rotation = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), std::f32::consts::FRAC_PI_2);
        let mid = a.interpolate(&b, 0.5);
        let v = mid.rotation.rotate_vec3(Vec3::new(1.0, 0.0, 0.0));
        let expected_x = (std::f32::consts::FRAC_PI_2 / 2.0).cos();
        assert!((v.x - expected_x).abs() < 1e-5);
    }

    #[test]
    fn transform_interpolate_lerps_scale() {
        let a = Transform {
            position: Vec3::ZERO,
            rotation: Quat::identity(),
            scale: Vec3::new(1.0, 1.0, 1.0),
        };
        let b = Transform {
            position: Vec3::ZERO,
            rotation: Quat::identity(),
            scale: Vec3::new(3.0, 3.0, 3.0),
        };
        let mid = a.interpolate(&b, 0.5);
        assert!((mid.scale.x - 2.0).abs() < 1e-5);
        assert!((mid.scale.y - 2.0).abs() < 1e-5);
        assert!((mid.scale.z - 2.0).abs() < 1e-5);
    }

    #[test]
    fn transform_zero_is_identity() {
        let z = Transform::zero();
        assert!((z.position.x).abs() < 1e-5);
        assert!((z.rotation.w - 1.0).abs() < 1e-5);
        assert!((z.scale.x - 1.0).abs() < 1e-5);
    }
}
