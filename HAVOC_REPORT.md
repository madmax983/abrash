# 👺 Havoc Report: "The Abyss Gazes Back"

## 1. Denial of Service via Huge Triangles
**The Trigger:** A triangle with coordinates spanning from `-1e30` to `1e30`.

**The Mechanism:**
- The rasterizer loop iterates from `y_min` to `y_max`.
- `f32` to `i32` cast saturates to `i32::MIN` (-2B) and `i32::MAX` (2B).
- Result: 4,294,967,296 iterations per triangle.
- **Outcome:** Application hangs for >2 minutes per frame. DoS.

**The Fix:** Clamped the rasterization loop to the framebuffer height (`0..fb.height()`).

## 2. Panic via Allocation Overflow
**The Trigger:** `Framebuffer::new(u32::MAX, u32::MAX)`.

**The Mechanism:**
- `checked_mul` detects the overflow correctly.
- But `.expect("Buffer size overflow")` panics the thread.
- **Outcome:** Server/Application crash.

**The Fix:** Changed constructors to return `Result<Self, Error>` and propagate errors up the stack.

## 3. Undefined Behavior via NaN
**The Trigger:** Triangle with `NaN` coordinates.

**The Mechanism:**
- `NaN as i32` is technically Undefined Behavior in Rust (though saturates to 0 in practice on many targets).
- Comparisons with `NaN` yield `false`, breaking sorting logic (`v0`, `v1`, `v2` remain unsorted).
- **Outcome:** Unpredictable rendering, potential future UB.

**The Fix:** Added explicit `.is_finite()` checks to reject invalid geometry early.

## 4. CPU Exhaustion via OBJ Cache Collision
**The Trigger:** An OBJ file with 50,000+ vertices sharing the same 3D position but unique texture coordinates (or normals).

**The Mechanism:**
- The OBJ loader uses a custom linked-list cache (`cache_nodes` and `cache_head`) to deduplicate vertices.
- Vertices are indexed by their raw position index `v_idx`.
- When many `f` commands reference the same `v` index but different `vt` indices, the linked list for that `v_idx` grows linearly.
- Each lookup iterates the list.
- **Outcome:** Quadratic complexity $O(N^2)$ for loading the mesh. A 50k vertex file takes >30 seconds to load instead of <100ms. DoS.

**The Fix:** Implemented a depth limit (8) for the cache chain traversal. If a match isn't found within 8 steps, the vertex is treated as new (skipping deduplication) to ensure O(1) lookup time.

## 5. SoftBody Panic via Invalid Mesh Indices
**The Trigger:** A `Mesh` constructed with indices pointing to non-existent vertices (e.g., `indices=[0, 0, 1]` but `vertices.len() == 1`).

**The Mechanism:**
- `SoftBody::new` (or `update`) blindly trusts `mesh.indices`.
- It accesses `mesh.vertices[index]` without bounds checking (or rather, relying on Rust's bounds checking which panics).
- **Outcome:** Panic (Crash).

**The Fix:** `SoftBody` now validates mesh integrity in `update()` and logs an error instead of panicking if a mismatch is detected.

## 6. SoftBody Panic via Mesh Truncation
**The Trigger:** Manually clearing `SoftBody.mesh.vertices` after creation.

**The Mechanism:**
- The internal physics state (`velocities`, `forces`) retains the old size.
- `update()` loop iterates based on `mesh.vertices.len()`.
- If truncated to 0, loop is skipped, but springs still hold old indices.
- **Outcome:** `update` panics when accessing springs if not validated.

**The Fix:** Added validation in `SoftBody::update` to ensure `mesh.vertices.len()` matches the internal physics state. If mismatched, the update is skipped gracefully.

## Other Findings
- **Mat4 SIMD Robustness**: `Mat4::transform_points` withstands fuzzing with `NaN`s and `Infinity` using AVX2, matching scalar implementation behavior.
- **Gouraud Rasterizer**: `src/rasterizer/gouraud.rs` contains numerous unnecessary `unsafe` blocks around safe SIMD intrinsics.
- **OBJ Loader Robustness**: `proptest` fuzzing confirmed `load_obj` does not panic on random input strings, gracefully returning errors.

# 👺 Havoc: SoftBody::collide_sdf Out-of-Bounds Panic

## 🧨 The Trigger
Calling `collide_sdf()` after mutating the public `velocities` array (e.g. `jelly.velocities.clear()`) while leaving `mesh.vertices` populated. Because `collide_sdf` lacks the structural integrity validation that the main `update` method has, a collision forces a read from the truncated `velocities` array based on the original vertex index, triggering a fatal out-of-bounds read panic.

## 📉 The Stack Trace
```
thread 'test_havoc_softbody_panic' panicked at src/experimental/jelly.rs:657:40:
index out of bounds: the len is 0 but the index is 0
stack backtrace:
   0: rust_begin_unwind
             at /rustc/ded5c06cf21d2b93bffd5d884aa6e96934ee4234/library/std/src/panicking.rs:662:5
   1: core::panicking::panic_fmt
             at /rustc/ded5c06cf21d2b93bffd5d884aa6e96934ee4234/library/core/src/panicking.rs:74:14
   2: core::panicking::panic_bounds_check
             at /rustc/ded5c06cf21d2b93bffd5d884aa6e96934ee4234/library/core/src/panicking.rs:276:5
   3: <usize as core::slice::index::SliceIndex<[T]>>::index
             at /rustc/ded5c06cf21d2b93bffd5d884aa6e96934ee4234/library/core/src/slice/index.rs:302:10
   4: core::slice::index::<impl core::ops::index::Index<I> for [T]>::index
             at /rustc/ded5c06cf21d2b93bffd5d884aa6e96934ee4234/library/core/src/slice/index.rs:16:9
   5: alloc::vec::<impl core::ops::index::Index<I> for alloc::vec::Vec<T,A>>::index
             at /rustc/ded5c06cf21d2b93bffd5d884aa6e96934ee4234/library/alloc/src/vec/mod.rs:2910:9
   6: abrash::experimental::jelly::SoftBody::collide_sdf
             at /app/src/experimental/jelly.rs:657:25
   7: havoc_softbody::test_havoc_softbody_panic
             at /app/tests/havoc_softbody.rs:24:5
```

## 🧪 Reproduction
```rust
use abrash::experimental::jelly::SoftBody;
use abrash::experimental::sdf::{SdfScene, SdfPrimitive, SdfObject};
use abrash::mesh::Mesh;
use abrash::math::Vec3;

#[test]
fn test_havoc_softbody_panic() {
    let mut mesh = Mesh::new();
    mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
    mesh.indices.push([0, 0, 0]);

    let mut jelly = SoftBody::new(mesh, 1.0, 10.0, 0.5).unwrap();

    let mut scene = SdfScene::new();
    scene.add(SdfObject {
        primitive: SdfPrimitive::Sphere { radius: 10.0, center: Vec3::new(0.0, 0.0, 0.0) },
        color: 0xFFFFFFFF,
    });

    jelly.velocities.clear();
    jelly.collide_sdf(&scene, 0.5);
}
```

## 😈 Comment
You verified structural synchronicity in `update()` but blindly assumed the user wouldn't touch the public `velocities` vector before throwing them into an SDF collision! You trusted public state. You were wrong.

# 👺 Havoc: Arboretum L-System OOM Vulnerability

## 🧨 The Trigger
Expanding an L-System string exclusively filled with `[` characters using `arboretum::LSystem::generate_mesh()`. While the module limits string expansion memory to 100MB, it does not limit the Turtle interpretation phase. The `[` instruction endlessly pushes `Turtle` state to a stack without bounds checking. Additionally, `Mesh::with_capacity` pre-allocates based on the string length.

## 📉 The Stack Trace
```
memory allocation of 9600000000 bytes failed
stack backtrace:
   0: std::alloc::rust_oom
   1: __rustc::__rust_alloc_error_handler
   2: alloc::alloc::handle_alloc_error::rt_error
   3: alloc::alloc::handle_alloc_error
   4: alloc::raw_vec::handle_error
   5: <alloc::raw_vec::RawVecInner>::with_capacity_in
   6: <alloc::raw_vec::RawVec<[usize; 3]>>::with_capacity_in
   7: <alloc::vec::Vec<[usize; 3]>>::with_capacity_in
   8: <alloc::vec::Vec<[usize; 3]>>::with_capacity
   9: <abrash_core::mesh::Mesh>::with_capacity
  10: <abrash_render::experimental::arboretum::LSystem>::generate_mesh
  11: havoc_arboretum_oom::test_havoc_arboretum_oom
```

## 🧪 Reproduction
Run the following test command:
```bash
cargo test --test havoc_arboretum_oom --features nova -- --ignored
```

## 😈 Comment
You remembered to limit the string expansion, but forgot that strings are executable code. You let the Turtle walk straight off a 9.6-Gigabyte cliff via pre-allocation and infinite stacks. You were wrong.

# 👺 Havoc: draw_circle and fill_circle i32 Math Overflow Panic

## 🧨 The Trigger
Calling `draw_circle` or `fill_circle` with an extremely large `radius` (e.g., `536894084` or higher). While `radius` is an `i32`, the algorithm calculates `let mut d = 3 - 2 * radius;`. When `radius` is large enough, `2 * radius` overflows the `i32` boundary, causing a fatal panic in debug/test builds or potentially undefined wrapping behavior in release mode.

## 📉 The Stack Trace
```
thread 'test_draw_circle_overflow' panicked at crates/abrash-render/src/rasterizer/circle.rs:84:25:
attempt to multiply with overflow
stack backtrace:
   0: rust_begin_unwind
             at /rustc/ded5c06cf21d2b93bffd5d884aa6e96934ee4234/library/std/src/panicking.rs:662:5
   1: core::panicking::panic_fmt
             at /rustc/ded5c06cf21d2b93bffd5d884aa6e96934ee4234/library/core/src/panicking.rs:74:14
   2: core::panicking::panic_const::panic_const_mul_overflow
             at /rustc/ded5c06cf21d2b93bffd5d884aa6e96934ee4234/library/core/src/panicking.rs:252:21
   3: abrash_render::rasterizer::circle::draw_circle
             at /app/crates/abrash-render/src/rasterizer/circle.rs:84:25
   4: havoc_circle_panic::test_draw_circle_overflow
             at /app/crates/abrash-render/tests/havoc_circle_panic.rs:10:5
```

## 🧪 Reproduction
Run the following fuzzing proptest command:
```bash
cargo test -p abrash-render --test havoc_circle_proptest
```

## 😈 Comment
You thought a radius parameter was just a number. You forgot that Bresenham’s circle algorithm multiplies that radius during the decision parameter `d` initialization (`d = 3 - 2 * r`). An `i32` maxes out at ~2.14 billion. Double `536,894,084` and subtract from 3, and you blow right past the numeric limits. You assumed `i32` was big enough for a screen, but users will pass anything they want. You were wrong.
