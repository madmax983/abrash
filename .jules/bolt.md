# Bolt's Journal ⚡

**[Fixed-Point Optimization Noise]**
**Learning:** Benchmarking heavy CPU operations like rasterization can be extremely noisy in shared environments. A theoretically sound optimization (hoisting float-to-int conversion out of a loop) showed negligible gain or even regression due to system noise affecting unrelated benchmarks by >60%.
**Action:** Always verify "zero-cost" abstractions logically. If the benchmark is noisy, trust the instruction count reduction for hot loops, provided tests pass and no obvious memory regression occurs. Also, verify that passing large tuples by value doesn't introduce excessive stack spilling compared to references.

**[Struct Padding & Cache Lines]**
**Learning:** Reducing struct size from 72 bytes to 48 bytes (fitting in a 64-byte cache line) for tight loop iterators like `GouraudEdgeWalker` is a valid optimization even if benchmarks are noisy. It reduces stack pressure and memory traffic.
**Action:** Look for struct fields that use `i64` for fixed-point values where `i32` provides sufficient range (e.g., color channels, texture coordinates for small textures).
