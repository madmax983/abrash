# Bolt's Journal

**Learning:** Bounds checking in inner rasterization loops is a major bottleneck.
**Action:** When optimizing hot loops like `fill_triangle`, clamp coordinates to screen bounds *outside* the loop and use `unsafe` unchecked access inside. This yielded a ~73% speedup.

**Learning:** `fill_triangle_3d` had a hidden per-pixel division for Z-interpolation.
**Action:** Always look for invariants in loops. `dz/dx` is constant across a scanline; pre-calculating it removed a division per pixel.

**Learning:** Windows-specific crates break `cargo bench` on Linux.
**Action:** Provide dummy implementations for platform-specific modules behind `#[cfg(not(target_os = "windows"))]` to allow CI/benchmarking to run everywhere.

**[Safe vs Unsafe Optimization]**
**Learning:** Replacing per-pixel division with incremental addition yielded ~49% speedup (unsafe) vs ~32% speedup (safe) in `fill_triangle_gouraud`. The cost of bounds checking is measurable (~1.2ms regression vs unsafe) but the algorithmic win dominates.
**Action:** Prefer algorithmic optimizations (hoisting invariants) first. Only resort to `unsafe` if the remaining overhead (bounds checks) is the primary bottleneck and strictly necessary.

**[Sorting Allocation]**
**Learning:** `slice::sort_by_key` allocates temporary storage even for tiny slices, which is costly in hot paths like triangle setup.
**Action:** Use manual sorting networks (swaps) for fixed-size small arrays (e.g., 3 vertices) to ensure zero allocation.

**[Edge Walking vs Explicit Calculation]**
**Learning:** Calculating barycentric-like coordinates per scanline using division is cleaner but slower (~12% on large triangles) than incremental edge walking (DDA).
**Action:** For rasterization outer loops, use incremental addition (DDA) to update edge coordinates. Pre-calculate gradients once per triangle.

**[Scalar Replacement of Aggregates (SRA)]**
**Learning:** Constructing structs (like `Vec3`) inside hot inner loops can prevent register allocation optimizations, even with inlining. Decomposing aggregates into scalar variables (`r, g, b` vs `Vec3`) in the inner loop yielded measurable improvement.
**Action:** For extreme hot loops (pixel shaders), manually scalarize vector operations if benchmarks indicate a bottleneck.

**[Integer Overflow in Geometry]**
**Learning:** Screen coordinates (`i32`) can be extreme (e.g., `i32::MAX/MIN`) when vertices are projected from far off-screen. Simple subtraction `p1.x - p0.x` overflows and causes panics in debug mode (or wrapping in release).
**Action:** Always cast screen coordinates to `i64` before subtraction when calculating gradients or edge spans: `(p1.x as i64 - p0.x as i64) as f32`.

**[Zero-Cost Math Simplification]**
**Learning:** Checking `x_long < x_other` to determine left/right edge involves division/multiplication. The sign of the 2D cross-product (`nz`) already computed for `dz/dx` gives the winding order and thus the side directly.
**Action:** Reuse the cross-product Z-component sign (`nz > 0.0`) to determine `long_edge_is_left` without extra math.

**[Micro-optimizations & Unrolling]**
**Learning:** Manual loop unrolling in `Mat4::mul` (4x4 matrix multiplication) caused a ~15% performance regression (31ns -> 36ns), likely due to increased register pressure or I-cache pressure preventing efficient autovectorization.
**Action:** Trust LLVM's autovectorizer for small fixed-size loops. Verify "obvious" optimizations with benchmarks.

**[Division Optimization]**
**Learning:** Replacing 3 divisions with 1 reciprocal calculation and 3 multiplications in `Vec3::normalize` yielded a ~15% speedup (3.2ns -> 2.7ns).
**Action:** Always prefer multiplication by inverse for vector normalization or scaling.

**[Fixed-Point Optimization]**
**Learning:** Hoisting float-to-fixed-point color conversion out of the inner scanline loop and using integer arithmetic for edge walking in the outer loop yielded a ~12% speedup in `fill_triangle_gouraud`.
**Action:** For rasterization, convert continuous attributes (like color, UVs) to fixed-point integers as early as possible (triangle setup) to avoid float overhead in inner loops.

**[Integer Demotion Optimization]**
**Learning:** Demoting 16.16 fixed-point accumulators from `i64` to `i32` in the hot rasterization loop yielded ~4.5% speedup without correctness loss, as values fit within `i32` range on-screen.
**Action:** Use the smallest integer type that fits the range for hot loops to reduce register pressure. Keep `i64` only for intermediate calculations (like off-screen jumps) that might overflow.
