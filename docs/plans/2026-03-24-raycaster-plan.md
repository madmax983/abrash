# Raycaster Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a raycaster to abrash — stateless DDA grid raycasting with BAM angle math, tiered query API, ECS-friendly batching, and hybrid rendering integration.

**Architecture:** Port `Bam` and `Fixed16_16` from doom-rs into `abrash-core` (with const table generation). Create new `abrash-raycast` crate with stateless free functions for LOS/ray/detailed queries using DDA grid stepping. Integrate into `abrash-render` for hybrid column rendering with z-buffer bridge to triangle rasterizer.

**Tech Stack:** Rust 2024 edition, `Fixed16_16` (16.16 fixed-point), `Bam` (u32 binary angle), DDA algorithm, optional rayon for batch parallelism, criterion for benchmarks, proptest for property tests.

**Design Doc:** `docs/plans/2026-03-24-raycaster-design.md`

---

## Phase 1: Core Math in `abrash-core`

### Task 1: Port `Fixed16_16` to `abrash-core`

**Files:**
- Create: `crates/abrash-core/src/fixed16_16.rs`
- Modify: `crates/abrash-core/src/lib.rs` (add `pub mod fixed16_16;`)

**Step 1: Write failing tests**

Create `crates/abrash-core/src/fixed16_16.rs` with tests only:

```rust
//! Fixed-point 16.16 arithmetic.
//!
//! Ported from doom-rs. The original engine used `typedef int fixed_t` with `FRACBITS = 16`.
//! This newtype enforces that distinction at the type level.

use core::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// Fixed-point 16.16 number: bits [31..16] = integer, bits [15..0] = fraction.
///
/// One unit = `1 << 16 = 65536`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fixed16_16(pub i32);

/// The fractional shift (FRACBITS in the original C source).
pub const FRAC_BITS: u32 = 16;

/// One unit in fixed-point: equivalent to the float `1.0`.
pub const FIXED_ONE: Fixed16_16 = Fixed16_16(1 << FRAC_BITS);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_int_roundtrip() {
        for n in -32768_i32..=32767 {
            assert_eq!(Fixed16_16::from_int(n).to_int(), n);
        }
    }

    #[test]
    fn fixed_one_is_unit() {
        assert_eq!(FIXED_ONE.to_int(), 1);
        assert_eq!(FIXED_ONE.fixed_mul(FIXED_ONE), FIXED_ONE);
    }

    #[test]
    fn fixed_mul_basic() {
        let a = Fixed16_16::from_int(3);
        let b = Fixed16_16::from_int(4);
        assert_eq!(a.fixed_mul(b), Fixed16_16::from_int(12));
    }

    #[test]
    fn fixed_mul_commutative() {
        let a = Fixed16_16::from_int(3);
        let b = Fixed16_16::from_int(7);
        assert_eq!(a.fixed_mul(b), b.fixed_mul(a));
    }

    #[test]
    fn fixed_div_basic() {
        let a = Fixed16_16::from_int(10);
        let b = Fixed16_16::from_int(2);
        assert_eq!(a.fixed_div(b), Fixed16_16::from_int(5));
    }

    #[test]
    fn to_f32_roundtrip() {
        let f = Fixed16_16::from_f32(3.5);
        assert!((f.to_f32() - 3.5).abs() < 0.001);
    }

    #[test]
    fn operators_consistent() {
        let a = Fixed16_16::from_int(10);
        let b = Fixed16_16::from_int(3);
        assert_eq!(a * b, a.fixed_mul(b));
        assert_eq!(a / b, a.fixed_div(b));
    }

    #[test]
    fn lerp_endpoints() {
        let a = Fixed16_16::from_int(10);
        let b = Fixed16_16::from_int(20);
        assert_eq!(a.lerp(b, Fixed16_16::ZERO), a);
        assert_eq!(a.lerp(b, FIXED_ONE), b);
    }

    #[test]
    fn lerp_midpoint() {
        let a = Fixed16_16::from_int(10);
        let b = Fixed16_16::from_int(20);
        let t_half = Fixed16_16(1 << 15);
        assert_eq!(a.lerp(b, t_half), Fixed16_16::from_int(15));
    }

    #[test]
    #[should_panic]
    fn fixed_div_by_zero_panics() {
        let _ = Fixed16_16::from_int(10).fixed_div(Fixed16_16::ZERO);
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    fn fixed_strategy() -> impl Strategy<Value = Fixed16_16> {
        (-32767_i32..=32767).prop_map(Fixed16_16::from_int)
    }

    proptest! {
        #[test]
        fn mul_commutative(a in fixed_strategy(), b in fixed_strategy()) {
            prop_assert_eq!(a.fixed_mul(b), b.fixed_mul(a));
        }

        #[test]
        fn add_sub_inverse(a in fixed_strategy(), b in fixed_strategy()) {
            prop_assert_eq!((a + b) - b, a);
        }

        #[test]
        fn mul_by_one_is_identity(a in fixed_strategy()) {
            prop_assert_eq!(a.fixed_mul(FIXED_ONE), a);
        }

        #[test]
        fn mul_by_zero_is_zero(a in fixed_strategy()) {
            prop_assert_eq!(a.fixed_mul(Fixed16_16::ZERO), Fixed16_16::ZERO);
        }
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p abrash-core fixed16_16 -- --nocapture`
Expected: FAIL — methods not implemented yet

**Step 3: Implement `Fixed16_16`**

Add the implementation above the tests in the same file. Port from `doom-rs/crates/doom-types/src/fixed.rs`:

```rust
impl Fixed16_16 {
    pub const ZERO: Self = Self(0);

    #[inline]
    pub const fn from_raw(raw: i32) -> Self { Self(raw) }

    #[inline]
    pub const fn from_int(n: i32) -> Self { Self(n << FRAC_BITS) }

    #[inline]
    pub const fn to_int(self) -> i32 { self.0 >> FRAC_BITS }

    #[inline]
    pub const fn raw(self) -> i32 { self.0 }

    #[inline]
    pub fn fixed_mul(self, rhs: Self) -> Self {
        let product = (self.0 as i64) * (rhs.0 as i64);
        Self((product >> FRAC_BITS) as i32)
    }

    #[inline]
    pub fn fixed_div(self, rhs: Self) -> Self {
        debug_assert!(rhs.0 != 0, "FixedDiv: division by zero");
        let numerator = (self.0 as i64) << FRAC_BITS;
        let result = numerator / rhs.0 as i64;
        Self(result.clamp(i32::MIN as i64, i32::MAX as i64) as i32)
    }

    #[inline]
    pub fn abs(self) -> Self { Self(self.0.wrapping_abs()) }

    #[inline]
    pub fn lerp(self, other: Self, t: Self) -> Self {
        self + (other - self).fixed_mul(t)
    }

    #[inline]
    #[must_use]
    pub fn to_f32(self) -> f32 { self.0 as f32 / (1 << FRAC_BITS) as f32 }

    #[inline]
    pub fn from_f32(v: f32) -> Self { Self((v * (1 << FRAC_BITS) as f32) as i32) }
}
```

Then the operator impls — `Add`, `AddAssign`, `Sub`, `SubAssign`, `Neg`, `Mul` (calls `fixed_mul`), `Div` (calls `fixed_div`), `From<i32>`, `From<Fixed16_16> for i32`, `Display`.

Port these verbatim from doom-rs — they are identical.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p abrash-core fixed16_16`
Expected: ALL PASS

**Step 5: Register module**

Add to `crates/abrash-core/src/lib.rs`:
```rust
pub mod fixed16_16;
```

**Step 6: Run full abrash-core test suite**

Run: `cargo test -p abrash-core`
Expected: ALL PASS (no regressions)

**Step 7: Commit**

```bash
git add crates/abrash-core/src/fixed16_16.rs crates/abrash-core/src/lib.rs
git commit -m "feat(core): add Fixed16_16 type ported from doom-rs"
```

---

### Task 2: Port `Bam` to `abrash-core` with const table

**Files:**
- Create: `crates/abrash-core/src/bam.rs`
- Modify: `crates/abrash-core/src/lib.rs` (add `pub mod bam;`)

**Step 1: Write failing tests**

Create `crates/abrash-core/src/bam.rs` with tests only:

```rust
//! Binary Angle Measure (BAM) — full-circle `u32` angle type with const sine table.
//!
//! The full circle is `u32::MAX + 1` (2³²). All arithmetic is naturally
//! modular via `u32` wrapping, so there is no "angle clamping" needed.
//!
//! Ported from doom-rs with the key improvement: the sine table is `const`
//! (compile-time generated) instead of `static mut` with runtime init.

use crate::fixed16_16::Fixed16_16;

/// Binary Angle Measure: 2³² = full circle.
///
/// Cardinal directions:
/// - `0x00000000` → East (0°)
/// - `0x40000000` → North (90°)
/// - `0x80000000` → West (180°)
/// - `0xC0000000` → South (270°)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bam(pub u32);

pub const ANG45: Bam = Bam(0x2000_0000);
pub const ANG90: Bam = Bam(0x4000_0000);
pub const ANG180: Bam = Bam(0x8000_0000);
pub const ANG270: Bam = Bam(0xC000_0000);

const FINE_TABLE_SIZE: usize = 8192;
pub const BAM_TO_FINE_SHIFT: u32 = 32 - 13;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ang90_plus_ang90_is_ang180() {
        assert_eq!(ANG90 + ANG90, ANG180);
    }

    #[test]
    fn full_circle_wraps_to_zero() {
        assert_eq!(ANG180 + ANG180, Bam::ZERO);
    }

    #[test]
    fn additive_inverse_holds() {
        let a = Bam(0x1234_5678);
        let b = Bam(0xABCD_EF01);
        assert_eq!((a + b) - b, a);
    }

    #[test]
    fn negate_is_additive_inverse() {
        let a = ANG90;
        assert_eq!(a + (-a), Bam::ZERO);
    }

    #[test]
    fn fine_angle_in_bounds() {
        for raw in [0u32, 0x1000_0000, 0x4000_0000, 0x8000_0000, 0xFFFF_FFFF] {
            let idx = Bam(raw).fine_angle();
            assert!(idx < FINE_TABLE_SIZE, "fine_angle {idx} out of bounds");
        }
    }

    #[test]
    fn sin_cos_at_cardinal_angles() {
        // sin(0°) = 0, cos(0°) = 1
        let (s, c) = Bam::ZERO.sin_cos_fixed();
        assert_eq!(s, Fixed16_16::ZERO);
        assert!((c.to_f32() - 1.0).abs() < 0.001);

        // sin(90°) = 1, cos(90°) = 0
        let (s, c) = ANG90.sin_cos_fixed();
        assert!((s.to_f32() - 1.0).abs() < 0.001);
        assert!(c.to_f32().abs() < 0.001);

        // sin(180°) = 0, cos(180°) = -1
        let (s, c) = ANG180.sin_cos_fixed();
        assert!(s.to_f32().abs() < 0.001);
        assert!((c.to_f32() + 1.0).abs() < 0.001);

        // sin(270°) = -1, cos(270°) = 0
        let (s, c) = ANG270.sin_cos_fixed();
        assert!((s.to_f32() + 1.0).abs() < 0.001);
        assert!(c.to_f32().abs() < 0.001);
    }

    #[test]
    fn sin_cos_f32_delegates_to_polynomial() {
        // sin(45°) ≈ 0.7071, cos(45°) ≈ 0.7071
        let (s, c) = ANG45.sin_cos_f32();
        assert!((s - 0.7071).abs() < 0.01);
        assert!((c - 0.7071).abs() < 0.01);
    }

    #[test]
    fn from_radians_roundtrip() {
        use std::f32::consts::PI;
        let bam = Bam::from_radians(PI);
        let rad = bam.to_radians();
        assert!((rad - PI).abs() < 0.001);
    }

    #[test]
    fn const_table_matches_stdlib() {
        // Verify compile-time table matches runtime f64 sin
        for i in (0..FINE_TABLE_SIZE).step_by(64) {
            let table_val = SINE_TABLE[i].to_f32();
            let angle = (i as f64) * std::f64::consts::TAU / (FINE_TABLE_SIZE as f64);
            let expected = angle.sin() as f32;
            assert!(
                (table_val - expected).abs() < 0.001,
                "table[{i}] = {table_val}, expected {expected}"
            );
        }
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn wrapping_add_sub_inverse(a: u32, b: u32) {
            let ba = Bam(a);
            let bb = Bam(b);
            prop_assert_eq!((ba + bb) - bb, ba);
        }

        #[test]
        fn sin_bounded(raw: u32) {
            let (s, c) = Bam(raw).sin_cos_fixed();
            prop_assert!(s.to_f32().abs() <= 1.01);
            prop_assert!(c.to_f32().abs() <= 1.01);
        }

        #[test]
        fn fine_angle_always_in_bounds(raw: u32) {
            prop_assert!(Bam(raw).fine_angle() < FINE_TABLE_SIZE);
        }
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p abrash-core bam -- --nocapture`
Expected: FAIL — methods/table not implemented yet

**Step 3: Implement `Bam` with const sine table**

Add the implementation above the tests. The const table generation is the key difference from doom-rs:

```rust
/// Compile-time sine table generation.
/// doom-rs uses `static mut` with runtime init — this is the safe, const version.
const SINE_TABLE: [Fixed16_16; FINE_TABLE_SIZE] = {
    let mut table = [Fixed16_16(0); FINE_TABLE_SIZE];
    let mut i = 0;
    while i < FINE_TABLE_SIZE {
        // Manual Taylor series in const context (no f64::sin in const fn)
        // Compute angle: i * 2π / 8192
        // Use enough terms for < 0.001 error
        let angle_num = i as i64;
        // We need fixed-point arithmetic in const context for the Taylor series.
        // Alternative: precompute with a build script. For now, use the
        // well-known approach of integer-scaled Taylor coefficients.
        //
        // Actually, the simplest const approach: use the Bhaskara I sine approximation
        // which is algebraic (no transcendental functions needed in const context):
        //   sin(x) ≈ 16x(π - x) / (5π² - 4x(π - x))  for x in [0, π]
        //
        // But since we need high precision and const f64 math is available in Rust 2024
        // via const trait impls... let's just precompute the table values directly.
        //
        // IMPLEMENTATION NOTE: If const f64 trig is not available, use a build.rs
        // script to generate the table as a source file. The values are deterministic.

        // Placeholder — see Step 3 implementation note below.
        i += 1;
    }
    table
};
```

**IMPLEMENTATION NOTE:** Rust 2024 does NOT have `const fn` access to `f64::sin()`. Two options:
1. **Build script** (`crates/abrash-core/build.rs`) that generates a `sine_table.rs` file with the 8192 values as a literal array. Include with `include!(concat!(env!("OUT_DIR"), "/sine_table.rs"))`.
2. **Const Taylor series** with enough terms (7-9) for < 0.001 error, using only const-available integer/float arithmetic (`+`, `-`, `*`, `/` are const on f64).

**Use option 2** — a const Taylor series. The implementation:

```rust
/// Compute sin(x) at compile time using Taylor series.
/// x is in radians. Uses 7 terms for < 0.0001 error on [0, 2π].
const fn const_sin_f64(mut x: f64) -> f64 {
    // Normalize x to [-π, π]
    // Use manual modulo since f64::rem_euclid isn't const
    const TAU: f64 = 6.283185307179586;
    const PI: f64 = 3.141592653589793;

    // Rough normalization (sufficient for our input range 0..2π)
    while x > PI { x -= TAU; }
    while x < -PI { x += TAU; }

    // Taylor series: sin(x) = x - x³/3! + x⁵/5! - x⁷/7! + x⁹/9! - x¹¹/11! + x¹³/13!
    let x2 = x * x;
    let x3 = x2 * x;
    let x5 = x3 * x2;
    let x7 = x5 * x2;
    let x9 = x7 * x2;
    let x11 = x9 * x2;
    let x13 = x11 * x2;

    x - x3 / 6.0
      + x5 / 120.0
      - x7 / 5040.0
      + x9 / 362880.0
      - x11 / 39916800.0
      + x13 / 6227020800.0
}

const SINE_TABLE: [Fixed16_16; FINE_TABLE_SIZE] = {
    let mut table = [Fixed16_16(0); FINE_TABLE_SIZE];
    let mut i = 0;
    while i < FINE_TABLE_SIZE {
        let angle = (i as f64) * 6.283185307179586 / (FINE_TABLE_SIZE as f64);
        let sin_val = const_sin_f64(angle);
        table[i] = Fixed16_16((sin_val * 65536.0) as i32);
        i += 1;
    }
    table
};
```

Then implement the `Bam` methods:

```rust
impl Bam {
    pub const ZERO: Self = Self(0);

    #[inline]
    pub const fn from_raw(raw: u32) -> Self { Self(raw) }

    #[inline]
    pub const fn raw(self) -> u32 { self.0 }

    #[inline]
    pub fn wrapping_add(self, rhs: Self) -> Self { Self(self.0.wrapping_add(rhs.0)) }

    #[inline]
    pub fn wrapping_sub(self, rhs: Self) -> Self { Self(self.0.wrapping_sub(rhs.0)) }

    #[inline]
    pub fn negate(self) -> Self { Self(self.0.wrapping_neg()) }

    #[inline]
    pub const fn fine_angle(self) -> usize { (self.0 >> BAM_TO_FINE_SHIFT) as usize }

    /// Table lookup — fixed-point sin/cos. Fast path for raycaster.
    #[inline]
    pub fn sin_cos_fixed(&self) -> (Fixed16_16, Fixed16_16) {
        let sin = SINE_TABLE[self.fine_angle()];
        let cos_angle = Bam(self.0.wrapping_add(ANG90.0));
        let cos = SINE_TABLE[cos_angle.fine_angle()];
        (sin, cos)
    }

    /// Polynomial sin/cos — f32 output. For effects/one-off use.
    #[inline]
    pub fn sin_cos_f32(&self) -> (f32, f32) {
        crate::math::fast_sin_cos(self.to_radians())
    }

    #[inline]
    pub fn to_radians(self) -> f32 {
        (self.0 as f64 * std::f64::consts::TAU / (u32::MAX as f64 + 1.0)) as f32
    }

    #[inline]
    pub fn from_radians(rad: f32) -> Self {
        let raw = (rad as f64 / std::f64::consts::TAU * (u32::MAX as f64 + 1.0)) as u32;
        Self(raw)
    }
}
```

Plus operator impls: `Add`, `Sub`, `Neg`, `Display` — port from doom-rs verbatim.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p abrash-core bam`
Expected: ALL PASS

**Step 5: Register module**

Add to `crates/abrash-core/src/lib.rs`:
```rust
pub mod bam;
```

**Step 6: Run full abrash-core test suite + clippy**

Run: `cargo test -p abrash-core && cargo clippy -p abrash-core`
Expected: ALL PASS, no warnings

**Step 7: Commit**

```bash
git add crates/abrash-core/src/bam.rs crates/abrash-core/src/lib.rs
git commit -m "feat(core): add Bam angle type with const sine table"
```

---

### Task 3: BAM vs polynomial benchmark

**Files:**
- Create: `benches/bam_bench.rs`
- Modify: `Cargo.toml` (add `[[bench]]` entry)

**Step 1: Write the benchmark**

Create `benches/bam_bench.rs`:

```rust
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use abrash_core::bam::{Bam, ANG45};
use abrash_core::fixed16_16::Fixed16_16;
use abrash_core::math::fast_sin_cos;

fn bam_table_lookup(c: &mut Criterion) {
    let angle = ANG45;
    c.bench_function("bam_sin_cos_fixed (table)", |b| {
        b.iter(|| black_box(black_box(angle).sin_cos_fixed()))
    });
}

fn bam_polynomial(c: &mut Criterion) {
    let angle = ANG45;
    c.bench_function("bam_sin_cos_f32 (polynomial)", |b| {
        b.iter(|| black_box(black_box(angle).sin_cos_f32()))
    });
}

fn std_sin_cos(c: &mut Criterion) {
    let rad = std::f32::consts::FRAC_PI_4;
    c.bench_function("f32::sin_cos (stdlib)", |b| {
        b.iter(|| black_box(black_box(rad).sin_cos()))
    });
}

fn batch_1000_table(c: &mut Criterion) {
    let angles: Vec<Bam> = (0..1000).map(|i| Bam(i * 4_294_967)).collect();
    c.bench_function("1000x bam_sin_cos_fixed", |b| {
        b.iter(|| {
            for &a in &angles {
                black_box(a.sin_cos_fixed());
            }
        })
    });
}

fn batch_1000_polynomial(c: &mut Criterion) {
    let angles: Vec<f32> = (0..1000)
        .map(|i| i as f32 * std::f32::consts::TAU / 1000.0)
        .collect();
    c.bench_function("1000x fast_sin_cos (polynomial)", |b| {
        b.iter(|| {
            for &a in &angles {
                black_box(fast_sin_cos(a));
            }
        })
    });
}

criterion_group!(
    benches,
    bam_table_lookup,
    bam_polynomial,
    std_sin_cos,
    batch_1000_table,
    batch_1000_polynomial,
);
criterion_main!(benches);
```

**Step 2: Add bench entry to workspace `Cargo.toml`**

Add after the last `[[bench]]` entry:

```toml
[[bench]]
name = "bam_bench"
harness = false
```

**Step 3: Run the benchmark**

Run: `cargo bench --bench bam_bench`
Expected: Results showing table vs polynomial vs stdlib per-call and batch performance

**Step 4: Commit**

```bash
git add benches/bam_bench.rs Cargo.toml
git commit -m "bench: add BAM table vs polynomial vs stdlib trig comparison"
```

---

## Phase 2: `abrash-raycast` Crate

### Task 4: Scaffold `abrash-raycast` crate

**Files:**
- Create: `crates/abrash-raycast/Cargo.toml`
- Create: `crates/abrash-raycast/src/lib.rs`
- Create: `crates/abrash-raycast/src/types.rs`
- Create: `crates/abrash-raycast/src/map.rs`
- Modify: `Cargo.toml` (add to workspace members)

**Step 1: Create `Cargo.toml`**

```toml
[package]
name = "abrash-raycast"
version = "0.1.0"
edition = "2024"

[features]
default = []
parallel = ["dep:rayon"]

[dependencies]
abrash-core = { path = "../abrash-core" }
rayon = { version = "1.10", optional = true }

[dev-dependencies]
proptest = "1.0"

[lints.clippy]
pedantic = { level = "warn", priority = -1 }
nursery = { level = "warn", priority = -1 }
cast_possible_truncation = "allow"
cast_possible_wrap = "allow"
cast_sign_loss = "allow"
cast_precision_loss = "allow"
similar_names = "allow"
many_single_char_names = "allow"
inline_always = "allow"
struct_field_names = "allow"
suboptimal_flops = "allow"
```

**Step 2: Create `src/types.rs`**

```rust
//! Core types for raycasting results.

use abrash_core::fixed16_16::Fixed16_16;

/// Which face of a grid cell was hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    North,
    South,
    East,
    West,
}

/// Grid cell contents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    /// Empty space — rays pass through.
    Empty,
    /// Solid wall with a material ID.
    Solid(u16),
    /// Portal to another connected space.
    Portal(u16),
}

impl Cell {
    /// Returns `true` if this cell blocks ray traversal.
    #[inline]
    pub fn is_solid(&self) -> bool {
        matches!(self, Self::Solid(_))
    }
}

/// 2D vector in fixed-point space for raycasting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Vec2Fixed {
    pub x: Fixed16_16,
    pub y: Fixed16_16,
}

impl Vec2Fixed {
    pub const ZERO: Self = Self {
        x: Fixed16_16::ZERO,
        y: Fixed16_16::ZERO,
    };

    #[inline]
    pub const fn new(x: Fixed16_16, y: Fixed16_16) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn from_ints(x: i32, y: i32) -> Self {
        Self {
            x: Fixed16_16::from_int(x),
            y: Fixed16_16::from_int(y),
        }
    }

    #[inline]
    pub fn from_f32(x: f32, y: f32) -> Self {
        Self {
            x: Fixed16_16::from_f32(x),
            y: Fixed16_16::from_f32(y),
        }
    }
}

/// Ray hit result — middle tier.
#[derive(Clone, Copy, Debug)]
pub struct RayHit {
    /// Perpendicular distance from camera plane (no fisheye).
    pub distance: Fixed16_16,
    /// Grid cell X coordinate that was hit.
    pub cell_x: u32,
    /// Grid cell Y coordinate that was hit.
    pub cell_y: u32,
    /// Which face of the cell was hit.
    pub side: Side,
}

/// Detailed ray hit — rendering tier.
#[derive(Clone, Copy, Debug)]
pub struct DetailedHit {
    /// Base hit information.
    pub hit: RayHit,
    /// Exact world-space hit point.
    pub point: Vec2Fixed,
    /// Surface normal at hit point.
    pub normal: Vec2Fixed,
    /// Texture U coordinate (0..1 along the wall face).
    pub texture_u: Fixed16_16,
}
```

**Step 3: Create `src/map.rs`**

```rust
//! Map traits for raycasting.
//!
//! Two abstraction layers:
//! - `GridMap`: uniform grid for DDA fast path (built first)
//! - `SectorMap`: arbitrary geometry for BSP traversal (designed, deferred)

use crate::types::Cell;

/// Uniform grid map — DDA + BAM fast path.
///
/// Implementors must be `Send + Sync` (only shared references needed).
/// The ECS owns the map data; the raycaster borrows it.
pub trait GridMap {
    /// Grid width in cells.
    fn width(&self) -> u32;
    /// Grid height in cells.
    fn height(&self) -> u32;
    /// Cell contents at the given grid coordinates.
    /// Returns `Cell::Empty` for out-of-bounds coordinates.
    fn cell_at(&self, x: u32, y: u32) -> Cell;
}

/// Simple owned grid map for testing and standalone use.
pub struct ArrayGridMap {
    cells: Vec<Cell>,
    width: u32,
    height: u32,
}

impl ArrayGridMap {
    /// Create a new grid map filled with `Cell::Empty`.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            cells: vec![Cell::Empty; (width * height) as usize],
            width,
            height,
        }
    }

    /// Set a cell value.
    pub fn set(&mut self, x: u32, y: u32, cell: Cell) {
        if x < self.width && y < self.height {
            self.cells[(y * self.width + x) as usize] = cell;
        }
    }
}

impl GridMap for ArrayGridMap {
    fn width(&self) -> u32 { self.width }
    fn height(&self) -> u32 { self.height }
    fn cell_at(&self, x: u32, y: u32) -> Cell {
        if x < self.width && y < self.height {
            self.cells[(y * self.width + x) as usize]
        } else {
            Cell::Empty
        }
    }
}
```

**Step 4: Create `src/lib.rs`**

```rust
#![allow(clippy::all, unused_variables, dead_code, unused_imports, unused_mut)]
//! Raycasting engine for the Abrash graphics project.
//!
//! Stateless free functions, borrow-only, `Send + Sync` by construction.
//! Designed for ECS integration: no owned state, no synchronization needed.

pub mod map;
pub mod types;
```

**Step 5: Add to workspace**

In root `Cargo.toml`, add `"crates/abrash-raycast"` to the `[workspace] members` list.

**Step 6: Verify it compiles**

Run: `cargo check -p abrash-raycast`
Expected: PASS

**Step 7: Commit**

```bash
git add crates/abrash-raycast/ Cargo.toml
git commit -m "feat(raycast): scaffold abrash-raycast crate with types and map traits"
```

---

### Task 5: DDA stepper

**Files:**
- Create: `crates/abrash-raycast/src/dda.rs`
- Modify: `crates/abrash-raycast/src/lib.rs` (add `pub mod dda;`)

**Step 1: Write failing tests**

Create `crates/abrash-raycast/src/dda.rs` with tests:

```rust
//! DDA (Digital Differential Analyzer) grid traversal.
//!
//! Steps through grid cells along a ray. Uses `Bam::sin_cos_fixed()` for
//! setup (one table lookup per ray), then pure fixed-point add+compare per step.

use abrash_core::bam::Bam;
use abrash_core::fixed16_16::{Fixed16_16, FIXED_ONE};
use crate::types::{Side, Vec2Fixed};

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::bam::{ANG90, ANG180, ANG270};

    #[test]
    fn step_east_along_x_axis() {
        // Ray at angle 0 (East) from (0.5, 0.5) should step through cells (1,0), (2,0), ...
        let origin = Vec2Fixed::from_f32(0.5, 0.5);
        let mut stepper = DdaStepper::new(origin, Bam::ZERO);
        let (cx, cy, side) = stepper.step();
        assert_eq!((cx, cy), (1, 0));
        assert_eq!(side, Side::West); // hit the west face of cell (1,0)
        let (cx, cy, _) = stepper.step();
        assert_eq!((cx, cy), (2, 0));
    }

    #[test]
    fn step_north_along_y_axis() {
        // Ray at 90° (North) from (0.5, 0.5) should step through cells (0,1), (0,2), ...
        let origin = Vec2Fixed::from_f32(0.5, 0.5);
        let mut stepper = DdaStepper::new(origin, ANG90);
        let (cx, cy, side) = stepper.step();
        assert_eq!((cx, cy), (0, 1));
        assert_eq!(side, Side::South); // hit the south face of cell (0,1)
    }

    #[test]
    fn step_west() {
        let origin = Vec2Fixed::from_f32(1.5, 0.5);
        let mut stepper = DdaStepper::new(origin, ANG180);
        let (cx, cy, side) = stepper.step();
        assert_eq!((cx, cy), (0, 0));
        assert_eq!(side, Side::East);
    }

    #[test]
    fn step_south() {
        let origin = Vec2Fixed::from_f32(0.5, 1.5);
        let mut stepper = DdaStepper::new(origin, ANG270);
        let (cx, cy, side) = stepper.step();
        assert_eq!((cx, cy), (0, 0));
        assert_eq!(side, Side::North);
    }

    #[test]
    fn perp_distance_east() {
        let origin = Vec2Fixed::from_f32(0.5, 0.5);
        let mut stepper = DdaStepper::new(origin, Bam::ZERO);
        stepper.step();
        let dist = stepper.perp_distance();
        // Distance from 0.5 to cell boundary at x=1 = 0.5
        assert!((dist.to_f32() - 0.5).abs() < 0.05, "dist = {}", dist.to_f32());
    }

    #[test]
    fn multiple_steps_increase_distance() {
        let origin = Vec2Fixed::from_f32(0.5, 0.5);
        let mut stepper = DdaStepper::new(origin, Bam::ZERO);
        stepper.step();
        let d1 = stepper.perp_distance();
        stepper.step();
        let d2 = stepper.perp_distance();
        assert!(d2.raw() > d1.raw(), "d2 ({}) should be > d1 ({})", d2.to_f32(), d1.to_f32());
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p abrash-raycast dda -- --nocapture`
Expected: FAIL — `DdaStepper` not implemented

**Step 3: Implement `DdaStepper`**

```rust
/// Internal DDA grid stepper. Shared by all query tiers.
pub(crate) struct DdaStepper {
    /// Current grid cell.
    pub cell_x: i32,
    pub cell_y: i32,
    /// Step direction: +1 or -1 for each axis.
    step_x: i32,
    step_y: i32,
    /// Distance to next grid line on each axis.
    side_dist_x: Fixed16_16,
    side_dist_y: Fixed16_16,
    /// Distance between consecutive grid lines along the ray direction.
    delta_dist_x: Fixed16_16,
    delta_dist_y: Fixed16_16,
    /// Which side was hit on the last step.
    last_side: Side,
}

impl DdaStepper {
    /// Create a new stepper for a ray from `origin` in direction `angle`.
    ///
    /// Uses `Bam::sin_cos_fixed()` — one table lookup. After this,
    /// stepping is pure fixed-point add+compare.
    pub fn new(origin: Vec2Fixed, angle: Bam) -> Self {
        let (sin, cos) = angle.sin_cos_fixed();

        let cell_x = origin.x.to_int();
        let cell_y = origin.y.to_int();

        // Avoid division by zero: if a component is zero, delta is "infinite"
        let abs_cos = Fixed16_16(cos.raw().wrapping_abs());
        let abs_sin = Fixed16_16(sin.raw().wrapping_abs());

        let delta_dist_x = if abs_cos.raw() > 0 {
            FIXED_ONE.fixed_div(abs_cos)
        } else {
            Fixed16_16(i32::MAX)
        };
        let delta_dist_y = if abs_sin.raw() > 0 {
            FIXED_ONE.fixed_div(abs_sin)
        } else {
            Fixed16_16(i32::MAX)
        };

        // Step direction and initial side distance
        let (step_x, side_dist_x) = if cos.raw() >= 0 {
            let frac = origin.x - Fixed16_16::from_int(cell_x);
            (1, (FIXED_ONE - frac).fixed_mul(delta_dist_x))
        } else {
            let frac = origin.x - Fixed16_16::from_int(cell_x);
            (-1, frac.fixed_mul(delta_dist_x))
        };

        let (step_y, side_dist_y) = if sin.raw() >= 0 {
            let frac = origin.y - Fixed16_16::from_int(cell_y);
            (1, (FIXED_ONE - frac).fixed_mul(delta_dist_y))
        } else {
            let frac = origin.y - Fixed16_16::from_int(cell_y);
            (-1, frac.fixed_mul(delta_dist_y))
        };

        Self {
            cell_x,
            cell_y,
            step_x,
            step_y,
            side_dist_x,
            side_dist_y,
            delta_dist_x,
            delta_dist_y,
            last_side: Side::East, // placeholder, updated on first step
        }
    }

    /// Advance one grid cell. Returns (cell_x, cell_y, side_hit).
    #[inline]
    pub fn step(&mut self) -> (i32, i32, Side) {
        if self.side_dist_x.raw() < self.side_dist_y.raw() {
            self.cell_x += self.step_x;
            self.last_side = if self.step_x > 0 { Side::West } else { Side::East };
            self.side_dist_x = self.side_dist_x + self.delta_dist_x;
        } else {
            self.cell_y += self.step_y;
            self.last_side = if self.step_y > 0 { Side::South } else { Side::North };
            self.side_dist_y = self.side_dist_y + self.delta_dist_y;
        }
        (self.cell_x, self.cell_y, self.last_side)
    }

    /// Perpendicular distance from the camera plane to the last hit.
    /// No fisheye — this is the corrected distance for rendering.
    #[inline]
    pub fn perp_distance(&self) -> Fixed16_16 {
        match self.last_side {
            Side::West | Side::East => self.side_dist_x - self.delta_dist_x,
            Side::North | Side::South => self.side_dist_y - self.delta_dist_y,
        }
    }
}
```

**Step 4: Register module**

Add to `crates/abrash-raycast/src/lib.rs`:
```rust
pub mod dda;
```

**Step 5: Run tests**

Run: `cargo test -p abrash-raycast dda`
Expected: ALL PASS

**Step 6: Commit**

```bash
git add crates/abrash-raycast/src/dda.rs crates/abrash-raycast/src/lib.rs
git commit -m "feat(raycast): implement DDA grid stepper with fixed-point arithmetic"
```

---

### Task 6: Tiered casting API (`cast_los`, `cast_ray`, `cast_ray_detailed`)

**Files:**
- Create: `crates/abrash-raycast/src/cast.rs`
- Modify: `crates/abrash-raycast/src/lib.rs` (add `pub mod cast;`)

**Step 1: Write failing tests**

Create `crates/abrash-raycast/src/cast.rs` with tests:

```rust
//! Tiered raycasting API — stateless free functions.

use abrash_core::bam::Bam;
use abrash_core::fixed16_16::Fixed16_16;
use crate::dda::DdaStepper;
use crate::map::GridMap;
use crate::types::{Cell, DetailedHit, RayHit, Side, Vec2Fixed};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::ArrayGridMap;
    use abrash_core::bam::{ANG90, ANG180, ANG270};

    fn corridor_map() -> ArrayGridMap {
        // 8x8 map with walls around the border, empty inside
        let mut map = ArrayGridMap::new(8, 8);
        for x in 0..8 {
            map.set(x, 0, Cell::Solid(1)); // south wall
            map.set(x, 7, Cell::Solid(1)); // north wall
        }
        for y in 0..8 {
            map.set(0, y, Cell::Solid(1)); // west wall
            map.set(7, y, Cell::Solid(1)); // east wall
        }
        // Add a wall in the middle at (4, 4)
        map.set(4, 4, Cell::Solid(2));
        map
    }

    // ---- cast_los tests ----

    #[test]
    fn los_clear_corridor() {
        let map = corridor_map();
        let from = Vec2Fixed::from_f32(2.5, 2.5);
        let to = Vec2Fixed::from_f32(5.5, 2.5);
        assert!(cast_los(&map, from, to));
    }

    #[test]
    fn los_blocked_by_wall() {
        let map = corridor_map();
        let from = Vec2Fixed::from_f32(2.5, 4.5);
        let to = Vec2Fixed::from_f32(5.5, 4.5); // wall at (4,4) blocks this
        assert!(!cast_los(&map, from, to));
    }

    #[test]
    fn los_blocked_by_border() {
        let map = corridor_map();
        let from = Vec2Fixed::from_f32(1.5, 1.5);
        let to = Vec2Fixed::from_f32(1.5, 7.5); // north border wall
        assert!(!cast_los(&map, from, to));
    }

    #[test]
    fn los_same_point() {
        let map = corridor_map();
        let p = Vec2Fixed::from_f32(3.5, 3.5);
        assert!(cast_los(&map, p, p));
    }

    #[test]
    fn los_symmetry() {
        let map = corridor_map();
        let a = Vec2Fixed::from_f32(2.5, 3.5);
        let b = Vec2Fixed::from_f32(5.5, 5.5);
        assert_eq!(cast_los(&map, a, b), cast_los(&map, b, a));
    }

    // ---- cast_ray tests ----

    #[test]
    fn ray_east_hits_wall() {
        let map = corridor_map();
        let origin = Vec2Fixed::from_f32(1.5, 3.5);
        let hit = cast_ray(&map, origin, Bam::ZERO).expect("should hit east wall");
        assert_eq!(hit.cell_x, 7);
        assert_eq!(hit.side, Side::West);
    }

    #[test]
    fn ray_hits_middle_wall() {
        let map = corridor_map();
        let origin = Vec2Fixed::from_f32(2.5, 4.5);
        let hit = cast_ray(&map, origin, Bam::ZERO).expect("should hit wall at (4,4)");
        assert_eq!(hit.cell_x, 4);
        assert_eq!(hit.cell_y, 4);
        assert_eq!(hit.side, Side::West);
    }

    #[test]
    fn ray_distance_increases_with_further_wall() {
        let map = corridor_map();
        let origin = Vec2Fixed::from_f32(1.5, 3.5); // no wall in between, hits border at x=7
        let far = cast_ray(&map, origin, Bam::ZERO).unwrap();

        let origin2 = Vec2Fixed::from_f32(5.5, 3.5); // closer to east wall
        let near = cast_ray(&map, origin2, Bam::ZERO).unwrap();

        assert!(far.distance.raw() > near.distance.raw());
    }

    #[test]
    fn ray_north() {
        let map = corridor_map();
        let origin = Vec2Fixed::from_f32(3.5, 1.5);
        let hit = cast_ray(&map, origin, ANG90).expect("should hit north wall");
        assert_eq!(hit.cell_y, 7);
        assert_eq!(hit.side, Side::South);
    }

    // ---- cast_ray_detailed tests ----

    #[test]
    fn detailed_has_texture_u() {
        let map = corridor_map();
        let origin = Vec2Fixed::from_f32(3.5, 1.5);
        let detail = cast_ray_detailed(&map, origin, Bam::ZERO).expect("should hit");
        // texture_u should be in [0, 1)
        assert!(detail.texture_u.to_f32() >= 0.0);
        assert!(detail.texture_u.to_f32() < 1.0);
    }

    #[test]
    fn detailed_normal_is_unit_cardinal() {
        let map = corridor_map();
        let origin = Vec2Fixed::from_f32(1.5, 3.5);
        let detail = cast_ray_detailed(&map, origin, Bam::ZERO).expect("should hit");
        // Hit west face of east wall — normal should point west (-1, 0)
        assert!(detail.normal.x.to_f32() < -0.9);
        assert!(detail.normal.y.to_f32().abs() < 0.1);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p abrash-raycast cast -- --nocapture`
Expected: FAIL — functions not implemented

**Step 3: Implement the three tiers**

```rust
/// Maximum number of DDA steps before giving up.
const MAX_STEPS: u32 = 256;

/// Fastest tier — line-of-sight check.
///
/// Returns `true` if there is a clear line from `from` to `to` (no solid cells in the way).
/// Early-outs on first solid hit. No distance computation.
pub fn cast_los(map: &impl GridMap, from: Vec2Fixed, to: Vec2Fixed) -> bool {
    // Same cell check
    let from_cx = from.x.to_int();
    let from_cy = from.y.to_int();
    let to_cx = to.x.to_int();
    let to_cy = to.y.to_int();
    if from_cx == to_cx && from_cy == to_cy {
        return true;
    }

    // Compute angle from `from` to `to`
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let angle = Bam::from_radians(f32::atan2(dy.to_f32(), dx.to_f32()));

    let mut stepper = DdaStepper::new(from, angle);
    for _ in 0..MAX_STEPS {
        let (cx, cy, _side) = stepper.step();
        // Reached target cell?
        if cx == to_cx && cy == to_cy {
            return true;
        }
        // Out of bounds = clear (no wall to block)
        if cx < 0 || cy < 0 || cx >= map.width() as i32 || cy >= map.height() as i32 {
            return true;
        }
        // Hit a solid cell?
        if map.cell_at(cx as u32, cy as u32).is_solid() {
            return false;
        }
    }
    false // max steps exceeded — assume blocked
}

/// Middle tier — cast a ray and get hit distance + cell + side.
///
/// Returns `None` if the ray exits the map without hitting anything solid.
pub fn cast_ray(map: &impl GridMap, origin: Vec2Fixed, angle: Bam) -> Option<RayHit> {
    let mut stepper = DdaStepper::new(origin, angle);
    for _ in 0..MAX_STEPS {
        let (cx, cy, side) = stepper.step();
        if cx < 0 || cy < 0 || cx >= map.width() as i32 || cy >= map.height() as i32 {
            return None;
        }
        if map.cell_at(cx as u32, cy as u32).is_solid() {
            return Some(RayHit {
                distance: stepper.perp_distance(),
                cell_x: cx as u32,
                cell_y: cy as u32,
                side,
            });
        }
    }
    None
}

/// Full detail tier — adds hit point, normal, texture U coordinate.
///
/// For rendering only (once per screen column).
pub fn cast_ray_detailed(
    map: &impl GridMap,
    origin: Vec2Fixed,
    angle: Bam,
) -> Option<DetailedHit> {
    let (sin, cos) = angle.sin_cos_fixed();
    let mut stepper = DdaStepper::new(origin, angle);

    for _ in 0..MAX_STEPS {
        let (cx, cy, side) = stepper.step();
        if cx < 0 || cy < 0 || cx >= map.width() as i32 || cy >= map.height() as i32 {
            return None;
        }
        if map.cell_at(cx as u32, cy as u32).is_solid() {
            let perp_dist = stepper.perp_distance();

            // Compute exact hit point
            let hit_x = origin.x + cos.fixed_mul(perp_dist);
            let hit_y = origin.y + sin.fixed_mul(perp_dist);

            // Surface normal (always axis-aligned for grid maps)
            let normal = match side {
                Side::West  => Vec2Fixed::new(Fixed16_16::from_int(-1), Fixed16_16::ZERO),
                Side::East  => Vec2Fixed::new(Fixed16_16::from_int(1), Fixed16_16::ZERO),
                Side::South => Vec2Fixed::new(Fixed16_16::ZERO, Fixed16_16::from_int(-1)),
                Side::North => Vec2Fixed::new(Fixed16_16::ZERO, Fixed16_16::from_int(1)),
            };

            // Texture U: fractional position along the wall face
            let texture_u = match side {
                Side::West | Side::East => {
                    let frac = hit_y - Fixed16_16::from_int(hit_y.to_int());
                    frac
                }
                Side::North | Side::South => {
                    let frac = hit_x - Fixed16_16::from_int(hit_x.to_int());
                    frac
                }
            };

            return Some(DetailedHit {
                hit: RayHit {
                    distance: perp_dist,
                    cell_x: cx as u32,
                    cell_y: cy as u32,
                    side,
                },
                point: Vec2Fixed::new(hit_x, hit_y),
                normal,
                texture_u,
            });
        }
    }
    None
}
```

**Step 4: Register module**

Add to `crates/abrash-raycast/src/lib.rs`:
```rust
pub mod cast;
```

**Step 5: Run tests**

Run: `cargo test -p abrash-raycast cast`
Expected: ALL PASS

**Step 6: Run full crate tests + clippy**

Run: `cargo test -p abrash-raycast && cargo clippy -p abrash-raycast`
Expected: ALL PASS

**Step 7: Commit**

```bash
git add crates/abrash-raycast/src/cast.rs crates/abrash-raycast/src/lib.rs
git commit -m "feat(raycast): implement tiered casting API (cast_los, cast_ray, cast_ray_detailed)"
```

---

### Task 7: Property tests for casting

**Files:**
- Modify: `crates/abrash-raycast/src/cast.rs` (add prop_tests module)

**Step 1: Add property tests to cast.rs**

Append to the bottom of `crates/abrash-raycast/src/cast.rs`:

```rust
#[cfg(test)]
mod prop_tests {
    use super::*;
    use crate::map::ArrayGridMap;
    use proptest::prelude::*;

    /// Create a fully-walled map — all border cells are solid.
    fn walled_map() -> ArrayGridMap {
        let mut map = ArrayGridMap::new(16, 16);
        for x in 0..16 {
            map.set(x, 0, Cell::Solid(1));
            map.set(x, 15, Cell::Solid(1));
        }
        for y in 0..16 {
            map.set(0, y, Cell::Solid(1));
            map.set(15, y, Cell::Solid(1));
        }
        map
    }

    proptest! {
        #[test]
        fn ray_distance_is_non_negative(raw_angle: u32) {
            let map = walled_map();
            let origin = Vec2Fixed::from_f32(8.0, 8.0);
            if let Some(hit) = cast_ray(&map, origin, Bam(raw_angle)) {
                prop_assert!(hit.distance.raw() >= 0, "negative distance: {}", hit.distance.to_f32());
            }
        }

        #[test]
        fn ray_always_hits_walled_map(raw_angle: u32) {
            let map = walled_map();
            let origin = Vec2Fixed::from_f32(8.0, 8.0);
            let hit = cast_ray(&map, origin, Bam(raw_angle));
            prop_assert!(hit.is_some(), "ray should always hit a wall in enclosed map");
        }

        #[test]
        fn hit_cell_is_solid(raw_angle: u32) {
            let map = walled_map();
            let origin = Vec2Fixed::from_f32(8.0, 8.0);
            if let Some(hit) = cast_ray(&map, origin, Bam(raw_angle)) {
                let cell = map.cell_at(hit.cell_x, hit.cell_y);
                prop_assert!(cell.is_solid(), "hit cell ({},{}) is not solid", hit.cell_x, hit.cell_y);
            }
        }

        #[test]
        fn detailed_texture_u_in_range(raw_angle: u32) {
            let map = walled_map();
            let origin = Vec2Fixed::from_f32(8.0, 8.0);
            if let Some(detail) = cast_ray_detailed(&map, origin, Bam(raw_angle)) {
                let u = detail.texture_u.to_f32();
                prop_assert!(u >= -0.01 && u <= 1.01, "texture_u out of range: {u}");
            }
        }

        #[test]
        fn los_symmetry(x1 in 1.5_f32..14.5, y1 in 1.5_f32..14.5,
                        x2 in 1.5_f32..14.5, y2 in 1.5_f32..14.5) {
            let map = walled_map();
            let a = Vec2Fixed::from_f32(x1, y1);
            let b = Vec2Fixed::from_f32(x2, y2);
            prop_assert_eq!(cast_los(&map, a, b), cast_los(&map, b, a));
        }
    }
}
```

**Step 2: Run property tests**

Run: `cargo test -p abrash-raycast prop_tests`
Expected: ALL PASS

**Step 3: Commit**

```bash
git add crates/abrash-raycast/src/cast.rs
git commit -m "test(raycast): add property tests for casting correctness"
```

---

## Phase 3: Batch & Parallel

### Task 8: Batch casting API

**Files:**
- Create: `crates/abrash-raycast/src/batch.rs`
- Modify: `crates/abrash-raycast/src/lib.rs` (add `pub mod batch;`)

**Step 1: Write failing tests**

Create `crates/abrash-raycast/src/batch.rs` with tests:

```rust
//! Batch raycasting for ECS systems.
//!
//! Scatter-gather pattern: collect queries, submit batch, read results.
//! Optional parallelism via `parallel` feature flag (rayon).

use abrash_core::bam::Bam;
use crate::cast::{cast_los, cast_ray};
use crate::map::GridMap;
use crate::types::{RayHit, Vec2Fixed};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::ArrayGridMap;
    use crate::types::Cell;
    use abrash_core::bam::ANG90;

    fn test_map() -> ArrayGridMap {
        let mut map = ArrayGridMap::new(8, 8);
        for x in 0..8 {
            map.set(x, 0, Cell::Solid(1));
            map.set(x, 7, Cell::Solid(1));
        }
        for y in 0..8 {
            map.set(0, y, Cell::Solid(1));
            map.set(7, y, Cell::Solid(1));
        }
        map
    }

    #[test]
    fn batch_ray_matches_individual() {
        let map = test_map();
        let rays = vec![
            (Vec2Fixed::from_f32(4.0, 4.0), Bam::ZERO),
            (Vec2Fixed::from_f32(4.0, 4.0), ANG90),
        ];
        let mut results = vec![None; 2];
        cast_rays_batch(&map, &rays, &mut results);

        for (i, (origin, angle)) in rays.iter().enumerate() {
            let individual = cast_ray(&map, *origin, *angle);
            match (&results[i], &individual) {
                (Some(a), Some(b)) => {
                    assert_eq!(a.cell_x, b.cell_x);
                    assert_eq!(a.cell_y, b.cell_y);
                    assert_eq!(a.side, b.side);
                }
                (None, None) => {}
                _ => panic!("mismatch at ray {i}"),
            }
        }
    }

    #[test]
    fn batch_los_matches_individual() {
        let map = test_map();
        let pairs = vec![
            (Vec2Fixed::from_f32(2.0, 2.0), Vec2Fixed::from_f32(5.0, 5.0)),
            (Vec2Fixed::from_f32(1.5, 1.5), Vec2Fixed::from_f32(6.5, 1.5)),
        ];
        let mut results = vec![false; 2];
        cast_los_batch(&map, &pairs, &mut results);

        for (i, (from, to)) in pairs.iter().enumerate() {
            assert_eq!(results[i], cast_los(&map, *from, *to), "mismatch at pair {i}");
        }
    }

    #[test]
    fn batch_empty_input() {
        let map = test_map();
        let mut results: Vec<Option<RayHit>> = vec![];
        cast_rays_batch(&map, &[], &mut results);
        assert!(results.is_empty());
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p abrash-raycast batch -- --nocapture`
Expected: FAIL

**Step 3: Implement batch functions**

```rust
/// Batch ray casting. Results are written into `results` (must be same length as `rays`).
///
/// When the `parallel` feature is enabled, uses rayon for parallel dispatch.
pub fn cast_rays_batch(
    map: &(impl GridMap + Sync),
    rays: &[(Vec2Fixed, Bam)],
    results: &mut [Option<RayHit>],
) {
    debug_assert_eq!(rays.len(), results.len());

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        rays.par_iter()
            .zip(results.par_iter_mut())
            .for_each(|((origin, angle), result)| {
                *result = cast_ray(map, *origin, *angle);
            });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for (i, (origin, angle)) in rays.iter().enumerate() {
            results[i] = cast_ray(map, *origin, *angle);
        }
    }
}

/// Batch LOS checks. Results are written into `results` (must be same length as `pairs`).
pub fn cast_los_batch(
    map: &(impl GridMap + Sync),
    pairs: &[(Vec2Fixed, Vec2Fixed)],
    results: &mut [bool],
) {
    debug_assert_eq!(pairs.len(), results.len());

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        pairs.par_iter()
            .zip(results.par_iter_mut())
            .for_each(|((from, to), result)| {
                *result = cast_los(map, *from, *to);
            });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for (i, (from, to)) in pairs.iter().enumerate() {
            results[i] = cast_los(map, *from, *to);
        }
    }
}
```

**Step 4: Register module**

Add to `crates/abrash-raycast/src/lib.rs`:
```rust
pub mod batch;
```

**Step 5: Run tests (both serial and parallel)**

Run: `cargo test -p abrash-raycast batch`
Run: `cargo test -p abrash-raycast batch --features parallel`
Expected: ALL PASS both ways

**Step 6: Commit**

```bash
git add crates/abrash-raycast/src/batch.rs crates/abrash-raycast/src/lib.rs
git commit -m "feat(raycast): add batch casting API with optional rayon parallelism"
```

---

### Task 9: Raycast benchmarks

**Files:**
- Create: `benches/raycast_bench.rs`
- Modify: `Cargo.toml` (add bench entry + abrash-raycast dep)

**Step 1: Write the benchmark**

Create `benches/raycast_bench.rs`:

```rust
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use abrash_core::bam::Bam;
use abrash_core::fixed16_16::Fixed16_16;
use abrash_raycast::cast::{cast_los, cast_ray, cast_ray_detailed};
use abrash_raycast::batch::cast_rays_batch;
use abrash_raycast::map::{ArrayGridMap, GridMap};
use abrash_raycast::types::{Cell, Vec2Fixed};

fn make_maze(size: u32) -> ArrayGridMap {
    let mut map = ArrayGridMap::new(size, size);
    // Border walls
    for x in 0..size {
        map.set(x, 0, Cell::Solid(1));
        map.set(x, size - 1, Cell::Solid(1));
    }
    for y in 0..size {
        map.set(0, y, Cell::Solid(1));
        map.set(size - 1, y, Cell::Solid(1));
    }
    // Checkerboard interior walls for depth complexity
    for y in 2..size - 2 {
        for x in 2..size - 2 {
            if x % 3 == 0 && y % 3 == 0 {
                map.set(x, y, Cell::Solid(1));
            }
        }
    }
    map
}

fn single_cast_ray(c: &mut Criterion) {
    let map = make_maze(64);
    let origin = Vec2Fixed::from_f32(32.0, 32.0);
    c.bench_function("cast_ray 64x64", |b| {
        b.iter(|| black_box(cast_ray(&map, black_box(origin), black_box(Bam::ZERO))))
    });
}

fn single_cast_ray_detailed(c: &mut Criterion) {
    let map = make_maze(64);
    let origin = Vec2Fixed::from_f32(32.0, 32.0);
    c.bench_function("cast_ray_detailed 64x64", |b| {
        b.iter(|| black_box(cast_ray_detailed(&map, black_box(origin), black_box(Bam::ZERO))))
    });
}

fn single_cast_los(c: &mut Criterion) {
    let map = make_maze(64);
    let from = Vec2Fixed::from_f32(5.5, 5.5);
    let to = Vec2Fixed::from_f32(58.5, 58.5);
    c.bench_function("cast_los 64x64 diagonal", |b| {
        b.iter(|| black_box(cast_los(&map, black_box(from), black_box(to))))
    });
}

fn batch_1000_rays(c: &mut Criterion) {
    let map = make_maze(64);
    let rays: Vec<(Vec2Fixed, Bam)> = (0..1000)
        .map(|i| {
            let origin = Vec2Fixed::from_f32(32.0, 32.0);
            let angle = Bam(i * 4_294_967); // spread across full circle
            (origin, angle)
        })
        .collect();
    let mut results = vec![None; 1000];

    c.bench_function("batch 1000 rays 64x64", |b| {
        b.iter(|| cast_rays_batch(&map, black_box(&rays), &mut results))
    });
}

fn map_size_scaling(c: &mut Criterion) {
    for size in [16, 64, 128, 256] {
        let map = make_maze(size);
        let center = size as f32 / 2.0;
        let origin = Vec2Fixed::from_f32(center, center);
        c.bench_function(&format!("cast_ray {size}x{size}"), |b| {
            b.iter(|| black_box(cast_ray(&map, black_box(origin), black_box(Bam::ZERO))))
        });
    }
}

criterion_group!(
    benches,
    single_cast_ray,
    single_cast_ray_detailed,
    single_cast_los,
    batch_1000_rays,
    map_size_scaling,
);
criterion_main!(benches);
```

**Step 2: Add bench entry + dependency to workspace `Cargo.toml`**

Add `abrash-raycast` to `[dependencies]`:
```toml
abrash-raycast = { path = "crates/abrash-raycast" }
```

Add `[[bench]]` entry:
```toml
[[bench]]
name = "raycast_bench"
harness = false
```

**Step 3: Run the benchmark**

Run: `cargo bench --bench raycast_bench`
Expected: Results for all benchmarks

**Step 4: Commit**

```bash
git add benches/raycast_bench.rs Cargo.toml
git commit -m "bench: add raycast performance benchmarks (single, batch, scaling)"
```

---

## Phase 4: Hybrid Rendering

### Task 10: Raycast column renderer in `abrash-render`

**Files:**
- Create: `crates/abrash-render/src/raycaster/mod.rs`
- Create: `crates/abrash-render/src/raycaster/hybrid.rs`
- Modify: `crates/abrash-render/src/lib.rs` (add `pub mod raycaster;`)
- Modify: `crates/abrash-render/Cargo.toml` (add abrash-raycast dep)

**Step 1: Add dependency**

Add to `crates/abrash-render/Cargo.toml`:
```toml
abrash-raycast = { path = "../abrash-raycast" }
```

**Step 2: Create `raycaster/mod.rs`**

```rust
//! Raycaster rendering — draws raycasted views to the framebuffer.
//!
//! Consumes `abrash-raycast` for the casting math. This module handles
//! the rendering side: column drawing, z-buffer integration, and
//! hybrid compositing with the triangle rasterizer.

pub mod hybrid;
```

**Step 3: Create `raycaster/hybrid.rs` with tests**

```rust
//! Hybrid rendering: raycasted walls + triangle-rasterized objects.

use abrash_core::bam::Bam;
use abrash_core::fixed16_16::Fixed16_16;
use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_raycast::cast::cast_ray_detailed;
use abrash_raycast::map::GridMap;
use abrash_raycast::types::{Side, Vec2Fixed};

/// Wall color palette — maps material ID + side to ARGB color.
/// Simple flat shading: E/W faces are darker to give depth.
fn wall_color(material_id: u16, side: Side) -> u32 {
    let base = match material_id % 4 {
        0 => (0xCC, 0x33, 0x33), // red
        1 => (0x33, 0xCC, 0x33), // green
        2 => (0x33, 0x33, 0xCC), // blue
        _ => (0x99, 0x99, 0x99), // grey
    };
    let (r, g, b) = match side {
        Side::North | Side::South => base,
        Side::East | Side::West => (base.0 / 2, base.1 / 2, base.2 / 2),
    };
    0xFF00_0000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// Render a raycasted view into the framebuffer.
///
/// Casts one `cast_ray_detailed` per screen column. Draws vertical wall strips.
/// Writes perpendicular distances into the z-buffer for hybrid compositing.
pub fn render_raycast_view(
    fb: &mut Framebuffer,
    zbuf: &mut ZBuffer,
    map: &impl GridMap,
    camera_pos: Vec2Fixed,
    camera_angle: Bam,
    fov: Bam,
) {
    let width = fb.width() as i32;
    let height = fb.height() as i32;
    let half_fov = Bam(fov.raw() / 2);

    for col in 0..width {
        // Compute ray angle for this column
        let frac = col as f32 / width as f32; // 0.0 .. 1.0
        let angle_offset = Bam::from_radians(
            -half_fov.to_radians() + frac * fov.to_radians()
        );
        let ray_angle = camera_angle + angle_offset;

        // Cast the ray
        let Some(detail) = cast_ray_detailed(map, camera_pos, ray_angle) else {
            continue; // ray escaped map — draw nothing (ceiling/floor only)
        };

        let perp_dist = detail.hit.distance.to_f32();
        if perp_dist <= 0.0 {
            continue;
        }

        // Wall strip height (proportional to 1/distance)
        let wall_height = (height as f32 / perp_dist).min(height as f32) as i32;
        let draw_start = (height / 2 - wall_height / 2).max(0);
        let draw_end = (height / 2 + wall_height / 2).min(height - 1);

        // Get wall material from map
        let cell = map.cell_at(detail.hit.cell_x, detail.hit.cell_y);
        let material_id = match cell {
            abrash_raycast::types::Cell::Solid(id) => id,
            _ => 0,
        };
        let color = wall_color(material_id, detail.hit.side);

        // Draw the column
        for y in draw_start..=draw_end {
            fb.set_pixel(col, y, color);
        }

        // Write distance to z-buffer for hybrid compositing
        for y in draw_start..=draw_end {
            // Z-buffer uses f32, perp_dist is already what we need
            zbuf.set_depth(col, y, perp_dist);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_raycast::map::ArrayGridMap;
    use abrash_raycast::types::Cell;

    fn simple_room() -> ArrayGridMap {
        let mut map = ArrayGridMap::new(8, 8);
        for x in 0..8 {
            map.set(x, 0, Cell::Solid(1));
            map.set(x, 7, Cell::Solid(1));
        }
        for y in 0..8 {
            map.set(0, y, Cell::Solid(1));
            map.set(7, y, Cell::Solid(1));
        }
        map
    }

    #[test]
    fn render_does_not_panic() {
        let map = simple_room();
        let mut fb = Framebuffer::new(320, 200).unwrap();
        let mut zbuf = ZBuffer::new(320, 200).unwrap();
        let pos = Vec2Fixed::from_f32(4.0, 4.0);
        let angle = Bam::ZERO;
        let fov = Bam(0x2000_0000); // ~45 degrees

        render_raycast_view(&mut fb, &mut zbuf, &map, pos, angle, fov);
    }

    #[test]
    fn render_draws_pixels() {
        let map = simple_room();
        let mut fb = Framebuffer::new(320, 200).unwrap();
        let mut zbuf = ZBuffer::new(320, 200).unwrap();
        let pos = Vec2Fixed::from_f32(4.0, 4.0);
        let angle = Bam::ZERO;
        let fov = Bam(0x4000_0000); // 90 degrees

        render_raycast_view(&mut fb, &mut zbuf, &map, pos, angle, fov);

        // Center of screen should have been drawn (wall in front)
        let center_x = 160;
        let center_y = 100;
        let pixel = fb.get_pixel(center_x, center_y);
        assert!(pixel != Some(0), "center pixel should be drawn");
    }

    #[test]
    fn zbuffer_written_for_wall_columns() {
        let map = simple_room();
        let mut fb = Framebuffer::new(320, 200).unwrap();
        let mut zbuf = ZBuffer::new(320, 200).unwrap();
        let pos = Vec2Fixed::from_f32(4.0, 4.0);
        let angle = Bam::ZERO;
        let fov = Bam(0x4000_0000);

        render_raycast_view(&mut fb, &mut zbuf, &map, pos, angle, fov);

        // Z-buffer at center should have a finite depth (not infinity)
        let depth = zbuf.get_depth(160, 100).unwrap_or(f32::INFINITY);
        assert!(depth < f32::INFINITY, "z-buffer should have wall depth");
        assert!(depth > 0.0, "depth should be positive");
    }
}
```

**Step 4: Register module**

Add to `crates/abrash-render/src/lib.rs`:
```rust
pub mod raycaster;
```

**Step 5: Run tests**

Run: `cargo test -p abrash-render raycaster`
Expected: ALL PASS

**Step 6: Commit**

```bash
git add crates/abrash-render/src/raycaster/ crates/abrash-render/src/lib.rs crates/abrash-render/Cargo.toml
git commit -m "feat(render): add raycaster column renderer with z-buffer bridge"
```

---

### Task 11: Raycast demo example

**Files:**
- Create: `examples/raycast_demo.rs`
- Modify: `Cargo.toml` (add `[[example]]` entry)

**Step 1: Write the demo**

Create `examples/raycast_demo.rs` — a Wolfenstein-style walkthrough using the winit backend:

```rust
//! Raycaster demo — Wolfenstein-style first-person walkthrough.
//!
//! Uses abrash-raycast for DDA grid raycasting and abrash-render for column rendering.
//! Controls: WASD to move, arrow keys to turn.

use abrash::platform::WindowBackend;
use abrash_core::bam::{Bam, ANG90};
use abrash_core::fixed16_16::Fixed16_16;
use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_raycast::map::ArrayGridMap;
use abrash_raycast::types::{Cell, Vec2Fixed};
use abrash_render::raycaster::hybrid::render_raycast_view;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;

fn make_demo_map() -> ArrayGridMap {
    let mut map = ArrayGridMap::new(16, 16);
    // Border walls
    for x in 0..16 {
        map.set(x, 0, Cell::Solid(3));
        map.set(x, 15, Cell::Solid(3));
    }
    for y in 0..16 {
        map.set(0, y, Cell::Solid(3));
        map.set(15, y, Cell::Solid(3));
    }
    // Interior rooms
    for x in 4..8 {
        map.set(x, 4, Cell::Solid(0)); // red wall
    }
    for y in 4..8 {
        map.set(8, y, Cell::Solid(1)); // green wall
    }
    map.set(10, 10, Cell::Solid(2)); // blue pillar
    map.set(12, 6, Cell::Solid(2));
    map.set(3, 10, Cell::Solid(0));
    map
}

fn main() {
    let map = make_demo_map();
    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut zbuf = ZBuffer::new(WIDTH, HEIGHT).unwrap();

    let mut pos = Vec2Fixed::from_f32(3.0, 3.0);
    let mut angle = Bam::ZERO;
    let fov = ANG90; // 90 degree FOV

    let move_speed = Fixed16_16::from_f32(0.05);
    let turn_speed = Bam(0x0200_0000); // ~11 degrees per frame

    let mut window = abrash::platform::Window::new("Raycast Demo", WIDTH, HEIGHT);

    window.run(|_dt, keys| {
        // Clear
        fb.clear(0xFF20_2020); // dark grey background (ceiling/floor)
        zbuf.clear();

        // Input
        let (sin, cos) = angle.sin_cos_fixed();
        if keys.contains(&winit::keyboard::KeyCode::KeyW) {
            pos.x = pos.x + cos.fixed_mul(move_speed);
            pos.y = pos.y + sin.fixed_mul(move_speed);
        }
        if keys.contains(&winit::keyboard::KeyCode::KeyS) {
            pos.x = pos.x - cos.fixed_mul(move_speed);
            pos.y = pos.y - sin.fixed_mul(move_speed);
        }
        if keys.contains(&winit::keyboard::KeyCode::ArrowLeft) {
            angle = angle + turn_speed;
        }
        if keys.contains(&winit::keyboard::KeyCode::ArrowRight) {
            angle = angle - turn_speed;
        }

        // Render
        render_raycast_view(&mut fb, &mut zbuf, &map, pos, angle, fov);

        &fb
    });
}
```

**Step 2: Add example entry to `Cargo.toml`**

```toml
[[example]]
name = "raycast_demo"
path = "examples/raycast_demo.rs"
```

**Step 3: Verify it compiles**

Run: `cargo build --example raycast_demo`
Expected: PASS (may need adjustments to the window.run() callback signature to match your platform abstraction — check how other examples like `cube_3d.rs` work and match that pattern)

**IMPORTANT**: The demo code above is a sketch. The actual input handling and window loop MUST match the existing platform abstraction pattern used in other examples. Read `examples/cube_3d.rs` to understand the exact callback signature and key input mechanism, then adapt.

**Step 4: Run it visually**

Run: `cargo run --example raycast_demo`
Expected: A window showing raycasted walls, WASD + arrows to navigate

**Step 5: Commit**

```bash
git add examples/raycast_demo.rs Cargo.toml
git commit -m "feat: add raycast_demo example with WASD navigation"
```

---

### Task 12: Full test suite pass + clippy

**Files:** None new — verification pass

**Step 1: Run full workspace tests**

Run: `cargo test --workspace`
Expected: ALL PASS

**Step 2: Run clippy on all crates**

Run: `cargo clippy --workspace`
Expected: No errors (warnings OK from existing code)

**Step 3: Format check**

Run: `cargo fmt --check`
Expected: No formatting issues (run `cargo fmt` if needed)

**Step 4: Commit any fixups**

```bash
git add -A
git commit -m "chore: fmt + clippy fixes for raycaster implementation"
```

---

## Summary

| Phase | Tasks | What it delivers |
|-------|-------|-----------------|
| 1 (Math) | Tasks 1-3 | `Fixed16_16`, `Bam` with const table, trig benchmarks |
| 2 (Raycast) | Tasks 4-7 | `abrash-raycast` crate: DDA, tiered API, property tests |
| 3 (Batch) | Tasks 8-9 | Batch casting, rayon parallelism, perf benchmarks |
| 4 (Hybrid) | Tasks 10-12 | Column renderer, z-buffer bridge, demo, full verification |
