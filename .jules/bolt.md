# Bolt's Journal ⚡

**[Fixed-Point Optimization Noise]**
**Learning:** Benchmarking heavy CPU operations like rasterization can be extremely noisy in shared environments. A theoretically sound optimization (hoisting float-to-int conversion out of a loop) showed negligible gain or even regression due to system noise affecting unrelated benchmarks by >60%.
**Action:** Always verify "zero-cost" abstractions logically. If the benchmark is noisy, trust the instruction count reduction for hot loops, provided tests pass and no obvious memory regression occurs. Also, verify that passing large tuples by value doesn't introduce excessive stack spilling compared to references.
