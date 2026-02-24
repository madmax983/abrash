# SoftBody SIMD Optimization Results

## Objective
Accelerate `SoftBody::update` using AVX2 instructions for Force Accumulation and Integration steps.

## Implementation
- Implemented `load_vec3s_avx` and `store_vec3s_avx` helpers to handle AoS to SoA conversion for `Vec3` (12-byte stride).
- Vectorized Gravity, Drag, and Integration loops processing 8 vertices at a time.
- Scalar fallback for Spring constraints (due to random memory access patterns).

## Results
Benchmark: `softbody/update_20x20` (400 vertices, ~1500 springs)

| Implementation | Time (µs) | Improvement |
|----------------|-----------|-------------|
| Scalar (Baseline) | ~36.6 | - |
| AVX2 Optimized | ~31.1 | ~15% |

## Analysis
The optimization targets the O(N) vertex loops. The O(E) spring loop remains scalar. Since E approx 4N for grid meshes, the spring loop dominates execution time.
Further optimization would require refactoring `Spring` storage to SoA to enable vectorized spring calculations.
