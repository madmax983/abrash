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

**The Fix:** Not fixed. Reproduction provided in `tests/havoc.rs`.

## Other Findings
- **Mat4 SIMD Robustness**: `Mat4::transform_points` withstands fuzzing with `NaN`s and `Infinity` using AVX2, matching scalar implementation behavior.
- **Gouraud Rasterizer**: `src/rasterizer/gouraud.rs` contains numerous unnecessary `unsafe` blocks around safe SIMD intrinsics.
