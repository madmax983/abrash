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

**[Integer Culling Precision]**
**Learning:** Switching from `f32` to `i64` for 2D cross-product backface culling improved performance by ~3-4% and eliminated potential floating-point precision errors for large coordinates. `f32` only has 24 bits of precision, which can be insufficient for `diff * diff` with large screen coordinates.
**Action:** Use integer arithmetic (`i64`) for geometric predicates on integer coordinates whenever possible.
