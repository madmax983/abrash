# 👺 Havoc: Undefined Behavior via `MaybeUninit` Array Length Manipulation

## 🧨 The Trigger
Mutating the public `count` field of `ClippedTriangles` or `PreparedTrianglesList` structures directly.

## 📉 The Stack Trace
```
(No panic stack trace for UB - Miri or Valgrind required to catch silent memory corruption)
Read uninitialized memory! (0.0, 0.0, 0.0) [Garbage Data]
```

## 🧪 Reproduction
```rust
use abrash_core::clipping::clip_triangle_to_frustum;
use abrash_core::math::Vec3;

#[test]
fn test_havoc_clipping_ub() {
    let v0 = (Vec3::new(0.0, 0.0, 0.0), 1.0);
    let v1 = (Vec3::new(1.0, 0.0, 0.0), 1.0);
    let v2 = (Vec3::new(0.0, 1.0, 0.0), 1.0);

    let mut clipped = clip_triangle_to_frustum(
        v0, v1, v2,
        |v| *v,
        |a, _, _| a
    );

    // The length tracker for the internal MaybeUninit array is PUBLIC!
    clipped.count = 8; // Manually expand the valid bounds

    // Read index 7, which was never initialized by the clipping algorithm.
    // Result: Undefined Behavior!
    let bad_read = clipped[7];
}
```

## 😈 Comment
You cleverly used `[MaybeUninit<V>; N]` to eliminate heap allocations for small, stack-bound triangle lists. But you left the door to the vault wide open by making `count` public. A single mutable assignment `count = 8` tells Rust that the entire uninitialized array is valid data, allowing safe code to bypass bounds checks and read garbage memory or trigger segfaults. "Safe" Rust just became Unsafe. You were wrong.
