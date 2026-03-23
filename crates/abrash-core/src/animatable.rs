//! Trait for types that can be smoothly interpolated in animations.

use crate::math::{Vec2, Vec3};
use crate::quat::Quat;

/// A type that supports interpolation, arithmetic, and distance for animation.
///
/// Implement this for any type you want to animate with `abrash-anim`.
pub trait Animatable: Clone + 'static {
    /// Interpolate between `self` and `other`.
    /// `t` is clamped to 0.0–1.0. At t=0 returns self, at t=1 returns other.
    fn interpolate(&self, other: &Self, t: f32) -> Self;

    /// Scale the value by a scalar factor.
    fn anim_scale(&self, scalar: f32) -> Self;

    /// Add another value to this one.
    fn anim_add(&self, other: &Self) -> Self;

    /// Subtract another value from this one.
    fn anim_sub(&self, other: &Self) -> Self;

    /// The additive identity (zero value).
    fn zero() -> Self;

    /// Squared distance between two values. Used for settling detection.
    fn distance_squared(&self, other: &Self) -> f32;
}

impl Animatable for f32 {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        self + (other - self) * t
    }

    fn anim_scale(&self, scalar: f32) -> Self {
        self * scalar
    }

    fn anim_add(&self, other: &Self) -> Self {
        self + other
    }

    fn anim_sub(&self, other: &Self) -> Self {
        self - other
    }

    fn zero() -> Self {
        0.0
    }

    fn distance_squared(&self, other: &Self) -> f32 {
        (self - other).powi(2)
    }
}

impl Animatable for Vec2 {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        self.lerp(*other, t)
    }

    fn anim_scale(&self, scalar: f32) -> Self {
        *self * scalar
    }

    fn anim_add(&self, other: &Self) -> Self {
        *self + *other
    }

    fn anim_sub(&self, other: &Self) -> Self {
        *self - *other
    }

    fn zero() -> Self {
        Self::new(0.0, 0.0)
    }

    fn distance_squared(&self, other: &Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }
}

impl Animatable for Vec3 {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        self.lerp(*other, t)
    }

    fn anim_scale(&self, scalar: f32) -> Self {
        *self * scalar
    }

    fn anim_add(&self, other: &Self) -> Self {
        *self + *other
    }

    fn anim_sub(&self, other: &Self) -> Self {
        *self - *other
    }

    fn zero() -> Self {
        Vec3::ZERO
    }

    fn distance_squared(&self, other: &Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }
}

impl Animatable for Quat {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        self.slerp(other, t)
    }

    fn anim_scale(&self, scalar: f32) -> Self {
        Quat::identity().slerp(self, scalar)
    }

    fn anim_add(&self, other: &Self) -> Self {
        *self * *other
    }

    fn anim_sub(&self, other: &Self) -> Self {
        *self * other.conjugate()
    }

    fn zero() -> Self {
        Quat::identity()
    }

    fn distance_squared(&self, other: &Self) -> f32 {
        let dot = self.dot(*other).abs();
        1.0 - dot * dot
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

    #[test]
    fn f32_add_sub_roundtrip() {
        let a: f32 = 3.0;
        let b: f32 = 7.0;
        let sum = a.anim_add(&b);
        let diff = sum.anim_sub(&b);
        assert!((diff - a).abs() < f32::EPSILON);
    }

    #[test]
    fn f32_scale() {
        let a: f32 = 5.0;
        assert!((a.anim_scale(2.0) - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn f32_distance_squared() {
        let a: f32 = 3.0;
        let b: f32 = 7.0;
        assert!((a.distance_squared(&b) - 16.0).abs() < f32::EPSILON);
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

    #[test]
    fn vec3_add_sub_roundtrip() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        let sum = a.anim_add(&b);
        let diff = sum.anim_sub(&b);
        assert!((diff.x - a.x).abs() < f32::EPSILON);
        assert!((diff.y - a.y).abs() < f32::EPSILON);
        assert!((diff.z - a.z).abs() < f32::EPSILON);
    }

    #[test]
    fn vec3_distance_squared() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(3.0, 4.0, 0.0);
        assert!((a.distance_squared(&b) - 25.0).abs() < f32::EPSILON);
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

    #[test]
    fn quat_distance_squared_same_is_zero() {
        let q = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), 0.5);
        assert!(q.distance_squared(&q) < 1e-10);
    }
}
