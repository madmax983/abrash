# Bolt's Journal ⚡

**[Fixed-Point Optimization Noise]**
**Learning:** Benchmarking heavy CPU operations like rasterization can be extremely noisy in shared environments. A theoretically sound optimization (hoisting float-to-int conversion out of a loop) showed negligible gain or even regression due to system noise affecting unrelated benchmarks by >60%.
**Action:** Always verify "zero-cost" abstractions logically. If the benchmark is noisy, trust the instruction count reduction for hot loops, provided tests pass and no obvious memory regression occurs. Also, verify that passing large tuples by value doesn't introduce excessive stack spilling compared to references.

**[Struct Layout Optimization Backfire]**
**Learning:** Reducing struct size to fit in a cache line by mixing types (i64/i32) caused a ~17% regression in `fill_triangle_gouraud_small`. The cost of sign-extending `i32` to `i64` during hot loop updates, plus potential loss of autovectorization for homogenous `i64` fields, outweighed the cache locality gains for this specific workload.
**Action:** When optimizing struct layout, consider the instruction cost of accessing mixed-type fields in hot paths, especially if it breaks SIMD patterns.

**[Reciprocal Table Win]**
**Learning:** Replacing integer division by `count` (1..16) with a lookup table of reciprocals (`1.0/count`) yielded a ~3-11% improvement in textured triangle rasterization.
**Action:** Always look for repeated divisions by small integers in hot loops and replace them with reciprocal multiplication.

**[Allocation Reuse & Cache Aliasing]**
**Learning:** Removing per-tile allocations in `render_single_tile` by reusing `Vec` buffers initially caused a massive 28% regression for 200+ triangles. The cause was 4KB cache aliasing between `tile_pixels` and `tile_depths` (both 4KB) being allocated back-to-back at conflicting cache set indices. Adding 32 elements (128 bytes) of padding to the capacity shifted the base pointers, resolving the conflict and turning the regression into a ~3.5% speedup.
**Action:** When reusing large buffers (>= page size) that are accessed together in a tight loop, ensure they are not aligned to cache way multiples (e.g. 4KB) to avoid set conflicts. Use `Vec::with_capacity(size + padding)` to force offset.
