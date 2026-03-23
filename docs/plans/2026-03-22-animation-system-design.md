# Animation System Design

**Date:** 2026-03-22
**Status:** Approved
**Inspired by:** Arthropod `anim-graph` crate

## Summary

Add a composable, phase-based animation system to the abrash workspace as a
new crate `abrash-anim`. The render pipeline (`abrash-render`) remains
untouched — animation is a peer crate that consumers wire up themselves.

Two new types (`Quat`, `Transform`) are added to `abrash-core` to support
smooth rotation interpolation and decomposed TRS transforms.

## Architecture Decision

**Animation lives outside the render layer.** `abrash-anim` depends on
`abrash-core` for math types but has no dependency on `abrash-render`.
Consumers call `timeline.tick(dt)` and apply the result to `SceneObject`
themselves via `transform.to_mat4()`.

This keeps the rasterizer focused on rasterization and makes the animation
system reusable outside of rendering contexts.

## New Types in `abrash-core`

### `Quat` (unit quaternion)

Location: `crates/abrash-core/src/math.rs`

```rust
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}
```

Core operations:
- `identity()`, `from_axis_angle(axis: Vec3, angle: f32)`, `from_euler(pitch, yaw, roll)`
- `slerp(&self, other: &Quat, t: f32) -> Quat` — spherical linear interpolation
- `normalize(&self) -> Quat`
- `to_mat4(&self) -> Mat4` — convert to rotation matrix
- `Mul<Quat>` — compose rotations
- `Animatable` impl where `interpolate` delegates to `slerp`

### `Transform` (decomposed TRS)

Location: `crates/abrash-core/src/math.rs`

```rust
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}
```

Core operations:
- `identity()` — zero position, identity rotation, uniform scale 1.0
- `to_mat4(&self) -> Mat4` — compose as Scale * Rotation * Translation
- `from_mat4(m: &Mat4) -> Transform` — decompose for interop
- `Animatable` impl: lerp position/scale, slerp rotation

### `Animatable` trait

Location: `crates/abrash-core/src/animatable.rs`

```rust
pub trait Animatable: Clone + 'static {
    fn interpolate(&self, other: &Self, t: f32) -> Self;
    fn scale(&self, scalar: f32) -> Self;
    fn add(&self, other: &Self) -> Self;
    fn sub(&self, other: &Self) -> Self;
    fn zero() -> Self;
    fn distance_squared(&self, other: &Self) -> f32;
}
```

Implementations for: `f32`, `Vec2`, `Vec3`, `Quat`, `Transform`

## `abrash-anim` Crate

### Module Layout

```
crates/abrash-anim/src/
├── lib.rs            // re-exports
├── clock.rs          // AnimationClock, PlaybackMode, ClockEvent
├── evaluable.rs      // Evaluable<T> trait, Sample<T>
├── easing.rs         // Easing enum with apply() + derivative()
├── keyframe.rs       // Keyframe<T> — tween segment with easing
├── hold.rs           // Hold<T> — constant value pause
├── sequence.rs       // Sequence<T> — proportional chaining
└── timeline.rs       // Timeline<T> — stateful driver
```

### Dependencies

- `abrash-core` (for math types + `Animatable` trait)
- No dependency on `abrash-render`

### Core Components

#### `AnimationClock`

Drift-free two-component time model using `(cycle: u64, phase: f32)`.
- `cycle` is an exact integer counter — never accumulates floating-point error
- `phase` stays in 0.0–1.0, resets each cycle
- `MAX_DELTA_SECS` (100ms) clamping for tab-backgrounding resilience
- `effective_phase()` handles PingPong by reversing on odd cycles

#### `Evaluable<T>` trait

Pure function from normalized phase (0.0–1.0) to `Sample<T>`.

```rust
pub trait Evaluable<T: Animatable>: Send + Sync {
    fn evaluate(&self, phase: f32) -> Sample<T>;
    fn natural_duration(&self) -> f32;
}
```

#### `Sample<T>`

Value paired with instantaneous velocity. Every `Evaluable` returns both,
enabling future velocity-preserving spring interruption.

```rust
pub struct Sample<T: Animatable> {
    pub value: T,
    pub velocity: T,
}
```

#### `Easing`

Standard easing functions with analytical derivatives for velocity computation.

Variants: `Linear`, `EaseIn`, `EaseOut`, `EaseInOut`, `CubicBezier(f32, f32, f32, f32)`

#### `Keyframe<T>`

Tween between two values with easing. Velocity derived analytically from
`easing.derivative(phase) * (to - from) / duration`.

#### `Hold<T>`

Constant value for a duration. Returns `Sample::at_rest(value)`. Needed for
pauses within sequences.

#### `Sequence<T>`

Proportional end-to-end chaining of `Evaluable` segments. Each child gets a
phase slice proportional to `natural_duration() / total_duration`.

#### `Timeline<T>`

Stateful orchestrator that owns an `Evaluable` root and an `AnimationClock`.

- `tick(delta_secs) -> Sample<T>` — advance and evaluate
- `tween()`, `sequence()` — convenience constructors
- `.easing()`, `.loop_forever()`, `.ping_pong()`, `.count(n)` — modifiers
- `is_completed()`, `reset()`, `current_value()`
- Interruption support is structurally present (future spring addition)

### Not Included (future additions)

- `SpringSegment` — physics-based spring with velocity preservation
- `Stagger` — offset parallel composition (UI-oriented)
- `TimeWarp` — phase remapping wrapper
- ECS integration — abrash has no ECS

## Integration Pattern

Consumer-side wiring, no framework coupling:

```rust
// Create animation
let patrol = Timeline::sequence()
    .then_tween(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0), Duration::from_secs(2))
        .easing(Easing::EaseInOut)
    .then_hold(Vec3::new(10.0, 0.0, 0.0), Duration::from_millis(500))
    .then_tween(Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO, Duration::from_secs(2))
        .easing(Easing::EaseInOut)
    .build()
    .loop_forever();

// In render loop
let dt = self.timestep.dt();
let sample = self.patrol_timeline.tick(dt);
let transform = Transform::from_position(sample.value);
self.scene.objects[0].transform = transform.to_mat4();
```

## `SceneObject` Unchanged

`SceneObject::transform` remains `Mat4`. Consumers call `.to_mat4()` on
`Transform` values produced by the animation system. No breaking changes to
the render pipeline.

## Testing Strategy

### `abrash-core` (new type tests)

- **Quat**: identity, from_axis_angle, slerp at 0/0.5/1, shortest path, normalize,
  to_mat4 matches `Mat4::rotation_*`, compose rotations
- **Transform**: identity, to_mat4/from_mat4 roundtrip, Animatable interpolate
  (verify position lerps, rotation slerps, scale lerps independently)
- **Animatable impls**: interpolate at boundaries (0.0, 1.0), zero(),
  distance_squared symmetry

### `abrash-anim` (animation tests)

- **AnimationClock**: tick phase advance, cycle boundary detection,
  effective_phase for PingPong, is_finished per mode, reset, delta clamping
- **Easing**: apply(0.0)==0.0 and apply(1.0)==1.0 for all variants,
  derivative matches numerical approximation
- **Keyframe**: evaluate at 0/0.5/1, nonzero mid-tween velocity
- **Hold**: constant value, zero velocity
- **Sequence**: proportional phase splitting, boundary handoff
- **Timeline**: tick drives value, loop cycles, ping_pong reverses,
  is_completed for Once, reset restarts

### Integration demo

Enhance existing `cube_3d` example to use `Timeline` instead of manual angle
accumulation, demonstrating the full pipeline.

No benchmarks — animation evaluation is trivially fast compared to rasterization.
