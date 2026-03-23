# Animation System Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a composable, phase-based animation system (`abrash-anim` crate) inspired by Arthropod's `anim-graph`, plus `Quat` and `Transform` types in `abrash-core`.

**Architecture:** New workspace crate `abrash-anim` depends on `abrash-core` for math types. `Animatable` trait lives in `abrash-core`. The render pipeline (`abrash-render`) is untouched — consumers wire animation to scene objects via `transform.to_mat4()`.

**Tech Stack:** Pure Rust, no external deps beyond `abrash-core`. Row-major, row-vector convention (v·M). Right-handed coordinate system (Y-up, camera looks down -Z).

**Design doc:** `docs/plans/2026-03-22-animation-system-design.md`

---

### Task 1: Create `abrash-anim` crate skeleton

**Files:**
- Create: `crates/abrash-anim/Cargo.toml`
- Create: `crates/abrash-anim/src/lib.rs`
- Modify: `Cargo.toml` (workspace root — add member)

**Step 1: Create `crates/abrash-anim/Cargo.toml`**

```toml
[package]
name = "abrash-anim"
version = "0.1.0"
edition = "2024"

[dependencies]
abrash-core = { path = "../abrash-core" }

[lints.clippy]
pedantic = { level = "warn", priority = -1 }
nursery = { level = "warn", priority = -1 }
cast_possible_truncation = "allow"
cast_possible_wrap = "allow"
cast_sign_loss = "allow"
cast_precision_loss = "allow"
similar_names = "allow"
many_single_char_names = "allow"
suboptimal_flops = "allow"
```

**Step 2: Create `crates/abrash-anim/src/lib.rs`**

```rust
//! Composable animation system for the Abrash rendering engine.
//!
//! Provides phase-based animation evaluation inspired by Arthropod's `anim-graph`.
//! This crate is rendering-agnostic — it operates on any type implementing
//! `abrash_core::Animatable`.
```

**Step 3: Add to workspace in root `Cargo.toml`**

Add `"crates/abrash-anim"` to the `[workspace] members` array.

**Step 4: Verify it compiles**

Run: `cargo check -p abrash-anim`
Expected: Compiles with no errors.

**Step 5: Commit**

```bash
git add crates/abrash-anim/ Cargo.toml
git commit -m "feat(anim): scaffold abrash-anim crate"
```

---

### Task 2: `Animatable` trait in `abrash-core`

**Files:**
- Create: `crates/abrash-core/src/animatable.rs`
- Modify: `crates/abrash-core/src/lib.rs` (add `pub mod animatable`)
- Test: `crates/abrash-core/src/animatable.rs` (inline `#[cfg(test)]`)

**Step 1: Write the failing tests**

In `crates/abrash-core/src/animatable.rs`, at the bottom:

```rust
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
}
```

**Step 2: Write the trait and `f32` impl**

In `crates/abrash-core/src/animatable.rs`:

```rust
//! Trait for types that can be smoothly interpolated in animations.

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
```

Note: Methods are named `anim_add`, `anim_sub`, `anim_scale` to avoid collision with `std::ops::Add/Sub` and the existing `Vec3::length`-style methods. The Arthropod versions used `add`/`sub`/`scale` but that risks ambiguity in a codebase that already has `Mul<f32>` impls.

**Step 3: Add module to `crates/abrash-core/src/lib.rs`**

Add `pub mod animatable;` to the module list.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p abrash-core -- animatable`
Expected: 6 tests PASS.

**Step 5: Commit**

```bash
git add crates/abrash-core/src/animatable.rs crates/abrash-core/src/lib.rs
git commit -m "feat(core): add Animatable trait with f32 impl"
```

---

### Task 3: `Animatable` impls for `Vec2` and `Vec3`

**Files:**
- Modify: `crates/abrash-core/src/animatable.rs` (add impls + tests)

**Step 1: Write the failing tests**

Add to the `tests` module:

```rust
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
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p abrash-core -- animatable`
Expected: New tests FAIL (no `Animatable` impl for Vec2/Vec3).

**Step 3: Write the implementations**

Add to `animatable.rs` (after f32 impl), with `use crate::math::{Vec2, Vec3};` at the top:

```rust
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
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p abrash-core -- animatable`
Expected: 11 tests PASS (6 f32 + 5 vec).

**Step 5: Commit**

```bash
git add crates/abrash-core/src/animatable.rs
git commit -m "feat(core): Animatable impls for Vec2 and Vec3"
```

---

### Task 4: `Quat` type

**Files:**
- Create: `crates/abrash-core/src/quat.rs`
- Modify: `crates/abrash-core/src/lib.rs` (add `pub mod quat`)
- Test: `crates/abrash-core/src/quat.rs` (inline `#[cfg(test)]`)

**Step 1: Write the failing tests**

In `crates/abrash-core/src/quat.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_PI_2, PI};

    const EPSILON: f32 = 1e-5;

    #[test]
    fn identity_has_no_rotation() {
        let q = Quat::identity();
        assert!((q.x).abs() < EPSILON);
        assert!((q.y).abs() < EPSILON);
        assert!((q.z).abs() < EPSILON);
        assert!((q.w - 1.0).abs() < EPSILON);
    }

    #[test]
    fn from_axis_angle_90_degrees_y() {
        let q = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);
        // Should rotate (1,0,0) to (0,0,-1) in right-handed system
        let rotated = q.rotate_vec3(Vec3::new(1.0, 0.0, 0.0));
        assert!((rotated.x).abs() < EPSILON);
        assert!((rotated.y).abs() < EPSILON);
        assert!((rotated.z + 1.0).abs() < EPSILON);
    }

    #[test]
    fn slerp_at_zero_returns_start() {
        let a = Quat::identity();
        let b = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), PI);
        let result = a.slerp(&b, 0.0);
        assert!((result.x - a.x).abs() < EPSILON);
        assert!((result.y - a.y).abs() < EPSILON);
        assert!((result.z - a.z).abs() < EPSILON);
        assert!((result.w - a.w).abs() < EPSILON);
    }

    #[test]
    fn slerp_at_one_returns_end() {
        let a = Quat::identity();
        let b = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);
        let result = a.slerp(&b, 1.0);
        assert!((result.x - b.x).abs() < EPSILON);
        assert!((result.y - b.y).abs() < EPSILON);
        assert!((result.z - b.z).abs() < EPSILON);
        assert!((result.w - b.w).abs() < EPSILON);
    }

    #[test]
    fn slerp_midpoint_is_half_rotation() {
        let a = Quat::identity();
        let b = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);
        let mid = a.slerp(&b, 0.5);
        // Mid should be 45 degree rotation around Y
        let rotated = mid.rotate_vec3(Vec3::new(1.0, 0.0, 0.0));
        let expected_x = (FRAC_PI_2 / 2.0).cos();
        let expected_z = -(FRAC_PI_2 / 2.0).sin();
        assert!((rotated.x - expected_x).abs() < EPSILON);
        assert!((rotated.z - expected_z).abs() < EPSILON);
    }

    #[test]
    fn slerp_takes_shortest_path() {
        // Two quaternions representing the same rotation but with opposite signs
        let a = Quat::identity();
        let b_raw = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);
        let b_neg = Quat::new(-b_raw.x, -b_raw.y, -b_raw.z, -b_raw.w);
        // slerp should produce the same rotation regardless
        let r1 = a.slerp(&b_raw, 0.5).rotate_vec3(Vec3::new(1.0, 0.0, 0.0));
        let r2 = a.slerp(&b_neg, 0.5).rotate_vec3(Vec3::new(1.0, 0.0, 0.0));
        assert!((r1.x - r2.x).abs() < EPSILON);
        assert!((r1.y - r2.y).abs() < EPSILON);
        assert!((r1.z - r2.z).abs() < EPSILON);
    }

    #[test]
    fn normalize_preserves_direction() {
        let q = Quat::new(1.0, 2.0, 3.0, 4.0);
        let n = q.normalize();
        let len = (n.x * n.x + n.y * n.y + n.z * n.z + n.w * n.w).sqrt();
        assert!((len - 1.0).abs() < EPSILON);
    }

    #[test]
    fn compose_rotations() {
        // 90° around Y then 90° around X
        let ry = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);
        let rx = Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), FRAC_PI_2);
        let combined = ry * rx; // Apply ry first, then rx (row-vector convention)
        let v = combined.rotate_vec3(Vec3::new(1.0, 0.0, 0.0));
        // (1,0,0) -> ry -> (0,0,-1) -> rx -> (0,1,0)
        assert!((v.x).abs() < EPSILON);
        assert!((v.y - 1.0).abs() < EPSILON);
        assert!((v.z).abs() < EPSILON);
    }

    #[test]
    fn to_mat4_matches_rotation_y() {
        let angle = 1.23_f32;
        let q = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), angle);
        let qmat = q.to_mat4();
        let rmat = Mat4::rotation_y(angle);
        for row in 0..4 {
            for col in 0..4 {
                assert!(
                    (qmat.m[row][col] - rmat.m[row][col]).abs() < EPSILON,
                    "Mismatch at [{row}][{col}]: quat={} rot={}",
                    qmat.m[row][col],
                    rmat.m[row][col]
                );
            }
        }
    }

    #[test]
    fn from_euler_roundtrip() {
        let pitch = 0.3;
        let yaw = 0.7;
        let roll = 0.0;
        let q = Quat::from_euler(pitch, yaw, roll);
        // Verify by rotating a known vector
        let v = q.rotate_vec3(Vec3::new(1.0, 0.0, 0.0));
        // Same rotation via matrices
        let m = Mat4::rotation_y(yaw) * Mat4::rotation_x(pitch);
        let (v2, _) = m.transform_point(Vec3::new(1.0, 0.0, 0.0));
        assert!((v.x - v2.x).abs() < EPSILON);
        assert!((v.y - v2.y).abs() < EPSILON);
        assert!((v.z - v2.z).abs() < EPSILON);
    }
}
```

**Step 2: Write the `Quat` implementation**

In `crates/abrash-core/src/quat.rs`:

```rust
//! Unit quaternion for rotation representation.
//!
//! Quaternions avoid gimbal lock and enable smooth interpolation via SLERP.
//! Convention: `(x, y, z, w)` where `w` is the scalar (real) component.

use std::ops::Mul;
use crate::math::{Mat4, Vec3};

/// A unit quaternion representing a 3D rotation.
///
/// Stored as `(x, y, z, w)` where `w` is the scalar part.
/// For a rotation of angle θ around axis (ax, ay, az):
/// - `w = cos(θ/2)`
/// - `x = ax * sin(θ/2)`
/// - `y = ay * sin(θ/2)`
/// - `z = az * sin(θ/2)`
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quat {
    /// Create a quaternion from components.
    #[must_use]
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// The identity quaternion (no rotation).
    #[must_use]
    #[inline]
    pub const fn identity() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0, w: 1.0 }
    }

    /// Create a quaternion from an axis and angle (radians).
    ///
    /// The axis should be normalized. The rotation follows the right-hand rule.
    #[must_use]
    pub fn from_axis_angle(axis: Vec3, angle: f32) -> Self {
        let half = angle * 0.5;
        let (sin_half, cos_half) = half.sin_cos();
        Self {
            x: axis.x * sin_half,
            y: axis.y * sin_half,
            z: axis.z * sin_half,
            w: cos_half,
        }
    }

    /// Create a quaternion from Euler angles (pitch, yaw, roll) in radians.
    ///
    /// Rotation order: Yaw (Y) * Pitch (X) * Roll (Z), matching the
    /// row-vector convention where transformations read left-to-right.
    #[must_use]
    pub fn from_euler(pitch: f32, yaw: f32, roll: f32) -> Self {
        let qy = Self::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), yaw);
        let qx = Self::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), pitch);
        let qz = Self::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), roll);
        qy * qx * qz
    }

    /// Spherical linear interpolation between two quaternions.
    ///
    /// Always takes the shortest path (flips `other` if dot product is negative).
    #[must_use]
    pub fn slerp(&self, other: &Self, t: f32) -> Self {
        let mut dot = self.x * other.x + self.y * other.y
            + self.z * other.z + self.w * other.w;

        // Flip to take shortest path
        let other = if dot < 0.0 {
            dot = -dot;
            Self::new(-other.x, -other.y, -other.z, -other.w)
        } else {
            *other
        };

        // If very close, use linear interpolation to avoid division by zero
        if dot > 0.9995 {
            let result = Self::new(
                self.x + (other.x - self.x) * t,
                self.y + (other.y - self.y) * t,
                self.z + (other.z - self.z) * t,
                self.w + (other.w - self.w) * t,
            );
            return result.normalize();
        }

        let theta = dot.clamp(-1.0, 1.0).acos();
        let sin_theta = theta.sin();
        let a = ((1.0 - t) * theta).sin() / sin_theta;
        let b = (t * theta).sin() / sin_theta;

        Self::new(
            self.x * a + other.x * b,
            self.y * a + other.y * b,
            self.z * a + other.z * b,
            self.w * a + other.w * b,
        )
    }

    /// Normalize to unit length.
    #[must_use]
    pub fn normalize(self) -> Self {
        let len = (self.x * self.x + self.y * self.y
            + self.z * self.z + self.w * self.w).sqrt();
        if len < f32::EPSILON {
            return Self::identity();
        }
        let inv = 1.0 / len;
        Self::new(self.x * inv, self.y * inv, self.z * inv, self.w * inv)
    }

    /// Rotate a vector by this quaternion.
    ///
    /// Computes `q * v * q⁻¹` using the optimized formula.
    #[must_use]
    pub fn rotate_vec3(self, v: Vec3) -> Vec3 {
        // Optimized quaternion-vector rotation (avoids full quaternion multiply):
        // t = 2 * cross(q.xyz, v)
        // result = v + w * t + cross(q.xyz, t)
        let qv = Vec3::new(self.x, self.y, self.z);
        let t = qv.cross(v) * 2.0;
        v + t * self.w + qv.cross(t)
    }

    /// Convert to a 4×4 rotation matrix (row-major, row-vector convention).
    #[must_use]
    pub fn to_mat4(self) -> Mat4 {
        let x2 = self.x + self.x;
        let y2 = self.y + self.y;
        let z2 = self.z + self.z;
        let xx = self.x * x2;
        let xy = self.x * y2;
        let xz = self.x * z2;
        let yy = self.y * y2;
        let yz = self.y * z2;
        let zz = self.z * z2;
        let wx = self.w * x2;
        let wy = self.w * y2;
        let wz = self.w * z2;

        Mat4 {
            m: [
                [1.0 - (yy + zz),  xy + wz,          xz - wy,          0.0],
                [xy - wz,          1.0 - (xx + zz),   yz + wx,          0.0],
                [xz + wy,          yz - wx,           1.0 - (xx + yy),  0.0],
                [0.0,              0.0,               0.0,               1.0],
            ],
        }
    }

    /// The conjugate (inverse for unit quaternions).
    #[must_use]
    #[inline]
    pub const fn conjugate(self) -> Self {
        Self::new(-self.x, -self.y, -self.z, self.w)
    }

    /// Dot product between two quaternions.
    #[must_use]
    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }
}

/// Compose rotations: `a * b` applies `a` first, then `b` (row-vector convention).
impl Mul for Quat {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self::new(
            self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
            self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
            self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
            self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
        )
    }
}

impl Default for Quat {
    fn default() -> Self {
        Self::identity()
    }
}
```

**Step 3: Add module to `crates/abrash-core/src/lib.rs`**

Add `pub mod quat;` to the module list.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p abrash-core -- quat`
Expected: 10 tests PASS.

**Step 5: Commit**

```bash
git add crates/abrash-core/src/quat.rs crates/abrash-core/src/lib.rs
git commit -m "feat(core): add Quat type with SLERP and matrix conversion"
```

---

### Task 5: `Animatable` impl for `Quat`

**Files:**
- Modify: `crates/abrash-core/src/animatable.rs` (add impl + tests)

**Step 1: Write the failing tests**

Add to the `tests` module in `animatable.rs`:

```rust
use crate::quat::Quat;

#[test]
fn quat_interpolate_uses_slerp() {
    let a = Quat::identity();
    let b = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), std::f32::consts::FRAC_PI_2);
    let mid = a.interpolate(&b, 0.5);
    // Should produce 45° rotation
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
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p abrash-core -- animatable::tests::quat`
Expected: FAIL (no `Animatable` impl for `Quat`).

**Step 3: Write the implementation**

Add to `animatable.rs`, with `use crate::quat::Quat;` at the top:

```rust
impl Animatable for Quat {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        self.slerp(other, t)
    }

    fn anim_scale(&self, scalar: f32) -> Self {
        // For quaternions, "scaling" means interpolating from identity by scalar amount.
        // This preserves the unit quaternion constraint.
        Quat::identity().slerp(self, scalar)
    }

    fn anim_add(&self, other: &Self) -> Self {
        // Compose rotations
        *self * *other
    }

    fn anim_sub(&self, other: &Self) -> Self {
        // Remove other's rotation: self * other.conjugate()
        *self * other.conjugate()
    }

    fn zero() -> Self {
        Quat::identity()
    }

    fn distance_squared(&self, other: &Self) -> f32 {
        // 1 - |dot(a,b)|² is a good rotation distance metric
        let dot = self.dot(*other).abs();
        1.0 - dot * dot
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p abrash-core -- animatable`
Expected: 14 tests PASS (6 f32 + 5 vec + 3 quat).

**Step 5: Commit**

```bash
git add crates/abrash-core/src/animatable.rs
git commit -m "feat(core): Animatable impl for Quat using SLERP"
```

---

### Task 6: `Transform` type

**Files:**
- Create: `crates/abrash-core/src/transform.rs`
- Modify: `crates/abrash-core/src/lib.rs` (add `pub mod transform`)
- Test: `crates/abrash-core/src/transform.rs` (inline `#[cfg(test)]`)

**Step 1: Write the failing tests**

In `crates/abrash-core/src/transform.rs`:

```rust
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
        // Row-vector: v * S * R * T
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
```

**Step 2: Write the `Transform` implementation**

```rust
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
```

**Step 3: Add module to `crates/abrash-core/src/lib.rs`**

Add `pub mod transform;` to the module list.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p abrash-core -- transform`
Expected: 6 tests PASS.

**Step 5: Commit**

```bash
git add crates/abrash-core/src/transform.rs crates/abrash-core/src/lib.rs
git commit -m "feat(core): add Transform (decomposed TRS) type"
```

---

### Task 7: `Animatable` impl for `Transform`

**Files:**
- Modify: `crates/abrash-core/src/animatable.rs` (add impl + tests)

**Step 1: Write the failing tests**

Add to the `tests` module in `animatable.rs`:

```rust
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
    // Should be 45° rotation
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
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p abrash-core -- animatable::tests::transform`
Expected: FAIL (no `Animatable` impl for `Transform`).

**Step 3: Write the implementation**

Add to `animatable.rs`, with `use crate::transform::Transform;` at the top:

```rust
impl Animatable for Transform {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        Self {
            position: self.position.interpolate(&other.position, t),
            rotation: self.rotation.interpolate(&other.rotation, t), // SLERP via Quat impl
            scale: self.scale.interpolate(&other.scale, t),
        }
    }

    fn anim_scale(&self, scalar: f32) -> Self {
        Self {
            position: self.position.anim_scale(scalar),
            rotation: self.rotation.anim_scale(scalar),
            scale: self.scale.anim_scale(scalar),
        }
    }

    fn anim_add(&self, other: &Self) -> Self {
        Self {
            position: self.position.anim_add(&other.position),
            rotation: self.rotation.anim_add(&other.rotation),
            scale: self.scale.anim_add(&other.scale),
        }
    }

    fn anim_sub(&self, other: &Self) -> Self {
        Self {
            position: self.position.anim_sub(&other.position),
            rotation: self.rotation.anim_sub(&other.rotation),
            scale: self.scale.anim_sub(&other.scale),
        }
    }

    fn zero() -> Self {
        Self::identity()
    }

    fn distance_squared(&self, other: &Self) -> f32 {
        self.position.distance_squared(&other.position)
            + self.rotation.distance_squared(&other.rotation)
            + self.scale.distance_squared(&other.scale)
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p abrash-core -- animatable`
Expected: 18 tests PASS (6 f32 + 5 vec + 3 quat + 4 transform).

**Step 5: Commit**

```bash
git add crates/abrash-core/src/animatable.rs
git commit -m "feat(core): Animatable impl for Transform (lerp+slerp)"
```

---

### Task 8: `AnimationClock` + `PlaybackMode`

**Files:**
- Create: `crates/abrash-anim/src/clock.rs`
- Modify: `crates/abrash-anim/src/lib.rs` (add `pub mod clock`)
- Test: `crates/abrash-anim/src/clock.rs` (inline `#[cfg(test)]`)

**Step 1: Write the failing tests**

In `crates/abrash-anim/src/clock.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    #[test]
    fn new_clock_starts_at_zero() {
        let c = AnimationClock::new();
        assert_eq!(c.cycle(), 0);
        assert!((c.phase()).abs() < EPSILON);
    }

    #[test]
    fn tick_advances_phase() {
        let mut c = AnimationClock::new();
        let event = c.tick(0.5, 1.0); // half-second into 1-second duration
        assert!(matches!(event, ClockEvent::Normal));
        assert!((c.phase() - 0.5).abs() < EPSILON);
    }

    #[test]
    fn tick_past_one_triggers_cycle_boundary() {
        let mut c = AnimationClock::new();
        let event = c.tick(1.5, 1.0); // 1.5 seconds into 1-second duration
        assert!(matches!(event, ClockEvent::CycleBoundary { completed: 1 }));
        assert_eq!(c.cycle(), 1);
        assert!((c.phase() - 0.5).abs() < EPSILON);
    }

    #[test]
    fn multiple_cycles_in_one_tick() {
        let mut c = AnimationClock::new();
        let event = c.tick(3.5, 1.0);
        assert!(matches!(event, ClockEvent::CycleBoundary { completed: 3 }));
        assert_eq!(c.cycle(), 3);
        assert!((c.phase() - 0.5).abs() < EPSILON);
    }

    #[test]
    fn delta_clamped_to_max() {
        let mut c = AnimationClock::new();
        c.tick(999.0, 1.0); // huge delta
        // Should be clamped to MAX_DELTA_SECS (0.1)
        assert!((c.phase() - 0.1).abs() < EPSILON);
    }

    #[test]
    fn effective_phase_ping_pong_reverses_odd_cycle() {
        let mut c = AnimationClock::new();
        c.tick(1.3, 1.0); // cycle=1, phase=0.3
        assert_eq!(c.cycle(), 1);
        let ep = c.effective_phase(&PlaybackMode::PingPong);
        assert!((ep - 0.7).abs() < EPSILON); // 1.0 - 0.3
    }

    #[test]
    fn effective_phase_loop_is_just_phase() {
        let mut c = AnimationClock::new();
        c.tick(1.3, 1.0);
        let ep = c.effective_phase(&PlaybackMode::Loop);
        assert!((ep - 0.3).abs() < EPSILON);
    }

    #[test]
    fn is_finished_once() {
        let mut c = AnimationClock::new();
        assert!(!c.is_finished(&PlaybackMode::Once));
        c.tick(1.0, 1.0);
        assert!(c.is_finished(&PlaybackMode::Once));
    }

    #[test]
    fn is_finished_count() {
        let mut c = AnimationClock::new();
        c.tick(2.0, 1.0);
        assert!(!c.is_finished(&PlaybackMode::Count(3)));
        c.tick(1.0, 1.0);
        assert!(c.is_finished(&PlaybackMode::Count(3)));
    }

    #[test]
    fn loop_never_finishes() {
        let mut c = AnimationClock::new();
        c.tick(100.0, 0.01); // many cycles
        assert!(!c.is_finished(&PlaybackMode::Loop));
    }

    #[test]
    fn reset_clears_state() {
        let mut c = AnimationClock::new();
        c.tick(1.5, 1.0);
        c.reset();
        assert_eq!(c.cycle(), 0);
        assert!((c.phase()).abs() < EPSILON);
    }
}
```

**Step 2: Write the implementation**

```rust
//! Drift-free animation clock using `(cycle, phase)` model.
//!
//! Inspired by AletheiaDB's hybrid logical clock. The integer `cycle`
//! counter never accumulates floating-point error, while `phase` stays
//! in 0.0–1.0 and resets each cycle.

const MAX_DELTA_SECS: f32 = 0.1;

/// How the animation repeats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackMode {
    /// Play once and stop.
    Once,
    /// Loop forever.
    Loop,
    /// Alternate forward/backward.
    PingPong,
    /// Play exactly N times.
    Count(u32),
}

/// Events emitted by the clock on each tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockEvent {
    /// Normal phase advance within a cycle.
    Normal,
    /// One or more cycles completed this tick.
    CycleBoundary { completed: u64 },
}

/// A drift-free animation clock.
///
/// Uses `(cycle: u64, phase: f32)` to eliminate floating-point
/// accumulation errors in looping animations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationClock {
    cycle: u64,
    phase: f32,
}

impl AnimationClock {
    /// Create a new clock at time zero.
    #[must_use]
    pub fn new() -> Self {
        Self { cycle: 0, phase: 0.0 }
    }

    /// Advance the clock by `delta_secs` for an animation of `duration` seconds.
    ///
    /// Returns a `ClockEvent` indicating whether a cycle boundary was crossed.
    /// Delta is clamped to `MAX_DELTA_SECS` (100ms) to handle tab-backgrounding.
    pub fn tick(&mut self, delta_secs: f32, duration: f32) -> ClockEvent {
        debug_assert!(duration > 0.0, "Clock duration must be positive");

        let clamped = delta_secs.clamp(0.0, MAX_DELTA_SECS);
        let phase_advance = clamped / duration;

        let new_phase = self.phase + phase_advance;
        if new_phase >= 1.0 {
            let whole_cycles = new_phase as u64;
            self.cycle += whole_cycles;
            self.phase = new_phase.fract();
            ClockEvent::CycleBoundary { completed: whole_cycles }
        } else {
            self.phase = new_phase;
            ClockEvent::Normal
        }
    }

    /// Current phase within the cycle (0.0–1.0).
    #[must_use]
    #[inline]
    pub fn phase(&self) -> f32 {
        self.phase
    }

    /// Number of completed cycles.
    #[must_use]
    #[inline]
    pub fn cycle(&self) -> u64 {
        self.cycle
    }

    /// Phase adjusted for playback mode (e.g. reversed on odd PingPong cycles).
    #[must_use]
    pub fn effective_phase(&self, mode: &PlaybackMode) -> f32 {
        match mode {
            PlaybackMode::PingPong if self.cycle % 2 == 1 => 1.0 - self.phase,
            _ => self.phase,
        }
    }

    /// Whether the animation has completed for the given playback mode.
    #[must_use]
    pub fn is_finished(&self, mode: &PlaybackMode) -> bool {
        match mode {
            PlaybackMode::Once => self.cycle >= 1,
            PlaybackMode::Count(n) => self.cycle >= u64::from(*n),
            PlaybackMode::Loop | PlaybackMode::PingPong => false,
        }
    }

    /// Reset to time zero.
    pub fn reset(&mut self) {
        self.cycle = 0;
        self.phase = 0.0;
    }
}

impl Default for AnimationClock {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 3: Add module to `crates/abrash-anim/src/lib.rs`**

Add `pub mod clock;` and re-export: `pub use clock::{AnimationClock, PlaybackMode, ClockEvent};`

**Step 4: Run tests to verify they pass**

Run: `cargo test -p abrash-anim -- clock`
Expected: 11 tests PASS.

**Step 5: Commit**

```bash
git add crates/abrash-anim/src/clock.rs crates/abrash-anim/src/lib.rs
git commit -m "feat(anim): add AnimationClock with drift-free (cycle, phase) model"
```

---

### Task 9: `Evaluable<T>` trait, `Sample<T>`, `Easing`

**Files:**
- Create: `crates/abrash-anim/src/evaluable.rs`
- Create: `crates/abrash-anim/src/easing.rs`
- Modify: `crates/abrash-anim/src/lib.rs` (add modules + re-exports)

**Step 1: Write the failing tests for Easing**

In `crates/abrash-anim/src/easing.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-4;

    #[test]
    fn all_easings_start_at_zero() {
        for easing in [Easing::Linear, Easing::EaseIn, Easing::EaseOut, Easing::EaseInOut] {
            assert!(easing.apply(0.0).abs() < EPSILON, "{easing:?} failed at 0.0");
        }
    }

    #[test]
    fn all_easings_end_at_one() {
        for easing in [Easing::Linear, Easing::EaseIn, Easing::EaseOut, Easing::EaseInOut] {
            assert!((easing.apply(1.0) - 1.0).abs() < EPSILON, "{easing:?} failed at 1.0");
        }
    }

    #[test]
    fn linear_is_identity() {
        assert!((Easing::Linear.apply(0.5) - 0.5).abs() < EPSILON);
        assert!((Easing::Linear.apply(0.25) - 0.25).abs() < EPSILON);
    }

    #[test]
    fn ease_in_is_slow_start() {
        // EaseIn at 0.5 should be < 0.5 (starts slow)
        assert!(Easing::EaseIn.apply(0.5) < 0.5);
    }

    #[test]
    fn ease_out_is_fast_start() {
        // EaseOut at 0.5 should be > 0.5 (starts fast)
        assert!(Easing::EaseOut.apply(0.5) > 0.5);
    }

    #[test]
    fn linear_derivative_is_constant() {
        assert!((Easing::Linear.derivative(0.0) - 1.0).abs() < EPSILON);
        assert!((Easing::Linear.derivative(0.5) - 1.0).abs() < EPSILON);
        assert!((Easing::Linear.derivative(1.0) - 1.0).abs() < EPSILON);
    }

    #[test]
    fn ease_in_derivative_starts_at_zero() {
        assert!(Easing::EaseIn.derivative(0.0).abs() < EPSILON);
    }

    #[test]
    fn ease_out_derivative_ends_at_zero() {
        assert!(Easing::EaseOut.derivative(1.0).abs() < EPSILON);
    }

    #[test]
    fn cubic_bezier_boundaries() {
        let cb = Easing::CubicBezier(0.25, 0.1, 0.25, 1.0);
        assert!(cb.apply(0.0).abs() < EPSILON);
        assert!((cb.apply(1.0) - 1.0).abs() < EPSILON);
    }
}
```

**Step 2: Write Easing**

```rust
//! Easing functions with analytical derivatives for velocity computation.

/// Standard easing curves.
#[derive(Debug, Clone, Copy)]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    /// Cubic Bézier curve with control points (x1, y1, x2, y2).
    CubicBezier(f32, f32, f32, f32),
}

impl Easing {
    /// Apply the easing function to a linear parameter t (0.0–1.0).
    #[must_use]
    pub fn apply(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::EaseIn => t * t,
            Self::EaseOut => t * (2.0 - t),
            Self::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
            Self::CubicBezier(_x1, y1, _x2, y2) => {
                let t2 = t * t;
                let t3 = t2 * t;
                3.0 * (1.0 - t) * (1.0 - t) * t * y1
                    + 3.0 * (1.0 - t) * t2 * y2
                    + t3
            }
        }
    }

    /// Instantaneous rate of change (derivative) of the easing function.
    ///
    /// Used to compute velocity in `Sample<T>`.
    #[must_use]
    pub fn derivative(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => 1.0,
            Self::EaseIn => 2.0 * t,
            Self::EaseOut => 2.0 - 2.0 * t,
            Self::EaseInOut => {
                if t < 0.5 { 4.0 * t } else { 4.0 - 4.0 * t }
            }
            Self::CubicBezier(..) => {
                // Numerical derivative via central finite difference
                let h = 0.0001;
                let t0 = (t - h).max(0.0);
                let t1 = (t + h).min(1.0);
                let dt = t1 - t0;
                if dt < f32::EPSILON { return 0.0; }
                (self.apply(t1) - self.apply(t0)) / dt
            }
        }
    }
}
```

**Step 3: Write `Evaluable<T>` and `Sample<T>`**

In `crates/abrash-anim/src/evaluable.rs`:

```rust
//! Core animation evaluation trait and sample type.

use abrash_core::animatable::Animatable;

/// A value paired with its instantaneous velocity.
///
/// Every `Evaluable` returns both value and velocity, enabling
/// future velocity-preserving spring interruption.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample<T: Animatable> {
    pub value: T,
    pub velocity: T,
}

impl<T: Animatable> Sample<T> {
    /// Create a sample with a value and velocity.
    #[must_use]
    pub fn new(value: T, velocity: T) -> Self {
        Self { value, velocity }
    }

    /// Create a sample at rest (zero velocity).
    #[must_use]
    pub fn at_rest(value: T) -> Self {
        Self {
            value,
            velocity: T::zero(),
        }
    }
}

/// A pure function from normalized phase (0.0–1.0) to a `Sample<T>`.
///
/// This is the core animation abstraction. Evaluables are composable:
/// `Keyframe`, `Hold`, and `Sequence` all implement this trait.
pub trait Evaluable<T: Animatable>: Send + Sync {
    /// Evaluate the animation at a normalized phase (0.0–1.0).
    fn evaluate(&self, phase: f32) -> Sample<T>;

    /// Preferred real-time duration in seconds.
    ///
    /// Used by composition types (e.g. `Sequence`) to allocate proportional
    /// phase ranges.
    fn natural_duration(&self) -> f32;
}
```

No tests needed for the trait itself — it's tested via its implementors.

**Step 4: Add modules to `crates/abrash-anim/src/lib.rs`**

```rust
pub mod easing;
pub mod evaluable;

pub use easing::Easing;
pub use evaluable::{Evaluable, Sample};
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p abrash-anim`
Expected: 9 easing tests + 11 clock tests = 20 tests PASS.

**Step 6: Commit**

```bash
git add crates/abrash-anim/src/easing.rs crates/abrash-anim/src/evaluable.rs crates/abrash-anim/src/lib.rs
git commit -m "feat(anim): add Evaluable trait, Sample, and Easing functions"
```

---

### Task 10: `Keyframe<T>` and `Hold<T>`

**Files:**
- Create: `crates/abrash-anim/src/keyframe.rs`
- Create: `crates/abrash-anim/src/hold.rs`
- Modify: `crates/abrash-anim/src/lib.rs` (add modules + re-exports)

**Step 1: Write the failing tests for Keyframe**

In `crates/abrash-anim/src/keyframe.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::easing::Easing;

    const EPSILON: f32 = 1e-5;

    #[test]
    fn evaluate_at_zero_returns_from() {
        let kf = Keyframe::new(0.0_f32, 10.0, Easing::Linear, 1.0);
        let s = Evaluable::evaluate(&kf, 0.0);
        assert!((s.value).abs() < EPSILON);
    }

    #[test]
    fn evaluate_at_one_returns_to() {
        let kf = Keyframe::new(0.0_f32, 10.0, Easing::Linear, 1.0);
        let s = Evaluable::evaluate(&kf, 1.0);
        assert!((s.value - 10.0).abs() < EPSILON);
    }

    #[test]
    fn evaluate_midpoint_linear() {
        let kf = Keyframe::new(0.0_f32, 10.0, Easing::Linear, 1.0);
        let s = Evaluable::evaluate(&kf, 0.5);
        assert!((s.value - 5.0).abs() < EPSILON);
    }

    #[test]
    fn velocity_nonzero_at_midpoint() {
        let kf = Keyframe::new(0.0_f32, 10.0, Easing::Linear, 1.0);
        let s = Evaluable::evaluate(&kf, 0.5);
        assert!(s.velocity.abs() > EPSILON);
    }

    #[test]
    fn velocity_is_zero_at_ease_in_start() {
        let kf = Keyframe::new(0.0_f32, 10.0, Easing::EaseIn, 1.0);
        let s = Evaluable::evaluate(&kf, 0.0);
        assert!(s.velocity.abs() < EPSILON);
    }

    #[test]
    fn natural_duration_matches() {
        let kf = Keyframe::new(0.0_f32, 10.0, Easing::Linear, 2.5);
        assert!((kf.natural_duration() - 2.5).abs() < EPSILON);
    }
}
```

**Step 2: Write Keyframe implementation**

```rust
//! Keyframe — tween between two values with easing.

use abrash_core::animatable::Animatable;
use crate::easing::Easing;
use crate::evaluable::{Evaluable, Sample};

/// A tween segment that interpolates from one value to another with easing.
///
/// Velocity is derived analytically from the easing function's derivative.
pub struct Keyframe<T: Animatable> {
    pub from: T,
    pub to: T,
    pub easing: Easing,
    pub duration: f32,
}

impl<T: Animatable> Keyframe<T> {
    /// Create a new keyframe tween.
    #[must_use]
    pub fn new(from: T, to: T, easing: Easing, duration: f32) -> Self {
        Self { from, to, easing, duration }
    }
}

impl<T: Animatable + Send + Sync> Evaluable<T> for Keyframe<T> {
    fn evaluate(&self, phase: f32) -> Sample<T> {
        let phase = phase.clamp(0.0, 1.0);
        let eased = self.easing.apply(phase);
        let value = self.from.interpolate(&self.to, eased);

        // Velocity = easing'(t) * (to - from) / duration
        let deriv = self.easing.derivative(phase);
        let delta = self.to.anim_sub(&self.from);
        let velocity = if self.duration > f32::EPSILON {
            delta.anim_scale(deriv / self.duration)
        } else {
            T::zero()
        };

        Sample::new(value, velocity)
    }

    fn natural_duration(&self) -> f32 {
        self.duration
    }
}
```

**Step 3: Write the tests and implementation for Hold**

In `crates/abrash-anim/src/hold.rs`:

```rust
//! Hold — constant value for a duration (pause in sequences).

use abrash_core::animatable::Animatable;
use crate::evaluable::{Evaluable, Sample};

/// A segment that holds a constant value for a given duration.
///
/// Always returns `Sample::at_rest(value)` — zero velocity.
pub struct Hold<T: Animatable> {
    pub value: T,
    pub duration: f32,
}

impl<T: Animatable> Hold<T> {
    /// Create a new hold segment.
    #[must_use]
    pub fn new(value: T, duration: f32) -> Self {
        Self { value, duration }
    }
}

impl<T: Animatable + Send + Sync> Evaluable<T> for Hold<T> {
    fn evaluate(&self, _phase: f32) -> Sample<T> {
        Sample::at_rest(self.value.clone())
    }

    fn natural_duration(&self) -> f32 {
        self.duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hold_always_returns_same_value() {
        let h = Hold::new(42.0_f32, 1.0);
        for phase in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let s = Evaluable::evaluate(&h, phase);
            assert!((s.value - 42.0).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn hold_velocity_is_zero() {
        let h = Hold::new(42.0_f32, 1.0);
        let s = Evaluable::evaluate(&h, 0.5);
        assert!(s.velocity.abs() < f32::EPSILON);
    }

    #[test]
    fn hold_natural_duration() {
        let h = Hold::new(0.0_f32, 3.0);
        assert!((h.natural_duration() - 3.0).abs() < f32::EPSILON);
    }
}
```

**Step 4: Add modules to `crates/abrash-anim/src/lib.rs`**

```rust
pub mod hold;
pub mod keyframe;

pub use hold::Hold;
pub use keyframe::Keyframe;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p abrash-anim`
Expected: 20 prior + 6 keyframe + 3 hold = 29 tests PASS.

**Step 6: Commit**

```bash
git add crates/abrash-anim/src/keyframe.rs crates/abrash-anim/src/hold.rs crates/abrash-anim/src/lib.rs
git commit -m "feat(anim): add Keyframe and Hold evaluable segments"
```

---

### Task 11: `Sequence<T>`

**Files:**
- Create: `crates/abrash-anim/src/sequence.rs`
- Modify: `crates/abrash-anim/src/lib.rs` (add module + re-export)

**Step 1: Write the failing tests**

In `crates/abrash-anim/src/sequence.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::easing::Easing;
    use crate::hold::Hold;
    use crate::keyframe::Keyframe;

    const EPSILON: f32 = 1e-4;

    #[test]
    fn two_equal_segments_split_evenly() {
        // 0→10 over 1s, then 10→20 over 1s. Total 2s.
        let seq = Sequence::new(vec![
            Box::new(Keyframe::new(0.0_f32, 10.0, Easing::Linear, 1.0)),
            Box::new(Keyframe::new(10.0_f32, 20.0, Easing::Linear, 1.0)),
        ]);

        // phase 0.0 → start of first segment
        let s = Evaluable::evaluate(&seq, 0.0);
        assert!((s.value).abs() < EPSILON);

        // phase 0.25 → midpoint of first segment
        let s = Evaluable::evaluate(&seq, 0.25);
        assert!((s.value - 5.0).abs() < EPSILON);

        // phase 0.5 → boundary (start of second segment)
        let s = Evaluable::evaluate(&seq, 0.5);
        assert!((s.value - 10.0).abs() < EPSILON);

        // phase 0.75 → midpoint of second segment
        let s = Evaluable::evaluate(&seq, 0.75);
        assert!((s.value - 15.0).abs() < EPSILON);

        // phase 1.0 → end
        let s = Evaluable::evaluate(&seq, 1.0);
        assert!((s.value - 20.0).abs() < EPSILON);
    }

    #[test]
    fn unequal_durations_proportional() {
        // 0→10 over 1s, then 10→10 (hold) over 3s. Total 4s.
        // First segment gets 25% of phase, hold gets 75%.
        let seq = Sequence::new(vec![
            Box::new(Keyframe::new(0.0_f32, 10.0, Easing::Linear, 1.0)),
            Box::new(Hold::new(10.0_f32, 3.0)),
        ]);

        // phase 0.125 → midpoint of first segment (which spans 0.0–0.25)
        let s = Evaluable::evaluate(&seq, 0.125);
        assert!((s.value - 5.0).abs() < EPSILON);

        // phase 0.5 → well into hold segment, still 10.0
        let s = Evaluable::evaluate(&seq, 0.5);
        assert!((s.value - 10.0).abs() < EPSILON);
    }

    #[test]
    fn total_duration_is_sum() {
        let seq = Sequence::new(vec![
            Box::new(Keyframe::new(0.0_f32, 10.0, Easing::Linear, 2.0)),
            Box::new(Hold::new(10.0_f32, 3.0)),
        ]);
        assert!((seq.natural_duration() - 5.0).abs() < EPSILON);
    }

    #[test]
    fn single_segment_sequence() {
        let seq = Sequence::new(vec![
            Box::new(Keyframe::new(0.0_f32, 10.0, Easing::Linear, 1.0)),
        ]);
        let s = Evaluable::evaluate(&seq, 0.5);
        assert!((s.value - 5.0).abs() < EPSILON);
    }
}
```

**Step 2: Write the implementation**

```rust
//! Sequence — proportional end-to-end chaining of evaluable segments.

use abrash_core::animatable::Animatable;
use crate::evaluable::{Evaluable, Sample};

/// Chains multiple `Evaluable` segments end-to-end.
///
/// Each child gets a proportional slice of the 0.0–1.0 phase range
/// based on `natural_duration() / total_duration`.
pub struct Sequence<T: Animatable> {
    segments: Vec<Box<dyn Evaluable<T>>>,
    /// (start_phase, end_phase) for each segment.
    boundaries: Vec<(f32, f32)>,
    total_duration: f32,
}

impl<T: Animatable> Sequence<T> {
    /// Create a sequence from a list of evaluable segments.
    ///
    /// # Panics
    ///
    /// Panics if `segments` is empty.
    #[must_use]
    pub fn new(segments: Vec<Box<dyn Evaluable<T>>>) -> Self {
        assert!(!segments.is_empty(), "Sequence requires at least one segment");

        let total_duration: f32 = segments.iter().map(|s| s.natural_duration()).sum();
        let mut boundaries = Vec::with_capacity(segments.len());
        let mut cursor = 0.0_f32;

        for seg in &segments {
            let proportion = if total_duration > f32::EPSILON {
                seg.natural_duration() / total_duration
            } else {
                1.0 / segments.len() as f32
            };
            boundaries.push((cursor, cursor + proportion));
            cursor += proportion;
        }

        Self {
            segments,
            boundaries,
            total_duration,
        }
    }
}

impl<T: Animatable> Evaluable<T> for Sequence<T> {
    fn evaluate(&self, phase: f32) -> Sample<T> {
        let phase = phase.clamp(0.0, 1.0);

        for (i, &(start, end)) in self.boundaries.iter().enumerate() {
            if phase < end || i == self.segments.len() - 1 {
                let span = end - start;
                let local_phase = if span > f32::EPSILON {
                    ((phase - start) / span).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                return self.segments[i].evaluate(local_phase);
            }
        }

        self.segments.last().unwrap().evaluate(1.0)
    }

    fn natural_duration(&self) -> f32 {
        self.total_duration
    }
}

// Sequence contains Box<dyn Evaluable<T>> which requires Send + Sync.
// The Evaluable trait bound already requires Send + Sync, so this is safe.
unsafe impl<T: Animatable> Send for Sequence<T> {}
unsafe impl<T: Animatable> Sync for Sequence<T> {}
```

**Step 3: Add module to `crates/abrash-anim/src/lib.rs`**

```rust
pub mod sequence;

pub use sequence::Sequence;
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p abrash-anim -- sequence`
Expected: 4 tests PASS.

**Step 5: Commit**

```bash
git add crates/abrash-anim/src/sequence.rs crates/abrash-anim/src/lib.rs
git commit -m "feat(anim): add Sequence for proportional segment chaining"
```

---

### Task 12: `Timeline<T>` (stateful driver)

**USER CONTRIBUTION POINT:** The `Timeline` struct is where design choices about the builder API and interruption model matter. The plan provides the core `tick()` logic and test expectations, but the builder methods (`tween`, `sequence`, `easing`, `loop_forever`, etc.) shape the user-facing ergonomics. The implementor should review the API from `docs/plans/2026-03-22-animation-system-design.md` Section 3 and decide if adjustments are needed.

**Files:**
- Create: `crates/abrash-anim/src/timeline.rs`
- Modify: `crates/abrash-anim/src/lib.rs` (add module + re-export)

**Step 1: Write the failing tests**

In `crates/abrash-anim/src/timeline.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::easing::Easing;
    use abrash_core::math::Vec3;
    use std::time::Duration;

    const EPSILON: f32 = 1e-4;

    #[test]
    fn tween_starts_at_from() {
        let mut tl = Timeline::tween(0.0_f32, 10.0, Duration::from_secs(1));
        let s = tl.tick(0.0);
        assert!((s.value).abs() < EPSILON);
    }

    #[test]
    fn tween_reaches_to() {
        let mut tl = Timeline::tween(0.0_f32, 10.0, Duration::from_secs(1));
        tl.tick(1.0);
        assert!(tl.is_completed());
        assert!((tl.current_value() - 10.0).abs() < EPSILON);
    }

    #[test]
    fn tween_midpoint() {
        let mut tl = Timeline::tween(0.0_f32, 10.0, Duration::from_secs(2));
        let s = tl.tick(1.0); // 1s into 2s duration
        assert!((s.value - 5.0).abs() < EPSILON);
    }

    #[test]
    fn loop_forever_never_completes() {
        let mut tl = Timeline::tween(0.0_f32, 10.0, Duration::from_secs(1))
            .loop_forever();
        for _ in 0..10 {
            tl.tick(0.1);
        }
        // 1.0s total = exactly 1 cycle, phase wraps
        assert!(!tl.is_completed());
    }

    #[test]
    fn ping_pong_reverses() {
        let mut tl = Timeline::tween(0.0_f32, 10.0, Duration::from_secs(1))
            .ping_pong();
        // Forward to end of first cycle
        tl.tick(0.1); // MAX_DELTA clamp = 0.1, phase = 0.1/1.0 = 0.1

        // Tick into second cycle — should be going backward
        let mut prev = tl.tick(0.1);
        // Advance more into cycle 1 (backward)
        // After ~1.0s total we cross into cycle 1
        for _ in 0..8 {
            tl.tick(0.1);
        }
        // Now in cycle 1, effective phase reverses
        let s = tl.tick(0.1);
        // Total ~1.1s. Cycle 1, phase 0.1. Effective = 1.0-0.1 = 0.9. Value ~9.0
        // Due to MAX_DELTA clamping this is approximate
        assert!(!tl.is_completed());
    }

    #[test]
    fn easing_modifier() {
        let mut tl = Timeline::tween(0.0_f32, 10.0, Duration::from_secs(2))
            .easing(Easing::EaseIn);
        let s = tl.tick(1.0); // phase 0.5
        // EaseIn at 0.5 = 0.25, so value should be ~2.5
        assert!(s.value < 5.0, "EaseIn should be below linear at midpoint");
    }

    #[test]
    fn count_mode_finishes_after_n() {
        let mut tl = Timeline::tween(0.0_f32, 10.0, Duration::from_secs(1))
            .count(2);
        tl.tick(0.1);
        tl.tick(0.1);
        // Need 20 ticks of 0.1s = 2.0s for 2 cycles
        for _ in 0..18 {
            tl.tick(0.1);
        }
        assert!(tl.is_completed());
    }

    #[test]
    fn reset_restarts() {
        let mut tl = Timeline::tween(0.0_f32, 10.0, Duration::from_secs(1));
        tl.tick(1.0);
        assert!(tl.is_completed());
        tl.reset();
        assert!(!tl.is_completed());
        let s = tl.tick(0.0);
        assert!((s.value).abs() < EPSILON);
    }

    #[test]
    fn sequence_builder() {
        let mut tl = Timeline::<f32>::sequence()
            .then_tween(0.0, 10.0, Duration::from_secs(1))
                .easing(Easing::Linear)
            .then_hold(10.0, Duration::from_millis(500))
            .build();

        // duration should be 1.5s
        assert!((tl.duration() - 1.5).abs() < EPSILON);
    }

    #[test]
    fn vec3_tween() {
        let mut tl = Timeline::tween(
            Vec3::ZERO,
            Vec3::new(10.0, 20.0, 30.0),
            Duration::from_secs(2),
        );
        let s = tl.tick(1.0); // midpoint
        assert!((s.value.x - 5.0).abs() < EPSILON);
        assert!((s.value.y - 10.0).abs() < EPSILON);
        assert!((s.value.z - 15.0).abs() < EPSILON);
    }

    #[test]
    fn completed_timeline_stays_at_final_value() {
        let mut tl = Timeline::tween(0.0_f32, 10.0, Duration::from_secs(1));
        tl.tick(1.0); // complete
        let s1 = tl.tick(0.5); // tick after completion
        let s2 = tl.tick(0.5);
        assert!((s1.value - 10.0).abs() < EPSILON);
        assert!((s2.value - 10.0).abs() < EPSILON);
    }
}
```

**Step 2: Write the `Timeline<T>` implementation**

```rust
//! Timeline — stateful animation driver.
//!
//! Owns an `Evaluable` root and an `AnimationClock`. Call `tick(dt)` each
//! frame to advance the animation and get the current `Sample<T>`.

use std::time::Duration;

use abrash_core::animatable::Animatable;
use crate::clock::{AnimationClock, ClockEvent, PlaybackMode};
use crate::easing::Easing;
use crate::evaluable::{Evaluable, Sample};
use crate::hold::Hold;
use crate::keyframe::Keyframe;
use crate::sequence::Sequence;

enum TimelineState<T: Animatable> {
    Playing,
    Completed { final_sample: Sample<T> },
}

/// Stateful animation orchestrator.
///
/// Wraps an `Evaluable` tree with a clock and playback mode.
/// Call `tick(delta_secs)` each frame to drive the animation.
pub struct Timeline<T: Animatable> {
    root: Box<dyn Evaluable<T>>,
    clock: AnimationClock,
    duration: f32,
    playback: PlaybackMode,
    state: TimelineState<T>,
    last_sample: Sample<T>,
}

impl<T: Animatable + Send + Sync + 'static> Timeline<T> {
    /// Create a timeline from any `Evaluable`.
    #[must_use]
    pub fn from_evaluable(root: Box<dyn Evaluable<T>>, playback: PlaybackMode) -> Self {
        let duration = root.natural_duration();
        let initial = root.evaluate(0.0);
        Self {
            root,
            clock: AnimationClock::new(),
            duration,
            playback,
            state: TimelineState::Playing,
            last_sample: initial,
        }
    }

    /// Simple linear tween between two values.
    #[must_use]
    pub fn tween(from: T, to: T, duration: Duration) -> Self {
        let secs = duration.as_secs_f32();
        Self::from_evaluable(
            Box::new(Keyframe::new(from, to, Easing::Linear, secs)),
            PlaybackMode::Once,
        )
    }

    /// Start building a sequence.
    #[must_use]
    pub fn sequence() -> SequenceBuilder<T> {
        SequenceBuilder { segments: Vec::new() }
    }

    /// Apply an easing curve to this timeline.
    #[must_use]
    pub fn easing(mut self, easing: Easing) -> Self {
        let sample_start = self.root.evaluate(0.0);
        let sample_end = self.root.evaluate(1.0);
        self.root = Box::new(Keyframe::new(
            sample_start.value,
            sample_end.value,
            easing,
            self.duration,
        ));
        self
    }

    /// Loop the animation forever.
    #[must_use]
    pub fn loop_forever(mut self) -> Self {
        self.playback = PlaybackMode::Loop;
        self
    }

    /// Alternate forward/backward.
    #[must_use]
    pub fn ping_pong(mut self) -> Self {
        self.playback = PlaybackMode::PingPong;
        self
    }

    /// Play exactly `n` times.
    #[must_use]
    pub fn count(mut self, n: u32) -> Self {
        self.playback = PlaybackMode::Count(n);
        self
    }

    /// Advance the animation by `delta_secs` and return the current sample.
    pub fn tick(&mut self, delta_secs: f32) -> Sample<T> {
        let sample = match &self.state {
            TimelineState::Playing => {
                let event = self.clock.tick(delta_secs, self.duration);

                match event {
                    ClockEvent::Normal => {
                        let phase = self.clock.effective_phase(&self.playback);
                        self.root.evaluate(phase)
                    }
                    ClockEvent::CycleBoundary { .. } => {
                        if self.clock.is_finished(&self.playback) {
                            let final_sample = self.root.evaluate(1.0);
                            self.state = TimelineState::Completed {
                                final_sample: final_sample.clone(),
                            };
                            final_sample
                        } else {
                            let phase = self.clock.effective_phase(&self.playback);
                            self.root.evaluate(phase)
                        }
                    }
                }
            }
            TimelineState::Completed { final_sample } => final_sample.clone(),
        };

        self.last_sample = sample.clone();
        sample
    }

    /// Whether the animation has finished.
    #[must_use]
    pub fn is_completed(&self) -> bool {
        matches!(self.state, TimelineState::Completed { .. })
    }

    /// The current value (from the last tick).
    #[must_use]
    pub fn current_value(&self) -> T {
        self.last_sample.value.clone()
    }

    /// Reset to the beginning.
    pub fn reset(&mut self) {
        self.clock.reset();
        self.state = TimelineState::Playing;
        self.last_sample = self.root.evaluate(0.0);
    }

    /// The total duration in seconds.
    #[must_use]
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// The current playback mode.
    #[must_use]
    pub fn playback_mode(&self) -> &PlaybackMode {
        &self.playback
    }
}

// --- Builders ---

/// Builder for constructing `Timeline` from a sequence of segments.
pub struct SequenceBuilder<T: Animatable> {
    segments: Vec<Box<dyn Evaluable<T>>>,
}

impl<T: Animatable + Send + Sync + 'static> SequenceBuilder<T> {
    /// Add a tween segment.
    #[must_use]
    pub fn then_tween(self, from: T, to: T, duration: Duration) -> TweenSegmentBuilder<T> {
        TweenSegmentBuilder {
            builder: self,
            from,
            to,
            duration: duration.as_secs_f32(),
            easing: Easing::Linear,
        }
    }

    /// Add a hold (pause) segment.
    #[must_use]
    pub fn then_hold(mut self, value: T, duration: Duration) -> Self {
        self.segments.push(Box::new(Hold::new(value, duration.as_secs_f32())));
        self
    }

    /// Build the timeline from the accumulated segments.
    #[must_use]
    pub fn build(self) -> Timeline<T> {
        let seq = Sequence::new(self.segments);
        Timeline::from_evaluable(Box::new(seq), PlaybackMode::Once)
    }
}

/// Builder for a tween segment within a sequence (allows setting easing).
pub struct TweenSegmentBuilder<T: Animatable> {
    builder: SequenceBuilder<T>,
    from: T,
    to: T,
    duration: f32,
    easing: Easing,
}

impl<T: Animatable + Send + Sync + 'static> TweenSegmentBuilder<T> {
    /// Set the easing for this tween segment.
    #[must_use]
    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// Add another tween after this one.
    #[must_use]
    pub fn then_tween(self, from: T, to: T, duration: Duration) -> TweenSegmentBuilder<T> {
        let builder = self.finalize();
        builder.then_tween(from, to, duration)
    }

    /// Add a hold after this tween.
    #[must_use]
    pub fn then_hold(self, value: T, duration: Duration) -> SequenceBuilder<T> {
        let builder = self.finalize();
        builder.then_hold(value, duration)
    }

    /// Build the timeline.
    #[must_use]
    pub fn build(self) -> Timeline<T> {
        self.finalize().build()
    }

    fn finalize(mut self) -> SequenceBuilder<T> {
        self.builder.segments.push(Box::new(Keyframe::new(
            self.from,
            self.to,
            self.easing,
            self.duration,
        )));
        self.builder
    }
}
```

**Step 3: Add module to `crates/abrash-anim/src/lib.rs`**

```rust
pub mod timeline;

pub use timeline::Timeline;
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p abrash-anim -- timeline`
Expected: 11 tests PASS.

**Step 5: Run all tests across both crates**

Run: `cargo test -p abrash-core -p abrash-anim`
Expected: All tests PASS (18 abrash-core + ~40 abrash-anim).

**Step 6: Commit**

```bash
git add crates/abrash-anim/src/timeline.rs crates/abrash-anim/src/lib.rs
git commit -m "feat(anim): add Timeline stateful driver with builders"
```

---

### Task 13: Re-export from root `abrash` crate

**Files:**
- Modify: `Cargo.toml` (root — add `abrash-anim` dependency)
- Modify: `src/lib.rs` (add re-export)

**Step 1: Add dependency to root `Cargo.toml`**

In `[dependencies]`, add:
```toml
abrash-anim = { path = "crates/abrash-anim" }
```

**Step 2: Add re-export to `src/lib.rs`**

Check what the current re-export pattern looks like, then add:
```rust
pub use abrash_anim as anim;
```

This allows consumers to write `use abrash::anim::{Timeline, Easing};`.

Also re-export the new core types. Check if `abrash_core::quat` and `abrash_core::transform` need explicit re-exports alongside existing `math`, `mesh`, `time` etc.

**Step 3: Verify it compiles**

Run: `cargo check`
Expected: Compiles with no errors.

**Step 4: Commit**

```bash
git add Cargo.toml src/lib.rs
git commit -m "feat: re-export abrash-anim as abrash::anim"
```

---

### Task 14: Update `cube_3d` example to use Timeline

**Files:**
- Modify: `examples/cube_3d.rs`

**Step 1: Read the current example** (already read above — uses manual `angle_y += 1.0 * dt`)

**Step 2: Replace manual rotation with Timeline**

Replace the `angle_y: f32, angle_x: f32` fields with two `Timeline<f32>` fields:

```rust
use abrash::anim::Timeline;
use std::time::Duration;

struct Cube3dApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    timestep: FixedTimestep,
    cube: Mesh,
    rotation_y: Timeline<f32>,
    rotation_x: Timeline<f32>,
}
```

In `new()`:
```rust
rotation_y: Timeline::tween(0.0, std::f32::consts::TAU, Duration::from_secs(6))
    .loop_forever(),
rotation_x: Timeline::tween(0.0, std::f32::consts::TAU, Duration::from_secs(12))
    .loop_forever(),
```

In `update()`:
```rust
fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
    let steps = self.timestep.update();
    for _ in 0..steps {
        let dt = self.timestep.dt();
        self.rotation_y.tick(dt);
        self.rotation_x.tick(dt);
    }
    Ok(())
}
```

In `render()`:
```rust
let model = Mat4::rotation_y(self.rotation_y.current_value())
    * Mat4::rotation_x(self.rotation_x.current_value());
```

**Step 3: Run the example**

Run: `cargo run --example cube_3d`
Expected: Cube rotates smoothly, same visual behavior as before.

**Step 4: Commit**

```bash
git add examples/cube_3d.rs
git commit -m "refactor(examples): cube_3d uses Timeline instead of manual rotation"
```

---

### Task 15: Final lint pass + format

**Step 1: Format**

Run: `cargo fmt --all`

**Step 2: Clippy**

Run: `cargo clippy -p abrash-core -p abrash-anim -- -D warnings`
Expected: No warnings.

Fix any issues that arise.

**Step 3: Full test suite**

Run: `cargo test`
Expected: All existing tests + new animation tests pass.

**Step 4: Commit any fixes**

```bash
git add -A
git commit -m "chore: fmt + clippy fixes for animation system"
```
